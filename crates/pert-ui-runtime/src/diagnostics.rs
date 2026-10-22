use crate::{Invalidation, KeyCode, ParameterId};
use std::{
    collections::{BTreeMap, VecDeque},
    time::Duration,
};

/// Event-history recording mode. Off is the allocation-free default.
///
/// Recording is independent of what is drawn: [`DebugOverlay`] switches the
/// development overlay, so a host that records history sees no markers and a
/// host that wants markers records nothing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum DebugMode {
    /// No event history recording. On-demand inspection works.
    #[default]
    Off,
    /// Record bounded event history for local tools or an application's tooling adapter.
    Inspect,
}

/// What development drawing [`Runtime::paint`](crate::Runtime::paint) appends
/// above the composed scene. Set with
/// [`Runtime::set_debug_overlay`](crate::Runtime::set_debug_overlay).
///
/// The overlay is part of the [`DisplayList`](crate::DisplayList), so every
/// backend, headless test and captured scene sees the same drawing. It consists
/// of plain [`FillRect`](crate::DrawCommand::FillRect) commands in fixed colors
/// that do not depend on the theme, carries no clip, and enters neither the
/// shape cache nor a resource table. It changes no layout, hit testing, focus,
/// inspected component data or workspace document.
///
/// Both drawing modes read what the development validation engine collected and
/// therefore draw only in builds with `debug_assertions`; a default release
/// build appends nothing in any mode. Composing the overlay never invalidates
/// anything: the scene is marked for one repaint when the setting changes and
/// when a completed validation pass changes what the overlay shows, so an idle
/// window with stable issues stays idle.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum DebugOverlay {
    /// Appends nothing and reads nothing. The default.
    #[default]
    Off,
    /// Outlines every component a [`ValidationMarker`](crate::ValidationMarker)
    /// names, in the marker's color, two physical pixels wide.
    ///
    /// At most [`Runtime::MAX_OVERLAY_MARKERS`](crate::Runtime::MAX_OVERLAY_MARKERS)
    /// outlines are drawn per window. Markers beyond that bound, and markers
    /// without visible bounds (a hidden or unmeasured component), are counted in
    /// a badge in the window's top-right corner instead of being dropped. The
    /// badge shows the count in a built-in block numeral, up to `999+`.
    Issues,
    /// Outlines the visible part of every presented, measured component's border
    /// box one physical pixel wide, and its content box where the component
    /// reports valid padding, in composition-independent incarnation order.
    ///
    /// At most [`Runtime::MAX_OVERLAY_BOXES`](crate::Runtime::MAX_OVERLAY_BOXES)
    /// components are outlined per window; the rest are left out. This mode does
    /// not draw validation markers.
    Boxes,
}

/// Owned application-provided diagnostic data, with no live UI handles.
///
/// A transport adapter chooses serialization and must represent non-finite numbers
/// explicitly if a host reports them. These are diagnostic observations, not
/// accepted input values or a project persistence schema.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum InspectValue {
    /// A boolean observation.
    Bool(bool),
    /// A numeric observation in units described by its property name/provider.
    Number(#[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f64"))] f64),
    /// UTF-8 text deliberately exposed by the application/component.
    Text(String),
    /// Ordered observations.
    List(Vec<InspectValue>),
    /// Named nested observations.
    Object(BTreeMap<String, InspectValue>),
}

/// Optional geometry/style observations, reported by inspection and drawn by
/// the [`DebugOverlay::Boxes`] development overlay.
///
/// Values must describe actual retained layout; absence means not measured, not
/// zero. Runtime supplies composed allocations; components supply known style
/// observations, using logical pixels and linear RGBA colors.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct BoxModelInspection {
    /// Border-box `[x, y, width, height]` in root/window logical pixels.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::serde_float::option_array4")
    )]
    pub border_box: Option<[f32; 4]>,
    /// Margins `[top, right, bottom, left]` in logical pixels.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::serde_float::option_array4")
    )]
    pub margin: Option<[f32; 4]>,
    /// Padding `[top, right, bottom, left]` in logical pixels.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::serde_float::option_array4")
    )]
    pub padding: Option<[f32; 4]>,
    /// Effective foreground color `[red, green, blue, alpha]`, each in `0..=1`.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::serde_float::option_array4")
    )]
    pub foreground: Option<[f32; 4]>,
    /// Effective background color `[red, green, blue, alpha]`, each in `0..=1`.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::serde_float::option_array4")
    )]
    pub background: Option<[f32; 4]>,
    /// Painted outline stroke, absent until a border is actually painted.
    pub border: Option<BorderInspection>,
}

/// A painted outline's measured stroke, inside the border box.
///
/// Absence means no border was painted, not a zero-width border. Containers
/// report the stroke of the last paint, so a role change is visible only after
/// the next paint.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct BorderInspection {
    /// Stroke width in logical pixels, from the effective `MetricRole::BorderWidth`.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f32"))]
    pub width: f32,
    /// Stroke color `[red, green, blue, alpha]`, each in `0..=1`.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::array4"))]
    pub color: [f32; 4],
}

/// What kind of thing a component is to a person or a tool, independent of how
/// it is drawn.
///
/// The vocabulary is general user-interface structure, with no application
/// meaning. It is deliberately close to what platform accessibility
/// interfaces expect, so that a later adapter maps roles instead of guessing
/// them from drawing, but no such adapter or platform dependency exists yet.
/// Layout-only containers report no semantics at all rather than a generic role.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SemanticRole {
    /// Static text.
    Label,
    /// An action that runs when activated.
    Button,
    /// Navigation to another place, such as a section of the application,
    /// that happens when activated.
    Link,
    /// A two-state box that is checked or not.
    Checkbox,
    /// A two-state on/off switch.
    Switch,
    /// Mutually exclusive options presented as a list of radio buttons.
    RadioGroup,
    /// Mutually exclusive options presented as joined segments.
    SegmentedChoice,
    /// A numeric value adjusted along a track.
    Slider,
    /// A numeric value edited as text.
    NumberInput,
    /// A single line of editable text.
    TextInput,
    /// A single line of editable text that is a query, typically filtering
    /// results presented beside it.
    SearchInput,
    /// Several lines of editable text.
    MultilineTextInput,
    /// A value presented for reading only.
    Readout,
    /// How far a task has come, or that it is running or how it ended.
    ProgressIndicator,
    /// A closed control that opens a list of options.
    ComboBox,
    /// A list from which one option is chosen.
    ListBox,
    /// A control or surface offering commands.
    Menu,
    /// A row of titles, each opening a menu of an application's commands.
    MenuBar,
    /// A strip of titles selecting which view is shown.
    TabList,
    /// The content belonging to one tab or other retained view.
    View,
    /// Related content under an optional heading.
    Group,
    /// One control together with its label, help text and status.
    Field,
    /// A viewport over content larger than itself.
    ScrollArea,
    /// A sequence of items, possibly mounted only in part.
    List,
    /// An expandable hierarchy of items, possibly mounted only in part.
    Tree,
    /// Rows of cells under column headers, possibly mounted only in part.
    Table,
    /// One cell of a table row, named by its column.
    Cell,
    /// A row of commands and compact controls.
    Toolbar,
    /// A surface that holds a task until it is answered or dismissed.
    Dialog,
    /// A dialog that interrupts to confirm or report something.
    AlertDialog,
    /// Persistent feedback in the ordinary layout.
    Alert,
    /// A stack of transient notifications.
    Notifications,
    /// Delayed help for another component.
    Tooltip,
    /// An anchored surface with arbitrary content.
    Popover,
    /// A movable window inside the application view.
    Window,
    /// A boundary that resizes the two regions beside it.
    Splitter,
    /// A fixed visual separator.
    Separator,
    /// A picture; its name is the text alternative for a person who cannot see
    /// it, and a purely decorative image has none.
    Image,
}

/// What a component currently presents, in a typed form a tool can compare.
///
/// Text is what a reader sees, already formatted and localized by whoever
/// supplied it. Numbers are canonical values in the component's own domain, not
/// normalized positions and not display strings.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub enum SemanticValue {
    /// Presented or edited text.
    Text(String),
    /// A two-state value.
    Bool(bool),
    /// A number with the bounds of its domain where they are known.
    Number {
        /// The current canonical value.
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f64"))]
        value: f64,
        /// The lowest accepted value, if bounded below.
        #[cfg_attr(
            feature = "serde",
            serde(default, with = "crate::serde_float::option_f64")
        )]
        minimum: Option<f64>,
        /// The highest accepted value, if bounded above.
        #[cfg_attr(
            feature = "serde",
            serde(default, with = "crate::serde_float::option_f64")
        )]
        maximum: Option<f64>,
    },
    /// One of several options.
    Choice {
        /// Position of the chosen option, absent when the bound value matches none.
        selected: Option<usize>,
        /// How many options are offered.
        count: usize,
        /// The chosen option's presented label, when one is chosen.
        label: Option<String>,
    },
}

/// Something a person can currently do to a component.
///
/// Actions describe what the component offers right now; a disabled or read-only
/// component offers fewer. Inspection is read-only, so listing an action does not
/// provide a way to invoke it: tools drive components through the same input the
/// runtime accepts from a host.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SemanticAction {
    /// Run the component's action.
    Activate,
    /// Flip a two-state value.
    Toggle,
    /// Replace the value, for example by typing.
    SetValue,
    /// Step the value up.
    Increment,
    /// Step the value down.
    Decrement,
    /// Choose one of the offered options.
    Select,
    /// Open the component's surface or content.
    Expand,
    /// Close the component's surface or content.
    Collapse,
    /// Dismiss the component.
    Dismiss,
    /// Change which part of the content is in view.
    Scroll,
    /// Change the component's extent.
    Resize,
    /// Change the component's position.
    Move,
}

/// One row of a component that presents several choices or commands itself,
/// such as an option, a menu command or a tab title.
///
/// Rows of this kind are drawn by one component and are not components, so they
/// have no identity, bounds or focus of their own in the snapshot; their order is
/// their presentation order. A component whose rows are real child components
/// reports none and lets the children describe themselves.
///
/// ```
/// use pert_ui_runtime::{SemanticAction, SemanticItem, SemanticRole, Semantics};
/// let semantics = Semantics::new(SemanticRole::Menu).items([
///     SemanticItem::new("Copy").action(SemanticAction::Activate),
///     SemanticItem::new("Paste").disabled(true).action(SemanticAction::Activate),
/// ]);
/// assert_eq!(semantics.items[0].actions, [SemanticAction::Activate]);
/// assert!(semantics.items[1].disabled && semantics.items[1].actions.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct SemanticItem {
    /// The row's presented name; absent when it has none, never empty.
    pub name: Option<String>,
    /// Whether this row is the chosen or highlighted one.
    pub selected: bool,
    /// Whether this row cannot be acted on. A disabled row offers no actions.
    pub disabled: bool,
    /// Whether the row's own content, such as a submenu, is open; absent when
    /// the row has none.
    pub expanded: Option<bool>,
    /// Whether a setting row, such as a checkable or exclusive menu command, is
    /// on; absent for a row that is not a setting.
    pub checked: Option<bool>,
    /// The keyboard shortcut the row presents, in the running platform's
    /// convention; absent when it shows none.
    pub shortcut: Option<String>,
    /// What can currently be done to this row, sorted and without duplicates.
    pub actions: Vec<SemanticAction>,
}
impl SemanticItem {
    /// Creates an enabled, unselected row; empty text leaves the name absent.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()).filter(|name| !name.is_empty()),
            selected: false,
            disabled: false,
            expanded: None,
            checked: None,
            shortcut: None,
            actions: Vec::new(),
        }
    }
    /// Marks the row as the chosen or highlighted one.
    #[must_use]
    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    /// Marks the row as unavailable, which withdraws its actions, including any
    /// offered afterwards. A later `disabled(false)` does not restore the
    /// withdrawn actions; only actions offered after it apply.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        if disabled {
            self.actions.clear();
        }
        self
    }
    /// Reports whether the row's own content is open.
    #[must_use]
    pub const fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }
    /// Reports the on/off state of a setting row.
    #[must_use]
    pub const fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
    /// Reports the keyboard shortcut the row presents; empty text leaves it absent.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into()).filter(|shortcut| !shortcut.is_empty());
        self
    }
    /// Offers one action while the row is not disabled, keeping the list sorted
    /// and free of duplicates.
    #[must_use]
    pub fn action(mut self, action: SemanticAction) -> Self {
        if !self.disabled
            && let Err(index) = self.actions.binary_search(&action)
        {
            self.actions.insert(index, action);
        }
        self
    }
}

/// A component's typed meaning: role, name, value, state and offered actions.
///
/// This is the contract a future accessibility adapter, an inspector and
/// automated tests share, so none of them has to interpret the free-form
/// [`ComponentDetails::properties`]. Every built-in control reports it from its
/// retained state, and a custom component does the same from
/// [`Component::inspect`](crate::Component::inspect):
///
/// ```
/// use pert_ui_runtime::{ComponentDetails, SemanticAction, SemanticRole, SemanticValue, Semantics};
/// let details = ComponentDetails {
///     semantics: Some(
///         Semantics::new(SemanticRole::Switch)
///             .name("Metronome")
///             .value(SemanticValue::Bool(true))
///             .action(SemanticAction::Toggle),
///     ),
///     ..ComponentDetails::default()
/// };
/// let semantics = details.semantics.as_ref().unwrap();
/// assert_eq!(semantics.name.as_deref(), Some("Metronome"));
/// assert_eq!(semantics.actions, [SemanticAction::Toggle]);
/// ```
///
/// Everything is an owned observation captured on the UI thread. Absent means
/// not known or not applicable, never an empty placeholder. Reporting semantics
/// schedules nothing and is evaluated only when a snapshot is captured.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct Semantics {
    /// What kind of thing this is.
    pub role: SemanticRole,
    /// The short name a person would call it by, such as a button's text or a
    /// field's label. A component without a name of its own may receive one from
    /// an enclosing [`SemanticRole::Field`]; see
    /// [`ComponentInspection::labelled_by`].
    pub name: Option<String>,
    /// Longer supporting text, such as help text or a status message.
    pub description: Option<String>,
    /// What it currently presents.
    pub value: Option<SemanticValue>,
    /// What can currently be done to it, sorted and without duplicates.
    pub actions: Vec<SemanticAction>,
    /// The rows this component presents itself, in presentation order; empty
    /// when it has none or when its rows are child components.
    pub items: Vec<SemanticItem>,
    /// Whether interaction is unavailable. A disabled component offers no actions.
    pub disabled: bool,
    /// Whether the value can be read and selected but not changed.
    pub read_only: bool,
    /// Whether the application or the component reports the content as not
    /// acceptable, such as a field in error or an unparsable draft.
    pub invalid: bool,
    /// Whether an owned surface or content is open; absent when there is none.
    pub expanded: Option<bool>,
    /// Whether the application requires a value, such as a mandatory form
    /// field. It is presentation only: the component reports it and never
    /// enforces it by itself. Schema 13 added this field.
    pub required: bool,
    /// Whether the value is secret, such as a password: [`Self::value`] is
    /// then absent, and search, history and other tooling built on semantics
    /// must not read or record the content. Schema 13 added this field.
    pub protected: bool,
    /// Whether this component describes its immediate children: a child with no
    /// name of its own receives this name, and every child that reports semantics
    /// receives this description, unless it has one, and this invalid state. A
    /// [`SemanticRole::Field`] sets it, which is how a wrapped control is named.
    pub names_children: bool,
}
impl Semantics {
    /// Creates semantics with a role and nothing else known.
    #[must_use]
    pub const fn new(role: SemanticRole) -> Self {
        Self {
            role,
            name: None,
            description: None,
            value: None,
            actions: Vec::new(),
            items: Vec::new(),
            disabled: false,
            read_only: false,
            invalid: false,
            expanded: None,
            required: false,
            protected: false,
            names_children: false,
        }
    }
    /// Replaces the role, for an owner that presents another component's
    /// semantics as its own kind of thing.
    #[must_use]
    pub const fn role(mut self, role: SemanticRole) -> Self {
        self.role = role;
        self
    }
    /// Sets the name; empty text leaves it absent.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into()).filter(|name| !name.is_empty());
        self
    }
    /// Sets the description; empty text leaves it absent.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into()).filter(|text| !text.is_empty());
        self
    }
    /// Sets the presented value; a [protected](Self::protected) value stays
    /// absent.
    #[must_use]
    pub fn value(mut self, value: SemanticValue) -> Self {
        if !self.protected {
            self.value = Some(value);
        }
        self
    }
    /// Offers one action while not disabled, keeping the list sorted and free
    /// of duplicates. A disabled control offers no actions, whichever of this
    /// and [`Self::disabled`] is called first.
    #[must_use]
    pub fn action(mut self, action: SemanticAction) -> Self {
        if !self.disabled
            && let Err(index) = self.actions.binary_search(&action)
        {
            self.actions.insert(index, action);
        }
        self
    }
    /// Offers several actions while not disabled, appending them to those
    /// offered earlier as [`Self::action`] does.
    #[must_use]
    pub fn actions(self, actions: impl IntoIterator<Item = SemanticAction>) -> Self {
        actions.into_iter().fold(self, Self::action)
    }
    /// Sets the rows this component presents itself, replacing earlier ones.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = SemanticItem>) -> Self {
        self.items = items.into_iter().collect();
        self
    }
    /// Marks interaction as unavailable when `disabled` is true, which also
    /// withdraws every action offered so far or later through
    /// [`Semantics::enabled_actions`]. A later `disabled(false)` does not
    /// restore the withdrawn actions; only actions offered after it apply.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        if disabled {
            self.actions.clear();
        }
        self
    }
    /// Offers the actions only while not disabled, so a control states its
    /// actions once and its enabled flag decides whether they apply.
    #[must_use]
    pub fn enabled_actions(self, actions: impl IntoIterator<Item = SemanticAction>) -> Self {
        if self.disabled {
            self
        } else {
            self.actions(actions)
        }
    }
    /// Marks the value as readable but not changeable.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
    /// Marks the content as not acceptable.
    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
    /// Reports whether an owned surface or content is open.
    #[must_use]
    pub const fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }
    /// Marks the value as required by the application.
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
    /// Marks the value as secret and withdraws any reported value, so a
    /// protected component never exposes its content through inspection,
    /// whichever of this and [`Self::value`] is called first.
    #[must_use]
    pub fn protected(mut self, protected: bool) -> Self {
        self.protected = protected;
        if protected {
            self.value = None;
        }
        self
    }
    /// Lets the name, description and invalid state describe immediate children.
    #[must_use]
    pub const fn names_children(mut self) -> Self {
        self.names_children = true;
        self
    }
}

/// A component's explicit inspection contribution; empty by default.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ComponentDetails {
    /// Typed role, name, value, state and actions; absent for layout-only
    /// containers and for components that have not described themselves.
    pub semantics: Option<Semantics>,
    /// Layout/input intent for automatic development validation. Standard controls
    /// populate it from their real metadata; custom controls may describe exceptions.
    pub validation: crate::ValidationHints,
    /// Actual layout/style data when available; unmeasured fields stay absent.
    pub box_model: BoxModelInspection,
    /// Explicit component state, such as labels, selections or cached sizes.
    /// Arbitrary Rust signal contents are not automatically serialized.
    pub properties: BTreeMap<String, InspectValue>,
}

/// One retained component in the inspection tree/registry.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ComponentInspection {
    /// Runtime-local incarnation number; scoped to the enclosing runtime snapshot.
    pub id: u64,
    /// Stable caller key, distinct from runtime incarnation.
    pub key: String,
    /// The addressable key path of [workspace persistence](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md):
    /// application keys and container slots from the root down to the segment
    /// this component introduces, without library-internal structure. `None`
    /// for a structural component, which shares its nearest addressable
    /// ancestor's address, and for a drag preview. Schema 12 added this field.
    pub path: Option<Vec<String>>,
    /// Lifetime owner incarnation, retained when presentation detaches.
    pub parent: Option<u64>,
    /// Presentation parent incarnation; None at a native root.
    pub presentation_parent: Option<u64>,
    /// Native presentation root incarnation. Bounds and clips use its coordinates.
    pub window: u64,
    /// Accumulated [view scale](crate::ViewTransform) of this component's local
    /// space, 1 outside any view transform. Bounds and clips stay window-logical,
    /// the actual rendered frame. Schema 11 added this field.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f32"))]
    pub view_scale: f32,
    /// Whether presentation is enabled for this mount.
    pub visible: bool,
    /// Whether this mount owns keyboard/edit focus.
    pub focused: bool,
    /// Strongest outstanding invalidation, including hidden accumulated work.
    pub dirty: Option<Invalidation>,
    /// Number of collected read subscriptions (including repeated reads).
    pub subscriptions: usize,
    /// Next elapsed wake, the earlier of an update deadline and an event-phase
    /// timer; absent for sleeping/hidden components.
    pub next_deadline: Option<Duration>,
    /// The enclosing component whose name was given to this one because it had
    /// none of its own, such as the [`SemanticRole::Field`] wrapping a control.
    /// Usually the parent; for a part of a composite (internal structure such
    /// as a `Select`'s anchor, under a composite that reports no semantics
    /// itself) it is the naming ancestor above that composite. The name is
    /// already copied into `details.semantics`.
    pub labelled_by: Option<u64>,
    /// Explicitly supplied component/layout data.
    pub details: ComponentDetails,
}

/// One still-owned local parameter, including availability after removal.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ParameterInspection {
    /// Complete parameter incarnation.
    pub id: ParameterId,
    /// Family-tagged specification together with base, preview, effective
    /// value, revision and availability. Choices stay identities.
    pub parameter: crate::AnyParameterSnapshot,
    /// Host notification health; `Unobserved` without a registered sink.
    pub delivery: crate::DeliveryStatus,
}

/// Recorded framework operation category, independent of any external transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub enum EventKind {
    /// Mount admission.
    Mount,
    /// Component removal.
    Unmount,
    /// Visibility transition.
    Visibility,
    /// Focus transfer or loss.
    Focus,
    /// General event closure dispatched; its arbitrary return type is opaque.
    Dispatch,
    /// Explicit fallible application event.
    Application,
    /// Physical key delivery, without capturing text content. Keys delivered
    /// to a component that [conceals its input](crate::Component::conceals_input)
    /// are not recorded.
    KeyInput(crate::KeyEvent),
    /// Committed text delivery; text itself is not retained in history.
    TextInput {
        /// UTF-8 byte count; actual text content is not retained.
        bytes: usize,
    },
    /// Retained component evaluation.
    Update,
    /// An elapsed event-phase component timer.
    Timer,
    /// A retained subtree moved to another owner or position.
    Reparent,
    /// A drag session was adopted; `component` is its source.
    DragStart,
    /// A drag session ended; `component` is its source. `Succeeded` is a drop,
    /// `Cancelled` a cancellation, and `Failed` a refusal or a participant's
    /// error. The payload is never recorded.
    DragEnd,
    /// Parameter edit admission.
    EditBegin,
    /// Parameter preview update.
    EditPreview,
    /// A local commit, remote submission, final authority response or replay of
    /// a terminal receipt. A successful submission means its proposal was retained,
    /// not that its base changed or an external authority accepted it. Match the
    /// authority receipt for remote acceptance. Event history is parameter-scoped
    /// diagnostic data and carries no proposal token; retain submission receipts
    /// separately to correlate the edit protocol.
    EditCommit,
    /// Explicit or lifecycle-triggered cancellation.
    EditCancel,
    /// External authoritative parameter write.
    HostWrite,
    /// Effective scalar overlay change.
    Effective,
    /// Parameter metadata replacement.
    Metadata,
    /// Parameter removal or local host shutdown.
    ParameterRemoved,
}

/// Outcome of a recorded operation. Success means accepted by this local contract.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum EventStatus {
    /// The operation completed successfully.
    Succeeded,
    /// A terminal gesture was cancelled without committing.
    Cancelled,
    /// The framework cannot infer an arbitrary event closure's application result.
    Dispatched,
    /// Failed or rejected, with a diagnostic message truncated to
    /// [`Self::MAX_MESSAGE_CHARS`] Unicode characters.
    Failed(String),
}

impl EventStatus {
    /// The most Unicode characters of a [`Self::Failed`] message that event
    /// history retains; the rest is cut off when the event is recorded.
    pub const MAX_MESSAGE_CHARS: usize = 512;
}

/// One chronological diagnostic event in a bounded in-memory history.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct EventRecord {
    /// Monotonic runtime-local sequence, independent of elapsed timestamp ties.
    pub sequence: u64,
    /// Elapsed time of the latest flush. Events between flushes may share this time.
    pub at: Duration,
    /// Operation category.
    pub kind: EventKind,
    /// Component incarnation, when applicable.
    pub component: Option<u64>,
    /// Parameter incarnation, when applicable.
    pub parameter: Option<ParameterId>,
    /// Result known to the runtime or explicitly reported by an application event.
    pub status: EventStatus,
}

/// Versioned, owned snapshot of observable runtime and explicitly exposed app state.
///
/// Capture on the UI thread, then transfer the snapshot to an external inspector
/// or the application's tooling thread. It contains no Rc, callbacks, backend handles or mutation
/// capabilities. This is not a heap dump or persisted application document. A host
/// explicitly contributes additional app state through `Runtime::inspect_with`.
/// The library starts no server, network listener or transport; an application
/// that offers MCP or another protocol implements it on top of this snapshot.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct InspectionSnapshot {
    /// Native presentation roots, including known hidden roots, in creation order.
    pub windows: Vec<WindowInspection>,
    /// Snapshot schema revision, independent of crate versions. Snapshots
    /// captured by this runtime carry [`InspectionSnapshot::SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Live-data sources adopted by the runtime, in creation order, with their
    /// schedule, freshness and loss totals. Schema 5 added this field.
    pub sources: Vec<crate::SourceInspection>,
    /// Current event-history recording mode.
    pub mode: DebugMode,
    /// Current development overlay setting. Schema 10 added this field and
    /// removed the `BoxModel` recording mode it replaces.
    pub overlay: DebugOverlay,
    /// The active drag session, without its payload. Schema 11 added this field.
    pub drag: Option<crate::DragInspection>,
    /// Current elapsed runtime clock.
    pub at: Duration,
    /// Earliest pending work, absent when the runtime can sleep.
    pub next_deadline: Option<Duration>,
    /// All mounted components, including hidden ones, in incarnation order.
    pub components: Vec<ComponentInspection>,
    /// Still-owned registered parameters, in registration order.
    pub parameters: Vec<ParameterInspection>,
    /// All currently held physical keys.
    pub pressed_keys: Vec<KeyCode>,
    /// Explicit host-provided app state. Empty for ordinary `inspect`.
    pub application: BTreeMap<String, InspectValue>,
    /// Up to [`Self::MAX_EVENTS`] retained events in oldest-to-newest order.
    pub events: Vec<EventRecord>,
    /// Events evicted or rejected by history exhaustion since this history began.
    /// Saturates at `u64::MAX`; sequence numbers never wrap.
    pub dropped_events: u64,
    /// Advances when the history is cleared or its mode changes, so a reader
    /// that keeps a sequence cursor can tell a reset from nothing new. Schema
    /// 12 added this field.
    pub history_generation: u64,
    /// Development errors and marker data, independently of event-history mode.
    /// Release builds return a disabled empty report. Results describe the last
    /// validation pass; `pending` indicates changes waiting for a flush.
    pub validation: crate::ValidationReport,
}

impl InspectionSnapshot {
    /// Schema revision this runtime writes into
    /// [`schema_version`](Self::schema_version).
    ///
    /// It changes when a field is added, removed or reinterpreted, not with the
    /// crate version. An external inspector compares it before decoding a
    /// snapshot and treats an unknown revision as unsupported rather than
    /// reinterpreting it.
    pub const SCHEMA_VERSION: u32 = 14;
    /// The most events history retains. Recording one more evicts the oldest
    /// and counts it in [`Self::dropped_events`].
    pub const MAX_EVENTS: usize = 256;
}

#[derive(Default)]
pub(crate) struct History {
    pub(crate) mode: DebugMode,
    pub(crate) records: VecDeque<EventRecord>,
    pub(crate) dropped: u64,
    pub(crate) generation: u64,
    sequence: u64,
}

impl History {
    pub(crate) fn record(
        &mut self,
        at: Duration,
        kind: EventKind,
        component: Option<u64>,
        parameter: Option<&ParameterId>,
        status: impl FnOnce() -> EventStatus,
    ) {
        if self.mode == DebugMode::Off {
            return;
        }
        let Some(sequence) = self.sequence.checked_add(1) else {
            self.dropped = self.dropped.saturating_add(1);
            return;
        };
        self.sequence = sequence;
        if self.records.len() == InspectionSnapshot::MAX_EVENTS {
            self.records.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        let mut status = status();
        if let EventStatus::Failed(message) = &mut status {
            *message = message
                .chars()
                .take(EventStatus::MAX_MESSAGE_CHARS)
                .collect();
        }
        self.records.push_back(EventRecord {
            sequence,
            at,
            kind,
            component,
            parameter: parameter.cloned(),
            status,
        });
    }
}

/// Owned observations of one native presentation root, captured on the UI thread.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct WindowInspection {
    /// Runtime-local window incarnation, never a native OS handle.
    pub id: u64,
    /// Effective logical dimensions and scale, including application zoom.
    pub viewport: crate::Viewport,
    /// Runtime presentation eligibility, not a claim about unknown OS minimization.
    pub presentable: bool,
    /// Retained focus incarnation in this root, if any.
    pub focused: Option<u64>,
    /// Active content command target, independent of component focus.
    pub command_target: Option<u64>,
    /// Earliest eligible work for this root.
    pub next_deadline: Option<Duration>,
}
