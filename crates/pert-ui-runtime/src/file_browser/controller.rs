//! The browser's state: navigation, listings, requests and keys.
use super::view::{EntryRow, FolderRow, PlaceholderText, placeholder};
use super::{
    DirectoryEntry, DirectoryRequest, DirectoryResponse, FileBrowserError, ListingDelivery,
    ListingError,
};
use crate::{
    Element, EventContext, ListItems, ListSelection, ListSource, ReadSignal, RequestTracker,
    RuntimeError, SelectionMode, Signal, TicketStatus, TreeController, TreeItems, TreeSource,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::rc::Rc;

type RequestFn = Box<dyn FnMut(DirectoryRequest)>;
type OpenFn = Box<dyn FnMut(&mut EventContext, &DirectoryEntry) -> Result<(), RuntimeError>>;

/// Row keys for paths: each path the browser has met gets a number once, and
/// its key is `p` and that number. A key therefore never depends on how a
/// path's name looks, and a path that is not UTF-8 keeps a key of its own.
#[derive(Default)]
pub(super) struct Paths {
    ids: HashMap<PathBuf, usize>,
    paths: Vec<PathBuf>,
}
impl Paths {
    pub(super) fn key(&mut self, path: &Path) -> String {
        let id = match self.ids.get(path) {
            Some(id) => *id,
            None => {
                let id = self.paths.len();
                self.paths.push(path.to_owned());
                self.ids.insert(path.to_owned(), id);
                id
            }
        };
        format!("p{id}")
    }
    fn known_key(&self, path: &Path) -> Option<String> {
        self.ids.get(path).map(|id| format!("p{id}"))
    }
    pub(super) fn path(&self, key: &str) -> Option<&PathBuf> {
        self.paths
            .get(key.strip_prefix('p')?.parse::<usize>().ok()?)
    }
}
type SharedPaths = Rc<RefCell<Paths>>;

/// What the browser knows about one directory.
#[derive(Clone, Debug)]
enum Listing {
    /// Its entries in display order, and the subdirectories the folder tree
    /// shows, in the same order.
    Loaded {
        entries: Rc<[DirectoryEntry]>,
        folders: Rc<[PathBuf]>,
    },
    /// Why it could not be listed.
    Failed(ListingError),
}
impl Listing {
    fn new(directory: &Path, listing: Result<Vec<DirectoryEntry>, ListingError>) -> Self {
        let mut entries = match listing {
            Ok(entries) => entries,
            Err(error) => return Self::Failed(error),
        };
        // Directories first, then by name ignoring case, then by the exact
        // name, so the order is total and does not depend on the host's.
        entries.sort_by(|a, b| {
            b.opens_as_directory()
                .cmp(&a.opens_as_directory())
                .then_with(|| {
                    let (x, y) = (a.name.to_string_lossy(), b.name.to_string_lossy());
                    x.to_lowercase().cmp(&y.to_lowercase())
                })
                .then_with(|| a.name.cmp(&b.name))
        });
        // A path is a row's identity: a repeat would give two rows one key,
        // and the directory itself would make it its own child.
        let mut seen = BTreeSet::new();
        entries.retain(|entry| entry.path != directory && seen.insert(entry.path.clone()));
        let folders = entries
            .iter()
            .filter(|entry| entry.kind == super::EntryKind::Directory)
            .map(|entry| entry.path.clone())
            .collect();
        Self::Loaded {
            entries: entries.into(),
            folders,
        }
    }
}

/// Every listing the browser holds, replaced as a whole when one arrives, so
/// signals compare it by allocation.
#[derive(Clone, Debug, Default)]
struct Listings(Rc<BTreeMap<PathBuf, Listing>>);
impl PartialEq for Listings {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Retains the current entry source across responses for unrelated folders.
struct EntryCache {
    directory: PathBuf,
    listing: Option<Listing>,
    query: String,
    source: ListSource,
}
impl EntryCache {
    fn matches(&self, directory: &Path, listing: Option<&Listing>, query: &str) -> bool {
        self.directory == directory
            && self.query == query
            && match (&self.listing, listing) {
                (None, None) => true,
                (Some(Listing::Failed(a)), Some(Listing::Failed(b))) => a == b,
                (
                    Some(Listing::Loaded { entries: a, .. }),
                    Some(Listing::Loaded { entries: b, .. }),
                ) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}

/// The folder tree over the listings at one moment.
struct Folders {
    texts: Rc<Texts>,
    roots: Rc<[PathBuf]>,
    listings: Listings,
    paths: SharedPaths,
    /// Expanded folders shown without a listing; the browser requests them.
    wanted: Rc<RefCell<BTreeSet<PathBuf>>>,
    activate: Rc<Hooks>,
}
const LOADING: &str = "loading:";
const FAILED: &str = "error:";
impl TreeItems for Folders {
    fn roots(&self) -> Vec<String> {
        let mut paths = self.paths.borrow_mut();
        self.roots.iter().map(|root| paths.key(root)).collect()
    }
    fn children(&self, key: &str) -> Vec<String> {
        let Some(path) = self.paths.borrow().path(key).cloned() else {
            return Vec::new();
        };
        match self.listings.0.get(&path) {
            Some(Listing::Loaded { folders, .. }) => {
                let mut paths = self.paths.borrow_mut();
                folders.iter().map(|folder| paths.key(folder)).collect()
            }
            Some(Listing::Failed(_)) => vec![format!("{FAILED}{key}")],
            None => {
                self.wanted.borrow_mut().insert(path);
                vec![format!("{LOADING}{key}")]
            }
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        // Only folders are nodes, so every path key can expand until its
        // listing shows it has no subfolders; no listing is needed to answer.
        let paths = self.paths.borrow();
        let Some(path) = paths.path(key) else {
            return false;
        };
        !matches!(self.listings.0.get(path), Some(Listing::Loaded { folders, .. }) if folders.is_empty())
    }
    fn text(&self, key: &str) -> Option<String> {
        // Placeholder rows are not folders and match nothing.
        let paths = self.paths.borrow();
        Some(super::display_name(paths.path(key)?))
    }
    fn known_children(&self, key: &str) -> Option<Vec<String>> {
        // Only what a listing already delivered; an unlisted folder is not
        // asked for, so filtering starts no request.
        let path = self.paths.borrow().path(key).cloned()?;
        match self.listings.0.get(&path)? {
            Listing::Loaded { folders, .. } => {
                let mut paths = self.paths.borrow_mut();
                Some(folders.iter().map(|folder| paths.key(folder)).collect())
            }
            Listing::Failed(_) => Some(Vec::new()),
        }
    }
    fn build(&self, key: &str) -> Option<Element> {
        if let Some(folder) = key.strip_prefix(LOADING) {
            self.paths.borrow().path(folder)?;
            return Some(placeholder(
                key,
                PlaceholderText::Bound(self.texts.loading.clone()),
                None,
            ));
        }
        if let Some(folder) = key.strip_prefix(FAILED) {
            let path = self.paths.borrow().path(folder)?.clone();
            let Some(Listing::Failed(error)) = self.listings.0.get(&path) else {
                return None;
            };
            return Some(placeholder(
                key,
                PlaceholderText::Fixed(error.message().to_owned()),
                Some(error),
            ));
        }
        let path = self.paths.borrow().path(key)?.clone();
        Some(Element::new(
            key,
            FolderRow::new(super::display_name(&path), path, self.activate.clone()),
        ))
    }
}

/// One row of the entry list.
enum EntryLine {
    Entry {
        key: String,
        entry: DirectoryEntry,
    },
    Placeholder {
        key: &'static str,
        text: PlaceholderText,
        error: Option<ListingError>,
    },
}
/// The entry list over one directory's listing and filter.
struct Entries {
    lines: Vec<EntryLine>,
    index: HashMap<String, usize>,
    activate: Rc<Hooks>,
}
impl ListItems for Entries {
    fn len(&self) -> usize {
        self.lines.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        Some(match self.lines.get(index)? {
            EntryLine::Entry { key, .. } => key.clone(),
            EntryLine::Placeholder { key, .. } => (*key).to_owned(),
        })
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(match self.lines.get(index)? {
            EntryLine::Entry { key, entry } => Element::new(
                key.clone(),
                EntryRow::new(key.clone(), entry, self.activate.clone()),
            ),
            EntryLine::Placeholder { key, text, error } => {
                placeholder(key, text.clone(), error.as_ref())
            }
        })
    }
    fn text(&self, index: usize) -> Option<String> {
        match self.lines.get(index)? {
            EntryLine::Entry { entry, .. } => Some(entry.name.to_string_lossy().into_owned()),
            // Bound placeholder messages are supplied by mounted presentation;
            // searching unmounted rows needs no signal reads or translation work.
            EntryLine::Placeholder { .. } => None,
        }
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.index.get(key).copied()
    }
}
fn entries(
    directory: &Path,
    listings: &Listings,
    query: &str,
    paths: &SharedPaths,
    activate: &Rc<Hooks>,
    texts: &Texts,
) -> ListSource {
    let placeholder =
        |key, text: PlaceholderText, error: Option<&ListingError>| EntryLine::Placeholder {
            key,
            text,
            error: error.cloned(),
        };
    let bound = |text: &ReadSignal<String>| PlaceholderText::Bound(text.clone());
    let lines = match listings.0.get(directory) {
        None => vec![placeholder("loading", bound(&texts.loading), None)],
        Some(Listing::Failed(error)) => vec![placeholder(
            "error",
            PlaceholderText::Fixed(error.message().to_owned()),
            Some(error),
        )],
        Some(Listing::Loaded { entries, .. }) => {
            let needle = query.trim().to_lowercase();
            let mut paths = paths.borrow_mut();
            let lines: Vec<_> = entries
                .iter()
                .filter(|entry| {
                    needle.is_empty()
                        || entry
                            .name
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&needle)
                })
                .map(|entry| EntryLine::Entry {
                    key: paths.key(&entry.path),
                    entry: entry.clone(),
                })
                .collect();
            if lines.is_empty() && entries.is_empty() {
                vec![placeholder("empty", bound(&texts.empty), None)]
            } else if lines.is_empty() {
                vec![placeholder("empty", bound(&texts.no_matches), None)]
            } else {
                lines
            }
        }
    };
    let index = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let key = match line {
                EntryLine::Entry { key, .. } => key.clone(),
                EntryLine::Placeholder { key, .. } => (*key).to_owned(),
            };
            (key, index)
        })
        .collect();
    ListSource::new(Entries {
        lines,
        index,
        activate: activate.clone(),
    })
}

/// What a row calls back into when it is activated. The rows of both lists
/// hold it, so it refers to the controller only weakly.
#[derive(Default)]
pub(super) struct Hooks {
    controller: RefCell<std::rc::Weak<Inner>>,
}
impl Hooks {
    fn controller(&self) -> Option<FileBrowserController> {
        self.controller
            .borrow()
            .upgrade()
            .map(FileBrowserController)
    }
    /// A folder row was chosen: show that folder.
    pub(super) fn open_folder(&self, path: &Path) -> Result<(), RuntimeError> {
        match self.controller() {
            Some(controller) => controller.navigate(path),
            None => Ok(()),
        }
    }
    /// A folder or breadcrumb segment was chosen by key: show that folder.
    pub(super) fn open_key(&self, key: &str) -> Result<(), RuntimeError> {
        match self.controller() {
            Some(controller) => match controller.path_of(key) {
                Some(path) => controller.navigate(path),
                None => Ok(()),
            },
            None => Ok(()),
        }
    }
    /// An entry was activated: open a directory, or hand anything else to the
    /// application.
    pub(super) fn activate(&self, cx: &mut EventContext, key: &str) -> Result<(), RuntimeError> {
        match self.controller() {
            Some(controller) => controller.activate(cx, key),
            None => Ok(()),
        }
    }
}

/// The placeholder rows' texts, following the installed string table.
pub(super) struct Texts {
    loading: ReadSignal<String>,
    empty: ReadSignal<String>,
    no_matches: ReadSignal<String>,
}

pub(super) struct Inner {
    settings: crate::UiSettings,
    texts: Rc<Texts>,
    roots: Rc<[PathBuf]>,
    paths: SharedPaths,
    current: Signal<PathBuf>,
    listings: Signal<Listings>,
    filter: Signal<String>,
    folder_filter: Signal<String>,
    tree: TreeController,
    selection: ListSelection,
    entries: ReadSignal<ListSource>,
    hooks: Rc<Hooks>,
    wanted: Rc<RefCell<BTreeSet<PathBuf>>>,
    /// The latest request for each directory still wanted, its generation and
    /// this controller's identity in its requests.
    requests: RequestTracker<PathBuf>,
    /// Requests made and not yet handed to the host.
    queue: RefCell<Vec<DirectoryRequest>>,
    request: RefCell<RequestFn>,
    open: RefCell<Option<OpenFn>>,
}

/// Application-owned state of a [`FileBrowser`](super::FileBrowser): the folder
/// shown, every listing received, the filter and the entry selection, and the
/// channel through which the browser asks the host for listings.
///
/// The controller performs no filesystem I/O and starts no thread. When it
/// needs a directory listed it calls the host's request callback with a
/// [`DirectoryRequest`]; the host lists the directory where it likes, normally
/// on a worker thread with [`list_directory`](super::list_directory), and gives
/// the [`DirectoryResponse`] back to [`Self::deliver`] on the UI thread, usually
/// through a [`Mailbox`](crate::Mailbox) handler. The browser shows a loading
/// row until the listing arrives, and an error row, never an empty folder, when
/// it failed.
///
/// A directory is requested when it becomes the current one, every time, so
/// entering a folder shows fresh entries, and when its folder is expanded in
/// the tree while the browser has no listing for it. A response is applied
/// only while its request is the latest for that directory and the browser
/// still wants the directory, because it is the current one or the tree shows
/// its folder expanded (it and every folder above it are expanded); otherwise [`Self::deliver`] drops it as
/// [`ListingDelivery::Stale`]. A request the host never answers leaves its
/// directory loading until it is requested again, for example by
/// [`Self::refresh`]. Failures are never hidden: a refused signal read while
/// the browser decides what to request fails the method or the browser's
/// update, and once the controller has issued `u64::MAX` requests every
/// further request fails with `Exhausted` instead of leaving a folder loading
/// without a request.
///
/// Identity is by path. Rows are keyed by a number the controller gives each
/// path it meets, not by the path's text, so a name that is not UTF-8 stays
/// exact and distinct; [`Self::path_of`] maps a key back. The table of paths
/// and the listings received grow with the folders visited and are released
/// with the controller; nothing is evicted. The entry
/// list shows directories first, then everything else, each ordered by name
/// ignoring case. A response for another folder keeps the current entry list
/// and its measurements when its directory, listing and query are unchanged.
/// The folder tree still updates independently. The tree shows directories
/// only, not symbolic links,
/// so it never follows a link back to an ancestor.
///
/// Every method is for application and event code on the UI thread and fails
/// like a signal write during an update, layout or paint. Clones share the
/// same state.
///
/// ```
/// use pert_ui_runtime::{
///     DirectoryEntry, EntryKind, Element, FileBrowser, FileBrowserController, ListSelection,
///     ListingDelivery, Runtime, SelectionMode,
/// };
/// use std::{cell::RefCell, rc::Rc};
///
/// let mut ui = Runtime::new();
/// // A real host sends requests to a worker; this one keeps them.
/// let requests = Rc::new(RefCell::new(Vec::new()));
/// let queue = requests.clone();
/// let controller = FileBrowserController::new(
///     &ui,
///     ["/projects"],
///     ListSelection::new(&ui, SelectionMode::Single),
///     move |request| queue.borrow_mut().push(request),
/// )?;
/// ui.mount_tree(Element::new("files", FileBrowser::new(&controller, 24.0)?))?;
///
/// // The browser asked for the folder it shows; answer it.
/// let request = requests.borrow_mut().remove(0);
/// assert_eq!(request.directory(), std::path::Path::new("/projects"));
/// let response = request.respond(Ok(vec![
///     DirectoryEntry::new("/projects/drafts", EntryKind::Directory),
///     DirectoryEntry::new("/projects/report.txt", EntryKind::File),
/// ]));
/// assert_eq!(controller.deliver(response)?, ListingDelivery::Applied);
///
/// controller.navigate("/projects/drafts")?;
/// assert_eq!(requests.borrow()[0].directory(), std::path::Path::new("/projects/drafts"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
#[must_use]
pub struct FileBrowserController(Rc<Inner>);
impl FileBrowserController {
    /// The current installed table for initial parts, without subscriptions.
    pub(super) fn initial_strings(&self) -> Result<crate::UiStrings, RuntimeError> {
        self.0.settings.strings()
    }

    /// Creates a controller whose folder tree starts at `roots`, in order,
    /// showing the first root, and asks the host for its listing at once.
    ///
    /// `selection` holds the entry list's selection by row key, in the mode it
    /// was created with; [`Self::selected_paths`] reports it as paths.
    /// `request` runs on the UI thread whenever the browser needs a directory
    /// listed: during this call, during the controller's methods and during the
    /// browser's update after a folder was expanded. It must return promptly
    /// and must not block, perform I/O, write signals or call this controller;
    /// hand the request to a worker instead, and if that fails, deliver an
    /// error response for it later so the directory does not stay loading.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// [`FileBrowserError::NoRoots`] without a root,
    /// [`FileBrowserError::DuplicateRoot`] for a root given twice, and the
    /// runtime's refusal to create state as [`FileBrowserError::Runtime`].
    pub fn new(
        ui: &impl crate::StateOwner,
        roots: impl IntoIterator<Item = impl Into<PathBuf>>,
        selection: ListSelection,
        request: impl FnMut(DirectoryRequest) + 'static,
    ) -> Result<Self, FileBrowserError> {
        let roots: Vec<PathBuf> = roots.into_iter().map(Into::into).collect();
        let first = roots.first().cloned().ok_or(FileBrowserError::NoRoots)?;
        for (index, root) in roots.iter().enumerate() {
            if roots[..index].contains(root) {
                return Err(FileBrowserError::DuplicateRoot(root.clone()));
            }
        }
        let roots: Rc<[PathBuf]> = roots.into();
        let paths = SharedPaths::default();
        let wanted = Rc::new(RefCell::new(BTreeSet::new()));
        let hooks = Rc::new(Hooks::default());
        let listings = ui.signal(Listings::default());
        let current = ui.signal(first.clone());
        let filter = ui.signal(String::new());
        // Placeholder rows bind their text to the string table, so installing
        // another table relabels them without rebuilding a source.
        let text = |role: crate::StringRole| {
            crate::runtime::settings::string_signal(listings.shared(), role).unwrap_or_else(|| {
                ui.signal(listings.shared().strings.borrow().get(role).to_owned())
                    .read_only()
            })
        };
        let texts = Rc::new(Texts {
            loading: text(crate::StringRole::FileBrowserLoading),
            empty: text(crate::StringRole::FileBrowserEmptyFolder),
            no_matches: text(crate::StringRole::FileBrowserNoMatches),
        });
        let folder_filter = ui.signal(String::new());
        let folders = |listings: Listings| {
            TreeSource::new(Folders {
                texts: texts.clone(),
                roots: roots.clone(),
                listings,
                paths: paths.clone(),
                wanted: wanted.clone(),
                activate: hooks.clone(),
            })
        };
        let tree = TreeController::new(
            ui,
            folders(Listings::default()),
            ListSelection::new(ui, SelectionMode::Single),
        )?;
        let shown = current
            .read_only()
            .combine(&listings.read_only(), |path, listings| {
                (path.clone(), listings.clone())
            })?;
        let (row_paths, row_hooks, row_texts) = (paths.clone(), hooks.clone(), texts.clone());
        let entry_cache: RefCell<Option<EntryCache>> = RefCell::new(None);
        let entries = shown.combine(&filter.read_only(), move |(path, listings), query| {
            let listing = listings.0.get(path);
            if let Some(previous) = entry_cache.borrow().as_ref()
                && previous.matches(path, listing, query)
            {
                return previous.source.clone();
            }
            let source = entries(path, listings, query, &row_paths, &row_hooks, &row_texts);
            *entry_cache.borrow_mut() = Some(EntryCache {
                directory: path.clone(),
                listing: listing.cloned(),
                query: query.clone(),
                source: source.clone(),
            });
            source
        })?;
        let controller = Self(Rc::new(Inner {
            settings: ui.ui_settings(),
            texts,
            roots,
            paths,
            current,
            listings,
            filter,
            folder_filter,
            tree,
            selection,
            entries,
            hooks,
            wanted,
            requests: RequestTracker::new(),
            queue: RefCell::new(Vec::new()),
            request: RefCell::new(Box::new(request)),
            open: RefCell::new(None),
        }));
        *controller.0.hooks.controller.borrow_mut() = Rc::downgrade(&controller.0);
        controller.select_in_tree(&first)?;
        controller.request(first)?;
        controller.dispatch()?;
        Ok(controller)
    }

    /// Sets what happens when the user activates an entry that is not a
    /// directory, by Enter on the entry list or a double click: the callback
    /// receives the entry. Opening, previewing and acting on files are the
    /// application's decisions; without a callback such an activation does
    /// nothing. Activating a directory, or a link to one, always opens it in the
    /// browser instead. The callback runs in the event phase and may write
    /// signals and call this controller; its error propagates to the input
    /// method. It replaces an earlier callback.
    pub fn on_open(
        &self,
        open: impl FnMut(&mut EventContext, &DirectoryEntry) -> Result<(), RuntimeError> + 'static,
    ) {
        *self.0.open.borrow_mut() = Some(Box::new(open));
    }

    /// The folder the entry list shows.
    #[must_use]
    pub fn current(&self) -> ReadSignal<PathBuf> {
        self.0.current.read_only()
    }

    /// The text the entry list is filtered by: an entry shows when its name
    /// contains the trimmed text, ignoring case. The browser's search field
    /// edits it, and the application may set it. Navigating clears it.
    #[must_use]
    pub fn filter(&self) -> Signal<String> {
        self.0.filter.clone()
    }

    /// The text the folder tree is filtered by, when the browser offers a
    /// folder filter ([`FileBrowser::folder_filter`](super::FileBrowser::folder_filter)):
    /// a folder shows when its name contains the trimmed text, ignoring case,
    /// together with the folders above it. Only folders already listed are
    /// searched, and filtering never asks the host for a listing. The
    /// application may set it; navigating keeps it.
    #[must_use]
    pub fn folder_filter(&self) -> Signal<String> {
        self.0.folder_filter.clone()
    }

    /// The entry list's selection, by row key; see [`Self::path_of`].
    pub fn selection(&self) -> ListSelection {
        self.0.selection.clone()
    }

    /// The paths of the selected entries, in key order: the active entry in
    /// [`SelectionMode::Single`], the selected set in
    /// [`SelectionMode::Multiple`]. Placeholder rows, such as the loading row,
    /// have no path and are left out.
    ///
    /// # Errors
    /// The selection signals' read failures.
    pub fn selected_paths(&self) -> Result<Vec<PathBuf>, RuntimeError> {
        let selection = &self.0.selection;
        let keys: Vec<String> = match selection.mode() {
            SelectionMode::Single => selection.active().get()?.into_iter().collect(),
            SelectionMode::Multiple => selection.selected().get()?.iter().cloned().collect(),
        };
        Ok(keys.iter().filter_map(|key| self.path_of(key)).collect())
    }

    /// The path a row key names, in the entry list or the folder tree, or
    /// `None` for a placeholder row or a key the browser never gave out.
    #[must_use]
    pub fn path_of(&self, key: &str) -> Option<PathBuf> {
        self.0.paths.borrow().path(key).cloned()
    }

    /// Shows `directory` in the entry list and requests its listing.
    ///
    /// The filter and the entry selection are cleared. When the directory is
    /// inside a root, the tree expands the folders down to it and makes it
    /// active. A listing the browser already holds is shown until the new one
    /// arrives. A pending request for the folder left behind, which the tree
    /// does not show expanded, becomes stale. Navigating to the current folder
    /// only requests it again.
    ///
    /// # Errors
    /// The signal writes' failures, such as `WriteDuringUpdate`, and
    /// `Exhausted` if request generations run out; the folder shown, the filter
    /// and the selection are then left unchanged.
    pub fn navigate(&self, directory: impl AsRef<Path>) -> Result<(), RuntimeError> {
        self.0.current.shared().writable()?;
        let directory = directory.as_ref().to_owned();
        // The ticket comes first: when generations have run out, nothing
        // changes, so the browser never shows a folder loading that no
        // request will ever answer.
        let leaving = self.0.current.get()? != directory;
        self.request(directory.clone())?;
        if leaving {
            self.0.selection.clear()?;
            self.0.filter.set(String::new())?;
            self.0.current.set(directory.clone())?;
            self.select_in_tree(&directory)?;
        }
        self.dispatch()
    }

    /// Shows the parent of the current folder, when it has one. Returns whether
    /// it navigated.
    ///
    /// # Errors
    /// Same as [`Self::navigate`].
    pub fn navigate_up(&self) -> Result<bool, RuntimeError> {
        let current = self.0.current.get()?;
        match current
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            Some(parent) => self.navigate(parent).map(|()| true),
            None => Ok(false),
        }
    }

    /// Requests the current folder again; its listing replaces the one shown
    /// when it arrives, and an older pending request becomes stale.
    ///
    /// # Errors
    /// Same as [`Self::navigate`].
    pub fn refresh(&self) -> Result<(), RuntimeError> {
        self.request(self.0.current.get()?)?;
        self.dispatch()
    }

    /// Applies the host's answer to a request, or drops it as stale.
    ///
    /// The listing is applied when its request is still the latest for its
    /// directory and the browser still wants that directory; the entries are
    /// then sorted for display, repeated paths and the directory itself are
    /// left out, and the folder tree and entry list show the result with their
    /// expansion and selection kept by key. For the current folder, selected
    /// entries and an active entry the new listing no longer has leave the
    /// selection, so [`Self::selected_paths`] never reports a removed entry.
    /// A failed listing is shown as an error row and selects nothing.
    /// Anything else, including a response to another controller's request,
    /// returns [`ListingDelivery::Stale`] and changes nothing.
    ///
    /// # Errors
    /// The signal reads' and writes' failures, such as `WriteDuringUpdate`.
    /// The request then stays pending, so a later response for it can still
    /// apply. After the listing is applied, handing out the requests it makes
    /// necessary (folders its tree now shows expanded) can still fail with
    /// `Exhausted` when request generations run out; the listing stays applied
    /// and the error is returned.
    pub fn deliver(&self, response: DirectoryResponse) -> Result<ListingDelivery, RuntimeError> {
        let DirectoryResponse { request, listing } = response;
        let ticket = request.ticket;
        if self.0.requests.status(&ticket) != TicketStatus::Current {
            return Ok(ListingDelivery::Stale);
        }
        let directory = ticket.key().clone();
        let listing = Listing::new(&directory, listing);
        let present = self.present_keys(&directory, &listing)?;
        let mut map = (*self.0.listings.get()?.0).clone();
        map.insert(directory.clone(), listing);
        let listings = Listings(Rc::new(map));
        self.0.listings.set(listings.clone())?;
        if let Some(present) = present {
            self.prune_selection(&present)?;
        }
        self.0.requests.complete(&ticket);
        self.0.wanted.borrow_mut().remove(&directory);
        self.0.tree.set_source(self.folders(listings))?;
        self.dispatch()?;
        Ok(ListingDelivery::Applied)
    }

    /// The row keys a new listing of `directory` shows, when it is the
    /// current folder: every entry's key for a loaded listing, none for a
    /// failed one. `None` when the listing is for another folder.
    fn present_keys(
        &self,
        directory: &Path,
        listing: &Listing,
    ) -> Result<Option<BTreeSet<String>>, RuntimeError> {
        if self.0.current.get()? != directory {
            return Ok(None);
        }
        let mut paths = self.0.paths.borrow_mut();
        Ok(Some(match listing {
            Listing::Loaded { entries, .. } => {
                entries.iter().map(|entry| paths.key(&entry.path)).collect()
            }
            Listing::Failed(_) => BTreeSet::new(),
        }))
    }

    /// Removes from the entry selection, and from the active row, the keys of
    /// entries the current folder's new listing no longer has. Entries that
    /// are still listed keep their keys, and so their selection.
    fn prune_selection(&self, present: &BTreeSet<String>) -> Result<(), RuntimeError> {
        let selection = &self.0.selection;
        if selection.mode() == SelectionMode::Multiple {
            let selected = selection.selected().get()?;
            if selected.iter().any(|key| !present.contains(key)) {
                selection.select(
                    selected
                        .iter()
                        .filter(|key| present.contains(*key))
                        .cloned()
                        .collect::<Vec<_>>(),
                )?;
            }
        }
        if selection
            .active()
            .get()?
            .is_some_and(|key| !present.contains(&key))
        {
            selection.set_active(None)?;
        }
        Ok(())
    }

    pub(super) fn tree(&self) -> &TreeController {
        &self.0.tree
    }
    pub(super) fn entries(&self) -> ReadSignal<ListSource> {
        self.0.entries.clone()
    }
    pub(super) fn hooks(&self) -> Rc<Hooks> {
        self.0.hooks.clone()
    }
    pub(super) fn roots(&self) -> &[PathBuf] {
        &self.0.roots
    }
    pub(super) fn key_of(&self, path: &Path) -> String {
        self.0.paths.borrow_mut().key(path)
    }

    /// The folder tree over `listings`.
    fn folders(&self, listings: Listings) -> TreeSource {
        TreeSource::new(Folders {
            texts: self.0.texts.clone(),
            roots: self.0.roots.clone(),
            listings,
            paths: self.0.paths.clone(),
            wanted: self.0.wanted.clone(),
            activate: self.0.hooks.clone(),
        })
    }

    /// Expands the folders above `directory` inside its root and makes it the
    /// tree's active node; a directory outside every root leaves the tree as it
    /// is.
    fn select_in_tree(&self, directory: &Path) -> Result<(), RuntimeError> {
        let Some(root) = self
            .0
            .roots
            .iter()
            .filter(|root| directory.starts_with(root))
            .max_by_key(|root| root.components().count())
        else {
            return Ok(());
        };
        let above: Vec<&Path> = directory
            .ancestors()
            .skip(1)
            .take_while(|ancestor| ancestor.starts_with(root))
            .collect();
        for ancestor in above.into_iter().rev() {
            let key = self.key_of(ancestor);
            self.0.tree.set_expanded(&key, true)?;
        }
        let key = self.key_of(directory);
        self.0.tree.selection().set_active(Some(key))
    }

    /// Opens a directory entry or hands another entry to the application.
    fn activate(&self, cx: &mut EventContext, key: &str) -> Result<(), RuntimeError> {
        let Some(path) = self.path_of(key) else {
            return Ok(());
        };
        let current = self.0.current.get()?;
        let entry = match self.0.listings.get()?.0.get(&current) {
            Some(Listing::Loaded { entries, .. }) => {
                entries.iter().find(|entry| entry.path == path).cloned()
            }
            _ => None,
        };
        let Some(entry) = entry else {
            return Ok(());
        };
        if entry.opens_as_directory() {
            return self.navigate(&path);
        }
        // Taken out while it runs, so it may replace itself or call back in.
        let open = self.0.open.borrow_mut().take();
        let Some(mut open) = open else {
            return Ok(());
        };
        let result = open(cx, &entry);
        let mut slot = self.0.open.borrow_mut();
        if slot.is_none() {
            *slot = Some(open);
        }
        result
    }

    /// Makes a request for `directory` the latest one.
    fn request(&self, directory: PathBuf) -> Result<(), RuntimeError> {
        let ticket = self.0.requests.issue(directory)?;
        self.0.queue.borrow_mut().push(DirectoryRequest { ticket });
        Ok(())
    }

    /// Requests the expanded folders the tree showed without a listing, forgets
    /// pending requests for directories no longer wanted, and hands queued
    /// requests to the host. Called from the controller's methods and the
    /// browser's update; it writes no signal.
    ///
    /// A failure to read the tree's expansion, the current folder or the
    /// listings is returned before anything changes. When request generations
    /// run out, the requests already made are still handed to the host and
    /// `Exhausted` is returned, so a folder never waits silently for a request
    /// that was never made.
    pub(super) fn dispatch(&self) -> Result<(), RuntimeError> {
        let expanded = self.0.tree.expanded().get()?;
        let current = self.0.current.get()?;
        let listings = self.0.listings.get()?;
        let roots = self.0.roots.clone();
        // A folder's listing is wanted while it is the current folder, or
        // while the tree shows it expanded: it and every folder above it, up
        // to its root, are expanded. A folder left expanded under a collapsed
        // ancestor is hidden and not wanted.
        let wanted_here = |path: &Path, paths: &Paths| {
            if current == path {
                return true;
            }
            let Some(root) = roots
                .iter()
                .filter(|root| path.starts_with(root))
                .max_by_key(|root| root.components().count())
            else {
                return false;
            };
            path.ancestors()
                .take_while(|ancestor| ancestor.starts_with(root))
                .all(|ancestor| {
                    paths
                        .known_key(ancestor)
                        .is_some_and(|key| expanded.contains(&key))
                })
        };
        let missing: Vec<PathBuf> = {
            let paths = self.0.paths.borrow();
            let mut wanted = self.0.wanted.borrow_mut();
            wanted.retain(|path| wanted_here(path, &paths));
            wanted
                .iter()
                .filter(|path| !listings.0.contains_key(*path) && !self.0.requests.is_pending(path))
                .cloned()
                .collect()
        };
        let mut exhausted = None;
        for directory in missing {
            // Generations run out only after 2^64 requests; the failure is
            // returned once the requests already made are handed out.
            if let Err(error) = self.request(directory) {
                exhausted = Some(error);
                break;
            }
        }
        {
            let paths = self.0.paths.borrow();
            self.0.requests.retain(|path| wanted_here(path, &paths));
        }
        // A request made while the callback runs waits in the queue for the
        // loop, so a reentrant call never runs the callback twice at once.
        while let Ok(mut request) = self.0.request.try_borrow_mut() {
            let queued = std::mem::take(&mut *self.0.queue.borrow_mut());
            if queued.is_empty() {
                break;
            }
            for item in queued {
                request(item);
            }
        }
        exhausted.map_or(Ok(()), Err)
    }
}

impl std::fmt::Debug for FileBrowserController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileBrowserController")
            .field("roots", &self.0.roots)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntryKind, FileBrowser, Runtime, WidgetId};

    type Requests = Rc<RefCell<Vec<DirectoryRequest>>>;

    fn mounted() -> (Runtime, FileBrowserController, Requests, WidgetId) {
        let mut ui = Runtime::new();
        // The tree lists expanded folders, and so records them as wanted,
        // only while it has room to show them.
        ui.set_viewport(crate::Viewport::new(800.0, 600.0, 1.0).unwrap())
            .unwrap();
        let requests: Requests = Rc::default();
        let queue = requests.clone();
        let controller = FileBrowserController::new(
            &ui,
            ["/r"],
            ListSelection::new(&ui, SelectionMode::Single),
            move |request| queue.borrow_mut().push(request),
        )
        .unwrap();
        let root = ui
            .mount_tree(Element::new(
                "browser",
                FileBrowser::new(&controller, 24.0).unwrap(),
            ))
            .unwrap();
        ui.flush(ui.now()).unwrap();
        let request = requests.borrow_mut().remove(0);
        let listing = vec![
            DirectoryEntry::new("/r/a", EntryKind::Directory),
            DirectoryEntry::new("/r/b", EntryKind::Directory),
        ];
        controller.deliver(request.respond(Ok(listing))).unwrap();
        ui.flush(ui.now()).unwrap();
        (ui, controller, requests, root)
    }

    #[test]
    fn another_folder_response_preserves_current_entry_source_and_row_revision() {
        use crate::Component;
        let (mut ui, controller, requests, _root) = mounted();
        controller
            .tree()
            .set_expanded(&controller.key_of(Path::new("/r")), true)
            .unwrap();
        controller
            .tree()
            .set_expanded(&controller.key_of(Path::new("/r/a")), true)
            .unwrap();
        ui.flush(ui.now()).unwrap();
        let index = requests
            .borrow()
            .iter()
            .position(|request| request.directory() == Path::new("/r/a"))
            .unwrap();
        let request = requests.borrow_mut().remove(index);
        let before = controller.entries().get().unwrap();
        let offered = crate::VirtualList::new(controller.entries(), 24.0).unwrap();
        let revision = offered.try_content_rows().unwrap().unwrap().revision();
        controller
            .deliver(request.respond(Ok(vec![DirectoryEntry::new("/r/a/file", EntryKind::File)])))
            .unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(controller.entries().get().unwrap(), before);
        assert_eq!(
            offered.try_content_rows().unwrap().unwrap().revision(),
            revision
        );
        controller.filter().set("a".to_owned()).unwrap();
        ui.flush(ui.now()).unwrap();
        assert_ne!(controller.entries().get().unwrap(), before);
    }

    #[test]
    fn exhausted_generations_fail_the_update_after_handing_out_the_made_requests() {
        let (mut ui, controller, requests, _root) = mounted();
        // One generation is left: the first folder gets it, the second cannot.
        controller.0.requests.set_generation(u64::MAX - 1);
        let (root, a, b) = (
            controller.key_of(Path::new("/r")),
            controller.key_of(Path::new("/r/a")),
            controller.key_of(Path::new("/r/b")),
        );
        controller.tree().set_expanded(&root, true).unwrap();
        controller.tree().set_expanded(&a, true).unwrap();
        controller.tree().set_expanded(&b, true).unwrap();
        assert_eq!(ui.flush(ui.now()), Err(RuntimeError::Exhausted));
        // The request that was made reached the host despite the failure.
        let handed: Vec<_> = requests
            .borrow()
            .iter()
            .map(|request| (request.directory().to_owned(), request.generation()))
            .collect();
        assert_eq!(handed, [(PathBuf::from("/r/a"), u64::MAX)]);
        // Methods that request report it as well.
        assert_eq!(controller.refresh(), Err(RuntimeError::Exhausted));
    }

    #[test]
    fn exhausted_navigation_leaves_the_browser_where_it_was() {
        let (mut ui, controller, requests, _root) = mounted();
        requests.borrow_mut().clear();
        controller.filter().set("a".into()).unwrap();
        ui.flush(ui.now()).unwrap();
        controller.0.requests.set_generation(u64::MAX);
        assert_eq!(controller.navigate("/r/b"), Err(RuntimeError::Exhausted));
        // Nothing moved: no folder waits for a request that was never made.
        assert_eq!(controller.current().get().unwrap(), PathBuf::from("/r"));
        assert_eq!(controller.filter().get().unwrap(), "a");
        assert!(requests.borrow().is_empty());
        ui.flush(ui.now()).unwrap();
    }

    #[test]
    fn a_refused_read_fails_dispatch_instead_of_requesting_nothing() {
        let (mut ui, controller, requests, _root) = mounted();
        requests.borrow_mut().clear();
        ui.shutdown();
        assert_eq!(controller.dispatch(), Err(RuntimeError::Closed));
        assert!(requests.borrow().is_empty());
    }
}
