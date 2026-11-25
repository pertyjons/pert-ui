//! File operations a file dialog asks its host to perform.
use crate::RequestTicket;
use std::fmt;
use std::path::{Path, PathBuf};

/// Something a file dialog asks the host to do to the filesystem.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileOperation {
    /// Create the folder `name` inside `parent`.
    CreateFolder {
        /// The folder the dialog showed when the user asked.
        parent: PathBuf,
        /// The new folder's name. In an operation received from a dialog it
        /// is one file name, validated like a save name; a value built
        /// elsewhere carries no such promise.
        name: String,
    },
}
impl FileOperation {
    /// The path the operation creates.
    #[must_use]
    pub fn target(&self) -> PathBuf {
        match self {
            Self::CreateFolder { parent, name } => parent.join(name),
        }
    }
}

/// The broad reason a file operation failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileOperationErrorKind {
    /// Something of that name exists already.
    AlreadyExists,
    /// The process may not write there.
    PermissionDenied,
    /// The folder to work in does not exist (any longer).
    NotFound,
    /// The host could not take the request, for example because its worker
    /// queue is full.
    Unavailable,
    /// Any other failure; the message says more.
    Other,
}

/// Why a file operation failed: a kind for decisions and a message the
/// dialog shows to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileOperationError {
    kind: FileOperationErrorKind,
    message: String,
}
impl FileOperationError {
    /// Creates an error with a kind and a user-visible message.
    pub fn new(kind: FileOperationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    /// The broad reason.
    #[must_use]
    pub const fn kind(&self) -> FileOperationErrorKind {
        self.kind
    }
    /// The user-visible message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
impl fmt::Display for FileOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for FileOperationError {}
impl From<std::io::Error> for FileOperationError {
    /// The kind of the I/O error and its text.
    fn from(error: std::io::Error) -> Self {
        let kind = match error.kind() {
            std::io::ErrorKind::AlreadyExists => FileOperationErrorKind::AlreadyExists,
            std::io::ErrorKind::PermissionDenied => FileOperationErrorKind::PermissionDenied,
            std::io::ErrorKind::NotFound => FileOperationErrorKind::NotFound,
            _ => FileOperationErrorKind::Other,
        };
        Self::new(kind, error.to_string())
    }
}

/// A request for one [`FileOperation`], handed to the handler given to
/// [`FileDialogController::on_file_operation`](super::FileDialogController::on_file_operation).
///
/// The host performs the operation wherever it likes, typically on a worker,
/// and answers with [`Self::respond`], delivering the response on the UI
/// thread with
/// [`FileDialogController::deliver_operation`](super::FileDialogController::deliver_operation).
/// The request is `Send`. It carries its controller's identity and a
/// generation, by the [`RequestTracker`](crate::RequestTracker) contract: an
/// answer is applied only while the dialog still waits for that request. Two
/// requests are equal when they come from the same controller with the same
/// generation and operation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a request the host never answers leaves the dialog waiting until the user goes back"]
pub struct FileOperationRequest {
    pub(super) ticket: RequestTicket<()>,
    pub(super) operation: FileOperation,
}
impl FileOperationRequest {
    /// What to do.
    #[must_use]
    pub const fn operation(&self) -> &FileOperation {
        &self.operation
    }
    /// The folder the operation works in.
    #[must_use]
    pub fn parent(&self) -> &Path {
        match &self.operation {
            FileOperation::CreateFolder { parent, .. } => parent,
        }
    }
    /// The request's generation, unique within its controller and increasing.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.ticket.generation()
    }
    /// Answers the request: `Ok` once the operation is done, or why it
    /// failed.
    pub fn respond(self, result: Result<(), FileOperationError>) -> FileOperationResponse {
        FileOperationResponse {
            request: self,
            result,
        }
    }
}

/// The host's answer to a [`FileOperationRequest`]. It is `Send`, so it can
/// travel through a [`UiSender`](crate::UiSender).
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a response does nothing until it is delivered"]
pub struct FileOperationResponse {
    pub(super) request: FileOperationRequest,
    pub(super) result: Result<(), FileOperationError>,
}
impl FileOperationResponse {
    /// The request this answers.
    pub const fn request(&self) -> &FileOperationRequest {
        &self.request
    }
    /// The result, as the host gave it.
    ///
    /// # Errors
    /// Why the operation failed, for a failure response.
    pub const fn result(&self) -> Result<(), &FileOperationError> {
        match &self.result {
            Ok(()) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

/// What
/// [`FileDialogController::deliver_operation`](super::FileDialogController::deliver_operation)
/// did with a response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperationDelivery {
    /// The dialog was waiting for this answer and acted on it.
    Applied,
    /// The answer was dropped: the user went back, the session ended or was
    /// opened again, or the request was never this controller's. Nothing in
    /// the dialog changed; the operation itself may still have happened.
    Stale,
}
