// The crate page is rustdoc-specific (intra-doc links, hidden doctest lines);
// README.md is the registry page. Both carry status.md, which tests/foundation/readme.rs
// keeps identical.
#![doc = include_str!("root.md")]
#![doc = include_str!("status.md")]
#![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
#![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
#![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
#![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
#![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
#![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
#![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
#![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]

pub mod guide;

// Runs the registry README's example as a doctest; it opens a window, so it
// needs `desktop`.
#[cfg(all(doctest, feature = "native"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

#[cfg(all(feature = "native", not(any(feature = "gpu", feature = "cpu"))))]
compile_error!(
    "the `native` feature needs a renderer: enable `desktop` (GPU) or `desktop-cpu` (CPU)"
);

#[doc(inline)]
pub use pert_ui_runtime::*;
#[cfg(feature = "native")]
mod app;
#[cfg(feature = "cpu")]
pub mod cpu;
#[cfg(feature = "images")]
pub mod images;
#[cfg(feature = "native")]
mod platform;
#[cfg(feature = "svg")]
pub mod svg_images;
// Placement: every widget, including a feature-gated one, is at the crate root;
// a feature module holds its loaders, rasterizers and their errors.
#[cfg(feature = "native")]
pub use app::{App, AppError, AppShutdownError, ErrorAction, NativeCloseAction};
#[cfg(feature = "svg")]
pub use svg_images::component::Svg;

/// The names an application's first program writes, for
/// `use pert_ui::prelude::*`.
///
/// This is the runtime prelude and, with the `native` feature, which
/// `desktop` and `desktop-cpu` enable, `App` and everything it reports,
/// errors included (`AppError`, its shutdown payload `AppShutdownError`, the
/// [`WindowError`] that `App::on_window_error` reports, `ErrorAction`,
/// `NativeCloseAction` and [`NativeWindowId`]); with `svg`, the `Svg` widget.
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
///   [`NativeWindowId`], which this prelude holds with `native` and
///   [`commands`] and [`windowing`] hold in every configuration.
/// - The editors' `session` builders take an [`EditorSession`]: [`component`].
/// - Arguments taken as `impl Into<TextSource>` or `impl Into<IconSource>` are
///   written as strings, signals and [`Icon`]s; [`TextSource`] is at the crate
///   root and [`IconSource`] in [`component`]. [`ReadSignal::combine`] takes
///   its inputs as a tuple of signals and read bindings; [`CombineInputs`] and
///   [`CombineInput`], which only generic code names, are at the crate root.
/// - The [`DescribeContext`] an [`Each`], [`Dynamic`] or [`Show`] closure
///   receives, and the [`EachItem`] an [`Each`]'s `build` returns, are
///   inferred; code that names either imports it from the crate root.
/// - [`TextEditError`], which `on_binding_refusal` reports, and the other
///   error types stay at the crate root, where code names them to match a
///   failure; `?` converts the leaf errors [`RuntimeError`] wraps into it.
///   Asset-loading errors such as `svg_images::SvgError` stay in their
///   feature modules.
/// - Services: [`NumberFormat`]'s localized constructors take a
///   [`NumberLocale`], and [`TextSpan::font`] and [`TextSpan::font_role`] take
///   a [`FontId`] and a [`FontRole`]; they are at the crate root.
/// - [`ProgressStatus::of`] builds progress from counts and
///   [`ProgressStatus::fraction`] carries a running task's fraction into
///   `Failed` or `Cancelled`; a fraction written as a number is a
///   [`NormalizedValue`] from [`parameter`].
/// - [`Runtime`] and [`EventContext`] have methods for every family; a
///   method's arguments are in that family's module.
///
/// # Where the other names are
///
/// | Names | Import |
/// |---|---|
/// | Implementing [`Component`]: contexts, drawing, input events, drag and drop, receipts, editor sessions and the embedded draft editor | `use pert_ui::component::*` ([`component`]) |
/// | A headless test: the driver, the viewport, key and pointer input, semantics, query results and validation, queries, captures, the CPU renderer and the headless application | `use pert_ui::testing::*` ([`testing`]) |
/// | A custom host that drives the runtime in place of `App`: [`HostIntegration`](host::HostIntegration) and the names a host writes | `use pert_ui::host::*` ([`host`]) |
/// | Authoritative values: parameters, edit gestures, hosts, external authority, and [`Slider`], [`Knob`] and [`InlineSlider`] | `use pert_ui::parameter::*` ([`parameter`]) |
/// | Commands beyond a shortcut's chord and scope, held input and the command palette | `use pert_ui::commands::*` ([`commands`]) |
/// | Windows inside a view, native windows, regions, splits and tool windows | `use pert_ui::windowing::*` ([`windowing`]) |
/// | Inspection, capture, profiling and validation | `use pert_ui::inspection::*` ([`inspection`]) |
/// | Data from other threads: live series, mailboxes, [`LevelMeter`] and [`Trace`] | `use pert_ui::live::*` ([`live`]) |
/// | Theme, UI settings, fonts and icons, strings and locale, the workspace document, other errors, [`DescribeContext`], [`EachItem`], [`PhysicalExtent`] and [`VirtualAxis`] | by name from the crate root, such as `use pert_ui::{Palette, Workspace};` |
///
/// A name can be in the prelude and in a module, or in several modules: each
/// re-exports the same crate-root item, so the globs combine without
/// conflict. Glob the prelude and the modules rather than the crate root: a
/// crate-root glob also brings in the module names, which can collide with a
/// dependency crate of the same name. The re-exports are `#[doc(no_inline)]`:
/// each item is documented once, at the crate root.
pub mod prelude {
    #[cfg(feature = "svg")]
    #[doc(no_inline)]
    pub use crate::Svg;
    #[cfg(feature = "native")]
    #[doc(no_inline)]
    pub use crate::{
        App, AppError, AppShutdownError, ErrorAction, NativeCloseAction, NativeWindowId,
        WindowError,
    };
    #[doc(no_inline)]
    pub use pert_ui_runtime::prelude::*;
}

/// The names implementing [`Component`] writes, for
/// `use pert_ui::component::*`.
///
/// Applications glob [`prelude`]; code that implements [`Component`] also
/// globs this module, which re-exports `pert_ui_runtime::component`. Two globs
/// that re-export one item do not conflict, so the names both hold, such as
/// [`Component`] and [`Painter`], can be imported through either.
///
/// It holds [`Component`], its contexts, the built-in editor a component
/// embeds ([`DraftEditor`] with its controller, alignment and selection) and
/// every name an implementation has to write: an argument the component
/// constructs, such as a [`DragPreview`];
/// an enum it matches, including the status of a receipt it keeps; a field or
/// payload it builds or destructures for a [`Component`] method, such as the
/// [`Semantics`] and [`InspectValue`] parts of the [`ComponentDetails`] that
/// [`Component::inspect`] returns; and a value it keeps where the rustdoc says
/// to keep it, such as a [`TaskReceipt`] or an [`EditorAttachment`]. A value
/// only passed on, compared or converted in the call that returned it, such as
/// [`FittedText`], is not required. Profiling, capture and inspection queries
/// and records are in [`inspection`], live-data plumbing in [`live`], and host
/// transport in [`host`]. The re-exports are `#[doc(no_inline)]`: each item is documented once, at
/// the
/// crate root.
///
/// ```
/// use pert_ui::{component::*, prelude::*};
///
/// /// A marker that names itself for inspection.
/// struct Marker;
///
/// impl Component for Marker {
///     fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> {
///         Ok(())
///     }
///     fn inspect(&self) -> ComponentDetails {
///         ComponentDetails {
///             semantics: Some(Semantics::new(SemanticRole::Image).name("marker")),
///             ..ComponentDetails::default()
///         }
///     }
/// }
///
/// let mut ui = Runtime::new();
/// ui.mount_tree(Element::new("marker", Marker))?;
/// # Ok::<(), RuntimeError>(())
/// ```
pub mod component {
    #[doc(no_inline)]
    pub use pert_ui_runtime::component::*;
}

/// Driving an application in a headless test, for `use pert_ui::testing::*`.
///
/// [`Driver`](testing::Driver) clicks a component, replaces or types text,
/// presses keys and chords, clears focus, settles the runtime after each step,
/// checks that a window stays quiet over an interval
/// ([`QuietReport`](testing::QuietReport)), finds a component's
/// [`Semantics`] and lists the components one address segment below a parent
/// in order ([`AddressChild`](testing::AddressChild));
/// [`TestError`](testing::TestError) names what a step could not do. Neither needs a window or a GPU. The module also
/// re-exports what a headless test writes besides the driver and the
/// prelude:
///
/// - input by hand and the viewport: the [`Viewport`] every harness sets;
///   [`KeyAction`], [`FocusDirection`], [`PointerEvent`], [`PointerKind`] with
///   the [`Magnification`] of a pinch, [`PointerButton`] and
///   [`CommandKeyInput`], with the [`InputOutcome`] input returns; and
///   [`ClickCounter`];
/// - what the driver's steps return: [`FlushReport`], which
///   [`Driver::settle`](testing::Driver::settle) and
///   [`Driver::advance`](testing::Driver::advance) return, the
///   [`RevealRefusal`] a `TestError::NotRevealed` carries, the
///   [`DrawError`] a `TestError::Draw` carries and each
///   [`AddressChild`](testing::AddressChild) that
///   [`Driver::address_children`](testing::Driver::address_children) lists;
/// - semantics: [`Semantics`] with [`SemanticRole`], [`SemanticValue`],
///   [`SemanticAction`] and [`SemanticItem`];
/// - queries: [`InspectionQuery`], [`Selector`], [`Detail`] and [`Depth`] for
///   [`Runtime::query`], and the [`QueryResult`] it returns, with its
///   [`ComponentInspection`]s, their [`ComponentDetails`] and the
///   [`InspectValue`] properties those hold;
/// - development validation: [`ValidationReport`] from
///   [`Runtime::validation_report`], with [`ValidationIssue`],
///   [`ValidationCode`] and [`ValidationMarker`];
/// - captures: [`CaptureRequest`] and [`CaptureTarget`], with the
///   [`NativeWindowId`] and [`Rect`] a target names, and [`Pending`],
///   [`InspectionReply`], [`WaitError`], [`Capture`], [`CaptureError`] and
///   [`FrameImage`] for [`Runtime::request_capture`];
/// - [`HostIntegration`](host::HostIntegration), because a test that selects
///   a native window or takes presentation requests stands in for a
///   [`host`]. A test clears focus with
///   [`Driver::clear_focus`](testing::Driver::clear_focus) and checks
///   idleness with [`Driver::quiet_for`](testing::Driver::quiet_for) beside
///   the driver's other actions.
///
/// With the `cpu` feature it adds [`CpuRenderer`], which serves captures
/// without a GPU, and the [`CpuRenderError`] its rendering returns, and with
/// the `native` feature, which `desktop` and `desktop-cpu` enable,
/// [`HeadlessApp`], an [`App`] started without a window by [`App::headless`]
/// so its content and its close policy can be tested.
///
/// # Membership
///
/// `testing` holds the driver and its error and everything else a headless
/// test writes besides the prelude: the viewport, input by hand, the query,
/// validation and capture names, the CPU renderer and the headless
/// application, and every type those names hand a test. It is closed: each
/// type that a member's constructors, methods and associated constants take
/// or return, and each type a member's public fields and variants hold, is in
/// `testing` or in the prelude, and the same holds for every name this adds.
/// An error nested inside another error is reached through `?` and `Display`
/// and stays where it is. The exceptions below are records a test reads by
/// field or passes on, and each is named with its module.
///
/// # Where the other names are
///
/// - The window, event, parameter and live-source answers of a query
///   ([`WindowInspection`], [`EventHistory`], [`ParameterInspection`],
///   [`SourceInspection`]) are records for an application's own tooling, in
///   [`inspection`].
/// - What a component reports about its geometry, invalidation and
///   validation hints ([`BoxModelInspection`], [`Invalidation`],
///   [`ValidationHints`]) is read by field and is in [`component`] and
///   [`inspection`].
/// - Key routing and key records a test's key input ignores:
///   [`KeyDispatch`], which [`Runtime::command_key_input`] returns, in
///   [`host`]; [`KeyEvent`], which [`Runtime::key_input`] returns, in
///   [`component`] and [`inspection`]; and [`KeyTransition`] in
///   [`component`].
/// - A host's [`PhysicalExtent`], for [`Viewport::from_physical`], is in
///   [`host`]; a test builds its viewport with [`Viewport::new`].
/// - The [`DisplayList`] that [`Runtime::paint`] returns is passed on to a
///   renderer, and is in [`component`] and [`host`]. The identities of its
///   resources, [`VectorId`], [`MaskId`] and [`ImageId`], which
///   `DrawError::MissingVector`, `MissingMask` and `MissingImage` hold, are
///   in [`component`].
/// - [`GeometryError`], which [`Viewport::new`] and the other value
///   constructors return, is propagated with `?` and stays at the crate root
///   with the other errors; so does an error nested in another, such as the
///   [`PathError`] of `DrawError::Path` or the [`LimitError`] of
///   `CaptureError::Limit`.
/// - What the [`HostIntegration`](host::HostIntegration) methods take and
///   return, such as presentation requests, capture jobs and the clipboard
///   service, is the [`host`] module's vocabulary.
///
/// The re-exported names stay at the crate root too, except
/// [`HostIntegration`](host::HostIntegration), which is reached through
/// [`host`] and this module, [`CpuRenderer`] and [`CpuRenderError`], which
/// are in [`cpu`], and [`HeadlessApp`], which only this module exports.
///
/// The [testing chapter](crate::guide::testing) of the guide walks through a
/// test.
///
/// ```
/// use pert_ui::prelude::*;
/// use pert_ui::testing::*;
///
/// /// The settled semantics of a component.
/// fn semantics_of(driver: &mut Driver<'_>, id: &WidgetId) -> Result<Semantics, TestError> {
///     driver.settle()?;
///     driver.semantics(id)
/// }
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
/// let clicks = ui.signal(0);
/// let counter = clicks.clone();
/// let button = ui.mount_tree(Element::new("add",
///     Button::new("Add", move |_| counter.update(|n| *n += 1))))?;
/// let mut driver = Driver::new(&mut ui);
/// driver.settle()?;
/// driver.click(&button)?;
/// assert_eq!(clicks.get()?, 1);
/// assert!(matches!(semantics_of(&mut driver, &button)?.role, SemanticRole::Button));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A test that stands in for a host calls the
/// [`HostIntegration`](host::HostIntegration) methods through the same glob:
///
/// ```
/// use pert_ui::prelude::*;
/// use pert_ui::testing::*;
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
/// ui.mount_tree(Element::new("name", TextField::new(ui.signal(String::new()))))?;
/// let main = ui.primary_window();
/// ui.select_window(&main)?;
/// // Nothing asked for a native window.
/// assert!(ui.take_presentation_requests()?.is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
#[cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
#[cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
#[cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
pub mod testing {
    #[doc(no_inline)]
    pub use pert_ui_runtime::testing::HostIntegration;
    // Each name of `pert_ui_runtime::testing`, listed so the page shows them;
    // pert-ui-diagnostics' closure check keeps the two lists equal.
    #[doc(inline)]
    pub use pert_ui_runtime::testing::{AddressChild, Driver, QuietReport, TestError};
    #[doc(no_inline)]
    pub use pert_ui_runtime::testing::{
        Capture, CaptureError, CaptureRequest, CaptureTarget, ClickCounter, CommandKeyInput,
        ComponentDetails, ComponentInspection, Depth, Detail, DrawError, FlushReport,
        FocusDirection, FrameImage, InputOutcome, InspectValue, InspectionQuery, InspectionReply,
        KeyAction, Magnification, NativeWindowId, Pending, PointerButton, PointerEvent,
        PointerKind, QueryResult, Rect, RevealRefusal, Selector, SemanticAction, SemanticItem,
        SemanticRole, SemanticValue, Semantics, ValidationCode, ValidationIssue, ValidationMarker,
        ValidationReport, Viewport, WaitError,
    };

    #[cfg(feature = "native")]
    pub use crate::app::HeadlessApp;
    #[cfg(feature = "cpu")]
    #[doc(no_inline)]
    pub use crate::cpu::{CpuRenderError, CpuRenderer};
}

/// Driving a [`Runtime`] in place of `App`, for a custom host:
/// [`HostIntegration`](host::HostIntegration) and the names a host writes,
/// for `use pert_ui::host::*`.
///
/// A *host* owns the event loop around a runtime. It supplies the viewport
/// from its surface, translates native input, flushes, paints and presents
/// frames, completes presentation requests, serves captures and drains
/// shutdown errors. `App` is the host most applications use, and an
/// application under `App` never imports this module. A custom host, built
/// on another window system or on none, imports it and calls the
/// [`HostIntegration`](host::HostIntegration) methods on its [`Runtime`],
/// together with the inherent methods a host shares with tests
/// ([`set_viewport`](Runtime::set_viewport),
/// [`pointer_input`](Runtime::pointer_input),
/// [`command_key_input`](Runtime::command_key_input),
/// [`text_input`](Runtime::text_input), [`flush`](Runtime::flush),
/// [`paint`](Runtime::paint), [`shutdown`](Runtime::shutdown) and others the
/// trait's page lists). `App` drives the runtime through this same trait, so
/// whatever it does, a custom host can do.
///
/// A headless test stands in for a host when it selects a native window or
/// takes presentation requests; it reaches the trait through [`testing`]. The
/// module also re-exports what a host writes while it drives the runtime:
/// input events and key entry, the viewport and its physical extent, the
/// scene and its drawing error, presentation requests and their statuses,
/// capture jobs and frames, and the clipboard service. Each name stays at the crate root too,
/// except [`HostIntegration`](host::HostIntegration), which is reached
/// through this module and [`testing`]. The re-exports are
/// `#[doc(no_inline)]`: each item is documented once, at the crate root.
///
/// # A host without a window system
///
/// A complete host loop that needs no window or GPU. It installs wake
/// delivery, supplies a viewport, runs one turn of its loop (flush, complete
/// the presentation requests, paint, read the next deadline), translates a
/// key and a click, and drains the shutdown errors before it drops the
/// runtime.
///
/// ```
/// use pert_ui::host::*;
/// use pert_ui::prelude::*;
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use std::time::Duration;
///
/// /// One turn of the loop: flush every window, answer the presentation
/// /// requests, present a frame when one is due and return when to wake.
/// fn turn(
///     ui: &mut Runtime,
///     now: Duration,
///     presented: &mut usize,
/// ) -> Result<Option<Duration>, Box<dyn std::error::Error>> {
///     ui.flush_windows(now)?;
///     for request in ui.take_presentation_requests()? {
///         let status = match request.action() {
///             // This host has no native windows to give a detached view.
///             PresentationAction::Detach { .. } => PresentationStatus::Failed(
///                 WindowError::Creation("this host has no windows".into()),
///             ),
///             _ => PresentationStatus::Failed(WindowError::Cancelled),
///         };
///         request.complete(status)?;
///     }
///     if ui.needs_paint() {
///         let frame: DisplayList = ui.paint()?;
///         // "Presents" the frame: a real host hands it to its renderer.
///         *presented += frame.len();
///     }
///     // A host sleeps until this deadline or until a wake arrives.
///     Ok(ui.application_deadline())
/// }
///
/// let mut ui = Runtime::new();
/// // 1. Wake delivery, before any work: a producer's wake sets the flag
/// //    the loop waits on and reports that it was delivered.
/// let woken = Arc::new(AtomicBool::new(false));
/// let flag = Arc::clone(&woken);
/// ui.set_wake_delivery(move || {
///     flag.store(true, Ordering::Release);
///     true
/// });
/// // 2. The viewport, from the physical surface size and the scale factor
/// //    a window system reports.
/// ui.set_viewport(Viewport::from_physical(PhysicalExtent::new(640, 400), 2.0)?)?;
///
/// // 3. Content: two buttons and a view that asks for its own window.
/// let saves = ui.signal(0);
/// let opens = ui.signal(0);
/// let (save_count, open_count) = (saves.clone(), opens.clone());
/// let root = ui.mount_tree(Element::new("main", Column::new([
///     Element::new("save", Button::new("Save", move |_| save_count.update(|n| *n += 1))),
///     Element::new("open", Button::new("Open", move |_| open_count.update(|n| *n += 1))),
///     Element::new("notes", View::new(Element::new("text", Label::new("Notes")))),
/// ])))?;
/// let notes = ui.find(&root, &["notes"])?.ok_or("mounted")?;
/// let detach = ui.request_detach(&notes, NativeWindowOptions::new("Notes")?)?;
///
/// // 4. One turn: the refused detach, a presented frame and the next deadline.
/// let mut presented = 0;
/// let mut now = Duration::ZERO;
/// let _wake_at = turn(&mut ui, now, &mut presented)?;
/// assert!(matches!(detach.status(), PresentationStatus::Failed(WindowError::Creation(_))));
/// assert!(presented > 0);
///
/// // 5. A native key: Space on the focused button. Text goes on only when
/// //    the dispatch says so.
/// let save = ui.find(&root, &["save"])?.ok_or("mounted")?;
/// ui.focus(&save)?;
/// for action in [KeyAction::Press, KeyAction::Release] {
///     let key = CommandKeyInput::new(KeyCode::Space, action, CommandKey::Named(KeyCode::Space));
///     let dispatch: KeyDispatch = ui.command_key_input(key)?;
///     if dispatch.deliver_text && action == KeyAction::Press {
///         ui.text_input(" ")?;
///     }
/// }
/// now += Duration::from_millis(16);
/// turn(&mut ui, now, &mut presented)?;
/// assert_eq!(saves.get()?, 1);
///
/// // 6. A native click at the second button's centre, then the cursor a
/// //    host shows there.
/// let open = ui.find(&root, &["open"])?.ok_or("mounted")?;
/// let bounds = ui.bounds(&open)?.ok_or("the button is laid out")?;
/// let centre = Point::new(bounds.x() + bounds.width() / 2.0, bounds.y() + bounds.height() / 2.0)?;
/// ui.pointer_input(PointerEvent::new(centre, PointerKind::Move))?;
/// let _cursor: CursorIcon = ui.cursor_icon();
/// ui.pointer_input(PointerEvent::new(centre, PointerKind::Press(PointerButton::Primary)))?;
/// ui.pointer_input(PointerEvent::new(centre, PointerKind::Release(PointerButton::Primary)))?;
/// now += Duration::from_millis(16);
/// turn(&mut ui, now, &mut presented)?;
/// assert_eq!(opens.get()?, 1);
///
/// // 7. Shut down and drain the cleanup errors before dropping the runtime.
/// ui.shutdown();
/// assert!(ui.take_shutdown_errors().is_empty());
/// # let _ = woken.load(Ordering::Acquire);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// The host's other duties follow the same pattern:
///
/// - IME: after each turn, [`text_input_state`](host::HostIntegration::text_input_state)
///   names the focused text recipient and where its composition is shown;
///   the host sends composition with
///   [`text_preedit`](host::HostIntegration::text_preedit) and committed
///   text with [`text_input`](Runtime::text_input).
/// - Window deactivation: [`clear_focus`](host::HostIntegration::clear_focus)
///   releases held keys and ends focused edits.
/// - Captures: a host that renders declares it with
///   [`set_capture_support`](Runtime::set_capture_support), then renders each
///   of [`pending_captures`](host::HostIntegration::pending_captures) from
///   [`capture_scene`](host::HostIntegration::capture_scene) and answers it
///   with [`complete_capture`](host::HostIntegration::complete_capture). With
///   the `cpu` feature, `cpu::CpuRenderer::serve_captures` does all three.
/// - The clipboard: [`set_clipboard`](host::HostIntegration::set_clipboard)
///   installs the host's [`Clipboard`] service.
/// - Several native windows: the host selects the window an event belongs
///   to with [`select_window`](host::HostIntegration::select_window) and
///   flushes every window together with
///   [`flush_windows`](host::HostIntegration::flush_windows).
///
/// [contracts.md's custom
/// hosts](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#custom-hosts)
/// states every obligation a host takes on.
///
/// # What application code sees
///
/// The prelude and the crate root do not bring the trait into scope, so an
/// application cannot call a host method by accident:
///
/// ```compile_fail
/// use pert_ui::prelude::*;
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// ui.flush_windows(Duration::ZERO)?; // `HostIntegration` is not in scope
/// # Ok::<(), RuntimeError>(())
/// ```
///
/// ```compile_fail
/// use pert_ui::*;
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// ui.flush_windows(Duration::ZERO)?; // `HostIntegration` is not in scope
/// # Ok::<(), RuntimeError>(())
/// ```
///
/// # The names a host writes
///
/// This glob alone builds what translating native events and producing
/// frames writes:
///
/// ```
/// use pert_ui::host::*;
///
/// fn vocabulary() -> Result<(), Box<dyn std::error::Error>> {
///     let viewport = Viewport::from_physical(PhysicalExtent::new(800, 600), 1.5)?;
///     let point = Point::new(4.0, 8.0)?;
///     let kinds = [
///         PointerKind::Move,
///         PointerKind::Leave,
///         PointerKind::Press(PointerButton::Primary),
///         PointerKind::Release(PointerButton::Secondary),
///         PointerKind::Scroll(Point::new(0.0, -1.0)?),
///         PointerKind::Pinch(Magnification::new(1.25)?),
///     ];
///     let events: Vec<PointerEvent> =
///         kinds.into_iter().map(|kind| PointerEvent::new(point, kind)).collect();
///     let a = CommandKey::Character('a');
///     let mut key = CommandKeyInput::new(KeyCode::KeyA, KeyAction::Press, a);
///     key.has_text = true;
///     let _modifiers = Modifiers::default();
///     let preedit = TextPreedit::new("å", Some(0..2))?;
///     let extent = LimitError::SurfaceExtent { width: 9000, height: 9000, max: 8192 };
///     let too_large = CaptureError::Limit(extent);
///     let _ = (viewport, events, key, preedit, too_large);
///     Ok(())
/// }
///
/// fn after_paint(result: Result<DisplayList, DrawError>) -> Option<DisplayList> {
///     match result {
///         Ok(frame) => Some(frame),
///         // Flush first, then paint again.
///         Err(DrawError::PendingUpdates) => None,
///         Err(_) => None,
///     }
/// }
///
/// fn describe(request: &PresentationRequest) -> &'static str {
///     match request.action() {
///         PresentationAction::Detach { .. } => "create a window",
///         PresentationAction::Redock(_) => "return a view",
///         PresentationAction::Activate(_) => "raise a window",
///         PresentationAction::Close(_) => "close a window",
///         _ => "other",
///     }
/// }
/// # vocabulary().unwrap();
/// # let _ = (after_paint, describe);
/// ```
pub mod host {
    #[doc(inline)]
    pub use pert_ui_runtime::host::HostIntegration;
    #[doc(no_inline)]
    pub use pert_ui_runtime::host::{
        CaptureError, CaptureJob, Clipboard, ClipboardError, CommandKey, CommandKeyInput,
        CursorIcon, DisplayList, DrawError, FlushReport, FrameImage, Invalidation, KeyAction,
        KeyCode, KeyDispatch, KeyboardState, LimitError, Magnification, Modifiers, NativeWindowId,
        NativeWindowOptions, PhysicalExtent, Point, PointerButton, PointerEvent, PointerKind,
        PreparedDetach, PresentationAction, PresentationRate, PresentationRequest,
        PresentationStatus, Runtime, RuntimeError, TextInputState, TextPreedit, Viewport, WidgetId,
        WindowError,
    };
}

/// Authoritative values, for `use pert_ui::parameter::*`: parameters,
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
    pub use pert_ui_runtime::parameter::*;
}

/// Commands, shortcuts and the command palette, for
/// `use pert_ui::commands::*`.
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
    pub use pert_ui_runtime::commands::*;
}

/// Windows inside a view and native windows, regions, splits and tool
/// windows, for `use pert_ui::windowing::*`.
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
    pub use pert_ui_runtime::windowing::*;
}

/// Inspection, capture, profiling and validation, for an application's own
/// tooling and tests, for `use pert_ui::inspection::*`.
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
    pub use pert_ui_runtime::inspection::*;
}

/// Data from other threads, for `use pert_ui::live::*`: lossy live
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
    pub use pert_ui_runtime::live::*;
}
