//! Public standalone application boundary. Backend types remain private.
use crate::{DrawError, Runtime, RuntimeError};

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
/// An error that is not `Send` cannot be carried; wrap its message instead, as
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
/// causes in [`Runtime::take_shutdown_errors`] order. No global chronology
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
type CloseHandler =
    Box<dyn FnMut(&mut Runtime, &crate::NativeWindowId) -> Result<NativeCloseAction, RuntimeError>>;

type WindowErrorHandler = Box<dyn FnMut(&mut Runtime, &crate::WindowError)>;

type ErrorHandler = Box<dyn FnMut(&mut Runtime, &AppError) -> ErrorAction>;
type Initialize = Box<dyn FnOnce(&mut Runtime) -> Result<(), AppError>>;

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
/// use pert_ui::{App, AppError, Color, Component, DrawError, Painter, Rect,
///               RuntimeError, UpdateContext};
/// struct Example;
/// impl Component for Example {
///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
///     fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
///         painter.fill_rect(Rect::new(32.0, 32.0, 240.0, 120.0)?, Color::ACCENT);
///         Ok(())
///     }
/// }
/// fn main() -> Result<(), AppError> {
///     App::new("Example", |ui| {
///         let root = ui.mount("example", Example)?;
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
#[must_use = "call run to open the application"]
pub struct App {
    pub(crate) title: String,
    pub(crate) window: MainWindow,
    pub(crate) initialize: Option<Initialize>,
    pub(crate) runtime: Runtime,
    /// Whether windows present CPU frames instead of using the GPU.
    pub(crate) software: bool,
    error_handler: Option<ErrorHandler>,
    close_handler: Option<CloseHandler>,
    window_error_handler: Option<WindowErrorHandler>,
}

/// Shows the title, the main window's geometry and whether presentation is
/// software, not the runtime or the callbacks.
impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("title", &self.title)
            .field("window", &self.window)
            .field("software", &self.software)
            .finish_non_exhaustive()
    }
}
impl App {
    /// Defines content once using the public runtime, with no backend configuration.
    ///
    /// Initialization runs on the calling main thread from `run`, after the native
    /// event loop exists and before any window opens, so the runtime has no
    /// viewport yet: it is supplied when the window opens, before the first
    /// flush the application runs. Content laid out by a flush inside the closure
    /// has empty bounds, which development validation does not report. Retain
    /// parameter host owners in components or captured application-owned state.
    /// The closure does not repeat on redraw. Errors are returned from `run`.
    pub fn new(
        title: impl Into<String>,
        initialize: impl FnOnce(&mut Runtime) -> Result<(), AppError> + 'static,
    ) -> Self {
        Self {
            title: title.into(),
            window: MainWindow::default(),
            initialize: Some(Box::new(initialize)),
            runtime: Runtime::new(),
            software: false,
            error_handler: None,
            close_handler: None,
            window_error_handler: None,
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
        self.window.size = nonzero(size)?;
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
        ordered(Some(size), self.window.max)?;
        self.window.min = Some(size);
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
        ordered(self.window.min, Some(size))?;
        self.window.max = Some(size);
        Ok(self)
    }
    /// Sets whether the user can resize the main window. Defaults to true. The
    /// content still receives size changes the window system makes, such as a
    /// scale change after moving to another monitor.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.window.resizable = resizable;
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
        self.software = true;
        self
    }

    /// Installs a UI-thread policy for component input, update and drawing failures,
    /// and for a window resized beyond the device's largest surface
    /// ([`AppError::Limit`]).
    ///
    /// Runs after input cleanup, outside runtime evaluation, so the handler
    /// may repair local signals through `runtime`. It must return promptly; no
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
    /// ```no_run
    /// use pert_ui::{App, Element, ErrorAction, Label};
    /// # fn main() -> Result<(), pert_ui::AppError> {
    /// App::new("Example", |ui| {
    ///     ui.mount_tree(Element::new("label", Label::new("Ready")))?;
    ///     Ok(())
    /// }).on_error(|_ui, _error| ErrorAction::Exit).run()?;
    /// # Ok(()) }
    /// ```
    pub fn on_error(
        mut self,
        handler: impl FnMut(&mut Runtime, &AppError) -> ErrorAction + 'static,
    ) -> Self {
        self.error_handler = Some(Box::new(handler));
        self
    }

    /// Installs close/save admission for main and detached windows. Defaults to
    /// [`NativeCloseAction::Close`]. The callback may present confirmation in the
    /// invoking window; return [`NativeCloseAction::Defer`] with its decision
    /// signal to preserve all windows while waiting.
    /// Errors abort closing and follow on_error without replaying the request.
    pub fn on_close(
        mut self,
        handler: impl FnMut(
            &mut Runtime,
            &crate::NativeWindowId,
        ) -> Result<NativeCloseAction, RuntimeError>
        + 'static,
    ) -> Self {
        self.close_handler = Some(Box::new(handler));
        self
    }
    /// Reports failed asynchronous native presentation, including tab drag failure.
    /// Defaults to stderr. The receipt retains the typed failure independently.
    /// Called on the UI thread after failed preparation/cleanup; existing content
    /// remains available and the operation is never automatically retried.
    pub fn on_window_error(
        mut self,
        handler: impl FnMut(&mut Runtime, &crate::WindowError) + 'static,
    ) -> Self {
        self.window_error_handler = Some(Box::new(handler));
        self
    }
    pub(crate) fn handle_window_error(
        &mut self,
        error: &crate::WindowError,
    ) -> Result<(), AppError> {
        if let Some(handler) = &mut self.window_error_handler {
            handler(&mut self.runtime, error);
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
        match &mut self.close_handler {
            Some(handler) => handler(&mut self.runtime, window),
            None => Ok(NativeCloseAction::Close),
        }
    }

    /// Asks the host what to do about a failure it can recover from. The
    /// default reports to standard error and retries; when that report cannot be
    /// written, the failure comes back inside [`AppError::ErrorReporting`].
    pub(crate) fn handle_error(
        &mut self,
        error: AppError,
    ) -> Result<(ErrorAction, AppError), AppError> {
        if let Some(handler) = &mut self.error_handler {
            let action = handler(&mut self.runtime, &error);
            return Ok((action, error));
        }
        use std::io::Write;
        // The site names the component, so the report says where to look.
        let site = self
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
    /// recovery failure, then a parked window's. Selection among several parked
    /// recovery failures is unspecified; it is not a cross-window error history.
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

    #[test]
    fn main_window_geometry_defaults_and_validates() {
        assert_eq!(app().window, MainWindow::default());
        assert!(empty(app().size(Size::from([0, 480]))));
        assert!(empty(app().min_size(Size::from([640, 0]))));
        let bounded = app()
            .size(Size::from([1280, 800]))
            .and_then(|app| app.min_size(Size::from([640, 400])))
            .and_then(|app| app.max_size(Size::from([1920, 1200])))
            .unwrap()
            .resizable(false);
        assert_eq!(bounded.window.size, Size::from([1280, 800]));
        assert_eq!(bounded.window.min, Some(Size::from([640, 400])));
        assert_eq!(bounded.window.max, Some(Size::from([1920, 1200])));
        assert!(!bounded.window.resizable);
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
