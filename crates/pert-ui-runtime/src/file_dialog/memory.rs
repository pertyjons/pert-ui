//! What file dialogs remember between sessions, and its stored document.
use super::valid_key;
use crate::workspace::{Scanner, write_quoted};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Why a [`FileDialogMemory`] document was refused by
/// [`FileDialogMemory::decode`]. Nothing is taken from a refused document.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FileDialogMemoryError {
    /// The document was written by another schema revision.
    #[error(
        "file dialog memory schema version {found} is not supported (this build writes {supported})"
    )]
    UnsupportedVersion {
        /// The revision the document declares.
        found: u32,
        /// The revision this build supports.
        supported: u32,
    },
    /// The text does not follow the document grammar: an unknown or
    /// misplaced line, an unknown, repeated or missing field, or an escape
    /// the grammar does not have.
    #[error("malformed file dialog memory: {0}")]
    Malformed(String),
    /// More bytes than [`FileDialogMemory::MAX_DOCUMENT_BYTES`].
    #[error(
        "file dialog memory is larger than {} bytes",
        FileDialogMemory::MAX_DOCUMENT_BYTES
    )]
    TooLarge,
    /// More contexts than [`FileDialogMemory::MAX_CONTEXTS`].
    #[error(
        "file dialog memory holds more than {} contexts",
        FileDialogMemory::MAX_CONTEXTS
    )]
    TooManyContexts,
    /// More recent folders than [`FileDialogMemory::MAX_RECENT`].
    #[error(
        "file dialog memory holds more than {} recent folders",
        FileDialogMemory::MAX_RECENT
    )]
    TooManyRecent,
    /// A context key or a filter identity that a
    /// [`FileDialogRequest`](super::FileDialogRequest) or a
    /// [`FileFilter`](super::FileFilter) would refuse.
    #[error("{0:?} is not a file dialog context or filter identity")]
    InvalidKey(String),
    /// A folder that is empty, longer than
    /// [`FileDialogMemory::MAX_PATH_BYTES`] or holds a control character.
    #[error("a remembered folder is not a storable path")]
    InvalidPath,
    /// The same context twice.
    #[error("the context {0:?} appears twice")]
    DuplicateContext(String),
    /// The same recent folder twice.
    #[error("the recent folder {0:?} appears twice")]
    DuplicateRecent(String),
}

/// What one context remembers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Remembered {
    folder: Option<String>,
    filter: Option<String>,
}
impl Remembered {
    const fn is_empty(&self) -> bool {
        self.folder.is_none() && self.filter.is_none()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Data {
    /// For requests without a context.
    default: Remembered,
    /// Most recently used first.
    contexts: Vec<(String, Remembered)>,
    /// Most recently used first.
    recent: Vec<String>,
}

/// Where the user was in earlier file dialog sessions: per purpose the last
/// folder and file type, and the folders used most recently.
///
/// The application owns it and gives it to any number of controllers with
/// [`FileDialogController::with_memory`](super::FileDialogController::with_memory);
/// it outlives each of them. A clone is another handle to the same memory,
/// and equality compares what two memories hold. It is a UI-thread value,
/// like the controllers that use it.
///
/// # What is remembered, and when
///
/// A session's purpose is its request's
/// [`context`](super::FileDialogRequest::context); requests without one
/// share one unnamed context. Only an accepted session writes, once, before
/// its outcome becomes readable: the folder the dialog showed and the
/// selected file type under the session's context, and that folder at the
/// front of the recent folders. A cancelled, dismissed or failed session
/// changes nothing. The write merges into the memory as it is at that
/// moment, so two dialogs sharing a memory do not erase each other's
/// contexts; for one context the later accept wins.
///
/// At most [`Self::MAX_CONTEXTS`] contexts and [`Self::MAX_RECENT`] recent
/// folders are kept; the least recently used is dropped. A file type is
/// remembered by its [`FileFilter`](super::FileFilter) identity; a session
/// whose request does not offer the remembered one starts with its first.
///
/// # Paths
///
/// Schema version 1 stores a folder as Unicode text. A folder whose path is
/// not valid Unicode, holds a control character or is longer than
/// [`Self::MAX_PATH_BYTES`] is not remembered: the context keeps its earlier
/// folder, nothing lossy is stored, and the accepted selection reports it
/// ([`FileDialogSelection::folder_remembered`](super::FileDialogSelection::folder_remembered)).
///
/// # Persistence
///
/// [`Self::encode`] and [`Self::decode`] turn the memory into a text
/// document of its own versioned schema and back; the host chooses where and
/// when to store it, typically after handling an accepted outcome. It is a
/// separate document from a [`Workspace`](crate::Workspace): a user
/// preference with its own lifetime, not the arrangement of a view.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::FileDialogMemory;
///
/// let stored = "pert-ui-file-dialog-memory 1\n\
///               context \"open-project\" folder=\"/home/ada/projects\" filter=\"pert\"\n\
///               recent \"/home/ada/projects\"\n";
/// let memory = FileDialogMemory::decode(stored)?;
/// assert_eq!(
///     memory.last_folder(Some("open-project")),
///     Some("/home/ada/projects".into())
/// );
/// assert_eq!(memory.last_filter(Some("open-project")).as_deref(), Some("pert"));
/// assert_eq!(memory.encode(), stored);
/// # Ok::<(), pert_ui::FileDialogMemoryError>(())
/// ```
#[derive(Clone, Default)]
pub struct FileDialogMemory(Rc<RefCell<Data>>);
impl std::fmt::Debug for FileDialogMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.borrow().fmt(f)
    }
}
impl PartialEq for FileDialogMemory {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || *self.0.borrow() == *other.0.borrow()
    }
}
impl Eq for FileDialogMemory {}

const MAGIC: &str = "pert-ui-file-dialog-memory";

fn malformed(reason: &str) -> FileDialogMemoryError {
    FileDialogMemoryError::Malformed(reason.to_owned())
}
/// The scanner reports grammar errors only; its reason is kept.
fn scan<T>(result: Result<T, crate::WorkspaceError>) -> Result<T, FileDialogMemoryError> {
    result.map_err(|error| match error {
        crate::WorkspaceError::Malformed(reason) => FileDialogMemoryError::Malformed(reason),
        other => FileDialogMemoryError::Malformed(other.to_string()),
    })
}
/// The text a folder is stored as, if version 1 can store it.
fn storable(path: &Path) -> Option<&str> {
    path.to_str().filter(|text| valid_path(text))
}
fn valid_path(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= FileDialogMemory::MAX_PATH_BYTES
        && !text.chars().any(char::is_control)
}

impl FileDialogMemory {
    /// The schema revision [`Self::encode`] writes and [`Self::decode`]
    /// accepts. It is independent of the crate version.
    pub const SCHEMA_VERSION: u32 = 1;
    /// The most contexts kept, besides the unnamed one.
    pub const MAX_CONTEXTS: usize = 64;
    /// The most recent folders kept.
    pub const MAX_RECENT: usize = 12;
    /// The longest folder path stored, in bytes of UTF-8.
    pub const MAX_PATH_BYTES: usize = 4096;
    /// The largest document [`Self::decode`] reads, in bytes. Every document
    /// [`Self::encode`] writes is smaller.
    pub const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;

    /// Creates an empty memory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The folder remembered for `context`: the one the last accepted
    /// session of that context showed whose path could be stored. `None`
    /// names the context of requests without one.
    #[must_use]
    pub fn last_folder(&self, context: Option<&str>) -> Option<PathBuf> {
        self.remembered(context)
            .and_then(|remembered| remembered.folder)
            .map(PathBuf::from)
    }
    /// The identity of the file type selected when the last session of
    /// `context` that offered types was accepted.
    #[must_use]
    pub fn last_filter(&self, context: Option<&str>) -> Option<String> {
        self.remembered(context)
            .and_then(|remembered| remembered.filter)
    }
    /// The folders of accepted sessions, most recent first; at most
    /// [`Self::MAX_RECENT`]. No two are equal as paths: a folder accepted
    /// again under another spelling, such as `/a/./b` for `/a/b`, replaces
    /// the earlier one.
    #[must_use]
    pub fn recent_folders(&self) -> Vec<PathBuf> {
        self.0.borrow().recent.iter().map(PathBuf::from).collect()
    }
    /// Forgets everything, for every handle to this memory.
    pub fn clear(&self) {
        *self.0.borrow_mut() = Data::default();
    }
    fn remembered(&self, context: Option<&str>) -> Option<Remembered> {
        let data = self.0.borrow();
        match context {
            None => Some(data.default.clone()),
            Some(key) => data
                .contexts
                .iter()
                .find(|(context, _)| context == key)
                .map(|(_, remembered)| remembered.clone()),
        }
    }

    /// Merges an accepted session into the memory: `folder` and `filter`
    /// under `context`, and the folder among the recent ones. Returns whether
    /// the folder could be stored. A missing filter leaves the context's
    /// remembered one.
    pub(super) fn record(
        &self,
        context: Option<&str>,
        folder: &Path,
        filter: Option<&str>,
    ) -> bool {
        let folder = storable(folder).map(str::to_owned);
        let mut data = self.0.borrow_mut();
        let data = &mut *data;
        let mut remembered = match context {
            None => std::mem::take(&mut data.default),
            Some(key) => data
                .contexts
                .iter()
                .position(|(context, _)| context == key)
                .map(|index| data.contexts.remove(index).1)
                .unwrap_or_default(),
        };
        if let Some(folder) = &folder {
            remembered.folder = Some(folder.clone());
        }
        if let Some(filter) = filter {
            remembered.filter = Some(filter.to_owned());
        }
        match context {
            None => data.default = remembered,
            // Nothing storable is no context: every line of the document
            // carries something.
            Some(_) if remembered.is_empty() => {}
            Some(key) => {
                data.contexts.insert(0, (key.to_owned(), remembered));
                data.contexts.truncate(Self::MAX_CONTEXTS);
            }
        }
        if let Some(folder) = &folder {
            // Paths compare as paths, so `/r/a` and `/r/./a` are one folder;
            // the newest spelling is the one kept.
            data.recent
                .retain(|recent| Path::new(recent) != Path::new(folder));
            data.recent.insert(0, folder.clone());
            data.recent.truncate(Self::MAX_RECENT);
        }
        folder.is_some()
    }

    /// Encodes the memory as text the host stores wherever it likes.
    ///
    /// The encoding is deterministic: equal memories produce the same bytes.
    /// The first line names the format and [`Self::SCHEMA_VERSION`]. Then
    /// follow, each only when there is something to say, a `default` line
    /// for the unnamed context, one `context` line per named context and one
    /// `recent` line per recent folder, the lists most recent first. Text is
    /// quoted with the escapes `\"` and `\\`, and every line ends with `\n`.
    #[must_use]
    pub fn encode(&self) -> String {
        fn fields(out: &mut String, remembered: &Remembered) {
            for (name, value) in [
                ("folder", &remembered.folder),
                ("filter", &remembered.filter),
            ] {
                if let Some(value) = value {
                    out.push(' ');
                    out.push_str(name);
                    out.push('=');
                    write_quoted(out, value);
                }
            }
            out.push('\n');
        }
        let data = self.0.borrow();
        let mut out = format!("{MAGIC} {}\n", Self::SCHEMA_VERSION);
        if !data.default.is_empty() {
            out.push_str("default");
            fields(&mut out, &data.default);
        }
        for (key, remembered) in &data.contexts {
            out.push_str("context ");
            write_quoted(&mut out, key);
            fields(&mut out, remembered);
        }
        for folder in &data.recent {
            out.push_str("recent ");
            write_quoted(&mut out, folder);
            out.push('\n');
        }
        out
    }

    /// Decodes a document into a new memory, validating the whole of it
    /// before returning.
    ///
    /// Tokens have the one spelling [`Self::encode`] writes, and the lines
    /// its order: `default` first, then `context` lines, then `recent`
    /// lines, with at least one space between a line's tokens. The line
    /// layout is tolerated where a text transport may change it: `\r\n`
    /// line endings, blank lines, a missing final newline and extra spaces
    /// between tokens. One invalid line rejects the document;
    /// nothing of it is kept.
    ///
    /// # Errors
    /// [`FileDialogMemoryError::TooLarge`];
    /// [`FileDialogMemoryError::UnsupportedVersion`] for another schema
    /// revision; [`FileDialogMemoryError::Malformed`] for text outside the
    /// grammar; [`FileDialogMemoryError::TooManyContexts`] and
    /// [`FileDialogMemoryError::TooManyRecent`];
    /// [`FileDialogMemoryError::InvalidKey`] and
    /// [`FileDialogMemoryError::InvalidPath`] for a value the dialog would
    /// never have stored; [`FileDialogMemoryError::DuplicateContext`] and
    /// [`FileDialogMemoryError::DuplicateRecent`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{FileDialogMemory, FileDialogMemoryError};
    /// assert_eq!(
    ///     FileDialogMemory::decode("pert-ui-file-dialog-memory 7\n"),
    ///     Err(FileDialogMemoryError::UnsupportedVersion { found: 7, supported: 1 }),
    /// );
    /// ```
    pub fn decode(text: &str) -> Result<Self, FileDialogMemoryError> {
        if text.len() > Self::MAX_DOCUMENT_BYTES {
            return Err(FileDialogMemoryError::TooLarge);
        }
        // `lines` takes `\n` and `\r\n`; a `\r` left over is no layout.
        let mut lines = text.lines().filter(|line| !line.trim().is_empty());
        let header = lines.next().ok_or_else(|| malformed("missing header"))?;
        let version = header
            .strip_prefix(MAGIC)
            .and_then(|rest| rest.strip_prefix(' '))
            .ok_or_else(|| malformed("unknown document format"))?
            .trim_matches(' ');
        let version: u32 = version
            .parse()
            .ok()
            .filter(|parsed: &u32| parsed.to_string() == version)
            .ok_or_else(|| malformed("unreadable schema version"))?;
        if version != Self::SCHEMA_VERSION {
            return Err(FileDialogMemoryError::UnsupportedVersion {
                found: version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        let mut data = Data::default();
        // 0 before any line, 1 after `default`, 2 in contexts, 3 in recents.
        let mut stage = 0;
        for line in lines {
            let mut scanner = Scanner::new(line);
            match scan(scanner.word())? {
                "default" if stage == 0 => {
                    stage = 1;
                    separated(&scanner)?;
                    data.default = decode_fields(&mut scanner)?;
                }
                "context" if stage <= 2 => {
                    stage = 2;
                    separated(&scanner)?;
                    let key = scan(scanner.quoted())?;
                    separated(&scanner)?;
                    if !valid_key(&key) {
                        return Err(FileDialogMemoryError::InvalidKey(key));
                    }
                    if data.contexts.iter().any(|(context, _)| *context == key) {
                        return Err(FileDialogMemoryError::DuplicateContext(key));
                    }
                    if data.contexts.len() == Self::MAX_CONTEXTS {
                        return Err(FileDialogMemoryError::TooManyContexts);
                    }
                    let remembered = decode_fields(&mut scanner)?;
                    data.contexts.push((key, remembered));
                }
                "recent" => {
                    stage = 3;
                    separated(&scanner)?;
                    let folder = scan(scanner.quoted())?;
                    if !valid_path(&folder) {
                        return Err(FileDialogMemoryError::InvalidPath);
                    }
                    if data
                        .recent
                        .iter()
                        .any(|recent| Path::new(recent) == Path::new(&folder))
                    {
                        return Err(FileDialogMemoryError::DuplicateRecent(folder));
                    }
                    if data.recent.len() == Self::MAX_RECENT {
                        return Err(FileDialogMemoryError::TooManyRecent);
                    }
                    if !scanner.at_end() {
                        return Err(malformed("text after a recent folder"));
                    }
                    data.recent.push(folder);
                }
                line @ ("default" | "context") => {
                    return Err(malformed(&format!("misplaced `{line}` line")));
                }
                other => return Err(malformed(&format!("unknown line `{other}`"))),
            }
        }
        Ok(Self(Rc::new(RefCell::new(data))))
    }
}

/// Tokens are separated by spaces: a token that runs into the next one is
/// outside the grammar.
fn separated(scanner: &Scanner<'_>) -> Result<(), FileDialogMemoryError> {
    if scanner.at_gap() {
        Ok(())
    } else {
        Err(malformed("expected a space between tokens"))
    }
}

/// The `folder` and `filter` fields of a line, at least one, each once.
fn decode_fields(scanner: &mut Scanner<'_>) -> Result<Remembered, FileDialogMemoryError> {
    let mut remembered = Remembered::default();
    while !scanner.at_end() {
        let name = scan(scanner.word())?;
        scan(scanner.expect('='))?;
        let value = scan(scanner.quoted())?;
        separated(scanner)?;
        let slot = match name {
            "folder" => {
                if !valid_path(&value) {
                    return Err(FileDialogMemoryError::InvalidPath);
                }
                &mut remembered.folder
            }
            "filter" => {
                if !valid_key(&value) {
                    return Err(FileDialogMemoryError::InvalidKey(value));
                }
                &mut remembered.filter
            }
            other => return Err(malformed(&format!("unknown field `{other}`"))),
        };
        if slot.replace(value).is_some() {
            return Err(malformed(&format!("repeated field `{name}`")));
        }
    }
    if remembered.is_empty() {
        return Err(malformed("a line without a folder or a filter"));
    }
    Ok(remembered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileDialogRequest;

    #[test]
    fn the_least_recently_used_gives_way_at_capacity() {
        let memory = FileDialogMemory::new();
        for index in 0..=FileDialogMemory::MAX_CONTEXTS {
            let folder = format!("/r/{index}");
            assert!(memory.record(Some(&format!("c{index}")), Path::new(&folder), None));
        }
        assert_eq!(memory.last_folder(Some("c0")), None, "the oldest context");
        assert_eq!(memory.last_folder(Some("c1")), Some("/r/1".into()));
        let recent = memory.recent_folders();
        assert_eq!(recent.len(), FileDialogMemory::MAX_RECENT);
        assert_eq!(
            recent[0],
            PathBuf::from(format!("/r/{}", FileDialogMemory::MAX_CONTEXTS))
        );
        // A folder used again moves to the front and is listed once.
        let again = recent[3].clone();
        assert!(memory.record(None, &again, None));
        let recent = memory.recent_folders();
        assert_eq!(recent[0], again);
        assert_eq!(recent.iter().filter(|folder| **folder == again).count(), 1);
        assert_eq!(recent.len(), FileDialogMemory::MAX_RECENT);
        // Every memory the dialog can build is one it can read back.
        assert_eq!(FileDialogMemory::decode(&memory.encode()).unwrap(), memory);
        assert!(memory.encode().len() < FileDialogMemory::MAX_DOCUMENT_BYTES);
    }

    #[test]
    fn a_context_used_again_is_not_the_one_dropped() {
        let memory = FileDialogMemory::new();
        for index in 0..FileDialogMemory::MAX_CONTEXTS {
            assert!(memory.record(Some(&format!("c{index}")), Path::new("/r"), None));
        }
        // The oldest is used again, then one more context arrives.
        assert!(memory.record(Some("c0"), Path::new("/r/again"), None));
        assert!(memory.record(Some("new"), Path::new("/r"), None));
        assert_eq!(memory.last_folder(Some("c0")), Some("/r/again".into()));
        assert_eq!(memory.last_folder(Some("c1")), None, "now the oldest");
        assert_eq!(memory.last_folder(Some("new")), Some("/r".into()));
    }

    #[test]
    fn one_folder_under_two_spellings_is_one_recent_folder() {
        let memory = FileDialogMemory::new();
        assert!(memory.record(None, Path::new("/r/a"), None));
        assert!(memory.record(None, Path::new("/r/b"), None));
        assert!(memory.record(None, Path::new("/r/./a"), None));
        assert_eq!(
            memory.recent_folders(),
            [PathBuf::from("/r/./a"), PathBuf::from("/r/b")]
        );
        assert_eq!(FileDialogMemory::decode(&memory.encode()).unwrap(), memory);
    }

    #[test]
    fn the_fullest_memory_fits_its_document_and_reads_back() {
        // Every character of every value needs an escape.
        let memory = FileDialogMemory::new();
        let key = |index: usize| {
            let tail = format!("{index:02}");
            format!(
                "{}{tail}",
                "\\".repeat(FileDialogRequest::MAX_KEY_CHARS - tail.len())
            )
        };
        let folder = |index: usize| {
            let tail = format!("/{index:03}");
            format!(
                "{}{tail}",
                "\"".repeat(FileDialogMemory::MAX_PATH_BYTES - tail.len())
            )
        };
        assert!(memory.record(None, Path::new(&folder(900)), Some(&key(99))));
        for index in 0..FileDialogMemory::MAX_CONTEXTS {
            assert!(memory.record(
                Some(&key(index)),
                Path::new(&folder(index)),
                Some(&key(index))
            ));
        }
        let text = memory.encode();
        assert!(
            text.len() <= FileDialogMemory::MAX_DOCUMENT_BYTES,
            "{}",
            text.len()
        );
        assert_eq!(FileDialogMemory::decode(&text).unwrap(), memory);
    }

    #[test]
    fn a_folder_that_cannot_be_stored_leaves_the_earlier_one() {
        let memory = FileDialogMemory::new();
        assert!(memory.record(Some("c"), Path::new("/r/a"), Some("txt")));
        for folder in [
            "/r/a\nb",
            "",
            &"a".repeat(FileDialogMemory::MAX_PATH_BYTES + 1),
        ] {
            assert!(!memory.record(Some("c"), Path::new(folder), None));
        }
        assert_eq!(memory.last_folder(Some("c")), Some("/r/a".into()));
        assert_eq!(memory.last_filter(Some("c")).as_deref(), Some("txt"));
        assert_eq!(memory.recent_folders(), [PathBuf::from("/r/a")]);
        // Nothing storable under a new context adds no context.
        assert!(!memory.record(Some("d"), Path::new(""), None));
        assert!(!memory.encode().contains("\"d\""));
    }
}
