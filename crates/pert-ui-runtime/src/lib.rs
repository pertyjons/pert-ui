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
    CellEdit, SortDirection, Table, TableColumn, TableController, TableItems, TableSort,
    TableSource,
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
pub use described::{DescribeContext, Dynamic, Each, Show};
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
    CollectionFilter, FilterCounts, FilterMatcher, FilterOutcome, FilterTarget, ListFilter,
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
mod inspection;
pub use inspection::{
    Depth, Detail, DiagnosticCommand, EventHistory, InspectionClient, InspectionQuery,
    InspectionReply, InspectionService, Pending, QueryResult, RequestError, Selector, WaitError,
};
mod profiling;
#[cfg(feature = "serde")]
mod serde_float;
pub use profiling::{ProfileOperation, ProfileSample, ProfileSnapshot, ProfileStats};
mod drag;
mod input;
mod live;
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
pub use runtime::tasks::{TaskReceipt, TaskStatus};
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
pub use live::{
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

/// The names ordinary application code and a custom component write, for
/// `use pert_ui_runtime::prelude::*`.
///
/// # Membership
///
/// The prelude holds, and both this prelude and `pert_ui::prelude` follow one
/// rule:
///
/// - the widgets, containers and surfaces, with the types their builders take
///   and their controllers and status values return;
/// - state and parameters: [`Runtime`], signals, parameter specifications and
///   their values, bindings with the outcomes and status values they return,
///   hosts, local ([`ParameterHost`]) and remote ([`RemoteHost`]), with the
///   types their sink, response and recovery methods take and return and the
///   receipts and status values they hand out, and the workspace document
///   with its settings, values and report entries;
/// - layout, theme, typography, keyboard, command and shape types an
///   application configures;
/// - the [`Component`] trait together with every type its method signatures
///   name, the enums a component matches on to read its events, and the drawing
///   types [`Painter`] records and a test reads back ([`DisplayList`],
///   [`DrawCommand`], [`Viewport`] and the images, masks, vectors and text
///   layouts a display list refers to).
///
/// The prelude includes errors returned by [`Component`] method
/// signatures: [`RuntimeError`], [`DrawError`], the
/// [`WorkspaceError`] of [`Component::workspace_state`] and the
/// [`RestoreRejection`] of [`Component::restore_workspace_state`]. Widget
/// configuration also brings [`HexColorError`], [`ListingError`] and
/// [`InputRejection`] into the prelude, and so do the errors an application
/// constructs to report its own failure: a host sink's [`SinkError`] and
/// [`ApplicationError`]. Other error types stay at the crate root, where code
/// names them to match specific failures; `?` converts the leaf errors wrapped
/// by `RuntimeError` into it. Inspection queries and records, profiling,
/// capture, live-data plumbing and host transport ([`Mailbox`], [`UiSender`]
/// and the series bridge) also stay at the crate root. A live widget's
/// [`SeriesSource`], and the bridge that feeds it, stay with the live-data
/// plumbing; the widgets and their builder types are in the prelude.
///
/// The `pert-ui` facade's prelude re-exports this one and adds only its native
/// startup types (`App` and everything it reports, errors included) and the
/// widgets its own features provide (`Svg` with `svg`). A custom
/// component that lays out, draws and handles input can be written from
/// `use pert_ui::prelude::*` alone. One that describes itself for inspection,
/// starts a drag, keeps a receipt, presents in its own native window or hosts
/// an editor session also globs [`component`], which holds every name
/// implementing [`Component`] writes. The re-exports are `#[doc(no_inline)]`:
/// each item is documented once, at the crate root.
///
/// # Root names to add
///
/// These are the crate-root names code most often adds to the glob:
///
/// - live data: [`SeriesSource`], [`series_bridge`], [`SeriesConfig`] and
///   [`Timestamp`];
/// - a custom component's inspection description, drag preview, receipts and
///   native windows: `use pert_ui::component::*` (see [`component`]);
/// - a headless test's driver, inspection queries and capture:
///   `use pert_ui::testing::*` (see [`testing`]).
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::ApplicationError;
    #[doc(no_inline)]
    pub use crate::NumberFormat;
    #[doc(no_inline)]
    pub use crate::{Accordion, Collapsible, ExpandMode, Section};
    #[doc(no_inline)]
    pub use crate::{AlertDialog, Dialog, DialogController, DialogOutcome, DialogStatus};
    #[doc(no_inline)]
    pub use crate::{
        Align, Column, Container, Element, Grid, IntoElement, Justify, Length, Row, Size, Track,
        Wrap,
    };
    #[doc(no_inline)]
    pub use crate::{
        AnyDeliveryMonitor, AnyParameter, AnyParameterHost, AnyParameterSnapshot, AnyPreviewEvent,
        AnySpec, AnyTerminalEvent, AnyValue,
    };
    #[doc(no_inline)]
    pub use crate::{ArcOrigin, InlineSlider, Knob, Orientation, Slider};
    #[doc(no_inline)]
    pub use crate::{
        ArrangeContext, ChangeOrigin, ChildInfo, ChildReveal, ComponentDetails, Constraints,
        ContentRows, MeasureContext, Notifier, RestoreRejection, SelectionChange, SizeLimits,
        ViewTransform, WidgetId,
    };
    #[doc(no_inline)]
    pub use crate::{
        AuthorityReceipt, AuthorityResponse, RemoteCommit, RemoteHost, ResponseDisposition,
        SubmissionDelivery,
    };
    #[doc(no_inline)]
    pub use crate::{
        AuthorityStatus, DeliveryMonitor, DeliveryResolution, DeliverySnapshot, DeliveryStatus,
        PreviewEvent, SinkError, SinkErrorKind, TerminalEvent,
    };
    #[doc(no_inline)]
    pub use crate::{Banner, Toast, ToastController, ToastHost, ToastReceipt, ToastStatus};
    #[doc(no_inline)]
    pub use crate::{
        Breadcrumb, BreadcrumbSegment, ContextMenu, ContextMenuController, Menu, MenuBar, MenuItem,
        MultiSelect, MultiSelectCommit, Select,
    };
    #[doc(no_inline)]
    pub use crate::{
        Button, Checkbox, Chip, Choice, Label, Link, LinkUnderline, RadioGroup, SegmentedControl,
        SelectableText, TextOverflow, TextStyle, Toggle,
    };
    #[doc(no_inline)]
    pub use crate::{
        CellEdit, SortDirection, Table, TableColumn, TableController, TableItems, TableSort,
        TableSource,
    };
    #[doc(no_inline)]
    pub use crate::{
        ChoiceId, ChoiceOption, NormalizedValue, ParameterId, ParameterMapping, ParameterSpec,
    };
    #[doc(no_inline)]
    pub use crate::{CollectionFilter, FilterMatcher, ListFilter};
    #[doc(no_inline)]
    pub use crate::{
        Color, ColorPicker, ColorSwatch, DisplayList, DrawCommand, DrawError, HexColorError,
        Painter, PhysicalExtent, Rect, Viewport,
    };
    #[doc(no_inline)]
    pub use crate::{
        ColorRole, Density, MetricRole, Palette, Spacing, ThemeManager, ThemeMetrics, ThemeMode,
    };
    #[doc(no_inline)]
    pub use crate::{
        CommandContext, CommandEntry, CommandId, CommandKey, CommandOutcome, CommandReceipt,
        CommandScope, CommandSource, CommandStatus, KeyChord, Modifiers,
    };
    #[doc(no_inline)]
    pub use crate::{
        CommandKeyInput, TextCommand, TextCommandOutcome, TextInputState, TextKeyOutcome,
        TextPreedit,
    };
    #[doc(no_inline)]
    pub use crate::{
        CommandMatcher, CommandPalette, CommandPaletteController, CommandPaletteStatus,
    };
    #[doc(no_inline)]
    pub use crate::{
        Component, EventContext, Invalidation, Permission, ReadSignal, Runtime, RuntimeError,
        Signal, StateOwner, UpdateContext,
    };
    #[doc(no_inline)]
    pub use crate::{
        CursorIcon, Divider, MotionPreference, RegionEdge, RegionLayout, Shape, TextRenderingMode,
        UiSettings, UiZoom,
    };
    #[doc(no_inline)]
    pub use crate::{
        DecimalInput, DraftAlign, DraftEditor, DraftEditorController, DraftSelection,
        InlineDecimal, InlineInteger, InputRejection, IntegerInput, SearchField, SearchKey,
        TextArea, TextField,
    };
    #[doc(no_inline)]
    pub use crate::{DescribeContext, Dynamic, Each, Group, ScrollArea, ScrollAxes, Show, View};
    #[doc(no_inline)]
    pub use crate::{
        DirectoryEntry, DirectoryRequest, DirectoryResponse, EntryKind, FileBrowser,
        FileBrowserController, ListingDelivery, ListingError,
    };
    #[doc(no_inline)]
    pub use crate::{
        DragEvent, DragOutcome, DragPayload, DragPhase, DragRole, DropOperation, DropOperations,
        DropResponse,
    };
    #[doc(no_inline)]
    pub use crate::{
        EdgeMode, StripeGroup, ToolEdge, ToolWindow, ToolWindowChange, ToolWindowChangeKind,
        ToolWindowController, ToolWindowLayout, ToolWindowMode,
    };
    #[doc(no_inline)]
    pub use crate::{
        EditOutcome, EditToken, ParameterFamily, ParameterSnapshot, ParameterValue, PowerExponent,
        PreviewOutcome, Revision,
    };
    #[doc(no_inline)]
    pub use crate::{EditSession, Parameter, ParameterHost};
    #[doc(no_inline)]
    pub use crate::{EditorSession, EditorStart};
    #[doc(no_inline)]
    pub use crate::{Expansion, ExpansionChange, WindowPlacement};
    #[doc(no_inline)]
    pub use crate::{
        Field, FieldHandle, FieldStatus, Form, FormFields, FormSummary, Property, PropertyGrid,
    };
    #[doc(no_inline)]
    pub use crate::{FillRule, LineCap, LineJoin, Path, PathBuilder, StrokeStyle};
    #[doc(no_inline)]
    pub use crate::{FilterCounts, FilterOutcome, FilterTarget};
    #[doc(no_inline)]
    pub use crate::{
        FloatingWindow, WindowCloseAction, WindowController, WindowLayer, WindowStatus,
    };
    #[doc(no_inline)]
    pub use crate::{FocusDirection, HeldCancellation, HeldInputId};
    #[doc(no_inline)]
    pub use crate::{FontData, FontId, FontRole, Icon, IconSource, RichText, TextColor, TextSpan};
    #[doc(no_inline)]
    pub use crate::{FontManager, IconGlyph, IconManager};
    #[doc(no_inline)]
    pub use crate::{
        HistoryButtons, HistoryEntry, HistoryEntryId, HistoryNavigator, HistorySnapshot,
        NavigationHistory,
    };
    #[doc(no_inline)]
    pub use crate::{IgnoredEntry, WorkspaceSettings, WorkspaceValue};
    #[doc(no_inline)]
    pub use crate::{Image, ImageFit, ImageSampling, RasterImage};
    #[doc(no_inline)]
    pub use crate::{ImageId, MaskId, MaskImage, TextLayout, VectorGeometry, VectorId};
    #[doc(no_inline)]
    pub use crate::{InlineChoice, InlineReadout, InlineSelect, InlineText, InlineToggle};
    #[doc(no_inline)]
    pub use crate::{InputOutcome, Magnification, Point, PointerButton, PointerEvent, PointerKind};
    #[doc(no_inline)]
    pub use crate::{KeyAction, KeyCode, KeyEvent, KeyTransition, KeyboardState};
    #[doc(no_inline)]
    pub use crate::{LevelMeter, MeterAxis, Trace};
    #[doc(no_inline)]
    pub use crate::{LinkTarget, ListingErrorKind};
    #[doc(no_inline)]
    pub use crate::{
        ListController, ListItems, ListRevealReceipt, ListRevealStatus, ListSource, VirtualList,
    };
    #[doc(no_inline)]
    pub use crate::{ListDrop, ListDropMark, ListDropPlacement, ListDropPoint, ListDropPosition};
    #[doc(no_inline)]
    pub use crate::{ListSelection, ListSelectionChange, SelectionMode, VirtualAxis};
    #[doc(no_inline)]
    pub use crate::{NumberLocale, StringRole, TextSource, UiMessage, UiStrings};
    #[doc(no_inline)]
    pub use crate::{OpaqueColor, QuarterTurn, ViewScale};
    #[doc(no_inline)]
    pub use crate::{
        PickChoice, PickItem, PickMatcher, PickPending, PickStatus, QuickPick, QuickPickController,
    };
    #[doc(no_inline)]
    pub use crate::{Popover, SurfaceDismissal, SurfaceKind, Tooltip};
    #[doc(no_inline)]
    pub use crate::{ProgressBar, ProgressStatus, Spinner};
    #[doc(no_inline)]
    pub use crate::{ScaleLimits, WheelGesture, ZoomController, ZoomView};
    #[doc(no_inline)]
    pub use crate::{SplitAxis, SplitPane, SplitRule, SplitSizing};
    #[doc(no_inline)]
    pub use crate::{Tab, Tabs};
    #[doc(no_inline)]
    pub use crate::{ToolGroup, Toolbar, ToolbarOverflow};
    #[doc(no_inline)]
    pub use crate::{Tree, TreeController, TreeItems, TreeSource};
    #[doc(no_inline)]
    pub use crate::{ViewState, Workspace, WorkspaceError, WorkspacePath, WorkspaceReport};
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
/// The module holds [`Component`], its contexts and every name an
/// implementation has to write. A name is a member when using the trait, a
/// context or a value they hand out requires writing it:
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
///   to keep it: the receipts ([`TaskReceipt`], [`RevealReceipt`],
///   [`PresentationReceipt`], [`CommandReceipt`]) and the
///   [`EditorAttachment`] through which a hosted editor reports the end of
///   its edit.
///
/// A value only passed on, compared or converted in the call that returned
/// it, such as [`FittedText`], is not required. The excluded families stay
/// at the crate root: profiling ([`ProfileSnapshot`]), capture, inspection
/// queries and records, live-data plumbing and host transport. The
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
        PresentationStatus, RevealOutcome, RevealReceipt, RevealRefusal, RevealStatus, TaskReceipt,
        TaskStatus,
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

pub use runtime::held::{HeldCancellation, HeldInputId};
