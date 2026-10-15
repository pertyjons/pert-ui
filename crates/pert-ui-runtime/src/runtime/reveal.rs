//! Revealing a possibly hidden target: opening the containers on its path,
//! scrolling to it and optionally focusing it.
use super::native_windows::NativeWindowId;
use super::*;
use crate::PresentationReceipt;

/// What a container did when asked to show a hidden child through
/// [`Component::reveal_child`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ChildReveal {
    /// The child needs no change, for example a tab presented in its own
    /// native window.
    AlreadyVisible,
    /// The container changed its own state so that
    /// [`Component::child_visible`] now shows the child.
    Changed,
    /// The application controls the child's visibility, as through
    /// `View::visible_signal`, so the container leaves it hidden.
    ApplicationControlled,
    /// The container does not show the child on request. This is the default
    /// of [`Component::reveal_child`].
    Refused,
}

/// Why a reveal stopped before its target became visible, or before it took
/// focus.
///
/// Opening the path is not atomic: containers opened before the refusal stay
/// open, and the refusal names where the walk stopped.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RevealRefusal {
    /// `ancestor` hides the next component on the path and its
    /// [`Component::reveal_child`] refused, or reported a change that left the
    /// component hidden.
    AncestorRefused {
        /// The container that refused.
        ancestor: WidgetId,
    },
    /// `ancestor` hides the next component on the path in visibility the
    /// application controls ([`ChildReveal::ApplicationControlled`]).
    ApplicationControlled {
        /// The container whose visibility the application binds.
        ancestor: WidgetId,
    },
    /// `component`, the target or one of its ancestors, is hidden explicitly
    /// through [`Runtime::set_visible`], which only the application undoes.
    Hidden {
        /// The explicitly hidden component.
        component: WidgetId,
    },
    /// An open modal surface blocks the target. Admission is checked before
    /// anything opens and, for a focusing reveal, again before focus.
    ModalBlocked,
}

/// The terminal result of revealing a mounted target.
///
/// `RuntimeError` remains for operational failures and stale or foreign
/// identities; everything a reveal can legitimately decline is an outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RevealOutcome {
    /// The target is visible and its scrolling ancestors show it. Focus did not
    /// move: none was asked for, or the target is not focusable.
    Revealed {
        /// Activation of the target's native window, requested when it differs
        /// from the window the request came from. The receipt reports what the
        /// platform did; a requested activation is not a raised window.
        activation: Option<PresentationReceipt>,
    },
    /// As [`Self::Revealed`], and the target now holds focus in its window.
    RevealedAndFocused {
        /// Activation of the target's native window, as for
        /// [`Self::Revealed`].
        activation: Option<PresentationReceipt>,
    },
    /// The reveal stopped; containers opened before the refusal stay open.
    Refused(RevealRefusal),
    /// Opening the path unmounted the target. Its key is never resolved to a
    /// replacement.
    Unmounted,
}
impl RevealOutcome {
    /// Whether the target ended visible, focused or not.
    #[must_use]
    pub fn is_revealed(&self) -> bool {
        matches!(
            self,
            Self::Revealed { .. } | Self::RevealedAndFocused { .. }
        )
    }
    /// The requested native window activation, if the target's window differs
    /// from the requesting one.
    #[must_use]
    pub fn activation(&self) -> Option<&PresentationReceipt> {
        match self {
            Self::Revealed { activation } | Self::RevealedAndFocused { activation } => {
                activation.as_ref()
            }
            Self::Refused(_) | Self::Unmounted => None,
        }
    }
}

/// Reactive completion of a reveal queued with
/// [`EventContext::request_reveal`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevealStatus {
    /// The reveal runs after the next flush's updates and layout settle.
    Pending,
    /// The reveal ran: its outcome, or the operational error that stopped it,
    /// such as `Unmounted` for a target already gone when it ran.
    Finished(Result<RevealOutcome, RuntimeError>),
    /// Runtime shutdown discarded the request before it ran.
    Cancelled,
}

/// UI-local completion of one queued reveal.
///
/// The status is stored outside the runtime's signal graph, like a
/// [`CommandReceipt`](crate::CommandReceipt)'s, so it stays readable after
/// runtime shutdown, where [`RevealStatus::Cancelled`] reports a request the
/// shutdown discarded. [`Self::status_binding`] mirrors it as a binding. Every
/// accepted request reaches a terminal status; none is dropped silently.
/// Retaining a receipt retains neither the target nor the runtime.
///
/// Receipts compare equal when they report the same request.
#[derive(Clone)]
#[must_use = "retain the receipt if the application needs the reveal's outcome"]
pub struct RevealReceipt(ReceiptStatus<RevealStatus>);
impl RevealReceipt {
    /// Pending or the retained terminal result, readable after shutdown.
    /// Reading creates no subscription or timer.
    #[must_use]
    pub fn status(&self) -> RevealStatus {
        self.0.get()
    }
    /// The same status as a binding, so a component that reads it during
    /// update is invalidated when the reveal finishes. Like every binding it
    /// reads `Err(RuntimeError::Closed)` after runtime shutdown; use
    /// [`Self::status`] then.
    pub fn status_binding(&self) -> ReadSignal<RevealStatus> {
        self.0.binding()
    }
}
impl PartialEq for RevealReceipt {
    fn eq(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}
impl Eq for RevealReceipt {}
/// Shows the current status, as [`RevealReceipt::status`] reads it.
impl std::fmt::Debug for RevealReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RevealReceipt")
            .field("status", &self.status())
            .finish()
    }
}

pub(crate) struct QueuedReveal {
    notification_origin: Option<Weak<super::notify::InputNotificationState>>,
    target: WidgetId,
    focus: bool,
    origin: NativeWindowId,
    receipt: RevealReceipt,
}
impl QueuedReveal {
    fn finish(&self, status: RevealStatus) {
        self.receipt.0.set(status);
    }
    /// Discards a request that runtime shutdown ended before it ran.
    pub(super) fn cancel(self) {
        self.finish(RevealStatus::Cancelled);
    }
}
impl Drop for QueuedReveal {
    fn drop(&mut self) {
        // A request dropped without a terminal status, for example while a
        // reveal unwinds, reports cancellation rather than staying pending.
        if self.receipt.status() == RevealStatus::Pending {
            self.finish(RevealStatus::Cancelled);
        }
    }
}

struct RevealDispatchGuard {
    shared: Rc<Shared>,
    previous: bool,
}
impl RevealDispatchGuard {
    fn new(shared: Rc<Shared>) -> Self {
        let previous = shared.reveal_dispatching.replace(true);
        Self { shared, previous }
    }
}
impl Drop for RevealDispatchGuard {
    fn drop(&mut self) {
        self.shared.reveal_dispatching.set(self.previous);
    }
}

impl EventContext {
    /// Queues a reveal of `target`, optionally focusing it, for the end of the
    /// next flush, and returns its receipt.
    ///
    /// The reveal runs as [`Runtime::reveal`] (`focus` false) or
    /// [`Runtime::reveal_and_focus`] (`focus` true) after that flush's updates
    /// and layout settle, so a surface this event closes, such as a modal
    /// palette that jumps elsewhere, no longer blocks the target. It runs even
    /// when the requesting component has been removed meanwhile. A native
    /// window activation is requested when the target lives in another window
    /// than this component. Requests run in the order they were made; one made
    /// while reveals are running waits for the next flush, which
    /// [`Runtime::next_deadline`] then reports as due.
    ///
    /// The receipt always reaches a terminal [`RevealStatus`]: the outcome, an
    /// operational error such as `Unmounted` for a target removed before the
    /// reveal ran, or `Cancelled` at shutdown.
    ///
    /// ```
    /// use pert_ui_runtime::prelude::*;
    /// use pert_ui_runtime::{Tab, Tabs};
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    /// let tabs = ui.mount_tree(Element::new(
    ///     "tabs",
    ///     Tabs::new([
    ///         Tab::new("home", "Home", Element::new("welcome", Label::new("Welcome"))),
    ///         Tab::new("settings", "Settings", Element::new("name", TextField::new(ui.signal(String::new())))),
    ///     ]),
    /// ))?;
    /// ui.flush(Duration::ZERO)?;
    /// let name = ui.find_descendant(&tabs, "name")?.ok_or("not mounted")?;
    ///
    /// // An internal link: activating the button opens the tab and focuses the field.
    /// let target = name.clone();
    /// let link = ui.mount_tree(Element::new(
    ///     "edit-name",
    ///     Button::new("Edit name", move |cx| cx.request_reveal(&target, true).map(|_| ())),
    /// ))?;
    /// ui.flush(Duration::ZERO)?;
    /// ui.focus(&link)?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(ui.focused(), Some(name));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for a target of another runtime,
    /// [`RuntimeError::RevealQueueFull`] when [`Runtime::MAX_REVEAL_REQUESTS`]
    /// requests are waiting, or the runtime phase/shutdown error, without
    /// queueing the request.
    pub fn request_reveal(
        &self,
        target: &WidgetId,
        focus: bool,
    ) -> Result<RevealReceipt, RuntimeError> {
        let shared = &self.node.shared;
        shared.writable()?;
        if !Rc::ptr_eq(shared, &target.0.shared) {
            return Err(RuntimeError::ForeignRuntime);
        }
        let mut requests = shared.reveal_requests.borrow_mut();
        if requests.len() >= Runtime::MAX_REVEAL_REQUESTS {
            return Err(RuntimeError::RevealQueueFull);
        }
        let receipt = RevealReceipt(ReceiptStatus::new(shared.clone(), RevealStatus::Pending));
        requests.push_back(QueuedReveal {
            notification_origin: shared.notification_origin.borrow().clone(),
            target: target.clone(),
            focus,
            origin: NativeWindowId(self.node.window()),
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }
}

impl Runtime {
    /// Maximum number of reveals queued with [`EventContext::request_reveal`]
    /// waiting for a flush. A request beyond it is refused with
    /// [`RuntimeError::RevealQueueFull`].
    pub const MAX_REVEAL_REQUESTS: usize = 16;

    /// Makes `target` visible and scrolls its ancestors to show it, without
    /// moving focus.
    ///
    /// The runtime first flushes, then walks the target's presentation
    /// ancestors from the outermost inwards. Where a container's
    /// [`Component::child_visible`] hides the next component on the path, it
    /// asks that container through [`Component::reveal_child`], applies the
    /// change and settles update and layout before going on, revalidating the
    /// same [`WidgetId`] after every step. Scrolling then uses
    /// [`Component::reveal`] from the innermost scrolling ancestor outwards,
    /// as focus does. A target in another native window than the selected one
    /// gets a requested activation of its window, reported in the outcome.
    ///
    /// The walk refuses, as a [`RevealOutcome::Refused`], an explicitly hidden
    /// component on the path ([`RevealRefusal::Hidden`]), a container that
    /// will not show it, application-controlled visibility, and a target an
    /// open modal surface blocks; admission is checked before anything opens.
    /// Containers opened before a refusal stay open. A target that opening the
    /// path unmounts ends as [`RevealOutcome::Unmounted`].
    ///
    /// The selected window is the same afterwards, unless it was closed during
    /// the walk; the target's window is then selected.
    ///
    /// ```
    /// use pert_ui_runtime::prelude::*;
    /// use pert_ui_runtime::{RevealOutcome, Tab, Tabs};
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    /// let tabs = ui.mount_tree(Element::new(
    ///     "tabs",
    ///     Tabs::new([
    ///         Tab::new("general", "General", Element::new("general-text", Label::new("General"))),
    ///         Tab::new("details", "Details", Element::new("details-text", Label::new("Details"))),
    ///     ]),
    /// ))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// let shown = |ui: &Runtime, key: &str| {
    ///     ui.inspect().components.iter().any(|c| c.key == key && c.visible)
    /// };
    /// // The second tab's content is mounted, with its state, but hidden.
    /// let details = ui.find_descendant(&tabs, "details-text")?.ok_or("not mounted")?;
    /// assert!(!shown(&ui, "details-text"));
    ///
    /// let outcome = ui.reveal(&details)?;
    /// assert_eq!(outcome, RevealOutcome::Revealed { activation: None });
    /// assert!(shown(&ui, "details-text"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns `ForeignRuntime` or `Unmounted` for a target that is not a
    /// mounted component of this runtime when called, the runtime
    /// phase/shutdown error, and errors of the flushes, of
    /// [`Component::reveal_child`] and of [`Component::reveal`].
    pub fn reveal(&mut self, target: &WidgetId) -> Result<RevealOutcome, RuntimeError> {
        let origin = self.current_window();
        self.reveal_target(target, false, &origin)
    }

    /// Reveals `target` as [`Self::reveal`] does, then focuses it.
    ///
    /// A target that is not focusable, such as a `Group`, is revealed without
    /// claiming focus and reports [`RevealOutcome::Revealed`]. Modal admission
    /// is checked again before focus. Focus moves without further scrolling,
    /// since the target has just been scrolled into view.
    ///
    /// "Search the UI and jump there" is built from commands rather than a
    /// destination registry: the application registers one command per
    /// destination, whose action reveals and focuses it, and lists them with
    /// [`Runtime::commands_for`].
    ///
    /// ```
    /// use pert_ui_runtime::prelude::*;
    /// use pert_ui_runtime::{CommandOutcome, CommandScope, CommandSource, Tab, Tabs};
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    /// let tabs = ui.mount_tree(Element::new(
    ///     "tabs",
    ///     Tabs::new([
    ///         Tab::new("home", "Home", Element::new("welcome", Label::new("Welcome"))),
    ///         Tab::new("mixer", "Mixer", Element::new("master", Button::new("Master", |_| Ok(())))),
    ///     ]),
    /// ))?;
    /// ui.flush(Duration::ZERO)?;
    /// let master = ui.find_descendant(&tabs, "master")?.ok_or("not mounted")?;
    /// let target = master.clone();
    /// let go = ui.register_command("Go to: Mixer", CommandScope::Application, vec![],
    ///     move |ui, _| ui.reveal_and_focus(&target).map(|_| ()))?;
    ///
    /// let context = ui.command_context(CommandSource::Programmatic);
    /// assert_eq!(ui.invoke_command(&go, context)?, CommandOutcome::Invoked);
    /// assert_eq!(ui.focused(), Some(master));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// The errors of [`Self::reveal`], and a previous recipient's
    /// `Component::focus_lost` error, returned after the transfer completes, as
    /// from [`Self::focus`].
    pub fn reveal_and_focus(&mut self, target: &WidgetId) -> Result<RevealOutcome, RuntimeError> {
        let origin = self.current_window();
        self.reveal_target(target, true, &origin)
    }

    /// Asks `container` to show its immediate child `key` through
    /// [`Component::reveal_child`], the same transition a reveal uses, and
    /// settles update and layout.
    ///
    /// This is how application code selects a tab by key:
    /// `ui.reveal_child(&tabs, "mixer")` selects the tab exactly as a click on
    /// its title does, so workspace capture records it, and it requests
    /// activation of the tab's window when the tab is detached. A child that
    /// is already shown returns [`ChildReveal::AlreadyVisible`] without asking
    /// the container. Focus does not move.
    ///
    /// ```
    /// use pert_ui_runtime::prelude::*;
    /// use pert_ui_runtime::{Tab, Tabs};
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    /// let tabs = ui.mount_tree(Element::new(
    ///     "tabs",
    ///     Tabs::new([
    ///         Tab::new("general", "General", Element::new("text", Label::new("General"))),
    ///         Tab::new("mixer", "Mixer", Element::new("text", Label::new("Mixer"))),
    ///     ]),
    /// ))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    ///
    /// assert_eq!(ui.reveal_child(&tabs, "mixer")?, ChildReveal::Changed);
    /// assert_eq!(ui.reveal_child(&tabs, "mixer")?, ChildReveal::AlreadyVisible);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns `ForeignRuntime` or `Unmounted` for a container that is not a
    /// mounted component of this runtime, `ModalBlocked` for one an open modal
    /// surface blocks, [`LayoutError::UnknownKey`](crate::LayoutError::UnknownKey)
    /// when it has no immediate child `key`, the runtime phase/shutdown error,
    /// and errors of the flushes and of the hook.
    pub fn reveal_child(
        &mut self,
        container: &WidgetId,
        key: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        self.shared.writable()?;
        self.check(container)?;
        let previous = self.current_window();
        self.select_window(&self.window_of(container)?)?;
        let result = self.reveal_child_selected(container, key);
        self.restore_window(&previous)?;
        result
    }

    fn reveal_child_selected(
        &mut self,
        container: &WidgetId,
        key: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        let now = self.shared.now.get();
        self.flush(now)?;
        self.check(container)?;
        if !self.modal_allows(container.0.serial) {
            return Err(RuntimeError::ModalBlocked);
        }
        let serial = container.0.serial;
        if !self.keys.contains_key(&(Some(serial), key.to_owned())) {
            return Err(crate::LayoutError::UnknownKey.into());
        }
        if self.entries[&serial].component.child_visible(key) {
            return Ok(ChildReveal::AlreadyVisible);
        }
        let outcome = self.call_reveal_child(serial, key)?;
        if outcome == ChildReveal::Changed {
            self.settle_revealed(serial)?;
        }
        Ok(outcome)
    }

    /// Restores the window selected before an operation, unless it closed.
    pub(super) fn restore_window(&mut self, previous: &NativeWindowId) -> Result<(), RuntimeError> {
        if previous.0.mounted.get() {
            self.select_window(previous)
        } else {
            Ok(())
        }
    }

    fn call_reveal_child(&mut self, serial: u64, key: &str) -> Result<ChildReveal, RuntimeError> {
        let entry = self
            .entries
            .get_mut(&serial)
            .ok_or(RuntimeError::Unmounted)?;
        entry.component.reveal_child(
            &mut EventContext {
                node: entry.node.clone(),
            },
            key,
        )
    }

    /// Applies a container's changed child visibility and settles the result.
    fn settle_revealed(&mut self, serial: u64) -> Result<(), RuntimeError> {
        if self.entries.contains_key(&serial) {
            self.refresh_child_visibility(serial);
            if let Some(entry) = self.entries.get(&serial) {
                entry.node.invalidate(Invalidation::Layout);
            }
        }
        self.flush(self.shared.now.get())?;
        Ok(())
    }

    fn reveal_target(
        &mut self,
        target: &WidgetId,
        focus: bool,
        origin: &NativeWindowId,
    ) -> Result<RevealOutcome, RuntimeError> {
        self.shared.writable()?;
        self.check(target)?;
        let previous = self.current_window();
        self.select_window(&self.window_of(target)?)?;
        let result = self.reveal_selected(target, focus, origin);
        self.restore_window(&previous)?;
        result
    }

    fn reveal_selected(
        &mut self,
        target: &WidgetId,
        focus: bool,
        origin: &NativeWindowId,
    ) -> Result<RevealOutcome, RuntimeError> {
        self.flush(self.shared.now.get())?;
        if !target.0.mounted.get() {
            return Ok(RevealOutcome::Unmounted);
        }
        if !self.modal_allows(target.0.serial) {
            return Ok(RevealOutcome::Refused(RevealRefusal::ModalBlocked));
        }
        // Each container is asked at most once: one that reports a change but
        // still hides the path refused in effect, and the walk ends.
        let mut asked = std::collections::BTreeSet::new();
        loop {
            if !target.0.mounted.get() {
                return Ok(RevealOutcome::Unmounted);
            }
            // The presentation path, target first, ending at its root.
            let mut path = vec![target.0.clone()];
            while let Some(parent) = path[path.len() - 1].presentation_parent() {
                path.push(parent);
            }
            if let Some(hidden) = path.iter().rev().find(|node| !node.local_visible.get()) {
                return Ok(RevealOutcome::Refused(RevealRefusal::Hidden {
                    component: WidgetId(hidden.clone()),
                }));
            }
            let link = (1..path.len()).rev().find_map(|index| {
                let parent = &path[index];
                let key = &self.entries.get(&path[index - 1].serial)?.key;
                let entry = self.entries.get(&parent.serial)?;
                (!entry.component.child_visible(key)).then(|| (parent.clone(), key.clone()))
            });
            let Some((parent, key)) = link else {
                break;
            };
            let ancestor = WidgetId(parent.clone());
            if !asked.insert(parent.serial) {
                return Ok(RevealOutcome::Refused(RevealRefusal::AncestorRefused {
                    ancestor,
                }));
            }
            match self.call_reveal_child(parent.serial, &key)? {
                ChildReveal::Changed | ChildReveal::AlreadyVisible => {
                    self.settle_revealed(parent.serial)?;
                }
                ChildReveal::ApplicationControlled => {
                    return Ok(RevealOutcome::Refused(
                        RevealRefusal::ApplicationControlled { ancestor },
                    ));
                }
                ChildReveal::Refused => {
                    return Ok(RevealOutcome::Refused(RevealRefusal::AncestorRefused {
                        ancestor,
                    }));
                }
            }
        }
        if !target.0.visible.get() {
            // Every container on the path shows the next one and nothing is
            // hidden explicitly, so visibility is only waiting to be applied.
            self.flush(self.shared.now.get())?;
            if !target.0.mounted.get() {
                return Ok(RevealOutcome::Unmounted);
            }
            if !target.0.visible.get() {
                return Err(RuntimeError::Hidden);
            }
        }
        self.reveal_focus(target)?;
        if !target.0.mounted.get() {
            return Ok(RevealOutcome::Unmounted);
        }
        if !target.0.visible.get() {
            // A scrolling ancestor's reveal hid the target again.
            return Ok(RevealOutcome::Refused(self.hiding_refusal(target)));
        }
        let window = NativeWindowId(target.0.window());
        let focusable = self
            .entries
            .get(&target.0.serial)
            .is_some_and(|entry| entry.component.focusable());
        if focus && focusable {
            if !self.modal_allows(target.0.serial) {
                return Ok(RevealOutcome::Refused(RevealRefusal::ModalBlocked));
            }
            let activation = self.activation_for(&window, origin)?;
            self.focus_without_reveal(target)?;
            return Ok(RevealOutcome::RevealedAndFocused { activation });
        }
        Ok(RevealOutcome::Revealed {
            activation: self.activation_for(&window, origin)?,
        })
    }

    /// Why a mounted target that the walk had shown is hidden again: an
    /// explicitly hidden component on its path, else the outermost container
    /// whose [`Component::child_visible`] hides the next component, else the
    /// target itself.
    fn hiding_refusal(&self, target: &WidgetId) -> RevealRefusal {
        let mut path = vec![target.0.clone()];
        while let Some(parent) = path[path.len() - 1].presentation_parent() {
            path.push(parent);
        }
        if let Some(hidden) = path.iter().rev().find(|node| !node.local_visible.get()) {
            return RevealRefusal::Hidden {
                component: WidgetId(hidden.clone()),
            };
        }
        (1..path.len())
            .rev()
            .find_map(|index| {
                let key = &self.entries.get(&path[index - 1].serial)?.key;
                let entry = self.entries.get(&path[index].serial)?;
                (!entry.component.child_visible(key)).then(|| RevealRefusal::AncestorRefused {
                    ancestor: WidgetId(path[index].clone()),
                })
            })
            .unwrap_or_else(|| RevealRefusal::Hidden {
                component: target.clone(),
            })
    }

    fn activation_for(
        &self,
        window: &NativeWindowId,
        origin: &NativeWindowId,
    ) -> Result<Option<PresentationReceipt>, RuntimeError> {
        if window == origin {
            Ok(None)
        } else {
            self.request_window_activation(window).map(Some)
        }
    }

    /// Runs the queued reveals in order, each to a terminal status, then
    /// settles the focus and presentation changes they made within the same
    /// flush. A reveal queued while these run waits for the next flush.
    pub(super) fn dispatch_reveal_requests(&mut self) -> Result<(), RuntimeError> {
        if self.shared.reveal_dispatching.get() || self.shared.reveal_requests.borrow().is_empty() {
            return Ok(());
        }
        let _guard = RevealDispatchGuard::new(self.shared.clone());
        let requests = std::mem::take(&mut *self.shared.reveal_requests.borrow_mut());
        for request in requests {
            let _origin = super::notify::NotificationOriginGuard::new(
                self.shared.clone(),
                request.notification_origin.clone(),
            );
            // An earlier reveal may have shut the runtime down.
            let status = if self.shared.closed.get() {
                RevealStatus::Cancelled
            } else {
                RevealStatus::Finished(self.reveal_target(
                    &request.target,
                    request.focus,
                    &request.origin,
                ))
            };
            request.finish(status);
        }
        if self.shared.closed.get() {
            return Ok(());
        }
        self.flush(self.shared.now.get()).map(|_| ())
    }
}
