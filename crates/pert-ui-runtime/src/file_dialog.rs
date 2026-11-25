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
    GeometryError, LinkTarget, ListSelection, ListingDelivery, ListingErrorKind, ReadSignal,
    RuntimeError, SelectionMode, Signal, StringRole, SurfaceDismissal, TextField, UiMessage,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::rc::Rc;

mod memory;
mod operations;
mod view;
pub use memory::{FileDialogMemory, FileDialogMemoryError};
pub use operations::{
    FileOperation, FileOperationError, FileOperationErrorKind, FileOperationRequest,
    FileOperationResponse, OperationDelivery,
};
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
    /// A place whose label is blank or holds a control character, or whose
    /// folder is the empty path.
    #[error("the place is not valid")]
    InvalidPlace,
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
/// separator, and text the name field accepts ([`TextField::check`]), so the
/// dialog never judges a name the field refused to show. Names a filesystem
/// may still reject are the application's to discover when it writes.
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
        && TextField::check(name).is_ok()
        && matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
}

/// A folder the application offers as a shortcut beside a file dialog's
/// browser, from [`FileDialogController::places`].
///
/// The library has no places of its own: it cannot know the user's home or
/// documents folder, so the application supplies what it knows. The folder
/// need not be inside one of the controller's roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePlace {
    label: String,
    folder: PathBuf,
}
impl FilePlace {
    /// Creates a place shown as `label` that opens `folder`.
    ///
    /// # Errors
    /// [`FileDialogError::InvalidPlace`] for a label that is blank or holds
    /// a control character, and for the empty path.
    pub fn new(
        label: impl Into<String>,
        folder: impl Into<PathBuf>,
    ) -> Result<Self, FileDialogError> {
        let (label, folder) = (label.into(), folder.into());
        if label.trim().is_empty()
            || label.chars().any(char::is_control)
            || folder.as_os_str().is_empty()
        {
            return Err(FileDialogError::InvalidPlace);
        }
        Ok(Self { label, folder })
    }
    /// The text the place is shown as.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    /// The folder the place opens.
    #[must_use]
    pub fn folder(&self) -> &Path {
        &self.folder
    }
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
    /// suggested name and no starting folder or context; see
    /// [`Self::start_folder`] for where such a session starts.
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
    /// Names the purpose of the dialog, such as `"open-project"`. A
    /// controller with a [`FileDialogMemory`] remembers the folder and the
    /// file type per purpose; requests without a context share one unnamed
    /// purpose. Without a memory the key has no effect.
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
    /// Offers file types. The first is selected when the session starts,
    /// unless the controller's [`FileDialogMemory`] remembers another of
    /// them for the request's context, and the selected one restricts which
    /// files the list shows. Without
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
    /// The folder the session starts in. It need not be inside a root. An
    /// explicit starting folder is used as given: it takes precedence over a
    /// folder the controller's [`FileDialogMemory`] remembers, and no other
    /// folder is tried when its listing fails. Without one the session
    /// starts in the remembered folder, else in the controller's first root.
    /// To prefer a remembered folder over a default of the application's,
    /// set this only when [`FileDialogMemory::last_folder`] is `None`.
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
    remembered: bool,
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
    /// `None` when the request offered none and for
    /// [`FileDialogMode::ChooseFolder`], which shows no selector. A save
    /// dialog offering several formats learns the chosen one here.
    #[must_use]
    pub fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }
    /// Whether the controller's [`FileDialogMemory`] took the folder the
    /// dialog showed. It is `false` without a memory, and with one when the
    /// folder's path cannot be stored (see the memory's rules for paths); a
    /// host that persists the memory can then tell the user that this folder
    /// will not be offered again.
    #[must_use]
    pub const fn folder_remembered(&self) -> bool {
        self.remembered
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
#[derive(Clone, Debug, PartialEq, Eq)]
enum Refusal {
    Unusable,
    NotAllFiles,
    InvalidName,
    AmbiguousName,
    /// The host could not create the folder; its message says why.
    CreateFailed(String),
}
impl Refusal {
    fn text(&self, strings: &crate::UiStrings) -> String {
        let role = match self {
            Self::Unusable => StringRole::FileDialogUnusable,
            Self::NotAllFiles => StringRole::FileDialogNotAllFiles,
            Self::InvalidName => StringRole::FileDialogInvalidName,
            Self::AmbiguousName => StringRole::FileDialogAmbiguousName,
            Self::CreateFailed(reason) => {
                return strings.format(UiMessage::FileDialogCreateFailed { reason });
            }
        };
        strings.text(role).to_owned()
    }
}

/// Where the user is in creating a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Creating {
    /// The name is being typed.
    Naming,
    /// The host has the request and has not answered.
    Pending,
}

/// The handler the host takes file operations with.
type OperationHandler = Box<dyn FnMut(FileOperationRequest) -> Result<(), FileOperationError>>;

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
    memory: RefCell<Option<FileDialogMemory>>,
    places: Signal<Vec<FilePlace>>,
    /// The memory's recent folders as the session found them.
    recent: Signal<Vec<PathBuf>>,
    /// `Some` while the user creates a folder.
    creating: Signal<Option<Creating>>,
    folder_name: Signal<String>,
    /// The host's capability to perform file operations, if it has one. It
    /// is taken out while it runs.
    operations: RefCell<Option<OperationHandler>>,
    /// Whether the host has that capability.
    can_operate: Cell<bool>,
    new_folder: RefCell<crate::Permission>,
    /// Counts changes of the capability and the permission, so the mounted
    /// content reads them again.
    capability: Signal<u64>,
    /// The one operation the dialog waits for.
    operation_requests: crate::RequestTracker<()>,
    /// A created folder to make active when the listing its creation asked
    /// for arrives; shared with the request sink, which notes that request.
    reveal: Rc<RefCell<Option<Reveal>>>,
    /// The memory the open session started from and writes to when accepted.
    session_memory: RefCell<Option<FileDialogMemory>>,
    /// The opening's search for the remembered folder; shared with the
    /// request sink, which notes the request the search waits for.
    search: Rc<RefCell<Option<Search>>>,
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
    folder_label: ReadSignal<String>,
    new_folder: ReadSignal<String>,
    type_label: ReadSignal<String>,
    choices: ReadSignal<Vec<crate::Choice<String>>>,
    shortcuts: ReadSignal<Rc<Shortcuts>>,
}

/// The opening's search for a remembered folder that may be gone: the folder
/// being tried and the one listing request whose answer continues it.
#[derive(Clone, Debug)]
struct Search {
    folder: PathBuf,
    /// The generation of the request the search issued for `folder`; a
    /// request the user's navigation issues later is not the search's.
    generation: Option<u64>,
    /// While the search's own navigation hands out its request.
    capturing: bool,
    /// Whether the first root is `folder` or was tried before it.
    root_tried: bool,
    /// Whether nothing follows `folder`: the first root after the ancestors.
    last: bool,
}

/// A created folder waiting for the one listing that was requested to show
/// it.
#[derive(Clone, Debug)]
struct Reveal {
    created: PathBuf,
    parent: PathBuf,
    generation: Option<u64>,
    /// While the refresh hands out its request.
    capturing: bool,
}

/// What the list beside the browser shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Shortcuts {
    places_heading: String,
    places: Vec<FilePlace>,
    recent_heading: String,
    recent: Vec<PathBuf>,
}
impl Shortcuts {
    fn is_empty(&self) -> bool {
        self.places.is_empty() && self.recent.is_empty()
    }
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
/// # Memory and places
///
/// With a [`FileDialogMemory`] ([`Self::with_memory`]) a session starts in
/// the folder, and with the file type, of the last accepted session of its
/// request's [context](FileDialogRequest::context), unless the request names
/// a [starting folder](FileDialogRequest::start_folder). A remembered folder
/// may be gone: when its listing fails as
/// [`NotFound`](crate::ListingErrorKind::NotFound) or
/// [`NotADirectory`](crate::ListingErrorKind::NotADirectory) the dialog
/// requests its parent instead, then that folder's parent, and after the
/// last ancestor the first root, unless that was among them. Only the answer
/// to the request the search itself made continues it. Any other failure,
/// such as a folder the user may not read, is shown and ends the search, and
/// so does the user opening a folder, the one shown included, so an answer
/// that arrives late never moves the browser. An accepted session is written to
/// the memory before its outcome can be taken; no other ending writes.
///
/// The dialog lists the memory's recent folders, as they were when the
/// session opened, and the application's [places](Self::places) beside the
/// browser; activating one opens its folder.
///
/// # Creating a folder
///
/// The dialog performs no file operation itself. With a handler from
/// [`Self::on_file_operation`] it shows a New folder button, in every mode,
/// once the folder it shows is listed: the user types a name, which must be
/// one file name, and confirm hands the host a [`FileOperationRequest`] for
/// that folder. While the host works, confirm is unavailable, so nothing is
/// sent twice. Back returns to the browser, and so does opening another
/// folder; either makes a later answer stale. The host's answer arrives
/// through [`Self::deliver_operation`]: after a success the folder is listed
/// again and the new folder made active when that listing shows it, and a
/// failure is shown with the name kept. An answer the dialog no longer
/// waits for changes nothing. Starting to name a folder ends the search for
/// a remembered folder. Creating a folder never ends a session and writes
/// nothing to the memory.
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
        let search: Rc<RefCell<Option<Search>>> = Rc::default();
        let reveal: Rc<RefCell<Option<Reveal>>> = Rc::default();
        let (noted, shown, mut request) = (search.clone(), reveal.clone(), request);
        let browser = FileBrowserController::new_idle(
            ui,
            ListSelection::new(ui, SelectionMode::Single),
            roots,
            move |asked: DirectoryRequest| {
                {
                    let mut search = noted.borrow_mut();
                    if let Some(search) = search.as_mut()
                        && search.capturing
                        && asked.directory() == search.folder
                    {
                        search.generation = Some(asked.generation());
                    }
                }
                {
                    let mut reveal = shown.borrow_mut();
                    if let Some(reveal) = reveal.as_mut()
                        && reveal.capturing
                        && asked.directory() == reveal.parent
                    {
                        reveal.generation = Some(asked.generation());
                    }
                }
                request(asked);
            },
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
        let creating = ui.signal(None::<Creating>);
        let folder_name = ui.signal(String::new());
        let capability = ui.signal(0_u64);
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
                &creating,
            ),
            move |(config, strings, question, shown, active, current, creating)| {
                if question.is_some() {
                    return strings.text(StringRole::FileDialogReplace).to_owned();
                }
                if creating.is_some() {
                    return strings.text(StringRole::FileDialogCreate).to_owned();
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
        let cancel = ReadSignal::combine(
            (&question, &creating, &strings),
            |(question, creating, strings)| {
                strings
                    .text(if question.is_some() || creating.is_some() {
                        StringRole::FileDialogBack
                    } else {
                        StringRole::DialogCancel
                    })
                    .to_owned()
            },
        )?;
        let message_text = ReadSignal::combine((&message, &strings), |(message, strings)| {
            message
                .as_ref()
                .map_or_else(String::new, |refusal| refusal.text(strings))
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
        let places = ui.signal(Vec::<FilePlace>::new());
        let recent = ui.signal(Vec::<PathBuf>::new());
        let shortcuts =
            ReadSignal::combine((&places, &recent, &strings), |(places, recent, strings)| {
                Rc::new(Shortcuts {
                    places_heading: strings.text(StringRole::FileDialogPlaces).to_owned(),
                    places: places.clone(),
                    recent_heading: strings.text(StringRole::FileDialogRecent).to_owned(),
                    recent: recent.clone(),
                })
            })?;
        let texts = Texts {
            shortcuts,
            title,
            accept,
            cancel,
            message: message_text,
            question: question_text,
            name_label: text(StringRole::FileDialogName)?,
            folder_label: text(StringRole::FileDialogFolderName)?,
            new_folder: text(StringRole::FileDialogNewFolder)?,
            type_label: text(StringRole::FileDialogType)?,
            choices,
        };
        let judging = browser.clone();
        // The shown paths, rebuilt only when the shown entries change, so a
        // moved selection is judged by lookups.
        let shown_paths = browser.shown_entries().map(|shown| {
            Rc::new(
                shown
                    .iter()
                    .map(|entry| entry.path().to_owned())
                    .collect::<HashSet<PathBuf>>(),
            )
        })?;
        // Whether confirm is available, by the first checks of
        // `Facts::decide`: it never needs the listing searched for a name.
        let can_confirm = ReadSignal::combine(
            (
                &status,
                &question,
                &config,
                &browser.listing(),
                &shown_paths,
                &selection.active(),
                &selection.selected(),
                &name,
                &creating,
                &folder_name,
            ),
            move |(
                status,
                question,
                config,
                listing,
                shown,
                active,
                selected,
                name,
                creating,
                folder_name,
            )| {
                let is_shown = |key: &String| {
                    judging
                        .path_of(key)
                        .is_some_and(|path| shown.contains(&path))
                };
                if *status != FileDialogStatus::Open {
                    return false;
                }
                if question.is_some() {
                    return true;
                }
                // Creating a folder: a name to send, and nothing sent yet.
                match creating {
                    Some(Creating::Naming) => {
                        return !folder_name.is_empty()
                            && matches!(listing, CurrentListing::Loaded(_));
                    }
                    Some(Creating::Pending) => return false,
                    None => {}
                }
                if !matches!(listing, CurrentListing::Loaded(_)) {
                    return false;
                }
                match config.0.mode {
                    FileDialogMode::ChooseFolder => true,
                    FileDialogMode::SaveFile => !name.is_empty(),
                    FileDialogMode::OpenFile => active.as_ref().is_some_and(is_shown),
                    FileDialogMode::OpenFiles => selected.iter().any(is_shown),
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
            creating,
            folder_name,
            operations: RefCell::new(None),
            can_operate: Cell::new(false),
            new_folder: RefCell::new(crate::Permission::from(true)),
            capability,
            operation_requests: crate::RequestTracker::new(),
            reveal,
            memory: RefCell::new(None),
            places,
            recent,
            session_memory: RefCell::new(None),
            search,
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

    /// Gives the controller a memory, replacing an earlier one: sessions
    /// then start where the last accepted session of their context was, and
    /// accepted sessions are recorded in it, by the rules on
    /// [`FileDialogMemory`]. It applies to every clone of the controller and
    /// from the next [`Self::open`]: a session that is open keeps, and when
    /// accepted writes to, the memory it started with. Call it where the
    /// controller is created.
    /// Any number of controllers may share one memory.
    pub fn with_memory(self, memory: &FileDialogMemory) -> Self {
        *self.0.memory.borrow_mut() = Some(memory.clone());
        self
    }

    /// Gives the dialog the host's capability to perform file operations,
    /// replacing an earlier handler; with it the dialog offers a New folder
    /// button. It applies to every clone of the controller.
    ///
    /// `handler` receives each [`FileOperationRequest`] and hands it to
    /// whatever performs it, typically a worker; it must not call back into
    /// this controller. It returns `Err` when it could not take the request,
    /// which the dialog shows at once, the name still in its field. The
    /// answer comes later through [`Self::deliver_operation`].
    ///
    /// # Errors
    /// The runtime's refusal to record the capability, such as a phase write
    /// error or `Closed` after shutdown.
    pub fn on_file_operation(
        self,
        handler: impl FnMut(FileOperationRequest) -> Result<(), FileOperationError> + 'static,
    ) -> Result<Self, RuntimeError> {
        self.0
            .capability
            .update(|revision| *revision = revision.wrapping_add(1))?;
        self.0.can_operate.set(true);
        *self.0.operations.borrow_mut() = Some(Box::new(handler));
        Ok(self)
    }

    /// Restricts creating folders; allowed by default. Restrictions compose:
    /// each call adds to the earlier ones ([`Permission::and`](crate::Permission::and)),
    /// so a later call never lifts an earlier denial. The New folder button
    /// shows only while a handler is installed and the permission reads
    /// true, and the permission is read again when a request is handed to
    /// the host and when its answer arrives: a request is not sent, and an
    /// answer is [`OperationDelivery::Stale`], once it reads false. Revoking
    /// it while the user names a folder, or while the host works on one,
    /// takes the dialog back to its browser. It applies to every clone of
    /// the controller and to a mounted dialog.
    ///
    /// # Errors
    /// The runtime's refusal to record the change, such as a phase write
    /// error or `Closed` after shutdown; the permission is then unchanged.
    pub fn new_folder_permission(
        self,
        permission: impl Into<crate::Permission>,
    ) -> Result<Self, RuntimeError> {
        self.0
            .capability
            .update(|revision| *revision = revision.wrapping_add(1))?;
        let earlier = self.0.new_folder.replace(crate::Permission::from(true));
        *self.0.new_folder.borrow_mut() = earlier.and(permission);
        Ok(self)
    }
    /// Whether the host can create folders and the application allows it.
    pub(super) fn may_create(&self) -> Result<bool, RuntimeError> {
        Ok(self.0.can_operate.get() && self.0.new_folder.borrow().get()?)
    }

    /// Applies the host's answer to a [`FileOperationRequest`].
    ///
    /// The answer is applied while the dialog still waits for that request:
    /// the session it was made in is open, the user has not gone back or
    /// opened another folder, and creating folders is still permitted. A
    /// created folder is then shown: the dialog requests the listing of the
    /// folder again and, when that very listing arrives, makes the new folder
    /// active if the list shows it as a folder. A failure is shown with the
    /// name still in its field. Any other answer is
    /// [`OperationDelivery::Stale`] and changes nothing: it neither selects
    /// nor opens a folder, also when the operation succeeded.
    ///
    /// # Errors
    /// The runtime's read and write errors, such as a phase write error. The
    /// request then stays awaited, so the same response can be delivered
    /// again. `Exhausted` if request generations run out when the folder is
    /// listed again; the answer is applied then, and only the listing is
    /// missing.
    pub fn deliver_operation(
        &self,
        response: FileOperationResponse,
    ) -> Result<OperationDelivery, RuntimeError> {
        let inner = &self.0;
        let ticket = &response.request.ticket;
        if inner.operation_requests.status(ticket) != crate::TicketStatus::Current
            || inner.status.get()? != FileDialogStatus::Open
            || inner.creating.get()? != Some(Creating::Pending)
        {
            return Ok(OperationDelivery::Stale);
        }
        let current = inner.browser.current().get()?;
        if !self.may_create()? || response.request.parent() != current {
            self.end_folder()?;
            return Ok(OperationDelivery::Stale);
        }
        // The writes come first: a refused one leaves the ticket current.
        match &response.result {
            Err(error) => {
                inner.creating.set(Some(Creating::Naming))?;
                inner
                    .message
                    .set(Some(Refusal::CreateFailed(error.message().to_owned())))?;
                inner.operation_requests.complete(ticket);
            }
            Ok(()) => {
                inner.creating.set(None)?;
                inner.message.set(None)?;
                inner.operation_requests.complete(ticket);
                // The listing is the browser's own request, noted so that
                // only its answer points at the new folder.
                *inner.reveal.borrow_mut() = Some(Reveal {
                    created: response.request.operation.target(),
                    parent: current,
                    generation: None,
                    capturing: true,
                });
                let refreshed = inner.browser.refresh();
                let mut reveal = inner.reveal.borrow_mut();
                match reveal.as_mut() {
                    Some(waiting) if refreshed.is_ok() && waiting.generation.is_some() => {
                        waiting.capturing = false;
                    }
                    _ => *reveal = None,
                }
                drop(reveal);
                refreshed?;
            }
        }
        Ok(OperationDelivery::Applied)
    }
    /// The New folder button: asks for the new folder's name. A folder is
    /// created in a folder the dialog has listed, so the button waits for
    /// the listing; pressing it ends the opening's search for a remembered
    /// folder, since the user has chosen to work where the dialog is.
    pub(super) fn begin_folder(&self) -> Result<(), RuntimeError> {
        let inner = &self.0;
        if inner.status.get()? != FileDialogStatus::Open
            || inner.question.get()?.is_some()
            || inner.creating.get()?.is_some()
            || !self.may_create()?
            || !matches!(inner.browser.listing().get()?, CurrentListing::Loaded(_))
        {
            return Ok(());
        }
        inner.folder_name.set(String::new())?;
        inner.message.set(None)?;
        inner.creating.set(Some(Creating::Naming))?;
        *inner.search.borrow_mut() = None;
        Ok(())
    }
    /// Leaves folder creation: a request the host still holds becomes stale.
    pub(super) fn end_folder(&self) -> Result<(), RuntimeError> {
        if self.0.creating.get()?.is_some() {
            self.0.creating.set(None)?;
            self.0.message.set(None)?;
        }
        self.0.operation_requests.cancel(&());
        Ok(())
    }
    /// Confirm while a folder is being named: validates the name and hands
    /// the request to the host. A request already with the host is not sent
    /// again.
    fn create_folder(&self, creating: Creating) -> Result<bool, RuntimeError> {
        let inner = &self.0;
        if creating == Creating::Pending {
            return Ok(false);
        }
        if !self.may_create()? {
            self.end_folder()?;
            return Ok(false);
        }
        let name = inner.folder_name.get()?;
        if name.is_empty() || !matches!(inner.browser.listing().get()?, CurrentListing::Loaded(_)) {
            return Ok(false);
        }
        if !valid_name(&name) {
            inner.message.set(Some(Refusal::InvalidName))?;
            return Ok(false);
        }
        let parent = inner.browser.current().get()?;
        // The writes come first: a refused one issues no ticket.
        inner.message.set(None)?;
        inner.creating.set(Some(Creating::Pending))?;
        let ticket = match inner.operation_requests.issue(()) {
            Ok(ticket) => ticket,
            Err(error) => {
                inner.creating.set(Some(Creating::Naming))?;
                return Err(error);
            }
        };
        let request = FileOperationRequest {
            ticket: ticket.clone(),
            operation: FileOperation::CreateFolder { parent, name },
        };
        // Taken out while it runs, so the handler may install its successor.
        let handler = inner.operations.borrow_mut().take();
        let handed = match handler {
            Some(mut handler) => {
                let handed = handler(request);
                let mut slot = inner.operations.borrow_mut();
                if slot.is_none() {
                    *slot = Some(handler);
                }
                handed
            }
            None => Err(FileOperationError::new(
                FileOperationErrorKind::Unavailable,
                String::new(),
            )),
        };
        // A refusal counts only while this request is still the awaited one.
        if let Err(error) = handed
            && inner.operation_requests.status(&ticket) == crate::TicketStatus::Current
        {
            inner.operation_requests.cancel(&());
            inner.creating.set(Some(Creating::Naming))?;
            inner
                .message
                .set(Some(Refusal::CreateFailed(error.message().to_owned())))?;
        }
        Ok(false)
    }
    /// Makes a created folder active when the listing its creation asked for
    /// was applied and shows it as a folder. Any other listing leaves the
    /// wait alone; that listing ends it whatever it holds, so a failed or
    /// changed folder is shown as it is and nothing is created again.
    fn reveal_created(
        &self,
        directory: &Path,
        generation: u64,
        listed: bool,
    ) -> Result<(), RuntimeError> {
        let inner = &self.0;
        let waiting = {
            let mut reveal = inner.reveal.borrow_mut();
            if !reveal.as_ref().is_some_and(|waiting| {
                waiting.parent == directory && waiting.generation == Some(generation)
            }) {
                return Ok(());
            }
            reveal.take()
        };
        let Some(waiting) = waiting else {
            return Ok(());
        };
        if !listed
            || inner.status.get()? != FileDialogStatus::Open
            || inner.browser.current().get()? != waiting.parent
        {
            return Ok(());
        }
        let is_folder_now = inner
            .browser
            .shown_entries()
            .get()?
            .iter()
            .any(|entry| entry.path() == waiting.created && is_folder(entry));
        if is_folder_now {
            inner.browser.select_entry(&waiting.created)?;
        }
        Ok(())
    }

    /// The places the dialog offers beside its browser, which the
    /// application writes or binds; empty at first. Activating a place opens
    /// its folder. The list is application data: it is not stored in a
    /// [`FileDialogMemory`]. A change shows in an open session as well. A
    /// place given twice, with the same label and folder, is shown once.
    #[must_use]
    pub fn places(&self) -> Signal<Vec<FilePlace>> {
        self.0.places.clone()
    }

    /// Starts a session configured by `request`.
    ///
    /// The session is reserved first, and a refusal there changes nothing
    /// and requests nothing. Then the session's state is written and the
    /// browser navigates to the starting folder
    /// ([`FileDialogRequest::start_folder`], else the folder the memory
    /// remembers for the request's context, else the first root), which
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
        *self.0.search.borrow_mut() = None;
        *self.0.reveal.borrow_mut() = None;
        // An operation of the earlier session is no longer waited for.
        self.0.operation_requests.cancel(&());
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
        // The session keeps the memory it starts from: one given to the
        // controller later is the next session's.
        let memory = inner.memory.borrow().clone();
        (*inner.session_memory.borrow_mut()).clone_from(&memory);
        let context = request.context.as_deref();
        let remembered = memory
            .as_ref()
            .and_then(|memory| memory.last_filter(context));
        // A folder is not of a file type, so that mode selects none. A
        // remembered type the request no longer offers selects the first.
        inner.filter.set(
            request
                .filters
                .iter()
                .find(|filter| Some(filter.id()) == remembered.as_deref())
                .or_else(|| request.filters.first())
                .filter(|_| request.mode != FileDialogMode::ChooseFolder)
                .map_or_else(String::new, |filter| filter.id().to_owned()),
        )?;
        inner.recent.set(
            memory
                .as_ref()
                .map_or_else(Vec::new, FileDialogMemory::recent_folders),
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
        inner.creating.set(None)?;
        // Only a remembered folder is searched for: it may be gone since.
        let remembered = match (&request.start, &memory) {
            (None, Some(memory)) => memory.last_folder(context),
            _ => None,
        };
        let start = request
            .start
            .clone()
            .unwrap_or_else(|| inner.first_root.clone());
        inner.config.set(Config(Rc::new(request)))?;
        self.apply_predicate()?;
        match remembered {
            Some(folder) => {
                let root_tried = folder == inner.first_root;
                self.seek(folder, root_tried, false)
            }
            None => inner.browser.navigate(start),
        }
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
    /// data under that rule; it never ends or accepts a session, and it
    /// moves one only as the search for a remembered folder that the type
    /// describes.
    ///
    /// # Errors
    /// Same as [`FileBrowserController::deliver`].
    pub fn deliver(&self, response: DirectoryResponse) -> Result<ListingDelivery, RuntimeError> {
        let asked = response.request();
        let (directory, generation) = (asked.directory().to_owned(), asked.generation());
        let listed = response.listing().is_ok();
        let missing = response.listing().is_err_and(|error| {
            matches!(
                error.kind(),
                ListingErrorKind::NotFound | ListingErrorKind::NotADirectory
            )
        });
        let delivery = self.0.browser.deliver(response)?;
        if delivery == ListingDelivery::Applied {
            self.search_on(&directory, generation, missing)?;
            self.reveal_created(&directory, generation, listed)?;
        }
        // A copied name's tie to its file ends when a listing of the folder
        // no longer has the file; the file appearing again does not restore it.
        let origin = self.0.origin.borrow().as_ref().map(|o| o.path.clone());
        if delivery == ListingDelivery::Applied
            && let Some(path) = origin
            && let CurrentListing::Loaded(entries) = self.0.browser.listing().get()?
            && !entries.iter().any(|entry| entry.path() == path)
        {
            *self.0.origin.borrow_mut() = None;
        }
        Ok(delivery)
    }

    /// Shows `folder` as a step of the opening's search and notes the
    /// request that step waits for.
    fn seek(&self, folder: PathBuf, root_tried: bool, last: bool) -> Result<(), RuntimeError> {
        let inner = &self.0;
        *inner.search.borrow_mut() = Some(Search {
            folder: folder.clone(),
            generation: None,
            capturing: true,
            root_tried,
            last,
        });
        let navigated = inner.browser.navigate(folder);
        let mut search = inner.search.borrow_mut();
        match search.as_mut() {
            Some(step) if navigated.is_ok() && step.generation.is_some() => step.capturing = false,
            // Refused, or no request was made: there is no answer to wait for.
            _ => *search = None,
        }
        navigated
    }
    /// Continues the opening's search when the answer to its own request
    /// was applied: a folder that is missing or no folder gives way to its
    /// parent, ancestor by ancestor, and after the last one to the first
    /// root unless that was tried on the way. Any other answer ends the
    /// search, and no other response touches it.
    fn search_on(
        &self,
        directory: &Path,
        generation: u64,
        missing: bool,
    ) -> Result<(), RuntimeError> {
        let inner = &self.0;
        let step = {
            let mut search = inner.search.borrow_mut();
            if !search
                .as_ref()
                .is_some_and(|step| step.folder == directory && step.generation == Some(generation))
            {
                return Ok(());
            }
            search.take()
        };
        let Some(step) = step else {
            return Ok(());
        };
        if !missing
            || step.last
            || inner.status.get()? != FileDialogStatus::Open
            || inner.browser.current().get()? != step.folder
        {
            return Ok(());
        }
        let parent = step
            .folder
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
        match parent {
            Some(parent) => {
                let root_tried = step.root_tried || parent == inner.first_root;
                self.seek(parent.to_owned(), root_tried, false)
            }
            None if !step.root_tried => self.seek(inner.first_root.clone(), true, true),
            None => Ok(()),
        }
    }
    /// Opens `folder` from the list of places and recent folders.
    pub(super) fn go_to(&self, folder: &Path) -> Result<(), RuntimeError> {
        let inner = &self.0;
        if inner.status.get()? != FileDialogStatus::Open || inner.question.get()?.is_some() {
            return Ok(());
        }
        *inner.search.borrow_mut() = None;
        // Leaving the folder ends a folder creation in it.
        self.end_folder()?;
        inner.message.set(None)?;
        inner.browser.navigate(folder)
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
    /// Returns `None` while idle or open, and once a mounted dialog's
    /// [`FileDialog::on_outcome`] callback has taken it: whichever takes
    /// first has the outcome. The outcome, with an accepted
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
        if let Some(creating) = inner.creating.get()? {
            return self.create_folder(creating);
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
        let current = self.0.browser.current().get()?;
        let context = self.0.config.get()?.0.context.clone();
        *self.0.selection.borrow_mut() = Some(FileDialogSelection {
            paths,
            filter: filter.clone(),
            remembered: false,
        });
        match self.0.dialog.accept() {
            Ok(true) => {
                // The session is accepted; its memory is written before
                // anything can take the outcome.
                if let Some(memory) = &*self.0.session_memory.borrow() {
                    let remembered = memory.record(context.as_deref(), &current, filter.as_deref());
                    if let Some(selection) = &mut *self.0.selection.borrow_mut() {
                        selection.remembered = remembered;
                    }
                }
                Ok(true)
            }
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
        if inner.status.get()? != FileDialogStatus::Open
            || inner.question.get()?.is_some()
            || inner.creating.get()?.is_some()
        {
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
    /// A name the field refuses ([`TextField::check`]), such as one with a
    /// control character or a Unicode line separator, is not copied: the
    /// field keeps its text and no origin is recorded.
    fn name_from(&self, entry: &DirectoryEntry) -> Result<(), RuntimeError> {
        let text = entry.name().to_string_lossy().into_owned();
        if TextField::check(&text).is_err() {
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
        if self.0.creating.get()?.is_some() {
            return self.end_folder();
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
        // The application withdrew the permission to create folders, or the
        // user left the folder one was being created in.
        if change.revoked || change.current {
            self.end_folder()?;
        }
        // The user went elsewhere: the opening's search is over.
        if change.current {
            let current = inner.browser.current().get()?;
            let mut search = inner.search.borrow_mut();
            if search.as_ref().is_some_and(|step| step.folder != current) {
                *search = None;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ListingError, Runtime};

    /// The dialog offers no refresh of its own, so the browser is refreshed
    /// directly, as a host-side change of the folder would arrive.
    #[test]
    fn a_copied_names_origin_does_not_return_with_its_file() {
        let ui = Runtime::new();
        let requests: Rc<RefCell<Vec<DirectoryRequest>>> = Rc::default();
        let queue = requests.clone();
        let dialog =
            FileDialogController::new(&ui, ["/r"], move |request| queue.borrow_mut().push(request))
                .unwrap();
        let answer = |entries: Vec<DirectoryEntry>| {
            let request = requests.borrow_mut().pop().unwrap();
            assert_eq!(
                dialog.deliver(request.respond(Ok(entries))).unwrap(),
                ListingDelivery::Applied
            );
        };
        let entry = DirectoryEntry::new("/r/a.txt", EntryKind::File);
        dialog
            .open(FileDialogRequest::new(FileDialogMode::SaveFile))
            .unwrap();
        answer(vec![entry.clone()]);
        dialog.name_from(&entry).unwrap();
        assert!(dialog.0.origin.borrow().is_some());

        dialog.0.browser.refresh().unwrap();
        answer(Vec::new());
        assert!(dialog.0.origin.borrow().is_none(), "the file is gone");
        dialog.0.browser.refresh().unwrap();
        answer(vec![entry]);
        assert!(dialog.0.origin.borrow().is_none(), "and stays untied");
    }

    /// Activating the shown folder in the browser's tree requests it again
    /// without changing it; that request is the user's, not the search's.
    #[test]
    fn a_request_the_user_made_for_the_sought_folder_ends_the_search() {
        let ui = Runtime::new();
        let requests: Rc<RefCell<Vec<DirectoryRequest>>> = Rc::default();
        let queue = requests.clone();
        let memory = FileDialogMemory::new();
        assert!(memory.record(None, Path::new("/r/a/b"), None));
        let dialog =
            FileDialogController::new(&ui, ["/r"], move |request| queue.borrow_mut().push(request))
                .unwrap()
                .with_memory(&memory);
        dialog
            .open(FileDialogRequest::new(FileDialogMode::ChooseFolder))
            .unwrap();
        dialog.0.browser.navigate("/r/a/b").unwrap();
        let missing = || Err(ListingError::new(ListingErrorKind::NotFound, "gone"));
        let mut asked: Vec<DirectoryRequest> = requests
            .borrow_mut()
            .drain(..)
            .filter(|request| request.directory() == Path::new("/r/a/b"))
            .collect();
        assert_eq!(asked.len(), 2);
        let (users, searchs) = (asked.remove(1), asked.remove(0));
        assert_eq!(
            dialog.deliver(searchs.respond(missing())).unwrap(),
            ListingDelivery::Stale
        );
        assert_eq!(
            dialog.deliver(users.respond(missing())).unwrap(),
            ListingDelivery::Applied
        );
        assert_eq!(
            dialog.current().get().unwrap(),
            PathBuf::from("/r/a/b"),
            "shown as failed, where the user asked to be"
        );
    }
}
