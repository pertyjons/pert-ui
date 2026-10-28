#![doc = include_str!("../README.md")]

/// Implements `Debug` for a description, showing its type name and, for a
/// keyed item description, its key.
///
/// A description's content may be bound to signals, which formatting must not
/// read inside a runtime phase, and its callbacks cannot be formatted, so
/// neither is shown. Nothing bounds the description's type parameter.
macro_rules! description_debug {
    ($name:ident $(<$param:ident>)?) => {
        /// Shows only the type. A description's content, bindings and
        /// callbacks are not formatted; where the description is mounted as an
        /// [`Element`](crate::Element), that element's `Debug` shows its key.
        impl$(<$param>)? ::std::fmt::Debug for $name$(<$param>)? {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
    };
    ($name:ident, key = $key:expr) => {
        /// Shows the item's key. Its content, bindings and callbacks are not
        /// formatted.
        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let key: fn(&Self) -> &str = $key;
                f.debug_struct(stringify!($name))
                    .field("key", &key(self))
                    .finish_non_exhaustive()
            }
        }
    };
}

mod error;
pub use error::ApplicationError;
mod resize_grip;
mod windows;
pub use windows::{
    FloatingWindow, WindowCloseAction, WindowController, WindowLayer, WindowPlacement, WindowStatus,
};
mod notifications;
pub use notifications::{Banner, Toast, ToastController, ToastHost, ToastReceipt, ToastStatus};
mod dialog;
pub use dialog::{AlertDialog, Dialog, DialogController, DialogOutcome, DialogStatus};
mod tabs;
pub use tabs::{Tab, Tabs};
mod conformance;
pub use conformance::{Redescribed, RedescriptionError, check_redescription, redescribe};
mod split;
pub use split::{SplitAxis, SplitPane, SplitRule, SplitSizing};
mod workspace;
pub use workspace::{
    IgnoredEntry, RestoreRejection, ViewState, Workspace, WorkspaceError, WorkspacePath,
    WorkspaceReport, WorkspaceSettings, WorkspaceValue,
};
mod region_layout;
pub use region_layout::{RegionEdge, RegionLayout};
mod list_drag;
pub use list_drag::{ListDragItems, ListDrop, ListDropPosition};
mod history_navigator;
mod list_selection;
mod navigation_history;
mod table;
mod tree;
pub use table::{
    CellEdit, CellEditEnd, CellEditEndReason, SortDirection, Table, TableColumn, TableController,
    TableItems, TableSort, TableSource,
};
mod described;
mod editor_session;
pub use editor_session::{
    EditorAttachment, EditorCompletion, EditorOutcome, EditorSession, EditorStart, EditorTrigger,
};
mod render_limits;
pub use render_limits::{LimitError, ResourceKind};
mod views;
mod virtual_axis;
mod virtual_list;
pub use described::{DescribeContext, Dynamic, Each, EachItem, Show};
pub use history_navigator::{HistoryButtons, HistoryNavigator};
pub use list_selection::{ListSelection, ListSelectionChange, SelectionMode};
pub use navigation_history::{HistoryEntry, HistoryEntryId, HistorySnapshot, NavigationHistory};
pub use tree::{Tree, TreeController, TreeItems, TreeSource};
pub use views::{Divider, Group, ScrollArea, ScrollAxes, View};
pub use virtual_axis::VirtualAxis;
pub use virtual_list::{
    ListController, ListDropMark, ListDropPlacement, ListDropPoint, ListItems, ListRevealReceipt,
    ListRevealStatus, ListSource, VirtualList,
};
mod axis_mapping;
pub use axis_mapping::AxisMapping;
mod zoom_view;
pub use zoom_view::{ScaleLimits, WheelGesture, ZoomController, ZoomView};
mod activation;
mod choice_controls;
mod number_format;
pub use number_format::{NumberFormat, NumberLocale, NumberLocaleError};
mod strings;
pub use strings::{StringRole, UiMessage, UiStringError, UiStrings};
mod numeric_controls;
pub use numeric_controls::{ArcOrigin, InlineSlider, Knob, Orientation, Slider};
mod breadcrumb;
mod chip;
mod collapsible;
mod collection_filter;
pub use collection_filter::{
    CollectionFilter, FilterCounts, FilterMatcher, FilterOutcome, FilterTarget, HiddenActive,
    ListFilter,
};
mod command_palette;
mod content_search;
pub use content_search::{
    ContentField, ContentHit, ContentJump, ContentQuery, ContentRows, ContentSearch,
    ContentSearchError, ContentStep, ContentTarget, PendingJump,
};
mod containers;
mod control_binding;
mod controls;
mod field_status;
pub use field_status::{Field, FieldHandle, FieldStatus, FormFields, FormSummary};
mod inline_controls;
mod live_widgets;
mod popover;
pub use live_widgets::{LevelMeter, MeterAxis, Trace};
mod progress;
pub use progress::{ProgressBar, ProgressStatus, Spinner};
mod menu;
mod menu_bar;
mod multi_select;
mod select;
mod toolbar;
pub use breadcrumb::{Breadcrumb, BreadcrumbSegment};
pub use chip::Chip;
pub use choice_controls::{
    Checkbox, Choice, ChoiceError, InlineChoice, RadioGroup, SegmentedControl, Toggle,
};
pub use collapsible::{Accordion, Collapsible, ExpandMode, ExpansionChange, Section};
pub use command_palette::{
    CommandMatcher, CommandPalette, CommandPaletteController, CommandPaletteStatus,
};
mod quick_pick;
pub use controls::{Button, Label};
pub use quick_pick::{
    PickChoice, PickItem, PickMatcher, PickPending, PickStatus, QuickPick, QuickPickController,
};
mod link;
pub use inline_controls::{InlineReadout, InlineToggle};
pub use link::{Link, LinkUnderline};
pub use menu::{ContextMenu, ContextMenuController, Menu, MenuItem};
pub use menu_bar::MenuBar;
pub use multi_select::{MultiSelect, MultiSelectCommit};
pub use popover::{Popover, SurfaceDismissal, SurfaceKind, Tooltip};
pub use select::{InlineSelect, Select};
pub use toolbar::{ToolGroup, Toolbar, ToolbarOverflow};
mod selectable_text;
mod tables;
mod text;
pub use selectable_text::SelectableText;
mod text_fields;
mod text_input;
mod text_selection;
pub use runtime::fonts::{FontManager, IconManager};
pub use text::{
    FittedText, FontData, FontError, FontId, FontRole, HighlightError, Icon, IconGlyph, IconSource,
    RichText, TextColor, TextError, TextLayout, TextLimit, TextOverflow, TextSpan, TextStyle,
};
pub use text_fields::{
    DecimalInput, DraftAlign, DraftEditor, DraftEditorController, DraftSelection, InlineDecimal,
    InlineInteger, InlineText, InputRejection, IntegerInput, NumberInputError, SearchField,
    SearchKey, TextArea, TextField,
};
pub use text_input::{
    Clipboard, ClipboardError, TextCommand, TextCommandOutcome, TextEditError, TextInputState,
    TextPreedit,
};
pub use text_selection::{SelectionExtension, TextMovement, TextSelection};
mod form;
mod layout;
pub use containers::{Align, Column, Container, Grid, Justify, Length, Row, Track, Wrap};
pub use form::Form;
mod property_grid;
mod tool_windows;
pub use layout::{
    ArrangeContext, Constraints, Element, GeometryError, IntoElement, LayoutError, MeasureContext,
    Size, SizeLimits,
};
pub use property_grid::{Property, PropertyGrid};
pub use tool_windows::{
    EdgeMode, Expansion, StripeGroup, ToolEdge, ToolWindow, ToolWindowChange, ToolWindowChangeKind,
    ToolWindowController, ToolWindowLayout, ToolWindowMode,
};
mod geometry;
mod transform;
pub use transform::{QuarterTurn, Transform2D, ViewScale, ViewTransform};
mod scrollbar;
mod shapes;
mod vector_scene;
pub use geometry::{
    FillRule, LineCap, LineJoin, Path, PathBuilder, PathElement, PathError, StrokeStyle,
};
pub use shapes::Shape;
pub use vector_scene::{VectorGeometry, VectorId, VectorRasterizer, VectorStyle};
mod color_picker;
pub use color_picker::ColorPicker;
mod async_source;
pub use async_source::{RequestTicket, RequestTracker, TicketStatus};
mod file_browser;
pub use file_browser::{
    DirectoryEntry, DirectoryRequest, DirectoryResponse, EntryKind, FileBrowser,
    FileBrowserController, FileBrowserError, LinkTarget, ListingDelivery, ListingError,
    ListingErrorKind, list_directory,
};
mod color_swatch;
pub use color_swatch::ColorSwatch;
mod drawing;
pub use drawing::{
    Color, DisplayList, DrawCommand, DrawError, HexColorError, MaskId, MaskImage, OpaqueColor,
    Painter, PhysicalExtent, Rect, TranslucentColorError, Viewport,
};
mod image;
pub use image::{Image, ImageFit};
mod resources;
pub use resources::{ImageId, ImageSampling, RasterImage};
mod capture;
pub use capture::{
    Capture, CaptureError, CaptureJob, CaptureRequest, CaptureTarget, FrameError, FrameImage,
};
mod diagnostics;
mod inspection_service;
pub use inspection_service::{
    Depth, Detail, DiagnosticCommand, EventHistory, InspectionClient, InspectionQuery,
    InspectionReply, InspectionService, Pending, QueryResult, RequestError, Selector, WaitError,
};
mod profiling;
#[cfg(feature = "serde")]
mod serde_float;
pub use profiling::{ProfileOperation, ProfileSample, ProfileSnapshot, ProfileStats};
mod drag;
mod input;
mod live_source;
mod mailbox;
pub use drag::{
    DRAG_THRESHOLD, DragEvent, DragInspection, DragOutcome, DragPayload, DragPhase, DragPreview,
    DragRole, DropOperation, DropOperations, DropResponse,
};
mod click;
pub use click::ClickCounter;
mod pointer;
pub use pointer::{
    CursorIcon, InputOutcome, Magnification, Point, PointerButton, PointerEvent, PointerKind,
};
mod parameter_binding;
pub use parameter_binding::remote::{
    AuthorityReceipt, AuthorityResponse, AuthorityStatus, RemoteCommit, RemoteHost,
    ResponseDisposition, SubmissionDelivery,
};
pub mod host;
mod runtime;
pub mod testing;
pub use runtime::application::{BackgroundTask, IntervalTimer};
pub use runtime::commands::{
    CommandContext, CommandEntry, CommandId, CommandInfo, CommandKey, CommandKeyInput,
    CommandOutcome, CommandReceipt, CommandScope, CommandSource, CommandStatus, KeyChord,
    KeyDispatch, TextKeyOutcome,
};
pub use runtime::native_windows::{
    DetachedClosePolicy, NativeWindowId, NativeWindowOptions, PreparedDetach, PresentationAction,
    PresentationReceipt, PresentationRequest, PresentationStatus, WindowError,
};
pub use runtime::notify::{ChangeOrigin, Notifier, SelectionChange};
pub use runtime::presentation::PresentationRate;
pub use runtime::reveal::{ChildReveal, RevealOutcome, RevealReceipt, RevealRefusal, RevealStatus};
pub use runtime::tasks::{ShownTaskReceipt, TaskReceipt, TaskStatus};
mod geometry_report;
mod permission;
pub use permission::Permission;
mod text_source;
mod window_titles;
pub(crate) use text_source::Caption;
pub use text_source::TextSource;
mod signal;
mod validation;

pub use validation::{
    OverlapPolicy, ShortcutBinding, ShortcutScope, ValidationCode, ValidationHints,
    ValidationIssue, ValidationMarker, ValidationReport,
};

pub use parameter_binding::{
    AnyDeliveryMonitor, AnyParameter, AnyParameterHost, AnyPreviewEvent, AnyTerminalEvent,
    BindingError, DeliveryError, DeliveryMonitor, DeliveryResolution, DeliverySnapshot,
    DeliveryStatus, EditSession, Parameter, ParameterHost, PreviewEvent, SinkError, SinkErrorKind,
    TerminalEvent,
};

pub use diagnostics::{
    BorderInspection, BoxModelInspection, ComponentDetails, ComponentInspection, DebugMode,
    DebugOverlay, EventKind, EventRecord, EventStatus, InspectValue, InspectionSnapshot,
    ParameterInspection, SemanticAction, SemanticItem, SemanticRole, SemanticValue, Semantics,
    WindowInspection,
};
pub use input::{
    FocusDirection, KeyAction, KeyCode, KeyEvent, KeyTransition, KeyboardState, Modifiers,
};
pub use live_source::{
    Delivery, Freshness, LiveWaker, SeriesSource, SourceInspection, Telemetry, WakeError,
};
pub use mailbox::{Mailbox, SendError, UiSender};
pub use runtime::settings::{MotionPreference, TextRenderingMode, UiSettings, UiZoom};
pub use runtime::theme::{
    ColorRole, Density, MetricRole, Palette, Spacing, ThemeManager, ThemeMetrics, ThemeMode,
};
pub use runtime::{
    ChildInfo, Component, EventContext, FailurePhase, FailureSite, FlushReport, Invalidation,
    Runtime, RuntimeError, UpdateContext, WidgetId,
};
pub use signal::{ReadSignal, Signal, StateOwner};

#[doc(inline)]
pub use pert_ui_core::{
    AnyParameterSnapshot, AnySpec, AnyValue, Capacity, ChoiceId, ChoiceOption, DrainReport,
    EditError, EditOutcome, EditSequence, EditSource, EditToken, HostValue, Incarnation, LiveError,
    LiveItem, NormalizedValue, NormalizedValueError, ParameterError, ParameterFamily, ParameterId,
    ParameterIdError, ParameterMapping, ParameterSnapshot, ParameterSpec, ParameterState,
    ParameterValue, PowerExponent, PreviewOutcome, PublishOutcome, Revision, Sample, SampleWindow,
    SampleWindowEntry, Sequence, SeriesConfig, SeriesConsumer, SeriesEndpoint, Timestamp, WakeFlag,
    series_bridge,
};

/// The names an application's first program writes, for
/// `use pert_ui_runtime::prelude::*`.
///
/// # Membership
///
/// The prelude is the glob for application code. It holds:
///
/// - every name the application part of the guide's representative programs
///   writes: a first window, a settings form, an editable list of records,
///   dialogs, a menu bar with a command, a form with a list from data and a
///   shortcut, and background work started from a button;
/// - every built-in component that belongs to no module below, is not a
///   building block for custom components and can be constructed without a
///   thematic family's names;
/// - the names an application writes everywhere: [`Runtime`], [`Signal`],
///   [`ReadSignal`], [`StateOwner`], [`Element`], [`IntoElement`],
///   [`WidgetId`], [`EventContext`], [`Permission`], [`RuntimeError`] and the
///   [`ApplicationError`] an application constructs;
/// - what those members need to be configured, applied again to every name it
///   adds: each type their constructors and builders take, each callback value
///   an application matches, each returned value it keeps, and each enum
///   payload or public field it constructs;
/// - [`ThemeMode`], which a theme switch writes; [`ThemeManager`] and
///   [`Palette`] stay at the crate root.
///
/// A name is written when a program constructs it, matches it or keeps it; a
/// value only passed on, such as a closure's context, is not.
///
/// # Guarantee
///
/// The application part of each representative program compiles with this
/// glob alone, its tests add [`testing`], and a custom component adds
/// [`component`]. An application configures every component the prelude
/// holds through the prelude alone, except along the paths below.
///
/// # Paths that need another import
///
/// - The `for_parameter` constructors take a [`Parameter`], and a choice
///   control's options are [`ChoiceId`]s: [`parameter`].
/// - `Button::command`, `MenuItem::command` and `Tooltip::command` take a
///   [`CommandId`], and a component that runs a command keeps a
///   [`CommandReceipt`]: [`commands`]. `CommandScope::Window` names a
///   [`NativeWindowId`], which [`commands`] and [`windowing`] hold.
/// - The editors' `session` builders take an [`EditorSession`]: [`component`].
/// - Arguments taken as `impl Into<TextSource>` or `impl Into<IconSource>` are
///   written as strings, signals and [`Icon`]s; [`TextSource`] is at the crate
///   root and [`IconSource`] in [`component`].
/// - The [`DescribeContext`] an [`Each`], [`Dynamic`] or [`Show`] closure
///   receives, and the [`EachItem`] an [`Each`]'s `build` returns, are
///   inferred; code that names either imports it from the crate root.
/// - [`TextEditError`], which `on_binding_refusal` reports, and the other
///   error types stay at the crate root, where code names them to match a
///   failure; `?` converts the leaf errors [`RuntimeError`] wraps into it.
/// - Services: [`NumberFormat`]'s localized constructors take a
///   [`NumberLocale`], and [`TextSpan::font`] and [`TextSpan::font_role`] take
///   a [`FontId`] and a [`FontRole`]; they are at the crate root.
/// - [`ProgressStatus::of`] builds progress from counts; a fraction is a
///   [`NormalizedValue`] from [`parameter`].
/// - [`Runtime`] and [`EventContext`] have methods for every family; a
///   method's arguments are in that family's module.
///
/// # Where the other names are
///
/// | Names | Import |
/// |---|---|
/// | Implementing [`Component`]: contexts, drawing, input events, drag and drop, receipts, editor sessions and the embedded draft editor | `use pert_ui_runtime::component::*` ([`component`]) |
/// | A headless test: the driver, the viewport, key and pointer input, semantics, query results and validation, queries and captures | `use pert_ui_runtime::testing::*` ([`testing`]) |
/// | A custom host that drives the runtime in place of the facade's `App`: [`HostIntegration`](host::HostIntegration) and the names a host writes | `use pert_ui_runtime::host::*` ([`host`]) |
/// | Authoritative values: parameters, edit gestures, hosts, external authority, and [`Slider`], [`Knob`] and [`InlineSlider`] | `use pert_ui_runtime::parameter::*` ([`parameter`]) |
/// | Commands beyond a shortcut's chord and scope, held input and the command palette | `use pert_ui_runtime::commands::*` ([`commands`]) |
/// | Windows inside a view, native windows, regions, splits and tool windows | `use pert_ui_runtime::windowing::*` ([`windowing`]) |
/// | Inspection, capture, profiling and validation | `use pert_ui_runtime::inspection::*` ([`inspection`]) |
/// | Data from other threads: live series, mailboxes, [`LevelMeter`] and [`Trace`] | `use pert_ui_runtime::live::*` ([`live`]) |
/// | Theme, UI settings, fonts and icons, strings and locale, the workspace document, other errors, [`DescribeContext`], [`EachItem`], [`PhysicalExtent`] and [`VirtualAxis`] | by name from the crate root, such as `use pert_ui_runtime::{Palette, Workspace};` |
///
/// A name can be in the prelude and in a module, or in several modules: each
/// re-exports the same crate-root item, so the globs combine without
/// conflict. Glob the prelude and the modules rather than the crate root: a
/// crate-root glob also brings in the module names, which can collide with a
/// dependency crate of the same name.
///
/// The `pert-ui` facade's prelude re-exports this one and adds its native
/// startup types (`App` and everything it reports, errors included) and the
/// widgets its own features provide (`Svg` with `svg`). The re-exports are
/// `#[doc(no_inline)]`: each item is documented once, at the crate root.
pub mod prelude {
    // Application-wide names.
    #[doc(no_inline)]
    pub use crate::{
        ApplicationError, Element, EventContext, IntoElement, Permission, ReadSignal, Runtime,
        RuntimeError, Signal, StateOwner, WidgetId,
    };
    // Built-in components outside the thematic modules.
    #[doc(no_inline)]
    pub use crate::{
        Accordion, AlertDialog, Banner, Breadcrumb, Button, Checkbox, Chip, Collapsible,
        CollectionFilter, ColorPicker, ColorSwatch, Column, Container, ContextMenu, DecimalInput,
        Dialog, Divider, Dynamic, Each, Field, FileBrowser, Form, Grid, Group, HistoryButtons,
        HistoryNavigator, Image, InlineChoice, InlineDecimal, InlineInteger, InlineReadout,
        InlineSelect, InlineText, InlineToggle, IntegerInput, Label, Link, Menu, MenuBar,
        MultiSelect, Popover, ProgressBar, PropertyGrid, QuickPick, RadioGroup, Row, ScrollArea,
        SearchField, SegmentedControl, Select, SelectableText, Show, Spinner, Table, Tabs,
        TextArea, TextField, ToastHost, Toggle, Toolbar, Tooltip, Tree, View, VirtualList, Wrap,
        ZoomView,
    };
    // Vocabulary: layout containers.
    #[doc(no_inline)]
    pub use crate::{
        Align, ChangeOrigin, Color, ColorRole, Constraints, CursorIcon, Justify, Length,
        MetricRole, ScrollAxes, Size, SizeLimits, Spacing, Track,
    };
    // Vocabulary: text, labels and editors.
    #[doc(no_inline)]
    pub use crate::{
        Icon, InputRejection, LinkUnderline, NumberFormat, NumberInputError, QuarterTurn, RichText,
        SearchKey, TextColor, TextOverflow, TextSpan, TextStyle,
    };
    // Vocabulary: choice and value controls.
    #[doc(no_inline)]
    pub use crate::{Choice, MultiSelectCommit, Orientation};
    // Vocabulary: disclosure and tabs.
    #[doc(no_inline)]
    pub use crate::{ExpandMode, ExpansionChange, Section, SelectionChange, Tab};
    // Vocabulary: forms.
    #[doc(no_inline)]
    pub use crate::{FieldHandle, FieldStatus, FormFields, FormSummary, Property};
    // Vocabulary: surfaces.
    #[doc(no_inline)]
    pub use crate::{
        ContextMenuController, DialogController, DialogOutcome, DialogStatus, MenuItem, PickChoice,
        PickItem, PickMatcher, PickPending, PickStatus, Point, QuickPickController,
        SurfaceDismissal, Toast, ToastController, ToastReceipt, ToastStatus,
    };
    // Vocabulary: collections, with what an edit-end callback matches.
    #[doc(no_inline)]
    pub use crate::{
        CellEdit, CellEditEnd, CellEditEndReason, DropOperation, DropOperations, EditorCompletion,
        EditorOutcome, EditorTrigger, FilterCounts, FilterMatcher, FilterOutcome, FilterTarget,
        FocusDirection, HiddenActive, ListController, ListDrop, ListDropMark, ListDropPlacement,
        ListDropPoint, ListDropPosition, ListFilter, ListItems, ListRevealReceipt,
        ListRevealStatus, ListSelection, ListSelectionChange, ListSource, SelectionMode,
        SortDirection, TableColumn, TableController, TableItems, TableSort, TableSource,
        TreeController, TreeItems, TreeSource,
    };
    // Vocabulary: files.
    #[doc(no_inline)]
    pub use crate::{
        DirectoryEntry, DirectoryRequest, DirectoryResponse, EntryKind, FileBrowserController,
        LinkTarget, ListingDelivery, ListingError, ListingErrorKind,
    };
    // Vocabulary: navigation.
    #[doc(no_inline)]
    pub use crate::{
        BreadcrumbSegment, HistoryEntry, HistoryEntryId, HistorySnapshot, NavigationHistory,
        ToolGroup, ToolbarOverflow,
    };
    // Vocabulary: media and progress.
    #[doc(no_inline)]
    pub use crate::{ImageFit, ImageSampling, ProgressStatus, RasterImage};
    // Vocabulary: zoom.
    #[doc(no_inline)]
    pub use crate::{ScaleLimits, ViewScale, ViewTransform, WheelGesture, ZoomController};
    // Commands and keys a shortcut writes, and the text command a rename begins.
    #[doc(no_inline)]
    pub use crate::{CommandKey, CommandScope, KeyChord, KeyCode, Modifiers, TextCommand};
    // The theme mode a theme switch writes; the theme services stay at the root.
    #[doc(no_inline)]
    pub use crate::ThemeMode;
}

/// The names implementing [`Component`] writes, for
/// `use pert_ui_runtime::component::*`.
///
/// Applications glob [`prelude`]; code that implements [`Component`] also
/// globs this module. Two globs that re-export one item do not conflict, so
/// the names both hold, such as [`Component`] and [`Painter`], can be imported
/// through either.
///
/// # Membership
///
/// The module holds [`Component`], its contexts, every name an
/// implementation has to write, and the built-in editor a component embeds
/// ([`DraftEditor`] with its controller, alignment and selection). A name is
/// a member when using the trait, a context or a value they hand out
/// requires writing it:
///
/// - an argument the component constructs, such as a [`DragPreview`] or the
///   [`NativeWindowOptions`] of [`EventContext::detach_view`];
/// - an enum it matches, including the status of a receipt it keeps and the
///   outcome that status carries ([`TaskStatus`], [`RevealStatus`] with
///   [`RevealOutcome`], [`PresentationStatus`], [`CommandStatus`] with
///   [`CommandOutcome`]);
/// - a field or payload it builds or destructures for a [`Component`]
///   method, such as the [`Semantics`], [`ValidationHints`],
///   [`BoxModelInspection`] and [`InspectValue`] parts of the
///   [`ComponentDetails`] that [`Component::inspect`] returns;
/// - a value it keeps past the call that returned it, where the rustdoc says
///   to keep it: the receipts ([`TaskReceipt`], [`ShownTaskReceipt`],
///   [`RevealReceipt`], [`PresentationReceipt`], [`CommandReceipt`]) and the
///   [`EditorAttachment`] through which a hosted editor reports the end of
///   its edit, and the [`ClickCounter`] a component that counts presses
///   stores.
///
/// A value only passed on, compared or converted in the call that returned
/// it, such as [`FittedText`], is not required. The excluded families are in
/// their own modules: profiling, capture and inspection queries and records
/// in [`inspection`], live-data plumbing and mailboxes in [`live`]; host
/// transport in [`host`]. The
/// description a component returns from [`Component::inspect`] is not an
/// inspection record and is here. The re-exports are `#[doc(no_inline)]`:
/// each item is documented once, at the crate root.
///
/// # Example
///
/// A component that paints, takes a click and describes itself for
/// inspection; the application around it globs the prelude:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{component::*, prelude::*};
/// use std::time::Duration;
///
/// struct Lamp {
///     on: bool,
/// }
///
/// impl Component for Lamp {
///     fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> {
///         Ok(())
///     }
///     fn focusable(&self) -> bool {
///         true
///     }
///     fn measure(
///         &self,
///         _cx: &MeasureContext<'_>,
///         _available: Constraints,
///     ) -> Result<Size, RuntimeError> {
///         Ok(Size::new(24.0, 24.0)?)
///     }
///     fn pointer_input(
///         &mut self,
///         cx: &mut EventContext,
///         event: PointerEvent,
///     ) -> Result<InputOutcome, RuntimeError> {
///         if let PointerKind::Press(PointerButton::Primary) = event.kind() {
///             self.on = !self.on;
///             cx.invalidate(Invalidation::Paint);
///             return Ok(InputOutcome::Handled);
///         }
///         Ok(InputOutcome::Ignored)
///     }
///     fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
///         let color = if self.on { Color::ACCENT } else { Color::SURFACE };
///         painter.fill_rect(Rect::new(0.0, 0.0, 24.0, 24.0)?, color);
///         Ok(())
///     }
///     fn inspect(&self) -> ComponentDetails {
///         ComponentDetails {
///             semantics: Some(
///                 Semantics::new(SemanticRole::Switch).value(SemanticValue::Bool(self.on)),
///             ),
///             properties: [("on".to_owned(), InspectValue::Bool(self.on))].into(),
///             ..ComponentDetails::default()
///         }
///     }
/// }
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(100.0, 50.0, 1.0)?)?;
/// ui.mount_tree(Element::new("lamp", Lamp { on: false }))?;
/// ui.flush(Duration::ZERO)?;
/// let press = PointerEvent::new(
///     Point::new(5.0, 5.0)?,
///     PointerKind::Press(PointerButton::Primary),
/// );
/// assert_eq!(ui.pointer_input(press)?, InputOutcome::Handled);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub mod component {
    #[doc(no_inline)]
    pub use crate::{
        ArrangeContext, ChildInfo, ChildReveal, Component, ComponentDetails, ContentRows,
        EventContext, Invalidation, MeasureContext, Notifier, UpdateContext, WidgetId,
    };
    #[doc(no_inline)]
    pub use crate::{
        BorderInspection, BoxModelInspection, InspectValue, OverlapPolicy, SemanticAction,
        SemanticItem, SemanticRole, SemanticValue, Semantics, ShortcutBinding, ShortcutScope,
        ValidationHints,
    };
    #[doc(no_inline)]
    pub use crate::{ClickCounter, DraftAlign, DraftEditor, DraftEditorController, DraftSelection};
    #[doc(no_inline)]
    pub use crate::{
        Color, ColorRole, DisplayList, DrawCommand, FillRule, Icon, IconSource, ImageId,
        ImageSampling, Length, LineCap, LineJoin, MaskId, MaskImage, MetricRole, Painter, Path,
        PathBuilder, PathElement, QuarterTurn, RasterImage, RichText, Shape, StrokeStyle,
        TextLayout, TextOverflow, TextStyle, VectorGeometry, VectorId,
    };
    #[doc(no_inline)]
    pub use crate::{
        CommandContext, CommandId, CommandOutcome, CommandReceipt, CommandSource, CommandStatus,
    };
    #[doc(no_inline)]
    pub use crate::{
        CommandKey, CommandKeyInput, FocusDirection, InputOutcome, KeyAction, KeyCode, KeyEvent,
        KeyTransition, KeyboardState, Magnification, Modifiers, PointerButton, PointerEvent,
        PointerKind, TextCommand, TextCommandOutcome, TextInputState, TextKeyOutcome, TextPreedit,
    };
    #[doc(no_inline)]
    pub use crate::{
        Constraints, CursorIcon, Point, Rect, Size, SizeLimits, SurfaceDismissal, SurfaceKind,
        Transform2D, ViewScale, ViewTransform, Viewport,
    };
    #[doc(no_inline)]
    pub use crate::{
        DRAG_THRESHOLD, DragEvent, DragOutcome, DragPayload, DragPhase, DragPreview, DragRole,
        DropOperation, DropOperations, DropResponse,
    };
    #[doc(no_inline)]
    pub use crate::{
        DetachedClosePolicy, NativeWindowId, NativeWindowOptions, PresentationReceipt,
        PresentationStatus, RevealOutcome, RevealReceipt, RevealRefusal, RevealStatus,
        ShownTaskReceipt, TaskReceipt, TaskStatus,
    };
    #[doc(no_inline)]
    pub use crate::{DrawError, RuntimeError};
    #[doc(no_inline)]
    pub use crate::{EditorAttachment, EditorOutcome, EditorSession, EditorStart, EditorTrigger};
    #[doc(no_inline)]
    pub use crate::{Element, MotionPreference, ReadSignal, Signal};
    #[doc(no_inline)]
    pub use crate::{RestoreRejection, ViewState, WorkspaceError, WorkspaceValue};
}

/// Authoritative values, for `use pert_ui_runtime::parameter::*`: parameters,
/// edit gestures, hosts, external authority and the parameter-only controls.
///
/// An application whose values a host validates, revisions and observes,
/// rather than UI-local [`Signal`]s, globs this module beside the
/// [`prelude`]. The [`Slider`], [`Knob`] and [`InlineSlider`] controls are
/// here because their only constructors take a [`Parameter`].
///
/// # Membership
///
/// The module holds the parameter specifications, identities and values, the
/// bindings, gestures and their outcomes, local ([`ParameterHost`]) and
/// external ([`RemoteHost`]) hosts with their delivery and authority records,
/// the parameter-only controls, and the family's errors, which a host
/// matches. It is closed one level: each argument of a member's constructors
/// and builders, and each enum payload or public field type of a member, is
/// here or in the prelude. [`ParameterInspection`] is an inspection record
/// and is in [`inspection`]; a control's [`Choice`] is in the prelude.
///
/// A name can also be in the prelude or another module, such as
/// [`ParameterSpec`], which [`live`] holds for a meter's range; each glob
/// re-exports the same crate-root item, so the globs combine. The
/// re-exports are `#[doc(no_inline)]`: each item is documented once, at the
/// crate root.
///
/// # Example
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{parameter::*, prelude::*};
/// use std::time::Duration;
///
/// let amount = ParameterSpec::new("Amount", -100.0, 100.0, 0.0)?;
/// let position = NormalizedValue::try_from(0.75)?;
/// assert_eq!(amount.denormalize(position), 50.0);
/// assert_eq!(amount.normalize(&f32::NAN), Err(ParameterError::NonFinite));
///
/// let mut ui = Runtime::new();
/// // The host owner must stay alive: dropping it removes the parameter.
/// let (value, host) = ui.parameter(ParameterId::new(1, 42, "amount")?, amount)?;
/// ui.mount_tree(Element::new("amount", Slider::new(value.clone())))?;
/// ui.flush(Duration::ZERO)?;
/// host.write_base(25.0)?;
/// assert_eq!(value.snapshot().get()?.base, 25.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub mod parameter {
    #[doc(no_inline)]
    pub use crate::{
        AnyDeliveryMonitor, AnyParameter, AnyParameterHost, AnyParameterSnapshot, AnyPreviewEvent,
        AnySpec, AnyTerminalEvent, AnyValue, ArcOrigin, AuthorityReceipt, AuthorityResponse,
        AuthorityStatus, BindingError, ChoiceId, ChoiceOption, DeliveryError, DeliveryMonitor,
        DeliveryResolution, DeliverySnapshot, DeliveryStatus, EditError, EditOutcome, EditSequence,
        EditSession, EditSource, EditToken, HostValue, InlineSlider, Knob, NormalizedValue,
        NormalizedValueError, Parameter, ParameterError, ParameterFamily, ParameterHost,
        ParameterId, ParameterIdError, ParameterMapping, ParameterSnapshot, ParameterSpec,
        ParameterState, ParameterValue, PowerExponent, PreviewEvent, PreviewOutcome, RemoteCommit,
        RemoteHost, ResponseDisposition, Revision, SinkError, SinkErrorKind, Slider,
        SubmissionDelivery, TerminalEvent,
    };
}

/// Commands, shortcuts and the command palette, for
/// `use pert_ui_runtime::commands::*`.
///
/// The prelude already holds what registering a shortcut writes: a
/// [`CommandScope`] and a [`KeyChord`] with its [`CommandKey`], [`KeyCode`]
/// and [`Modifiers`]. An application that keeps a command's [`CommandId`],
/// invokes a command, reads its [`CommandInfo`] or [`CommandOutcome`], lists
/// commands or shows a [`CommandPalette`] globs this module beside the
/// [`prelude`].
///
/// # Membership
///
/// The module holds the command registry's identities, contexts, sources,
/// outcomes and receipts, held-input cancellation, the shortcut chords and
/// the command palette with its controller. It is closed one level: each
/// argument of a member's constructors and builders, and each enum payload or
/// public field type of a member, is here or in the prelude, so it adds the
/// [`NativeWindowId`] of `CommandScope::Window` and
/// [`CommandContext::window`]. Raw key events and a host's key entry
/// ([`KeyEvent`], [`KeyAction`], [`KeyTransition`], [`KeyboardState`],
/// [`KeyDispatch`], [`CommandKeyInput`], [`TextKeyOutcome`]) are in
/// [`component`], [`testing`] or at the crate root, and shortcut validation
/// records in [`inspection`].
///
/// A name can also be in the prelude or another module; each glob re-exports
/// the same crate-root item, so the globs combine. The re-exports are
/// `#[doc(no_inline)]`: each item is documented once, at the crate root.
///
/// # Example
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{commands::*, prelude::*};
///
/// let mut ui = Runtime::new();
/// let deleted = ui.signal(0);
/// let count = deleted.clone();
/// let delete: CommandId = ui.register_command(
///     "Delete",
///     CommandScope::Application,
///     vec![KeyChord::named(KeyCode::Delete)],
///     move |_, _| count.update(|n| *n += 1),
/// )?;
/// let context = ui.command_context(CommandSource::Programmatic);
/// let info: CommandInfo = ui.command_info(&delete, &context)?;
/// assert!(info.enabled);
/// let outcome = ui.invoke_command(&delete, context)?;
/// assert!(matches!(outcome, CommandOutcome::Invoked));
/// assert_eq!(deleted.get()?, 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub mod commands {
    #[doc(no_inline)]
    pub use crate::{
        CommandContext, CommandEntry, CommandId, CommandInfo, CommandKey, CommandMatcher,
        CommandOutcome, CommandPalette, CommandPaletteController, CommandPaletteStatus,
        CommandReceipt, CommandScope, CommandSource, CommandStatus, HeldCancellation, HeldInputId,
        KeyChord, KeyCode, Modifiers, NativeWindowId,
    };
}

/// Windows inside a view and native windows, regions, splits and tool
/// windows, for `use pert_ui_runtime::windowing::*`.
///
/// An application that arranges its content in movable windows, a
/// [`RegionLayout`], a [`SplitPane`] or a [`ToolWindowLayout`], or presents a
/// view in its own native window, globs this module beside the [`prelude`].
///
/// # Membership
///
/// The module holds the window layer and its windows, native window
/// identities, options and presentation receipts, regions, splits and tool
/// windows with their controllers, changes and modes, and [`WindowError`].
/// It is closed one level: each argument of a member's constructors and
/// builders, and each enum payload or public field type of a member, is here
/// or in the prelude, so it adds the [`NormalizedValue`] of
/// `SplitRule::Ratio` and the [`Rect`] of a tool window's floating bounds. A
/// host's presentation plumbing ([`PresentationRequest`],
/// [`PresentationAction`], [`PreparedDetach`], [`PresentationRate`]) is in
/// [`host`], and the error payload of `WindowError::Drawing` is
/// [`DrawError`] in [`component`].
///
/// The name is `windowing`, not `windows`: a crate that globs the crate root
/// and depends on the `windows` crate would otherwise find the name
/// ambiguous. A name can also be in the prelude or another module; each glob
/// re-exports the same crate-root item, so the globs combine. The re-exports
/// are `#[doc(no_inline)]`: each item is documented once, at the crate root.
///
/// # Example
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{prelude::*, windowing::*};
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// let split = SplitPane::new(
///     SplitAxis::LeftRight,
///     Element::new("navigation", Label::new("Navigation")),
///     Element::new("editor", Label::new("Editor")),
/// )
/// .sizing(SplitSizing::ratio(0.25)?);
/// ui.mount_tree(Element::new("main", split))?;
/// ui.flush(Duration::ZERO)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub mod windowing {
    #[doc(no_inline)]
    pub use crate::{
        DetachedClosePolicy, EdgeMode, Expansion, FloatingWindow, NativeWindowId,
        NativeWindowOptions, NormalizedValue, PresentationReceipt, PresentationStatus, Rect,
        RegionEdge, RegionLayout, SplitAxis, SplitPane, SplitRule, SplitSizing, StripeGroup,
        ToolEdge, ToolWindow, ToolWindowChange, ToolWindowChangeKind, ToolWindowController,
        ToolWindowLayout, ToolWindowMode, WindowCloseAction, WindowController, WindowError,
        WindowLayer, WindowPlacement, WindowStatus,
    };
}

/// Inspection, capture, profiling and validation, for an application's own
/// tooling and tests, for `use pert_ui_runtime::inspection::*`.
///
/// An application that serves its own diagnostic tool, reads inspection
/// snapshots and event history, captures frames, profiles or validates its
/// tree globs this module beside the [`prelude`]. A headless test reaches the
/// common queries and captures through [`testing`].
///
/// # Membership
///
/// The module holds queries and their replies, the inspection service and
/// client, snapshots and component records with their semantics, event
/// history, capture requests and frames, profiling records and validation
/// reports. It is closed one level: each argument of a member's constructors
/// and builders, and each enum payload or public field type of a member, is
/// here or in the prelude, so it adds the [`Rect`] and [`NativeWindowId`] of
/// `CaptureTarget::region`, the [`ComponentDetails`] and [`Invalidation`] of
/// a [`ComponentInspection`], the [`KeyEvent`] of `EventKind::KeyInput` and
/// the [`Viewport`] of a [`WindowInspection`]. A host's [`CaptureJob`] is in
/// [`host`], the error payload of `CaptureError::Drawing` is
/// [`DrawError`] in [`component`], and the fields records take from another
/// family are in that family's module: the parameter names of a
/// [`ParameterInspection`] and an [`EventRecord`] in [`parameter`], and the
/// [`Delivery`] and [`Freshness`] of a [`SourceInspection`] in [`live`].
///
/// A name can also be in the prelude or another module; each glob re-exports
/// the same crate-root item, so the globs combine. The re-exports are
/// `#[doc(no_inline)]`: each item is documented once, at the crate root.
///
/// # Example
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{inspection::*, prelude::*};
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
/// ui.mount_tree(Element::new("title", Label::new("Profile")))?;
/// ui.flush(Duration::ZERO)?;
/// let query = InspectionQuery::components(Selector::key("title")).detail(Detail::Semantics);
/// let result: QueryResult = ui.query(&query);
/// let semantics: &Semantics = result.components[0]
///     .details
///     .semantics
///     .as_ref()
///     .ok_or("the label has semantics")?;
/// assert_eq!(semantics.name.as_deref(), Some("Profile"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub mod inspection {
    #[doc(no_inline)]
    pub use crate::{
        BorderInspection, BoxModelInspection, Capture, CaptureError, CaptureRequest, CaptureTarget,
        ComponentDetails, ComponentInspection, DebugMode, DebugOverlay, Depth, Detail,
        DiagnosticCommand, DragInspection, EventHistory, EventKind, EventRecord, EventStatus,
        FailurePhase, FailureSite, FlushReport, FrameError, FrameImage, InspectValue,
        InspectionClient, InspectionQuery, InspectionReply, InspectionService, InspectionSnapshot,
        Invalidation, KeyEvent, LimitError, NativeWindowId, OverlapPolicy, ParameterInspection,
        Pending, ProfileOperation, ProfileSample, ProfileSnapshot, ProfileStats, QueryResult, Rect,
        RequestError, ResourceKind, Selector, SemanticAction, SemanticItem, SemanticRole,
        SemanticValue, Semantics, ShortcutBinding, ShortcutScope, SourceInspection, ValidationCode,
        ValidationHints, ValidationIssue, ValidationMarker, ValidationReport, Viewport, WaitError,
        WindowInspection,
    };
}

/// Data from other threads, for `use pert_ui_runtime::live::*`: lossy live
/// series and owned values through mailboxes.
///
/// An application whose producer publishes high-rate, lossy data, or whose
/// worker thread hands owned results to the UI thread, globs this module
/// beside the [`prelude`]. The live widgets [`LevelMeter`] and [`Trace`] are
/// here with the [`SeriesSource`] they read.
///
/// # Membership
///
/// The module holds the series bridge ([`series_bridge`], its endpoint,
/// consumer, configuration and items), the UI-thread [`SeriesSource`] with
/// its delivery, freshness and wake-up names, the live widgets, and the
/// mailbox family ([`Mailbox`], [`UiSender`], [`SendError`]), which shares
/// [`Capacity`]. It is closed one level: each argument of a member's
/// constructors and builders, and each enum payload or public field type of a
/// member, is here or in the prelude, so it adds the [`ParameterSpec`] that
/// [`LevelMeter::new`] and [`Trace::new`] take for their range; that
/// specification's own builders' names are in [`parameter`]. The handles of
/// `spawn` and `start_interval` ([`BackgroundTask`], [`IntervalTimer`],
/// [`TaskReceipt`]) stay at the crate root, and [`SourceInspection`] is in
/// [`inspection`].
///
/// A name can also be in the prelude or another module; each glob re-exports
/// the same crate-root item, so the globs combine. The re-exports are
/// `#[doc(no_inline)]`: each item is documented once, at the crate root.
///
/// # Example
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{live::*, prelude::*};
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// // The endpoint is `Send`: a producer thread publishes through it with
/// // bounded work and no allocation.
/// let (mut endpoint, consumer) = series_bridge::<f32>(SeriesConfig::new(Capacity::new(64)?));
/// let source = ui.series_source(consumer, Delivery::Polled);
/// let scale = ParameterSpec::new("Level", 0.0, 1.0, 0.0)?;
/// ui.mount_tree(Element::new("meter", LevelMeter::new(source, scale)))?;
/// ui.flush(Duration::ZERO)?;
/// endpoint.publish(0.5, Timestamp::new(1));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub mod live {
    #[doc(no_inline)]
    pub use crate::{
        Capacity, Delivery, DrainReport, Freshness, Incarnation, LevelMeter, LiveError, LiveItem,
        LiveWaker, Mailbox, MeterAxis, ParameterSpec, PublishOutcome, Sample, SampleWindow,
        SampleWindowEntry, SendError, Sequence, SeriesConfig, SeriesConsumer, SeriesEndpoint,
        SeriesSource, Telemetry, Timestamp, Trace, UiSender, WakeError, WakeFlag, series_bridge,
    };
}

pub use runtime::held::{HeldCancellation, HeldInputId};
