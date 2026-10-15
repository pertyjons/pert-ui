//! An in-app file browser over a directory source the host supplies.
//!
//! The browser never touches the filesystem. It asks the host for directory
//! listings with [`DirectoryRequest`]s, the host lists them wherever it likes,
//! normally on a worker thread, and hands each [`DirectoryResponse`] back to
//! [`FileBrowserController::deliver`] on the UI thread, typically through a
//! [`Mailbox`](crate::Mailbox). [`list_directory`] is an optional helper the host
//! can call on its worker.
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

mod controller;
mod view;

pub use controller::FileBrowserController;
pub use view::FileBrowser;

/// What a [`DirectoryEntry`] is, as its directory listing reports it.
///
/// The kind comes from the entry itself, without following a symbolic link,
/// so a link is always reported as a link; [`LinkTarget`] says what it pointed
/// to when it was listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EntryKind {
    /// A directory. The browser lists it on request and shows it in its folder
    /// tree.
    Directory,
    /// A regular file.
    File,
    /// A symbolic link, with what it resolved to when listed.
    Symlink(LinkTarget),
    /// Anything else, such as a device, socket or pipe, and an entry whose
    /// type could not be read.
    Other,
}

/// What a symbolic link resolved to when its directory was listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LinkTarget {
    /// The link leads to a directory. Activating it in the entry list opens
    /// that directory under the link's own path; the folder tree does not
    /// expand links, so a link back to an ancestor never makes it recurse.
    Directory,
    /// The link leads to a regular file.
    File,
    /// The link leads to something else.
    Other,
    /// The link leads nowhere: its target is missing or unreadable.
    Broken,
}

/// The broad reason a directory or an entry could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListingErrorKind {
    /// The path does not exist (any longer).
    NotFound,
    /// The process may not read it.
    PermissionDenied,
    /// The path is not a directory.
    NotADirectory,
    /// Any other failure; the message says more.
    Other,
}

/// Why a directory, or one entry in it, could not be read: a kind for
/// decisions and a message for people.
///
/// The browser shows it rather than discarding it: a failed directory listing
/// becomes an error row under its folder and in the entry list, never an empty
/// folder, and an unreadable entry stays in the list with its message.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ListingError {
    kind: ListingErrorKind,
    message: String,
}
impl ListingError {
    /// Creates an error with a kind and a user-visible message.
    pub fn new(kind: ListingErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    /// The broad reason.
    #[must_use]
    pub fn kind(&self) -> ListingErrorKind {
        self.kind
    }
    /// The user-visible message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
impl From<std::io::Error> for ListingError {
    fn from(error: std::io::Error) -> Self {
        let kind = match error.kind() {
            std::io::ErrorKind::NotFound => ListingErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied => ListingErrorKind::PermissionDenied,
            std::io::ErrorKind::NotADirectory => ListingErrorKind::NotADirectory,
            _ => ListingErrorKind::Other,
        };
        Self::new(kind, error.to_string())
    }
}

/// One entry of a directory listing.
///
/// The path is the entry's identity: the browser keys its rows by path, never
/// by display name or position, so two entries whose names look alike stay
/// distinct and a path that is not valid UTF-8 is kept exactly. The name is
/// the last path component, shown lossily converted when it is not UTF-8.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    path: PathBuf,
    name: OsString,
    kind: EntryKind,
    error: Option<ListingError>,
}
impl DirectoryEntry {
    /// Creates an entry for `path`, named by its last component, or by the
    /// whole path when it has none, such as `/`.
    pub fn new(path: impl Into<PathBuf>, kind: EntryKind) -> Self {
        let path = path.into();
        let name = path
            .file_name()
            .map_or_else(|| path.as_os_str().to_owned(), ToOwned::to_owned);
        Self {
            path,
            name,
            kind,
            error: None,
        }
    }
    /// Creates an entry whose type could not be read. It is listed with kind
    /// [`EntryKind::Other`] and shows `error`, so it is never silently dropped.
    pub fn unreadable(path: impl Into<PathBuf>, error: ListingError) -> Self {
        let mut entry = Self::new(path, EntryKind::Other);
        entry.error = Some(error);
        entry
    }
    /// The entry's path, its identity.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// The entry's name, its last path component.
    #[must_use]
    pub fn name(&self) -> &std::ffi::OsStr {
        &self.name
    }
    /// What the entry is.
    #[must_use]
    pub fn kind(&self) -> EntryKind {
        self.kind
    }
    /// Why the entry could not be read, for an [unreadable](Self::unreadable)
    /// entry.
    #[must_use]
    pub fn error(&self) -> Option<&ListingError> {
        self.error.as_ref()
    }
    /// Whether activating the entry opens it as a directory: a directory, or a
    /// link that led to one.
    #[must_use]
    pub fn opens_as_directory(&self) -> bool {
        matches!(
            self.kind,
            EntryKind::Directory | EntryKind::Symlink(LinkTarget::Directory)
        )
    }
}

/// The browser's request for the listing of one directory, handed to the
/// host's request callback.
///
/// It is `Send`, so the host can pass it to a worker thread. Answer it with
/// [`DirectoryRequest::respond`] and give the response to
/// [`FileBrowserController::deliver`] on the UI thread. Each request carries a
/// generation: a response is applied only while its request is still the
/// latest for a directory the browser still wants, and is otherwise dropped as
/// stale. A request also names the controller that made it, so a response
/// delivered to another controller is stale even when its directory and
/// generation coincide with one of that controller's requests. Two requests
/// are equal when they come from the same controller for the same directory
/// and generation. The bookkeeping is the general
/// [`RequestTracker`](crate::RequestTracker) contract, which other components
/// that load data through their host can use as well.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a request the host never answers leaves its directory loading"]
pub struct DirectoryRequest {
    /// The ticket from the controller's [`RequestTracker`](crate::RequestTracker),
    /// which carries the directory, the generation and the requesting
    /// controller's identity.
    pub(crate) ticket: crate::RequestTicket<PathBuf>,
}
impl DirectoryRequest {
    /// The directory to list.
    #[must_use]
    pub fn directory(&self) -> &Path {
        self.ticket.key()
    }
    /// The request's generation, unique within its controller and increasing.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.ticket.generation()
    }
    /// Answers the request with a listing or the reason the directory could not
    /// be listed.
    pub fn respond(self, listing: Result<Vec<DirectoryEntry>, ListingError>) -> DirectoryResponse {
        DirectoryResponse {
            request: self,
            listing,
        }
    }
}

/// The host's answer to a [`DirectoryRequest`], delivered with
/// [`FileBrowserController::deliver`]. It is `Send`, so it can travel through a
/// [`UiSender`](crate::UiSender).
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a response does nothing until it is delivered"]
pub struct DirectoryResponse {
    request: DirectoryRequest,
    listing: Result<Vec<DirectoryEntry>, ListingError>,
}
impl DirectoryResponse {
    /// The request this answers.
    pub fn request(&self) -> &DirectoryRequest {
        &self.request
    }
    /// The listing, as the host gave it.
    ///
    /// # Errors
    /// Why the directory could not be listed, for a failure response.
    pub fn listing(&self) -> Result<&[DirectoryEntry], &ListingError> {
        self.listing.as_deref()
    }
}

/// What [`FileBrowserController::deliver`] did with a response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ListingDelivery {
    /// The listing replaced what the browser showed for its directory.
    Applied,
    /// The response was dropped: a newer request for the directory superseded
    /// it, or the browser no longer wanted the directory, because the user left
    /// it or collapsed its folder, or the request was never this controller's.
    Stale,
}

/// Why a [`FileBrowserController`] could not be created.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FileBrowserError {
    /// No root folder was given.
    #[error("a file browser needs at least one root folder")]
    NoRoots,
    /// The same root folder was given twice.
    #[error("the root folder {} was given twice", .0.display())]
    DuplicateRoot(PathBuf),
    /// The runtime refused to create the controller's state.
    #[error(transparent)]
    Runtime(#[from] crate::RuntimeError),
}

/// Lists `directory` with [`std::fs`], for a host to call on its own worker
/// thread when it answers a [`DirectoryRequest`].
///
/// **This blocks** on filesystem I/O, possibly for a long time on a network or
/// removable drive, so never call it on the UI thread. Every entry is
/// reported: an entry whose type cannot be read is listed as
/// [`DirectoryEntry::unreadable`], and a symbolic link as
/// [`EntryKind::Symlink`] with the kind of its target, found by following it
/// once. The order is the operating system's; the browser sorts what it shows.
///
/// ```no_run
/// use pert_ui_runtime::list_directory;
///
/// let entries = list_directory(std::path::Path::new("."))?;
/// for entry in &entries {
///     println!("{} {:?}", entry.name().to_string_lossy(), entry.kind());
/// }
/// # Ok::<(), pert_ui_runtime::ListingError>(())
/// ```
///
/// # Errors
/// Returns the [`ListingError`] of a directory that cannot be opened or read
/// to the end.
pub fn list_directory(directory: &Path) -> Result<Vec<DirectoryEntry>, ListingError> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let kind = match entry.file_type() {
            Ok(kind) if kind.is_symlink() => EntryKind::Symlink(match std::fs::metadata(&path) {
                Ok(target) if target.is_dir() => LinkTarget::Directory,
                Ok(target) if target.is_file() => LinkTarget::File,
                Ok(_) => LinkTarget::Other,
                Err(_) => LinkTarget::Broken,
            }),
            Ok(kind) if kind.is_dir() => EntryKind::Directory,
            Ok(kind) if kind.is_file() => EntryKind::File,
            Ok(_) => EntryKind::Other,
            Err(error) => {
                entries.push(DirectoryEntry::unreadable(path, error.into()));
                continue;
            }
        };
        entries.push(DirectoryEntry::new(path, kind));
    }
    Ok(entries)
}

/// Shows a path's last component, or the whole path when it has none.
fn display_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

impl fmt::Display for ListingErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotFound => "not found",
            Self::PermissionDenied => "permission denied",
            Self::NotADirectory => "not a directory",
            Self::Other => "unreadable",
        })
    }
}
