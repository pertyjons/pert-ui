//! In-layout feedback and bounded, scheduled nonmodal notifications.
use crate::activation::{ActivationState, key_result};
use crate::choice_controls::{focus_ring, rounded};
use crate::layout::rect;
use crate::signal::ReceiptStatus;
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::{Rc, Weak},
    time::Duration,
};

/// Persistent feedback in ordinary layout and focus order, with standard spacing.
///
/// The message and optional action elements are application content. No automatic
/// expiry, focus transfer or timer occurs. A visibility signal can dismiss and
/// later restore the same retained content; omission keeps it visible.
///
/// Describing the banner again applies a changed message, which is also its
/// accessible name, and a changed or removed visibility signal; actions
/// reconcile by key. The message is a [`TextSource`]: a binding updates the
/// shown message and the name without describing the banner again.
#[must_use]
pub struct Banner {
    /// The message, which names the banner; its label child draws it.
    message: Caption,
    content: Container,
    visible: Option<ReadSignal<bool>>,
    shown: bool,
}
impl Banner {
    /// Creates informational feedback with a message and optional ordinary actions.
    /// Use an empty iterator for a message without actions.
    pub fn new(message: impl Into<TextSource>, actions: impl IntoIterator<Item = Element>) -> Self {
        let message = message.into();
        let mut children = vec![Element::new("message", Label::new(message.clone())).structural()];
        children.extend(actions);
        Self {
            message: Caption::new(message),
            content: Container::new(Element::new("content", Wrap::new(children)).structural())
                .background_role(ColorRole::Surface)
                .padding(MetricRole::GroupPadding),
            visible: None,
            shown: true,
        }
    }
    /// Follows local visibility. False removes the banner's layout extent and
    /// hides/cancels descendant actions while preserving their retained state.
    pub fn visible_signal(mut self, visible: impl Into<ReadSignal<bool>>) -> Self {
        let visible: ReadSignal<bool> = visible.into();
        self.visible = Some(visible);
        self
    }
}
impl Component for Banner {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // The message label and the actions are reconciled children.
        let mut invalidation = None;
        if !ReadSignal::same_optional(&self.visible, &next.visible) {
            self.visible = next.visible;
            invalidation = Some(Invalidation::Layout);
        }
        if self.message.reconcile(next.message) {
            invalidation.get_or_insert(Invalidation::Paint);
        }
        invalidation
    }
    fn inspect(&self) -> ComponentDetails {
        // A hidden banner has no extent, so the container's padding describes no
        // box; reporting it would read as padding larger than the banner.
        let mut details = if self.shown {
            self.content.inspect()
        } else {
            ComponentDetails::default()
        };
        details.semantics = Some(
            Semantics::new(SemanticRole::Alert)
                .name(self.message.text())
                .expanded(self.shown),
        );
        details
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.content.take_children()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.message.update_paint(cx)?;
        self.shown = self
            .visible
            .as_ref()
            .map(|s| cx.read(s))
            .transpose()?
            .unwrap_or(true);
        self.content.update(cx)
    }
    fn child_visible(&self, _: &str) -> bool {
        self.shown
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        if self.shown {
            self.content.measure(cx, a)
        } else {
            Ok(Size::ZERO)
        }
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.content.arrange(cx, s)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if self.shown {
            self.content.paint(p)
        } else {
            Ok(())
        }
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
}

type ToastAction = Box<dyn FnMut(&mut EventContext) -> Result<(), RuntimeError>>;
/// One transient plain-text notification, optionally with a user action.
///
/// Posted through [`ToastQueue::push`]. Each toast expires after five seconds of
/// unpaused elapsed runtime time after first presentation. It can always be dismissed. Callbacks run only
/// on explicit activation, after removal; failure propagates without replay.
///
/// The message and action label are [`TextSource`]s; only their plain text is
/// shown. The host reads a bound message or label while the toast is visible,
/// so a progress message can change without posting another toast; a changed
/// text relayouts the stack and keeps an armed press, focus selection and the
/// remaining lifetime. Toasts are identified by their admission, never by
/// their text, so equal messages are separate toasts. A bound text longer than
/// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES)
/// fails the host's update with [`RuntimeError::TextEdit`] and keeps the last
/// shown text.
#[must_use]
pub struct Toast {
    message: TextSource,
    action: Option<(TextSource, ToastAction)>,
}
impl Toast {
    /// Creates a message with the default five-second presentation lifetime.
    pub fn new(message: impl Into<TextSource>) -> Self {
        Self {
            message: message.into(),
            action: None,
        }
    }
    /// Adds one explicit action. Use dialogs for authoritative task responses;
    /// a toast can expire or disappear with its owner without invoking the action.
    pub fn action(
        mut self,
        label: impl Into<TextSource>,
        action: impl FnMut(&mut EventContext) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.action = Some((label.into(), Box::new(action)));
        self
    }
}
/// Observable state of one admitted toast; terminal states never change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToastStatus {
    /// Waiting for a visible slot; lifetime has not started.
    Pending,
    /// Visible, possibly paused by hover or keyboard focus.
    Visible,
    /// Its presentation lifetime elapsed.
    Expired,
    /// Explicitly dismissed without invoking its action.
    Dismissed,
    /// Its action completed successfully.
    ActionSucceeded,
    /// Its action returned an error, which also propagated through input routing.
    ActionFailed,
    /// Its owning host was removed or its runtime shut down.
    OwnerRemoved,
}
/// UI-local receipt for one toast incarnation.
///
/// [`Self::status`] reads the owned status, which stays readable after host or
/// runtime destruction, where [`ToastStatus::OwnerRemoved`] reports a toast
/// that ended with its owner. [`Self::status_binding`] mirrors it as a binding
/// while the runtime lives. Retaining a receipt does not keep the toast or host
/// alive.
///
/// Receipts compare equal when they report the same toast.
#[derive(Clone)]
#[must_use]
pub struct ToastReceipt(ReceiptStatus<ToastStatus>);
impl PartialEq for ToastReceipt {
    fn eq(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}
impl Eq for ToastReceipt {}
/// Shows the latest status, as [`ToastReceipt::status`] reads it.
impl std::fmt::Debug for ToastReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToastReceipt")
            .field("status", &self.status())
            .finish()
    }
}
impl ToastReceipt {
    /// Reads the latest state without polling, scheduling or holding live UI
    /// handles, including after runtime shutdown.
    #[must_use]
    pub fn status(&self) -> ToastStatus {
        let status = self.0.get();
        if matches!(status, ToastStatus::Pending | ToastStatus::Visible)
            && matches!(self.0.binding().get(), Err(RuntimeError::Closed))
        {
            ToastStatus::OwnerRemoved
        } else {
            status
        }
    }
    /// The same status as a binding, so a component that reads it during
    /// update is invalidated when the toast becomes visible or ends. Like every
    /// binding it reads `Err(RuntimeError::Closed)` after runtime shutdown; use
    /// [`Self::status`] then.
    pub fn status_binding(&self) -> ReadSignal<ToastStatus> {
        self.0.binding()
    }
}
struct Entry {
    id: u64,
    toast: Toast,
    receipt: ToastReceipt,
    remaining: Duration,
}
struct Queue {
    entries: VecDeque<Entry>,
    next: u64,
    /// The mounted host's token; empty while no host is mounted.
    owner: Weak<()>,
    closed: bool,
}
impl Drop for Queue {
    fn drop(&mut self) {
        for entry in &self.entries {
            entry.receipt.0.set(ToastStatus::OwnerRemoved);
        }
    }
}
/// Cloneable UI-local notification sender with bounded FIFO retention.
///
/// Up to three visible and sixteen pending toasts are retained. Overflow returns
/// `ToastFull`, without dropping an admitted toast. Pending items do not age.
/// One [`ToastHost`] owns presentation. No worker thread, callback-safe endpoint or background
/// executor is created; posting invalidates only that host. Removal closes the
/// queue and publishes `OwnerRemoved` to surviving receipts, without callbacks.
#[derive(Clone)]
#[must_use]
pub struct ToastQueue {
    queue: Rc<RefCell<Queue>>,
    revision: Signal<u64>,
}
/// Shows how many toasts the queue retains, whether a host owns it and whether
/// the host's removal closed it. A queue borrowed by its host reports only that.
impl std::fmt::Debug for ToastQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("ToastQueue");
        match self.queue.try_borrow() {
            Ok(queue) => out
                .field("retained", &queue.entries.len())
                .field("owned", &(queue.owner.strong_count() > 0))
                .field("closed", &queue.closed),
            Err(_) => out.field("borrowed", &true),
        };
        out.finish()
    }
}
impl ToastQueue {
    /// Maximum simultaneous visible notifications.
    pub const MAX_VISIBLE: usize = 3;
    /// Maximum additional notifications waiting for presentation.
    pub const MAX_PENDING: usize = 16;
    /// Creates an empty queue; it may receive items before its host is mounted.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    pub fn new(ui: &impl crate::StateOwner) -> Self {
        Self {
            queue: Rc::new(RefCell::new(Queue {
                entries: VecDeque::new(),
                next: 0,
                owner: Weak::new(),
                closed: false,
            })),
            revision: ui.signal(0),
        }
    }
    /// Admits one toast and returns its receipt.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; [`RuntimeError::TextEdit`]
    /// with [`TextEditError::TooLarge`](crate::TextEditError::TooLarge) for a fixed
    /// message or action label above
    /// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES) (a bound
    /// one is checked when the host reads it);
    /// `BorrowConflict` when called from inside the queue's own processing; `Unmounted`
    /// after the host was removed; `ToastFull` at the visible plus pending capacity; and
    /// `Exhausted` when toast identities run out. A rejected toast is dropped; existing
    /// items and receipts are intact.
    pub fn push(&self, toast: Toast) -> Result<ToastReceipt, RuntimeError> {
        self.revision.read_only().shared().writable()?;
        let too_large = |text: &TextSource| {
            text.fixed()
                .is_some_and(|text| text.plain_text().len() > crate::TextEditError::MAX_TEXT_BYTES)
        };
        if too_large(&toast.message)
            || toast
                .action
                .as_ref()
                .is_some_and(|(label, _)| too_large(label))
        {
            return Err(crate::TextEditError::TooLarge.into());
        }
        let mut q = self
            .queue
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?;
        if q.closed {
            return Err(RuntimeError::Unmounted);
        }
        if q.entries.len() >= Self::MAX_VISIBLE + Self::MAX_PENDING {
            return Err(RuntimeError::ToastFull);
        }
        let id = q.next.checked_add(1).ok_or(RuntimeError::Exhausted)?;
        self.revision.set(id)?;
        q.next = id;
        let receipt = ToastReceipt(ReceiptStatus::new(
            self.revision.read_only().shared().clone(),
            ToastStatus::Pending,
        ));
        q.entries.push_back(Entry {
            id,
            toast,
            receipt: receipt.clone(),
            remaining: Duration::from_secs(5),
        });
        Ok(receipt)
    }
}
/// Wraps application content with a bottom-right notification stack.
///
/// The stack is five themed [`MetricRole::ControlWidth`] units wide (320 logical
/// pixels at standard density), capped at the viewport width. Messages wrap
/// within that width and each toast's height follows its content.
///
/// A toast's action and Dismiss sit at its trailing edge, each as wide as its
/// own label plus the themed padding on both sides and normally at least one
/// `ControlWidth`, separated by an `ItemGap`. When the viewport is too narrow
/// for both controls, they stack below the message and wrap their labels within
/// the viewport's available width.
///
/// Admission never steals focus. Tab reaches the visible stack; unmodified
/// Up/Down selects a toast and keeps stepping while held, unmodified Enter/Space
/// activates its action (or dismisses a message-only toast), and unmodified
/// Delete dismisses it. Modified keys are left to command bindings. Selection follows the toast's identity when earlier
/// entries disappear. Removing the selected entry selects its first surviving
/// following toast, then nearest surviving preceding toast, then first newly
/// visible toast. This also applies when several toasts expire at once.
/// Pointer controls offer the action and Dismiss.
/// Each button uses ButtonHover only while its own target is hovered.
/// Hover over a toast or keyboard focus in the stack pauses all visible lifetimes.
/// Transparent gaps between toasts pass pointer input to content behind them.
/// Wheel input scrolls overflowing content; fitting content and unchanged offsets
/// yield to scrolling ancestors.
/// Hidden/occluded hosts schedule no presentation work. Already-started lifetimes
/// continue in elapsed runtime time while hidden and expired items retire on resume;
/// pending items do not age. Hover/focus pauses are accounted at their input event
/// time. Deadlines use the shared deterministic clock.
///
/// The host claims its queue when it mounts, not when it is constructed, so a
/// tree containing it can be described again with the same queue: the new
/// description is applied to the mounted host, and a changed
/// [`Self::dismiss_label`] takes effect. A description with a different queue
/// closes the old one as a removal would and presents the new one. A second host
/// mounted with the same queue shows nothing and fails its updates: with
/// [`RuntimeError::DuplicateKey`] while the first host is mounted, and with
/// [`RuntimeError::Unmounted`] once the first host's removal has closed the
/// queue, as for any host mounted with a closed queue.
#[must_use]
pub struct ToastHost {
    queue: ToastQueue,
    content: Option<Element>,
    /// The application's dismiss label, or `None` for the string table's.
    dismiss: Option<TextSource>,
}
impl ToastHost {
    /// Describes the queue's single presentation owner around `content`. The
    /// queue is claimed when the host mounts.
    pub fn new(queue: ToastQueue, content: Element) -> Self {
        Self {
            queue,
            content: Some(content),
            dismiss: None,
        }
    }
    /// Replaces the label of every toast's dismiss control with a
    /// [`TextSource`]: fixed text, a binding the label follows, or a
    /// [`StringRole`](crate::StringRole) of the installed string table; only
    /// its plain text is used. By default the installed
    /// [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::ToastDismiss`](crate::StringRole::ToastDismiss), `"Dismiss"`
    /// in English, and a replaced table relabels it. The control keeps its
    /// behavior and sizing rules; re-describing the host applies a changed
    /// label, while the same binding or role keeps the text already read. A
    /// bound or role label is read in the stack's update. Inspection reports
    /// it as the stack's `dismiss_label` property.
    ///
    /// ```
    /// use pert_ui_runtime::{Element, Label, Runtime, Toast, ToastHost, ToastQueue};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let queue = ToastQueue::new(&ui);
    /// let host = ToastHost::new(queue.clone(), Element::new("body", Label::new("Innehåll")))
    ///     .dismiss_label("Stäng");
    /// ui.mount_tree(Element::new("host", host))?;
    /// let _receipt = queue.push(Toast::new("Sparat"))?;
    /// ui.flush(Duration::ZERO)?;
    /// let label = ui.inspect().components.into_iter().find(|c| c.key == "toasts")
    ///     .and_then(|c| c.details.properties.get("dismiss_label").cloned());
    /// assert_eq!(label, Some(pert_ui_runtime::InspectValue::Text("Stäng".into())));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn dismiss_label(mut self, label: impl Into<TextSource>) -> Self {
        self.dismiss = Some(label.into());
        self
    }
}
fn content_wrapper(content: Element) -> Container {
    Container::new(content)
}
impl Component for ToastHost {
    fn take_children(&mut self) -> Vec<Element> {
        let Some(content) = self.content.take() else {
            return Vec::new();
        };
        vec![
            Element::new("content", content_wrapper(content)).structural(),
            Element::new(
                "toasts",
                ToastStack {
                    queue: self.queue.clone(),
                    token: Rc::new(()),
                    label: self
                        .dismiss
                        .take()
                        .unwrap_or_else(|| crate::StringRole::ToastDismiss.into()),
                    dismiss: String::new(),
                    items: Vec::new(),
                    prepared: RefCell::new(Vec::new()),
                    last: None,
                    paused: false,
                    hovered: false,
                    focused: false,
                    selected: None,
                    offset: Cell::new(0.0),
                    height: Cell::new(0.0),
                    input: ActivationState::default(),
                    pressed: None,
                    hovered_control: None,
                },
            ),
        ]
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "toasts").then_some(SurfaceKind::Notification)
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        cx.measure(0, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::from_size(s))?;
        let v = cx.measurement().viewport();
        let t = cx.measure(1, Constraints::new(Some(v.width()), Some(v.height()))?)?;
        cx.place(1, Rect::from_size(t))
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
}
/// The plain text a visible toast shows from `source`: its current text, or
/// `previous` (empty for a new toast) when that text exceeds the admission
/// limit, which sets `oversized`.
fn shown_text(
    cx: &mut UpdateContext,
    source: &TextSource,
    previous: Option<&str>,
    oversized: &mut bool,
) -> Result<String, RuntimeError> {
    let text = source.read(cx)?.plain_text();
    if text.len() > crate::TextEditError::MAX_TEXT_BYTES {
        *oversized = true;
        return Ok(previous.unwrap_or_default().to_owned());
    }
    Ok(text)
}
#[derive(PartialEq)]
struct VisibleToast {
    id: u64,
    message: String,
    action: Option<String>,
}
struct PreparedToast {
    text: TextLayout,
    action: TextLayout,
    dismiss: TextLayout,
    rect: Rect,
    action_rect: Rect,
    dismiss_rect: Rect,
}
struct ToastStack {
    queue: ToastQueue,
    /// Identifies this mount as the queue's owner.
    token: Rc<()>,
    /// The application's dismiss label, or the string table's role.
    label: TextSource,
    /// The dismiss label shown, resolved in the update.
    dismiss: String,
    items: Vec<VisibleToast>,
    prepared: RefCell<Vec<PreparedToast>>,
    last: Option<Duration>,
    paused: bool,
    hovered: bool,
    focused: bool,
    selected: Option<u64>,
    offset: Cell<f32>,
    height: Cell<f32>,
    input: ActivationState,
    pressed: Option<(u64, bool)>,
    hovered_control: Option<(u64, bool)>,
}
impl Drop for ToastStack {
    fn drop(&mut self) {
        // Normally already released by `unmounted`; a description that never
        // mounted owns nothing.
        self.release();
    }
}
impl ToastStack {
    /// Takes the queue over unless another live mount owns it or an earlier
    /// host's removal closed it.
    fn claim(&self) -> Result<(), RuntimeError> {
        let mut q = self
            .queue
            .queue
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?;
        match q.owner.upgrade() {
            Some(owner) if Rc::ptr_eq(&owner, &self.token) => Ok(()),
            Some(_) => Err(RuntimeError::DuplicateKey),
            None if q.closed => Err(RuntimeError::Unmounted),
            None => {
                q.owner = Rc::downgrade(&self.token);
                Ok(())
            }
        }
    }
    /// Closes the queue and ends its toasts with `OwnerRemoved`, if this mount
    /// owns it.
    fn release(&self) {
        let entries = {
            let Ok(mut q) = self.queue.queue.try_borrow_mut() else {
                return;
            };
            if !q
                .owner
                .upgrade()
                .is_some_and(|owner| Rc::ptr_eq(&owner, &self.token))
            {
                return;
            }
            q.owner = Weak::new();
            q.closed = true;
            std::mem::take(&mut q.entries)
        };
        for entry in entries {
            entry.receipt.0.set(ToastStatus::OwnerRemoved);
        }
    }
    fn account(&mut self, now: Duration) {
        if !self.paused
            && let Some(last) = self.last
        {
            let elapsed = now.saturating_sub(last);
            for entry in self
                .queue
                .queue
                .borrow_mut()
                .entries
                .iter_mut()
                .take(ToastQueue::MAX_VISIBLE)
            {
                if entry.receipt.status() == ToastStatus::Visible {
                    entry.remaining = entry.remaining.saturating_sub(elapsed);
                }
            }
        }
        self.last = Some(now);
    }
    fn activate(
        &mut self,
        id: u64,
        action: bool,
        cx: &mut EventContext,
    ) -> Result<(), RuntimeError> {
        let entry = {
            let mut q = self
                .queue
                .queue
                .try_borrow_mut()
                .map_err(|_| RuntimeError::BorrowConflict)?;
            q.entries
                .iter()
                .position(|e| e.id == id)
                .and_then(|i| q.entries.remove(i))
        };
        let Some(mut entry) = entry else {
            return Ok(());
        };
        cx.invalidate(Invalidation::Layout);
        if action && let Some((_, callback)) = &mut entry.toast.action {
            let result = callback(cx);
            entry.receipt.0.set(if result.is_ok() {
                ToastStatus::ActionSucceeded
            } else {
                ToastStatus::ActionFailed
            });
            result
        } else {
            entry.receipt.0.set(ToastStatus::Dismissed);
            Ok(())
        }
    }
    fn hit(&self, cx: &EventContext, e: PointerEvent) -> Option<(u64, bool)> {
        if !cx.pointer_inside() {
            return None;
        }
        let point = cx.to_local(e.position())?;
        self.items
            .iter()
            .zip(self.prepared.borrow().iter())
            .find_map(|(item, p)| {
                if p.dismiss_rect.contains(point) {
                    Some((item.id, false))
                } else if item.action.is_some() && p.action_rect.contains(point) {
                    Some((item.id, true))
                } else {
                    None
                }
            })
    }
    fn prepare(
        &self,
        cx: &MeasureContext<'_>,
        width: f32,
    ) -> Result<Vec<PreparedToast>, RuntimeError> {
        let pad = cx.metric(MetricRole::ControlPaddingX).min(width / 2.0);
        let gap = cx.metric(MetricRole::GroupPadding);
        let button_gap = cx.metric(MetricRole::ItemGap);
        let mut top = -self.offset.get();
        let mut prepared = Vec::new();
        for item in &self.items {
            let text = cx.text(
                &item.message,
                Some((width - 2.0 * pad).max(1.0)),
                TextStyle::Body,
            )?;
            // Each control is as wide as its own label plus the themed padding on
            // both sides and normally at least one ControlWidth. They share the
            // trailing edge when space permits; narrower toasts stack them.
            let inner = (width - 2.0 * pad).max(0.0);
            let dismiss = cx.text(
                &self.dismiss,
                Some((inner - 2.0 * pad).max(1.0)),
                TextStyle::Body,
            )?;
            let button = |label: &TextLayout| {
                (label.size().width() + 2.0 * pad).max(cx.metric(MetricRole::ControlWidth))
            };
            let dismiss_width = button(&dismiss).min(inner);
            // The action keeps what Dismiss, the gap and the margins leave it.
            let available = (inner - button_gap - dismiss_width).max(0.0);
            let full_action = cx.text(
                item.action.as_deref().unwrap_or(""),
                Some((inner - 2.0 * pad).max(1.0)),
                TextStyle::Body,
            )?;
            let stacked = item.action.is_some()
                && button(&full_action) + button_gap + button(&dismiss) > inner;
            let action = if stacked {
                full_action
            } else {
                cx.text(
                    item.action.as_deref().unwrap_or(""),
                    Some((available - 2.0 * pad).max(1.0)),
                    TextStyle::Body,
                )?
            };
            let action_width = if item.action.is_some() {
                button(&action).min(available)
            } else {
                0.0
            };
            let controls = cx
                .metric(MetricRole::ControlHeight)
                .max(action.size().height() + pad)
                .max(dismiss.size().height() + pad);
            let y = top + pad + text.size().height();
            let height = pad
                + text.size().height()
                + controls * if stacked { 2.0 } else { 1.0 }
                + if stacked { button_gap } else { 0.0 }
                + pad;
            let dismiss_x = (width - pad - dismiss_width).max(pad);
            let action_x = (dismiss_x - button_gap - action_width).max(pad);
            prepared.push(PreparedToast {
                text,
                action,
                dismiss,
                rect: rect(0.0, top, width, height)?,
                action_rect: if stacked {
                    rect(pad, y, inner, controls)?
                } else {
                    rect(action_x, y, action_width, controls)?
                },
                dismiss_rect: if stacked {
                    rect(pad, y + controls + button_gap, inner, controls)?
                } else {
                    rect(dismiss_x, y, dismiss_width, controls)?
                },
            });
            top += height + gap;
        }
        Ok(prepared)
    }
}
impl Component for ToastStack {
    fn reconcile(&mut self, mut next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        if !Rc::ptr_eq(&self.queue.queue, &next.queue.queue) {
            self.release();
            // `next` keeps the old queue, which its token never owned, so
            // dropping it releases nothing.
            std::mem::swap(&mut self.queue, &mut next.queue);
            // A refused claim is reported by the next update.
            let _ = self.claim();
            // Local toast IDs restart in every queue, so a press or held key
            // armed on the old queue's toast would match the new one's.
            self.pressed = None;
            self.input.reset();
            self.items.clear();
            self.selected = None;
            // The hovered control names an old toast ID too; the layout this
            // requests refreshes hover from the pointer's current position.
            self.hovered_control = None;
            invalidation = Some(Invalidation::Layout);
        }
        if !self.label.same_as(&next.label) {
            // The update resolves the shown label again.
            self.label = std::mem::take(&mut next.label);
            invalidation = Some(Invalidation::Layout);
        }
        invalidation
    }
    fn mounted(&mut self) {
        // A refused claim is reported by the first update.
        let _ = self.claim();
    }
    fn unmounted(&mut self) {
        self.release();
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.validation.interactive = !self.items.is_empty();
        d.properties.insert(
            "visible_count".into(),
            InspectValue::Number(self.items.len() as f64),
        );
        d.properties.insert(
            "scroll_offset".into(),
            InspectValue::Number(self.offset.get().into()),
        );
        d.properties
            .insert("paused".into(), InspectValue::Bool(self.paused));
        d.properties.insert(
            "dismiss_label".into(),
            InspectValue::Text(self.dismiss.clone()),
        );
        d.semantics = Some(
            Semantics::new(SemanticRole::Notifications)
                .value(SemanticValue::Number {
                    value: self.items.len() as f64,
                    minimum: Some(0.0),
                    maximum: None,
                })
                .enabled_actions((!self.items.is_empty()).then_some(SemanticAction::Dismiss)),
        );
        d
    }
    fn focusable(&self) -> bool {
        !self.items.is_empty()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let dismiss = self.label.read(cx)?.plain_text();
        if dismiss != self.dismiss {
            self.dismiss = dismiss;
            cx.request_layout();
        }
        // Checked on every update: a host that found the queue taken shows
        // nothing and keeps reporting why.
        if let Err(error) = self.claim() {
            if !self.items.is_empty() {
                self.items.clear();
                cx.request_layout();
            }
            return Err(error);
        }
        cx.read(&self.queue.revision)?;
        self.account(cx.now());
        // Register hover/focus dependencies in addition to event-time accounting.
        self.hovered = cx.hovered();
        self.focused = cx.focused();
        let paused = self.hovered || self.focused;
        let mut q = self
            .queue
            .queue
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?;
        let mut removed = Vec::new();
        for i in (0..q.entries.len()).rev() {
            if q.entries[i].remaining.is_zero()
                && let Some(e) = q.entries.remove(i)
            {
                e.receipt.0.set(ToastStatus::Expired);
                removed.push(e);
            }
        }
        let mut items = Vec::new();
        let mut oversized = false;
        for e in q.entries.iter_mut().take(ToastQueue::MAX_VISIBLE) {
            e.receipt.0.set(ToastStatus::Visible);
            if !paused {
                cx.wake_after(e.remaining)?;
            }
            // Bound texts are read while visible; an oversized one keeps the
            // text shown before.
            let previous = self.items.iter().find(|item| item.id == e.id);
            let message = shown_text(
                cx,
                &e.toast.message,
                previous.map(|item| item.message.as_str()),
                &mut oversized,
            )?;
            let action = match &e.toast.action {
                Some((label, _)) => Some(shown_text(
                    cx,
                    label,
                    previous.and_then(|item| item.action.as_deref()),
                    &mut oversized,
                )?),
                None => None,
            };
            items.push(VisibleToast {
                id: e.id,
                message,
                action,
            });
        }
        drop(q);
        drop(removed);
        let structure = |items: &[VisibleToast]| -> Vec<(u64, bool)> {
            items
                .iter()
                .map(|item| (item.id, item.action.is_some()))
                .collect()
        };
        if structure(&self.items) == structure(&items) {
            // Only texts changed: relayout, keeping input and selection.
            if self.items != items {
                self.items = items;
                cx.request_layout();
            }
        } else {
            let neighbor = self
                .selected
                .and_then(|id| self.items.iter().position(|item| item.id == id))
                .and_then(|index| {
                    self.items[index + 1..]
                        .iter()
                        .chain(self.items[..index].iter().rev())
                        .find(|old| items.iter().any(|item| item.id == old.id))
                        .map(|item| item.id)
                });
            self.selected = self
                .selected
                .filter(|id| items.iter().any(|item| item.id == *id))
                .or(neighbor)
                .or_else(|| items.first().map(|item| item.id));
            self.items = items;
            self.pressed = None;
            self.input.reset();
            cx.cancel_input();
            cx.request_layout();
        }
        self.input.focused = cx.focused();
        self.paused = paused;
        self.last = Some(cx.now());
        if oversized {
            return Err(crate::TextEditError::TooLarge.into());
        }
        Ok(())
    }

    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        if self.items.is_empty() {
            return Ok(Size::ZERO);
        }
        let width = (cx.metric(MetricRole::ControlWidth) * 5.0).min(cx.viewport().width());
        let p = self.prepare(cx, width)?;
        let height = p
            .last()
            .map(|p| {
                let [_, y, _, h] = p.rect.values();
                y + h + self.offset.get()
            })
            .unwrap_or(0.0);
        Ok(Size::new(width, height.min(cx.viewport().height()))?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.height.set(s.height());
        let p = self.prepare(cx.measurement(), s.width())?;
        let total = p
            .last()
            .map(|p| {
                let [_, y, _, h] = p.rect.values();
                y + h + self.offset.get()
            })
            .unwrap_or(0.0);
        let previous_offset = self.offset.get();
        let offset = previous_offset.clamp(0.0, (total - s.height()).max(0.0));
        self.offset.set(offset);
        *self.prepared.borrow_mut() = if offset == previous_offset {
            p
        } else {
            self.prepare(cx.measurement(), s.width())?
        };
        Ok(())
    }
    fn hit_test(&self, point: Point) -> bool {
        self.prepared
            .borrow()
            .iter()
            .any(|toast| toast.rect.contains(point))
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        self.account(cx.now());
        let previous_hover = (self.hovered, self.hovered_control, self.paused);
        self.hovered = e.kind() != PointerKind::Leave
            && cx.pointer_inside()
            && cx
                .to_local(e.position())
                .is_some_and(|point| self.hit_test(point));
        self.hovered_control = if self.hovered && !e.is_outside() {
            self.hit(cx, e)
        } else {
            None
        };
        self.paused = self.hovered || self.focused;
        if previous_hover != (self.hovered, self.hovered_control, self.paused) {
            cx.invalidate(Invalidation::Paint);
        }
        if let PointerKind::Scroll(delta) = e.kind() {
            let total = self
                .prepared
                .borrow()
                .last()
                .map(|toast| toast.rect.y() + toast.rect.height() + self.offset.get())
                .unwrap_or(0.0);
            let next =
                (self.offset.get() + delta.y()).clamp(0.0, (total - self.height.get()).max(0.0));
            if next == self.offset.get() {
                return Ok(InputOutcome::Ignored);
            }
            self.offset.set(next);
            cx.invalidate(Invalidation::Layout);
            return Ok(InputOutcome::Handled);
        }
        if matches!(e.kind(),PointerKind::Press(b)|PointerKind::Release(b) if b != PointerButton::Primary)
        {
            return Ok(InputOutcome::Ignored);
        }
        let activate = self.input.pointer(cx, e)?;
        if e.kind() == PointerKind::Press(PointerButton::Primary) {
            self.pressed = self.hit(cx, e);
        }
        if e.kind() == PointerKind::Release(PointerButton::Primary)
            && let Some((id, action)) = self
                .pressed
                .take()
                .filter(|hit| activate && self.hit(cx, e).as_ref() == Some(hit))
        {
            self.activate(id, action, cx)?;
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(e);
        if self.items.is_empty() {
            return Ok(InputOutcome::Ignored);
        }
        // Modified keys are left to command bindings.
        if matches!(
            control,
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::Delete
        ) && keys.chord_modifiers(e) == Modifiers::default()
        {
            // Holding an arrow keeps stepping; Delete dismisses once per press.
            if e.transition == KeyTransition::Pressed
                || (e.transition == KeyTransition::Repeated && control != KeyCode::Delete)
            {
                self.input.reset();
                self.pressed = None;
                cx.release_pointer();
                let current = self
                    .selected
                    .and_then(|id| self.items.iter().position(|item| item.id == id))
                    .unwrap_or(0);
                let index = match control {
                    KeyCode::ArrowUp => (current + self.items.len() - 1) % self.items.len(),
                    KeyCode::ArrowDown => (current + 1) % self.items.len(),
                    _ => {
                        self.activate(self.items[current].id, false, cx)?;
                        current
                    }
                };
                self.selected = Some(self.items[index].id);
                if let Some(p) = self.prepared.borrow().get(index) {
                    let [_, y, _, h] = p.rect.values();
                    if y < 0.0 {
                        self.offset.set((self.offset.get() + y).max(0.0));
                    }
                    if y + h > self.height.get() {
                        self.offset
                            .set((self.offset.get() + y + h - self.height.get()).max(0.0));
                    }
                }
                cx.invalidate(Invalidation::Layout);
            }
            return Ok(InputOutcome::Handled);
        }
        if matches!(control, KeyCode::Enter | KeyCode::Space | KeyCode::Escape) {
            let result = key_result(control, self.input.pressed(), cx.modifiers());
            if control == KeyCode::Escape && result == InputOutcome::Handled {
                self.pressed = None;
            }
            if self.input.keyboard(cx, e, control)
                && let Some(id) = self.selected
            {
                self.activate(id, true, cx)?;
            }
            return Ok(result);
        }
        Ok(InputOutcome::Ignored)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        self.account(cx.now());
        self.hovered = hovered;
        if !hovered {
            self.hovered_control = None;
        }
        self.paused = self.hovered || self.focused;
        cx.invalidate(Invalidation::Paint);
    }
    fn focus_gained(&mut self, cx: &mut EventContext) {
        self.account(cx.now());
        self.focused = true;
        self.paused = true;
        self.input.focused = true;
        cx.invalidate(Invalidation::Paint);
    }
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.account(cx.now());
        self.focused = false;
        self.paused = self.hovered;
        self.input.reset();
        self.pressed = None;
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.account(cx.now());
        self.focused = false;
        self.hovered = false;
        self.paused = false;
        self.input.reset();
        self.pressed = None;
        cx.invalidate(Invalidation::Paint);
    }
    fn input_reset(&mut self) {
        self.input.reset();
        self.pressed = None;
        self.hovered_control = None;
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        for (item, t) in self.items.iter().zip(self.prepared.borrow().iter()) {
            let shape = rounded(p.metric(MetricRole::CornerRadius))?;
            p.fill_shape(t.rect, shape, p.color(ColorRole::Surface))?;
            p.stroke_shape(
                t.rect,
                shape,
                Length::new(p.metric(MetricRole::BorderWidth))?,
                p.color(ColorRole::PanelBorder),
            )?;
            let [_, y, _, _] = t.rect.values();
            let pad = p
                .metric(MetricRole::ControlPaddingX)
                .min(t.rect.width() / 2.0);
            t.text
                .paint(p, pad, y + pad, p.color(ColorRole::OnSurface))?;
            for (text, rect, visible, action) in [
                (&t.action, t.action_rect, item.action.is_some(), true),
                (&t.dismiss, t.dismiss_rect, true, false),
            ] {
                if visible && !rect.is_empty() {
                    p.fill_shape(
                        rect,
                        shape,
                        p.color(if self.hovered_control == Some((item.id, action)) {
                            ColorRole::ButtonHover
                        } else {
                            ColorRole::Button
                        }),
                    )?;
                    let [x, y, w, h] = rect.values();
                    p.with_clip(rect, |p| {
                        text.paint(
                            p,
                            x + (w - text.size().width()).max(0.0) / 2.0,
                            y + (h - text.size().height()).max(0.0) / 2.0,
                            p.color(ColorRole::OnButton),
                        )
                    })?;
                }
            }
            if self.input.focused && self.selected == Some(item.id) {
                focus_ring(p, t.rect)?;
            }
        }
        Ok(())
    }
}
