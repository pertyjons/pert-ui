//! A file dialog the library draws itself, over the file browser.
//!
//! [`FileDialogController`] owns one dialog's session: it is opened with a
//! [`FileDialogRequest`], shows a [`FileBrowser`] with a name field, a type
//! selector and confirm and cancel buttons, and retains a
//! [`FileDialogOutcome`] until the application takes it. [`FileDialog`]
//! presents it inside the owner's window. Like the browser, the dialog
//! performs no filesystem I/O: the host answers its [`DirectoryRequest`]s.
use crate::{
    CurrentListing, DialogController, DialogOutcome, DialogStatus, DirectoryEntry,
    DirectoryRequest, DirectoryResponse, EntryKind, FileBrowserController, FileBrowserError,
    GeometryError, LinkTarget, ListSelection, ListingDelivery, ReadSignal, RuntimeError,
    SelectionMode, Signal, StringRole, SurfaceDismissal, UiMessage,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::rc::Rc;

mod view;
pub use view::FileDialog;

/// What a file dialog asks the user for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileDialogMode {
    /// One existing file.
    OpenFile,
    /// One or more existing files, reported in the order the list shows them.
    OpenFiles,
    /// One folder: the highlighted one, or the folder the dialog shows. The
    /// entry list shows folders only.
    ChooseFolder,
    /// A path to save to: a name typed into the name field, in the folder the
    /// dialog shows, or an existing file after the user agreed to replace it.
    SaveFile,
}

/// Why a file dialog value was refused or a session could not start.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FileDialogError {
    /// A session is open, its outcome has not been taken, or its mounted
    /// owner has not yet observed the closure in a flush.
    #[error("the file dialog is busy with a session")]
    Busy,
    /// No root folder was given.
    #[error("a file dialog needs at least one root folder")]
    NoRoots,
    /// The same root folder was given twice.
    #[error("the root folder {} was given twice", .0.display())]
    DuplicateRoot(PathBuf),
    /// A context key that is empty, only whitespace, longer than
    /// [`FileDialogRequest::MAX_KEY_CHARS`] characters or holds a control
    /// character.
    #[error("a file dialog context is 1 to 64 visible characters")]
    InvalidContext,
    /// A filter whose identity breaks the rule of
    /// [`Self::InvalidContext`], whose label is blank or holds a control
    /// character, or with an extension that is empty, starts or ends with a
    /// dot, or holds a separator, `*`, `:`, whitespace or a control
    /// character; or a filter without extensions.
    #[error("the file filter is not valid")]
    InvalidFilter,
    /// Two filters of one request share an identity.
    #[error("the file filter {0:?} was given twice")]
    DuplicateFilter(String),
    /// A suggested name that is not one file name; see
    /// [`FileDialogRequest::suggested_name`].
    #[error("the suggested name is not one file name")]
    InvalidName,
    /// A dialog size below [`FileDialog::MINIMUM_SIZE`].
    #[error("the file dialog cannot be smaller than 480 by 320")]
    TooSmall,
    /// A row height or size that is not a valid length.
    #[error(transparent)]
    Geometry(#[from] GeometryError),
    /// The runtime refused a read or a write, for example during an update
    /// or after shutdown.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}
impl From<FileBrowserError> for FileDialogError {
    fn from(error: FileBrowserError) -> Self {
        match error {
            FileBrowserError::NoRoots => Self::NoRoots,
            FileBrowserError::DuplicateRoot(root) => Self::DuplicateRoot(root),
            FileBrowserError::Runtime(error) => Self::Runtime(error),
        }
    }
}

/// Whether `text` can be a context key or a filter identity.
fn valid_key(text: &str) -> bool {
    let count = text.chars().count();
    (1..=FileDialogRequest::MAX_KEY_CHARS).contains(&count)
        && !text.trim().is_empty()
        && !text.chars().any(char::is_control)
}

/// Whether `name` is one file name: one normal path component without a
/// separator or a control character, which the name field could not show. Names a filesystem may still reject are the application's to
/// discover when it writes.
fn valid_name(name: &str) -> bool {
    use std::path::Component;
    let separators: &[char] = if cfg!(windows) {
        &['/', '\\', ':', '\0']
    } else {
        &['/', '\0']
    };
    let mut components = Path::new(name).components();
    !name.is_empty()
        && !name.contains(separators)
        && !name.chars().any(char::is_control)
        && matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
}

/// A file type the user can choose in a file dialog: an identity, a label
/// and the name extensions that belong to it.
///
/// A file passes the filter when its name, compared without regard to case,
/// ends with a dot and one of the extensions and has at least one character
/// before that dot; a name that is not valid Unicode is compared through its
/// lossy text. Folders are never filtered out.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::FileFilter;
/// use std::ffi::OsStr;
///
/// let archives = FileFilter::new("archives", "Archives", ["zip", "tar.gz"])?;
/// assert!(archives.matches(OsStr::new("Backup.TAR.GZ")));
/// assert!(!archives.matches(OsStr::new(".zip")));
/// assert!(FileFilter::new("bad", "Bad", ["*.zip"]).is_err());
/// # Ok::<(), pert_ui::FileDialogError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    id: String,
    label: String,
    /// Lowercased, without the leading dot; empty for a filter all files pass.
    extensions: Vec<String>,
}
impl FileFilter {
    /// Creates a filter for names ending in one of `extensions`, each given
    /// without its leading dot (`"txt"`, `"tar.gz"`). `id` identifies the
    /// filter in a [`FileDialogSelection`] and, later, in remembered choices;
    /// `label` is what the selector shows.
    ///
    /// # Errors
    /// [`FileDialogError::InvalidFilter`] for an identity that is not 1 to
    /// [`FileDialogRequest::MAX_KEY_CHARS`] characters, is only whitespace
    /// or holds a control character; for a blank label; without any
    /// extension; and for an extension that is empty, starts or ends with a
    /// dot, or holds `/`, `\`, `*`, `:`, whitespace or a control character.
    /// A label that is blank or holds a control character is refused too.
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        extensions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, FileDialogError> {
        let extensions: Vec<String> = extensions
            .into_iter()
            .map(|extension| extension.into().to_lowercase())
            .collect();
        let valid = |extension: &String| {
            !extension.is_empty()
                && !extension.starts_with('.')
                && !extension.ends_with('.')
                && !extension.chars().any(|c| {
                    matches!(c, '/' | '\\' | '*' | ':') || c.is_whitespace() || c.is_control()
                })
        };
        if extensions.is_empty() || !extensions.iter().all(valid) {
            return Err(FileDialogError::InvalidFilter);
        }
        Self::build(id.into(), label.into(), extensions)
    }
    /// Creates a filter every file passes, such as "All files".
    ///
    /// # Errors
    /// [`FileDialogError::InvalidFilter`] for an invalid identity, or a label
    /// that is blank or holds a control character, as [`Self::new`].
    pub fn all(id: impl Into<String>, label: impl Into<String>) -> Result<Self, FileDialogError> {
        Self::build(id.into(), label.into(), Vec::new())
    }
    fn build(id: String, label: String, extensions: Vec<String>) -> Result<Self, FileDialogError> {
        if !valid_key(&id) || label.trim().is_empty() || label.chars().any(char::is_control) {
            return Err(FileDialogError::InvalidFilter);
        }
        Ok(Self {
            id,
            label,
            extensions,
        })
    }
    /// The filter's identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// The text the selector shows.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    /// The extensions, lowercased and without their leading dots; empty for
    /// a filter every file passes.
    #[must_use]
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }
    /// Whether a file called `name` passes the filter.
    #[must_use]
    pub fn matches(&self, name: &OsStr) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        let name = name.to_string_lossy().to_lowercase();
        self.extensions.iter().any(|extension| {
            name.len() > extension.len() + 1
                && name.ends_with(extension.as_str())
                && name[..name.len() - extension.len()].ends_with('.')
        })
    }
}

/// The configuration of one file dialog session, given to
/// [`FileDialogController::open`].
///
/// It is plain data, fixed for the session it opens: what is asked for, the
/// texts, the file types offered and where to start. Builders replace earlier
/// values. The title and the confirm caption are fixed strings rather than
/// bindings, because a request is meant to travel, later also to an
/// operating-system dialog; a text that is not given comes from the installed
/// string table and follows a later change of it.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{FileDialogMode, FileDialogRequest, FileFilter};
///
/// let request = FileDialogRequest::new(FileDialogMode::SaveFile)
///     .title("Export drawing")
///     .filters([
///         FileFilter::new("png", "PNG image", ["png"])?,
///         FileFilter::new("svg", "SVG drawing", ["svg"])?,
///     ])?
///     .suggested_name("drawing")?;
/// assert_eq!(request.mode(), FileDialogMode::SaveFile);
/// # Ok::<(), pert_ui::FileDialogError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct FileDialogRequest {
    mode: FileDialogMode,
    title: Option<String>,
    accept_label: Option<String>,
    context: Option<String>,
    filters: Vec<FileFilter>,
    start: Option<PathBuf>,
    name: String,
    append_extension: bool,
}
impl FileDialogRequest {
    /// The most characters a context key or a filter identity can have.
    pub const MAX_KEY_CHARS: usize = 64;

    /// Creates a request with the mode's default texts, no file types, no
    /// suggested name, starting in the controller's first root.
    pub fn new(mode: FileDialogMode) -> Self {
        Self {
            mode,
            title: None,
            accept_label: None,
            context: None,
            filters: Vec::new(),
            start: None,
            name: String::new(),
            append_extension: true,
        }
    }
    /// Names the dialog. Without it the title is the mode's text from the
    /// string table.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    /// Captions the confirm button, replacing every default caption,
    /// including [`FileDialogMode::ChooseFolder`]'s, which names the folder
    /// it will accept. The question about replacing a file keeps its own
    /// caption.
    pub fn accept_label(mut self, label: impl Into<String>) -> Self {
        self.accept_label = Some(label.into());
        self
    }
    /// Names the purpose of the dialog, such as `"open-project"`. It is
    /// carried for remembering where the user was, per purpose; this version
    /// of the dialog validates it and does not use it yet.
    ///
    /// # Errors
    /// [`FileDialogError::InvalidContext`] unless the key is 1 to
    /// [`Self::MAX_KEY_CHARS`] characters, not only whitespace and free of
    /// control characters.
    pub fn context(mut self, key: impl Into<String>) -> Result<Self, FileDialogError> {
        let key = key.into();
        if !valid_key(&key) {
            return Err(FileDialogError::InvalidContext);
        }
        self.context = Some(key);
        Ok(self)
    }
    /// Offers file types. The first is selected when the session starts, and
    /// the selected one restricts which files the list shows. Without
    /// filters, the default, every file shows and the dialog has no selector.
    /// [`FileDialogMode::ChooseFolder`] ignores them.
    ///
    /// # Errors
    /// [`FileDialogError::DuplicateFilter`] when two filters share an
    /// identity.
    pub fn filters(
        mut self,
        filters: impl IntoIterator<Item = FileFilter>,
    ) -> Result<Self, FileDialogError> {
        let filters: Vec<FileFilter> = filters.into_iter().collect();
        let mut seen = BTreeSet::new();
        for filter in &filters {
            if !seen.insert(filter.id()) {
                return Err(FileDialogError::DuplicateFilter(filter.id().to_owned()));
            }
        }
        self.filters = filters;
        Ok(self)
    }
    /// The folder the session starts in, instead of the controller's first
    /// root. It need not be inside a root.
    pub fn start_folder(mut self, folder: impl Into<PathBuf>) -> Self {
        self.start = Some(folder.into());
        self
    }
    /// The name field's initial text in [`FileDialogMode::SaveFile`]; other
    /// modes ignore it.
    ///
    /// # Errors
    /// [`FileDialogError::InvalidName`] unless `name` is one file name: not
    /// empty, one normal path component (so not `.` or `..`), and without
    /// `/` or a control character, or on Windows `\` or `:`.
    pub fn suggested_name(mut self, name: impl Into<String>) -> Result<Self, FileDialogError> {
        let name = name.into();
        if !valid_name(&name) {
            return Err(FileDialogError::InvalidName);
        }
        self.name = name;
        Ok(self)
    }
    /// Whether [`FileDialogMode::SaveFile`] completes a typed name that has
    /// no extension with the selected filter's first one: with the filter
    /// `txt`, `report` becomes `report.txt` and `.env` becomes `.env.txt`,
    /// while `report.`, `a.tar.gz` and `photo.png` stay as typed. On by
    /// default.
    pub fn append_extension(mut self, append: bool) -> Self {
        self.append_extension = append;
        self
    }
    /// What the request asks for.
    #[must_use]
    pub fn mode(&self) -> FileDialogMode {
        self.mode
    }
    /// The title, when the request gives one.
    #[must_use]
    pub fn title_text(&self) -> Option<&str> {
        self.title.as_deref()
    }
    /// The confirm caption, when the request gives one.
    #[must_use]
    pub fn accept_text(&self) -> Option<&str> {
        self.accept_label.as_deref()
    }
    /// The context key, when the request gives one.
    #[must_use]
    pub fn context_key(&self) -> Option<&str> {
        self.context.as_deref()
    }
    /// The file types offered, in order.
    #[must_use]
    pub fn filter_list(&self) -> &[FileFilter] {
        &self.filters
    }
    /// The starting folder, when the request gives one.
    #[must_use]
    pub fn start(&self) -> Option<&Path> {
        self.start.as_deref()
    }
    /// The suggested name; empty without one.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Whether a typed name without an extension is completed.
    #[must_use]
    pub fn appends_extension(&self) -> bool {
        self.append_extension
    }
}

/// What the user chose in a file dialog: one or more paths and the file type
/// that was selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDialogSelection {
    paths: Vec<PathBuf>,
    filter: Option<String>,
}
impl FileDialogSelection {
    /// The chosen paths; never empty. Several files are in the order the
    /// list showed them. The path of a listed entry is the one its listing
    /// delivered, unchanged; a new save target is the dialog's folder joined
    /// with the typed name.
    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
    /// The chosen paths, owned.
    #[must_use]
    pub fn into_paths(self) -> Vec<PathBuf> {
        self.paths
    }
    /// The identity of the file type selected when the user confirmed, or
    /// `None` when the request offered none. A save dialog offering several
    /// formats learns the chosen one here.
    #[must_use]
    pub fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }
}

/// Why a file dialog session ended without being shown to its end.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FileDialogFailure {
    /// The dialog's surface could not be shown: another modal occupies its
    /// scope, or there is no room for it. The second can also end a session
    /// that was visible.
    Surface(SurfaceDismissal),
    /// The session was reserved but its starting state could not be written
    /// or its first listing requested; [`FileDialogController::open`]
    /// returned the same error.
    Start(RuntimeError),
}

/// How a file dialog session ended. Retained by the controller until
/// [`FileDialogController::take_outcome`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FileDialogOutcome {
    /// The user confirmed a choice.
    Accepted(FileDialogSelection),
    /// The user declined: the cancel button, Escape or
    /// [`FileDialogController::cancel`].
    Cancelled,
    /// The session ended without a response, for example because its owner
    /// was removed or hidden.
    Dismissed(SurfaceDismissal),
    /// The session could not be shown or started.
    Failed(FileDialogFailure),
}

/// Whether a file dialog has a session; from
/// [`FileDialogController::status`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogStatus {
    /// No session is open and no outcome waits.
    Idle,
    /// A session is open: requested or showing.
    Open,
    /// A session ended and its outcome waits for
    /// [`FileDialogController::take_outcome`].
    Finished,
}

/// Why confirm was refused in place; the status line shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Unusable,
    NotAllFiles,
    InvalidName,
    AmbiguousName,
}
impl Refusal {
    fn role(self) -> StringRole {
        match self {
            Self::Unusable => StringRole::FileDialogUnusable,
            Self::NotAllFiles => StringRole::FileDialogNotAllFiles,
            Self::InvalidName => StringRole::FileDialogInvalidName,
            Self::AmbiguousName => StringRole::FileDialogAmbiguousName,
        }
    }
}

/// What confirm does in the dialog's present state.
#[derive(Clone, Debug, PartialEq)]
enum Decision {
    /// The confirm button is disabled.
    Unavailable,
    Refuse(Refusal),
    Navigate(PathBuf),
    Accept(Vec<PathBuf>),
    /// Ask before replacing this existing file.
    Ask(PathBuf),
}

fn is_folder(entry: &DirectoryEntry) -> bool {
    entry.opens_as_directory()
}
fn is_file(entry: &DirectoryEntry) -> bool {
    entry.error().is_none()
        && matches!(
            entry.kind(),
            EntryKind::File | EntryKind::Symlink(LinkTarget::File)
        )
}

/// The state confirm is judged from.
struct Facts<'a> {
    mode: FileDialogMode,
    question: Option<&'a PathBuf>,
    listing: &'a CurrentListing,
    current: &'a Path,
    /// The shown entry on the active row.
    active: Option<&'a DirectoryEntry>,
    /// The shown entries that are selected, in display order.
    selected: Vec<&'a DirectoryEntry>,
    name: &'a str,
    /// The listed file the name field's text was copied from, while it stands.
    origin: Option<&'a DirectoryEntry>,
    filter: Option<&'a FileFilter>,
    append_extension: bool,
}
impl Facts<'_> {
    fn decide(&self) -> Decision {
        if let Some(candidate) = self.question {
            return Decision::Accept(vec![candidate.clone()]);
        }
        let CurrentListing::Loaded(listing) = self.listing else {
            return Decision::Unavailable;
        };
        match self.mode {
            FileDialogMode::OpenFile => match self.active {
                None => Decision::Unavailable,
                Some(entry) if is_folder(entry) => Decision::Navigate(entry.path().to_owned()),
                Some(entry) if is_file(entry) => Decision::Accept(vec![entry.path().to_owned()]),
                Some(_) => Decision::Refuse(Refusal::Unusable),
            },
            FileDialogMode::OpenFiles => match self.selected.as_slice() {
                [] => Decision::Unavailable,
                entries if entries.iter().all(|entry| is_file(entry)) => Decision::Accept(
                    entries
                        .iter()
                        .map(|entry| entry.path().to_owned())
                        .collect(),
                ),
                [entry] if is_folder(entry) => Decision::Navigate(entry.path().to_owned()),
                _ => Decision::Refuse(Refusal::NotAllFiles),
            },
            FileDialogMode::ChooseFolder => match self.active {
                Some(entry) if is_folder(entry) => Decision::Accept(vec![entry.path().to_owned()]),
                _ => Decision::Accept(vec![self.current.to_owned()]),
            },
            FileDialogMode::SaveFile => self.save(listing),
        }
    }
    fn save(&self, listing: &[DirectoryEntry]) -> Decision {
        if self.name.is_empty() {
            return Decision::Unavailable;
        }
        let existing = |entry: &DirectoryEntry| {
            if is_folder(entry) {
                Decision::Navigate(entry.path().to_owned())
            } else if is_file(entry) {
                Decision::Ask(entry.path().to_owned())
            } else {
                Decision::Refuse(Refusal::Unusable)
            }
        };
        if let Some(origin) = self.origin {
            return existing(origin);
        }
        if !valid_name(self.name) {
            return Decision::Refuse(Refusal::InvalidName);
        }
        let named = |name: &str| {
            let name = OsStr::new(name);
            let mut found = listing.iter().filter(move |entry| entry.name() == name);
            (found.next(), found.next().is_some())
        };
        if let (Some(entry), false) = named(self.name)
            && is_folder(entry)
        {
            return Decision::Navigate(entry.path().to_owned());
        }
        let extension = self
            .filter
            .and_then(|filter| filter.extensions().first())
            .filter(|_| self.append_extension && Path::new(self.name).extension().is_none());
        let resolved = match extension {
            Some(extension) => format!("{}.{extension}", self.name),
            None => self.name.to_owned(),
        };
        match named(&resolved) {
            // The completed name is judged as the typed one was.
            (None, _) if !valid_name(&resolved) => Decision::Refuse(Refusal::InvalidName),
            (None, _) => Decision::Accept(vec![self.current.join(resolved)]),
            (Some(_), true) => Decision::Refuse(Refusal::AmbiguousName),
            (Some(entry), false) => existing(entry),
        }
    }
}

/// The session's configuration as the mounted dialog reads it. Compared by
/// allocation: every session installs a new one.
#[derive(Clone)]
struct Config(Rc<FileDialogRequest>);
impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// A listed file the name field's text was copied from.
struct Origin {
    text: String,
    path: PathBuf,
}

struct Inner {
    /// Distinguishes this controller's mounted content from another's.
    id: u64,
    dialog: DialogController,
    browser: FileBrowserController,
    first_root: PathBuf,
    config: Signal<Config>,
    /// The selected filter's identity; empty without filters.
    filter: Signal<String>,
    name: Signal<String>,
    message: Signal<Option<Refusal>>,
    question: Signal<Option<PathBuf>>,
    origin: RefCell<Option<Origin>>,
    /// The accepted selection, between acceptance and `take_outcome`.
    selection: RefCell<Option<FileDialogSelection>>,
    /// Why the session failed to start, between `open` and `take_outcome`.
    failure: RefCell<Option<RuntimeError>>,
    status: ReadSignal<FileDialogStatus>,
    texts: Texts,
    can_confirm: ReadSignal<bool>,
}

/// The texts the mounted dialog shows, derived from the session and the
/// installed string table.
#[derive(Clone)]
struct Texts {
    title: ReadSignal<String>,
    accept: ReadSignal<String>,
    cancel: ReadSignal<String>,
    message: ReadSignal<String>,
    question: ReadSignal<String>,
    name_label: ReadSignal<String>,
    type_label: ReadSignal<String>,
    choices: ReadSignal<Vec<crate::Choice<String>>>,
}

/// Shows a path's last component, or the whole path when it has none.
fn display_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// The shown entry on the active row, and the selected shown entries in
/// display order. Keys of hidden entries and placeholder rows name no shown
/// entry and are left out.
fn chosen<'a>(
    browser: &FileBrowserController,
    shown: &'a [DirectoryEntry],
    active: Option<&str>,
    selected: &BTreeSet<String>,
) -> (Option<&'a DirectoryEntry>, Vec<&'a DirectoryEntry>) {
    let active = active
        .and_then(|key| browser.path_of(key))
        .and_then(|path| shown.iter().find(|entry| entry.path() == path));
    let paths: BTreeSet<PathBuf> = selected
        .iter()
        .filter_map(|key| browser.path_of(key))
        .collect();
    let selected = shown
        .iter()
        .filter(|entry| paths.contains(entry.path()))
        .collect();
    (active, selected)
}

/// Application-owned state of a file dialog: its session, the browser it
/// shows and the outcome it retains.
///
/// Create it once, with the roots and the request callback a
/// [`FileBrowserController`] takes, mount a [`FileDialog`] that uses it, and
/// call [`Self::open`] with a [`FileDialogRequest`] whenever the user should
/// choose something. Creating it asks the host for nothing; each session
/// requests the listing of its starting folder. Hand the host's
/// [`DirectoryResponse`]s to [`Self::deliver`].
///
/// # Sessions
///
/// A session ends exactly once and the first ending wins, so accepting and
/// then removing the dialog in one input stays accepted. The outcome belongs
/// to the controller, not to a mounted component: read [`Self::status`] where
/// the application reacts and call [`Self::take_outcome`], which returns the
/// outcome once. [`Self::open`] is refused as
/// [`FileDialogError::Busy`] during a session, until its outcome is taken,
/// and after an ending until the mounted dialog has observed it in a flush.
/// After the runtime has shut down nothing can be read, so take an outcome
/// the application must act on before then.
///
/// A session opened while no [`FileDialog`] uses the controller is reserved
/// and shows nothing until one mounts or [`Self::cancel`] ends it. Removing
/// the mounted dialog, or logically hiding it or an ancestor, ends an open
/// session as [`FileDialogOutcome::Dismissed`]; a natively hidden window only
/// suspends it. A listing that fails is shown in the dialog and ends nothing.
///
/// # What confirm accepts
///
/// Confirm is the confirm button, Enter in the name field or
/// [`Self::accept`]; activating a file (Enter or a double click) confirms
/// that file. It is unavailable while the shown folder is loading or failed.
/// An entry counts when the list shows it: one hidden by the type filter or
/// the filter text is ignored, as are the list's placeholder rows.
///
/// - [`FileDialogMode::OpenFile`]: the active entry. A file is accepted, a
///   folder opened; anything else (a broken link, a device, an unreadable
///   entry) is refused with a message and the session continues.
/// - [`FileDialogMode::OpenFiles`]: the selected entries. All files are
///   accepted in display order; one folder alone is opened; any other mix is
///   refused. Activating a file accepts that file alone.
/// - [`FileDialogMode::ChooseFolder`]: the active folder, or without one the
///   folder shown.
/// - [`FileDialogMode::SaveFile`]: the name field. A name copied from a
///   listed file by making it active stands for exactly that file. A typed
///   name must be one file name; the name of a listed folder opens it;
///   otherwise the name is completed as
///   [`FileDialogRequest::append_extension`] describes and looked up in the
///   folder's unfiltered listing. No entry: the folder joined with the name
///   is accepted. A file: the dialog asks before replacing it, and Replace
///   accepts that file's own path. An unusable entry, or a name several
///   entries share, is refused.
///
/// The listing may be out of date, so an accepted path says what the user
/// chose, not what the filesystem holds.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{
///     DirectoryEntry, Element, EntryKind, FileDialog, FileDialogController, FileDialogMode,
///     FileDialogOutcome, FileDialogRequest, FileDialogStatus, Label, Runtime, Viewport,
/// };
/// use std::{cell::RefCell, path::PathBuf, rc::Rc};
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(900.0, 700.0, 1.0)?)?;
/// // A real host sends requests to a worker; this one keeps them.
/// let requests = Rc::new(RefCell::new(Vec::new()));
/// let queue = requests.clone();
/// let dialog = FileDialogController::new(&ui, ["/projects"], move |request| {
///     queue.borrow_mut().push(request)
/// })?;
/// ui.mount_tree(Element::new(
///     "open",
///     FileDialog::in_window(&ui, &dialog, Element::new("view", Label::new("Document"))),
/// ))?;
/// assert!(requests.borrow().is_empty());
///
/// dialog.open(FileDialogRequest::new(FileDialogMode::ChooseFolder))?;
/// ui.flush(ui.now())?;
/// let request = requests.borrow_mut().remove(0);
/// dialog.deliver(request.respond(Ok(vec![
///     DirectoryEntry::new("/projects/demo", EntryKind::Directory),
/// ])))?;
///
/// // The user confirms the folder shown.
/// assert!(dialog.accept()?);
/// assert_eq!(dialog.status().get()?, FileDialogStatus::Finished);
/// let Some(FileDialogOutcome::Accepted(selection)) = dialog.take_outcome()? else {
///     panic!("the session was accepted");
/// };
/// assert_eq!(selection.paths(), [PathBuf::from("/projects")]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
#[must_use]
pub struct FileDialogController(Rc<Inner>);
impl std::fmt::Debug for FileDialogController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileDialogController")
            .field("status", &self.0.status.get())
            .finish_non_exhaustive()
    }
}
impl FileDialogController {
    /// Creates an idle controller whose browser starts at `roots`, in order.
    /// It requests no listing until a session opens.
    ///
    /// `request` runs on the UI thread whenever the dialog needs a directory
    /// listed: during [`Self::open`] and this controller's other methods, and
    /// during the dialog's update. It must return promptly and must not
    /// block, perform I/O, write signals or call this controller; hand the
    /// request to a worker and deliver its answer with [`Self::deliver`].
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner).
    ///
    /// # Errors
    /// [`FileDialogError::NoRoots`] without a root,
    /// [`FileDialogError::DuplicateRoot`] for a root given twice, and the
    /// runtime's refusal to create state as [`FileDialogError::Runtime`].
    pub fn new(
        ui: &impl crate::StateOwner,
        roots: impl IntoIterator<Item = impl Into<PathBuf>>,
        request: impl FnMut(DirectoryRequest) + 'static,
    ) -> Result<Self, FileDialogError> {
        thread_local! {
            static NEXT_ID: Cell<u64> = const { Cell::new(0) };
        }
        let roots: Vec<PathBuf> = roots.into_iter().map(Into::into).collect();
        let first_root = roots.first().cloned().ok_or(FileDialogError::NoRoots)?;
        let browser = FileBrowserController::new_idle(
            ui,
            ListSelection::new(ui, SelectionMode::Single),
            roots,
            request,
        )?;
        let dialog = DialogController::new(ui);
        let config = ui.signal(Config(Rc::new(FileDialogRequest::new(
            FileDialogMode::OpenFile,
        ))));
        let filter = ui.signal(String::new());
        let name = ui.signal(String::new());
        let message = ui.signal(None::<Refusal>);
        let question = ui.signal(None::<PathBuf>);
        let status = dialog.status().map(|status| match status {
            DialogStatus::Idle => FileDialogStatus::Idle,
            DialogStatus::Open => FileDialogStatus::Open,
            DialogStatus::Finished(_) => FileDialogStatus::Finished,
        })?;
        let strings = ui.ui_settings().strings_binding()?;
        let text = |role: StringRole| strings.map(move |table| table.text(role).to_owned());
        let selection = browser.selection();
        let choosing = browser.clone();
        let accept = ReadSignal::combine(
            (
                &config,
                &strings,
                &question,
                &browser.shown_entries(),
                &selection.active(),
                &browser.current(),
            ),
            move |(config, strings, question, shown, active, current)| {
                if question.is_some() {
                    return strings.text(StringRole::FileDialogReplace).to_owned();
                }
                if let Some(label) = &config.0.accept_label {
                    return label.clone();
                }
                match config.0.mode {
                    FileDialogMode::OpenFile | FileDialogMode::OpenFiles => {
                        strings.text(StringRole::FileDialogOpen).to_owned()
                    }
                    FileDialogMode::SaveFile => strings.text(StringRole::FileDialogSave).to_owned(),
                    FileDialogMode::ChooseFolder => {
                        let (active, _) =
                            chosen(&choosing, shown, active.as_deref(), &BTreeSet::new());
                        let folder = active
                            .filter(|entry| is_folder(entry))
                            .map_or(current.as_path(), DirectoryEntry::path);
                        strings.format(UiMessage::FileDialogChoose {
                            name: &display_name(folder),
                        })
                    }
                }
            },
        )?;
        let title = ReadSignal::combine((&config, &strings), |(config, strings)| {
            config.0.title.clone().unwrap_or_else(|| {
                strings
                    .text(match config.0.mode {
                        FileDialogMode::OpenFile | FileDialogMode::OpenFiles => {
                            StringRole::FileDialogOpen
                        }
                        FileDialogMode::ChooseFolder => StringRole::FileDialogChooseFolder,
                        FileDialogMode::SaveFile => StringRole::FileDialogSave,
                    })
                    .to_owned()
            })
        })?;
        let cancel = ReadSignal::combine((&question, &strings), |(question, strings)| {
            strings
                .text(if question.is_some() {
                    StringRole::FileDialogBack
                } else {
                    StringRole::DialogCancel
                })
                .to_owned()
        })?;
        let message_text = ReadSignal::combine((&message, &strings), |(message, strings)| {
            message.map_or_else(String::new, |refusal| {
                strings.text(refusal.role()).to_owned()
            })
        })?;
        let question_text = ReadSignal::combine((&question, &strings), |(question, strings)| {
            question.as_ref().map_or_else(String::new, |candidate| {
                strings.format(UiMessage::FileDialogReplaceQuestion {
                    name: &display_name(candidate),
                })
            })
        })?;
        let choices = config.map(|config: &Config| {
            config
                .0
                .filters
                .iter()
                .map(|filter| crate::Choice::new(filter.id().to_owned(), filter.label()))
                .collect::<Vec<_>>()
        })?;
        let texts = Texts {
            title,
            accept,
            cancel,
            message: message_text,
            question: question_text,
            name_label: text(StringRole::FileDialogName)?,
            type_label: text(StringRole::FileDialogType)?,
            choices,
        };
        let judging = browser.clone();
        // Whether confirm is available, by the first checks of
        // `Facts::decide`: it never needs the listing searched for a name.
        let can_confirm = ReadSignal::combine(
            (
                &status,
                &question,
                &config,
                &browser.listing(),
                &browser.shown_entries(),
                &selection.active(),
                &selection.selected(),
                &name,
            ),
            move |(status, question, config, listing, shown, active, selected, name)| {
                if *status != FileDialogStatus::Open {
                    return false;
                }
                if question.is_some() {
                    return true;
                }
                if !matches!(listing, CurrentListing::Loaded(_)) {
                    return false;
                }
                match config.0.mode {
                    FileDialogMode::ChooseFolder => true,
                    FileDialogMode::SaveFile => !name.is_empty(),
                    FileDialogMode::OpenFile => {
                        chosen(&judging, shown, active.as_deref(), &BTreeSet::new())
                            .0
                            .is_some()
                    }
                    FileDialogMode::OpenFiles => {
                        !chosen(&judging, shown, None, selected).1.is_empty()
                    }
                }
            },
        )?;
        let controller = Self(Rc::new(Inner {
            id: NEXT_ID.with(|next| next.replace(next.get().wrapping_add(1))),
            dialog,
            browser,
            first_root,
            config,
            filter,
            name,
            message,
            question,
            origin: RefCell::new(None),
            selection: RefCell::new(None),
            failure: RefCell::new(None),
            status,
            texts,
            can_confirm,
        }));
        // The browser this controller owns holds it only weakly.
        let weak = Rc::downgrade(&controller.0);
        controller
            .0
            .browser
            .on_open(move |_, entry| match weak.upgrade() {
                Some(inner) => Self(inner).activated(entry),
                None => Ok(()),
            });
        Ok(controller)
    }

    /// Starts a session configured by `request`.
    ///
    /// The session is reserved first, and a refusal there changes nothing
    /// and requests nothing. Then the session's state is written and the
    /// browser navigates to the starting folder
    /// ([`FileDialogRequest::start_folder`], else the first root), which
    /// hands the host its listing request before this call returns. Should
    /// one of those steps fail after the reservation, nothing is rolled back:
    /// the session ends as [`FileDialogFailure::Start`] and the same error is
    /// returned.
    ///
    /// # Errors
    /// [`FileDialogError::Busy`] as described on the type;
    /// [`FileDialogError::Runtime`] for the runtime's refusal, such as
    /// `Closed` after shutdown, a phase write error inside an update, layout
    /// or paint, or `Exhausted` when request generations have run out.
    pub fn open(&self, request: FileDialogRequest) -> Result<(), FileDialogError> {
        self.0.dialog.open().map_err(|error| match error {
            RuntimeError::DialogBusy => FileDialogError::Busy,
            error => FileDialogError::Runtime(error),
        })?;
        *self.0.selection.borrow_mut() = None;
        *self.0.failure.borrow_mut() = None;
        *self.0.origin.borrow_mut() = None;
        if let Err(error) = self.start(request) {
            *self.0.failure.borrow_mut() = Some(error.clone());
            // The reservation stands, so the session ends rather than vanishes.
            let _ = self.0.dialog.cancel();
            return Err(error.into());
        }
        Ok(())
    }
    /// Writes a reserved session's starting state and requests its folder.
    fn start(&self, request: FileDialogRequest) -> Result<(), RuntimeError> {
        let inner = &self.0;
        let selection = inner.browser.selection();
        selection.set_mode(if request.mode == FileDialogMode::OpenFiles {
            SelectionMode::Multiple
        } else {
            SelectionMode::Single
        })?;
        selection.clear()?;
        inner.browser.filter().set(String::new())?;
        // A folder is not of a file type, so that mode selects none.
        inner.filter.set(
            request
                .filters
                .first()
                .filter(|_| request.mode != FileDialogMode::ChooseFolder)
                .map_or_else(String::new, |filter| filter.id().to_owned()),
        )?;
        inner
            .name
            .set(if request.mode == FileDialogMode::SaveFile {
                request.name.clone()
            } else {
                String::new()
            })?;
        inner.message.set(None)?;
        inner.question.set(None)?;
        let start = request
            .start
            .clone()
            .unwrap_or_else(|| inner.first_root.clone());
        inner.config.set(Config(Rc::new(request)))?;
        self.apply_predicate()?;
        inner.browser.navigate(start)
    }
    /// The file type selected in the session, if it offers any.
    fn selected_filter(&self) -> Result<Option<FileFilter>, RuntimeError> {
        let id = self.0.filter.get()?;
        let config = self.0.config.get()?;
        Ok(config
            .0
            .filters
            .iter()
            .find(|filter| filter.id() == id)
            .cloned())
    }
    /// Shows the files of the selected type, or folders only when a folder
    /// is asked for.
    fn apply_predicate(&self) -> Result<(), RuntimeError> {
        let mode = self.0.config.get()?.0.mode;
        let filter = self.selected_filter()?;
        self.0.browser.set_entry_predicate(move |entry| {
            is_folder(entry)
                || (mode != FileDialogMode::ChooseFolder
                    && filter
                        .as_ref()
                        .is_none_or(|filter| filter.matches(entry.name())))
        })
    }

    /// Applies the host's answer to one of the dialog's listing requests, as
    /// [`FileBrowserController::deliver`] does: a response is applied while
    /// it answers this controller's latest request for a directory the
    /// browser still wants, and is otherwise [`ListingDelivery::Stale`]. A
    /// response that arrives after its session ended is ordinary browser
    /// data under that rule; it never ends, accepts or moves a session.
    ///
    /// # Errors
    /// Same as [`FileBrowserController::deliver`].
    pub fn deliver(&self, response: DirectoryResponse) -> Result<ListingDelivery, RuntimeError> {
        self.0.browser.deliver(response)
    }

    /// Whether a session is open or an outcome waits.
    #[must_use]
    pub fn status(&self) -> ReadSignal<FileDialogStatus> {
        self.0.status.clone()
    }

    /// The folder the dialog shows: the first root before the first session.
    #[must_use]
    pub fn current(&self) -> ReadSignal<PathBuf> {
        self.0.browser.current()
    }

    /// Takes the ended session's outcome and makes the controller idle.
    /// Returns `None` while idle or open. The outcome, with an accepted
    /// selection, is released only when the take succeeds.
    ///
    /// # Errors
    /// The status signal's read and write errors, such as `Closed` after
    /// shutdown or a phase write error; the outcome then stays.
    pub fn take_outcome(&self) -> Result<Option<FileDialogOutcome>, RuntimeError> {
        let Some(outcome) = self.0.dialog.take_outcome()? else {
            return Ok(None);
        };
        let selection = self.0.selection.borrow_mut().take();
        let failure = self.0.failure.borrow_mut().take();
        Ok(Some(match (outcome, selection, failure) {
            (DialogOutcome::Accepted, Some(selection), _) => FileDialogOutcome::Accepted(selection),
            (_, _, Some(error)) => FileDialogOutcome::Failed(FileDialogFailure::Start(error)),
            (DialogOutcome::Failed(reason), ..) => {
                FileDialogOutcome::Failed(FileDialogFailure::Surface(reason))
            }
            (DialogOutcome::Dismissed(reason), ..) => FileDialogOutcome::Dismissed(reason),
            // Acceptance always stores a selection first; without one the
            // session reports that the user chose nothing.
            (DialogOutcome::Cancelled | DialogOutcome::Accepted, ..) => {
                FileDialogOutcome::Cancelled
            }
        }))
    }

    /// Ends an open session as [`FileDialogOutcome::Cancelled`]. Returns
    /// whether a session was open.
    ///
    /// # Errors
    /// The status signal's read and write errors.
    pub fn cancel(&self) -> Result<bool, RuntimeError> {
        self.0.dialog.cancel()
    }

    /// Does what the confirm button does, by the rules on the type, and
    /// returns whether that ended the session. It returns `false` when no
    /// session is open, when confirm is unavailable, and when it refused,
    /// opened a folder or asked about replacing a file.
    ///
    /// # Errors
    /// The runtime's read and write errors, such as a phase write error; a
    /// failed write ends nothing, and an accepted selection is then
    /// discarded with the session still open.
    pub fn accept(&self) -> Result<bool, RuntimeError> {
        let inner = &self.0;
        if inner.status.get()? != FileDialogStatus::Open {
            return Ok(false);
        }
        let config = inner.config.get()?;
        let listing = inner.browser.listing().get()?;
        let shown = inner.browser.shown_entries().get()?;
        let current = inner.browser.current().get()?;
        let selection = inner.browser.selection();
        let active_key = selection.active().get()?;
        let selected_keys = selection.selected().get()?;
        let question = inner.question.get()?;
        let name = inner.name.get()?;
        let filter = self.selected_filter()?;
        let (active, selected) = chosen(
            &inner.browser,
            &shown,
            active_key.as_deref(),
            &selected_keys,
        );
        // A name's origin stands while the field still shows the copied text
        // and the listing still has its path; the entry judged is the
        // listing's present one, which may have become something else.
        let origin_path = inner
            .origin
            .borrow()
            .as_ref()
            .filter(|origin| origin.text == name)
            .map(|origin| origin.path.clone());
        let origin = match (&origin_path, &listing) {
            (Some(path), CurrentListing::Loaded(entries)) => {
                entries.iter().find(|entry| entry.path() == path)
            }
            _ => None,
        };
        let decision = Facts {
            mode: config.0.mode,
            question: question.as_ref(),
            listing: &listing,
            current: &current,
            active,
            selected,
            name: &name,
            origin,
            filter: filter.as_ref(),
            append_extension: config.0.append_extension,
        }
        .decide();
        match decision {
            Decision::Unavailable => Ok(false),
            Decision::Refuse(refusal) => {
                inner.message.set(Some(refusal))?;
                Ok(false)
            }
            Decision::Navigate(folder) => {
                if config.0.mode == FileDialogMode::SaveFile {
                    inner.name.set(String::new())?;
                }
                inner.message.set(None)?;
                inner.browser.navigate(folder)?;
                Ok(false)
            }
            Decision::Ask(candidate) => {
                inner.message.set(None)?;
                inner.question.set(Some(candidate))?;
                Ok(false)
            }
            Decision::Accept(paths) => self.finish(paths),
        }
    }
    /// Ends the session as accepted with `paths`.
    fn finish(&self, paths: Vec<PathBuf>) -> Result<bool, RuntimeError> {
        let filter = self.selected_filter()?.map(|filter| filter.id);
        *self.0.selection.borrow_mut() = Some(FileDialogSelection { paths, filter });
        match self.0.dialog.accept() {
            Ok(true) => Ok(true),
            ended => {
                // The selection and the ended session are kept together.
                *self.0.selection.borrow_mut() = None;
                ended
            }
        }
    }
    /// The browser reported an activated entry that is not a folder.
    fn activated(&self, entry: &DirectoryEntry) -> Result<(), RuntimeError> {
        let inner = &self.0;
        if inner.status.get()? != FileDialogStatus::Open || inner.question.get()?.is_some() {
            return Ok(());
        }
        let mode = inner.config.get()?.0.mode;
        if mode == FileDialogMode::ChooseFolder {
            return Ok(());
        }
        if !is_file(entry) {
            return inner.message.set(Some(Refusal::Unusable));
        }
        if mode == FileDialogMode::SaveFile {
            // The activated entry itself is the candidate, whatever the name
            // field can show of its name.
            self.name_from(entry)?;
            inner.message.set(None)?;
            return inner.question.set(Some(entry.path().to_owned()));
        }
        self.finish(vec![entry.path().to_owned()]).map(|_| ())
    }
    /// Copies a listed file's name into the name field and remembers the
    /// entry as the text's origin.
    ///
    /// A name the field cannot show, one with a control character, is not
    /// copied: the field keeps its text and no origin is recorded.
    fn name_from(&self, entry: &DirectoryEntry) -> Result<(), RuntimeError> {
        let text = entry.name().to_string_lossy().into_owned();
        if text.chars().any(char::is_control) {
            return Ok(());
        }
        self.0.name.set(text.clone())?;
        *self.0.origin.borrow_mut() = Some(Origin {
            text,
            path: entry.path().to_owned(),
        });
        Ok(())
    }
    /// The cancel button: back from the replace question, else cancel.
    fn back_or_cancel(&self) -> Result<(), RuntimeError> {
        if self.0.question.get()?.is_some() {
            return self.0.question.set(None);
        }
        self.cancel().map(|_| ())
    }
    /// The user changed what confirm is judged from: the refusal shown no
    /// longer describes it, a newly selected type filters the list, and in
    /// a save dialog a file that became active names the target.
    fn observed(&self, change: view::Change) -> Result<(), RuntimeError> {
        let inner = &self.0;
        if inner.status.get()? != FileDialogStatus::Open {
            return Ok(());
        }
        if change.filter {
            self.apply_predicate()?;
        }
        // An edit ends a copied name's tie to its file, also when the text
        // later reads the same again; so does leaving the folder.
        let name = inner.name.get()?;
        let edited = change.name
            && inner
                .origin
                .borrow()
                .as_ref()
                .is_some_and(|origin| origin.text != name);
        if edited || change.current {
            *inner.origin.borrow_mut() = None;
        }
        if inner.message.get()?.is_some() {
            inner.message.set(None)?;
        }
        if change.active && inner.config.get()?.0.mode == FileDialogMode::SaveFile {
            let shown = inner.browser.shown_entries().get()?;
            let active = inner.browser.selection().active().get()?;
            let (active, _) = chosen(&inner.browser, &shown, active.as_deref(), &BTreeSet::new());
            if let Some(entry) = active.filter(|entry| is_file(entry)) {
                self.name_from(entry)?;
            }
        }
        Ok(())
    }
}
