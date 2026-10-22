//! Development-only UI checks and transport-independent validation observations.

use crate::{KeyCode, Modifiers};

#[cfg(debug_assertions)]
pub(crate) mod development;

/// Whether a component intentionally intersects peer components in its layer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum OverlapPolicy {
    /// Report intersections with other disallowing peers. The default for controls.
    #[default]
    Disallow,
    /// This component intentionally overlaps, such as an overlay/background.
    /// Does not suppress numeric, color or shortcut validation.
    Allow,
}

/// Dispatch scope of a declared shortcut, used to identify active competitors.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ShortcutScope {
    /// Active whenever the declaring component is visible and the binding enabled.
    #[default]
    Window,
    /// Active only while its declaring component has focus and is visible.
    FocusedComponent,
}

/// A component's shortcut declaration for development conflict checking.
///
/// This does not register or execute an action. Use it for chords a component
/// handles in its own `key_input`; commands registered with
/// `Runtime::register_command` are validated by the command registry instead. The
/// declaration must describe its actual input handling; arbitrary code inside
/// `key_input` cannot be inferred.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ShortcutBinding {
    /// Physical main key; modifiers are represented independently below.
    pub key: KeyCode,
    /// Exact combined Shift/Control/Alt/Super state required by this binding.
    pub modifiers: Modifiers,
    /// Nonempty action description included in diagnostics.
    pub action: String,
    /// Window-wide or focused-component dispatch scope.
    pub scope: ShortcutScope,
    /// Whether this binding currently participates in dispatch.
    pub enabled: bool,
}

impl ShortcutBinding {
    /// Declares an enabled window shortcut without modifiers.
    ///
    /// The action has application-defined meaning. An empty
    /// action is reported by development validation rather than silently repaired.
    #[must_use]
    pub fn new(key: KeyCode, action: impl Into<String>) -> Self {
        Self {
            key,
            modifiers: Modifiers::default(),
            action: action.into(),
            scope: ShortcutScope::Window,
            enabled: true,
        }
    }
}

/// Component/layout semantics needed to distinguish errors from intentional design.
///
/// Defaults require no setup: peers must not overlap, no clip is assumed and no
/// shortcuts are declared. Ordinary library controls will populate these from
/// their actual layout/input metadata. Applications only describe exceptions or
/// custom components. The engine sees intent only through these hints: a custom
/// component that declares nothing is checked with the defaults, and nothing is
/// inferred from its drawing or input handling. These observations remain inspectable in release builds,
/// but the validation engine and its automatic collection are compiled out there.
///
/// A component builds it from [`Default`], with `..ValidationHints::default()`
/// in a struct expression or by assigning fields of a default value, so a field
/// added in a later version does not break it.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ValidationHints {
    /// Intentional intersection policy for peer components.
    pub overlap: OverlapPolicy,
    /// Actual effective ancestor clip `[x, y, width, height]` in root logical pixels.
    /// `None` means no additional known clip, not a fabricated viewport.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::serde_float::option_array4")
    )]
    pub clip: Option<[f32; 4]>,
    /// Whether a visible measured component needs a nonempty hit area.
    pub interactive: bool,
    /// Declared keyboard bindings; inactive scopes do not conflict.
    pub shortcuts: Vec<ShortcutBinding>,
}

/// Stable diagnostic classification. Codes describe development issues, not errors
/// that a runtime may safely ignore when enforcing identity or numeric invariants.
///
/// Codes about measured geometry (bounds, clips, padding against the box, empty
/// hit areas, collapsed parents, escaping children and overlaps) are reported
/// only for windows laid out with a viewport that has area; see
/// [`Runtime::validation_report`](crate::Runtime::validation_report).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ValidationCode {
    /// Updates/layout/focus/hover did not settle within the runtime's wave limit.
    UpdateLimit,
    /// A mount attempted to reuse an already-mounted key.
    DuplicateKey,
    /// Bounds contain non-finite coordinates or negative width/height.
    InvalidBounds,
    /// Clip data contains non-finite coordinates or negative width/height.
    InvalidClip,
    /// Margin contains a non-finite value. Finite negative margins are permitted.
    InvalidMargin,
    /// Padding contains a negative or non-finite value.
    InvalidPadding,
    /// Padding leaves negative content width or height.
    PaddingExceedsBounds,
    /// Foreground/background/border has a non-finite or out-of-range RGBA channel.
    InvalidColor,
    /// A painted border reports a non-finite or negative stroke width.
    InvalidBorderWidth,
    /// A visible interactive component has zero measured width or height.
    /// Fully clipped nonempty components, such as scrolled content, are permitted.
    EmptyHitArea,
    /// Visible, clipped peer rectangles intersect without explicit permission.
    /// A component permits it by reporting [`OverlapPolicy::Allow`] in
    /// [`ComponentDetails::validation`](crate::ComponentDetails::validation),
    /// and the children of a parent whose
    /// [`Component::stacks_children`](crate::Component::stacks_children) is true
    /// may overlap each other.
    UnexpectedOverlap,
    /// A virtualized collection (a [`VirtualList`](crate::VirtualList), also the
    /// body of a [`Table`](crate::Table) or [`Tree`](crate::Tree)) of at least 100
    /// rows was given its whole content extent, so it cannot scroll itself and
    /// mounts every row, while less than half of that extent is visible in its
    /// window: an ancestor offered it an unbounded height (or width), as a
    /// default [`ScrollArea`](crate::ScrollArea), [`Tabs`](crate::Tabs) or
    /// [`RegionLayout`](crate::RegionLayout) does. See the list's
    /// [bounded extent](crate::VirtualList#a-bounded-extent). Schema 14 added
    /// this code.
    UnboundedCollection,
    /// A visible component's visible bounds leave its parent's bounds although no
    /// clip removes the excess. Content a scrolled or clipping ancestor cuts off is
    /// permitted, and so is a component with [`OverlapPolicy::Allow`], such as an
    /// overlay or a transient surface.
    EscapesParent,
    /// A visible component has zero width or height while a child of it is
    /// visibly drawn, so the parent reserved no space for content that shows.
    CollapsedParent,
    /// A shortcut declaration has no non-whitespace action description.
    InvalidShortcut,
    /// Two enabled bindings compete for the same key/modifier chord.
    ShortcutConflict,
}

/// One actionable development error, referring to every affected component.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ValidationIssue {
    /// Machine-readable error category for tools and overlays.
    pub code: ValidationCode,
    /// Runtime-local component incarnations; pair errors include both components.
    pub components: Vec<u64>,
    /// Human-readable explanation with component keys and relevant details.
    pub message: String,
}

/// Development error marker consumed by inspection and drawn by
/// [`DebugOverlay::Issues`](crate::DebugOverlay::Issues).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ValidationMarker {
    /// Component incarnation to mark, even if geometry is not yet available.
    pub component: u64,
    /// Safe visible marker bounds in root logical pixels, if known and nonempty.
    /// Missing bounds still allow the inspector's component row to be marked.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::serde_float::option_array4")
    )]
    pub bounds: Option<[f32; 4]>,
    /// Error outline color in linear RGBA, currently opaque red.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::array4"))]
    pub color: [f32; 4],
    /// Error categories affecting this component, without duplicates.
    pub codes: Vec<ValidationCode>,
}

/// Latest development validation results, carried by inspection snapshots since
/// schema revision 2.
///
/// Default release builds report `enabled = false` and empty collections. The
/// development engine/logger/cache are absent from those builds. Runtime validity
/// checks (for example rejecting duplicate mounts) still protect all builds.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ValidationReport {
    /// Failure writing the error log, if any. The runtime keeps running and retries
    /// current unlogged issues on the next flush without repeating unchanged
    /// geometry validation. Completed issue lines are not repeated; an interrupted
    /// line may repeat its prefix. This alone does not make [`Self::pending`] true
    /// or schedule a repaint. Inspection must not hide this failure.
    pub logging_error: Option<String>,
    /// Whether automatic development validation was compiled into this runtime.
    pub enabled: bool,
    /// Some changed component data has not yet been validated by a completed flush.
    pub pending: bool,
    /// Current issues, rejected duplicate-key attempts and the latest update limit.
    pub issues: Vec<ValidationIssue>,
    /// Per-component marker data, drawn into the scene only while
    /// [`DebugOverlay::Issues`](crate::DebugOverlay::Issues) is set.
    pub markers: Vec<ValidationMarker>,
}
