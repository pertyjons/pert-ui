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
pub mod svg;
// Placement: every widget, including a feature-gated one, is at the crate root;
// a feature module holds its loaders, rasterizers and their errors.
#[cfg(feature = "native")]
pub use app::{App, AppError, AppShutdownError, ErrorAction, NativeCloseAction};
#[cfg(feature = "svg")]
pub use svg::component::Svg;

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
///   payload or public field it constructs.
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
///   root and [`IconSource`] in [`component`].
/// - The [`DescribeContext`] an [`Each`], [`Dynamic`] or [`Show`] closure
///   receives is inferred; code that names it imports it from the crate root.
/// - [`TextEditError`], which `on_binding_refusal` reports, and the other
///   error types stay at the crate root, where code names them to match a
///   failure; `?` converts the leaf errors [`RuntimeError`] wraps into it.
///   Asset-loading errors such as `svg::SvgError` stay in their feature
///   modules.
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
/// | Implementing [`Component`]: contexts, drawing, input events, drag and drop, receipts, editor sessions and the embedded draft editor | `use pert_ui::component::*` ([`component`]) |
/// | A headless test: the driver, the viewport, key and pointer input, queries, captures, the CPU renderer and the headless application | `use pert_ui::testing::*` ([`testing`]) |
/// | Authoritative values: parameters, edit gestures, hosts, external authority, and [`Slider`], [`Knob`] and [`InlineSlider`] | `use pert_ui::parameter::*` ([`parameter`]) |
/// | Commands beyond a shortcut's chord and scope, held input and the command palette | `use pert_ui::commands::*` ([`commands`]) |
/// | Windows inside a view, native windows, regions, splits and tool windows | `use pert_ui::windowing::*` ([`windowing`]) |
/// | Inspection, capture, profiling and validation | `use pert_ui::inspection::*` ([`inspection`]) |
/// | Data from other threads: live series, mailboxes, [`LevelMeter`] and [`Trace`] | `use pert_ui::live::*` ([`live`]) |
/// | Theme, UI settings, fonts and icons, strings and locale, the workspace document, other errors, [`DescribeContext`], [`PhysicalExtent`] and [`VirtualAxis`] | by name from the crate root, such as `use pert_ui::{ThemeMode, Workspace};` |
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
/// transport stays at the crate root. The re-exports are `#[doc(no_inline)]`: each item is documented once, at the
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
/// presses keys and chords, settles the runtime after each step and finds a
/// component's semantics; [`TestError`](testing::TestError) names what a step
/// could not do. Neither needs a window or a GPU. The module also re-exports
/// what a headless test writes besides the driver: the [`Viewport`] every
/// harness sets; [`KeyAction`], [`FocusDirection`], [`PointerEvent`],
/// [`PointerKind`], [`PointerButton`] and [`CommandKeyInput`] for input by
/// hand, with the [`InputOutcome`] it returns; [`SemanticValue`] and
/// [`ClickCounter`]; [`InspectionQuery`], [`Selector`] and [`Detail`] for
/// [`Runtime::query`]; and [`CaptureRequest`], [`CaptureTarget`] and
/// [`FrameImage`] for a capture.
/// With the `cpu` feature it adds [`CpuRenderer`], which serves captures
/// without a GPU, and with the `native` feature, which `desktop` and
/// `desktop-cpu` enable, [`HeadlessApp`], an [`App`] started without a window
/// by [`App::headless`] so its content and its close policy can be tested.
/// The re-exported names stay at the crate root too.
///
/// The [testing chapter](crate::guide::testing) of the guide walks through a
/// test.
///
/// ```
/// use pert_ui::prelude::*;
/// use pert_ui::testing::*;
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
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
#[cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
#[cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
#[cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
pub mod testing {
    #[doc(no_inline)]
    pub use pert_ui_runtime::testing::{
        CaptureRequest, CaptureTarget, ClickCounter, CommandKeyInput, Detail, FocusDirection,
        FrameImage, InputOutcome, InspectionQuery, KeyAction, PointerButton, PointerEvent,
        PointerKind, Selector, SemanticValue, Viewport,
    };
    #[doc(inline)]
    pub use pert_ui_runtime::testing::{Driver, TestError};

    #[cfg(feature = "native")]
    pub use crate::app::HeadlessApp;
    #[cfg(feature = "cpu")]
    #[doc(no_inline)]
    pub use crate::cpu::CpuRenderer;
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
/// [`PresentationAction`], [`PreparedDetach`], [`PresentationRate`]) stays at
/// the crate root, and the error payload of `WindowError::Drawing` is
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
/// the [`Viewport`] of a [`WindowInspection`]. A host's [`CaptureJob`] stays
/// at the crate root, the error payload of `CaptureError::Drawing` is
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
