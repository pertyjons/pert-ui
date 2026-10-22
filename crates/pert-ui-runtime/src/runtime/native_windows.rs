//! Retained native presentation roots. Window/GPU creation belongs to the host.
use super::*;

pub(super) struct WindowShared {
    /// The runtime that created this root, compared by address only, so a
    /// foreign identity is told apart from a retired one of this runtime.
    pub(super) runtime: Weak<Shared>,
    pub(super) presentation_rate: Cell<Option<crate::PresentationRate>>,
    pub(super) presentation_interval: Cell<Duration>,
    pub(super) serial: u64,
    pub(super) command_target: RefCell<Option<Weak<Node>>>,
    pub(super) mounted: Cell<bool>,
    pub(super) visible: Cell<bool>,
    pub(super) native_viewport: Cell<crate::Viewport>,
    /// The title a detached window opened with, from its options; `None`
    /// for the main window.
    pub(super) opening_title: RefCell<Option<String>>,
    pub(super) geometry_change: Cell<Option<f64>>,
    pub(super) theme_geometry_change: Cell<bool>,
    pub(super) scene_dirty: Cell<bool>,
    pub(super) stack_hover_pending: Cell<bool>,
    pub(super) capture: Cell<Option<u64>>,
    pub(super) requested_focus: RefCell<Option<Weak<Node>>>,
    /// Whether the pending focus request asks for the first focusable
    /// component inside its target rather than the target itself.
    pub(super) requested_within: Cell<bool>,
    pub(super) held_buttons: Cell<usize>,
    pub(super) pointer_position: Cell<Option<crate::Point>>,
    pub(super) focused: Cell<Option<u64>>,
    pub(super) focus_node: RefCell<Option<Weak<Node>>>,
    pub(super) viewport: Cell<crate::Viewport>,
    pub(super) layout_pending: Cell<bool>,
    pub(super) pending: RefCell<BTreeMap<u64, Weak<Node>>>,
    pub(super) deadlines: RefCell<BTreeMap<(Duration, u64), Weak<Node>>>,
    pub(super) timers: RefCell<BTreeMap<(Duration, u64), Weak<Node>>>,
}

pub(super) struct WindowData {
    pub(super) detached_root: Option<WidgetId>,
    pub(super) return_focus: Option<WidgetId>,
    pub(super) options: Option<NativeWindowOptions>,
    pub(super) shared: Rc<WindowShared>,
    pub(super) focus_errors: std::collections::VecDeque<RuntimeError>,
    /// Ancestors last told that focus is within them, innermost first. A
    /// cancellation that clears focus leaves the ones it did not cancel, hide
    /// or remove here, so focus returning inside them tells them nothing new.
    pub(super) focus_within: Vec<u64>,
    pub(super) surfaces: Vec<surfaces::ActiveSurface>,
    pub(super) surface_error: Option<RuntimeError>,
    pub(super) deferred_surface_dismissals: BTreeMap<u64, crate::SurfaceDismissal>,
    pub(super) surface_focus: Option<u64>,
    pub(super) dismissed_buttons: std::collections::BTreeSet<crate::PointerButton>,
    pub(super) buttons: std::collections::BTreeSet<crate::PointerButton>,
    pub(super) pointer_target: Option<WidgetId>,
    pub(super) restore_focus: bool,
    /// Presentation ancestors, innermost first, of the component whose focus
    /// a cancellation cleared; `restore_focus` tries them before moving on.
    pub(super) focus_fallback: Vec<Weak<Node>>,
    /// Whether the focused component was focusable when it received focus;
    /// explicit focus on a non-focusable component is left alone.
    pub(super) focus_was_focusable: bool,
    pub(super) focus: Option<WidgetId>,
}
impl WindowData {
    pub(super) fn new(serial: u64, runtime: Weak<Shared>) -> Self {
        Self {
            detached_root: None,
            return_focus: None,
            options: None,
            shared: Rc::new(WindowShared {
                presentation_rate: Cell::new(None),
                presentation_interval: Cell::new(super::PRESENTATION_INTERVAL),
                runtime,
                serial,
                command_target: RefCell::new(None),
                mounted: Cell::new(true),
                visible: Cell::new(true),
                native_viewport: Cell::new(crate::Viewport::default()),
                opening_title: RefCell::new(None),
                geometry_change: Cell::new(None),
                theme_geometry_change: Cell::new(false),
                scene_dirty: Cell::new(false),
                stack_hover_pending: Cell::new(false),
                capture: Cell::new(None),
                requested_focus: RefCell::new(None),
                requested_within: Cell::new(false),
                held_buttons: Cell::new(0),
                pointer_position: Cell::new(None),
                focused: Cell::new(None),
                focus_node: RefCell::new(None),
                viewport: Cell::new(crate::Viewport::default()),
                layout_pending: Cell::new(false),
                pending: RefCell::new(BTreeMap::new()),
                deadlines: RefCell::new(BTreeMap::new()),
                timers: RefCell::new(BTreeMap::new()),
            }),
            focus_errors: std::collections::VecDeque::new(),
            focus_within: Vec::new(),
            surfaces: Vec::new(),
            surface_error: None,
            deferred_surface_dismissals: BTreeMap::new(),
            surface_focus: None,
            dismissed_buttons: std::collections::BTreeSet::new(),
            buttons: std::collections::BTreeSet::new(),
            pointer_target: None,
            restore_focus: false,
            focus_fallback: Vec::new(),
            focus_was_focusable: false,
            focus: None,
        }
    }
}

/// Rejects a view whose description forbids its own native window, reading
/// the permission's current value. Re-docking an already detached view stays
/// available regardless of a later refusal.
fn permitted(view: &WidgetId) -> Result<(), RuntimeError> {
    if view.0.detachable.borrow().get_in(&view.0.shared)? {
        Ok(())
    } else {
        Err(RuntimeError::NotDetachable)
    }
}

/// Identity of one retained native presentation root in a runtime.
///
/// UI-thread only. Clones retain the identity, not its mounted lifetime. A retired
/// identity never addresses a replacement window and must not be persisted.
#[derive(Clone)]
#[must_use]
pub struct NativeWindowId(pub(super) Rc<WindowShared>);
impl NativeWindowId {
    /// The title this detached window opened with, `None` for the main
    /// window, so a container that retitles it knows what it shows first.
    pub(crate) fn opening_title(&self) -> Option<String> {
        self.0.opening_title.borrow().clone()
    }
}
impl PartialEq for NativeWindowId {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for NativeWindowId {}
impl std::hash::Hash for NativeWindowId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(Rc::as_ptr(&self.0), state);
    }
}
impl NativeWindowId {
    /// The incarnation number inspection reports for this window,
    /// [`WindowInspection::id`](crate::WindowInspection::id) and the window
    /// of a [`CaptureJob`](crate::CaptureJob). Distinct for every window the
    /// runtime ever creates; never an operating-system handle.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.0.serial
    }
}
impl std::fmt::Debug for NativeWindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeWindowId")
            .field("serial", &self.0.serial)
            .field("mounted", &self.0.mounted.get())
            .finish()
    }
}

/// Action after the host accepts closing a detached native window.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DetachedClosePolicy {
    /// Return the retained view to its original owner. Failure keeps it available.
    #[default]
    Redock,
    /// Unmount the view through normal runtime lifetime cleanup. Hosts using
    /// declarative reconciliation must also remove it from their source state.
    RemoveView,
    /// Keep the window available; useful when the host presents its own controls.
    KeepOpen,
}

/// Validated options for a detached native window. Ordinary native decorations
/// and independent stacking are used. Defaults to 640 × 480 logical pixels.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct NativeWindowOptions {
    title: String,
    size: crate::Size,
    close_policy: DetachedClosePolicy,
}
impl NativeWindowOptions {
    /// Creates default options.
    ///
    /// # Errors
    /// Returns [`RuntimeError::EmptyWindowTitle`](crate::RuntimeError::EmptyWindowTitle)
    /// for an empty or whitespace-only title.
    pub fn new(title: impl Into<String>) -> Result<Self, RuntimeError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(RuntimeError::EmptyWindowTitle);
        }
        Ok(Self {
            title,
            size: crate::Size::from([640, 480]),
            close_policy: DetachedClosePolicy::default(),
        })
    }
    /// Sets the action after host close admission. Defaults to re-dock.
    pub fn close_policy(mut self, policy: DetachedClosePolicy) -> Self {
        self.close_policy = policy;
        self
    }
    /// Configured detached close behavior.
    pub const fn closing_policy(&self) -> DetachedClosePolicy {
        self.close_policy
    }
    /// Sets a nonzero logical initial size. Native placement remains OS policy.
    ///
    /// # Errors
    /// [`RuntimeError::Geometry`] holding
    /// [`GeometryError::Empty`](crate::GeometryError::Empty) for a zero width or
    /// height.
    pub fn size(mut self, size: crate::Size) -> Result<Self, RuntimeError> {
        if size.width() <= 0.0 || size.height() <= 0.0 {
            return Err(crate::GeometryError::Empty.into());
        }
        self.size = size;
        Ok(self)
    }
    /// Human-readable native title, independent of identity.
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Initial logical size, before user zoom and native DPI scaling.
    pub const fn initial_size(&self) -> crate::Size {
        self.size
    }
}

/// Validated presentation movement awaiting native window/surface preparation.
///
/// Dropping this token leaves the view unchanged. The native host prepares all
/// resources before calling [`Runtime::commit_detach`], which revalidates identity
/// and modality and applies focus-loss cleanup. Tokens are UI-local and single-use.
#[must_use]
pub struct PreparedDetach {
    view: WidgetId,
    source: NativeWindowId,
    destination: NativeWindowId,
    options: NativeWindowOptions,
}
/// Shows the moved view, its source and reserved destination and the options.
impl std::fmt::Debug for PreparedDetach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedDetach")
            .field("view", &self.view)
            .field("source", &self.source)
            .field("destination", &self.destination)
            .field("options", &self.options)
            .finish()
    }
}
impl PreparedDetach {
    /// Reserved, not-yet-mounted destination identity for native preparation.
    pub fn destination(&self) -> NativeWindowId {
        self.destination.clone()
    }
    /// Options validated before the native host allocates resources.
    pub fn options(&self) -> &NativeWindowOptions {
        &self.options
    }
}

impl Node {
    pub(super) fn presentation_parent(&self) -> Option<Rc<Node>> {
        if self.detached.get() {
            None
        } else {
            self.logical_parent()
        }
    }
    pub(super) fn presentation_parent_serial(&self) -> Option<u64> {
        if self.detached.get() {
            None
        } else {
            self.logical_parent_serial()
        }
    }
}

impl Runtime {
    /// The main presentation root, retained until runtime destruction.
    pub fn primary_window(&self) -> NativeWindowId {
        self.primary_window.clone()
    }
    /// Presentation root used by viewport, painting and native input operations.
    /// Selecting a root is routing, not a focus transition or native activation.
    pub fn current_window(&self) -> NativeWindowId {
        NativeWindowId(self.window.shared.clone())
    }
    /// All mounted native presentation roots, in creation order.
    pub fn native_windows(&self) -> Vec<NativeWindowId> {
        self.shared
            .windows
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|w| w.mounted.get())
            .map(NativeWindowId)
            .collect()
    }
    /// Routes subsequent viewport/paint/input operations to a mounted window.
    ///
    /// Does not call component focus callbacks or request OS activation. Native
    /// hosts select the event's root before dispatch. Applications normally use
    /// the default App host.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// [`RuntimeError::ForeignRuntime`](crate::RuntimeError::ForeignRuntime) for a window
    /// identity of another runtime; and `Unmounted` for one of this runtime that is retired
    /// or no longer mounted.
    pub fn select_window(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        if !std::ptr::eq(window.0.runtime.as_ptr(), Rc::as_ptr(&self.shared)) {
            return Err(RuntimeError::ForeignRuntime);
        }
        if self.current_window() == *window {
            return if window.0.mounted.get() {
                Ok(())
            } else {
                Err(RuntimeError::Unmounted)
            };
        }
        let valid = self
            .parked_windows
            .get(&window.0.serial)
            .is_some_and(|w| Rc::ptr_eq(&w.shared, &window.0) && window.0.mounted.get());
        if !valid {
            return Err(RuntimeError::Unmounted);
        }
        let next = self
            .parked_windows
            .remove(&window.0.serial)
            .ok_or(RuntimeError::Unmounted)?;
        let previous = std::mem::replace(&mut self.window, next);
        self.parked_windows.insert(previous.shared.serial, previous);
        Ok(())
    }
    /// The view's current presentation window.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for a view of another runtime and `Unmounted` for an
    /// unmounted one.
    pub fn window_of(&self, view: &WidgetId) -> Result<NativeWindowId, RuntimeError> {
        self.check(view)?;
        Ok(NativeWindowId(view.0.window()))
    }
    /// Whether this identity still names an available presentation root.
    pub fn contains_window(&self, window: &NativeWindowId) -> bool {
        self.native_windows().contains(window)
    }
    /// Supplies known native presentation visibility for the selected root.
    ///
    /// Hidden/zero-sized roots stop deadlines and presentation work; showing
    /// refreshes retained state. Unknown native visibility must be left unchanged,
    /// and lack of keyboard focus must never be treated as hidden.
    ///
    /// Hiding ends input like a native focus loss: focus, held actions,
    /// gestures and nonmodal surfaces such as popovers and menus end. It only
    /// suspends an open modal `Dialog`, which stays open with its outcome
    /// pending, keeps an application-modal session blocking other roots, and is
    /// admitted again, with safe focus, when the root is shown. Hiding a view
    /// with [`Runtime::set_visible`] instead dismisses its dialogs as
    /// `SurfaceDismissal::OwnerHidden`.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase.
    pub fn set_window_visible(&mut self, visible: bool) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        if self.window.shared.visible.replace(visible) == visible {
            return Ok(());
        }
        if !visible {
            self.clear_focus();
        }
        self.native_hiding = !visible;
        for root in self.root_order() {
            self.refresh_visibility(root, visible);
        }
        self.native_hiding = false;
        Ok(())
    }
    /// Reserves a native destination without changing mounted state or input.
    ///
    /// Native creation failure can simply drop the token.
    ///
    /// ```
    /// use pert_ui_runtime::{Runtime, Element, Label, NativeWindowOptions};
    /// let mut ui = Runtime::new();
    /// let view = ui.mount_tree(Element::new("editor", Label::new("Retained")))?;
    /// let original = ui.window_of(&view)?;
    /// let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    /// // A native host prepares its OS window and GPU surface here. Headless
    /// // tests can commit directly to exercise retained identity and cleanup.
    /// let detached = ui.commit_detach(prepared)?;
    /// assert_eq!(ui.window_of(&view)?, detached);
    /// ui.redock(&detached)?;
    /// assert_eq!(ui.window_of(&view)?, original);
    /// # Ok::<(), pert_ui_runtime::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `ForeignRuntime` or `Unmounted` for a foreign or unmounted view; `Hidden` for a
    /// hidden view; `NotDetachable` when its description forbids its own window;
    /// `ModalBlocked` when the subtree holds an open modal dialog; `AlreadyDetached` for a
    /// view that already has its own window; and `Exhausted` when window identities run
    /// out.
    pub fn prepare_detach(
        &mut self,
        view: &WidgetId,
        options: NativeWindowOptions,
    ) -> Result<PreparedDetach, RuntimeError> {
        self.shared.writable()?;
        self.check(view)?;
        permitted(view)?;
        self.validate_move(view)?;
        if view.0.detached.get() {
            return Err(RuntimeError::AlreadyDetached);
        }
        self.window_serial = self
            .window_serial
            .checked_add(1)
            .ok_or(RuntimeError::Exhausted)?;
        let destination = WindowData::new(self.window_serial, Rc::downgrade(&self.shared)).shared;
        destination.mounted.set(false);
        Ok(PreparedDetach {
            view: view.clone(),
            source: self.window_of(view)?,
            destination: NativeWindowId(destination),
            options,
        })
    }
    fn validate_move(&self, view: &WidgetId) -> Result<(), RuntimeError> {
        self.check(view)?;
        if !view.0.visible.get() {
            return Err(RuntimeError::Hidden);
        }
        // Inspect current component declarations too: a just-opened dialog may
        // not yet have reached surface admission/layout.
        for (&serial, entry) in &self.entries {
            if self.descendant(serial, view.0.serial)
                && self.surface_kind(serial).is_some_and(|s| s.modal())
            {
                let parent = entry
                    .node
                    .presentation_parent_serial()
                    .ok_or(RuntimeError::Unmounted)?;
                if self.entries[&parent].component.child_visible(&entry.key) {
                    return Err(RuntimeError::ModalBlocked);
                }
            }
        }
        Ok(())
    }
    /// Ends the moved subtree's part in its source window's input, leaving the
    /// rest of that window alone: no native focus loss happened, so held keys,
    /// modifiers, pointer buttons, capture and surfaces owned outside the subtree
    /// continue. Returns the focused component inside the subtree, if any.
    fn movement_cleanup(&mut self, view: &WidgetId) -> Result<Option<WidgetId>, RuntimeError> {
        let source = self.window_of(view)?;
        self.cancel_held_subtree(view.0.serial, crate::HeldCancellation::PresentationMoved);
        self.select_window(&source)?;
        let inside = |runtime: &Self, serial: u64| runtime.descendant(serial, view.0.serial);
        let focus = self
            .window
            .focus
            .clone()
            .filter(|f| inside(self, f.0.serial));
        if let Some(focused) = &focus {
            self.release_moved_focus(focused);
        }
        // Ancestors inside the view that were told focus is within them, with
        // focus already cleared silently, leave this window's record; they are
        // not told later from here, and the other window keeps its own.
        let moved: Vec<u64> = self
            .window
            .focus_within
            .iter()
            .copied()
            .filter(|serial| inside(self, *serial))
            .collect();
        self.window
            .focus_within
            .retain(|serial| !moved.contains(serial));
        if self
            .window
            .shared
            .requested_focus
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|node| inside(self, node.serial))
        {
            self.window.shared.requested_focus.borrow_mut().take();
        }
        let captured = self
            .window
            .shared
            .capture
            .get()
            .is_some_and(|serial| inside(self, serial));
        let pointed = self
            .window
            .pointer_target
            .as_ref()
            .is_some_and(|target| inside(self, target.0.serial));
        if pointed {
            self.update_hover_path(None);
            self.window.pointer_target = None;
            self.window.shared.pointer_position.set(None);
        }
        // A press the moved content received must not complete elsewhere: its
        // release is swallowed. A press owned outside the subtree continues.
        if captured || pointed {
            self.window
                .dismissed_buttons
                .extend(self.window.buttons.iter().copied());
        }
        if self.drag_in_window()
            && self
                .shared
                .drag_source
                .get()
                .is_some_and(|serial| inside(self, serial))
        {
            self.cancel_drag_infallibly();
        }
        if let Some(error) = self.window.focus_errors.pop_front() {
            return Err(error);
        }
        let serials: Vec<_> = self
            .entries
            .keys()
            .copied()
            .filter(|id| self.descendant(*id, view.0.serial))
            .collect();
        for serial in serials {
            self.dismiss_owner(serial, crate::SurfaceDismissal::WindowFocusLost)?;
            if let Some(entry) = self.entries.get_mut(&serial) {
                entry.node.lose_context();
                if !entry.node.input_cancelled.replace(true) {
                    entry.component.cancel_input(&mut EventContext {
                        node: entry.node.clone(),
                    });
                }
            }
        }
        let removals = std::mem::take(&mut *self.shared.requested_removals.borrow_mut());
        for node in removals
            .into_values()
            .filter_map(|n| n.upgrade())
            .filter(|n| n.mounted.get())
        {
            self.unmount(&WidgetId(node))?;
        }
        self.validate_move(view)?;
        Ok(focus)
    }
    /// Applies ordinary focus-loss policy to a focused component that moves
    /// away: `focus_lost` runs (a valid numeric draft saves), its IME session,
    /// timers, capture and gestures end, and the source window keeps no focus.
    /// A `focus_lost` error is queued like a native focus loss's.
    fn release_moved_focus(&mut self, focused: &WidgetId) {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Focus, None, "");
        self.window.restore_focus = false;
        self.window.focus = None;
        self.window.shared.focused.set(None);
        self.window.shared.focus_node.borrow_mut().take();
        // Set before `focus_lost`, so the subtree-wide cancellation that follows
        // does not cancel this component a second time.
        focused.0.input_cancelled.set(true);
        focused.0.invalidate_focus_path();
        focused.0.stop_deadline();
        focused.0.lose_context();
        if let Some(entry) = self.entries.get_mut(&focused.0.serial)
            && let Err(error) = entry.component.focus_lost(&mut EventContext {
                node: entry.node.clone(),
            })
        {
            self.window.focus_errors.push_back(error);
        }
        focused.0.invalidate(Invalidation::Paint);
        #[cfg(debug_assertions)]
        self.validator
            .flags(focused.0.serial, focused.0.visible.get(), false);
        // Runs before the presentation moves; the source window keeps no
        // focus, so every ancestor told it had focus within loses it.
        let leaving = self.notified_focus_chain();
        self.window.focus_within.clear();
        let errors = self.notify_focus_within(leaving, false);
        self.window.focus_errors.extend(errors);
        self.shared.record(crate::EventKind::Focus, None, None, || {
            crate::EventStatus::Succeeded
        });
    }
    /// Selects `previous` again after a failed movement, so a headless host's
    /// next operation reaches the root it had selected. A root the failed
    /// operation retired cannot be selected and is skipped.
    fn restore_selection(&mut self, previous: &NativeWindowId) {
        if self.contains_window(previous)
            && let Err(error) = self.select_window(previous)
        {
            // Only a closed runtime or a forbidden phase refuses a mounted root;
            // both already refused the movement, so this is not reached.
            self.window.focus_errors.push_back(error);
        }
    }
    /// Commits movement after the native host successfully prepares all resources.
    ///
    /// The same view incarnation and subscriptions move; no mount/unmount callback
    /// runs. Cleanup failure leaves presentation at its source. Completed numeric
    /// draft saves and input cancellation are not rolled back. A stale token fails
    /// without recreating an owner. Returns the new root and selects it for routing;
    /// on failure the root selected before the call stays selected.
    ///
    /// Initial geometry is validated against current UI zoom before cleanup.
    /// If cleanup callbacks change zoom, geometry is validated again before
    /// ownership moves; a rejection then leaves completed input cleanup in effect.
    ///
    /// Cleanup is confined to the moved subtree: its focused component loses
    /// focus through ordinary `focus_lost` policy, and its held keys, pointer
    /// capture, gestures and owned nonmodal surfaces end. Held keys, modifiers,
    /// pointer presses and surfaces owned elsewhere in the source window continue.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `ForeignRuntime` or `Unmounted` for a foreign or unmounted view; `Hidden` for a
    /// hidden view; `NotDetachable` when its description forbids its own window;
    /// `ModalBlocked` when the subtree holds an open modal dialog (all checked again); and
    /// `StalePresentation` when the view moved or was detached since the token was
    /// prepared. It forwards the errors of applying the new window's size as its viewport.
    /// When the focused component inside the view saves on focus loss, an error
    /// from its [`Component::focus_lost`](crate::Component::focus_lost) or from an ancestor's
    /// [`Component::focus_within_changed`](crate::Component::focus_within_changed) is returned
    /// and the view stays where it was; further such errors wait for the next flush.
    pub fn commit_detach(
        &mut self,
        prepared: PreparedDetach,
    ) -> Result<NativeWindowId, RuntimeError> {
        self.shared.writable()?;
        let previous = self.current_window();
        let result = self.commit_detach_selected(prepared);
        if result.is_err() {
            self.restore_selection(&previous);
        }
        result
    }
    fn commit_detach_selected(
        &mut self,
        prepared: PreparedDetach,
    ) -> Result<NativeWindowId, RuntimeError> {
        permitted(&prepared.view)?;
        self.validate_move(&prepared.view)?;
        if self.window_of(&prepared.view)? != prepared.source || prepared.view.0.detached.get() {
            return Err(RuntimeError::StalePresentation);
        }
        // Admission must use the current zoom, which can change after prepare.
        // Reject geometry before releasing focus/capture or moving ownership.
        let native = crate::Viewport::new(
            prepared.options.size.width(),
            prepared.options.size.height(),
            1.0,
        )?;
        super::settings::effective(native, self.shared.zoom.get())?;
        let focus = self.movement_cleanup(&prepared.view)?;
        if self.window_of(&prepared.view)? != prepared.source {
            return Err(RuntimeError::StalePresentation);
        }
        // Cleanup callbacks may change global settings. Validate the resulting
        // scale once more before the destination becomes a mounted owner.
        let viewport = super::settings::effective(native, self.shared.zoom.get())?;
        let mut window =
            WindowData::new(prepared.destination.0.serial, Rc::downgrade(&self.shared));
        window.shared = prepared.destination.0.clone();
        window.shared.native_viewport.set(native);
        window.shared.change_viewport(viewport);
        window.shared.mounted.set(true);
        window.detached_root = Some(prepared.view.clone());
        window.return_focus = focus;
        window
            .shared
            .opening_title
            .replace(Some(prepared.options.title().to_owned()));
        window.options = Some(prepared.options);
        self.shared
            .windows
            .borrow_mut()
            .push(Rc::downgrade(&window.shared));
        self.parked_windows.insert(window.shared.serial, window);
        prepared.view.0.detached.set(true);
        self.move_presentation(&prepared.view, &prepared.destination);
        // Its own window shows it at that window's scale, outside any view transform.
        let serial = prepared.view.0.serial;
        self.apply_view_scale(serial, self.inherited_scale(serial));
        if let Some(parent) = prepared.view.0.logical_parent() {
            parent.invalidate(Invalidation::Layout);
        }
        self.select_window(&prepared.destination)?;
        self.refresh_visibility(prepared.view.0.serial, true);
        *self.window.shared.command_target.borrow_mut() = Some(Rc::downgrade(&prepared.view.0));
        Ok(prepared.destination)
    }
    fn move_presentation(&mut self, view: &WidgetId, destination: &NativeWindowId) {
        let serials: Vec<_> = self
            .entries
            .keys()
            .copied()
            .filter(|id| self.descendant(*id, view.0.serial))
            .collect();
        for serial in serials {
            let node = &self.entries[&serial].node;
            let old = node.window();
            old.pending.borrow_mut().remove(&serial);
            old.scene_dirty.set(true);
            old.layout_pending.set(true);
            node.stop_deadline();
            node.stop_timer();
            *node.window.borrow_mut() = destination.0.clone();
            #[cfg(debug_assertions)]
            self.validator.relocate(
                serial,
                destination.0.serial,
                node.presentation_parent_serial(),
            );
            node.invalidate(Invalidation::Layout);
        }
    }
    /// Returns a detached window's view to its retained original owner.
    ///
    /// Revalidates the owner and modality before cleanup. Failure leaves the
    /// detached root available and the root selected before the call selected.
    /// On success retires its identity and selects the owner's window. Native
    /// hosts destroy the old resources after success. Input cleanup is confined
    /// to the moved view, as for [`Self::commit_detach`].
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `ForeignRuntime` for a window identity of another runtime; `Unmounted` for one
    /// that is retired or unmounted; `NotDetached` for a window that is not a detached view; `Hidden` when the view, its
    /// owner or the owner's window is hidden; `ModalBlocked` when the subtree holds an open
    /// modal dialog; and `ForeignRuntime` or `Unmounted` when the view or its owner is no
    /// longer mounted. A focus-loss error from the focused component or an ancestor is
    /// returned as for [`Self::commit_detach`], and the view stays detached.
    pub fn redock(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        let previous = self.current_window();
        let result = self.redock_selected(window);
        if result.is_err() {
            self.restore_selection(&previous);
        }
        result
    }
    fn redock_selected(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError> {
        self.select_window(window)?;
        let view = self
            .window
            .detached_root
            .clone()
            .ok_or(RuntimeError::NotDetached)?;
        self.validate_move(&view)?;
        let destination = if let Some(parent) = view.0.logical_parent() {
            self.check(&WidgetId(parent.clone()))?;
            NativeWindowId(parent.window())
        } else {
            self.primary_window()
        };
        if !destination.0.visible.get() {
            return Err(RuntimeError::Hidden);
        }
        if let Some(parent) = view.0.logical_parent()
            && !parent.visible.get()
        {
            return Err(RuntimeError::Hidden);
        }
        self.movement_cleanup(&view)?;
        // The detached window is retired below; a session in it ends first.
        if self.drag_in_window() {
            self.cancel_drag_infallibly();
        }
        let focus = self.window.return_focus.clone();
        self.move_presentation(&view, &destination);
        view.0.detached.set(false);
        self.apply_view_scale(view.0.serial, self.inherited_scale(view.0.serial));
        self.select_window(&destination)?;
        self.retire_window(window);
        if let Some(parent) = view.0.logical_parent() {
            if let Some(entry) = self.entries.get_mut(&parent.serial) {
                entry.component.child_redocked(
                    &mut EventContext {
                        node: parent.clone(),
                    },
                    &view,
                );
            }
            parent.invalidate(Invalidation::Layout);
            self.refresh_child_visibility(parent.serial);
        } else {
            self.refresh_visibility(view.0.serial, self.window.shared.visible.get());
        }
        if let Some(focus) = focus.filter(|f| f.0.mounted.get() && f.0.visible.get()) {
            *self.window.shared.requested_focus.borrow_mut() = Some(Rc::downgrade(&focus.0));
            self.window.shared.requested_within.set(false);
        }
        Ok(())
    }
    pub(super) fn retire_window(&mut self, window: &NativeWindowId) {
        window.0.mounted.set(false);
        self.remove_window_commands(window);
        // The selected window survives both retirement callers. Preserve its
        // queue first, then append the retired window's undelivered causes.
        if let Some(mut retired) = self.parked_windows.remove(&window.0.serial) {
            if self.shared.shutting_down.get() {
                self.shutdown_errors.extend(retired.drain_cleanup_errors());
            } else {
                self.window
                    .focus_errors
                    .extend(retired.drain_cleanup_errors());
            }
        }
        // Remove registry weak references too: retaining a stale public identity
        // must not make later theme/zoom work visit a retired presentation root.
        self.shared.windows.borrow_mut().retain(|candidate| {
            candidate
                .upgrade()
                .is_some_and(|window| window.mounted.get())
        });
    }
    /// Current detached root's content, absent for the main window.
    pub fn detached_view(&self) -> Option<WidgetId> {
        self.window.detached_root.clone()
    }
    /// Options of the selected detached window; the main window uses App defaults.
    pub fn native_window_options(&self) -> Option<&NativeWindowOptions> {
        self.window.options.as_ref()
    }
}

/// Recoverable native presentation failure. Existing windows remain available.
///
/// It sits above [`RuntimeError`]: every runtime failure, including invalid
/// geometry of a reported size or scale, is [`Self::Runtime`], and a
/// [`DrawError`](crate::DrawError) that holds a runtime error converts to that
/// same form. Backend causes are their diagnostic text.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum WindowError {
    /// Window allocation failed before presentation was moved.
    #[error("native window creation failed: {0}")]
    Creation(String),
    /// GPU/surface preparation failed before presentation was moved.
    #[error("native surface preparation failed: {0}")]
    Surface(String),
    /// Identity, modality or component cleanup rejected movement, or the native
    /// window reported a size or scale that yields no valid viewport, so
    /// presentation was not moved.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    /// The moved content could not be drawn. Never holds
    /// [`DrawError::Runtime`](crate::DrawError::Runtime); that is [`Self::Runtime`].
    #[error(transparent)]
    Drawing(crate::DrawError),
    /// The host discarded the request or shut down before completing it,
    /// including after a failure of its own unrelated to windows.
    #[error("native window request cancelled")]
    Cancelled,
}
impl From<WindowError> for crate::RuntimeError {
    /// Carries a window operation failure out of an application callback with `?`, as
    /// [`RuntimeError::Application`](crate::RuntimeError::Application) holding
    /// the whole error, downcastable through
    /// [`ApplicationError::downcast_ref`](crate::ApplicationError::downcast_ref).
    /// [`WindowError::Runtime`] becomes the runtime error it holds, so a runtime
    /// failure keeps one form. This is the error policy's callback-boundary
    /// exception; see [`RuntimeError`](crate::RuntimeError).
    fn from(error: WindowError) -> Self {
        match error {
            WindowError::Runtime(error) => error,
            error => Self::Application(crate::ApplicationError::new(error)),
        }
    }
}
impl From<crate::DrawError> for WindowError {
    /// [`DrawError::Runtime`](crate::DrawError::Runtime) becomes
    /// [`WindowError::Runtime`], so a runtime failure has one form.
    fn from(error: crate::DrawError) -> Self {
        match error {
            crate::DrawError::Runtime(error) => Self::Runtime(error),
            error => Self::Drawing(error),
        }
    }
}
crate::error::leaf_conversions!(WindowError => WindowError::Runtime);
/// Observable terminal result of a queued presentation operation.
#[derive(Clone, Debug, PartialEq)]
pub enum PresentationStatus {
    /// The host has not completed resource preparation/admission.
    Pending,
    /// Presentation now occupies this validated root.
    Presented(NativeWindowId),
    /// Native activation was requested, without an acknowledgement from the OS.
    ActivationRequested(NativeWindowId),
    /// Existing-window activation is unsupported by the native backend.
    ActivationUnsupported(NativeWindowId),
    /// The host accepted a close request and applied its configured policy.
    Closed,
    /// The host rejected closing, or the window uses KeepOpen policy.
    CloseRejected,
    /// The operation failed; prior successful editing cleanup is not rolled back.
    Failed(WindowError),
}
/// UI-local receipt for one native presentation request.
///
/// [`Self::status`] reads the owned status, which stays readable after runtime
/// shutdown: a request the shutdown discarded reports
/// `Failed(WindowError::Cancelled)`. [`Self::status_binding`] mirrors it as a
/// binding, so a component observes completion without polling while the
/// runtime lives. Retaining a receipt retains neither the request nor the
/// runtime.
///
/// Receipts compare equal when they report the same request: clones of one
/// receipt are equal, and two requests are different even when they end in the
/// same status. A signal holding the latest receipt therefore notifies its
/// readers for every new request.
#[derive(Clone)]
#[must_use]
pub struct PresentationReceipt {
    status: ReceiptStatus<PresentationStatus>,
}
impl PresentationReceipt {
    /// Pending or the retained terminal result, readable after shutdown.
    /// Reading creates no subscription or timer.
    #[must_use]
    pub fn status(&self) -> PresentationStatus {
        self.status.get()
    }
    /// The same status as a binding, so a component that reads it during
    /// update is invalidated when the request completes. Like every binding it
    /// reads `Err(RuntimeError::Closed)` after runtime shutdown; use
    /// [`Self::status`] then.
    pub fn status_binding(&self) -> ReadSignal<PresentationStatus> {
        self.status.binding()
    }
}
impl PartialEq for PresentationReceipt {
    fn eq(&self, other: &Self) -> bool {
        self.status.same(&other.status)
    }
}
impl Eq for PresentationReceipt {}
/// Shows the current status, as [`PresentationReceipt::status`] reads it.
impl std::fmt::Debug for PresentationReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresentationReceipt")
            .field("status", &self.status())
            .finish()
    }
}
/// A native operation requested from a component or ordinary application action.
#[derive(Clone, Debug)]
pub enum PresentationAction {
    /// Detach the exact mounted view after preparing a native window and surface.
    Detach {
        /// Stable view incarnation retaining its lifetime owner.
        view: WidgetId,
        /// Validated native options.
        options: NativeWindowOptions,
    },
    /// Return the detached root to its original lifetime owner.
    Redock(NativeWindowId),
    /// Request access to an existing native window without duplicating content.
    Activate(NativeWindowId),
    /// Ask the host close/save policy before destroying any native resource.
    Close(NativeWindowId),
    /// Show `title` as the native title of an open window, for a view whose
    /// title changed after its window opened. Requests for one window
    /// coalesce while queued, so the host applies only the latest title. The
    /// host completes it with [`PresentationStatus::Presented`] once the title
    /// is applied. The window's [`NativeWindowOptions`] keep the title it
    /// opened with.
    Retitle {
        /// The open window to retitle.
        window: NativeWindowId,
        /// The new nonblank title.
        title: String,
    },
}
/// Queued operation for a native host. Dropping an unfinished request cancels it.
/// Hosts must prepare resources before committing movement and report the actual
/// result. The default App consumes these automatically on the UI thread.
#[must_use]
pub struct PresentationRequest {
    action: PresentationAction,
    state: Option<ReceiptStatus<PresentationStatus>>,
}
/// Shows the requested action and whether the request is still unfinished.
impl std::fmt::Debug for PresentationRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresentationRequest")
            .field("action", &self.action)
            .field("pending", &self.state.is_some())
            .finish()
    }
}
impl PresentationRequest {
    /// Requested operation and stable target identities.
    pub fn action(&self) -> &PresentationAction {
        &self.action
    }
    /// Completes a native operation with the actual result. Pending is rejected.
    /// Binding observers are invalidated automatically. Do not report success
    /// before committing runtime movement or claim observed OS activation.
    ///
    /// # Errors
    /// Returns [`RuntimeError::StalePresentation`] for
    /// [`PresentationStatus::Pending`], which cancels the request.
    pub fn complete(mut self, status: PresentationStatus) -> Result<(), RuntimeError> {
        if status == PresentationStatus::Pending {
            return Err(RuntimeError::StalePresentation);
        }
        if let Some(state) = self.state.take() {
            state.set(status);
        }
        Ok(())
    }
}
impl Drop for PresentationRequest {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            // The owned status records the cancellation even after shutdown,
            // when the binding's observers are already detached.
            state.set(PresentationStatus::Failed(WindowError::Cancelled));
        }
    }
}
/// Queues a retitle of `window`, or gives a retitle still queued for it the
/// new title.
fn queue_title(
    shared: &Rc<Shared>,
    window: &NativeWindowId,
    mounted: bool,
    title: String,
) -> Result<PresentationReceipt, RuntimeError> {
    if title.trim().is_empty() {
        return Err(RuntimeError::EmptyWindowTitle);
    }
    if !mounted {
        return Err(RuntimeError::Unmounted);
    }
    shared.writable()?;
    for request in shared.presentation_requests.borrow_mut().iter_mut() {
        if let PresentationAction::Retitle {
            window: queued,
            title: queued_title,
        } = &mut request.action
            && *queued == *window
            && let Some(status) = &request.state
        {
            *queued_title = title;
            return Ok(PresentationReceipt {
                status: status.clone(),
            });
        }
    }
    enqueue(
        shared,
        PresentationAction::Retitle {
            window: window.clone(),
            title,
        },
    )
}
fn enqueue(
    shared: &Rc<Shared>,
    action: PresentationAction,
) -> Result<PresentationReceipt, RuntimeError> {
    shared.writable()?;
    if shared.presentation_requests.borrow().len() >= Runtime::MAX_WINDOW_REQUESTS {
        return Err(RuntimeError::WindowRequestFull);
    }
    let status = ReceiptStatus::new(shared.clone(), PresentationStatus::Pending);
    shared
        .presentation_requests
        .borrow_mut()
        .push(PresentationRequest {
            action,
            state: Some(status.clone()),
        });
    Ok(PresentationReceipt { status })
}
impl Runtime {
    /// Queues native activation of a mounted presentation root on behalf of a
    /// runtime operation, such as a reveal into another window.
    pub(super) fn request_window_activation(
        &self,
        window: &NativeWindowId,
    ) -> Result<PresentationReceipt, RuntimeError> {
        if !window.0.mounted.get() {
            return Err(RuntimeError::Unmounted);
        }
        enqueue(&self.shared, PresentationAction::Activate(window.clone()))
    }
    /// Maximum number of native presentation requests waiting for the host.
    ///
    /// One queue holds every detach, re-dock, close, activation and retitle
    /// request, from
    /// [`Self::request_detach`], [`Self::request_redock`], [`Self::request_close`],
    /// [`Self::request_window_title`] and the [`EventContext`] methods [`EventContext::detach_view`],
    /// [`EventContext::redock_window`], [`EventContext::close_window`],
    /// [`EventContext::activate_window`] and [`EventContext::set_window_title`].
    /// A request beyond it is refused with
    /// [`RuntimeError::WindowRequestFull`] without being queued; taking the
    /// requests with [`Self::take_presentation_requests`] frees the queue.
    pub const MAX_WINDOW_REQUESTS: usize = 64;

    /// Queues a native detach; the default App prepares resources before movement.
    /// Observe the receipt for typed native/cleanup
    /// failures. Headless hosts consume [`Self::take_presentation_requests`].
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `ForeignRuntime` or `Unmounted` for a foreign or unmounted view; `Hidden` for a
    /// hidden view; `NotDetachable` when its description forbids its own window;
    /// `ModalBlocked` when the subtree holds an open modal dialog; and
    /// [`RuntimeError::WindowRequestFull`](crate::RuntimeError::WindowRequestFull) when
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued.
    pub fn request_detach(
        &self,
        view: &WidgetId,
        options: NativeWindowOptions,
    ) -> Result<PresentationReceipt, RuntimeError> {
        self.check(view)?;
        permitted(view)?;
        self.validate_move(view)?;
        enqueue(
            &self.shared,
            PresentationAction::Detach {
                view: view.clone(),
                options,
            },
        )
    }
    /// Queues return to the original owner; failure leaves the detached root open.
    ///
    /// # Errors
    /// Returns `Unmounted` for a window identity that is not an available presentation
    /// root, [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::WindowRequestFull`](crate::RuntimeError::WindowRequestFull) when
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued.
    pub fn request_redock(
        &self,
        window: &NativeWindowId,
    ) -> Result<PresentationReceipt, RuntimeError> {
        if !self.contains_window(window) {
            return Err(RuntimeError::Unmounted);
        }
        enqueue(&self.shared, PresentationAction::Redock(window.clone()))
    }
    /// Takes native work once, preserving queued order. Only native hosts need
    /// this API; ordinary applications use App. Dropping requests cancels receipts.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase.
    pub fn take_presentation_requests(&self) -> Result<Vec<PresentationRequest>, RuntimeError> {
        self.shared.writable()?;
        Ok(std::mem::take(
            &mut *self.shared.presentation_requests.borrow_mut(),
        ))
    }
    /// Reads UI-local state without subscribing.
    /// Native hosts use this for deferred admission; it never schedules polling.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for a signal of another runtime, `Closed` after shutdown
    /// and `BorrowConflict` when the value is being written.
    pub fn read_signal<T: Clone + 'static>(
        &self,
        signal: &ReadSignal<T>,
    ) -> Result<T, RuntimeError> {
        if !Rc::ptr_eq(&self.shared, signal.shared()) {
            return Err(RuntimeError::ForeignRuntime);
        }
        signal.get()
    }
    /// Requests close/save admission. The main window asks for application exit;
    /// detached windows use their configured close policy after acceptance.
    ///
    /// # Errors
    /// Returns `Unmounted` for a window identity that is not an available presentation
    /// root, [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::WindowRequestFull`](crate::RuntimeError::WindowRequestFull) when
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued.
    pub fn request_close(
        &self,
        window: &NativeWindowId,
    ) -> Result<PresentationReceipt, RuntimeError> {
        if !self.contains_window(window) {
            return Err(RuntimeError::Unmounted);
        }
        enqueue(&self.shared, PresentationAction::Close(window.clone()))
    }
    /// Queues `title` as the native title of the open window `window`, with the
    /// coalescing and errors of [`EventContext::set_window_title`], for
    /// application code outside a component, such as a main window title that
    /// names the open document.
    ///
    /// # Errors
    /// Returns [`RuntimeError::EmptyWindowTitle`] for a blank title, `Unmounted`
    /// for a window identity that is not an available presentation root,
    /// [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown,
    /// the phase errors of [`Self::request_close`], and
    /// [`RuntimeError::WindowRequestFull`](crate::RuntimeError::WindowRequestFull)
    /// when [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued and
    /// none of them retitles this window.
    pub fn request_window_title(
        &self,
        window: &NativeWindowId,
        title: impl Into<String>,
    ) -> Result<PresentationReceipt, RuntimeError> {
        queue_title(
            &self.shared,
            window,
            self.contains_window(window),
            title.into(),
        )
    }
    /// Applies accepted close policy to the selected detached window. Main-window
    /// shutdown belongs to the native host. Failed re-dock leaves content available.
    ///
    /// # Errors
    /// Returns `NotDetached` when the selected window is the main window. The `Redock`
    /// policy forwards the errors of [`Self::redock`] and the `RemoveView` policy those of
    /// [`Self::unmount`].
    pub fn close_detached_window(&mut self) -> Result<PresentationStatus, RuntimeError> {
        let policy = self
            .native_window_options()
            .ok_or(RuntimeError::NotDetached)?
            .closing_policy();
        match policy {
            DetachedClosePolicy::Redock => {
                self.redock(&self.current_window())?;
            }
            DetachedClosePolicy::RemoveView => {
                let view = self.detached_view().ok_or(RuntimeError::NotDetached)?;
                self.unmount(&view)?;
            }
            DetachedClosePolicy::KeepOpen => return Ok(PresentationStatus::CloseRejected),
        }
        Ok(PresentationStatus::Closed)
    }
    /// Whether the native host has presentation operations to process.
    pub fn has_presentation_requests(&self) -> bool {
        !self.shared.presentation_requests.borrow().is_empty()
    }
}
impl EventContext {
    /// Window receiving this component event, independent of lifetime ownership.
    pub fn native_window(&self) -> NativeWindowId {
        NativeWindowId(self.node.window())
    }
    /// Queues detachment of a retained view. Its lifetime owner is unchanged.
    /// Observe the receipt for native failure; no GPU work runs inside the callback.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` or `Unmounted` for a foreign or unmounted view,
    /// `NotDetachable` when its description forbids its own window,
    /// [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::WindowRequestFull`](crate::RuntimeError::WindowRequestFull) when
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued. Hidden views and modal
    /// subtrees are refused later, through the receipt.
    pub fn detach_view(
        &self,
        view: &WidgetId,
        options: NativeWindowOptions,
    ) -> Result<PresentationReceipt, RuntimeError> {
        if !Rc::ptr_eq(&view.0.shared, &self.node.shared) {
            return Err(RuntimeError::ForeignRuntime);
        }
        if !view.0.mounted.get() {
            return Err(RuntimeError::Unmounted);
        }
        permitted(view)?;
        enqueue(
            &self.node.shared,
            PresentationAction::Detach {
                view: view.clone(),
                options,
            },
        )
    }
    /// Requests host close/save admission for this native window. The main window
    /// requests application exit; detached windows apply their configured policy.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::WindowRequestFull`](crate::RuntimeError::WindowRequestFull) when
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued.
    pub fn close_window(&self) -> Result<PresentationReceipt, RuntimeError> {
        enqueue(
            &self.node.shared,
            PresentationAction::Close(self.native_window()),
        )
    }
    /// Queues return of this event's detached window to its original owner.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::WindowRequestFull`](crate::RuntimeError::WindowRequestFull) when
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued. A window that is not
    /// detached is refused later, through the receipt.
    pub fn redock_window(&self) -> Result<PresentationReceipt, RuntimeError> {
        enqueue(
            &self.node.shared,
            PresentationAction::Redock(self.native_window()),
        )
    }
    /// Queues `title` as the native title of the open window `window`, such as
    /// a detached view whose title changed. A request still queued for the
    /// same window takes the new title instead of queuing another, and the
    /// returned receipt observes that request. Tabs and tool windows call this
    /// for their detached views; an application can call it for its own.
    ///
    /// # Errors
    /// Returns [`RuntimeError::EmptyWindowTitle`] for a blank title, `Unmounted`
    /// for a window identity that is not a mounted presentation root of this
    /// runtime, [`RuntimeError::Closed`](crate::RuntimeError::Closed) after
    /// shutdown, `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside
    /// that phase, and [`RuntimeError::WindowRequestFull`] when
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued and none of
    /// them retitles this window.
    pub fn set_window_title(
        &self,
        window: &NativeWindowId,
        title: impl Into<String>,
    ) -> Result<PresentationReceipt, RuntimeError> {
        let mounted = self
            .node
            .shared
            .windows
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .any(|w| Rc::ptr_eq(&w, &window.0) && w.mounted.get());
        queue_title(&self.node.shared, window, mounted, title.into())
    }
    /// Requests native access without assuming OS focus or restoration succeeds.
    ///
    /// # Errors
    /// Returns `Unmounted` for a window identity that is not a mounted presentation root of
    /// this runtime, [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown,
    /// and `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::WindowRequestFull`](crate::RuntimeError::WindowRequestFull) when
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests are already queued.
    pub fn activate_window(
        &self,
        window: &NativeWindowId,
    ) -> Result<PresentationReceipt, RuntimeError> {
        if !self
            .node
            .shared
            .windows
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .any(|w| Rc::ptr_eq(&w, &window.0) && w.mounted.get())
        {
            return Err(RuntimeError::Unmounted);
        }
        enqueue(
            &self.node.shared,
            PresentationAction::Activate(window.clone()),
        )
    }
}
impl ChildInfo {
    /// Detached presentation root, or None when the child is laid out inline.
    /// Ownership and identity are retained in both cases.
    pub fn detached_window(&self) -> Option<NativeWindowId> {
        self.id
            .0
            .detached
            .get()
            .then(|| NativeWindowId(self.id.0.window()))
    }
}

impl WindowData {
    // Focus failures retain FIFO order, followed by the earliest coalesced
    // surface failure. This does not imply cross-window chronological ordering.
    pub(super) fn drain_cleanup_errors(&mut self) -> impl Iterator<Item = RuntimeError> + '_ {
        self.focus_errors.drain(..).chain(self.surface_error.take())
    }
    fn deadline(&self, now: Duration) -> Option<Duration> {
        if !self.shared.visible.get() {
            return None;
        }
        if !self.focus_errors.is_empty()
            || self.surface_error.is_some()
            || self.restore_focus
            || self.shared.stack_hover_pending.get()
            || !self.shared.pending.borrow().is_empty()
            || self.shared.layout_pending.get()
        {
            Some(now)
        } else {
            let update = self
                .shared
                .deadlines
                .borrow()
                .first_key_value()
                .map(|((time, _), _)| *time);
            let timer = self
                .shared
                .timers
                .borrow()
                .first_key_value()
                .map(|((time, _), _)| *time);
            match (update, timer) {
                (Some(update), Some(timer)) => Some(update.min(timer)),
                (value, None) | (None, value) => value,
            }
        }
    }
}
impl Runtime {
    /// Earliest eligible work across all roots, independent of selected/focused
    /// window. Known hidden roots are excluded. Native hosts additionally account
    /// for OS redraw availability and recovery deadlines; reading never polls.
    pub fn application_deadline(&self) -> Option<Duration> {
        if !self.shared.tasks.borrow().is_empty()
            || self.drag_needs_service()
            || !self.shared.computations.borrow().is_empty()
            || self.has_presentation_requests()
            || !self.shared.command_requests.borrow().is_empty()
            || !self.shared.requested_removals.borrow().is_empty()
            || !self.shared.requested_moves.borrow().is_empty()
            || !self.shared.requested_children.borrow().is_empty()
            || !self.shared.requested_text_commands.borrow().is_empty()
            || !self.shared.reveal_requests.borrow().is_empty()
            || self.shared.notifications_due()
        {
            return Some(self.now());
        }
        // A stretched component is prepared by whichever visible root flushes
        // first; with every root hidden, nothing is due.
        let presentable = std::iter::once(&self.window)
            .chain(self.parked_windows.values())
            .any(|window| window.shared.visible.get());
        std::iter::once(&self.window)
            .chain(self.parked_windows.values())
            .filter_map(|window| window.deadline(self.now()))
            .chain(self.shared.source_deadline(self.now()))
            .chain(self.interval_deadline())
            .chain(self.shared.scale_settle_deadline().filter(|_| presentable))
            .min()
    }
    /// Flushes eligible roots at a shared time, retaining the selected root.
    /// Only each root's pending components run; static roots do not rebuild.
    /// Returns the total update count and earliest application deadline.
    /// When every root is hidden, deferred tasks are still delivered once, so a
    /// task cannot keep [`Self::application_deadline`] due without being run.
    ///
    /// # Errors
    /// Returns the first error of [`Self::flush`] for a visible root, or of a
    /// task delivered while every root is hidden; the remaining roots
    /// are not flushed and the previously selected root is selected again. `Unmounted` when
    /// a root cannot be selected.
    pub fn flush_windows(&mut self, now: Duration) -> Result<FlushReport, RuntimeError> {
        let previous = self.current_window();
        let mut count = 0;
        // Inspection is answered once, after every root, so a reply describes
        // the whole application; and even when every root is hidden, so a
        // request does not keep the application due forever. Nested inside
        // another boundary, such as a queued command's flush, it answers
        // nothing; the boundary is released on unwinding too.
        let nested = self.shared.answer_depth.get() > 0;
        if !nested {
            // A new outermost flush of every root describes its own failure.
            self.shared.reset_failure();
        }
        let result = {
            let _boundary = super::AnswerBoundary::new(self.shared.clone());
            (|| {
                let mut flushed = false;
                for window in self.native_windows() {
                    if !self.contains_window(&window) {
                        continue;
                    }
                    self.select_window(&window)?;
                    if window.0.visible.get() {
                        flushed = true;
                        count += self.flush(now)?.components_updated;
                    }
                }
                // With every root hidden no flush delivers deferred tasks, and
                // a queued task would keep the application due forever: deliver
                // them here once. A task owned by a widget that is not shown is
                // cancelled, as `EventContext::defer` documents.
                if !flushed && !self.shared.tasks.borrow().is_empty() {
                    self.dispatch_tasks()?;
                }
                // Interval timers are the application's, not a window's: with
                // every root hidden they still run.
                if !flushed {
                    self.service_intervals(now)?;
                }
                Ok(FlushReport {
                    components_updated: count,
                    next_deadline: self.application_deadline(),
                })
            })()
        };
        if self.contains_window(&previous) {
            self.select_window(&previous)?;
        }
        if nested {
            return result;
        }
        let flush_error = result.as_ref().err();
        let shared = self.shared.clone();
        shared.answer_inspection(self, flush_error);
        result.map(|report| FlushReport {
            next_deadline: self.application_deadline(),
            ..report
        })
    }
    pub(super) fn window_inspections(&self) -> Vec<crate::WindowInspection> {
        let mut windows: Vec<_> = std::iter::once(&self.window)
            .chain(self.parked_windows.values())
            .map(|window| crate::WindowInspection {
                id: window.shared.serial,
                viewport: window.shared.viewport.get(),
                presentable: window.shared.visible.get(),
                focused: window.shared.focused.get(),
                command_target: window
                    .shared
                    .command_target
                    .borrow()
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .filter(|node| {
                        node.mounted.get()
                            && node.visible.get()
                            && Rc::ptr_eq(&node.window(), &window.shared)
                    })
                    .map(|node| node.serial),
                next_deadline: window.deadline(self.now()),
            })
            .collect();
        windows.sort_by_key(|w| w.id);
        windows
    }
}
