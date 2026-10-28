//! An [`App`] without an event loop, for tests of its content and close policy.
use super::{App, AppError, Bound, Host, NativeCloseAction, close};
use crate::host::HostIntegration;
use crate::{NativeWindowId, Runtime};
use std::time::Duration;

/// An [`App`] started without an event loop, window, renderer or GPU, for
/// testing its content and its [`App::on_close`] policy. Created by
/// [`App::headless`].
///
/// The runtime is the application's own, initialized as [`App::run`] would
/// initialize it, and is driven like any [`Runtime`]: with
/// [`Driver`](crate::testing::Driver) or the runtime's input methods, through
/// [`Self::runtime_mut`]. [`Self::admit_close`] runs the close policy as a
/// close request does. Text layout needs no window, so content measures and
/// lays out as in a window of the configured size.
///
/// End it with [`Self::shutdown`], which reports cleanup failures. Dropping
/// it shuts the runtime down through [`Runtime`]'s drop, which reports
/// nothing. Either way the state is dropped after the runtime has shut down.
///
/// The initializer's value is the [application state](App#application-state):
/// the close policy receives it, and the test reads it with [`Self::state`]
/// and drives the runtime with the handles it holds through
/// [`Self::runtime_and_state_mut`].
///
/// ```
/// use pert_ui::prelude::*;
/// use pert_ui::testing::{Driver, HeadlessApp};
///
/// /// What the application builds, and its close policy and a test read.
/// struct Notes {
///     field: WidgetId,
///     text: Signal<String>,
///     saved: Signal<String>,
/// }
/// fn build(ui: &mut Runtime) -> Result<Notes, RuntimeError> {
///     let text = ui.signal(String::new());
///     // A length limit makes the field hold a draft until Enter or focus loss.
///     let field = ui.mount_tree(Element::new("note", TextField::new(text.clone()).max_length(Some(200))))?;
///     Ok(Notes { field, text, saved: ui.signal(String::new()) })
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut app: HeadlessApp<Notes> = App::new("Notes", |ui| Ok(build(ui)?))
///     // Unsaved text keeps the window open.
///     .on_close(|_, notes, _| {
///         Ok(if notes.text.get()? == notes.saved.get()? {
///             NativeCloseAction::Close
///         } else {
///             NativeCloseAction::KeepOpen
///         })
///     })
///     .headless()?;
///
/// let (ui, notes) = app.runtime_and_state_mut();
/// Driver::new(ui).replace_text(&notes.field, "Call the bank")?;
/// assert_eq!(app.state().text.get()?, ""); // still a draft
/// // Admission saves the focused draft before the policy runs, so the policy
/// // sees it; keeping the window open focuses the field again.
/// let main = app.main_window();
/// assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
/// assert_eq!(app.state().text.get()?, "Call the bank");
/// assert_eq!(app.runtime().focused(), Some(app.state().field.clone()));
///
/// let notes = app.state();
/// notes.saved.set(notes.text.get()?)?;
/// assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));
/// app.shutdown()?;
/// # Ok(())
/// # }
/// ```
#[must_use = "dropping it shuts the runtime down without reporting cleanup errors"]
pub struct HeadlessApp<S = ()> {
    // Declared before the state so that dropping the application drops the
    // runtime, which shuts down in its drop, before the state.
    host: Host,
    bound: Bound<S>,
}

/// Shows the title and the main window's geometry, as [`App`]'s does, not
/// the runtime, the callbacks or the state.
impl<S> std::fmt::Debug for HeadlessApp<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeadlessApp")
            .field("title", &self.host.title)
            .field("window", &self.host.window)
            .finish_non_exhaustive()
    }
}

impl<S: 'static> App<S> {
    /// Starts the application without an event loop, for a test.
    ///
    /// In this order:
    /// 1. the title is validated as [`Self::run`] validates it;
    /// 2. the viewport is set to the main window's configured
    ///    [size](Self::size) at scale 1;
    /// 3. the initialization closure runs on the runtime at clock zero, and
    ///    the [application state](Self#application-state) it returns is kept
    ///    for [`HeadlessApp::state`] and the close policy;
    /// 4. the primary window is selected again, since the closure may have
    ///    selected a detached one, and every visible root is flushed with
    ///    [`HostIntegration::flush_windows`](crate::host::HostIntegration::flush_windows) at
    ///    [`Duration::ZERO`], so its content is
    ///    laid out. The primary window stays selected; a hidden root lays out
    ///    when it is shown.
    ///
    /// The size is simulated geometry: [`Self::min_size`] and
    /// [`Self::max_size`] are not applied, because no window system clamps.
    /// No event loop, window, renderer or GPU is created, so it can be called
    /// on any thread and more than once per process, and
    /// `software_rendering` has no effect. The [`Self::on_close`] policy runs
    /// through [`HeadlessApp::admit_close`]; [`Self::on_error`] and
    /// [`Self::on_window_error`] belong to the native runner and are not
    /// called. No clipboard is installed; a test installs one with
    /// [`HostIntegration::set_clipboard`](crate::host::HostIntegration::set_clipboard).
    ///
    /// # Errors
    /// [`AppError::InvalidTitle`] for a title without a non-whitespace
    /// character, before anything runs. When setting the viewport, the
    /// initialization closure or the first flush fails, the runtime is shut
    /// down, as [`Self::run`] does for an initialization failure, and that
    /// failure is returned, inside [`AppError::Shutdown`] when cleanup failed
    /// too. A state the closure returned is dropped after that shutdown.
    pub fn headless(mut self) -> Result<HeadlessApp<S>, AppError> {
        if self.host.title.trim().is_empty() {
            return Err(AppError::InvalidTitle);
        }
        let size = self.host.window.size;
        if let Err(error) = viewport(&mut self.host.runtime, size) {
            return crate::platform::finish_runtime(&mut self.host.runtime, Err(error));
        }
        let (host, bound) = self.start()?;
        // The application owns both before anything can panic, so unwinding
        // drops the runtime before the state, as a normal return does.
        let mut app = HeadlessApp { host, bound };
        if let Err(error) = settle(&mut app.host.runtime) {
            // Returning drops the state after the runtime has shut down.
            return crate::platform::finish_runtime(&mut app.host.runtime, Err(error));
        }
        Ok(app)
    }
}

fn viewport(runtime: &mut Runtime, size: crate::Size) -> Result<(), AppError> {
    runtime.set_viewport(crate::Viewport::new(size.width(), size.height(), 1.0)?)?;
    Ok(())
}

/// Selects the primary window again and lays out every visible root.
fn settle(runtime: &mut Runtime) -> Result<(), AppError> {
    let primary = runtime.primary_window();
    runtime.select_window(&primary)?;
    runtime.flush_windows(Duration::ZERO)?;
    Ok(())
}

impl<S: 'static> HeadlessApp<S> {
    /// The application's runtime, for reading what it shows.
    pub fn runtime(&self) -> &Runtime {
        &self.host.runtime
    }

    /// The application's runtime, for driving it.
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.host.runtime
    }

    /// The main window: [`Runtime::primary_window`].
    pub fn main_window(&self) -> NativeWindowId {
        self.host.runtime.primary_window()
    }

    /// The [application state](App#application-state) the initializer
    /// returned, as the callbacks see it.
    ///
    /// ```
    /// use pert_ui::{App, NativeCloseAction};
    /// /// The policy's count of the closes it refused.
    /// struct Closes {
    ///     refused: u32,
    /// }
    /// # fn main() -> Result<(), pert_ui::AppError> {
    /// let mut app = App::new("Counted", |_| Ok(Closes { refused: 0 }))
    ///     .on_close(|_, closes, _| {
    ///         closes.refused += 1;
    ///         Ok(NativeCloseAction::KeepOpen)
    ///     })
    ///     .headless()?;
    /// let main = app.main_window();
    /// app.admit_close(&main)?;
    /// assert_eq!(app.state().refused, 1);
    /// # app.shutdown() }
    /// ```
    pub fn state(&self) -> &S {
        self.bound.state()
    }

    /// The application state, for a change a test makes as the application
    /// would. The callbacks see the change on their next call.
    ///
    /// ```
    /// use pert_ui::{App, NativeCloseAction};
    /// /// The policy's count of the closes it refused.
    /// struct Closes {
    ///     refused: u32,
    /// }
    /// # fn main() -> Result<(), pert_ui::AppError> {
    /// let mut app = App::new("Counted", |_| Ok(Closes { refused: 0 }))
    ///     .on_close(|_, closes, _| {
    ///         closes.refused += 1;
    ///         Ok(NativeCloseAction::KeepOpen)
    ///     })
    ///     .headless()?;
    /// let main = app.main_window();
    /// app.admit_close(&main)?;
    /// app.state_mut().refused = 0;
    /// app.admit_close(&main)?;
    /// assert_eq!(app.state().refused, 1);
    /// # app.shutdown() }
    /// ```
    pub fn state_mut(&mut self) -> &mut S {
        self.bound.state_mut()
    }

    /// The runtime and the application state at once, as a callback
    /// receives them, so a test drives the runtime with the handles the
    /// state holds without copying them out first.
    ///
    /// ```
    /// use pert_ui::testing::Driver;
    /// use pert_ui::{App, Element, Signal, TextField, WidgetId};
    /// /// The field the application built and the text it edits.
    /// struct Notes {
    ///     field: WidgetId,
    ///     text: Signal<String>,
    /// }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut app = App::new("Notes", |ui| {
    ///     let text = ui.signal(String::new());
    ///     let field = ui.mount_tree(Element::new("note", TextField::new(text.clone())))?;
    ///     Ok(Notes { field, text })
    /// })
    /// .headless()?;
    /// let (ui, notes) = app.runtime_and_state_mut();
    /// Driver::new(ui).replace_text(&notes.field, "Call the bank")?;
    /// assert_eq!(app.state().text.get()?, "Call the bank");
    /// app.shutdown()?;
    /// # Ok(()) }
    /// ```
    pub fn runtime_and_state_mut(&mut self) -> (&mut Runtime, &mut S) {
        (&mut self.host.runtime, self.bound.state_mut())
    }

    /// Runs close admission and the [`App::on_close`] policy for `window`, as
    /// a close request does, and returns the decision without applying it.
    ///
    /// Admission is the native runner's: focus is released in every window
    /// the close would discard, so each focused editor applies its focus-loss
    /// policy before the policy runs, those windows are flushed, the policy
    /// runs with `window` selected and receives the state [`Self::state`]
    /// reads, and a synchronous
    /// [`NativeCloseAction::KeepOpen`] focuses the released components again;
    /// [`App::on_close`] lists the steps. There is no component-error backoff
    /// here, so admission always flushes. The window selected before the call
    /// is selected again afterwards, when it is still open.
    ///
    /// A test asserts [`NativeCloseAction::Close`] or
    /// [`NativeCloseAction::KeepOpen`]; for [`NativeCloseAction::Defer`] it
    /// drives the confirmation and reads the decision with
    /// [`Runtime::read_signal`]. Nothing is closed, and on `Defer` no focus is
    /// restored when the decision later keeps the window open, as in the
    /// native runner. Each call runs the policy again: coalescing a repeated
    /// close request while a decision is pending is the native runner's state.
    ///
    /// # Errors
    /// `ForeignRuntime` or `Unmounted` as [`AppError::Runtime`] for a window
    /// that is not an open window of this runtime, before anything changes.
    /// Then the errors of admission's flush, including a failed
    /// `Component::focus_lost` of an editor it released, which abort before
    /// the policy runs; the policy's error; `ForeignRuntime` for a
    /// [`NativeCloseAction::Defer`] signal of another runtime, checked at
    /// once; and the errors of focusing a released component again. Errors
    /// come back directly: [`App::on_error`] belongs to the native runner.
    pub fn admit_close(&mut self, window: &NativeWindowId) -> Result<NativeCloseAction, AppError> {
        let runtime = &mut self.host.runtime;
        let previous = runtime.current_window();
        // Validates the identity before anything changes.
        runtime.select_window(window)?;
        runtime.select_window(&previous)?;
        let affected = close::affected_roots(runtime, window);
        let now = runtime.now();
        let released = close::prepare_close(runtime, &affected, true, now)?;
        runtime.select_window(window)?;
        let decision = self.bound.close_policy(&mut self.host.runtime, window);
        let runtime = &mut self.host.runtime;
        let decided = decision.map_err(AppError::from).and_then(|action| {
            match &action {
                NativeCloseAction::KeepOpen => close::restore_focus(runtime, released)?,
                NativeCloseAction::Defer(decision) => {
                    runtime.read_signal(decision)?;
                }
                NativeCloseAction::Close => {}
            }
            Ok(action)
        });
        if runtime.contains_window(&previous) {
            runtime.select_window(&previous)?;
        }
        decided
    }

    /// Shuts the runtime down, as the application ends.
    ///
    /// # Errors
    /// [`AppError::Shutdown`] with the runtime's cleanup failures, in
    /// [`HostIntegration::take_shutdown_errors`](crate::host::HostIntegration::take_shutdown_errors) order.
    pub fn shutdown(mut self) -> Result<(), AppError> {
        // Returning drops the state after the runtime has shut down, and
        // unwinding from a panic in the shutdown drops the runtime first.
        crate::platform::finish_runtime(&mut self.host.runtime, Ok(()))
    }
}
