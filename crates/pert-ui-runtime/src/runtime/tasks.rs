//! Bounded application operations applied outside component callbacks.
use super::*;

/// Result of an operation queued with [`EventContext::defer`],
/// [`EventContext::defer_while_shown`] or [`Runtime::defer`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskStatus {
    /// The callback has not run.
    Pending,
    /// The callback returned successfully.
    Succeeded,
    /// The operation did not complete: the runtime shut down before delivery;
    /// for an operation queued with [`EventContext::defer_while_shown`], its
    /// component was hidden or removed first; or its callback panicked and the
    /// operation unwound.
    Cancelled,
    /// The callback failed; it is never replayed automatically.
    Failed(RuntimeError),
}

/// UI-local receipt for one operation queued with [`EventContext::defer`],
/// [`EventContext::spawn`] or [`Runtime::defer`].
///
/// [`Self::status`] reads the owned status, which stays readable after runtime
/// shutdown, where [`TaskStatus::Cancelled`] reports an operation the shutdown
/// discarded. [`Self::status_binding`] mirrors it as a binding while the
/// runtime lives. Retaining a receipt does not retain the originating widget or
/// runtime.
///
/// Dropping the receipt loses no report the application does not already
/// have, so the type is not `#[must_use]` and a callback writes
/// `cx.defer(...)?;` without binding it. A failure also propagates from the
/// flush that delivers the operation, and only runtime shutdown, or a panic
/// as the operation unwinds, cancels it before it completes: events the
/// application sees itself. Keep the receipt to show progress, or to tell a
/// shutdown's cancellation from success afterwards, such as for a save. The
/// operation a component queues with [`EventContext::defer_while_shown`],
/// which hiding or removal also cancels, returns a [`ShownTaskReceipt`]
/// instead.
///
/// Receipts compare equal when they report the same operation.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Runtime, TaskStatus};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let saved = ui.signal(false);
/// let writer = saved.clone();
/// let receipt = ui.defer(move |_ui| writer.set(true))?;
/// assert_eq!(receipt.status(), TaskStatus::Pending);
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(receipt.status(), TaskStatus::Succeeded);
/// assert!(saved.get()?);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[derive(Clone)]
pub struct TaskReceipt(ReceiptStatus<TaskStatus>);
impl TaskReceipt {
    /// Current admission/delivery outcome, including cancellation after teardown.
    /// Reading creates no subscription or timer.
    #[must_use]
    pub fn status(&self) -> TaskStatus {
        self.0.get()
    }
    /// The same status as a binding, so a component that reads it during
    /// update is invalidated when the operation finishes. Like every binding it
    /// reads `Err(RuntimeError::Closed)` after runtime shutdown; use
    /// [`Self::status`] then.
    pub fn status_binding(&self) -> ReadSignal<TaskStatus> {
        self.0.binding()
    }
}
impl PartialEq for TaskReceipt {
    fn eq(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}
impl Eq for TaskReceipt {}
/// Shows the current status, as [`TaskReceipt::status`] reads it.
impl std::fmt::Debug for TaskReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskReceipt")
            .field("status", &self.status())
            .finish()
    }
}

/// UI-local receipt for one operation a component queued on its own behalf
/// with [`EventContext::defer_while_shown`].
///
/// It reads like a [`TaskReceipt`]: [`Self::status`] is the owned status,
/// readable after runtime shutdown, and [`Self::status_binding`] mirrors it as
/// a binding while the runtime lives. Retaining it does not retain the
/// component or the runtime. Receipts compare equal when they report the same
/// operation.
///
/// The type is `#[must_use]` because it is the only report of the
/// cancellation this operation alone has: [`TaskStatus::Cancelled`] when the
/// component is hidden or removed before delivery, which no flush and no
/// other call reports. A failure also propagates from the flush, as for
/// every queued operation. Because the attribute is on the receipt and not on
/// the `Result` that carries it, discarding the receipt after `?` still warns;
/// bind it as `let _receipt` only where skipping the operation silently is the
/// intent.
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{EventContext, RuntimeError};
/// fn refresh(cx: &EventContext) -> Result<(), RuntimeError> {
///     cx.defer_while_shown(|_ui| Ok(()))?; // denied: the receipt is discarded
///     Ok(())
/// }
/// ```
#[derive(Clone)]
#[must_use = "the receipt is the only report of a `Cancelled` operation, including the \
              cancellation by hiding or removing its component; failures also propagate \
              from flush"]
pub struct ShownTaskReceipt(ReceiptStatus<TaskStatus>);
impl ShownTaskReceipt {
    /// Current admission/delivery outcome, including cancellation by hiding,
    /// removal or teardown. Reading creates no subscription or timer.
    #[must_use]
    pub fn status(&self) -> TaskStatus {
        self.0.get()
    }
    /// The same status as a binding, so a component that reads it during
    /// update is invalidated when the operation finishes or is cancelled. Like
    /// every binding it reads `Err(RuntimeError::Closed)` after runtime
    /// shutdown; use [`Self::status`] then.
    pub fn status_binding(&self) -> ReadSignal<TaskStatus> {
        self.0.binding()
    }
}
impl PartialEq for ShownTaskReceipt {
    fn eq(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}
impl Eq for ShownTaskReceipt {}
/// Shows the current status, as [`ShownTaskReceipt::status`] reads it.
impl std::fmt::Debug for ShownTaskReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShownTaskReceipt")
            .field("status", &self.status())
            .finish()
    }
}

type Action = Box<dyn FnOnce(&mut Runtime) -> Result<(), RuntimeError>>;
/// What a queued operation's delivery and routing depend on.
enum Owner {
    /// [`EventContext::defer`]: cancelled only by shutdown; runs in the
    /// widget's window while the widget is mounted, else in the window it was
    /// queued in while that window is open.
    Event {
        widget: Weak<Node>,
        origin: Weak<native_windows::WindowShared>,
    },
    /// [`EventContext::defer_while_shown`]: runs in the widget's window while
    /// the widget is mounted and effectively visible, and is cancelled
    /// otherwise.
    Shown(Weak<Node>),
    /// [`Runtime::defer`]: runs in whichever window is selected.
    Application,
}
pub(super) struct Queued {
    owner: Owner,
    action: Option<Action>,
    receipt: ReceiptStatus<TaskStatus>,
}
impl Drop for Queued {
    fn drop(&mut self) {
        if self.receipt.get() == TaskStatus::Pending {
            self.receipt.set(TaskStatus::Cancelled);
        }
    }
}
fn enqueue(
    shared: &Rc<Shared>,
    owner: Owner,
    action: Action,
) -> Result<ReceiptStatus<TaskStatus>, RuntimeError> {
    // Shutdown clears the queue early in teardown; an operation queued after
    // that, such as from a `cancel_input` that removal calls, would never be
    // delivered or cancelled.
    if shared.shutting_down.get() {
        return Err(RuntimeError::Closed);
    }
    shared.writable()?;
    let mut queue = shared.tasks.borrow_mut();
    if queue.len() >= Runtime::MAX_TASKS {
        return Err(RuntimeError::TaskQueueFull);
    }
    let receipt = ReceiptStatus::new(shared.clone(), TaskStatus::Pending);
    queue.push_back(Queued {
        owner,
        action: Some(action),
        receipt: receipt.clone(),
    });
    Ok(receipt)
}

impl EventContext {
    /// Queues an application operation with mutable runtime access for the next
    /// flush, before component evaluation. This is the deferral for
    /// application callbacks: use it to create document parameters, register
    /// commands, open or save a document or change a tree in response to
    /// input. Background work starts with [`Self::spawn`].
    ///
    /// Timing class: [at the next flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).
    ///
    /// The operation belongs to the application, not to the widget: only
    /// runtime shutdown cancels it before delivery. It runs once at the next
    /// flush even when the widget is hidden or removed first, so a callback
    /// on a surface that closes, such as a [`Select`](crate::Select) change, a
    /// menu row, a [`QuickPick`](crate::QuickPick) choice or a dialog's Save
    /// button, queues its work here, and the work runs after the surface has
    /// closed. It also runs when the callback closes or redocks its detached
    /// window. An operation whose targets may be gone by then reports their
    /// error, such as `Unmounted`, from the flush; work a component does on
    /// itself, which should be skipped with it, uses
    /// [`Self::defer_while_shown`].
    ///
    /// At delivery the operation runs with the widget's window selected while
    /// the widget is mounted, wherever detaching or redocking has moved it;
    /// once the widget is gone, with the window it was queued in (the
    /// [`Self::native_window`] of this call) while that window is open; and
    /// otherwise with whichever window the delivering flush has selected.
    /// Routing never cancels the operation or fails. Work that must address
    /// the window the event came from, wherever the widget is later, captures
    /// [`Self::native_window`] and selects it in the operation. After the
    /// operation the earlier selection is restored while its window is open.
    ///
    /// Cancellation reaches only queued operations. Once delivered, an
    /// operation is executing and nothing cancels it: if it calls
    /// [`Runtime::shutdown`], the shutdown cancels the operations still queued
    /// and refuses new ones, and this operation's receipt records its own
    /// result when it returns. If it panics, its receipt reads `Cancelled` as
    /// it unwinds, and the operations behind it stay queued until the next
    /// delivery or shutdown.
    ///
    /// The [`TaskReceipt`] may be dropped, as `cx.defer(...)?;` does: a
    /// failure also propagates from the flush, and the only cancellations,
    /// shutdown and a panic as the operation unwinds, are events the
    /// application sees itself. Keep it to tell a shutdown's cancellation
    /// from success afterwards, such as for a save. Focus/capture are not transferred
    /// to replacement widgets. New operations queued during delivery wait for
    /// the next flush. Errors propagate from flush and remain in the receipt;
    /// partial application work is not rolled back or replayed. A returned
    /// callback error is an explicit application operation failure, including
    /// a nested runtime error it chooses to propagate; a caught error does not
    /// classify subsequent runtime work. Callbacks must return promptly. This
    /// is UI-thread work, never a callback-safe endpoint.
    ///
    /// The callback may call any runtime operation, including those that flush
    /// internally, such as [`Runtime::reveal_child`], [`Runtime::reveal`],
    /// [`Runtime::focus`] and [`Runtime::flush`] itself. Such a nested flush
    /// settles the tree but delivers no further deferred tasks: tasks queued
    /// before delivery started still run in order after this one, and tasks
    /// queued from inside it wait for the next flush.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Element, KeyAction, KeyCode, Label, PickChoice, PickItem, PickPending, PickStatus,
    ///     QuickPick, QuickPickController, Runtime, Viewport,
    /// };
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    /// let picker = QuickPickController::new(&ui);
    /// let opened = ui.signal(None::<String>);
    /// let writer = opened.clone();
    /// ui.mount_tree(Element::new(
    ///     "app",
    ///     QuickPick::new(
    ///         picker.clone(),
    ///         Element::new("doc", Label::new("Document")),
    ///         move |cx, item| {
    ///             // Answering `Pending` closes the picker, which hides the row
    ///             // this callback runs for; the queued work still runs.
    ///             let (result, key) = (writer.clone(), item.key.clone());
    ///             cx.defer(move |_ui| {
    ///                 // Work that flushes, such as revealing a page, goes here.
    ///                 result.set(Some(key))
    ///             })?;
    ///             let done = writer.read_only();
    ///             Ok(PickChoice::Pending(PickPending::new(done, |value| {
    ///                 value.as_ref().map(|_| Ok(()))
    ///             })))
    ///         },
    ///     ),
    /// ))?;
    /// picker.open(vec![PickItem::new("knobs", "Knobs")])?;
    /// ui.flush(Duration::ZERO)?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(opened.get()?.as_deref(), Some("knobs"));
    /// assert_eq!(picker.status()?, PickStatus::Closed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns `TaskQueueFull` at [`Runtime::MAX_TASKS`] pending operations,
    /// or the runtime phase/shutdown error, including from cleanup callbacks
    /// that run during [`Runtime::shutdown`], without accepting the operation.
    pub fn defer(
        &self,
        action: impl FnOnce(&mut Runtime) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError> {
        enqueue(
            &self.node.shared,
            Owner::Event {
                widget: Rc::downgrade(&self.node),
                origin: Rc::downgrade(&self.node.window()),
            },
            Box::new(action),
        )
        .map(TaskReceipt)
    }

    /// Queues an operation on this component's behalf, cancelled when the
    /// component is hidden or removed before the next flush. It is for custom
    /// component authors (see [`Component`]), for work a component does on
    /// itself, such as describing its own children again from an input hook,
    /// that should end with the component as its
    /// [`set_timer`](Self::set_timer) deadline does. Application callbacks use
    /// [`Self::defer`].
    ///
    /// Timing class: [at the next flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).
    ///
    /// The [`ShownTaskReceipt`] reads `Cancelled` without running the
    /// operation when, at delivery, the component is unmounted or not
    /// effectively visible: hidden itself or through an ancestor, on a surface
    /// that has closed, or in a native window that is hidden. Runtime shutdown
    /// also cancels it. Otherwise it runs with the component's window at
    /// delivery selected, wherever detaching or redocking has moved it.
    /// Delivery order, statuses, errors, nested flushes and capacity match
    /// [`Self::defer`], with which it shares one queue.
    ///
    /// Nothing but the receipt reports a cancellation by hiding or removal,
    /// so its type is `#[must_use]` and discarding it after `?` warns, unlike
    /// [`Self::defer`]'s [`TaskReceipt`]. Keep it, or bind it as
    /// `let _receipt` where skipping the work silently is the intent.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Component, Element, EventContext, InputOutcome, KeyAction, KeyCode, KeyEvent,
    ///     KeyTransition, KeyboardState, Runtime, RuntimeError, ShownTaskReceipt, TaskStatus,
    ///     UpdateContext,
    /// };
    /// use std::{cell::RefCell, rc::Rc, time::Duration};
    ///
    /// /// Refreshes its own content one flush after Enter, while it is shown.
    /// struct Refreshing(Rc<RefCell<Option<ShownTaskReceipt>>>);
    /// impl Component for Refreshing {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn focusable(&self) -> bool { true }
    ///     fn keyboard_event(
    ///         &mut self,
    ///         cx: &mut EventContext,
    ///         event: KeyEvent,
    ///         _: &KeyboardState,
    ///     ) -> Result<InputOutcome, RuntimeError> {
    ///         if event.transition == KeyTransition::Pressed {
    ///             // Describing this component's children again would go here.
    ///             *self.0.borrow_mut() = Some(cx.defer_while_shown(|_ui| Ok(()))?);
    ///         }
    ///         Ok(InputOutcome::Handled)
    ///     }
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// let receipt = Rc::new(RefCell::new(None));
    /// let id = ui.mount_tree(Element::new("refreshing", Refreshing(receipt.clone())))?;
    /// ui.focus(&id)?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    /// // Hidden before the flush delivers it: the refresh is skipped.
    /// ui.set_visible(&id, false)?;
    /// ui.flush(Duration::ZERO)?;
    /// let status = receipt.borrow().as_ref().map(ShownTaskReceipt::status);
    /// assert_eq!(status, Some(TaskStatus::Cancelled));
    /// # Ok::<(), RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns `TaskQueueFull` at [`Runtime::MAX_TASKS`] pending operations,
    /// or the runtime phase/shutdown error, including from cleanup callbacks
    /// that run during [`Runtime::shutdown`], without accepting the operation.
    pub fn defer_while_shown(
        &self,
        action: impl FnOnce(&mut Runtime) -> Result<(), RuntimeError> + 'static,
    ) -> Result<ShownTaskReceipt, RuntimeError> {
        enqueue(
            &self.node.shared,
            Owner::Shown(Rc::downgrade(&self.node)),
            Box::new(action),
        )
        .map(ShownTaskReceipt)
    }

    /// Starts `work` on a background thread from a callback and hands its
    /// result to `deliver` on the UI thread, as [`Runtime::spawn`] does.
    ///
    /// Timing class: [at the next flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush), for the start and for `deliver`.
    ///
    /// A callback has no runtime, so this queues the start: it is the
    /// operation `|ui| ui.spawn(work, deliver).map(|_| ())` queued with
    /// [`Self::defer`]. The thread starts at the beginning of the next flush,
    /// before component evaluation, as every queued operation does. The start
    /// belongs to the application, as `defer`'s operation does: it runs at the
    /// next flush whatever happens to the widget or its window, and only
    /// shutdown cancels it before it starts. Once started, the result follows
    /// [`Runtime::spawn`]'s rules: results are delivered in arrival order, a
    /// panicking `work` fails the flush that receives its outcome with
    /// [`RuntimeError::BackgroundPanicked`], and nothing is delivered after
    /// shutdown.
    ///
    /// The [`TaskReceipt`] reports the start, not the result:
    ///
    /// - [`TaskStatus::Succeeded`]: the thread started.
    /// - [`TaskStatus::Failed`]: [`Runtime::spawn`] refused with that error
    ///   and nothing started, such as `BackgroundFull` while
    ///   [`Runtime::MAX_BACKGROUND_TASKS`] tasks wait, `BackgroundSpawn` with
    ///   the operating system's reason, or `Exhausted`. As for every queued
    ///   operation, the error also propagates from that flush.
    /// - [`TaskStatus::Cancelled`]: the runtime shut down before the start.
    ///
    /// It may be dropped, as `cx.spawn(...)?;` does, for the reasons
    /// [`Self::defer`] gives.
    ///
    /// There is no handle to cancel the delivery. Code that must discard a
    /// result that has not been delivered, or tie the start to a component,
    /// queues [`Runtime::spawn`] itself, with [`Self::defer`] or
    /// [`Self::defer_while_shown`], and keeps the
    /// [`BackgroundTask`](crate::BackgroundTask) it returns. `work` receives
    /// no UI handles; return what the interface needs and let `deliver` write
    /// it to signals.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Button, Element, KeyAction, KeyCode, Runtime};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let status = ui.signal("Ready".to_owned());
    /// let shown = status.clone();
    /// let count = Button::new("Count words", move |cx| {
    ///     shown.set("Counting".to_owned())?;
    ///     // Real work would read a file; the result carries any failure.
    ///     let text = String::from("one two three");
    ///     let shown = shown.clone();
    ///     cx.spawn(
    ///         move || text.split_whitespace().count(),
    ///         move |words| shown.set(format!("{words} words")),
    ///     )?;
    ///     Ok(())
    /// });
    /// let button = ui.mount_tree(Element::new("count", count))?;
    /// ui.flush(Duration::ZERO)?;
    /// ui.focus(&button)?;
    /// ui.key_input(KeyCode::Space, KeyAction::Press)?;
    /// ui.key_input(KeyCode::Space, KeyAction::Release)?;
    /// // A native host flushes when the result wakes it; a headless one polls.
    /// while status.get()? == "Counting" {
    ///     std::thread::sleep(Duration::from_millis(1));
    ///     ui.flush(Duration::ZERO)?;
    /// }
    /// assert_eq!(status.get()?, "3 words");
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns `TaskQueueFull` at [`Runtime::MAX_TASKS`] pending operations,
    /// or the runtime phase/shutdown error, including from cleanup callbacks
    /// that run during [`Runtime::shutdown`], without accepting the operation.
    /// [`Runtime::spawn`]'s own refusals arrive later, in the receipt and from
    /// the flush that starts the work.
    pub fn spawn<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
        deliver: impl FnOnce(T) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError> {
        self.defer(move |ui| ui.spawn(work, deliver).map(|_| ()))
    }
}
impl Runtime {
    /// Maximum number of accepted, undelivered application operations, across
    /// [`EventContext::defer`], [`EventContext::defer_while_shown`] and
    /// [`Self::defer`].
    pub const MAX_TASKS: usize = 64;

    /// Queues an application-owned operation, independent of a widget lifetime.
    /// Delivery, receipts, errors and capacity match [`EventContext::defer`],
    /// including that only runtime shutdown cancels it before delivery, that
    /// nothing cancels it once it executes and that its [`TaskReceipt`] may be
    /// dropped. The selection is not changed:
    /// the operation runs with whichever window the delivering flush has
    /// selected; select one explicitly for window-specific work.
    ///
    /// Timing class: [at the next flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).
    ///
    /// # Errors
    /// Returns `TaskQueueFull` or the runtime phase/shutdown error, `Closed`
    /// from the moment [`Self::shutdown`] begins.
    pub fn defer(
        &self,
        action: impl FnOnce(&mut Self) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError> {
        enqueue(&self.shared, Owner::Application, Box::new(action)).map(TaskReceipt)
    }

    pub(super) fn dispatch_tasks(&mut self) -> Result<(), RuntimeError> {
        // A flush nested inside a task does not deliver further tasks: the
        // outer loop delivers those queued before it started, and tasks queued
        // during delivery wait for the next flush, as `defer` documents.
        if self.shared.task_dispatching.replace(true) {
            return Ok(());
        }
        // Cleared on return and on unwind from a panicking task alike.
        struct Delivering(Rc<super::Shared>);
        impl Drop for Delivering {
            fn drop(&mut self) {
                self.0.task_dispatching.set(false);
            }
        }
        let _delivering = Delivering(self.shared.clone());
        self.dispatch_tasks_inner()
    }

    fn dispatch_tasks_inner(&mut self) -> Result<(), RuntimeError> {
        let count = self.shared.tasks.borrow().len();
        for _ in 0..count {
            let task = self.shared.tasks.borrow_mut().pop_front();
            let Some(mut task) = task else {
                break;
            };
            let window = match &task.owner {
                Owner::Event { widget, origin } => widget
                    .upgrade()
                    .filter(|n| n.mounted.get())
                    .map(|n| n.window())
                    .or_else(|| origin.upgrade())
                    // Routing never cancels or fails: a closed window is
                    // skipped, not selected.
                    .filter(|w| w.mounted.get())
                    .map(native_windows::NativeWindowId),
                Owner::Shown(owner) => {
                    let Some(node) = owner
                        .upgrade()
                        .filter(|n| n.mounted.get() && n.visible.get())
                    else {
                        continue;
                    };
                    Some(native_windows::NativeWindowId(node.window()))
                }
                Owner::Application => None,
            };
            let previous = self.current_window();
            if let Some(window) = window {
                self.select_window(&window)?;
            }
            let result = match task.action.take() {
                Some(action) => {
                    let _errors = super::notify::InputErrorGuard::new(&self.shared);
                    action(self)
                }
                None => Ok(()),
            };
            // An operation may retire its originating window.
            if previous.0.mounted.get() {
                self.select_window(&previous)?;
            }
            let status = match &result {
                Ok(()) => TaskStatus::Succeeded,
                Err(e) => TaskStatus::Failed(e.clone()),
            };
            task.receipt.set(status);
            result?;
        }
        Ok(())
    }
}
