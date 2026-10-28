//! Application state: the document, the selection, the tool, the view, the
//! application-owned undo history and the inspector's view model.
//!
//! Every change goes through `Editor`'s methods, which run in event phases
//! (callbacks, commands, mailbox handlers) where signals may be written. That
//! is where the inspector's fields are synchronized from the document: no
//! component observes the document to copy it into other signals.
use crate::model::{Diagram, EdgeId, NodeId, NodeShape};
use pert_ui::prelude::*;
use pert_ui::{StateOwner, ViewScale, ViewTransform};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    path::PathBuf,
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Box,
    Ellipse,
    Connect,
}

/// What the inspector shows. Its structure depends only on this: another
/// node is a new description under another key, so a draft typed for one
/// node can never be saved to another, while a change of values never
/// describes the panel again. Leaving a node discards its drafts, also when
/// it is selected again before the screen updates: each change of what is
/// shown has a new `incarnation`, which the panel's key includes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    Nothing,
    One { node: NodeId, incarnation: u64 },
    Many(usize),
}

impl Shown {
    /// Whether both show the same thing, whatever their incarnations.
    fn same_target(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::One { node: a, .. }, Self::One { node: b, .. }) => a == b,
            _ => self == other,
        }
    }
}

/// The inspector's fields, created once and kept for the editor's lifetime.
/// The fields stay bound to these signals, so a draft being typed survives
/// any document change that leaves its own value alone: `sync` writes only
/// values that differ. An application write to a field replaces its draft,
/// as selecting another node, hiding or removing the panel discards it.
///
/// The fields that can keep a draft report it to `form`, which close
/// admission reads: a draft a field refuses is not saved by admission.
pub struct Inspector {
    pub shown: Signal<Shown>,
    pub label: Signal<String>,
    pub width: Signal<f64>,
    pub height: Signal<f64>,
    pub shape: Signal<NodeShape>,
    pub fill: Signal<Color>,
    pub form: FormFields,
    pub label_field: FieldHandle,
    pub width_field: FieldHandle,
    pub height_field: FieldHandle,
    /// Advances with every change of what is shown; see `Shown`.
    incarnation: Cell<u64>,
}

impl Inspector {
    /// Forgets the kept drafts of a panel that is being replaced. Its
    /// editors are removed without a report, so their marks would stay set.
    fn forget_drafts(&self) -> Result<(), RuntimeError> {
        for field in [&self.label_field, &self.width_field, &self.height_field] {
            field.set_modified(false)?;
        }
        Ok(())
    }
}

const HISTORY: usize = 200;

#[derive(Default)]
struct History {
    undo: Vec<(Rc<Diagram>, String)>,
    redo: Vec<(Rc<Diagram>, String)>,
}

pub struct Editor {
    pub doc: Signal<Rc<Diagram>>,
    /// The selected nodes, by their keys. The canvas and the outline share
    /// this one selection, so neither copies the other's.
    pub nodes: ListSelection,
    pub edges: Signal<BTreeSet<EdgeId>>,
    pub tool: Signal<Tool>,
    pub view: Signal<ViewTransform>,
    pub dirty: Signal<bool>,
    pub status: Signal<String>,
    pub can_undo: Signal<bool>,
    pub can_redo: Signal<bool>,
    pub path: Signal<Option<PathBuf>>,
    pub inspector: Inspector,
    history: RefCell<History>,
    /// The document as last saved or opened, to tell whether it is dirty.
    saved: RefCell<Rc<Diagram>>,
    /// Advances whenever `replace` installs another document, so work that
    /// began on the previous one can tell that it no longer applies. A
    /// signal, because a replacement equal to the old document changes
    /// nothing in `doc`.
    pub incarnation: Signal<u64>,
}

impl Editor {
    pub fn new(ui: &impl StateOwner, diagram: Diagram) -> Result<Rc<Self>, RuntimeError> {
        let doc = Rc::new(diagram);
        let form = FormFields::new(ui);
        let (label_field, width_field, height_field) = (
            form.field("label")?,
            form.field("width")?,
            form.field("height")?,
        );
        let editor = Rc::new(Self {
            doc: ui.signal(doc.clone()),
            nodes: ListSelection::new(ui, SelectionMode::Multiple),
            edges: ui.signal(BTreeSet::new()),
            tool: ui.signal(Tool::Select),
            view: ui.signal(ViewTransform::IDENTITY),
            dirty: ui.signal(false),
            status: ui.signal("Ready".to_owned()),
            can_undo: ui.signal(false),
            can_redo: ui.signal(false),
            path: ui.signal(None),
            inspector: Inspector {
                shown: ui.signal(Shown::Nothing),
                label: ui.signal(String::new()),
                width: ui.signal(0.0),
                height: ui.signal(0.0),
                shape: ui.signal(NodeShape::Box),
                fill: ui.signal(Color::WHITE),
                form,
                label_field,
                width_field,
                height_field,
                incarnation: Cell::new(0),
            },
            history: RefCell::default(),
            saved: RefCell::new(doc),
            incarnation: ui.signal(0),
        });
        Ok(editor)
    }

    pub fn diagram(&self) -> Result<Rc<Diagram>, RuntimeError> {
        self.doc.get()
    }

    pub fn selected_nodes(&self) -> Result<Vec<NodeId>, RuntimeError> {
        Ok(self
            .nodes
            .selected()
            .get()?
            .iter()
            .filter_map(|key| NodeId::from_key(key))
            .collect())
    }

    pub fn has_selection(&self) -> Result<ReadSignal<bool>, RuntimeError> {
        ReadSignal::combine((self.nodes.selected(), &self.edges), |(nodes, edges)| {
            !nodes.is_empty() || !edges.is_empty()
        })
    }

    /// Applies one undoable change. A change that changes nothing records
    /// nothing; returns whether it changed the document.
    pub fn commit(
        &self,
        label: &str,
        change: impl FnOnce(&mut Diagram),
    ) -> Result<bool, RuntimeError> {
        let before = self.doc.get()?;
        let mut after = (*before).clone();
        change(&mut after);
        if after == *before {
            return Ok(false);
        }
        {
            let mut history = self.history.borrow_mut();
            history.undo.push((before, label.to_owned()));
            if history.undo.len() > HISTORY {
                history.undo.remove(0);
            }
            history.redo.clear();
        }
        self.install(Rc::new(after))?;
        self.status.set(label.to_owned())?;
        Ok(true)
    }

    fn install(&self, doc: Rc<Diagram>) -> Result<(), RuntimeError> {
        self.dirty.set(*self.saved.borrow() != doc)?;
        self.doc.set(doc.clone())?;
        // Drop selected ids the document no longer has.
        let kept: Vec<String> = self
            .selected_nodes()?
            .into_iter()
            .filter(|id| doc.node(*id).is_some())
            .map(NodeId::key)
            .collect();
        self.nodes.select(kept)?;
        self.edges
            .update(|edges| edges.retain(|id| doc.edges.iter().any(|edge| edge.id == *id)))?;
        let history = self.history.borrow();
        self.can_undo.set(!history.undo.is_empty())?;
        self.can_redo.set(!history.redo.is_empty())?;
        drop(history);
        self.sync()
    }

    /// Copies the selected node's values into the inspector's fields,
    /// writing only those that differ, and shows another panel when the
    /// selection asks for one.
    pub fn sync(&self) -> Result<(), RuntimeError> {
        let doc = self.doc.get()?;
        let nodes = self.selected_nodes()?;
        let edges = self.edges.get()?.len();
        let inspector = &self.inspector;
        let node = match nodes.as_slice() {
            [id] if edges == 0 => doc.node(*id).cloned(),
            _ => None,
        };
        let incarnation = inspector.incarnation.get().wrapping_add(1);
        let target = match &node {
            Some(node) => Shown::One {
                node: node.id,
                incarnation,
            },
            None if nodes.len() + edges == 0 => Shown::Nothing,
            None => Shown::Many(nodes.len() + edges),
        };
        if let Some(node) = node {
            inspector.label.set(node.label.clone())?;
            inspector.width.set(f64::from(node.width))?;
            inspector.height.set(f64::from(node.height))?;
            inspector.shape.set(node.shape)?;
            inspector.fill.set(node.fill)?;
        }
        if target.same_target(&inspector.shown.get()?) {
            return Ok(());
        }
        // Another panel: its key changes with the incarnation, so the panel
        // shown now is removed with its drafts, which therefore no longer
        // count.
        inspector.incarnation.set(incarnation);
        inspector.shown.set(target)?;
        inspector.forget_drafts()
    }

    /// Selects nodes and edges.
    pub fn select(
        &self,
        nodes: impl IntoIterator<Item = NodeId>,
        edges: impl IntoIterator<Item = EdgeId>,
    ) -> Result<(), RuntimeError> {
        self.nodes.select(nodes.into_iter().map(NodeId::key))?;
        self.edges.set(edges.into_iter().collect())?;
        self.sync()
    }

    /// Applies a save to one node, if it still exists.
    pub fn edit_node(
        &self,
        id: NodeId,
        label: &str,
        change: impl FnOnce(&mut crate::model::Node),
    ) -> Result<(), RuntimeError> {
        self.commit(label, |doc| {
            if let Some(node) = doc.node_mut(id) {
                change(node);
            }
        })?;
        Ok(())
    }

    pub fn undo(&self) -> Result<(), RuntimeError> {
        let current = self.doc.get()?;
        let step = self.history.borrow_mut().undo.pop();
        let Some((previous, label)) = step else {
            return self.status.set("Nothing to undo".to_owned());
        };
        self.history
            .borrow_mut()
            .redo
            .push((current, label.clone()));
        self.install(previous)?;
        self.status.set(format!("Undid {label}"))
    }

    pub fn redo(&self) -> Result<(), RuntimeError> {
        let current = self.doc.get()?;
        let step = self.history.borrow_mut().redo.pop();
        let Some((next, label)) = step else {
            return self.status.set("Nothing to redo".to_owned());
        };
        self.history
            .borrow_mut()
            .undo
            .push((current, label.clone()));
        self.install(next)?;
        self.status.set(format!("Redid {label}"))
    }

    /// Which document is installed; see `replace`.
    pub fn incarnation(&self) -> Result<u64, RuntimeError> {
        self.incarnation.get()
    }

    /// Replaces the document, as opening a file does: the history restarts.
    pub fn replace(&self, diagram: Diagram, path: Option<PathBuf>) -> Result<(), RuntimeError> {
        self.incarnation
            .update(|incarnation| *incarnation = incarnation.wrapping_add(1))?;
        let doc = Rc::new(diagram);
        *self.saved.borrow_mut() = doc.clone();
        *self.history.borrow_mut() = History::default();
        self.nodes.clear()?;
        self.edges.set(BTreeSet::new())?;
        self.path.set(path)?;
        self.install(doc)
    }

    /// Records that `doc` is what the file at `path` now holds.
    pub fn mark_saved(&self, doc: Rc<Diagram>, path: PathBuf) -> Result<(), RuntimeError> {
        let dirty = *self.doc.get()? != *doc;
        *self.saved.borrow_mut() = doc;
        self.dirty.set(dirty)?;
        self.path.set(Some(path))
    }

    pub fn delete_selection(&self) -> Result<(), RuntimeError> {
        let nodes = self.selected_nodes()?;
        let edges: Vec<_> = self.edges.get()?.into_iter().collect();
        if nodes.is_empty() && edges.is_empty() {
            return Ok(());
        }
        self.commit("Delete", |doc| doc.remove(&nodes, &edges))?;
        Ok(())
    }

    pub fn select_all(&self) -> Result<(), RuntimeError> {
        let doc = self.doc.get()?;
        self.select(doc.nodes.iter().map(|node| node.id), [])
    }

    pub fn duplicate(&self) -> Result<(), RuntimeError> {
        let selected = self.selected_nodes()?;
        let mut created = Vec::new();
        self.commit("Duplicate", |doc| {
            for id in &selected {
                if let Some(node) = doc.node(*id).cloned() {
                    let Some(copy) =
                        doc.add_node(node.shape, node.x + 20.0, node.y + 20.0, &node.label)
                    else {
                        break;
                    };
                    if let Some(new) = doc.node_mut(copy) {
                        new.width = node.width;
                        new.height = node.height;
                        new.fill = node.fill;
                    }
                    created.push(copy);
                }
            }
        })?;
        self.select(created, [])
    }

    pub fn bring_to_front(&self) -> Result<(), RuntimeError> {
        let selected = self.selected_nodes()?;
        self.commit("Bring to front", |doc| doc.bring_to_front(&selected))?;
        Ok(())
    }

    /// Zooms by `factor` around a point in the canvas's local pixels.
    pub fn zoom_by(&self, factor: f32, anchor: Point) -> Result<(), RuntimeError> {
        let view = self.view.get()?;
        let scale = (view.scale().get() * factor).clamp(ViewScale::MIN.get(), ViewScale::MAX.get());
        self.view
            .set(view.zoomed_at(ViewScale::new(scale)?, anchor)?)
    }
}
