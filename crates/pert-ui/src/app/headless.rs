//! An [`App`] without an event loop, for tests of its content and close policy.
use super::{App, AppError, NativeCloseAction, close};
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
/// nothing.
///
/// ```
/// use pert_ui::prelude::*;
/// use pert_ui::testing::{Driver, HeadlessApp};
/// use std::{cell::RefCell, rc::Rc};
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
/// let notes: Rc<RefCell<Option<Rc<Notes>>>> = Rc::default();
/// let (built, policy) = (notes.clone(), notes.clone());
/// let mut app: HeadlessApp = App::new("Notes", move |ui| {
///     *built.borrow_mut() = Some(Rc::new(build(ui)?));
///     Ok(())
/// })
/// .on_close(move |_, _| {
///     let notes = policy.borrow().clone().ok_or(ApplicationError::from("not built"))?;
///     // Unsaved text keeps the window open.
///     Ok(if notes.text.get()? == notes.saved.get()? {
///         NativeCloseAction::Close
///     } else {
///         NativeCloseAction::KeepOpen
///     })
/// })
/// .headless()?;
/// let notes = notes.borrow().clone().ok_or("not built")?;
///
/// Driver::new(app.runtime_mut()).replace_text(&notes.field, "Call the bank")?;
/// assert_eq!(notes.text.get()?, ""); // still a draft
/// // Admission saves the focused draft before the policy runs, so the policy
/// // sees it; keeping the window open focuses the field again.
/// let main = app.main_window();
/// assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
/// assert_eq!(notes.text.get()?, "Call the bank");
/// assert_eq!(app.runtime().focused(), Some(notes.field.clone()));
///
/// notes.saved.set(notes.text.get()?)?;
/// assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));
/// app.shutdown()?;
/// # Ok(())
/// # }
/// ```
#[must_use = "dropping it shuts the runtime down without reporting cleanup errors"]
pub struct HeadlessApp {
    app: App,
}

/// Shows the title and the main window's geometry, as [`App`]'s does, not
/// the runtime or the callbacks.
impl std::fmt::Debug for HeadlessApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeadlessApp")
            .field("title", &self.app.title)
            .field("window", &self.app.window)
            .finish_non_exhaustive()
    }
}

impl App {
    /// Starts the application without an event loop, for a test.
    ///
    /// In this order:
    /// 1. the title is validated as [`Self::run`] validates it;
    /// 2. the viewport is set to the main window's configured
    ///    [size](Self::size) at scale 1;
    /// 3. the initialization closure runs on the runtime at clock zero;
    /// 4. the primary window is selected again, since the closure may have
    ///    selected a detached one, and every visible root is flushed with
    ///    [`Runtime::flush_windows`] at [`Duration::ZERO`], so its content is
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
    /// [`Runtime::set_clipboard`].
    ///
    /// # Errors
    /// [`AppError::InvalidTitle`] for a title without a non-whitespace
    /// character, before anything runs. When setting the viewport, the
    /// initialization closure or the first flush fails, the runtime is shut
    /// down, as [`Self::run`] does for an initialization failure, and that
    /// failure is returned, inside [`AppError::Shutdown`] when cleanup failed
    /// too.
    pub fn headless(mut self) -> Result<HeadlessApp, AppError> {
        if self.title.trim().is_empty() {
            return Err(AppError::InvalidTitle);
        }
        let started = self.start_headless();
        if let Err(error) = started {
            // Always an error: this failure, or the shutdown aggregate that
            // keeps it.
            crate::platform::finish_runtime(&mut self.runtime, Err(error))?;
        }
        Ok(HeadlessApp { app: self })
    }

    fn start_headless(&mut self) -> Result<(), AppError> {
        let size = self.window.size;
        self.runtime
            .set_viewport(crate::Viewport::new(size.width(), size.height(), 1.0)?)?;
        if let Some(initialize) = self.initialize.take() {
            initialize(&mut self.runtime)?;
        }
        let primary = self.runtime.primary_window();
        self.runtime.select_window(&primary)?;
        self.runtime.flush_windows(Duration::ZERO)?;
        Ok(())
    }
}

impl HeadlessApp {
    /// The application's runtime, for reading what it shows.
    pub fn runtime(&self) -> &Runtime {
        &self.app.runtime
    }

    /// The application's runtime, for driving it.
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.app.runtime
    }

    /// The main window: [`Runtime::primary_window`].
    pub fn main_window(&self) -> NativeWindowId {
        self.app.runtime.primary_window()
    }

    /// Runs close admission and the [`App::on_close`] policy for `window`, as
    /// a close request does, and returns the decision without applying it.
    ///
    /// Admission is the native runner's: focus is released in every window
    /// the close would discard, so each focused editor applies its focus-loss
    /// policy before the policy runs, those windows are flushed, the policy
    /// runs with `window` selected, and a synchronous
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
        let runtime = &mut self.app.runtime;
        let previous = runtime.current_window();
        // Validates the identity before anything changes.
        runtime.select_window(window)?;
        runtime.select_window(&previous)?;
        let affected = close::affected_roots(runtime, window);
        let now = runtime.now();
        let released = close::prepare_close(runtime, &affected, true, now)?;
        runtime.select_window(window)?;
        let decision = self.app.handle_close(window);
        let runtime = &mut self.app.runtime;
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
    /// [`Runtime::take_shutdown_errors`] order.
    pub fn shutdown(mut self) -> Result<(), AppError> {
        crate::platform::finish_runtime(&mut self.app.runtime, Ok(()))
    }
}
