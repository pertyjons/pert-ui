//! The diagram canvas: a custom container built only from public extension
//! points. Each node is a child component positioned by `arrange` and
//! presented through `child_transform` for pan and zoom; the canvas draws the
//! edges below its children and the selection and gesture feedback above
//! them, and owns a retained `DraftEditor` for renaming a node in place.
use crate::editor::{Editor, Tool};
use crate::model::{Diagram, EdgeId, MAX_LABEL, Node, NodeId, NodeShape};
use pert_ui::component::*;
use std::{cell::Cell, collections::BTreeSet, rc::Rc};

/// A press must move this far, in window pixels, before it drags.
const SLOP: f32 = 4.0;
/// The key of the rename editor among the canvas's children.
const RENAME: &str = "rename";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Gesture {
    Idle,
    /// A primary press, not yet a drag. `local` is canvas-local, `at` in
    /// diagram coordinates.
    Pressed {
        local: Point,
        at: Point,
        node: Option<NodeId>,
        additive: bool,
    },
    /// Moving the selected nodes by a diagram offset.
    Moving {
        from: Point,
        delta: (f32, f32),
    },
    /// A rubber band between two diagram points.
    Band {
        from: Point,
        to: Point,
        additive: bool,
    },
    /// Dragging a new edge from a node.
    Connecting {
        from: NodeId,
        to: Point,
    },
    /// Panning, owned by the button that started it.
    Panning {
        last: Point,
        button: PointerButton,
    },
}

/// State the application shares with the canvas, outside any signal because
/// nothing renders from it: whether Space is held for panning (kept by a
/// held-input registration), whether the canvas has focus (which decides
/// whether that registration takes Space at all), and the canvas's size from
/// its last arrangement, for commands that zoom around its center or fit the
/// document into it.
#[derive(Default)]
pub struct Shared {
    pub space: Cell<bool>,
    pub focused: Cell<bool>,
    pub size: Cell<Option<Size>>,
}

pub struct Canvas {
    editor: Rc<Editor>,
    keys: Rc<Shared>,
    doc: Rc<Diagram>,
    selected: BTreeSet<NodeId>,
    edges: BTreeSet<EdgeId>,
    tool: Tool,
    view: ViewTransform,
    gesture: Gesture,
    clicks: ClickCounter,
    rename: DraftEditorController,
    /// The node being renamed, in the document incarnation it belongs to.
    renaming: Option<(NodeId, u64)>,
    rename_editor: Option<WidgetId>,
    /// The document last described as children.
    described: Option<Rc<Diagram>>,
    initial: Vec<Element>,
}

impl Canvas {
    pub fn new(editor: Rc<Editor>, keys: Rc<Shared>) -> Self {
        let rename = DraftEditorController::new();
        Self {
            initial: vec![rename_element(&rename)],
            editor,
            keys,
            doc: Rc::default(),
            selected: BTreeSet::new(),
            edges: BTreeSet::new(),
            tool: Tool::Select,
            view: ViewTransform::IDENTITY,
            gesture: Gesture::Idle,
            clicks: ClickCounter::new(),
            rename,
            renaming: None,
            rename_editor: None,
            described: None,
        }
    }

    fn to_diagram(&self, local: Point) -> Result<Point, RuntimeError> {
        Ok(self.view.invert(local)?)
    }

    /// The node rectangle as presented, with a drag in progress.
    fn shown_rect(&self, node: &Node) -> Rect {
        let (dx, dy) = match self.gesture {
            Gesture::Moving { delta, .. } if self.selected.contains(&node.id) => delta,
            _ => (0.0, 0.0),
        };
        Rect::new(node.x + dx, node.y + dy, node.width, node.height).unwrap_or(node.rect())
    }

    /// A diagram rectangle in the canvas's local pixels.
    fn view_rect(&self, rect: Rect) -> Result<Rect, RuntimeError> {
        let [x, y, w, h] = rect.values();
        let origin = self.view.apply(Point::new(x, y)?)?;
        let scale = self.view.scale().get();
        Ok(Rect::new(origin.x(), origin.y(), w * scale, h * scale)?)
    }

    fn select(&self, nodes: BTreeSet<NodeId>, edges: BTreeSet<EdgeId>) -> Result<(), RuntimeError> {
        self.editor.select(nodes, edges)
    }

    /// Opens the rename editor over a node with its label selected. The
    /// canvas reports itself unfocusable meanwhile, as the `DraftEditor`
    /// contract asks, and moves focus into the editor.
    fn begin_rename(&mut self, cx: &mut EventContext, id: NodeId) -> Result<(), RuntimeError> {
        let Some(node) = self.doc.node(id) else {
            return Ok(());
        };
        self.rename
            .begin(cx, &node.label, None, DraftSelection::All)?;
        self.renaming = Some((id, self.editor.incarnation()?));
        if let Some(editor) = &self.rename_editor {
            cx.request_focus_for(editor)?;
        }
        cx.invalidate(Invalidation::Layout);
        Ok(())
    }

    /// Ends renaming, committing the draft as one undoable step or not. A
    /// label longer than the document format stores is refused: on Enter
    /// the session stays open for correction (returns `false`), and when
    /// focus leaves it is discarded, with the reason in the status line.
    fn end_rename(
        &mut self,
        cx: &mut EventContext,
        commit: bool,
        keep_invalid: bool,
    ) -> Result<bool, RuntimeError> {
        let Some((id, _)) = self.renaming else {
            return Ok(true);
        };
        let text = self.rename.text();
        let fits = text.chars().count() <= MAX_LABEL;
        if commit && !fits {
            self.editor
                .status
                .set(format!("A label has at most {MAX_LABEL} characters"))?;
            if keep_invalid {
                return Ok(false);
            }
        }
        self.renaming = None;
        self.rename.end();
        cx.invalidate(Invalidation::Layout);
        if commit && fits {
            self.editor.commit("Rename", |doc| {
                if let Some(node) = doc.node_mut(id) {
                    node.label = text;
                }
            })?;
        }
        Ok(true)
    }

    fn finish(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let gesture = std::mem::replace(&mut self.gesture, Gesture::Idle);
        cx.release_pointer();
        match gesture {
            Gesture::Moving {
                delta: (dx, dy), ..
            } if dx != 0.0 || dy != 0.0 => {
                let ids: Vec<_> = self.selected.iter().copied().collect();
                self.editor
                    .commit("Move", |doc| doc.move_by(&ids, dx, dy))?;
                cx.invalidate(Invalidation::Layout);
            }
            Gesture::Band { from, to, additive } => {
                let band = band_rect(from, to)?;
                let mut nodes = if additive {
                    self.selected.clone()
                } else {
                    BTreeSet::new()
                };
                for node in &self.doc.nodes {
                    if band.intersection(node.rect()) == node.rect() {
                        nodes.insert(node.id);
                    }
                }
                let edges = if additive {
                    self.edges.clone()
                } else {
                    BTreeSet::new()
                };
                self.select(nodes, edges)?;
                cx.invalidate(Invalidation::Paint);
            }
            Gesture::Connecting { from, to } => {
                if let Some(target) = self.doc.node_at(to) {
                    self.editor.commit("Connect", |doc| {
                        doc.connect(from, target);
                    })?;
                }
                cx.invalidate(Invalidation::Paint);
            }
            Gesture::Pressed { at, node: None, .. }
                if matches!(self.tool, Tool::Box | Tool::Ellipse) =>
            {
                let shape = if self.tool == Tool::Box {
                    NodeShape::Box
                } else {
                    NodeShape::Ellipse
                };
                let mut created = None;
                self.editor.commit("Add node", |doc| {
                    let label = format!("Node {}", doc.nodes.len() + 1);
                    created = doc.add_node(shape, at.x() - 60.0, at.y() - 30.0, &label);
                })?;
                self.select(created.into_iter().collect(), BTreeSet::new())?;
            }
            Gesture::Pressed {
                node: None,
                additive: false,
                ..
            } => {
                self.select(BTreeSet::new(), BTreeSet::new())?;
            }
            _ => cx.invalidate(Invalidation::Paint),
        }
        Ok(())
    }

    fn press(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
        local: Point,
    ) -> Result<InputOutcome, RuntimeError> {
        // A press outside the rename editor commits it; presses on the
        // editor reach the editor first and do not bubble here.
        self.end_rename(cx, true, false)?;
        cx.request_focus();
        let at = self.to_diagram(local)?;
        let additive = cx.modifiers().shift;
        let node = self.doc.node_at(at);
        if self.keys.space.get() {
            self.clicks.reset();
            self.gesture = Gesture::Panning {
                last: local,
                button: PointerButton::Primary,
            };
            cx.capture_pointer()?;
            return Ok(InputOutcome::Handled);
        }
        if let (Tool::Connect, Some(from)) = (self.tool, node) {
            self.gesture = Gesture::Connecting { from, to: at };
            cx.capture_pointer()?;
            cx.invalidate(Invalidation::Paint);
            return Ok(InputOutcome::Handled);
        }
        let count = self.clicks.press_event(cx, event).unwrap_or(1);
        if let Some(id) = node {
            if count > 1 {
                // Acted on: the next press starts a new series.
                self.clicks.reset();
                self.begin_rename(cx, id)?;
                return Ok(InputOutcome::Handled);
            }
            if !self.selected.contains(&id) {
                let mut nodes = if additive {
                    self.selected.clone()
                } else {
                    BTreeSet::new()
                };
                nodes.insert(id);
                let edges = if additive {
                    self.edges.clone()
                } else {
                    BTreeSet::new()
                };
                self.select(nodes, edges)?;
            } else if additive {
                let mut nodes = self.selected.clone();
                nodes.remove(&id);
                self.select(nodes, self.edges.clone())?;
            }
        } else if let Some(edge) = self.edge_at(at) {
            let mut edges = if additive {
                self.edges.clone()
            } else {
                BTreeSet::new()
            };
            edges.insert(edge);
            let nodes = if additive {
                self.selected.clone()
            } else {
                BTreeSet::new()
            };
            self.select(nodes, edges)?;
            return Ok(InputOutcome::Handled);
        }
        self.gesture = Gesture::Pressed {
            local,
            at,
            node,
            additive,
        };
        cx.capture_pointer()?;
        Ok(InputOutcome::Handled)
    }

    fn edge_at(&self, at: Point) -> Option<EdgeId> {
        let tolerance = 5.0 / self.view.scale().get();
        self.doc.edges.iter().rev().find_map(|edge| {
            let (a, b) = (self.doc.node(edge.from)?, self.doc.node(edge.to)?);
            let ((ax, ay), (bx, by)) = (a.center(), b.center());
            let (dx, dy) = (bx - ax, by - ay);
            let length = dx * dx + dy * dy;
            if length == 0.0 {
                return None;
            }
            let t = (((at.x() - ax) * dx + (at.y() - ay) * dy) / length).clamp(0.0, 1.0);
            let (px, py) = (ax + t * dx - at.x(), ay + t * dy - at.y());
            (px * px + py * py <= tolerance * tolerance).then_some(edge.id)
        })
    }

    /// Follows a move. The runtime repeats the last position after each
    /// layout (`PointerEvent::is_refresh`), so this invalidates only when
    /// the state the position produces changed; otherwise every layout would
    /// bring the next move and the flush would never settle.
    fn drag(&mut self, cx: &mut EventContext, local: Point) -> Result<InputOutcome, RuntimeError> {
        let at = self.to_diagram(local)?;
        match self.gesture {
            Gesture::Pressed {
                local: start,
                at: from,
                node,
                additive,
            } => {
                if (local.x() - start.x()).hypot(local.y() - start.y()) < SLOP {
                    return Ok(InputOutcome::Handled);
                }
                // A press that became a drag is not half a double click.
                self.clicks.reset();
                self.gesture = match node {
                    Some(_) => Gesture::Moving {
                        from,
                        delta: (0.0, 0.0),
                    },
                    None => Gesture::Band {
                        from,
                        to: from,
                        additive,
                    },
                };
                self.drag(cx, local)
            }
            Gesture::Moving { from, delta } => {
                let next = (at.x() - from.x(), at.y() - from.y());
                if next != delta {
                    self.gesture = Gesture::Moving { from, delta: next };
                    // The children move: arrangement only.
                    cx.invalidate(Invalidation::Layout);
                }
                Ok(InputOutcome::Handled)
            }
            Gesture::Band { from, to, additive } => {
                if to != at {
                    self.gesture = Gesture::Band {
                        from,
                        to: at,
                        additive,
                    };
                    cx.invalidate(Invalidation::Paint);
                }
                Ok(InputOutcome::Handled)
            }
            Gesture::Connecting { from, to } => {
                if to != at {
                    self.gesture = Gesture::Connecting { from, to: at };
                    cx.invalidate(Invalidation::Paint);
                }
                Ok(InputOutcome::Handled)
            }
            Gesture::Panning { last, button } => {
                if local != last {
                    let offset = self.view.offset();
                    let moved = Point::new(
                        offset.x() + local.x() - last.x(),
                        offset.y() + local.y() - last.y(),
                    )?;
                    self.gesture = Gesture::Panning {
                        last: local,
                        button,
                    };
                    self.editor
                        .view
                        .set(ViewTransform::new(self.view.scale(), moved))?;
                }
                Ok(InputOutcome::Handled)
            }
            Gesture::Idle => Ok(InputOutcome::Ignored),
        }
    }

    fn cancel(&mut self, cx: &mut EventContext) -> bool {
        if self.gesture == Gesture::Idle {
            return false;
        }
        self.gesture = Gesture::Idle;
        cx.release_pointer();
        cx.invalidate(Invalidation::Layout);
        true
    }

    fn nudge(&self, dx: f32, dy: f32) -> Result<InputOutcome, RuntimeError> {
        let ids: Vec<_> = self.selected.iter().copied().collect();
        self.editor
            .commit("Nudge", |doc| doc.move_by(&ids, dx, dy))?;
        Ok(InputOutcome::Handled)
    }

    fn describe(&self) -> Vec<Element> {
        self.doc
            .nodes
            .iter()
            .map(|node| Element::new(node.id.key(), NodeView { node: node.clone() }))
            .chain([rename_element(&self.rename)])
            .collect()
    }
}

/// The rename editor, named for inspection since it has no visible label.
fn rename_element(rename: &DraftEditorController) -> Element {
    Element::new(RENAME, DraftEditor::new(rename.clone())).with_label("Node name")
}

fn band_rect(a: Point, b: Point) -> Result<Rect, RuntimeError> {
    Ok(Rect::new(
        a.x().min(b.x()),
        a.y().min(b.y()),
        (a.x() - b.x()).abs(),
        (a.y() - b.y()).abs(),
    )?)
}

/// Where the segment from a node's center towards `toward` leaves it.
fn boundary(node: &Node, rect: Rect, toward: (f32, f32)) -> (f32, f32) {
    let [x, y, w, h] = rect.values();
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let (dx, dy) = (toward.0 - cx, toward.1 - cy);
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let t = match node.shape {
        NodeShape::Box => {
            let tx = if dx == 0.0 {
                f32::INFINITY
            } else {
                (w / 2.0) / dx.abs()
            };
            let ty = if dy == 0.0 {
                f32::INFINITY
            } else {
                (h / 2.0) / dy.abs()
            };
            tx.min(ty)
        }
        NodeShape::Ellipse => 1.0 / ((dx / (w / 2.0)).powi(2) + (dy / (h / 2.0)).powi(2)).sqrt(),
    };
    (cx + dx * t, cy + dy * t)
}

impl Component for Canvas {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.initial)
    }

    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let doc = cx.read(&self.editor.doc.read_only())?;
        let incarnation = cx.read(&self.editor.incarnation.read_only())?;
        self.selected = cx
            .read_paint(&self.editor.nodes.selected())?
            .iter()
            .filter_map(|key| NodeId::from_key(key))
            .collect();
        self.edges = cx.read_paint(&self.editor.edges.read_only())?;
        self.tool = cx.read_paint(&self.editor.tool.read_only())?;
        let view = cx.read(&self.editor.view.read_only())?;
        if view != self.view {
            self.view = view;
            cx.request_layout();
        }
        self.rename_editor = cx
            .children()
            .iter()
            .find(|child| child.key() == RENAME)
            .map(|child| child.id().clone());
        if self
            .described
            .as_ref()
            .is_none_or(|old| !Rc::ptr_eq(old, &doc))
        {
            self.doc = doc.clone();
            self.described = Some(doc);
            cx.request_children(self.describe());
            cx.request_layout();
        }
        // A rename belongs to one node of one document: it ends when the
        // node goes away (an undo) or another document replaces this one,
        // even one equal to it.
        if self
            .renaming
            .is_some_and(|(id, of)| of != incarnation || self.doc.node(id).is_none())
        {
            self.rename.end();
            self.renaming = None;
            cx.request_layout();
        }
        Ok(())
    }

    fn focusable(&self) -> bool {
        !self.rename.is_active()
    }

    fn command_target(&self) -> bool {
        true
    }

    fn child_visible(&self, key: &str) -> bool {
        key != RENAME || self.rename.is_active()
    }

    fn stacks_children(&self) -> bool {
        true
    }

    /// Document order is z-order, so a press does not raise a node on its
    /// own; the rename editor stays on top.
    fn child_stacking_priority(&self, key: &str) -> i32 {
        NodeId::from_key(key)
            .and_then(|id| self.doc.nodes.iter().position(|node| node.id == id))
            .map_or(i32::MAX, |index| {
                i32::try_from(index).unwrap_or(i32::MAX - 1)
            })
    }

    fn child_transform(&self) -> ViewTransform {
        self.view
    }

    fn measure(&self, _: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            space.width().unwrap_or(600.0),
            space.height().unwrap_or(400.0),
        )?)
    }

    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.keys.size.set(Some(size));
        for index in 0..cx.len() {
            let key = cx.measurement().key(index)?.to_owned();
            if key == RENAME {
                let Some(node) = self.renaming.and_then(|(id, _)| self.doc.node(id)) else {
                    cx.place(index, Rect::from_size(Size::ZERO))?;
                    continue;
                };
                let height = cx.measure(index, Constraints::default())?.height();
                let [x, y, w, h] = self.shown_rect(node).values();
                cx.place(
                    index,
                    Rect::new(x + 6.0, y + (h - height) / 2.0, (w - 12.0).max(1.0), height)?,
                )?;
            } else if let Some(node) = NodeId::from_key(&key).and_then(|id| self.doc.node(id)) {
                cx.place(index, self.shown_rect(node))?;
            } else {
                cx.place(index, Rect::from_size(Size::ZERO))?;
            }
        }
        Ok(())
    }

    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        match self.gesture {
            Gesture::Panning { .. } => Some(CursorIcon::Grabbing),
            Gesture::Moving { .. } => Some(CursorIcon::Move),
            _ if self.keys.space.get() => Some(CursorIcon::Grabbing),
            _ => None,
        }
    }

    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(local) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) if self.gesture == Gesture::Idle => {
                self.press(cx, event, local)
            }
            PointerKind::Press(PointerButton::Middle) if self.gesture == Gesture::Idle => {
                self.gesture = Gesture::Panning {
                    last: local,
                    button: PointerButton::Middle,
                };
                cx.capture_pointer()?;
                Ok(InputOutcome::Handled)
            }
            PointerKind::Move => self.drag(cx, local),
            PointerKind::Release(button) => {
                let owner = match self.gesture {
                    Gesture::Idle => return Ok(InputOutcome::Ignored),
                    Gesture::Panning { button, .. } => button,
                    _ => PointerButton::Primary,
                };
                // Only the button that started a gesture ends it.
                if button == owner {
                    self.finish(cx)?;
                }
                Ok(InputOutcome::Handled)
            }
            PointerKind::Scroll(delta) => {
                if cx.modifiers().primary() {
                    let factor = if delta.y() > 0.0 { 1.0 / 1.1 } else { 1.1 };
                    self.editor.zoom_by(factor, local)?;
                } else {
                    let offset = self.view.offset();
                    let moved = Point::new(offset.x() - delta.x(), offset.y() - delta.y())?;
                    self.editor
                        .view
                        .set(ViewTransform::new(self.view.scale(), moved))?;
                }
                Ok(InputOutcome::Handled)
            }
            PointerKind::Pinch(factor) => {
                self.editor.zoom_by(factor.get(), local)?;
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }

    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition != KeyTransition::Pressed && event.transition != KeyTransition::Repeated
        {
            return Ok(InputOutcome::Ignored);
        }
        let key = keyboard.control_key(event);
        // The rename editor leaves Enter and Escape to its owner.
        if self.renaming.is_some() {
            return match key {
                KeyCode::Enter | KeyCode::Escape if event.transition == KeyTransition::Pressed => {
                    if self.end_rename(cx, key == KeyCode::Enter, true)? {
                        cx.request_focus();
                    }
                    Ok(InputOutcome::Handled)
                }
                _ => Ok(InputOutcome::Ignored),
            };
        }
        let step = if keyboard.modifiers().shift {
            10.0
        } else {
            1.0
        };
        let moving = !self.selected.is_empty() && self.gesture == Gesture::Idle;
        match key {
            KeyCode::Escape => {
                if self.cancel(cx) {
                    return Ok(InputOutcome::Handled);
                }
                if self.selected.is_empty() && self.edges.is_empty() {
                    return Ok(InputOutcome::Ignored);
                }
                self.select(BTreeSet::new(), BTreeSet::new())?;
                Ok(InputOutcome::Handled)
            }
            KeyCode::Delete | KeyCode::Backspace if self.gesture == Gesture::Idle => {
                self.editor.delete_selection()?;
                Ok(InputOutcome::Handled)
            }
            KeyCode::ArrowLeft if moving => self.nudge(-step, 0.0),
            KeyCode::ArrowRight if moving => self.nudge(step, 0.0),
            KeyCode::ArrowUp if moving => self.nudge(0.0, -step),
            KeyCode::ArrowDown if moving => self.nudge(0.0, step),
            KeyCode::Enter | KeyCode::F2 if event.transition == KeyTransition::Pressed => {
                match self.selected.iter().next() {
                    Some(id) if self.selected.len() == 1 => {
                        let id = *id;
                        self.begin_rename(cx, id)?;
                        Ok(InputOutcome::Handled)
                    }
                    _ => Ok(InputOutcome::Ignored),
                }
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }

    fn focus_gained(&mut self, _: &mut EventContext) {
        self.keys.focused.set(true);
    }

    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.keys.focused.set(false);
        self.cancel(cx);
        Ok(())
    }

    /// Focus leaving the canvas and its rename editor saves the rename, the
    /// policy this application chose; hiding and removal end it unsaved.
    fn focus_within_changed(
        &mut self,
        cx: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        if !within {
            self.end_rename(cx, true, false)?;
        }
        Ok(())
    }

    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.clicks.reset();
        self.cancel(cx);
    }

    fn input_reset(&mut self) {
        self.clicks.reset();
        self.gesture = Gesture::Idle;
    }

    /// Hiding the canvas ends a rename unsaved, as removing it does; the
    /// embedded editor would otherwise keep its session while hidden.
    fn visibility_changed(&mut self, visible: bool) {
        if !visible && self.renaming.take().is_some() {
            self.rename.end();
        }
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let background = painter.color(ColorRole::Background);
        painter.fill_rect(Rect::from_size(size), background);
        // A dot grid every 20 diagram units, at least 8 pixels apart.
        let mut spacing = 20.0 * self.view.scale().get();
        while spacing < 8.0 {
            spacing *= 2.0;
        }
        let dot = painter.color(ColorRole::Outline);
        let offset = self.view.offset();
        let mut x = offset.x().rem_euclid(spacing);
        while x < size.width() {
            let mut y = offset.y().rem_euclid(spacing);
            while y < size.height() {
                painter.fill_rect(Rect::new(x, y, 1.0, 1.0)?, dot);
                y += spacing;
            }
            x += spacing;
        }
        // Edges under the nodes, with an arrowhead at the target.
        let line = painter.color(ColorRole::OnSurface);
        let selected = painter.color(ColorRole::Primary);
        for edge in &self.doc.edges {
            let (Some(a), Some(b)) = (self.doc.node(edge.from), self.doc.node(edge.to)) else {
                continue;
            };
            let (ra, rb) = (
                self.view_rect(self.shown_rect(a))?,
                self.view_rect(self.shown_rect(b))?,
            );
            let center = |r: Rect| (r.x() + r.width() / 2.0, r.y() + r.height() / 2.0);
            let start = boundary(a, ra, center(rb));
            let end = boundary(b, rb, center(ra));
            let chosen = self.edges.contains(&edge.id);
            let color = if chosen { selected } else { line };
            painter.stroke_polyline(
                &[Point::new(start.0, start.1)?, Point::new(end.0, end.1)?],
                StrokeStyle::new(Length::new(if chosen { 3.0 } else { 1.5 })?),
                color,
            )?;
            let (dx, dy) = (end.0 - start.0, end.1 - start.1);
            let length = dx.hypot(dy);
            if length > 1.0 {
                let (ux, uy) = (dx / length, dy / length);
                let (bx, by) = (end.0 - ux * 10.0, end.1 - uy * 10.0);
                painter.fill_triangle(
                    [
                        Point::new(end.0, end.1)?,
                        Point::new(bx - uy * 5.0, by + ux * 5.0)?,
                        Point::new(bx + uy * 5.0, by - ux * 5.0)?,
                    ],
                    color,
                )?;
            }
        }
        Ok(())
    }

    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let primary = painter.color(ColorRole::Primary);
        for node in &self.doc.nodes {
            if self.selected.contains(&node.id) {
                let [x, y, w, h] = self.view_rect(self.shown_rect(node))?.values();
                painter.stroke_shape(
                    Rect::new(x - 3.0, y - 3.0, w + 6.0, h + 6.0)?,
                    Shape::Rounded(Length::new(4.0)?),
                    Length::new(2.0)?,
                    primary,
                )?;
            }
        }
        match self.gesture {
            Gesture::Band { from, to, .. } => {
                let band = self.view_rect(band_rect(from, to)?)?;
                painter.fill_rect(band, primary.scale_alpha(0.15)?);
                painter.stroke_shape(band, Shape::Rectangle, Length::new(1.0)?, primary)?;
            }
            Gesture::Connecting { from, to } => {
                if let Some(node) = self.doc.node(from) {
                    let (x, y) = node.center();
                    let path = PathBuilder::new()
                        .move_to(self.view.apply(Point::new(x, y)?)?)
                        .line_to(self.view.apply(to)?)
                        .build()?;
                    painter.stroke_path(&path, StrokeStyle::new(Length::new(2.0)?), primary)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Group)
                    .name("Diagram")
                    .description(format!(
                        "{} nodes, {} edges, {} selected",
                        self.doc.nodes.len(),
                        self.doc.edges.len(),
                        self.selected.len() + self.edges.len()
                    )),
            ),
            ..ComponentDetails::default()
        }
    }
}

/// One node, drawn as its shape with a centered label.
pub struct NodeView {
    node: Node,
}

impl Component for NodeView {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        if next.node == self.node {
            return None;
        }
        let resized = (next.node.width, next.node.height) != (self.node.width, self.node.height);
        self.node = next.node;
        Some(if resized {
            Invalidation::Layout
        } else {
            Invalidation::Paint
        })
    }

    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(self.node.width, self.node.height)?)
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let rect = Rect::from_size(size);
        let border = painter.color(ColorRole::OnSurface);
        let width = Length::new(1.5)?;
        match self.node.shape {
            NodeShape::Box => {
                let shape = Shape::Rounded(Length::new(6.0)?);
                painter.fill_shape(rect, shape, self.node.fill)?;
                painter.stroke_shape(rect, shape, width, border)?;
            }
            NodeShape::Ellipse => {
                painter.fill_ellipse(rect, self.node.fill)?;
                painter.stroke_ellipse(rect, StrokeStyle::new(width), border)?;
            }
        }
        // Dark text on the node's own light fill, whatever the theme.
        let ink = Color::from_srgb8(0x10, 0x18, 0x20, 0xff);
        let text = painter.text(
            &self.node.label,
            Some((size.width() - 12.0).max(1.0)),
            TextStyle::Body,
        )?;
        let shown = text.size();
        painter.with_clip(rect, |painter| {
            text.paint(
                painter,
                Point::new(
                    (size.width() - shown.width()) / 2.0,
                    (size.height() - shown.height()) / 2.0,
                )?,
                ink,
            )
        })
    }

    fn inspect(&self) -> ComponentDetails {
        let kind = match self.node.shape {
            NodeShape::Box => "Box",
            NodeShape::Ellipse => "Ellipse",
        };
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Group)
                    .name(self.node.label.clone())
                    .description(kind),
            ),
            ..ComponentDetails::default()
        }
    }
}
