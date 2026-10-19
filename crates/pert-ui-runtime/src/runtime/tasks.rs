//! Bounded application operations applied outside component callbacks.
use super::*;

/// Result of an operation queued with [`EventContext::defer`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskStatus {
    /// The callback has not run.
    Pending,
    /// The callback returned successfully.
    Completed,
    /// The owner disappeared or the runtime shut down before delivery.
    Cancelled,
    /// The callback failed; it is never replayed automatically.
    Failed(RuntimeError),
}

/// UI-local receipt for one deferred application operation.
///
/// [`Self::status`] reads the owned status, which stays readable after runtime
/// shutdown, where [`TaskStatus::Cancelled`] reports an operation the shutdown
/// discarded. [`Self::status_binding`] mirrors it as a binding while the
/// runtime lives. Retaining a receipt does not retain the originating widget or
/// runtime.
///
/// Receipts compare equal when they report the same operation.
#[derive(Clone)]
#[must_use = "retain the receipt if the application needs the operation's outcome"]
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

type Action = Box<dyn FnOnce(&mut Runtime) -> Result<(), RuntimeError>>;
/// What a queued operation's delivery depends on.
enum Owner {
    /// Runs in the widget's window while the widget is mounted and visible.
    Widget(Weak<Node>),
    /// Runs in the window while it is open.
    Window(Weak<native_windows::WindowShared>),
    /// Runs in whichever window is selected.
    Application,
}
pub(super) struct Queued {
    owner: Owner,
    action: Option<Action>,
    receipt: TaskReceipt,
}
impl Drop for Queued {
    fn drop(&mut self) {
        if self.receipt.status() == TaskStatus::Pending {
            self.receipt.0.set(TaskStatus::Cancelled);
        }
    }
}
fn enqueue(shared: &Rc<Shared>, owner: Owner, action: Action) -> Result<TaskReceipt, RuntimeError> {
    shared.writable()?;
    let mut queue = shared.tasks.borrow_mut();
    if queue.len() >= Runtime::MAX_TASKS {
        return Err(RuntimeError::TaskQueueFull);
    }
    let receipt = TaskReceipt(ReceiptStatus::new(shared.clone(), TaskStatus::Pending));
    queue.push_back(Queued {
        owner,
        action: Some(action),
        receipt: receipt.clone(),
    });
    Ok(receipt)
}

impl EventContext {
    /// Queues an application operation with mutable runtime access for the next
    /// flush, before component evaluation. Use it to create document parameters,
    /// register commands or change a tree in response to ordinary button input.
    ///
    /// The callback runs once in the originating window if its widget is still
    /// mounted and logically visible. Otherwise its receipt becomes `Cancelled`.
    /// Focus/capture are not transferred to replacement widgets. New operations
    /// queued during delivery wait for the next flush. Errors propagate from
    /// flush and remain in the receipt; partial application work is not rolled
    /// back or replayed. A returned callback error is an explicit application
    /// operation failure, including a nested runtime error it chooses to propagate;
    /// a caught error does not classify subsequent runtime work. Callbacks must return promptly. This is UI-thread
    /// work, never a callback-safe endpoint.
    ///
    /// The callback may call any runtime operation, including those that flush
    /// internally, such as [`Runtime::reveal_child`], [`Runtime::reveal`],
    /// [`Runtime::focus`] and [`Runtime::flush`] itself. Such a nested flush
    /// settles the tree but delivers no further deferred tasks: tasks queued
    /// before delivery started still run in order after this one, and tasks
    /// queued from inside it wait for the next flush.
    ///
    /// # Errors
    /// Returns `TaskQueueFull` at [`Runtime::MAX_TASKS`] pending operations,
    /// or the runtime phase/shutdown error, without accepting the operation.
    pub fn defer(
        &self,
        action: impl FnOnce(&mut Runtime) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError> {
        enqueue(
            &self.node.shared,
            Owner::Widget(Rc::downgrade(&self.node)),
            Box::new(action),
        )
    }

    /// Queues an application operation like [`Self::defer`], but bound to the
    /// widget's window instead of the widget: it still runs when the widget is
    /// hidden or unmounted before the next flush, and its receipt becomes
    /// `Cancelled` only when that window has closed or the runtime shut down.
    ///
    /// Use it for work the callback starts that outlives the widget's
    /// presence, such as a jump that a [`QuickPick`](crate::QuickPick)'s choose
    /// callback starts before answering [`PickChoice::Pending`](crate::PickChoice::Pending):
    /// that answer closes the picker, so an operation queued with
    /// [`Self::defer`] from inside it is cancelled whenever the picker closes
    /// before the flush delivers it. The callback runs in the window the
    /// widget was in when the operation was queued. Delivery order, errors,
    /// nested flushes and capacity match [`Self::defer`].
    ///
    /// ```
    /// use pert_ui_runtime::{
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
    ///             // this callback runs for, so the work is bound to the window.
    ///             let (result, key) = (writer.clone(), item.key.clone());
    ///             let _receipt = cx.defer_detached(move |_ui| {
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
    /// or the runtime phase/shutdown error, without accepting the operation.
    pub fn defer_detached(
        &self,
        action: impl FnOnce(&mut Runtime) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError> {
        enqueue(
            &self.node.shared,
            Owner::Window(Rc::downgrade(&self.node.window())),
            Box::new(action),
        )
    }
}
impl Runtime {
    /// Maximum number of accepted, undelivered application operations.
    pub const MAX_TASKS: usize = 64;

    /// Queues an application-owned operation, independent of a widget lifetime.
    /// Delivery, errors and capacity match [`EventContext::defer`]; only runtime
    /// shutdown cancels it before delivery. The selected window at delivery is
    /// unspecified: select one explicitly for window-specific work.
    ///
    /// # Errors
    /// Returns `TaskQueueFull` or the runtime phase/shutdown error.
    pub fn defer(
        &self,
        action: impl FnOnce(&mut Self) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError> {
        enqueue(&self.shared, Owner::Application, Box::new(action))
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
                Owner::Widget(owner) => {
                    let Some(node) = owner
                        .upgrade()
                        .filter(|n| n.mounted.get() && n.visible.get())
                    else {
                        continue;
                    };
                    Some(native_windows::NativeWindowId(node.window()))
                }
                Owner::Window(owner) => {
                    let Some(window) = owner.upgrade().filter(|w| w.mounted.get()) else {
                        continue;
                    };
                    Some(native_windows::NativeWindowId(window))
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
                Ok(()) => TaskStatus::Completed,
                Err(e) => TaskStatus::Failed(e.clone()),
            };
            task.receipt.0.set(status);
            result?;
        }
        Ok(())
    }
}
