//! Public standalone application boundary. Backend types remain private.
use crate::{DrawError, Runtime, RuntimeError};

pub(crate) mod close;
mod headless;
pub use headless::HeadlessApp;

/// Failure starting or running a native application; project data remains host-owned.
///
/// Backend details are diagnostic strings, not third-party types in the public API.
/// Direct conversion of a [`RuntimeError`] has one form here, [`Self::Runtime`]: a
/// [`DrawError::Runtime`] converts to it, and so does every leaf error that
/// converts into `RuntimeError`, including an application's own failure wrapped
/// in [`ApplicationError`](crate::ApplicationError). `?` therefore works on all of
/// them in the initialization closure.
///
/// `From` and `?` accept exactly these errors: [`DrawError`], [`RuntimeError`],
/// [`PathError`](crate::PathError), and the leaf errors `RuntimeError` wraps,
/// [`GeometryError`](crate::GeometryError), [`LayoutError`](crate::LayoutError),
/// [`ChoiceError`](crate::ChoiceError), [`BindingError`](crate::BindingError),
/// [`EditError`](crate::EditError), [`DeliveryError`](crate::DeliveryError),
/// [`ApplicationError`](crate::ApplicationError), [`TextError`](crate::TextError),
/// [`TextEditError`](crate::TextEditError), [`FontError`](crate::FontError),
/// [`ClipboardError`](crate::ClipboardError),
/// [`NumberInputError`](crate::NumberInputError),
/// [`ParameterError`](crate::ParameterError),
/// [`ParameterIdError`](crate::ParameterIdError),
/// [`NormalizedValueError`](crate::NormalizedValueError) and
/// [`LiveError`](crate::LiveError). Any other error is wrapped first as an
/// [`ApplicationError`](crate::ApplicationError): a `Send + Sync` error such
/// as `std::io::Error` with [`ApplicationError::new`](crate::ApplicationError::new),
/// and a `Box<dyn Error + Send + Sync>` with `.map_err(ApplicationError::from)?`.
/// An error that is not `Send + Sync` cannot be carried; wrap its message instead, as
/// [`ApplicationError::new`](crate::ApplicationError::new) describes. The compiler does
/// not list these types when a conversion fails, because the conversion is one
/// blanket implementation that it is told not to recommend; it reports the
/// missing `From` for `AppError` instead.
///
/// Unlike the library's other errors, `AppError` is neither `Clone` nor
/// `PartialEq`: [`Self::ErrorReporting`] keeps the `std::io::Error` of a failed
/// report as its typed source. Match on the variants instead.
///
/// ```
/// use pert_ui::{AppError, ApplicationError, ChoiceError, RuntimeError};
/// let parsed: Result<u8, _> = "x".parse::<u8>();
/// let error = AppError::from(ApplicationError::new(parsed.unwrap_err()));
/// assert!(matches!(error, AppError::Runtime(RuntimeError::Application(_))));
/// let error = AppError::from(ChoiceError::DuplicateValue { first: 0, second: 1 });
/// assert!(matches!(
///     error,
///     AppError::Runtime(RuntimeError::Choice(ChoiceError::DuplicateValue { .. }))
/// ));
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AppError {
    /// A title must contain a non-whitespace character.
    #[error("application title must not be empty")]
    InvalidTitle,
    /// Creation or dispatch of the native event loop failed.
    #[error("native event loop failed: {0}")]
    EventLoop(String),
    /// Native window creation failed.
    #[error("window creation failed: {0}")]
    Window(String),
    /// No compatible hardware GPU/surface exists on the supported backend.
    #[error("unsupported graphics configuration: {0}")]
    UnsupportedGraphics(String),
    /// Device creation or GPU validation failed.
    #[error("graphics operation failed: {0}")]
    Graphics(String),
    /// A surface, capture or scene exceeds a documented budget or a device limit.
    /// A window resized beyond the device's largest surface, or beyond the
    /// software frame budget with software rendering, does not end the
    /// application: it stays unpresented until a smaller size, and the error goes
    /// to [`App::on_error`]. The renderer's other limits end it.
    #[error(transparent)]
    Limit(crate::LimitError),
    /// Surface acquisition failed after the bounded recovery allowance.
    #[error("presentation surface failed: {0}")]
    Surface(String),
    /// The device was lost again after one recovery attempt.
    #[error("graphics device recovery failed: {0}")]
    DeviceLost(String),
    /// A component, callback or the initialization closure failed. This includes
    /// an application's own failure, [`RuntimeError::Application`], to which the
    /// facade adds no retry policy of its own beyond the component recovery below.
    #[error(transparent)]
    Runtime(RuntimeError),
    /// A component could not produce valid drawing commands, or the renderer
    /// refused what it drew, such as vector geometry beyond the rasterizer's
    /// limits. Like [`AppError::Runtime`], this goes through [`App::on_error`].
    /// Never holds [`DrawError::Runtime`]; that is [`Self::Runtime`].
    #[error(transparent)]
    Drawing(DrawError),
    /// Runtime cleanup failed while the application was closing. Terminal
    /// cleanup bypasses [`App::on_error`]: the runtime is already closed and
    /// cannot retry. The aggregate retains all typed causes and the preceding
    /// failure, when present; see [`AppShutdownError`].
    #[error(transparent)]
    Shutdown(AppShutdownError),
    /// Writing the default report of a component failure to standard error
    /// failed. The failure is kept whole; the write error is the source.
    #[error("could not report a failure to standard error: {failure}")]
    ErrorReporting {
        /// The failure whose report could not be written.
        failure: Box<AppError>,
        /// Why standard error could not be written.
        #[source]
        io: std::io::Error,
    },
    /// Writing the default report of a native window failure to standard error
    /// failed. The failure is kept whole; the write error is the source.
    #[error("could not report a window failure to standard error: {failure}")]
    WindowErrorReporting {
        /// The window failure whose report could not be written.
        failure: crate::WindowError,
        /// Why standard error could not be written.
        #[source]
        io: std::io::Error,
    },
}

/// Owned terminal failures returned by [`App::run`] through [`AppError::Shutdown`].
///
/// The preceding app failure is retained alongside a nonempty list of cleanup
/// causes in
/// [`HostIntegration::take_shutdown_errors`](crate::host::HostIntegration::take_shutdown_errors)
/// order. No global chronology
/// across windows or error kinds is implied. There is no recovery callback or
/// retry after shutdown. The error source is the preceding failure, or the first
/// cleanup cause on an otherwise successful close; inspect [`Self::errors`] for
/// every additional cause. Display supplies the aggregate context without
/// repeating its source's message. This type does not borrow the closed runtime.
#[derive(Debug)]
pub struct AppShutdownError {
    failure: Option<Box<AppError>>,
    errors: Vec<RuntimeError>,
}
impl AppShutdownError {
    pub(crate) fn new(
        failure: Option<AppError>,
        first: RuntimeError,
        rest: impl Iterator<Item = RuntimeError>,
    ) -> Self {
        let errors = std::iter::once(first).chain(rest).collect();
        Self {
            failure: failure.map(Box::new),
            errors,
        }
    }
    /// The failure that would have been returned without cleanup failures,
    /// absent for an otherwise successful close. Event-loop dispatch failure
    /// precedes runner/recovery failures, as [`App::run`] documents.
    #[must_use]
    pub fn failure(&self) -> Option<&AppError> {
        self.failure.as_deref()
    }
    /// Nonempty owned cleanup causes in runtime report order. This does not
    /// establish cross-window or cross-kind chronological ordering.
    #[must_use]
    pub fn errors(&self) -> &[RuntimeError] {
        &self.errors
    }
    /// Takes ownership of the preceding failure and all cleanup causes, for
    /// reporting or retention independently of the closed application.
    #[must_use]
    pub fn into_parts(self) -> (Option<AppError>, Vec<RuntimeError>) {
        (self.failure.map(|failure| *failure), self.errors)
    }
}
impl std::fmt::Display for AppShutdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("application shutdown cleanup failed")
    }
}
impl std::error::Error for AppShutdownError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Some(failure) = self.failure() {
            Some(failure)
        } else {
            self.errors
                .first()
                .map(|error| error as &dyn std::error::Error)
        }
    }
}

/// Converts every error [`DrawError`] converts from: [`DrawError`] itself,
/// [`RuntimeError`] and every leaf error `RuntimeError` wraps, such as
/// [`GeometryError`](crate::GeometryError). A runtime failure keeps one form:
/// [`DrawError::Runtime`] and a leaf both become [`AppError::Runtime`], and any
/// other drawing failure becomes [`AppError::Drawing`]. Deriving the set from
/// `DrawError` keeps the list of leaf errors in one place, the runtime.
///
/// `#[diagnostic::do_not_recommend]` keeps rustc from reporting a failed
/// conversion as a missing `From` for `DrawError`; it names `AppError`, and
/// [`AppError`]'s own documentation lists the accepted errors.
#[diagnostic::do_not_recommend]
impl<E> From<E> for AppError
where
    DrawError: From<E>,
{
    fn from(error: E) -> Self {
        match DrawError::from(error) {
            DrawError::Runtime(error) => Self::Runtime(error),
            error => Self::Drawing(error),
        }
    }
}

/// Host decision after a component update, input or drawing failure.
///
/// Native startup and GPU failures retain their separate fatal/bounded recovery
/// rules. This decision never replays an input event or commits an application edit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ErrorAction {
    /// Retry pending presentation after backoff (100, 200, then 400 ms).
    /// A fourth consecutive failure exits even if the callback requests retry.
    /// A successful presentation resets the allowance. Failed events are not replayed.
    #[default]
    Retry,
    /// Close the window and return the original error from [`App::run`].
    Exit,
}

/// Host decision on closing a native window, evaluated on the UI thread before
/// destroying it; the native counterpart of
/// [`WindowCloseAction`](crate::WindowCloseAction), with the same `Close` and
/// `KeepOpen` vocabulary.
///
/// A deferred decision does not add a polling timer. Set the signal from a normal
/// UI event (for example a confirmation button); `None` keeps every window alive.
#[derive(Debug)]
pub enum NativeCloseAction {
    /// Apply the window close policy; the primary window shuts down the application.
    Close,
    /// Keep every window available and abandon this close attempt.
    KeepOpen,
    /// Await `Some(WindowCloseAction::Close)` to close or
    /// `Some(WindowCloseAction::KeepOpen)` to keep open; `None` keeps waiting.
    /// The signal must belong to this runtime. Repeated native close requests
    /// reuse the pending decision.
    Defer(crate::ReadSignal<Option<crate::WindowCloseAction>>),
}
type CloseHandler<S> = Box<
    dyn FnMut(
        &mut Runtime,
        &mut S,
        &crate::NativeWindowId,
    ) -> Result<NativeCloseAction, RuntimeError>,
>;
type WindowErrorHandler<S> = Box<dyn FnMut(&mut Runtime, &mut S, &crate::WindowError)>;
type ErrorHandler<S> = Box<dyn FnMut(&mut Runtime, &mut S, &AppError) -> ErrorAction>;
type Initialize<S> = Box<dyn FnOnce(&mut Runtime) -> Result<S, AppError>>;

/// A standalone desktop application with independently scheduled native windows with automatic retained scheduling.
///
/// Requires the default `desktop` feature, or `desktop-cpu`, and a main-thread
/// call to `run`. With `desktop-cpu` alone every window presents CPU frames, as
/// `App::software_rendering` chooses with `desktop` and `cpu`, and wgpu is
/// not compiled. When both features are enabled, for example because another
/// dependency enables `desktop` through Cargo's feature unification, the GPU
/// renderer is present and is the default again; `software_rendering` then
/// chooses CPU frames.
/// Defaults are a 960×640 logical-pixel resizable window, a neutral background,
/// hardware rendering and sRGB presentation. Linux uses Wayland/Vulkan, macOS
/// Metal and Windows DX12. An idle app waits for events; no redraw timer is added.
/// Provides retained layout, text fields, buttons, pointer/keyboard routing and
/// native committed text/IME with candidate positioning. Clipboard operations are
/// UI-thread services; on Linux the backend tries the Wayland data-control
/// protocol and falls back to X11, which reaches GNOME's clipboard through
/// XWayland. Unsupported clipboard operations return typed errors.
/// Native motion retains only the latest
/// position until the next frame or button/key/wheel/leave/geometry boundary.
/// Button transitions stay ordered, with the final position delivered before
/// release. Focus loss, occlusion and zero-size suspension cancel active input.
/// Motion coalescing adds no timer and does not affect direct runtime input.
/// Component failures are reported to stderr, with the failing component's
/// key path from [`Runtime::last_failure`], and get three delayed presentation
/// retries by default; see [`Self::on_error`] for cleanup and host policy. GPU and
/// native startup failures use their separate recovery rules. A successful frame
/// resets the component allowance; repeated failure returns the last error.
///
/// ```no_run
/// use pert_ui::{App, AppError, Color, Component, DrawError, Element, Painter, Rect,
///               RuntimeError, UpdateContext};
/// struct Example;
/// impl Component for Example {
///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
///     fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
///         // The root's allocation is the whole window; inset the panel in it.
///         let size = painter.size();
///         let panel = Rect::new(32.0, 32.0, (size.width() - 64.0).max(0.0),
///             (size.height() - 64.0).max(0.0))?;
///         painter.fill_rect(panel, Color::ACCENT);
///         Ok(())
///     }
/// }
/// fn main() -> Result<(), AppError> {
///     App::new("Example", |ui| {
///         let root = ui.mount_tree(Element::new("example", Example))?;
///         ui.focus(&root)?;
///         Ok(())
///     }).run()
/// }
/// ```
///
/// Open a retained view in a second native window without importing a backend:
///
/// ```no_run
/// use pert_ui::{App, AppError, Element, Label, NativeWindowOptions, View};
/// fn main() -> Result<(), AppError> {
///     App::new("Workspace", |ui| {
///         ui.mount_tree(Element::new("main", Label::new("Main window")))?;
///         let editor = ui.mount_tree(Element::new("editor",
///             View::new(Element::new("content", Label::new("Detached content")))))?;
///         let _receipt = ui.request_detach(&editor, NativeWindowOptions::new("Editor")?)?;
///         // App reports failures through on_window_error (stderr by default).
///         // A UI can additionally bind receipt.status() to show completion.
///         Ok(())
///     }).run()
/// }
/// ```
///
/// # Application state
///
/// The initializer's `Ok` value is the application state `S`: whatever the
/// application's lifecycle callbacks and tests must reach, typically the
/// handles the initializer built (signals, `WidgetId`s, controllers, an `Rc`
/// of the application's own object) or the model itself. [`Self::on_close`],
/// [`Self::on_error`] and [`Self::on_window_error`] receive it by `&mut`
/// after the runtime, one call at a time, so a change one of them makes is
/// there for the next; [`HeadlessApp::state`] hands it to a test. An
/// initializer that returns `Ok(())` has no state, `S = ()`.
///
/// The state lives from the initializer's successful return until the
/// application ends, and is dropped after the runtime has shut down, on
/// every path: its own `Drop` finds components unmounted and parameter
/// edits ended, and a signal it reads there reports
/// [`RuntimeError::Closed`]. It stays on the UI thread and need not be
/// `Send`, `Sync`, `Clone`, `Debug` or `Default`. The library keeps it for
/// the application and never reads, copies, compares or persists it. No
/// callback runs without it: a failing initializer's error returns from
/// [`Self::run`] or [`Self::headless`] without calling any of them.
///
/// ```no_run
/// use pert_ui::{App, AppError, Element, NativeCloseAction, Signal, TextField};
/// /// What the close policy reads.
/// struct Notes {
///     text: Signal<String>,
///     saved: Signal<String>,
/// }
/// fn main() -> Result<(), AppError> {
///     App::new("Notes", |ui| {
///         let text = ui.signal(String::new());
///         ui.mount_tree(Element::new("note", TextField::new(text.clone())))?;
///         Ok(Notes { text, saved: ui.signal(String::new()) })
///     })
///     // Unsaved text keeps the window open.
///     .on_close(|_, notes, _| {
///         Ok(if notes.text.get()? == notes.saved.get()? {
///             NativeCloseAction::Close
///         } else {
///             NativeCloseAction::KeepOpen
///         })
///     })
///     .run()
/// }
/// ```
#[must_use = "call run to open the application, or headless in a test"]
pub struct App<S = ()> {
    pub(crate) host: Host,
    initialize: Initialize<S>,
    handlers: Handlers<S>,
}

/// What every form of an application shares: before it starts ([`App`]),
/// while the native runner owns it ([`Running`]) and in a test
/// ([`HeadlessApp`]).
pub(crate) struct Host {
    pub(crate) title: String,
    pub(crate) window: MainWindow,
    pub(crate) runtime: Runtime,
    /// Whether windows present CPU frames instead of using the GPU.
    pub(crate) software: bool,
}

/// The lifecycle callbacks an application installed, typed in its state.
struct Handlers<S> {
    error: Option<ErrorHandler<S>>,
    close: Option<CloseHandler<S>>,
    window_error: Option<WindowErrorHandler<S>>,
}

/// The state the initializer returned and the callbacks that receive it.
/// Built only from the initializer's `Ok` value, so no callback can run
/// without state.
pub(crate) struct Bound<S> {
    state: S,
    handlers: Handlers<S>,
}

/// The callbacks of a [`Bound`] with its state type erased, for the native
/// runner. Each method returns `None` when the application installed no
/// callback, and the caller applies the default.
trait Lifecycle {
    fn error(&mut self, runtime: &mut Runtime, error: &AppError) -> Option<ErrorAction>;
    fn close(
        &mut self,
        runtime: &mut Runtime,
        window: &crate::NativeWindowId,
    ) -> Option<Result<NativeCloseAction, RuntimeError>>;
    fn window_error(&mut self, runtime: &mut Runtime, error: &crate::WindowError) -> Option<()>;
}

impl<S> Lifecycle for Bound<S> {
    fn error(&mut self, runtime: &mut Runtime, error: &AppError) -> Option<ErrorAction> {
        let handler = self.handlers.error.as_mut()?;
        Some(handler(runtime, &mut self.state, error))
    }
    fn close(
        &mut self,
        runtime: &mut Runtime,
        window: &crate::NativeWindowId,
    ) -> Option<Result<NativeCloseAction, RuntimeError>> {
        let handler = self.handlers.close.as_mut()?;
        Some(handler(runtime, &mut self.state, window))
    }
    fn window_error(&mut self, runtime: &mut Runtime, error: &crate::WindowError) -> Option<()> {
        let handler = self.handlers.window_error.as_mut()?;
        handler(runtime, &mut self.state, error);
        Some(())
    }
}

impl<S> Bound<S> {
    pub(crate) fn state(&self) -> &S {
        &self.state
    }
    pub(crate) fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }
    /// Runs the close policy, [`NativeCloseAction::Close`] without one.
    pub(crate) fn close_policy(
        &mut self,
        runtime: &mut Runtime,
        window: &crate::NativeWindowId,
    ) -> Result<NativeCloseAction, RuntimeError> {
        close_policy(self, runtime, window)
    }
}

/// The close policy's decision, [`NativeCloseAction::Close`] without one.
fn close_policy(
    lifecycle: &mut dyn Lifecycle,
    runtime: &mut Runtime,
    window: &crate::NativeWindowId,
) -> Result<NativeCloseAction, RuntimeError> {
    lifecycle
        .close(runtime, window)
        .unwrap_or(Ok(NativeCloseAction::Close))
}

/// A started application as the native runner owns it, its state erased.
pub(crate) struct Running {
    // Declared before the lifecycle so that a drop without an explicit
    // shutdown drops the runtime, which shuts down in its drop, before the
    // state and the callbacks' captured values.
    pub(crate) host: Host,
    lifecycle: Box<dyn Lifecycle>,
}

impl<S: 'static> From<(Host, Bound<S>)> for Running {
    fn from((host, bound): (Host, Bound<S>)) -> Self {
        Self {
            host,
            lifecycle: Box::new(bound),
        }
    }
}

impl Running {
    pub(crate) fn handle_window_error(
        &mut self,
        error: &crate::WindowError,
    ) -> Result<(), AppError> {
        if self
            .lifecycle
            .window_error(&mut self.host.runtime, error)
            .is_some()
        {
            return Ok(());
        }
        use std::io::Write;
        writeln!(std::io::stderr().lock(), "[pert-ui window error] {error}").map_err(|io| {
            AppError::WindowErrorReporting {
                failure: error.clone(),
                io,
            }
        })
    }
    pub(crate) fn handle_close(
        &mut self,
        window: &crate::NativeWindowId,
    ) -> Result<NativeCloseAction, RuntimeError> {
        close_policy(&mut *self.lifecycle, &mut self.host.runtime, window)
    }

    /// Asks the host what to do about a failure it can recover from. The
    /// default reports to standard error and retries; when that report cannot be
    /// written, the failure comes back inside [`AppError::ErrorReporting`].
    pub(crate) fn handle_error(
        &mut self,
        error: AppError,
    ) -> Result<(ErrorAction, AppError), AppError> {
        if let Some(action) = self.lifecycle.error(&mut self.host.runtime, &error) {
            return Ok((action, error));
        }
        use std::io::Write;
        // The site names the component, so the report says where to look.
        let site = self
            .host
            .runtime
            .last_failure()
            .map(|site| format!(" (in the {site})"))
            .unwrap_or_default();
        match writeln!(
            std::io::stderr().lock(),
            "[pert-ui component error] {error}{site}"
        ) {
            Ok(()) => Ok((ErrorAction::Retry, error)),
            Err(io) => Err(AppError::ErrorReporting {
                failure: Box::new(error),
                io,
            }),
        }
    }
}

/// Shows the title, the main window's geometry and whether presentation is
/// software, not the runtime, the callbacks or the state.
impl<S> std::fmt::Debug for App<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("title", &self.host.title)
            .field("window", &self.host.window)
            .field("software", &self.host.software)
            .finish_non_exhaustive()
    }
}
impl<S: 'static> App<S> {
    /// Defines content once using the public runtime, with no backend configuration.
    ///
    /// Initialization runs on the calling main thread from `run`, after the native
    /// event loop exists and before any window opens, so the runtime has no
    /// viewport yet: it is supplied when the window opens, before the first
    /// flush the application runs. Content laid out by a flush inside the closure
    /// has empty bounds, which development validation does not report.
    /// The closure does not repeat on redraw. Errors are returned from `run`.
    /// A test starts the same application without a window with
    /// [`Self::headless`].
    ///
    /// The closure's `Ok` value becomes the
    /// [application state](Self#application-state) the lifecycle callbacks
    /// receive. Values the application must keep for as long as it runs,
    /// parameter host owners and mailboxes included, go in components or in
    /// that state. `Ok(())` keeps the default `S = ()`. The type is inferred
    /// from the `Ok` value, so a closure that never returns `Ok`, one that
    /// always fails or panics, names it: `App::<()>::new`.
    ///
    /// ```
    /// use pert_ui::{App, AppError, ApplicationError, Element, Label};
    /// # fn main() -> Result<(), AppError> {
    /// // An initializer that returns `Ok(())` has no state: `App` is `App<()>`.
    /// let app: App = App::new("Status", |ui| {
    ///     ui.mount_tree(Element::new("status", Label::new("Ready")))?;
    ///     Ok(())
    /// });
    /// app.headless()?.shutdown()?;
    /// // One that never returns `Ok` names the state type.
    /// let failing = App::<()>::new("Failing", |_| Err(ApplicationError::from("no data").into()));
    /// assert!(failing.headless().is_err());
    /// # Ok(()) }
    /// ```
    pub fn new(
        title: impl Into<String>,
        initialize: impl FnOnce(&mut Runtime) -> Result<S, AppError> + 'static,
    ) -> Self {
        Self {
            host: Host {
                title: title.into(),
                window: MainWindow::default(),
                runtime: Runtime::new(),
                software: false,
            },
            initialize: Box::new(initialize),
            handlers: Handlers {
                error: None,
                close: None,
                window_error: None,
            },
        }
    }

    /// Runs the initializer and binds the callbacks to the state it returns.
    /// On its failure the runtime is shut down, and that failure is returned,
    /// inside [`AppError::Shutdown`] when cleanup failed too; the callbacks'
    /// captured values are dropped after that shutdown.
    pub(crate) fn start(self) -> Result<(Host, Bound<S>), AppError> {
        let Self {
            mut host,
            initialize,
            handlers,
        } = self;
        match initialize(&mut host.runtime) {
            Ok(state) => Ok((host, Bound { state, handlers })),
            Err(error) => crate::platform::finish_runtime(&mut host.runtime, Err(error)),
        }
    }
    /// Sets the main window's initial logical size, before user zoom and native
    /// DPI scaling. Defaults to 960 × 640. Placement remains operating-system
    /// policy, and the window system may clamp the size to the screen or to
    /// [`Self::min_size`] and [`Self::max_size`].
    ///
    /// # Errors
    /// [`RuntimeError::Geometry`] holding
    /// [`GeometryError::Empty`](crate::GeometryError::Empty) for a zero width or
    /// height.
    ///
    /// ```no_run
    /// use pert_ui::{App, AppError, Element, Label, Size};
    /// fn main() -> Result<(), AppError> {
    ///     App::new("Mixer", |ui| {
    ///         ui.mount_tree(Element::new("label", Label::new("Ready")))?;
    ///         Ok(())
    ///     })
    ///     .size(Size::from([1280, 800]))?
    ///     .min_size(Size::from([640, 400]))?
    ///     .run()
    /// }
    /// ```
    pub fn size(mut self, size: crate::Size) -> Result<Self, RuntimeError> {
        self.host.window.size = nonzero(size)?;
        Ok(self)
    }
    /// Sets the smallest logical size the user can resize the main window to.
    /// Unset by default, leaving the limit to the window system.
    ///
    /// # Errors
    /// [`RuntimeError::Geometry`] holding
    /// [`GeometryError::Empty`](crate::GeometryError::Empty) for a zero width or
    /// height, or [`GeometryError::InvertedBounds`](crate::GeometryError::InvertedBounds)
    /// for a dimension larger than a maximum set with [`Self::max_size`].
    pub fn min_size(mut self, size: crate::Size) -> Result<Self, RuntimeError> {
        let size = nonzero(size)?;
        ordered(Some(size), self.host.window.max)?;
        self.host.window.min = Some(size);
        Ok(self)
    }
    /// Sets the largest logical size the user can resize the main window to.
    /// Unset by default.
    ///
    /// # Errors
    /// [`RuntimeError::Geometry`] holding
    /// [`GeometryError::Empty`](crate::GeometryError::Empty) for a zero width or
    /// height, or [`GeometryError::InvertedBounds`](crate::GeometryError::InvertedBounds)
    /// for a dimension smaller than a minimum set with [`Self::min_size`].
    pub fn max_size(mut self, size: crate::Size) -> Result<Self, RuntimeError> {
        let size = nonzero(size)?;
        ordered(self.host.window.min, Some(size))?;
        self.host.window.max = Some(size);
        Ok(self)
    }
    /// Sets whether the user can resize the main window. Defaults to true. The
    /// content still receives size changes the window system makes, such as a
    /// scale change after moving to another monitor.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.host.window.resizable = resizable;
        self
    }

    /// Presents every window with the CPU renderer instead of the GPU.
    ///
    /// Each frame is rendered by [`CpuRenderer`](crate::cpu::CpuRenderer) as an
    /// opaque surface and copied to the window through the platform's software
    /// presentation (Wayland shared memory on Linux). No GPU adapter or device
    /// is created, so the application runs where none is available. Input,
    /// text editing, scale, captures and native windows work as with the GPU;
    /// captures keep the background's alpha as GPU captures do.
    ///
    /// The choice is made once, before [`Self::run`]: there is no automatic
    /// fallback in either direction, and with the GPU renderer compiled in the
    /// GPU path is unchanged when this is not called. Without it
    /// (`desktop-cpu` alone) every window presents CPU frames anyway. A whole frame is rendered on the UI thread for every
    /// presentation, which costs far more than the GPU path; the
    /// [verification notes](https://github.com/pertyjons/pert-ui/blob/main/docs/verification.md#cpu-rendering-cost)
    /// record measured frame times. A window whose frame would exceed
    /// [`FrameImage::MAX_BYTES`](crate::FrameImage::MAX_BYTES) stays
    /// unpresented and reports [`LimitError::FrameSize`](crate::LimitError::FrameSize)
    /// through [`Self::on_error`], as an oversized GPU surface does.
    ///
    /// ```no_run
    /// use pert_ui::{App, AppError, Element, Label};
    /// fn main() -> Result<(), AppError> {
    ///     App::new("Software", |ui| {
    ///         ui.mount_tree(Element::new("label", Label::new("Drawn on the CPU")))?;
    ///         Ok(())
    ///     })
    ///     .software_rendering()
    ///     .run()
    /// }
    /// ```
    #[cfg(feature = "cpu")]
    pub fn software_rendering(mut self) -> Self {
        self.host.software = true;
        self
    }

    /// Installs a UI-thread policy for component input, update and drawing failures,
    /// and for a window resized beyond the device's largest surface
    /// ([`AppError::Limit`]).
    ///
    /// Runs after input cleanup, outside runtime evaluation, so the handler
    /// may repair local signals through `runtime`. It receives the
    /// [application state](Self#application-state) after the runtime. It must
    /// return promptly; no
    /// callback is safe for time-critical producers. The error is borrowed only
    /// for this call. Report or retain its details here: retrying does not
    /// return recovered errors from `run`.
    /// Default writes each failure to standard error, naming the component
    /// [`Runtime::last_failure`] reports, and requests [`ErrorAction::Retry`].
    /// Failure to write that report is fatal. Initialization and native/GPU failures
    /// bypass this callback. Terminal shutdown cleanup failures also bypass it
    /// and return through [`AppError::Shutdown`]. Panics are not caught.
    ///
    /// For a surface limit, [`ErrorAction::Retry`] keeps the window open and
    /// unpresented until the user resizes it to a size the device accepts, with
    /// no timer and no retry count; [`ErrorAction::Exit`] ends the application
    /// with the limit.
    ///
    /// Retries are limited even for a custom handler. Input is canceled on failure
    /// and native editing events are discarded during backoff; close, resize and
    /// visibility events still work. Component focus and local drafts are retained
    /// unless a subsequent native lifecycle event removes focus. Neither the failed
    /// input nor its application action is replayed.
    ///
    /// The state here is a status line the window shows; a failure appears in
    /// it, and the application exits when the line cannot be written:
    ///
    /// ```no_run
    /// use pert_ui::{App, AppError, Element, ErrorAction, Label};
    /// fn main() -> Result<(), AppError> {
    ///     App::new("Example", |ui| {
    ///         let status = ui.signal(String::from("Ready"));
    ///         ui.mount_tree(Element::new("status", Label::new(status.read_only())))?;
    ///         Ok(status)
    ///     })
    ///     .on_error(|_, status, error| match status.set(format!("Failed: {error}")) {
    ///         Ok(()) => ErrorAction::Retry,
    ///         Err(_) => ErrorAction::Exit,
    ///     })
    ///     .run()
    /// }
    /// ```
    pub fn on_error(
        mut self,
        handler: impl FnMut(&mut Runtime, &mut S, &AppError) -> ErrorAction + 'static,
    ) -> Self {
        self.handlers.error = Some(Box::new(handler));
        self
    }

    /// Installs close/save admission for main and detached windows. Defaults to
    /// [`NativeCloseAction::Close`]. The callback may present confirmation in the
    /// invoking window; return [`NativeCloseAction::Defer`] with its decision
    /// signal to preserve all windows while waiting.
    /// Errors abort closing and follow on_error without replaying the request.
    ///
    /// # Close order
    ///
    /// A close request, from the window system or [`Runtime::request_close`],
    /// is admitted in this order, so the policy sees what the user typed:
    ///
    /// 1. The windows an accepted close discards are affected: a detached
    ///    window's own, or every open window for the main window, which ends
    ///    the application.
    /// 2. Each affected window's focus is released through
    ///    [`HostIntegration::clear_focus`](crate::host::HostIntegration::clear_focus), as a
    ///    window deactivation releases it, and the window selected before is
    ///    selected again. Each recipient's
    ///    `Component::focus_lost` runs and its error is queued in its window.
    ///    `clear_focus` also ends the runtime's held keys and held-input
    ///    registrations; for a close the user started, the closing window has
    ///    native focus, so nothing else is held, but a programmatic close of an
    ///    inactive window also ends held input in the active one.
    /// 3. Each focused editor makes the focus-loss save attempt of the
    ///    [draft policy](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#draft-policy),
    ///    which closing does not change: a local draft its rules accept is
    ///    saved, a refused one is kept unsaved, a parameter draft is cancelled
    ///    (parameter authority commits only on an explicit commit), and an
    ///    `EditorSession` reports `FocusLeft` with the outcome. Drafts kept
    ///    elsewhere, such as by a disabled or unfocused field, are untouched;
    ///    the editors' `on_modified` reports them, so the policy can count
    ///    them with a [`FormFields`](crate::FormFields).
    ///
    ///    So the policy sees a saved value only where that editor saves on
    ///    focus loss.
    /// 4. When no window is in component-error backoff, the affected windows
    ///    are flushed: the detached window alone, or every window through
    ///    [`HostIntegration::flush_windows`](crate::host::HostIntegration::flush_windows).
    ///    A failed `focus_lost` in any of them aborts the close: [`Self::on_error`] handles it as it handles an
    ///    error of this callback, and the request is not replayed. That flush
    ///    may also settle notifications in other visible windows, as any
    ///    frame's flush does; a hidden window is not flushed. While any window
    ///    is in backoff, nothing is flushed, so closing never needs a failing
    ///    update to succeed first: bindings the drafts wrote are current, and
    ///    derived values and the queued focus errors wait for the next flush.
    /// 5. This callback runs, with the closing window selected and the
    ///    [application state](Self#application-state).
    /// 6. On a synchronous [`NativeCloseAction::KeepOpen`], in each affected
    ///    window, the component whose focus step 2 released is focused again
    ///    when nothing in that window holds focus after the callback, so a
    ///    field the policy focused keeps focus, and when
    ///    [`Runtime::can_focus`] still admits it: a hidden, unmounted,
    ///    unfocusable (such as disabled) or modal-blocked component leaves
    ///    focus cleared. The affected windows are flushed first, as in step 4,
    ///    so what the callback changed applies before that check. Flushing
    ///    and focusing, which reveals the component, are why nothing is
    ///    restored while a window is in component-error backoff. On
    ///    [`NativeCloseAction::Defer`] and
    ///    [`NativeCloseAction::Close`] nothing is restored; when a deferred
    ///    decision keeps the window open, the policy or its confirmation
    ///    decides where focus goes. Whatever the decision, a component an
    ///    affected window recorded to focus again on reactivation is
    ///    forgotten: the close supersedes it.
    ///
    /// [`HeadlessApp::admit_close`] runs the same steps in a test. The
    /// [type's example](Self#application-state) keeps the window open while
    /// the state holds unsaved text.
    pub fn on_close(
        mut self,
        handler: impl FnMut(
            &mut Runtime,
            &mut S,
            &crate::NativeWindowId,
        ) -> Result<NativeCloseAction, RuntimeError>
        + 'static,
    ) -> Self {
        self.handlers.close = Some(Box::new(handler));
        self
    }
    /// Reports failed asynchronous native presentation, including tab drag failure.
    /// The receipt retains the typed failure independently. Called on the UI
    /// thread after failed preparation/cleanup, with the
    /// [application state](Self#application-state) after the runtime;
    /// existing content remains available and the operation is never
    /// automatically retried.
    ///
    /// The default writes `[pert-ui window error]` and the failure to standard
    /// error; when that report cannot be written, the application ends with
    /// [`AppError::WindowErrorReporting`]. A callback replaces the report.
    ///
    /// ```no_run
    /// use pert_ui::{App, AppError, Element, Label, NativeWindowOptions, View};
    /// /// How many window requests failed so far.
    /// struct Windows {
    ///     failures: u32,
    /// }
    /// fn main() -> Result<(), AppError> {
    ///     App::new("Workspace", |ui| {
    ///         let editor = ui.mount_tree(Element::new("editor",
    ///             View::new(Element::new("content", Label::new("Detached content")))))?;
    ///         let _receipt = ui.request_detach(&editor, NativeWindowOptions::new("Editor")?)?;
    ///         Ok(Windows { failures: 0 })
    ///     })
    ///     .on_window_error(|_, windows, error| {
    ///         windows.failures += 1;
    ///         eprintln!("window failure {}: {error}", windows.failures);
    ///     })
    ///     .run()
    /// }
    /// ```
    pub fn on_window_error(
        mut self,
        handler: impl FnMut(&mut Runtime, &mut S, &crate::WindowError) + 'static,
    ) -> Self {
        self.handlers.window_error = Some(Box::new(handler));
        self
    }

    /// Opens the native window and runs until close or an unrecovered failure.
    ///
    /// Call once per process on its main thread (the platform's event-loop rule).
    /// The native event loop is created first, and the initialization closure
    /// runs only once that has succeeded, so a call that cannot run never builds
    /// the host's content. Closing releases component input/edits through runtime
    /// shutdown. Save/undo and background-service shutdown are host responsibilities;
    /// on_close admits or defers native closing. No library worker or background
    /// executor is started.
    ///
    /// ```no_run
    /// use pert_ui::{App, AppError, Element, Label};
    /// let result = App::new("Example", |ui| {
    ///     ui.mount_tree(Element::new("content", Label::new("Ready")))?;
    ///     Ok(())
    /// }).run();
    /// match result {
    ///     Err(AppError::Shutdown(shutdown)) => {
    ///         let (preceding, cleanup) = shutdown.into_parts();
    ///         if let Some(error) = preceding { eprintln!("{error}"); }
    ///         for error in cleanup { eprintln!("{error}"); }
    ///     }
    ///     Err(error) => eprintln!("{error}"),
    ///     Ok(()) => {}
    /// }
    /// ```
    ///
    /// # Errors
    /// [`AppError::InvalidTitle`] for a title without a non-whitespace character,
    /// checked before anything native starts. [`AppError::EventLoop`] when the
    /// event loop cannot be created, which includes any call after the first in
    /// one process: the platform allows one event loop per process, even after
    /// the first has returned, and the initialization closure does not run.
    /// Otherwise initialization, native startup, rendering and unrecovered
    /// component failures. Closing during backoff returns its pending error.
    /// [`AppError::Shutdown`] retains every queued runtime cleanup failure and
    /// the error that would otherwise have been returned, including initialization
    /// failure after content was built. Terminal cleanup is not retried through
    /// [`Self::on_error`]. With no cleanup failure, the original result is unchanged.
    /// Native event-loop dispatch failure precedes runner/recovery failure;
    /// otherwise the runner failure precedes the current window's pending
    /// recovery failure, then a parked window's, then that of a detached window
    /// closed while it waited to retry. Selection among several parked or closed
    /// windows' recovery failures is unspecified; it is not a cross-window error
    /// history.
    /// Unsupported graphics fail explicitly; no software/legacy fallback is used
    /// unless the application chose `App::software_rendering` (`cpu` feature).
    ///
    /// # Panics
    /// When called on a thread other than the main thread, on every supported
    /// platform: the windowing backend panics while creating the event loop,
    /// before the
    /// initialization closure runs. The panic message may name backend
    /// extension traits that this facade does not expose; there is no supported
    /// way to run an `App` off the main thread.
    pub fn run(self) -> Result<(), AppError> {
        crate::platform::run(self)
    }
}

impl AppError {
    pub(crate) fn from_renderer(error: crate::platform::presenter::RenderError) -> Self {
        match error {
            crate::platform::presenter::RenderError::UnsupportedGraphics(message) => {
                Self::UnsupportedGraphics(message)
            }
            // Content the vector rasterizer refuses, such as geometry over its
            // limits, is the component's failure, not the GPU's.
            crate::platform::presenter::RenderError::Drawing(error) => Self::from(error),
            crate::platform::presenter::RenderError::Limit(limit) => Self::Limit(limit),
            crate::platform::presenter::RenderError::Graphics(message) => Self::Graphics(message),
            crate::platform::presenter::RenderError::Surface(message) => Self::Surface(message),
            // Unconfigured and future variants.
            error => Self::Graphics(error.to_string()),
        }
    }
    /// Whether this failure goes through component recovery, [`App::on_error`]
    /// and its bounded retries, rather than ending the application.
    pub(crate) const fn is_component_failure(&self) -> bool {
        matches!(self, Self::Runtime(_) | Self::Drawing(_))
    }
}

/// Main-window geometry in logical pixels, validated by the [`App`] builders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MainWindow {
    pub(crate) size: crate::Size,
    pub(crate) min: Option<crate::Size>,
    pub(crate) max: Option<crate::Size>,
    pub(crate) resizable: bool,
}
impl Default for MainWindow {
    fn default() -> Self {
        Self {
            size: crate::Size::from([960, 640]),
            min: None,
            max: None,
            resizable: true,
        }
    }
}
fn nonzero(size: crate::Size) -> Result<crate::Size, RuntimeError> {
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return Err(crate::GeometryError::Empty.into());
    }
    Ok(size)
}
fn ordered(min: Option<crate::Size>, max: Option<crate::Size>) -> Result<(), RuntimeError> {
    if let (Some(min), Some(max)) = (min, max)
        && (min.width() > max.width() || min.height() > max.height())
    {
        return Err(crate::GeometryError::InvertedBounds.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeometryError, Size};

    fn app() -> App {
        App::new("Test", |_| Ok(()))
    }
    fn empty(result: Result<App, RuntimeError>) -> bool {
        matches!(result, Err(RuntimeError::Geometry(GeometryError::Empty)))
    }
    fn inverted(result: Result<App, RuntimeError>) -> bool {
        matches!(
            result,
            Err(RuntimeError::Geometry(GeometryError::InvertedBounds))
        )
    }

    #[test]
    fn a_renderer_drawing_error_is_recovered_as_a_component_failure() {
        let error = AppError::from_renderer(crate::platform::presenter::RenderError::Drawing(
            DrawError::InvalidColor,
        ));
        assert!(matches!(error, AppError::Drawing(DrawError::InvalidColor)));
        assert!(error.is_component_failure());
        for fatal in [
            crate::platform::presenter::RenderError::Graphics("device".into()),
            crate::platform::presenter::RenderError::UnsupportedGraphics("adapter".into()),
            crate::platform::presenter::RenderError::Surface("surface".into()),
        ] {
            assert!(!AppError::from_renderer(fatal).is_component_failure());
        }
    }

    #[test]
    fn a_runtime_error_has_one_form_whichever_path_it_takes() {
        let runtime = RuntimeError::Geometry(GeometryError::NonFinite);
        let paths = [
            AppError::from(runtime.clone()),
            AppError::from(DrawError::Runtime(runtime.clone())),
            AppError::from(GeometryError::NonFinite),
            AppError::from_renderer(crate::platform::presenter::RenderError::Drawing(
                DrawError::Runtime(runtime.clone()),
            )),
        ];
        for error in paths {
            assert!(
                matches!(&error, AppError::Runtime(inner) if *inner == runtime),
                "{error:?}"
            );
            // `Runner::fail` recovers every form a component failure can take.
            assert!(error.is_component_failure());
        }
        let binding = AppError::from(crate::BindingError::DuplicateParameter);
        assert!(matches!(
            binding,
            AppError::Runtime(RuntimeError::Binding(
                crate::BindingError::DuplicateParameter
            ))
        ));
        assert!(binding.is_component_failure());
        let application = AppError::from(crate::ApplicationError::new(std::fmt::Error));
        assert!(matches!(
            application,
            AppError::Runtime(RuntimeError::Application(_))
        ));
        assert!(application.is_component_failure());
    }

    #[test]
    fn renderer_limits_stay_typed_and_are_not_component_failures() {
        let limit = crate::LimitError::SurfaceExtent {
            width: 20_000,
            height: 10,
            max: 16_384,
        };
        let error = AppError::from_renderer(crate::platform::presenter::RenderError::Limit(
            limit.clone(),
        ));
        assert!(matches!(&error, AppError::Limit(inner) if *inner == limit));
        assert!(!error.is_component_failure());
        assert_eq!(error.to_string(), limit.to_string());
    }

    #[test]
    fn a_failed_report_keeps_the_failure_and_its_cause_once() {
        use std::error::Error as _;
        let failure = AppError::from(DrawError::InvalidColor);
        let error = AppError::ErrorReporting {
            failure: Box::new(failure),
            io: std::io::Error::other("stderr closed"),
        };
        assert_eq!(
            error.to_string(),
            "could not report a failure to standard error: \
             color channels must be finite and within 0..=1"
        );
        let source = error.source().expect("the write error");
        assert_eq!(source.to_string(), "stderr closed");
        assert!(!error.to_string().contains(&source.to_string()));
    }

    /// Records, when dropped, whether its signal's runtime had shut down.
    struct Observed {
        signal: crate::Signal<i32>,
        closed: std::rc::Rc<std::cell::Cell<Option<bool>>>,
    }
    impl Drop for Observed {
        fn drop(&mut self) {
            self.closed
                .set(Some(matches!(self.signal.get(), Err(RuntimeError::Closed))));
        }
    }

    /// New-behavior test (P11) 5: the native runner's erased owner, dropped
    /// with its runtime still open, drops the runtime first, so the state
    /// and the value a callback captured both find it closed. Named
    /// assertion: both record `Some(true)`. Mutation checked: `lifecycle`
    /// declared before `host` in `Running` (both record `Some(false)`).
    #[test]
    fn the_erased_native_owner_drops_the_state_after_the_runtime() -> Result<(), AppError> {
        let state = std::rc::Rc::new(std::cell::Cell::new(None));
        let captured = std::rc::Rc::new(std::cell::Cell::new(None));
        let records = state.clone();
        let app = App::new("drop", move |ui| {
            Ok(Observed {
                signal: ui.signal(0),
                closed: records,
            })
        });
        let kept = Observed {
            signal: app.host.runtime.signal(0),
            closed: captured.clone(),
        };
        let app = app.on_error(move |_, _, _| {
            let _kept = &kept;
            ErrorAction::Exit
        });
        let running = Running::from(app.start()?);
        assert_eq!((state.get(), captured.get()), (None, None));
        drop(running);
        assert_eq!((state.get(), captured.get()), (Some(true), Some(true)));
        Ok(())
    }

    #[test]
    fn main_window_geometry_defaults_and_validates() {
        assert_eq!(app().host.window, MainWindow::default());
        assert!(empty(app().size(Size::from([0, 480]))));
        assert!(empty(app().min_size(Size::from([640, 0]))));
        let bounded = app()
            .size(Size::from([1280, 800]))
            .and_then(|app| app.min_size(Size::from([640, 400])))
            .and_then(|app| app.max_size(Size::from([1920, 1200])))
            .unwrap()
            .resizable(false);
        assert_eq!(bounded.host.window.size, Size::from([1280, 800]));
        assert_eq!(bounded.host.window.min, Some(Size::from([640, 400])));
        assert_eq!(bounded.host.window.max, Some(Size::from([1920, 1200])));
        assert!(!bounded.host.window.resizable);
        // A minimum above the maximum is refused in either order.
        assert!(inverted(
            app()
                .max_size(Size::from([800, 600]))
                .and_then(|app| app.min_size(Size::from([900, 400])))
        ));
        assert!(inverted(
            app()
                .min_size(Size::from([640, 700]))
                .and_then(|app| app.max_size(Size::from([800, 600])))
        ));
    }
}
