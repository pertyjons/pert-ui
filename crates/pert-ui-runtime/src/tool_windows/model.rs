//! The state a tool window layout shares with its stripes, panels and
//! dividers: which tool windows exist, which are expanded, their sizes and
//! the changes not yet reported.
use super::{
    EdgeMode, StripeGroup, ToolEdge, ToolWindowChange, ToolWindowChangeKind, ToolWindowMode,
};
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};

/// A value per edge, addressed by [`ToolEdge`] rather than by position.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct PerEdge<T> {
    pub(super) left: T,
    pub(super) right: T,
    pub(super) top: T,
    pub(super) bottom: T,
}
impl<T> PerEdge<T> {
    pub(super) fn get(&self, edge: ToolEdge) -> &T {
        match edge {
            ToolEdge::Left => &self.left,
            ToolEdge::Right => &self.right,
            ToolEdge::Top => &self.top,
            ToolEdge::Bottom => &self.bottom,
        }
    }
    pub(super) fn get_mut(&mut self, edge: ToolEdge) -> &mut T {
        match edge {
            ToolEdge::Left => &mut self.left,
            ToolEdge::Right => &mut self.right,
            ToolEdge::Top => &mut self.top,
            ToolEdge::Bottom => &mut self.bottom,
        }
    }
}

/// One registered tool window as the layout keeps it.
pub(super) struct Window {
    pub(super) key: String,
    /// The described title.
    pub(super) title_source: TextSource,
    /// The title's current plain text: the fixed title, or the binding's text
    /// as the layout or a stripe last read it.
    pub(super) title: String,
    pub(super) icon: Option<IconSource>,
    pub(super) edge: ToolEdge,
    pub(super) group: StripeGroup,
    pub(super) expanded: bool,
    /// The extent across the edge the user dragged or a restore applied; the
    /// described or default extent applies while it is `None`.
    pub(super) extent: Option<f32>,
    /// The document number a restore applied, captured again unchanged while
    /// `extent` still holds the value it produced.
    pub(super) restored: Option<(f32, f64)>,
    /// The extent the description asked for, if any.
    pub(super) described_extent: Option<f32>,
    pub(super) resizable: Permission,
    pub(super) toggles: Permission,
    pub(super) movable: Permission,
    pub(super) reorderable: Permission,
    pub(super) draggable: Permission,
    pub(super) overlayable: Permission,
    pub(super) floatable: Permission,
    pub(super) detachable: Permission,
    /// Whether the expanded panel slides over the center instead of taking
    /// space from it, and hides when focus leaves it. Applies while docked.
    pub(super) overlay: bool,
    /// Where the expanded tool window is presented: docked at its edge,
    /// floating over the layout, or in its own native window.
    pub(super) mode: ToolWindowMode,
    /// The floating frame's position and size in the layout's coordinates,
    /// once the user, the application or a restore set it; a default frame
    /// applies while it is `None`.
    pub(super) floating: Option<Rect>,
    /// The document numbers a restore applied, captured again unchanged
    /// while `floating` still holds the frame they produced.
    pub(super) restored_floating: Option<(Rect, [f64; 4])>,
    /// Whether the user, the application or a restore moved the tool window
    /// to its edge and group; until then it follows its description.
    pub(super) placed: bool,
    /// Whether the user, the application or a restore chose its mode; until
    /// then it follows its description.
    pub(super) mode_chosen: bool,
    /// Whether focus entered its panel since it last expanded: only then
    /// does an overlay panel hide when focus leaves.
    pub(super) focus_entered: bool,
    /// The edge, group, mode and position the description gave; a restore
    /// returns to them for what its document leaves out.
    pub(super) described_edge: ToolEdge,
    pub(super) described_group: StripeGroup,
    pub(super) described_overlay: bool,
    pub(super) described_mode: ToolWindowMode,
    pub(super) described_index: usize,
}

/// The arranged extent of an edge's expanded panels and the range the
/// divider may move them in, both across the edge.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct EdgeGeometry {
    pub(super) extent: f32,
    pub(super) minimum: f32,
    pub(super) maximum: f32,
}

/// Everything the layout and its parts share. Only ever borrowed briefly and
/// never across a call into application code.
pub(super) struct Model {
    pub(super) windows: Vec<Window>,
    pub(super) edges: PerEdge<EdgeMode>,
    pub(super) resizable: Permission,
    pub(super) toggles: Permission,
    pub(super) movable: Permission,
    pub(super) reorderable: Permission,
    pub(super) draggable: Permission,
    pub(super) overlayable: Permission,
    pub(super) floatable: Permission,
    pub(super) detachable: Permission,
    /// The native windows presenting detached panels, by tool window key, as
    /// the layout last saw its children.
    pub(super) detached: BTreeMap<String, NativeWindowId>,
    /// What each detached panel's window shows, so a changed tool window
    /// title retitles it.
    pub(super) window_titles: crate::window_titles::WindowTitles,
    /// Detach requests not yet presented, by tool window key.
    pub(super) detaching: BTreeMap<String, PresentationReceipt>,
    /// Detached panels that asked their window to return, with the request's
    /// receipt; cleared when it has returned or the request failed.
    pub(super) redocking: BTreeMap<String, PresentationReceipt>,
    /// Each panel's notifier, so a detached panel hears that it must return.
    pub(super) panel_notifiers: BTreeMap<String, Notifier>,
    /// Coalesces idempotent detached-panel wakeups until delivery or removal.
    pub(super) panel_pending: RefCell<BTreeSet<String>>,
    /// Whether the order of tool windows was changed after the description,
    /// by the user, the application or a restore; until then the
    /// description's order applies.
    pub(super) reordered: bool,
    /// Whether one of this layout's stripes holds focus. An overlay panel
    /// stays open while focus moves from it to a stripe, whose button then
    /// decides; it hides when focus is in neither.
    pub(super) stripe_focus: bool,
    pub(super) names: bool,
    /// The application's binding for `names`, when it owns the setting.
    pub(super) names_binding: Option<Signal<bool>>,
    /// The arranged geometry per edge, `None` while no panel is expanded there.
    pub(super) geometry: PerEdge<Option<EdgeGeometry>>,
    /// Changes not yet delivered to `on_change`.
    pub(super) deliveries: RefCell<VecDeque<Option<ToolWindowChange>>>,
    pub(super) presentation_dirty: Cell<bool>,
    /// The tool window whose panel holds focus, as its panel last heard.
    pub(super) focused_panel: Option<String>,
    /// Whether focus must return to the center at the next delivery, because
    /// the tool window that held it collapsed.
    pub(super) return_focus: bool,
    /// Whether a divider is being dragged or resized with held keys.
    pub(super) resizing: PerEdge<bool>,
    pub(super) floating_interactions: BTreeSet<String>,
    /// Whether the layout has run its first update; changes before it, such
    /// as a restore during mounting, are applied but not reported.
    pub(super) started: bool,
    pub(super) notifier: Option<Notifier>,
    /// Incremented at each delivery so the stripes and panels repaint.
    pub(super) revision: Option<Signal<u64>>,
    pub(super) center: Option<WidgetId>,
    pub(super) panels: BTreeMap<String, WidgetId>,
}
impl Model {
    pub(super) fn new() -> Self {
        Self {
            windows: Vec::new(),
            edges: PerEdge::default(),
            resizable: Permission::allowed(),
            toggles: Permission::allowed(),
            movable: Permission::allowed(),
            reorderable: Permission::allowed(),
            draggable: Permission::allowed(),
            overlayable: Permission::allowed(),
            floatable: Permission::allowed(),
            detachable: Permission::allowed(),
            detached: BTreeMap::new(),
            window_titles: crate::window_titles::WindowTitles::default(),
            detaching: BTreeMap::new(),
            redocking: BTreeMap::new(),
            panel_notifiers: BTreeMap::new(),
            panel_pending: RefCell::new(BTreeSet::new()),
            reordered: false,
            stripe_focus: false,
            names: false,
            names_binding: None,
            geometry: PerEdge::default(),
            deliveries: RefCell::new(VecDeque::new()),
            presentation_dirty: Cell::new(false),
            focused_panel: None,
            return_focus: false,
            resizing: PerEdge::default(),
            floating_interactions: BTreeSet::new(),
            started: false,
            notifier: None,
            revision: None,
            center: None,
            panels: BTreeMap::new(),
        }
    }
    pub(super) fn window(&self, key: &str) -> Option<&Window> {
        self.windows.iter().find(|window| window.key == key)
    }
    fn window_mut(&mut self, key: &str) -> Option<&mut Window> {
        self.windows.iter_mut().find(|window| window.key == key)
    }
    pub(super) fn is_expanded(&self, key: &str) -> bool {
        self.window(key).is_some_and(|window| window.expanded)
    }
    /// The expanded docked tool windows of an edge, start group first: the
    /// ones that take space from the center.
    pub(super) fn open_on(&self, edge: ToolEdge) -> Vec<&Window> {
        let mut open: Vec<&Window> = self
            .windows
            .iter()
            .filter(|window| {
                window.edge == edge
                    && window.expanded
                    && window.mode == ToolWindowMode::Docked
                    && !window.overlay
            })
            .collect();
        open.sort_by_key(|window| window.group == StripeGroup::End);
        open
    }
    /// The expanded overlay tool windows of an edge, start group first.
    pub(super) fn overlays_on(&self, edge: ToolEdge) -> Vec<&Window> {
        let mut open: Vec<&Window> = self
            .windows
            .iter()
            .filter(|window| {
                window.edge == edge
                    && window.expanded
                    && window.mode == ToolWindowMode::Docked
                    && window.overlay
            })
            .collect();
        open.sort_by_key(|window| window.group == StripeGroup::End);
        open
    }
    /// The tool windows of one edge group in stripe order.
    pub(super) fn group_keys(&self, edge: ToolEdge, group: StripeGroup) -> Vec<String> {
        self.windows
            .iter()
            .filter(|window| window.edge == edge && window.group == group)
            .map(|window| window.key.clone())
            .collect()
    }
    /// The layout's and the tool window's permission for one capability,
    /// combined so that the stricter wins.
    fn combined(
        &self,
        key: &str,
        layout: &Permission,
        pick: fn(&Window) -> &Permission,
    ) -> Permission {
        self.window(key).map_or_else(Permission::denied, |window| {
            layout.clone().and(pick(window).clone())
        })
    }
    /// Whether the user may move this tool window to another edge or group.
    pub(super) fn move_permission(&self, key: &str) -> Permission {
        self.combined(key, &self.movable, |window| &window.movable)
    }
    /// Whether the user may reorder this tool window within its group.
    pub(super) fn reorder_permission(&self, key: &str) -> Permission {
        self.combined(key, &self.reorderable, |window| &window.reorderable)
    }
    /// Whether the user may drag this tool window's stripe button.
    pub(super) fn drag_permission(&self, key: &str) -> Permission {
        self.combined(key, &self.draggable, |window| &window.draggable)
    }
    /// Whether the user may switch this tool window between docked and
    /// overlay.
    pub(super) fn overlay_permission(&self, key: &str) -> Permission {
        self.combined(key, &self.overlayable, |window| &window.overlayable)
    }
    /// Whether the user may switch this tool window between docked and
    /// floating.
    pub(super) fn float_permission(&self, key: &str) -> Permission {
        self.combined(key, &self.floatable, |window| &window.floatable)
    }
    /// Whether the user may switch this tool window into and out of its own
    /// native window.
    pub(super) fn detach_permission(&self, key: &str) -> Permission {
        self.combined(key, &self.detachable, |window| &window.detachable)
    }
    /// Whether the user may switch this tool window to `mode` from its
    /// current mode: leaving or entering floating needs the float
    /// permission, leaving or entering a native window the detach permission.
    pub(super) fn mode_permission(&self, key: &str, mode: ToolWindowMode) -> Permission {
        let Some(window) = self.window(key) else {
            return Permission::denied();
        };
        let need = |mode: ToolWindowMode| match mode {
            ToolWindowMode::Docked => Permission::allowed(),
            ToolWindowMode::Floating => self.float_permission(key),
            ToolWindowMode::Window => self.detach_permission(key),
        };
        need(window.mode).and(need(mode))
    }
    /// Whether the user may expand and collapse this tool window: the
    /// layout's and the tool window's permission both allow.
    pub(super) fn toggle_permission(&self, key: &str) -> Permission {
        self.window(key).map_or_else(Permission::denied, |window| {
            self.toggles.clone().and(window.toggles.clone())
        })
    }
    /// Whether the user may resize an edge: the layout and every tool window
    /// expanded there allow.
    pub(super) fn resize_permission(&self, edge: ToolEdge) -> Permission {
        self.open_on(edge)
            .into_iter()
            .fold(self.resizable.clone(), |permission, window| {
                permission.and(window.resizable.clone())
            })
    }
    /// Expands or collapses a tool window. Expanding collapses the other tool
    /// window expanded in the same edge group. Records every change for
    /// `on_change` and asks for delivery; returns whether anything changed.
    pub(super) fn set_expanded(&mut self, key: &str, expanded: bool, origin: ChangeOrigin) -> bool {
        let Some(window) = self.window(key) else {
            return false;
        };
        if window.expanded == expanded {
            return false;
        }
        let (edge, group, docked) = (
            window.edge,
            window.group,
            window.mode == ToolWindowMode::Docked,
        );
        if expanded && docked {
            let others: Vec<String> = self
                .windows
                .iter()
                .filter(|other| {
                    other.key != key
                        && other.edge == edge
                        && other.group == group
                        && other.expanded
                        && other.mode == ToolWindowMode::Docked
                })
                .map(|other| other.key.clone())
                .collect();
            for other in others {
                self.apply(&other, false, origin);
            }
        }
        self.apply(key, expanded, origin);
        self.request();
        true
    }
    fn apply(&mut self, key: &str, expanded: bool, origin: ChangeOrigin) {
        if let Some(window) = self.window_mut(key) {
            window.expanded = expanded;
            window.focus_entered = false;
        }
        if !expanded && self.focused_panel.as_deref() == Some(key) {
            self.return_focus = true;
        }
        self.record(key, ToolWindowChangeKind::Expansion, origin);
    }
    /// Records a change of `key` for `on_change`, with its current state.
    pub(super) fn record(&mut self, key: &str, kind: ToolWindowChangeKind, origin: ChangeOrigin) {
        if !self.started {
            return;
        }
        let Some(window) = self.window(key) else {
            return;
        };
        let expansion = if window.expanded {
            super::Expansion::Expanded
        } else {
            super::Expansion::Collapsed
        };
        let change = ToolWindowChange::new(key.to_owned(), expansion, origin)
            .with_kind(kind)
            .with_placement(window.edge, window.group)
            .with_overlay(window.overlay)
            .with_mode(window.mode)
            .with_geometry(window.extent, window.floating);
        if let Some(notifier) = &self.notifier {
            if kind == ToolWindowChangeKind::Geometry
                && let Some(queued) = self.deliveries.borrow_mut().iter_mut().find(|queued| {
                    queued.as_ref().is_some_and(|queued| {
                        queued.key == key && queued.kind == ToolWindowChangeKind::Geometry
                    })
                })
            {
                // One delivery per tool window's geometry: the last value
                // and origin replace a change not yet delivered.
                *queued = Some(change);
                return;
            }
            self.presentation_dirty.set(true);
            self.deliveries.borrow_mut().push_back(Some(change));
            notifier.request();
        }
    }
    /// Moves a tool window to `edge` and `group` at `index` among that
    /// group's other tool windows (`None` or past the end appends). An
    /// expanded tool window stays expanded and collapses the one expanded in
    /// its new group. Returns whether anything changed; the caller has
    /// checked that `edge` holds panels.
    pub(super) fn move_window(
        &mut self,
        key: &str,
        edge: ToolEdge,
        group: StripeGroup,
        index: Option<usize>,
        origin: ChangeOrigin,
    ) -> bool {
        let Some(from) = self.windows.iter().position(|window| window.key == key) else {
            return false;
        };
        let (old_edge, old_group, expanded) = {
            let window = &self.windows[from];
            (
                window.edge,
                window.group,
                window.expanded && window.mode == ToolWindowMode::Docked,
            )
        };
        let before = self.group_keys(edge, group);
        let mut window = self.windows.remove(from);
        window.edge = edge;
        window.group = group;
        // Before the `index`-th remaining member of the target group, or
        // after its last member; every other tool window keeps its order.
        let members: Vec<usize> = self
            .windows
            .iter()
            .enumerate()
            .filter(|(_, other)| other.edge == edge && other.group == group)
            .map(|(position, _)| position)
            .collect();
        let at = match index {
            Some(index) if index < members.len() => members[index],
            _ => members.last().map_or(self.windows.len(), |last| last + 1),
        };
        self.windows.insert(at.min(self.windows.len()), window);
        let moved = old_edge != edge || old_group != group;
        if !moved && before == self.group_keys(edge, group) {
            return false;
        }
        if moved && let Some(window) = self.window_mut(key) {
            window.placed = true;
        }
        self.reordered = true;
        if moved && expanded {
            let others: Vec<String> = self
                .windows
                .iter()
                .filter(|other| {
                    other.key != key
                        && other.edge == edge
                        && other.group == group
                        && other.expanded
                        && other.mode == ToolWindowMode::Docked
                })
                .map(|other| other.key.clone())
                .collect();
            for other in others {
                self.apply(&other, false, origin);
            }
        }
        self.record(key, ToolWindowChangeKind::Placement, origin);
        self.request();
        true
    }
    /// Hides every expanded overlay tool window that focus entered and that
    /// holds focus no longer, unless focus is on one of the layout's stripes.
    /// Called at each delivery.
    pub(super) fn hide_left_overlays(&mut self) {
        if self.stripe_focus {
            return;
        }
        let left: Vec<String> = self
            .windows
            .iter()
            .filter(|window| {
                window.overlay
                    && window.mode == ToolWindowMode::Docked
                    && window.expanded
                    && window.focus_entered
                    && self.focused_panel.as_deref() != Some(window.key.as_str())
            })
            .map(|window| window.key.clone())
            .collect();
        for key in left {
            self.set_expanded(&key, false, ChangeOrigin::User);
        }
    }
    /// Switches a tool window between docked and overlay. Returns whether
    /// anything changed.
    pub(super) fn set_overlay(&mut self, key: &str, overlay: bool, origin: ChangeOrigin) -> bool {
        let Some(window) = self.window_mut(key) else {
            return false;
        };
        if window.overlay == overlay {
            return false;
        }
        window.overlay = overlay;
        window.mode_chosen = true;
        self.record(key, ToolWindowChangeKind::Mode, origin);
        self.request();
        true
    }
    /// Switches a tool window between docked, floating and its own window.
    /// A tool window that becomes docked while expanded collapses the one
    /// expanded in its edge group. Returns whether anything changed.
    pub(super) fn set_mode(
        &mut self,
        key: &str,
        mode: ToolWindowMode,
        origin: ChangeOrigin,
    ) -> bool {
        let Some(window) = self.window_mut(key) else {
            return false;
        };
        if window.mode == mode {
            return false;
        }
        window.mode = mode;
        window.mode_chosen = true;
        let (edge, group, expanded) = (window.edge, window.group, window.expanded);
        if mode == ToolWindowMode::Docked && expanded {
            let others: Vec<String> = self
                .windows
                .iter()
                .filter(|other| {
                    other.key != key
                        && other.edge == edge
                        && other.group == group
                        && other.expanded
                        && other.mode == ToolWindowMode::Docked
                })
                .map(|other| other.key.clone())
                .collect();
            for other in others {
                self.apply(&other, false, origin);
            }
        }
        self.record(key, ToolWindowChangeKind::Mode, origin);
        self.request();
        true
    }
    /// Sets the extent of every tool window expanded on an edge.
    pub(super) fn set_extent(&mut self, edge: ToolEdge, extent: f32) {
        for window in self.windows.iter_mut().filter(|window| {
            window.edge == edge
                && window.expanded
                && window.mode == ToolWindowMode::Docked
                && !window.overlay
        }) {
            if window.extent != Some(extent) {
                window.extent = Some(extent);
                window.restored = None;
            }
        }
    }
    /// Asks the layout for a delivery, which also re-evaluates which parts
    /// are shown.
    pub(super) fn request(&self) {
        if let Some(notifier) = &self.notifier {
            self.presentation_dirty.set(true);
            self.deliveries.borrow_mut().push_back(None);
            notifier.request();
        }
        // A detached panel decides in its own window whether it must return.
        for key in self.detached.keys() {
            if let Some(notifier) = self.panel_notifiers.get(key)
                && self.panel_pending.borrow_mut().insert(key.clone())
            {
                notifier.request();
            }
        }
    }
    /// Whether a detached panel must return to the layout: its tool window
    /// collapsed or left window mode.
    pub(super) fn must_return(&self, key: &str) -> bool {
        self.detached.contains_key(key)
            && self
                .window(key)
                .is_none_or(|window| !window.expanded || window.mode != ToolWindowMode::Window)
    }
}

/// Marks the model changed for the layout and its parts from an event phase
/// or outside a flush: the layout re-evaluates which parts it shows in the
/// same flush, before a focus request made in the same event is applied.
///
/// # Errors
/// Returns the phase error of writing a binding, such as `WriteDuringUpdate`.
pub(super) fn touch(model: &Rc<RefCell<Model>>) -> Result<(), RuntimeError> {
    let revision = model.borrow().revision.clone();
    if let Some(revision) = revision {
        revision.set(revision.get()?.wrapping_add(1))?;
    }
    Ok(())
}

/// Reads the bound titles of the tool windows `shows` accepts into the model,
/// for a part that presents them, subscribing that part to their bindings.
/// Returns whether a title changed.
///
/// # Errors
/// Returns the runtime error of reading a binding, such as
/// [`RuntimeError::ForeignRuntime`].
pub(super) fn read_titles(
    model: &Rc<RefCell<Model>>,
    cx: &mut UpdateContext,
    shows: impl Fn(&Window) -> bool,
) -> Result<bool, RuntimeError> {
    let sources: Vec<(String, TextSource)> = model
        .borrow()
        .windows
        .iter()
        .filter(|window| window.title_source.is_bound() && shows(window))
        .map(|window| (window.key.clone(), window.title_source.clone()))
        .collect();
    let mut changed = false;
    for (key, source) in sources {
        let title = source.read(cx)?.plain_text();
        let mut model = model.borrow_mut();
        if let Some(window) = model.windows.iter_mut().find(|window| window.key == key)
            && window.title != title
        {
            window.title = title;
            changed = true;
        }
    }
    Ok(changed)
}

/// The revision every part reads in its update to follow the model,
/// created by whichever part updates first.
pub(super) fn revision(model: &Rc<RefCell<Model>>, cx: &UpdateContext) -> Signal<u64> {
    model
        .borrow_mut()
        .revision
        .get_or_insert_with(|| cx.signal(0))
        .clone()
}

/// Checks that the model can be marked changed now, before changing it.
///
/// # Errors
/// As [`touch`].
pub(super) fn check_writable(model: &Rc<RefCell<Model>>) -> Result<(), RuntimeError> {
    let revision = model.borrow().revision.clone();
    if let Some(revision) = revision {
        revision.set(revision.get()?)?;
    }
    Ok(())
}

/// The model a description's parts reach. Re-describing a mounted layout
/// points the new description's link at the mounted model, so parts created
/// for the new description share the state of the parts that stay.
pub(super) struct Link(RefCell<Rc<RefCell<Model>>>);
impl Link {
    pub(super) fn new(model: Model) -> Rc<Self> {
        Rc::new(Self(RefCell::new(Rc::new(RefCell::new(model)))))
    }
    pub(super) fn model(&self) -> Rc<RefCell<Model>> {
        self.0.borrow().clone()
    }
    pub(super) fn redirect(&self, model: Rc<RefCell<Model>>) {
        *self.0.borrow_mut() = model;
    }
}
