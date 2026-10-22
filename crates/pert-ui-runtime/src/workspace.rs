//! Restorable workspace state: the captured document, its text encoding and the
//! report a restore produces.
//!
//! The library captures the view arrangement it alone holds, encodes it as a
//! versioned document, and restores it into a mounted tree. It never decides
//! where the document is stored, when it is written, or whether it is written at
//! all: [`Workspace::encode`] returns a string the host stores wherever it likes,
//! and [`Workspace::decode`] accepts one back. See
//! [`Runtime::capture_workspace`](crate::Runtime::capture_workspace) and
//! [`Runtime::restore_workspace`](crate::Runtime::restore_workspace).
//!
//! The document schema is versioned separately from the crate: a document
//! carries [`Workspace::SCHEMA_VERSION`], and a version this build does not
//! support is rejected rather than reinterpreted.
use std::collections::BTreeMap;
use std::fmt;

/// One value inside a [`ViewState`].
///
/// The vocabulary is deliberately small and closed, so that a document can be
/// validated completely on decode. Inside a [`ViewState`] or a decoded document,
/// numbers are always finite; text is bounded by [`Workspace::MAX_STRING_BYTES`]
/// and may not contain control characters; and a map holds scalars only, which
/// keeps the encoding one level deep and the parser small.
///
/// The representation is private, so every value is built through
/// [`Self::flag`], [`Self::integer`], [`Self::number`], [`Self::text`] or
/// [`Self::map`] and holds these rules from the start; the `as_*` accessors
/// read it back.
///
/// # Serialization
///
/// With the `serde` feature a value is tagged by its variant, for example
/// `{ "Number": 0.25 }` or `{ "Map": { "a": { "Integer": 1 } } }`, and reading
/// one applies the same rules as building it: finite numbers, bounded text, no
/// nested map and no repeated map key.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(transparent))]
pub struct WorkspaceValue(Repr);

/// The kinds of [`WorkspaceValue`]; constructing one checks its rules.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
enum Repr {
    /// A boolean, such as whether a floating window is minimized.
    Flag(bool),
    /// A whole number, such as a count or an index into a fixed vocabulary.
    Integer(i64),
    /// A finite logical-pixel extent, ratio or offset.
    Number(f64),
    /// A key or name that identifies something the application named, such as a
    /// tab key or a column key. Never display text.
    Text(String),
    /// Scalars keyed by an application-supplied name, such as a width per table
    /// column. Nested maps are rejected.
    Map(BTreeMap<String, WorkspaceValue>),
}

impl WorkspaceValue {
    /// Creates a boolean, such as whether a floating window is minimized.
    #[must_use]
    pub const fn flag(value: bool) -> Self {
        Self(Repr::Flag(value))
    }
    /// Creates a whole number, such as a count or an index into a fixed
    /// vocabulary.
    #[must_use]
    pub const fn integer(value: i64) -> Self {
        Self(Repr::Integer(value))
    }
    /// A text value the library names itself, such as an edge or mode name,
    /// which is short plain text; [`Self::map`] and [`ViewState::with`] still
    /// check it.
    pub(crate) fn name(value: &str) -> Self {
        Self(Repr::Text(value.to_owned()))
    }
    /// A number from a finite `f32`, which is always finite as `f64`;
    /// [`Self::map`] and [`ViewState::with`] still check it.
    pub(crate) fn finite(value: f32) -> Self {
        Self(Repr::Number(f64::from(value)))
    }
    /// Creates a finite number.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::NotFinite`] for an infinite or NaN value, which
    /// is a rejection rather than a clamp: a stored preference value is never
    /// coerced.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{WorkspaceError, WorkspaceValue};
    /// assert!(WorkspaceValue::number(0.25).is_ok());
    /// assert_eq!(WorkspaceValue::number(f64::NAN), Err(WorkspaceError::NotFinite));
    /// ```
    pub fn number(value: f64) -> Result<Self, WorkspaceError> {
        if value.is_finite() {
            Ok(Self(Repr::Number(value)))
        } else {
            Err(WorkspaceError::NotFinite)
        }
    }
    /// Creates a text value.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::ControlCharacter`] for text holding a control
    /// character and [`WorkspaceError::TextTooLong`] for text longer than
    /// [`Workspace::MAX_STRING_BYTES`].
    pub fn text(value: impl Into<String>) -> Result<Self, WorkspaceError> {
        let value = value.into();
        check_text(&value)?;
        Ok(Self(Repr::Text(value)))
    }
    /// Creates a map of scalars.
    ///
    /// # Errors
    /// Rejects a nested map, an invalid or repeated map key and any invalid
    /// scalar. A repeated key is [`WorkspaceError::DuplicateKey`], never a
    /// silent choice between the two values.
    pub fn map(entries: impl IntoIterator<Item = (String, Self)>) -> Result<Self, WorkspaceError> {
        let mut values = BTreeMap::new();
        for (key, value) in entries {
            check_text(&key)?;
            if key.is_empty() {
                return Err(WorkspaceError::InvalidName(key));
            }
            if matches!(value.0, Repr::Map(_)) {
                return Err(WorkspaceError::NestedMap);
            }
            value.validate()?;
            match values.entry(key) {
                std::collections::btree_map::Entry::Occupied(entry) => {
                    return Err(WorkspaceError::DuplicateKey(entry.key().clone()));
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(value);
                }
            }
        }
        Ok(Self(Repr::Map(values)))
    }
    /// The boolean, or `None` for another kind of value.
    #[must_use]
    pub const fn as_flag(&self) -> Option<bool> {
        match &self.0 {
            Repr::Flag(value) => Some(*value),
            _ => None,
        }
    }
    /// The whole number, or `None` for another kind of value.
    #[must_use]
    pub const fn as_integer(&self) -> Option<i64> {
        match &self.0 {
            Repr::Integer(value) => Some(*value),
            _ => None,
        }
    }
    /// The number, or `None` for another kind of value. It is finite for every
    /// value read from a [`ViewState`] or a decoded document; a value built
    /// directly from the variant returns what it holds.
    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        match &self.0 {
            Repr::Number(value) => Some(*value),
            _ => None,
        }
    }
    /// The text, or `None` for another kind of value.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match &self.0 {
            Repr::Text(value) => Some(value),
            _ => None,
        }
    }
    /// The map, or `None` for another kind of value.
    #[must_use]
    pub const fn as_map(&self) -> Option<&BTreeMap<String, Self>> {
        match &self.0 {
            Repr::Map(values) => Some(values),
            _ => None,
        }
    }
    fn validate(&self) -> Result<(), WorkspaceError> {
        match &self.0 {
            Repr::Flag(_) | Repr::Integer(_) => Ok(()),
            Repr::Number(value) if !value.is_finite() => Err(WorkspaceError::NotFinite),
            Repr::Number(_) => Ok(()),
            Repr::Text(value) => check_text(value),
            Repr::Map(values) => {
                for (key, value) in values {
                    check_text(key)?;
                    if key.is_empty() {
                        return Err(WorkspaceError::InvalidName(key.clone()));
                    }
                    if matches!(value.0, Repr::Map(_)) {
                        return Err(WorkspaceError::NestedMap);
                    }
                    value.validate()?;
                }
                Ok(())
            }
        }
    }
}

/// The state one participating component captured, and the kind of component it
/// came from.
///
/// The kind is a short stable tag the component owns, such as `tabs` or `split`.
/// It is never a Rust type name: renaming a type must not invalidate saved
/// workspaces, and a tag that no longer matches the component at a path is what
/// turns an application's restructuring into a reported entry instead of a split
/// proportion applied to a tab strip.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{ViewState, WorkspaceValue};
/// let state = ViewState::new("tabs")?.with("selected", WorkspaceValue::text("scope")?)?;
/// assert_eq!(state.kind(), "tabs");
/// assert_eq!(state.text("selected"), Ok("scope"));
/// # Ok::<(), pert_ui::WorkspaceError>(())
/// ```
///
/// # Serialization
///
/// With the `serde` feature a state is `{ kind, values }`, and reading one
/// validates the kind and every name and value as [`ViewState::new`] and
/// [`ViewState::with`] do; a repeated name is an error.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    kind: String,
    values: BTreeMap<String, WorkspaceValue>,
}

impl ViewState {
    /// The longest kind tag or value name, in bytes.
    pub const MAX_NAME_BYTES: usize = 64;
    /// Creates empty state for a component kind.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::InvalidName`] unless the kind tag is 1 to
    /// [`Self::MAX_NAME_BYTES`] bytes of ASCII letters, digits, `-` or `_`.
    pub fn new(kind: impl Into<String>) -> Result<Self, WorkspaceError> {
        let kind = kind.into();
        check_name(&kind)?;
        Ok(Self {
            kind,
            values: BTreeMap::new(),
        })
    }
    /// Adds a named value, replacing an existing one with the same name.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::InvalidName`] for a name outside the rule of
    /// kinds, and for a value that fails its constructor's checks the error that
    /// constructor returns: [`WorkspaceError::NotFinite`],
    /// [`WorkspaceError::TextTooLong`], [`WorkspaceError::ControlCharacter`],
    /// [`WorkspaceError::InvalidName`] for an empty map key or
    /// [`WorkspaceError::NestedMap`].
    pub fn with(
        mut self,
        name: impl Into<String>,
        value: WorkspaceValue,
    ) -> Result<Self, WorkspaceError> {
        let name = name.into();
        check_name(&name)?;
        value.validate()?;
        self.values.insert(name, value);
        Ok(self)
    }
    /// The component kind this state came from.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
    /// A named value, or `None` when the document did not carry it.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&WorkspaceValue> {
        self.values.get(name)
    }
    /// Ordered names, for a participant that validates its own fields.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
    /// Whether any value is present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Rejects any name outside `expected`, which keeps a participant's fields a
    /// closed set: a document written by a newer build that added a field is
    /// refused by this one instead of being applied in part.
    ///
    /// # Errors
    /// [`RestoreRejection::UnknownValue`] naming the first unexpected name.
    pub fn expect_only(&self, expected: &[&str]) -> Result<(), RestoreRejection> {
        match self
            .values
            .keys()
            .find(|name| !expected.contains(&name.as_str()))
        {
            Some(name) => Err(RestoreRejection::UnknownValue(name.clone())),
            None => Ok(()),
        }
    }
    /// Reads a boolean.
    ///
    /// # Errors
    /// [`RestoreRejection::Value`] when it is missing or another kind of value.
    pub fn flag(&self, name: &str) -> Result<bool, RestoreRejection> {
        self.get(name)
            .and_then(WorkspaceValue::as_flag)
            .ok_or_else(|| RestoreRejection::Value(name.to_owned()))
    }
    /// Reads a whole number.
    ///
    /// # Errors
    /// [`RestoreRejection::Value`] when it is missing or another kind of value.
    pub fn integer(&self, name: &str) -> Result<i64, RestoreRejection> {
        self.get(name)
            .and_then(WorkspaceValue::as_integer)
            .ok_or_else(|| RestoreRejection::Value(name.to_owned()))
    }
    /// Reads a finite number.
    ///
    /// # Errors
    /// [`RestoreRejection::Value`] when it is missing or another kind of value.
    pub fn number(&self, name: &str) -> Result<f64, RestoreRejection> {
        self.get(name)
            .and_then(WorkspaceValue::as_number)
            .ok_or_else(|| RestoreRejection::Value(name.to_owned()))
    }
    /// Reads text.
    ///
    /// # Errors
    /// [`RestoreRejection::Value`] when it is missing or another kind of value.
    pub fn text(&self, name: &str) -> Result<&str, RestoreRejection> {
        self.get(name)
            .and_then(WorkspaceValue::as_text)
            .ok_or_else(|| RestoreRejection::Value(name.to_owned()))
    }
    /// Reads a map of scalars.
    ///
    /// # Errors
    /// [`RestoreRejection::Value`] when it is missing or another kind of value.
    pub fn map(&self, name: &str) -> Result<&BTreeMap<String, WorkspaceValue>, RestoreRejection> {
        self.get(name)
            .and_then(WorkspaceValue::as_map)
            .ok_or_else(|| RestoreRejection::Value(name.to_owned()))
    }
}

/// The address of one participant: the path of addressable segments from the
/// mount root, such as `main/frame/center/editor`.
///
/// Segments come from application keys and from slot names a container
/// declares. Library-internal structure contributes nothing, so a container may
/// be restructured without invalidating saved documents, and neither a child
/// index nor a runtime incarnation is ever part of an address: reordering or
/// recreating a view must not retarget saved state.
///
/// # Serialization
///
/// With the `serde` feature a path is its list of segments, root first, and
/// reading one validates them through [`WorkspacePath::new`].
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkspacePath(Vec<String>);

impl WorkspacePath {
    /// Builds a path from ordered segments.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::EmptyPath`] for an empty path or an empty
    /// segment, [`WorkspaceError::ControlCharacter`] for a segment holding a
    /// control character, [`WorkspaceError::TextTooLong`] for a segment longer
    /// than [`Workspace::MAX_STRING_BYTES`] and [`WorkspaceError::PathTooLong`]
    /// for a path longer than [`Workspace::MAX_PATH_BYTES`] once encoded.
    pub fn new(
        segments: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, WorkspaceError> {
        let segments: Vec<String> = segments.into_iter().map(Into::into).collect();
        if segments.is_empty() {
            return Err(WorkspaceError::EmptyPath);
        }
        for segment in &segments {
            if segment.is_empty() {
                return Err(WorkspaceError::EmptyPath);
            }
            check_text(segment)?;
        }
        let path = Self(segments);
        if path.to_string().len() > Workspace::MAX_PATH_BYTES {
            return Err(WorkspaceError::PathTooLong);
        }
        Ok(path)
    }
    /// Ordered segments, root first.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}

impl fmt::Display for WorkspacePath {
    /// Writes segments separated by `/`, escaping `\` and `/` inside a segment.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, segment) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("/")?;
            }
            for character in segment.chars() {
                match character {
                    '\\' => f.write_str("\\\\")?,
                    '/' => f.write_str("\\/")?,
                    _ => write!(f, "{character}")?,
                }
            }
        }
        Ok(())
    }
}

/// Workspace-wide presentation preferences.
///
/// A host that already persists these applies its own values after restoring;
/// the last write wins, and that ordering is the documented one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct WorkspaceSettings {
    zoom: Option<crate::UiZoom>,
    mode: Option<crate::ThemeMode>,
    density: Option<crate::Density>,
}

impl WorkspaceSettings {
    /// The captured application zoom, absent when the document carried none.
    #[must_use]
    pub const fn zoom(&self) -> Option<crate::UiZoom> {
        self.zoom
    }
    /// The captured color scheme, absent when the document carried none.
    #[must_use]
    pub const fn theme_mode(&self) -> Option<crate::ThemeMode> {
        self.mode
    }
    /// The captured spacing preset, absent when the document carried none.
    #[must_use]
    pub const fn density(&self) -> Option<crate::Density> {
        self.density
    }
    /// Replaces the zoom, or removes it with `None`.
    #[must_use]
    pub const fn with_zoom(mut self, zoom: Option<crate::UiZoom>) -> Self {
        self.zoom = zoom;
        self
    }
    /// Replaces the color scheme, or removes it with `None`.
    #[must_use]
    pub const fn with_theme_mode(mut self, mode: Option<crate::ThemeMode>) -> Self {
        self.mode = mode;
        self
    }
    /// Replaces the spacing preset, or removes it with `None`.
    #[must_use]
    pub const fn with_density(mut self, density: Option<crate::Density>) -> Self {
        self.density = density;
        self
    }
}

/// A captured workspace: presentation settings and one entry per participant.
///
/// Entries are ordered by path and every map is ordered by key, so capturing an
/// unchanged workspace twice produces byte-identical text and storing the
/// document in version control produces no spurious differences.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::Workspace;
/// let text = "pert-ui-workspace 1\nentry \"main/split\" split ratio=0.25\n";
/// let workspace = Workspace::decode(text)?;
/// assert_eq!(workspace.len(), 1);
/// assert_eq!(workspace.encode(), text);
/// # Ok::<(), pert_ui::WorkspaceError>(())
/// ```
///
/// # Serialization
///
/// With the `serde` feature the document is also `Serialize`/`Deserialize`, as
/// `{ schema_version, settings, entries }` with each entry `{ path, state }`,
/// for a host that embeds it in its own structured file. Deserialization applies
/// the rules of [`Workspace::decode`]: the schema version, the closed vocabulary,
/// repeated names and map keys, [`Workspace::MAX_ENTRIES`], the path and text
/// limits, duplicate paths and finite numbers, and the canonical text encoding
/// must fit [`Workspace::MAX_DOCUMENT_BYTES`]. The size of the encoded input
/// itself is the caller's to bound before decoding. The text encoding remains
/// the default path.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct Workspace {
    settings: WorkspaceSettings,
    entries: Vec<(WorkspacePath, ViewState)>,
}

impl Workspace {
    /// The document schema revision this build writes and accepts.
    ///
    /// It is versioned separately from the crate version. A document from a
    /// newer revision is rejected with [`WorkspaceError::UnsupportedVersion`]
    /// rather than partially applied, and the host falls back to its default
    /// workspace.
    pub const SCHEMA_VERSION: u32 = 1;
    /// The largest number of entries one document may hold.
    pub const MAX_ENTRIES: usize = 4096;
    /// The largest encoded path, in bytes.
    pub const MAX_PATH_BYTES: usize = 512;
    /// The largest text value, map key or path segment, in bytes of UTF-8.
    pub const MAX_STRING_BYTES: usize = 256;
    /// The largest encoded document, in bytes: 1 MiB (1,048,576 bytes).
    pub const MAX_DOCUMENT_BYTES: usize = 1 << 20;

    /// Builds a document from settings and entries, as a capture does.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::TooManyEntries`] for more than
    /// [`Self::MAX_ENTRIES`] entries, [`WorkspaceError::DuplicatePath`] for two
    /// entries at one path, the error of an invalid value as
    /// [`ViewState::with`] names them, and [`WorkspaceError::TooLarge`] for an
    /// encoding longer than [`Self::MAX_DOCUMENT_BYTES`]. A duplicate path
    /// means two components would be addressed identically, which is a
    /// container declaring its slots incorrectly rather than something to
    /// resolve by dropping one of them.
    pub fn new(
        settings: WorkspaceSettings,
        entries: impl IntoIterator<Item = (WorkspacePath, ViewState)>,
    ) -> Result<Self, WorkspaceError> {
        let mut collected: Vec<(WorkspacePath, ViewState)> = entries.into_iter().collect();
        if collected.len() > Self::MAX_ENTRIES {
            return Err(WorkspaceError::TooManyEntries);
        }
        collected.sort_by(|(left, _), (right, _)| left.cmp(right));
        for pair in collected.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(WorkspaceError::DuplicatePath(pair[0].0.to_string()));
            }
        }
        for (_, state) in &collected {
            for value in state.values.values() {
                value.validate()?;
            }
        }
        let workspace = Self {
            settings,
            entries: collected,
        };
        if workspace.encode().len() > Self::MAX_DOCUMENT_BYTES {
            return Err(WorkspaceError::TooLarge);
        }
        Ok(workspace)
    }
    /// The captured presentation settings.
    #[must_use]
    pub const fn settings(&self) -> &WorkspaceSettings {
        &self.settings
    }
    /// Replaces the presentation settings, for a host that owns them itself.
    pub const fn set_settings(&mut self, settings: WorkspaceSettings) {
        self.settings = settings;
    }
    /// Entries in document order.
    pub fn entries(&self) -> impl Iterator<Item = (&WorkspacePath, &ViewState)> {
        self.entries.iter().map(|(path, state)| (path, state))
    }
    /// The number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether the document holds no entries. Settings may still be present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A schema violation in a workspace document.
///
/// Every variant rejects the whole document: an unknown format is never applied
/// in part, and old data is never reinterpreted. Structure drift, where the
/// application's own tree changed between sessions, is not an error here; it is
/// reported per entry in a [`WorkspaceReport`].
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum WorkspaceError {
    /// The document was written by another schema revision.
    #[error("workspace schema version {found} is not supported (this build writes {supported})")]
    UnsupportedVersion {
        /// The revision the document declares.
        found: u32,
        /// The revision this build supports.
        supported: u32,
    },
    /// The text does not follow the document grammar.
    #[error("malformed workspace document: {0}")]
    Malformed(String),
    /// More entries than [`Workspace::MAX_ENTRIES`].
    #[error(
        "workspace document holds more than {} entries",
        Workspace::MAX_ENTRIES
    )]
    TooManyEntries,
    /// More bytes than [`Workspace::MAX_DOCUMENT_BYTES`].
    #[error(
        "workspace document is larger than {} bytes",
        Workspace::MAX_DOCUMENT_BYTES
    )]
    TooLarge,
    /// An encoded path longer than [`Workspace::MAX_PATH_BYTES`].
    #[error("workspace path is longer than {} bytes", Workspace::MAX_PATH_BYTES)]
    PathTooLong,
    /// Text longer than [`Workspace::MAX_STRING_BYTES`].
    #[error("workspace text is longer than {} bytes", Workspace::MAX_STRING_BYTES)]
    TextTooLong,
    /// A control character in text, a key or a path segment.
    #[error("workspace text contains a control character")]
    ControlCharacter,
    /// A value name or kind tag outside 1 to [`ViewState::MAX_NAME_BYTES`]
    /// bytes of `A-Za-z0-9_-`, or an empty map key.
    #[error("`{0}` is not a valid workspace name")]
    InvalidName(String),
    /// A map inside a map.
    #[error("workspace maps hold scalars only")]
    NestedMap,
    /// An infinite or NaN number, which is rejected rather than clamped.
    #[error("workspace numbers must be finite")]
    NotFinite,
    /// An empty path or an empty path segment.
    #[error("workspace paths need at least one nonempty segment")]
    EmptyPath,
    /// Two entries addressed the same path.
    #[error("two workspace entries address `{0}`")]
    DuplicatePath(String),
    /// Two entries of one map value used the same key.
    #[error("two workspace map entries use the key `{0}`")]
    DuplicateKey(String),
    /// The runtime refused the capture or restore itself: it is closed, or the
    /// call came from a phase that forbids it, or a setting was rejected.
    #[error(transparent)]
    Runtime(#[from] crate::RuntimeError),
    /// A participant could not build its state, so the capture or restore
    /// failed as a whole instead of leaving that participant out.
    ///
    /// Returned by [`Runtime::capture_workspace`](crate::Runtime::capture_workspace)
    /// and [`Runtime::restore_workspace`](crate::Runtime::restore_workspace)
    /// when [`Component::workspace_state`](crate::Component::workspace_state)
    /// fails, for example because a key the participant must record is too
    /// long for workspace text or holds a control character. Rename the key
    /// or turn the participant's persistence off where it has that option.
    #[error("workspace participant `{path}` could not build its state: {source}")]
    Participant {
        /// The participant's address, written as [`WorkspacePath`] displays
        /// it; segments that are not valid workspace text are written as they
        /// are.
        path: String,
        /// Why its state could not be built.
        #[source]
        source: Box<WorkspaceError>,
    },
}
crate::error::leaf_conversions!(WorkspaceError => WorkspaceError::Runtime);

impl From<WorkspaceError> for crate::RuntimeError {
    /// Carries a workspace capture, decode or restore failure out of an application callback with `?`, as
    /// [`RuntimeError::Application`](crate::RuntimeError::Application) holding
    /// the whole error, downcastable through
    /// [`ApplicationError::downcast_ref`](crate::ApplicationError::downcast_ref).
    /// [`WorkspaceError::Runtime`] becomes the runtime error it holds, so a runtime
    /// failure keeps one form. This is the error policy's callback-boundary
    /// exception; see [`RuntimeError`](crate::RuntimeError).
    fn from(error: WorkspaceError) -> Self {
        match error {
            WorkspaceError::Runtime(error) => error,
            error => Self::Application(crate::ApplicationError::new(error)),
        }
    }
}
/// Why a participant refused state addressed to it.
///
/// A rejection is reported, never silently ignored, and never partially
/// applied: a participant validates before it writes anything.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RestoreRejection {
    /// A named value is missing or holds another kind of value.
    #[error("`{0}` is missing or holds another kind of value")]
    Value(String),
    /// A named value this participant does not know, which a newer build may
    /// have written.
    #[error("`{0}` is not a value this component knows")]
    UnknownValue(String),
    /// The value no longer addresses anything, such as a tab key or a column key
    /// the application has removed.
    #[error("`{0}` no longer exists in this component")]
    Unresolved(String),
    /// The value lies outside what this participant accepts.
    #[error("`{0}` is outside the valid range")]
    OutOfRange(String),
    /// A gesture owns the value right now. Restoring under a drag or a held key
    /// would fight the user for it, so the entry is reported instead.
    #[error("a gesture is in progress")]
    Busy,
    /// The component could not build its own current state, which is needed
    /// to check the entry's kind, so nothing was offered to it. A mounted
    /// participant's failure fails an explicit
    /// [`Runtime::restore_workspace`](crate::Runtime::restore_workspace) as a
    /// whole; this rejection reports it for an entry that was retained and
    /// applied when its view mounted later.
    #[error("the component could not build its current state: {0}")]
    State(WorkspaceError),
    /// The component does not restore state, although something was addressed
    /// to it. A participant must override
    /// [`Component::restore_workspace_state`](crate::Component::restore_workspace_state).
    #[error("this component does not restore workspace state")]
    Unsupported,
    /// Applying an already validated entry failed in the runtime, for example
    /// with `Closed` after shutdown or `BorrowConflict` for a reentrant read.
    /// The participant leaves its previous state in place.
    #[error("the runtime refused the restored state: {0}")]
    Runtime(crate::RuntimeError),
}

/// Why an entry was not offered to a participant.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum IgnoredEntry {
    /// A deferred entry matched several components in a completed mount. No
    /// participant receives it. Correct the container's path markings before
    /// restoring again; an explicit restore into such a tree returns
    /// [`WorkspaceError::DuplicatePath`] before changing state.
    AmbiguousPath,
    /// A component is mounted at the path, but it is not a participant.
    NotAParticipant,
    /// A different kind of component now occupies the path.
    KindMismatch {
        /// The kind the document carried.
        found: String,
        /// The kind mounted at that path now.
        mounted: String,
    },
}

/// What a restore did with every entry in a document.
///
/// The report is returned rather than logged, because the library does not log,
/// and discarding it discards the only record that something did not apply.
/// Entries whose view is not mounted yet are `pending`: they are retained and
/// applied when a matching path mounts, at most once each, and
/// [`Runtime::workspace_report`](crate::Runtime::workspace_report) reflects them
/// once they do.
#[derive(Clone, Debug, Default, PartialEq)]
#[must_use]
pub struct WorkspaceReport {
    pub(crate) applied: Vec<WorkspacePath>,
    pub(crate) pending: Vec<WorkspacePath>,
    pub(crate) ignored: Vec<(WorkspacePath, IgnoredEntry)>,
    pub(crate) rejected: Vec<(WorkspacePath, RestoreRejection)>,
}

impl WorkspaceReport {
    /// Paths whose state a participant accepted.
    pub fn applied(&self) -> impl Iterator<Item = &WorkspacePath> {
        self.applied.iter()
    }
    /// Paths that are not mounted yet and may still apply.
    pub fn pending(&self) -> impl Iterator<Item = &WorkspacePath> {
        self.pending.iter()
    }
    /// Paths that will never apply to the current tree, with the reason.
    pub fn ignored(&self) -> impl Iterator<Item = (&WorkspacePath, &IgnoredEntry)> {
        self.ignored.iter().map(|(path, reason)| (path, reason))
    }
    /// Paths a participant refused, with its reason.
    pub fn rejected(&self) -> impl Iterator<Item = (&WorkspacePath, &RestoreRejection)> {
        self.rejected.iter().map(|(path, reason)| (path, reason))
    }
    /// Whether every entry applied.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.pending.is_empty() && self.ignored.is_empty() && self.rejected.is_empty()
    }
}

fn check_text(value: &str) -> Result<(), WorkspaceError> {
    if value.len() > Workspace::MAX_STRING_BYTES {
        return Err(WorkspaceError::TextTooLong);
    }
    if value.chars().any(char::is_control) {
        return Err(WorkspaceError::ControlCharacter);
    }
    Ok(())
}

fn check_name(value: &str) -> Result<(), WorkspaceError> {
    let valid = !value.is_empty()
        && value.len() <= ViewState::MAX_NAME_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(WorkspaceError::InvalidName(value.to_owned()))
    }
}

impl Workspace {
    /// Encodes the document as text the host stores wherever it likes.
    ///
    /// The encoding is deterministic: the same document always produces the same
    /// bytes. The first line names the format and the schema revision, an
    /// optional `settings` line carries presentation preferences, and one
    /// `entry` line follows per participant. Text is quoted, numbers are written
    /// so that decoding returns the same value, and every line ends with `\n`.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut out = format!("{MAGIC} {}\n", Self::SCHEMA_VERSION);
        let settings = &self.settings;
        if settings.zoom.is_some() || settings.mode.is_some() || settings.density.is_some() {
            out.push_str("settings");
            if let Some(zoom) = settings.zoom {
                out.push_str(&format!(" zoom={:?}", zoom.get()));
            }
            if let Some(mode) = settings.mode {
                out.push_str(&format!(" mode={}", theme_mode_name(mode)));
            }
            if let Some(density) = settings.density {
                out.push_str(&format!(" density={}", density_name(density)));
            }
            out.push('\n');
        }
        for (path, state) in &self.entries {
            out.push_str("entry ");
            write_quoted(&mut out, &path.to_string());
            out.push(' ');
            out.push_str(&state.kind);
            for (name, value) in &state.values {
                out.push(' ');
                out.push_str(name);
                out.push('=');
                write_value(&mut out, value);
            }
            out.push('\n');
        }
        out
    }

    /// Decodes a document, validating the whole of it before returning.
    ///
    /// Every token has exactly the one spelling [`Self::encode`] writes: the
    /// schema version and integers in plain decimal (`1`, not `+1` or `01`),
    /// numbers as Rust's shortest round-trip form (`0.5`, not `0.50` or `.5`),
    /// and text with only the escapes `\"` and `\\`. The line layout is
    /// tolerated where a text transport may change it: `\r\n` line endings,
    /// blank lines, a missing final newline and extra spaces between tokens.
    /// Such a document decodes to the same value, and encoding it again
    /// produces the canonical text.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::TooLarge`] for text longer than
    /// [`Self::MAX_DOCUMENT_BYTES`]; [`WorkspaceError::UnsupportedVersion`] for
    /// another schema revision; [`WorkspaceError::Malformed`] for text outside
    /// the grammar, including an unknown line, setting or escape, a repeated
    /// field or setting, a non-canonical number and a zoom outside its range;
    /// [`WorkspaceError::TooManyEntries`]; [`WorkspaceError::PathTooLong`]; and
    /// the errors of the constructors the document's parts pass through:
    /// [`WorkspaceError::EmptyPath`], [`WorkspaceError::InvalidName`],
    /// [`WorkspaceError::TextTooLong`], [`WorkspaceError::ControlCharacter`],
    /// [`WorkspaceError::NotFinite`], [`WorkspaceError::NestedMap`],
    /// [`WorkspaceError::DuplicateKey`] for a repeated map key and
    /// [`WorkspaceError::DuplicatePath`]. Nothing is applied from a document
    /// that fails here.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Workspace, WorkspaceError};
    /// let newer = "pert-ui-workspace 9\n";
    /// assert_eq!(
    ///     Workspace::decode(newer),
    ///     Err(WorkspaceError::UnsupportedVersion { found: 9, supported: 1 }),
    /// );
    /// ```
    pub fn decode(text: &str) -> Result<Self, WorkspaceError> {
        if text.len() > Self::MAX_DOCUMENT_BYTES {
            return Err(WorkspaceError::TooLarge);
        }
        let mut lines = text
            .lines()
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .filter(|line| !line.trim().is_empty());
        let header = lines.next().ok_or_else(|| malformed("missing header"))?;
        let version = header
            .strip_prefix(MAGIC)
            .and_then(|rest| rest.strip_prefix(' '))
            .ok_or_else(|| malformed("unknown document format"))?;
        let version = version.trim_matches(' ');
        let version: u32 = version
            .parse()
            .ok()
            .filter(|parsed: &u32| parsed.to_string() == version)
            .ok_or_else(|| malformed("unreadable schema version"))?;
        if version != Self::SCHEMA_VERSION {
            return Err(WorkspaceError::UnsupportedVersion {
                found: version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        let mut settings = WorkspaceSettings::default();
        let mut entries = Vec::new();
        let mut settings_seen = false;
        for line in lines {
            let mut scanner = Scanner::new(line);
            match scanner.word()? {
                "settings" => {
                    if settings_seen || !entries.is_empty() {
                        return Err(malformed("misplaced settings line"));
                    }
                    settings_seen = true;
                    settings = decode_settings(&mut scanner)?;
                }
                "entry" => entries.push(decode_entry(&mut scanner)?),
                other => return Err(malformed(&format!("unknown line `{other}`"))),
            }
            if entries.len() > Self::MAX_ENTRIES {
                return Err(WorkspaceError::TooManyEntries);
            }
        }
        Self::new(settings, entries)
    }
}

const MAGIC: &str = "pert-ui-workspace";

fn malformed(reason: &str) -> WorkspaceError {
    WorkspaceError::Malformed(reason.to_owned())
}

const fn theme_mode_name(mode: crate::ThemeMode) -> &'static str {
    match mode {
        crate::ThemeMode::Dark => "dark",
        crate::ThemeMode::Light => "light",
    }
}

const fn density_name(density: crate::Density) -> &'static str {
    match density {
        crate::Density::Compact => "compact",
        crate::Density::Standard => "standard",
        crate::Density::Comfortable => "comfortable",
    }
}

fn write_quoted(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(character),
        }
    }
    out.push('"');
}

fn write_value(out: &mut String, value: &WorkspaceValue) {
    match &value.0 {
        Repr::Flag(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Repr::Integer(number) => out.push_str(&number.to_string()),
        Repr::Number(number) => out.push_str(&format!("{number:?}")),
        Repr::Text(text) => write_quoted(out, text),
        Repr::Map(values) => {
            out.push('{');
            for (index, (key, value)) in values.iter().enumerate() {
                if index > 0 {
                    out.push(' ');
                }
                write_quoted(out, key);
                out.push('=');
                write_value(out, value);
            }
            out.push('}');
        }
    }
}

fn decode_settings(scanner: &mut Scanner<'_>) -> Result<WorkspaceSettings, WorkspaceError> {
    let mut settings = WorkspaceSettings::default();
    let mut seen = std::collections::BTreeSet::new();
    while !scanner.at_end() {
        let name = scanner.word()?;
        if !seen.insert(name) {
            return Err(malformed("duplicate setting"));
        }
        scanner.expect('=')?;
        match name {
            "zoom" => {
                let factor = canonical_number(scanner.bare()?)?;
                settings.zoom = Some(
                    crate::UiZoom::new(factor).map_err(|_| malformed("zoom outside 0.75..=2.0"))?,
                );
            }
            "mode" => {
                settings.mode = Some(match scanner.bare()? {
                    "dark" => crate::ThemeMode::Dark,
                    "light" => crate::ThemeMode::Light,
                    other => return Err(malformed(&format!("unknown theme mode `{other}`"))),
                });
            }
            "density" => {
                settings.density = Some(match scanner.bare()? {
                    "compact" => crate::Density::Compact,
                    "standard" => crate::Density::Standard,
                    "comfortable" => crate::Density::Comfortable,
                    other => return Err(malformed(&format!("unknown density `{other}`"))),
                });
            }
            other => return Err(malformed(&format!("unknown setting `{other}`"))),
        }
    }
    Ok(settings)
}

fn decode_entry(scanner: &mut Scanner<'_>) -> Result<(WorkspacePath, ViewState), WorkspaceError> {
    let path = scanner.quoted()?;
    let path = decode_path(&path)?;
    let mut state = ViewState::new(scanner.word()?)?;
    while !scanner.at_end() {
        let name = scanner.word()?.to_owned();
        if state.get(&name).is_some() {
            return Err(malformed("duplicate field"));
        }
        scanner.expect('=')?;
        let value = scanner.value(true)?;
        state = state.with(name, value)?;
    }
    Ok((path, state))
}

/// Reads a number spelled exactly as [`Workspace::encode`] writes it, so that
/// each value has one spelling: `0.5`, not `.5`, `0.50` or `+0.5`.
fn canonical_number(token: &str) -> Result<f64, WorkspaceError> {
    token
        .parse()
        .ok()
        .filter(|number: &f64| format!("{number:?}") == token)
        .ok_or_else(|| malformed(&format!("unreadable number `{token}`")))
}

fn decode_path(text: &str) -> Result<WorkspacePath, WorkspaceError> {
    if text.len() > Workspace::MAX_PATH_BYTES {
        return Err(WorkspaceError::PathTooLong);
    }
    let mut segments = Vec::new();
    let mut segment = String::new();
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        match character {
            '\\' => match characters.next() {
                Some(escaped @ ('\\' | '/')) => segment.push(escaped),
                _ => return Err(malformed("unknown escape in a path")),
            },
            '/' => segments.push(std::mem::take(&mut segment)),
            _ => segment.push(character),
        }
    }
    segments.push(segment);
    WorkspacePath::new(segments)
}

struct Scanner<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Scanner<'a> {
    const fn new(text: &'a str) -> Self {
        Self { text, position: 0 }
    }
    fn rest(&self) -> &'a str {
        &self.text[self.position..]
    }
    fn skip_spaces(&mut self) {
        while self.rest().starts_with(' ') {
            self.position += 1;
        }
    }
    fn at_end(&mut self) -> bool {
        self.skip_spaces();
        self.rest().is_empty()
    }
    fn expect(&mut self, character: char) -> Result<(), WorkspaceError> {
        if self.rest().starts_with(character) {
            self.position += character.len_utf8();
            Ok(())
        } else {
            Err(malformed(&format!("expected `{character}`")))
        }
    }
    /// A bare token of `A-Za-z0-9_-`, used for names, kinds and settings values.
    fn word(&mut self) -> Result<&'a str, WorkspaceError> {
        self.skip_spaces();
        let rest = self.rest();
        let end = rest
            .bytes()
            .position(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
            .unwrap_or(rest.len());
        if end == 0 {
            return Err(malformed("expected a name"));
        }
        self.position += end;
        Ok(&rest[..end])
    }
    /// A bare token up to the next space, used for numeric settings values.
    fn bare(&mut self) -> Result<&'a str, WorkspaceError> {
        self.skip_spaces();
        let rest = self.rest();
        let end = rest.find(' ').unwrap_or(rest.len());
        if end == 0 {
            return Err(malformed("expected a value"));
        }
        self.position += end;
        Ok(&rest[..end])
    }
    fn quoted(&mut self) -> Result<String, WorkspaceError> {
        self.skip_spaces();
        self.expect('"')?;
        let mut value = String::new();
        loop {
            let rest = self.rest();
            let mut characters = rest.chars();
            let character = characters
                .next()
                .ok_or_else(|| malformed("unterminated text"))?;
            self.position += character.len_utf8();
            match character {
                '"' => return Ok(value),
                '\\' => {
                    let escaped = characters
                        .next()
                        .ok_or_else(|| malformed("unterminated escape"))?;
                    self.position += escaped.len_utf8();
                    match escaped {
                        '"' | '\\' => value.push(escaped),
                        _ => return Err(malformed("unknown escape")),
                    }
                }
                _ => value.push(character),
            }
        }
    }
    fn value(&mut self, maps: bool) -> Result<WorkspaceValue, WorkspaceError> {
        self.skip_spaces();
        let rest = self.rest();
        if rest.starts_with('"') {
            return WorkspaceValue::text(self.quoted()?);
        }
        if rest.starts_with('{') {
            if !maps {
                return Err(WorkspaceError::NestedMap);
            }
            self.expect('{')?;
            let mut values = BTreeMap::new();
            loop {
                self.skip_spaces();
                if self.rest().starts_with('}') {
                    self.expect('}')?;
                    return WorkspaceValue::map(values);
                }
                let key = self.quoted()?;
                self.expect('=')?;
                if values.insert(key, self.value(false)?).is_some() {
                    return Err(malformed("duplicate map key"));
                }
            }
        }
        let token = {
            let end = rest.find([' ', '}']).unwrap_or(rest.len());
            self.position += end;
            &rest[..end]
        };
        match token {
            "true" => Ok(WorkspaceValue::flag(true)),
            "false" => Ok(WorkspaceValue::flag(false)),
            "" => Err(malformed("expected a value")),
            _ if token.contains(['.', 'e', 'E']) => {
                WorkspaceValue::number(canonical_number(token)?)
            }
            _ => {
                let number: i64 = token
                    .parse()
                    .ok()
                    .filter(|number: &i64| number.to_string() == token)
                    .ok_or_else(|| malformed(&format!("unreadable number `{token}`")))?;
                Ok(WorkspaceValue::integer(number))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(segments: &[&str]) -> WorkspacePath {
        WorkspacePath::new(segments.iter().copied()).expect("valid path")
    }

    fn document() -> Workspace {
        let settings = WorkspaceSettings::default()
            .with_zoom(Some(crate::UiZoom::new(1.25).expect("valid zoom")))
            .with_theme_mode(Some(crate::ThemeMode::Light))
            .with_density(Some(crate::Density::Compact));
        let tabs = ViewState::new("tabs")
            .and_then(|state| state.with("selected", WorkspaceValue::text("scope").unwrap()))
            .expect("valid state");
        let split = ViewState::new("split")
            .and_then(|state| state.with("ratio", WorkspaceValue::number(0.25).unwrap()))
            .expect("valid state");
        let table = ViewState::new("table")
            .and_then(|state| {
                state.with(
                    "widths",
                    WorkspaceValue::map([
                        ("name".to_owned(), WorkspaceValue::number(240.0).unwrap()),
                        ("value".to_owned(), WorkspaceValue::number(96.5).unwrap()),
                    ])
                    .unwrap(),
                )
            })
            .and_then(|state| state.with("visible", WorkspaceValue::flag(true)))
            .and_then(|state| state.with("revision", WorkspaceValue::integer(-3)))
            .expect("valid state");
        Workspace::new(
            settings,
            [
                (path(&["main", "tabs"]), tabs),
                (path(&["main", "split"]), split),
                (path(&["main", "table"]), table),
            ],
        )
        .expect("valid document")
    }

    #[test]
    fn round_trips_every_value_kind() {
        let workspace = document();
        let text = workspace.encode();
        assert_eq!(Workspace::decode(&text), Ok(workspace.clone()));
        assert_eq!(Workspace::decode(&text).unwrap().encode(), text);
    }

    #[test]
    fn encodes_deterministically_in_path_order() {
        let text = document().encode();
        let paths: Vec<_> = text
            .lines()
            .filter_map(|line| line.strip_prefix("entry \""))
            .filter_map(|line| line.split('"').next())
            .collect();
        assert_eq!(paths, ["main/split", "main/table", "main/tabs"]);
        assert_eq!(document().encode(), text);
    }

    #[test]
    fn keeps_integers_and_numbers_distinct() {
        let state = ViewState::new("k")
            .and_then(|state| state.with("whole", WorkspaceValue::integer(3)))
            .and_then(|state| state.with("real", WorkspaceValue::number(3.0).unwrap()))
            .unwrap();
        let text = Workspace::new(WorkspaceSettings::default(), [(path(&["a"]), state)])
            .unwrap()
            .encode();
        assert!(text.contains("real=3.0 whole=3"), "{text}");
        let decoded = Workspace::decode(&text).unwrap();
        let (_, state) = decoded.entries().next().unwrap();
        assert_eq!(state.integer("whole"), Ok(3));
        assert_eq!(state.number("real"), Ok(3.0));
        assert!(state.integer("real").is_err());
    }

    #[test]
    fn escapes_separators_and_quotes_in_paths_and_text() {
        let state = ViewState::new("k")
            .and_then(|state| state.with("name", WorkspaceValue::text("a\"b\\c").unwrap()))
            .unwrap();
        let workspace = Workspace::new(
            WorkspaceSettings::default(),
            [(path(&["with/slash", "with\\backslash"]), state)],
        )
        .unwrap();
        let decoded = Workspace::decode(&workspace.encode()).unwrap();
        let (path, state) = decoded.entries().next().unwrap();
        assert_eq!(
            path.segments().collect::<Vec<_>>(),
            ["with/slash", "with\\backslash"],
        );
        assert_eq!(state.text("name"), Ok("a\"b\\c"));
    }

    #[test]
    fn rejects_an_unsupported_version_without_reading_entries() {
        let text = "pert-ui-workspace 2\nentry \"a\" tabs selected=\"x\"\n";
        assert_eq!(
            Workspace::decode(text),
            Err(WorkspaceError::UnsupportedVersion {
                found: 2,
                supported: Workspace::SCHEMA_VERSION,
            }),
        );
    }

    #[test]
    fn rejects_malformed_documents() {
        let entry = |line: &str| format!("pert-ui-workspace 1\n{line}\n");
        let number = |token: &str| malformed(&format!("unreadable number `{token}`"));
        for (text, expected) in [
            (String::new(), malformed("missing header")),
            (
                "something else 1\n".into(),
                malformed("unknown document format"),
            ),
            (
                "pert-ui-workspace\n".into(),
                malformed("unknown document format"),
            ),
            (
                "pert-ui-workspace x\n".into(),
                malformed("unreadable schema version"),
            ),
            (
                entry("unknown \"a\" tabs"),
                malformed("unknown line `unknown`"),
            ),
            (
                entry("entry \"a\" tabs selected=\"x"),
                malformed("unterminated text"),
            ),
            (
                entry("entry \"a\" tabs selected=\"\\q\""),
                malformed("unknown escape"),
            ),
            (
                entry("entry \"a\" tabs selected"),
                malformed("expected `=`"),
            ),
            (entry("entry \"a\" tabs selected=zzz"), number("zzz")),
            (entry("entry \"\" tabs"), WorkspaceError::EmptyPath),
            (
                entry("settings unknown=1"),
                malformed("unknown setting `unknown`"),
            ),
            (
                entry("settings mode=sepia"),
                malformed("unknown theme mode `sepia`"),
            ),
            (
                entry("settings zoom=9.0"),
                malformed("zoom outside 0.75..=2.0"),
            ),
            (entry("entry \"a\" tabs n=nan"), number("nan")),
            (entry("entry \"a\" tabs n=inf"), number("inf")),
            (
                entry("entry \"a\" tabs w={\"k\"={\"j\"=1}}"),
                WorkspaceError::NestedMap,
            ),
            (
                entry("entry \"a\" tabs sel!ected=1"),
                malformed("expected `=`"),
            ),
            (
                "pert-ui-workspace 1\nentry \"a\" tabs x=1\nsettings zoom=1.0\n".into(),
                malformed("misplaced settings line"),
            ),
        ] {
            assert_eq!(Workspace::decode(&text), Err(expected), "{text:?}");
        }
    }

    #[test]
    fn every_token_has_one_spelling_and_the_layout_is_tolerated() {
        let entry = |line: &str| format!("pert-ui-workspace 1\n{line}\n");
        let number = |token: &str| malformed(&format!("unreadable number `{token}`"));
        let version = malformed("unreadable schema version");
        for (text, expected) in [
            ("pert-ui-workspace +1\n".into(), version.clone()),
            ("pert-ui-workspace 01\n".into(), version),
            (entry("entry \"a\" tabs x=+1"), number("+1")),
            (entry("entry \"a\" tabs x=01"), number("01")),
            (entry("entry \"a\" tabs x=0.50"), number("0.50")),
            (entry("entry \"a\" tabs x=.5"), number(".5")),
            (entry("entry \"a\" tabs x=1e0"), number("1e0")),
            (entry("settings zoom=1.00"), number("1.00")),
            (entry("settings zoom=+1.0"), number("+1.0")),
            // The writer escapes only quotes and backslashes; a slash inside
            // a path segment is `\/` at the path level, so `\\/` when quoted.
            (
                entry("entry \"a\" tabs t=\"\\/\""),
                malformed("unknown escape"),
            ),
            (entry("entry \"a\\/b\" tabs"), malformed("unknown escape")),
        ] {
            assert_eq!(Workspace::decode(&text), Err(expected), "{text:?}");
        }
        let canonical = "pert-ui-workspace 1\nentry \"a\\\\/b\" tabs t=\"/\" x=0.5 y=-3\n";
        let decoded = Workspace::decode(canonical).expect("canonical text decodes");
        assert_eq!(
            decoded
                .entries()
                .next()
                .map(|(path, _)| path.segments().collect::<Vec<_>>()),
            Some(vec!["a/b"])
        );
        assert_eq!(decoded.encode(), canonical);
        // CRLF line endings, blank lines, extra spaces and a missing final
        // newline are a transport's changes, not other spellings.
        let transported =
            "pert-ui-workspace  1 \r\n\r\nentry  \"a\\\\/b\"  tabs t=\"/\"   x=0.5 y=-3 ";
        assert_eq!(Workspace::decode(transported), Ok(decoded));
    }

    #[test]
    fn a_map_value_rejects_a_repeated_key_rather_than_dropping_one() {
        assert_eq!(
            WorkspaceValue::map([
                ("a".to_owned(), WorkspaceValue::integer(1)),
                ("a".to_owned(), WorkspaceValue::integer(2)),
            ]),
            Err(WorkspaceError::DuplicateKey("a".to_owned()))
        );
    }

    #[test]
    fn a_map_value_holds_scalars_only() {
        let inner = WorkspaceValue::map([("j".to_owned(), WorkspaceValue::integer(1))])
            .expect("a flat map is valid");
        assert_eq!(
            WorkspaceValue::map([("k".to_owned(), inner)]),
            Err(WorkspaceError::NestedMap)
        );
    }

    #[test]
    fn rejects_duplicate_paths_rather_than_dropping_one() {
        let state = ViewState::new("tabs").unwrap();
        assert_eq!(
            Workspace::new(
                WorkspaceSettings::default(),
                [
                    (path(&["a", "b"]), state.clone()),
                    (path(&["a", "b"]), state),
                ],
            ),
            Err(WorkspaceError::DuplicatePath("a/b".to_owned())),
        );
    }

    #[test]
    fn enforces_documented_limits() {
        assert_eq!(
            WorkspaceValue::text("x".repeat(Workspace::MAX_STRING_BYTES + 1)),
            Err(WorkspaceError::TextTooLong),
        );
        assert_eq!(
            WorkspacePath::new(["x".repeat(Workspace::MAX_STRING_BYTES + 1)]),
            Err(WorkspaceError::TextTooLong),
        );
        let deep = std::iter::repeat_n("x".repeat(64), 16);
        assert_eq!(WorkspacePath::new(deep), Err(WorkspaceError::PathTooLong));
        assert_eq!(WorkspacePath::new([""; 0]), Err(WorkspaceError::EmptyPath));
        assert_eq!(WorkspacePath::new([""]), Err(WorkspaceError::EmptyPath));
        assert_eq!(
            WorkspaceValue::text("a\nb"),
            Err(WorkspaceError::ControlCharacter),
        );
        assert_eq!(
            WorkspaceValue::number(f64::INFINITY),
            Err(WorkspaceError::NotFinite)
        );
        assert_eq!(
            ViewState::new("bad kind"),
            Err(WorkspaceError::InvalidName("bad kind".to_owned()))
        );
        let many = (0..=Workspace::MAX_ENTRIES).map(|index| {
            (
                path(&[&format!("view-{index}")]),
                ViewState::new("tabs").unwrap(),
            )
        });
        assert_eq!(
            Workspace::new(WorkspaceSettings::default(), many),
            Err(WorkspaceError::TooManyEntries),
        );
        assert_eq!(
            Workspace::decode(&"x".repeat(Workspace::MAX_DOCUMENT_BYTES + 1)),
            Err(WorkspaceError::TooLarge),
        );
    }

    #[test]
    fn expect_only_closes_the_field_set() {
        let state = ViewState::new("tabs")
            .and_then(|state| state.with("selected", WorkspaceValue::text("a").unwrap()))
            .and_then(|state| state.with("future", WorkspaceValue::flag(true)))
            .unwrap();
        assert_eq!(
            state.expect_only(&["selected"]),
            Err(RestoreRejection::UnknownValue("future".to_owned())),
        );
        assert_eq!(state.expect_only(&["selected", "future"]), Ok(()));
    }

    #[test]
    fn settings_survive_a_round_trip_and_stay_optional() {
        let empty = Workspace::new(WorkspaceSettings::default(), []).unwrap();
        assert_eq!(empty.encode(), "pert-ui-workspace 1\n");
        assert_eq!(Workspace::decode(&empty.encode()), Ok(empty));
        let decoded = Workspace::decode(&document().encode()).unwrap();
        assert_eq!(
            decoded.settings().zoom().map(crate::UiZoom::get),
            Some(1.25)
        );
        assert_eq!(
            decoded.settings().theme_mode(),
            Some(crate::ThemeMode::Light)
        );
        assert_eq!(decoded.settings().density(), Some(crate::Density::Compact));
    }
}

// The serde representation of the document; the public types document it.
#[cfg(feature = "serde")]
mod serde_impls {
    use std::collections::BTreeMap;

    use serde::{
        Deserialize, Deserializer, Serialize, Serializer,
        de::{Error as _, MapAccess, Visitor},
    };

    use super::{
        ViewState, Workspace, WorkspaceError, WorkspacePath, WorkspaceSettings, WorkspaceValue,
    };

    /// A map that refuses a repeated key, as the text decoder does; serde's
    /// own map deserialization would silently keep the last value.
    fn unique_map<'de, D, T>(deserializer: D) -> Result<BTreeMap<String, T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        struct Unique<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for Unique<T> {
            type Value = BTreeMap<String, T>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a map with distinct keys")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = BTreeMap::new();
                while let Some((key, value)) = access.next_entry::<String, T>()? {
                    if values.contains_key(&key) {
                        return Err(A::Error::custom(format!("duplicate key {key:?}")));
                    }
                    values.insert(key, value);
                }
                Ok(values)
            }
        }
        deserializer.deserialize_map(Unique(std::marker::PhantomData))
    }

    /// The representation without its validation; every value goes through
    /// [`WorkspaceValue::validate`] before it becomes one.
    #[derive(Deserialize)]
    enum Unchecked {
        Flag(bool),
        Integer(i64),
        Number(f64),
        Text(String),
        Map(#[serde(deserialize_with = "unique_map")] BTreeMap<String, Unchecked>),
    }

    impl From<Unchecked> for WorkspaceValue {
        fn from(value: Unchecked) -> Self {
            match value {
                Unchecked::Flag(flag) => Self(super::Repr::Flag(flag)),
                Unchecked::Integer(integer) => Self(super::Repr::Integer(integer)),
                Unchecked::Number(number) => Self(super::Repr::Number(number)),
                Unchecked::Text(text) => Self(super::Repr::Text(text)),
                Unchecked::Map(values) => Self(super::Repr::Map(
                    values
                        .into_iter()
                        .map(|(key, value)| (key, Self::from(value)))
                        .collect(),
                )),
            }
        }
    }

    impl<'de> Deserialize<'de> for WorkspaceValue {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let value = Self::from(Unchecked::deserialize(deserializer)?);
            value.validate().map_err(D::Error::custom)?;
            Ok(value)
        }
    }

    impl Serialize for WorkspacePath {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for WorkspacePath {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Self::new(Vec::<String>::deserialize(deserializer)?).map_err(D::Error::custom)
        }
    }

    #[derive(Serialize)]
    struct StateWrite<'a> {
        kind: &'a str,
        values: &'a BTreeMap<String, WorkspaceValue>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct StateRead {
        kind: String,
        #[serde(deserialize_with = "unique_map")]
        values: BTreeMap<String, WorkspaceValue>,
    }

    impl Serialize for ViewState {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            StateWrite {
                kind: &self.kind,
                values: &self.values,
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ViewState {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr = StateRead::deserialize(deserializer)?;
            let mut state = Self::new(repr.kind).map_err(D::Error::custom)?;
            for (name, value) in repr.values {
                state = state.with(name, value).map_err(D::Error::custom)?;
            }
            Ok(state)
        }
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct EntryRepr<'a> {
        path: std::borrow::Cow<'a, WorkspacePath>,
        state: std::borrow::Cow<'a, ViewState>,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct DocumentRepr<'a> {
        schema_version: u32,
        settings: WorkspaceSettings,
        entries: Vec<EntryRepr<'a>>,
    }

    impl Serialize for Workspace {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            DocumentRepr {
                schema_version: Self::SCHEMA_VERSION,
                settings: self.settings,
                entries: self
                    .entries
                    .iter()
                    .map(|(path, state)| EntryRepr {
                        path: std::borrow::Cow::Borrowed(path),
                        state: std::borrow::Cow::Borrowed(state),
                    })
                    .collect(),
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Workspace {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr = DocumentRepr::deserialize(deserializer)?;
            if repr.schema_version != Self::SCHEMA_VERSION {
                return Err(D::Error::custom(WorkspaceError::UnsupportedVersion {
                    found: repr.schema_version,
                    supported: Self::SCHEMA_VERSION,
                }));
            }
            Self::new(
                repr.settings,
                repr.entries
                    .into_iter()
                    .map(|entry| (entry.path.into_owned(), entry.state.into_owned())),
            )
            .map_err(D::Error::custom)
        }
    }
}
