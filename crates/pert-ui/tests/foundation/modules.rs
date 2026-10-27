//! The membership of `prelude`, `component`, `testing` and the thematic
//! modules: every promised name, each module's sufficiency for the names its
//! closure adds, and that all the globs combine.

/// Each name every module promises, imported one by one in one child module
/// per list, so removing any re-export fails to compile even where no program
/// uses it. One list per module keeps a name from being imported twice.
#[allow(unused_imports)]
mod every_name {
    mod prelude_names {
        #[cfg(feature = "svg")]
        use pert_ui::prelude::Svg;
        use pert_ui::prelude::{
            Accordion, AlertDialog, Align, ApplicationError, Banner, Breadcrumb, BreadcrumbSegment,
            Button, CellEdit, ChangeOrigin, Checkbox, Chip, Choice, Collapsible, CollectionFilter,
            Color, ColorPicker, ColorRole, ColorSwatch, Column, CommandKey, CommandScope,
            Constraints, Container, ContextMenu, ContextMenuController, CursorIcon, DecimalInput,
            Dialog, DialogController, DialogOutcome, DialogStatus, DirectoryEntry,
            DirectoryRequest, DirectoryResponse, Divider, DropOperation, DropOperations, Dynamic,
            Each, Element, EntryKind, EventContext, ExpandMode, ExpansionChange, Field,
            FieldHandle, FieldStatus, FileBrowser, FileBrowserController, FilterCounts,
            FilterMatcher, FilterOutcome, FilterTarget, Form, FormFields, FormSummary, Grid, Group,
            HiddenActive, HistoryButtons, HistoryEntry, HistoryEntryId, HistoryNavigator,
            HistorySnapshot, Icon, Image, ImageFit, ImageSampling, InlineChoice, InlineDecimal,
            InlineInteger, InlineReadout, InlineSelect, InlineText, InlineToggle, InputRejection,
            IntegerInput, IntoElement, Justify, KeyChord, KeyCode, Label, Length, Link, LinkTarget,
            LinkUnderline, ListController, ListDrop, ListDropMark, ListDropPlacement,
            ListDropPoint, ListDropPosition, ListFilter, ListItems, ListRevealReceipt,
            ListRevealStatus, ListSelection, ListSelectionChange, ListSource, ListingDelivery,
            ListingError, ListingErrorKind, Menu, MenuBar, MenuItem, MetricRole, Modifiers,
            MultiSelect, MultiSelectCommit, NavigationHistory, NumberFormat, NumberInputError,
            Orientation, Permission, PickChoice, PickItem, PickMatcher, PickPending, PickStatus,
            Point, Popover, ProgressBar, ProgressStatus, Property, PropertyGrid, QuarterTurn,
            QuickPick, QuickPickController, RadioGroup, RasterImage, ReadSignal, RichText, Row,
            Runtime, RuntimeError, ScaleLimits, ScrollArea, ScrollAxes, SearchField, SearchKey,
            Section, SegmentedControl, Select, SelectableText, SelectionChange, SelectionMode,
            Show, Signal, Size, SizeLimits, SortDirection, Spacing, Spinner, StateOwner,
            SurfaceDismissal, Tab, Table, TableColumn, TableController, TableItems, TableSort,
            TableSource, Tabs, TextArea, TextColor, TextCommand, TextField, TextOverflow, TextSpan,
            TextStyle, Toast, ToastController, ToastHost, ToastReceipt, ToastStatus, Toggle,
            ToolGroup, Toolbar, ToolbarOverflow, Tooltip, Track, Tree, TreeController, TreeItems,
            TreeSource, View, ViewScale, ViewTransform, VirtualList, WheelGesture, WidgetId, Wrap,
            ZoomController, ZoomView,
        };
        #[cfg(feature = "native")]
        use pert_ui::prelude::{
            App, AppError, AppShutdownError, ErrorAction, NativeCloseAction, NativeWindowId,
            WindowError,
        };
    }
    mod component_names {
        use pert_ui::component::{
            ArrangeContext, BorderInspection, BoxModelInspection, ChildInfo, ChildReveal,
            ClickCounter, Color, ColorRole, CommandContext, CommandId, CommandKey, CommandKeyInput,
            CommandOutcome, CommandReceipt, CommandSource, CommandStatus, Component,
            ComponentDetails, Constraints, ContentRows, CursorIcon, DRAG_THRESHOLD,
            DetachedClosePolicy, DisplayList, DraftAlign, DraftEditor, DraftEditorController,
            DraftSelection, DragEvent, DragOutcome, DragPayload, DragPhase, DragPreview, DragRole,
            DrawCommand, DrawError, DropOperation, DropOperations, DropResponse, EditorAttachment,
            EditorOutcome, EditorSession, EditorStart, EditorTrigger, Element, EventContext,
            FillRule, FocusDirection, Icon, IconSource, ImageId, ImageSampling, InputOutcome,
            InspectValue, Invalidation, KeyAction, KeyCode, KeyEvent, KeyTransition, KeyboardState,
            Length, LineCap, LineJoin, Magnification, MaskId, MaskImage, MeasureContext,
            MetricRole, Modifiers, MotionPreference, NativeWindowId, NativeWindowOptions, Notifier,
            OverlapPolicy, Painter, Path, PathBuilder, PathElement, Point, PointerButton,
            PointerEvent, PointerKind, PresentationReceipt, PresentationStatus, QuarterTurn,
            RasterImage, ReadSignal, Rect, RestoreRejection, RevealOutcome, RevealReceipt,
            RevealRefusal, RevealStatus, RichText, RuntimeError, SemanticAction, SemanticItem,
            SemanticRole, SemanticValue, Semantics, Shape, ShortcutBinding, ShortcutScope, Signal,
            Size, SizeLimits, StrokeStyle, SurfaceDismissal, SurfaceKind, TaskReceipt, TaskStatus,
            TextCommand, TextCommandOutcome, TextInputState, TextKeyOutcome, TextLayout,
            TextOverflow, TextPreedit, TextStyle, Transform2D, UpdateContext, ValidationHints,
            VectorGeometry, VectorId, ViewScale, ViewState, ViewTransform, Viewport, WidgetId,
            WorkspaceError, WorkspaceValue,
        };
    }
    mod testing_names {
        #[cfg(feature = "cpu")]
        use pert_ui::testing::CpuRenderer;
        #[cfg(feature = "native")]
        use pert_ui::testing::HeadlessApp;
        use pert_ui::testing::{
            CaptureRequest, CaptureTarget, ClickCounter, CommandKeyInput, Detail, Driver,
            FocusDirection, FrameImage, InputOutcome, InspectionQuery, KeyAction, PointerButton,
            PointerEvent, PointerKind, Selector, SemanticValue, TestError, Viewport,
        };
    }
    mod parameter_names {
        use pert_ui::parameter::{
            AnyDeliveryMonitor, AnyParameter, AnyParameterHost, AnyParameterSnapshot,
            AnyPreviewEvent, AnySpec, AnyTerminalEvent, AnyValue, ArcOrigin, AuthorityReceipt,
            AuthorityResponse, AuthorityStatus, BindingError, ChoiceId, ChoiceOption,
            DeliveryError, DeliveryMonitor, DeliveryResolution, DeliverySnapshot, DeliveryStatus,
            EditError, EditOutcome, EditSequence, EditSession, EditSource, EditToken, HostValue,
            InlineSlider, Knob, NormalizedValue, NormalizedValueError, Parameter, ParameterError,
            ParameterFamily, ParameterHost, ParameterId, ParameterIdError, ParameterMapping,
            ParameterSnapshot, ParameterSpec, ParameterState, ParameterValue, PowerExponent,
            PreviewEvent, PreviewOutcome, RemoteCommit, RemoteHost, ResponseDisposition, Revision,
            SinkError, SinkErrorKind, Slider, SubmissionDelivery, TerminalEvent,
        };
    }
    mod commands_names {
        use pert_ui::commands::{
            CommandContext, CommandEntry, CommandId, CommandInfo, CommandKey, CommandMatcher,
            CommandOutcome, CommandPalette, CommandPaletteController, CommandPaletteStatus,
            CommandReceipt, CommandScope, CommandSource, CommandStatus, HeldCancellation,
            HeldInputId, KeyChord, KeyCode, Modifiers, NativeWindowId,
        };
    }
    mod windowing_names {
        use pert_ui::windowing::{
            DetachedClosePolicy, EdgeMode, Expansion, FloatingWindow, NativeWindowId,
            NativeWindowOptions, NormalizedValue, PresentationReceipt, PresentationStatus, Rect,
            RegionEdge, RegionLayout, SplitAxis, SplitPane, SplitRule, SplitSizing, StripeGroup,
            ToolEdge, ToolWindow, ToolWindowChange, ToolWindowChangeKind, ToolWindowController,
            ToolWindowLayout, ToolWindowMode, WindowCloseAction, WindowController, WindowError,
            WindowLayer, WindowPlacement, WindowStatus,
        };
    }
    mod inspection_names {
        use pert_ui::inspection::{
            BorderInspection, BoxModelInspection, Capture, CaptureError, CaptureRequest,
            CaptureTarget, ComponentDetails, ComponentInspection, DebugMode, DebugOverlay, Depth,
            Detail, DiagnosticCommand, DragInspection, EventHistory, EventKind, EventRecord,
            EventStatus, FailurePhase, FailureSite, FlushReport, FrameError, FrameImage,
            InspectValue, InspectionClient, InspectionQuery, InspectionReply, InspectionService,
            InspectionSnapshot, Invalidation, KeyEvent, LimitError, NativeWindowId, OverlapPolicy,
            ParameterInspection, Pending, ProfileOperation, ProfileSample, ProfileSnapshot,
            ProfileStats, QueryResult, Rect, RequestError, ResourceKind, Selector, SemanticAction,
            SemanticItem, SemanticRole, SemanticValue, Semantics, ShortcutBinding, ShortcutScope,
            SourceInspection, ValidationCode, ValidationHints, ValidationIssue, ValidationMarker,
            ValidationReport, Viewport, WaitError, WindowInspection,
        };
    }
    mod live_names {
        use pert_ui::live::{
            Capacity, Delivery, DrainReport, Freshness, Incarnation, LevelMeter, LiveError,
            LiveItem, LiveWaker, Mailbox, MeterAxis, ParameterSpec, PublishOutcome, Sample,
            SampleWindow, SampleWindowEntry, SendError, Sequence, SeriesConfig, SeriesConsumer,
            SeriesEndpoint, SeriesSource, Telemetry, Timestamp, Trace, UiSender, WakeError,
            WakeFlag, series_bridge,
        };
    }
}

/// Every glob a program might write, together with the crate root: each name
/// any of the modules exports is resolved in every namespace it has, so a
/// glob-ambiguous name fails to compile (E0659) wherever it would be used.
#[allow(unused_imports)]
mod every_glob_combines {
    use pert_ui::*;
    use pert_ui::{commands::*, component::*, inspection::*, live::*, parameter::*};
    use pert_ui::{prelude::*, testing::*, windowing::*};

    #[cfg(feature = "cpu")]
    use self::CpuRenderer as _;
    #[cfg(feature = "svg")]
    use self::Svg as _;
    use self::{
        Accordion as _, AlertDialog as _, Align as _, AnyDeliveryMonitor as _, AnyParameter as _,
        AnyParameterHost as _, AnyParameterSnapshot as _, AnyPreviewEvent as _, AnySpec as _,
        AnyTerminalEvent as _, AnyValue as _, ApplicationError as _, ArcOrigin as _,
        ArrangeContext as _, AuthorityReceipt as _, AuthorityResponse as _, AuthorityStatus as _,
        Banner as _, BindingError as _, BorderInspection as _, BoxModelInspection as _,
        Breadcrumb as _, BreadcrumbSegment as _, Button as _, Capacity as _, Capture as _,
        CaptureError as _, CaptureRequest as _, CaptureTarget as _, CellEdit as _,
        ChangeOrigin as _, Checkbox as _, ChildInfo as _, ChildReveal as _, Chip as _, Choice as _,
        ChoiceId as _, ChoiceOption as _, ClickCounter as _, Collapsible as _,
        CollectionFilter as _, Color as _, ColorPicker as _, ColorRole as _, ColorSwatch as _,
        Column as _, CommandContext as _, CommandEntry as _, CommandId as _, CommandInfo as _,
        CommandKey as _, CommandKeyInput as _, CommandMatcher as _, CommandOutcome as _,
        CommandPalette as _, CommandPaletteController as _, CommandPaletteStatus as _,
        CommandReceipt as _, CommandScope as _, CommandSource as _, CommandStatus as _,
        Component as _, ComponentDetails as _, ComponentInspection as _, Constraints as _,
        Container as _, ContentRows as _, ContextMenu as _, ContextMenuController as _,
        CursorIcon as _, DRAG_THRESHOLD as _, DebugMode as _, DebugOverlay as _, DecimalInput as _,
        Delivery as _, DeliveryError as _, DeliveryMonitor as _, DeliveryResolution as _,
        DeliverySnapshot as _, DeliveryStatus as _, Depth as _, DetachedClosePolicy as _,
        Detail as _, DiagnosticCommand as _, Dialog as _, DialogController as _,
        DialogOutcome as _, DialogStatus as _, DirectoryEntry as _, DirectoryRequest as _,
        DirectoryResponse as _, DisplayList as _, Divider as _, DraftAlign as _, DraftEditor as _,
        DraftEditorController as _, DraftSelection as _, DragEvent as _, DragInspection as _,
        DragOutcome as _, DragPayload as _, DragPhase as _, DragPreview as _, DragRole as _,
        DrainReport as _, DrawCommand as _, DrawError as _, Driver as _, DropOperation as _,
        DropOperations as _, DropResponse as _, Dynamic as _, Each as _, EdgeMode as _,
        EditError as _, EditOutcome as _, EditSequence as _, EditSession as _, EditSource as _,
        EditToken as _, EditorAttachment as _, EditorOutcome as _, EditorSession as _,
        EditorStart as _, EditorTrigger as _, Element as _, EntryKind as _, EventContext as _,
        EventHistory as _, EventKind as _, EventRecord as _, EventStatus as _, ExpandMode as _,
        Expansion as _, ExpansionChange as _, FailurePhase as _, FailureSite as _, Field as _,
        FieldHandle as _, FieldStatus as _, FileBrowser as _, FileBrowserController as _,
        FillRule as _, FilterCounts as _, FilterMatcher as _, FilterOutcome as _,
        FilterTarget as _, FloatingWindow as _, FlushReport as _, FocusDirection as _, Form as _,
        FormFields as _, FormSummary as _, FrameError as _, FrameImage as _, Freshness as _,
        Grid as _, Group as _, HeldCancellation as _, HeldInputId as _, HiddenActive as _,
        HistoryButtons as _, HistoryEntry as _, HistoryEntryId as _, HistoryNavigator as _,
        HistorySnapshot as _, HostValue as _, Icon as _, IconSource as _, Image as _,
        ImageFit as _, ImageId as _, ImageSampling as _, Incarnation as _, InlineChoice as _,
        InlineDecimal as _, InlineInteger as _, InlineReadout as _, InlineSelect as _,
        InlineSlider as _, InlineText as _, InlineToggle as _, InputOutcome as _,
        InputRejection as _, InspectValue as _, InspectionClient as _, InspectionQuery as _,
        InspectionReply as _, InspectionService as _, InspectionSnapshot as _, IntegerInput as _,
        IntoElement as _, Invalidation as _, Justify as _, KeyAction as _, KeyChord as _,
        KeyCode as _, KeyEvent as _, KeyTransition as _, KeyboardState as _, Knob as _, Label as _,
        Length as _, LevelMeter as _, LimitError as _, LineCap as _, LineJoin as _, Link as _,
        LinkTarget as _, LinkUnderline as _, ListController as _, ListDrop as _, ListDropMark as _,
        ListDropPlacement as _, ListDropPoint as _, ListDropPosition as _, ListFilter as _,
        ListItems as _, ListRevealReceipt as _, ListRevealStatus as _, ListSelection as _,
        ListSelectionChange as _, ListSource as _, ListingDelivery as _, ListingError as _,
        ListingErrorKind as _, LiveError as _, LiveItem as _, LiveWaker as _, Magnification as _,
        Mailbox as _, MaskId as _, MaskImage as _, MeasureContext as _, Menu as _, MenuBar as _,
        MenuItem as _, MeterAxis as _, MetricRole as _, Modifiers as _, MotionPreference as _,
        MultiSelect as _, MultiSelectCommit as _, NativeWindowId as _, NativeWindowOptions as _,
        NavigationHistory as _, NormalizedValue as _, NormalizedValueError as _, Notifier as _,
        NumberFormat as _, NumberInputError as _, Orientation as _, OverlapPolicy as _,
        Painter as _, Parameter as _, ParameterError as _, ParameterFamily as _,
        ParameterHost as _, ParameterId as _, ParameterIdError as _, ParameterInspection as _,
        ParameterMapping as _, ParameterSnapshot as _, ParameterSpec as _, ParameterState as _,
        ParameterValue as _, Path as _, PathBuilder as _, PathElement as _, Pending as _,
        Permission as _, PickChoice as _, PickItem as _, PickMatcher as _, PickPending as _,
        PickStatus as _, Point as _, PointerButton as _, PointerEvent as _, PointerKind as _,
        Popover as _, PowerExponent as _, PresentationReceipt as _, PresentationStatus as _,
        PreviewEvent as _, PreviewOutcome as _, ProfileOperation as _, ProfileSample as _,
        ProfileSnapshot as _, ProfileStats as _, ProgressBar as _, ProgressStatus as _,
        Property as _, PropertyGrid as _, PublishOutcome as _, QuarterTurn as _, QueryResult as _,
        QuickPick as _, QuickPickController as _, RadioGroup as _, RasterImage as _,
        ReadSignal as _, Rect as _, RegionEdge as _, RegionLayout as _, RemoteCommit as _,
        RemoteHost as _, RequestError as _, ResourceKind as _, ResponseDisposition as _,
        RestoreRejection as _, RevealOutcome as _, RevealReceipt as _, RevealRefusal as _,
        RevealStatus as _, Revision as _, RichText as _, Row as _, Runtime as _, RuntimeError as _,
        Sample as _, SampleWindow as _, SampleWindowEntry as _, ScaleLimits as _, ScrollArea as _,
        ScrollAxes as _, SearchField as _, SearchKey as _, Section as _, SegmentedControl as _,
        Select as _, SelectableText as _, SelectionChange as _, SelectionMode as _, Selector as _,
        SemanticAction as _, SemanticItem as _, SemanticRole as _, SemanticValue as _,
        Semantics as _, SendError as _, Sequence as _, SeriesConfig as _, SeriesConsumer as _,
        SeriesEndpoint as _, SeriesSource as _, Shape as _, ShortcutBinding as _,
        ShortcutScope as _, Show as _, Signal as _, SinkError as _, SinkErrorKind as _, Size as _,
        SizeLimits as _, Slider as _, SortDirection as _, SourceInspection as _, Spacing as _,
        Spinner as _, SplitAxis as _, SplitPane as _, SplitRule as _, SplitSizing as _,
        StateOwner as _, StripeGroup as _, StrokeStyle as _, SubmissionDelivery as _,
        SurfaceDismissal as _, SurfaceKind as _, Tab as _, Table as _, TableColumn as _,
        TableController as _, TableItems as _, TableSort as _, TableSource as _, Tabs as _,
        TaskReceipt as _, TaskStatus as _, Telemetry as _, TerminalEvent as _, TestError as _,
        TextArea as _, TextColor as _, TextCommand as _, TextCommandOutcome as _, TextField as _,
        TextInputState as _, TextKeyOutcome as _, TextLayout as _, TextOverflow as _,
        TextPreedit as _, TextSpan as _, TextStyle as _, Timestamp as _, Toast as _,
        ToastController as _, ToastHost as _, ToastReceipt as _, ToastStatus as _, Toggle as _,
        ToolEdge as _, ToolGroup as _, ToolWindow as _, ToolWindowChange as _,
        ToolWindowChangeKind as _, ToolWindowController as _, ToolWindowLayout as _,
        ToolWindowMode as _, Toolbar as _, ToolbarOverflow as _, Tooltip as _, Trace as _,
        Track as _, Transform2D as _, Tree as _, TreeController as _, TreeItems as _,
        TreeSource as _, UiSender as _, UpdateContext as _, ValidationCode as _,
        ValidationHints as _, ValidationIssue as _, ValidationMarker as _, ValidationReport as _,
        VectorGeometry as _, VectorId as _, View as _, ViewScale as _, ViewState as _,
        ViewTransform as _, Viewport as _, VirtualList as _, WaitError as _, WakeError as _,
        WakeFlag as _, WheelGesture as _, WidgetId as _, WindowCloseAction as _,
        WindowController as _, WindowError as _, WindowInspection as _, WindowLayer as _,
        WindowPlacement as _, WindowStatus as _, WorkspaceError as _, WorkspaceValue as _,
        Wrap as _, ZoomController as _, ZoomView as _, series_bridge as _,
    };
    #[cfg(feature = "native")]
    use self::{
        App as _, AppError as _, AppShutdownError as _, ErrorAction as _, HeadlessApp as _,
        NativeCloseAction as _,
    };
}

/// Each thematic module with the prelude is enough to construct the members
/// whose closure the module adds; each child module imports nothing else from
/// `pert_ui`.
mod sufficiency {
    mod inspection {
        use pert_ui::{inspection::*, prelude::*};

        #[test]
        fn a_capture_region_a_key_record_and_a_shortcut_are_written()
        -> Result<(), Box<dyn std::error::Error>> {
            let ui = Runtime::new();
            let window = ui.primary_window();
            let target = CaptureTarget::region(&window, Rect::new(0.0, 0.0, 10.0, 10.0)?);
            let _request = CaptureRequest::new(target);
            let key = |kind: &EventKind| matches!(kind, EventKind::KeyInput(KeyEvent { .. }));
            let _ = key;
            let binding = ShortcutBinding {
                modifiers: Modifiers::PRIMARY,
                ..ShortcutBinding::new(KeyCode::Enter, "Save")
            };
            assert_eq!(binding.key, KeyCode::Enter);
            Ok(())
        }
    }

    mod windowing {
        use pert_ui::{prelude::*, windowing::*};

        #[test]
        fn a_tool_window_change_and_a_split_ratio_are_written()
        -> Result<(), Box<dyn std::error::Error>> {
            let change =
                ToolWindowChange::new("files".to_owned(), Expansion::Expanded, ChangeOrigin::User)
                    .with_geometry(None, Some(Rect::new(0.0, 0.0, 100.0, 80.0)?));
            assert_eq!(change.clone(), change);
            let ratio = match SplitSizing::ratio(0.25)?.rule() {
                SplitRule::Ratio(ratio) => ratio,
                _ => NormalizedValue::try_from(0.5)?,
            };
            assert_eq!(ratio, NormalizedValue::try_from(0.25)?);
            Ok(())
        }
    }

    mod commands {
        use pert_ui::{commands::*, prelude::*};

        #[test]
        fn a_window_scope_is_written() -> Result<(), Box<dyn std::error::Error>> {
            let mut ui = Runtime::new();
            let window = ui.primary_window();
            let id: CommandId = ui.register_command(
                "Close",
                CommandScope::Window(window),
                vec![KeyChord::named(KeyCode::Escape)],
                |_, _| Ok(()),
            )?;
            assert_eq!(ui.command_name(&id)?, "Close");
            Ok(())
        }
    }

    mod live {
        use pert_ui::{live::*, prelude::*};

        #[test]
        fn a_meter_takes_its_range() -> Result<(), Box<dyn std::error::Error>> {
            let mut ui = Runtime::new();
            let (_endpoint, consumer) = series_bridge::<f32>(SeriesConfig::new(Capacity::new(8)?));
            let source = ui.series_source(consumer, Delivery::Polled);
            let meter = LevelMeter::new(source, ParameterSpec::new("Level", 0.0, 1.0, 0.0)?);
            ui.mount_tree(meter.key("meter"))?;
            Ok(())
        }
    }

    mod parameter {
        use pert_ui::{parameter::*, prelude::*};

        #[test]
        fn a_slider_is_bound_to_a_parameter() -> Result<(), Box<dyn std::error::Error>> {
            let mut ui = Runtime::new();
            let (gain, _host) = ui.parameter(
                ParameterId::new(1, 1, "gain")?,
                ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
            )?;
            ui.mount_tree(Slider::new(gain).key("gain"))?;
            Ok(())
        }
    }
}
