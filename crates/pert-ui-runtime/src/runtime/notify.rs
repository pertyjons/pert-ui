//! Change notifications delivered in an event phase after the change.

use super::{EventContext, Node, Runtime, RuntimeError, Shared};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

pub(super) struct NotificationRequest {
    node: Weak<Node>,
    input: Option<Weak<InputNotificationState>>,
}
impl NotificationRequest {
    fn belongs_to_current_input(&self, shared: &Shared) -> bool {
        self.input
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|input| {
                shared
                    .notification_input
                    .borrow()
                    .as_ref()
                    .is_some_and(|current| Rc::ptr_eq(current, &input))
            })
    }
}
pub(super) struct InputNotificationState {
    pub(super) error_from_current_input: Cell<Option<bool>>,
    pub(super) dispatched: Cell<bool>,
    pub(super) command_cleanup: Cell<bool>,
}
/// Associates requests with their dispatch, without retaining past input scopes.
pub(super) struct InputNotificationGuard {
    shared: Rc<Shared>,
    state: Rc<InputNotificationState>,
    previous: Option<Rc<InputNotificationState>>,
    _origin: NotificationOriginGuard,
    previous_error: Option<bool>,
    previous_dispatch: bool,
    previous_cleanup: bool,
}
impl InputNotificationGuard {
    pub(super) fn new(shared: Rc<Shared>) -> Self {
        let state = shared
            .notification_input
            .borrow()
            .clone()
            .unwrap_or_else(|| {
                Rc::new(InputNotificationState {
                    error_from_current_input: Cell::new(None),
                    dispatched: Cell::new(false),
                    command_cleanup: Cell::new(false),
                })
            });
        let previous_error = state.error_from_current_input.replace(None);
        let previous_dispatch = state.dispatched.replace(false);
        let previous_cleanup = state.command_cleanup.replace(false);
        let previous = shared.notification_input.replace(Some(state.clone()));
        let origin = NotificationOriginGuard::new(shared.clone(), None);
        Self {
            shared,
            state,
            previous,
            _origin: origin,
            previous_error,
            previous_dispatch,
            previous_cleanup,
        }
    }
    pub(super) fn activate(&self) {
        self.shared
            .notification_origin
            .replace(Some(Rc::downgrade(&self.state)));
    }
    pub(super) fn cancel_on_error(&self, terminal: bool) -> bool {
        self.shared.notification_error_cancels_input(terminal)
    }
}
impl Drop for InputNotificationGuard {
    fn drop(&mut self) {
        self.state.error_from_current_input.set(self.previous_error);
        self.state.dispatched.set(self.previous_dispatch);
        self.state.command_cleanup.set(self.previous_cleanup);
        self.shared.notification_input.replace(self.previous.take());
    }
}

/// Application callbacks have an explicit returned-error boundary. A caught
/// nested flush failure must not classify work performed after the callback.
pub(super) struct InputErrorGuard {
    state: Option<Rc<InputNotificationState>>,
    previous: Option<bool>,
    previous_cleanup: bool,
}
impl InputErrorGuard {
    pub(super) fn new(shared: &Shared) -> Self {
        let state = shared.notification_input.borrow().clone();
        let previous = state
            .as_ref()
            .and_then(|state| state.error_from_current_input.replace(None));
        let previous_cleanup = state
            .as_ref()
            .is_some_and(|state| state.command_cleanup.replace(false));
        Self {
            state,
            previous,
            previous_cleanup,
        }
    }
}
impl Drop for InputErrorGuard {
    fn drop(&mut self) {
        if let Some(state) = &self.state {
            state.error_from_current_input.set(self.previous);
            state.command_cleanup.set(self.previous_cleanup);
        }
    }
}

/// Restores request provenance across preparation, external work and unwinding.
pub(super) struct NotificationOriginGuard {
    shared: Rc<Shared>,
    previous: Option<Weak<InputNotificationState>>,
}
impl NotificationOriginGuard {
    pub(super) fn new(shared: Rc<Shared>, origin: Option<Weak<InputNotificationState>>) -> Self {
        let previous = shared.notification_origin.replace(origin);
        Self { shared, previous }
    }
}
impl Drop for NotificationOriginGuard {
    fn drop(&mut self) {
        self.shared
            .notification_origin
            .replace(self.previous.take());
    }
}

/// Where a change to a component's own state came from.
///
/// Components that report changes to state the user can change, such as
/// [`Tabs::on_select`](crate::Tabs::on_select), name the origin with this type,
/// so an application can treat user navigation differently from the changes
/// it caused itself. A navigation history, for example, records
/// [`Self::User`] changes and ignores [`Self::Reveal`], which its own back
/// and forward buttons cause.
///
/// New origins may be added as components report them, so matches need a
/// wildcard arm.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ChangeOrigin {
    /// A gesture or key press of the user on the component itself.
    User,
    /// [`Runtime::reveal`], [`Runtime::reveal_and_focus`],
    /// [`EventContext::request_reveal`] or [`Runtime::reveal_child`] asked the
    /// component to show a child: the application's navigation, never a user
    /// gesture on the component.
    Reveal,
    /// [`Runtime::restore_workspace`] applied a saved arrangement.
    Restore,
    /// A new description changed the component's children, for example by
    /// removing the selected one, and the component chose a replacement.
    Redescription,
    /// A view moved between windows: the selected child was detached into its
    /// own native window, or a detached child returned. Either the user or the
    /// application can cause it; the component cannot tell which.
    Presentation,
    /// The application changed the state through the component's own API, such
    /// as a controller method. Components document whether they report it.
    Application,
}

/// A change of a keyed selection, reported after it took effect.
///
/// Keys identify children, never positions or labels. `previous` and
/// `selected` are `None` when nothing was selected, such as after the last
/// child was removed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SelectionChange {
    /// The key selected before the change.
    pub previous: Option<String>,
    /// The key selected after the change.
    pub selected: Option<String>,
    /// What caused the change.
    pub origin: ChangeOrigin,
}
impl SelectionChange {
    /// Describes a selection change from `previous` to `selected`.
    #[must_use]
    pub fn new(previous: Option<String>, selected: Option<String>, origin: ChangeOrigin) -> Self {
        Self {
            previous,
            selected,
            origin,
        }
    }
}

/// Asks the runtime to call one component's
/// [`Component::deliver_notifications`](crate::Component::deliver_notifications).
///
/// Obtain it from [`UpdateContext::notifier`](crate::UpdateContext::notifier)
/// or [`EventContext::notifier`] and keep it for the component's lifetime: it
/// is the way to schedule application callbacks from places that have no
/// [`EventContext`], such as
/// [`Component::restore_workspace_state_with_children`](crate::Component::restore_workspace_state_with_children)
/// and [`Component::update`](crate::Component::update). It refers to one mount
/// and never retargets a component recreated under the same key; requests for
/// a removed component are ignored. UI-thread only.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::*;
/// use std::{cell::RefCell, rc::Rc, time::Duration};
///
/// /// Reports each update to the application from an event phase.
/// struct Reporter {
///     log: Rc<RefCell<Vec<&'static str>>>,
///     notifier: Option<Notifier>,
/// }
/// impl Component for Reporter {
///     fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
///         let notifier = self.notifier.get_or_insert_with(|| cx.notifier());
///         notifier.request();
///         Ok(())
///     }
///     fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
///         self.log.borrow_mut().push("delivered");
///         Ok(())
///     }
/// }
///
/// let log = Rc::new(RefCell::new(Vec::new()));
/// let mut ui = Runtime::new();
/// ui.mount_tree(Element::new("reporter", Reporter { log: log.clone(), notifier: None }))?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(*log.borrow(), ["delivered"]);
/// # Ok::<(), RuntimeError>(())
/// ```
#[derive(Clone)]
pub struct Notifier {
    node: Weak<Node>,
}
/// Shows only whether the component is still mounted.
impl std::fmt::Debug for Notifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notifier")
            .field(
                "mounted",
                &self.node.upgrade().is_some_and(|node| node.mounted.get()),
            )
            .finish()
    }
}
impl Notifier {
    pub(crate) fn new(node: &Rc<Node>) -> Self {
        Self {
            node: Rc::downgrade(node),
        }
    }
    /// Asks for one call of the component's `deliver_notifications` at the
    /// next event-phase point: after the update and layout waves of the flush
    /// in progress have settled, or in the next flush. A request made while
    /// an input entry point such as [`Runtime::key_input`] or
    /// [`Runtime::pointer_input`] dispatches its event is delivered by a flush
    /// that entry point runs before it returns, provided its current presentation
    /// window is visible and preparation succeeds. Each request is one
    /// call, and calls across visible presentation windows run in global request order,
    /// so a component that requests once per change and delivers one change
    /// per call keeps its changes in order with other components' changes.
    /// Requests in a hidden native window remain queued until it is shown,
    /// without blocking visible windows or scheduling a polling deadline. Hidden
    /// components inside a visible window still receive callbacks. Reentrant
    /// requests join the next frontier: it runs in the same bounded flush when
    /// the preceding frontier settled presentation work, otherwise the next flush.
    /// Preparation failures
    /// retain undelivered requests; callback failures consume their request once.
    /// Input cancellation for a delivery failure is restricted to requests
    /// attributable to that dispatch. Older requests and their descendants,
    /// preflush/external work, and collective settlement errors without a unique
    /// request propagate errors without cancelling the current gesture after
    /// dispatch. A blocked terminal release still cleans up its owner, so no
    /// held action or edit is stranded. Command action errors retain the explicit
    /// failure boundary documented by [`Runtime::invoke_command`].
    /// Callable in any phase, including update, layout and workspace
    /// restoration; it writes nothing and runs no application code. Does
    /// nothing once the component is removed.
    pub fn request(&self) {
        let Some(node) = self.node.upgrade().filter(|node| node.mounted.get()) else {
            return;
        };
        node.shared
            .notifications
            .borrow_mut()
            .push(NotificationRequest {
                node: Rc::downgrade(&node),
                input: node.shared.notification_origin.borrow().clone(),
            });
    }
}

impl Shared {
    pub(super) fn notification_error_cancels_input(&self, terminal: bool) -> bool {
        self.notification_input
            .borrow()
            .as_ref()
            .is_none_or(|state| {
                !state.command_cleanup.get()
                    && (state.error_from_current_input.get().unwrap_or(true)
                        || (terminal && !state.dispatched.get()))
            })
    }
    /// Whether a mounted component in a visible window waits for delivery,
    /// dropping requests of removed components.
    pub(super) fn notifications_due(&self) -> bool {
        let mut queue = self.notifications.borrow_mut();
        queue.retain(|request| {
            request
                .node
                .upgrade()
                .is_some_and(|node| node.mounted.get())
        });
        queue
            .iter()
            .filter_map(|request| request.node.upgrade())
            .any(|node| node.window().visible.get())
    }
}

impl Runtime {
    /// Whether a mounted component in any visible window waits for delivery.
    pub(super) fn notifications_pending(&self) -> bool {
        let mut queue = self.shared.notifications.borrow_mut();
        queue.retain(|request| {
            request
                .node
                .upgrade()
                .is_some_and(|node| node.mounted.get())
        });
        queue
            .iter()
            .filter_map(|request| request.node.upgrade())
            .any(|node| node.window().visible.get())
    }
    /// Settles any visible pending frontier before returning from input.
    /// Provenance distinguishes this dispatch from older and external requests;
    /// a failed unrelated request must not cancel the current gesture.
    pub(super) fn deliver_input_notifications(&mut self) -> Result<(), RuntimeError> {
        if self.notifications_pending() {
            self.flush(self.shared.now.get())?;
        }
        Ok(())
    }
    /// Delivers one global visible-window frontier without replaying external work.
    /// A frontier that settles presentation work can produce another frontier
    /// inside the shared wave budget; otherwise new requests wait for the next flush.
    pub(super) fn dispatch_notifications(&mut self) -> Result<(usize, bool), (RuntimeError, bool)> {
        if self.shared.notification_settlement.get() || !self.shared.notifications_due() {
            return Ok((0, false));
        }
        let queued = std::mem::take(&mut *self.shared.notifications.borrow_mut());
        if queued.is_empty() {
            return Ok((0, false));
        }
        let previous = self.current_window();
        let _guard = NotificationSettlementGuard::new(self.shared.clone());
        let mut count = 0;
        let mut changed = false;
        // Settle the frontier's roots before running any callbacks. A preparation
        // failure consumes no requests; newly queued requests remain behind them.
        let prepared = (|| {
            for request in &queued {
                let _origin =
                    NotificationOriginGuard::new(self.shared.clone(), request.input.clone());
                if let Some(node) = request.node.upgrade() {
                    self.prepare_notification_recipient(&node, &mut count, &mut changed)
                        .map_err(|error| (error, request.belongs_to_current_input(&self.shared)))?;
                }
            }
            Ok(())
        })();
        let result = match prepared {
            Err(error) => {
                let mut pending = self.shared.notifications.borrow_mut();
                let mut retained = queued;
                retained.append(&mut pending);
                *pending = retained;
                Err(error)
            }
            Ok(()) => self.deliver_notification_frontier(queued, count, changed),
        };
        if previous.0.mounted.get() {
            // The identity was selected before preparation and remains in this runtime.
            let _ = self.select_window(&previous);
        } else {
            let _ = self.select_window(&self.primary_window());
        }
        result
    }
    /// A preparation may move its own recipient; follow the incarnation until
    /// its current visible presentation owner remains settled.
    fn prepare_notification_recipient(
        &mut self,
        node: &Rc<Node>,
        count: &mut usize,
        changed: &mut bool,
    ) -> Result<bool, RuntimeError> {
        loop {
            if !node.mounted.get() || !node.window().visible.get() {
                return Ok(false);
            }
            let window = node.window();
            self.select_window(&super::native_windows::NativeWindowId(window.clone()))?;
            if self.notification_presentation_pending() {
                *changed = true;
                *count += self.settle_notification_presentation()?;
            }
            if !node.mounted.get() {
                return Ok(false);
            }
            if Rc::ptr_eq(&window, &node.window()) && !self.notification_presentation_pending() {
                return Ok(window.visible.get());
            }
        }
    }
    fn deliver_notification_frontier(
        &mut self,
        queued: Vec<NotificationRequest>,
        mut count: usize,
        mut changed: bool,
    ) -> Result<(usize, bool), (RuntimeError, bool)> {
        let mut retained = Vec::new();
        let mut first_error = None;
        let mut error_from_current_input = false;
        let mut preparation_failed = false;
        for (index, request) in queued.iter().enumerate() {
            let _origin = NotificationOriginGuard::new(self.shared.clone(), request.input.clone());
            let Some(node) = request.node.upgrade().filter(|node| node.mounted.get()) else {
                continue;
            };
            match self.prepare_notification_recipient(&node, &mut count, &mut changed) {
                Ok(true) => {}
                Ok(false) => {
                    if node.mounted.get() {
                        retained.push(index);
                    }
                    continue;
                }
                Err(error) => {
                    retained.extend(index..queued.len());
                    error_from_current_input |= request.belongs_to_current_input(&self.shared);
                    first_error.get_or_insert(error);
                    preparation_failed = true;
                    break;
                }
            }
            let Some(entry) = self
                .entries
                .get_mut(&node.serial)
                .filter(|entry| Rc::ptr_eq(&entry.node, &node))
            else {
                continue;
            };
            let current_input = request.belongs_to_current_input(&self.shared);
            let result = entry
                .component
                .deliver_notifications(&mut EventContext { node: node.clone() });
            if let Err(error) = result {
                // Reporting precedence and input cleanup are independent:
                // an earlier unrelated error cannot hide this input's failure.
                error_from_current_input |= current_input;
                first_error.get_or_insert(error);
            }
        }
        retained.sort_unstable();
        let mut retained_indices = retained.into_iter().peekable();
        let mut retained: Vec<_> = queued
            .into_iter()
            .enumerate()
            .filter_map(|(index, request)| {
                if retained_indices.peek() == Some(&index) {
                    retained_indices.next();
                    Some(request)
                } else {
                    None
                }
            })
            .collect();
        {
            let mut pending = self.shared.notifications.borrow_mut();
            retained.append(&mut pending);
            *pending = retained;
        }
        // Callback writes in any visible root settle before returning. Hidden
        // roots retain presentation work and their notification requests.
        let _origin = NotificationOriginGuard::new(self.shared.clone(), None);
        if !preparation_failed {
            'settle: loop {
                let mut serviced = false;
                for window in self.native_windows() {
                    if !window.0.mounted.get() || !window.0.visible.get() {
                        continue;
                    }
                    self.select_window(&window)
                        .map_err(|error| (error, false))?;
                    if self.notification_presentation_pending() {
                        serviced = true;
                        changed = true;
                        match self.settle_notification_presentation() {
                            Ok(updated) => count += updated,
                            Err(error) => {
                                first_error.get_or_insert(error);
                                break 'settle;
                            }
                        }
                    }
                }
                if !serviced {
                    break;
                }
            }
        }
        if let Some(error) = first_error {
            return Err((error, error_from_current_input));
        }
        Ok((count, changed && self.shared.notifications_due()))
    }
}

/// Restores the preparation mode even if component code unwinds.
struct NotificationSettlementGuard {
    shared: Rc<Shared>,
    previous: bool,
}
impl NotificationSettlementGuard {
    fn new(shared: Rc<Shared>) -> Self {
        let previous = shared.notification_settlement.replace(true);
        Self { shared, previous }
    }
}
impl Drop for NotificationSettlementGuard {
    fn drop(&mut self) {
        self.shared.notification_settlement.set(self.previous);
    }
}
