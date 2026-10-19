//! The application's own document: nodes, edges and a versioned text format.
//! Nothing here depends on pert-ui except its color and geometry value types;
//! the library holds no part of it.
use pert_ui::{Color, Point, Rect};
use std::fmt::Write as _;

/// A node's identity, never reused within a document. It is also the node's
/// key among the canvas's children and in the outline (`n` and the number),
/// so reordering or relabelling a node keeps its components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);
impl NodeId {
    pub fn key(self) -> String {
        format!("n{}", self.0)
    }
    pub fn from_key(key: &str) -> Option<Self> {
        key.strip_prefix('n')?.parse().ok().map(Self)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeShape {
    Box,
    Ellipse,
}
impl NodeShape {
    fn name(self) -> &'static str {
        match self {
            Self::Box => "box",
            Self::Ellipse => "ellipse",
        }
    }
    fn parse(text: &str) -> Option<Self> {
        match text {
            "box" => Some(Self::Box),
            "ellipse" => Some(Self::Ellipse),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub shape: NodeShape,
    /// Diagram coordinates of the top-left corner.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fill: Color,
    pub label: String,
}
impl Node {
    pub fn rect(&self) -> Rect {
        // Decoding and editing keep coordinates finite and sizes positive.
        Rect::new(self.x, self.y, self.width, self.height)
            .unwrap_or(Rect::from_size(pert_ui::Size::ZERO))
    }
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
    pub fn contains(&self, point: Point) -> bool {
        match self.shape {
            NodeShape::Box => self.rect().contains(point),
            NodeShape::Ellipse => {
                let (cx, cy) = self.center();
                let (rx, ry) = (self.width / 2.0, self.height / 2.0);
                let (dx, dy) = ((point.x() - cx) / rx, (point.y() - cy) / ry);
                dx * dx + dy * dy <= 1.0
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
}

/// The document. Node order is z-order, back to front.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Diagram {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub next_id: u64,
}

pub const MIN_SIDE: f32 = 16.0;
pub const MAX_SIDE: f32 = 2000.0;
pub const MAX_COORD: f32 = 100_000.0;
pub const MAX_LABEL: usize = 200;
/// The largest identity a document may use, far below `u64::MAX` so that
/// allocation can never wrap.
pub const MAX_ID: u64 = 1 << 53;

impl Diagram {
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }
    /// The topmost node under a diagram point.
    pub fn node_at(&self, point: Point) -> Option<NodeId> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.contains(point))
            .map(|node| node.id)
    }
    /// A fresh identity, or `None` once the document has used `MAX_ID`.
    fn allocate(&mut self) -> Option<u64> {
        let next = self.next_id.checked_add(1).filter(|next| *next <= MAX_ID)?;
        self.next_id = next;
        Some(next)
    }
    /// Adds a node; `None` when the document has no identity left.
    pub fn add_node(&mut self, shape: NodeShape, x: f32, y: f32, label: &str) -> Option<NodeId> {
        let id = NodeId(self.allocate()?);
        let fill = Color::from_srgb8(0x8e, 0xc5, 0xff, 0xff);
        self.nodes.push(Node {
            id,
            shape,
            x,
            y,
            width: 120.0,
            height: 60.0,
            fill,
            label: label.to_owned(),
        });
        Some(id)
    }
    /// Adds an edge unless it would be a self-loop or duplicate.
    pub fn connect(&mut self, from: NodeId, to: NodeId) -> Option<EdgeId> {
        if from == to
            || self.node(from).is_none()
            || self.node(to).is_none()
            || self.edges.iter().any(|e| e.from == from && e.to == to)
        {
            return None;
        }
        let id = EdgeId(self.allocate()?);
        self.edges.push(Edge { id, from, to });
        Some(id)
    }
    /// Removes nodes and their edges, and the listed edges.
    pub fn remove(&mut self, nodes: &[NodeId], edges: &[EdgeId]) {
        self.nodes.retain(|node| !nodes.contains(&node.id));
        self.edges.retain(|edge| {
            !edges.contains(&edge.id) && !nodes.contains(&edge.from) && !nodes.contains(&edge.to)
        });
    }
    /// Moves nodes, keeping them inside the coordinates the format stores.
    pub fn move_by(&mut self, ids: &[NodeId], dx: f32, dy: f32) {
        for node in self.nodes.iter_mut().filter(|node| ids.contains(&node.id)) {
            node.x = (node.x + dx).clamp(-MAX_COORD, MAX_COORD);
            node.y = (node.y + dy).clamp(-MAX_COORD, MAX_COORD);
        }
    }
    pub fn bring_to_front(&mut self, ids: &[NodeId]) {
        let (front, mut back): (Vec<_>, Vec<_>) = self
            .nodes
            .drain(..)
            .partition(|node| ids.contains(&node.id));
        back.extend(front);
        self.nodes = back;
    }
    /// The bounding box of every node, in diagram coordinates.
    pub fn bounds(&self) -> Option<Rect> {
        let mut nodes = self.nodes.iter();
        let first = nodes.next()?.rect();
        let [mut l, mut t, w, h] = first.values();
        let (mut r, mut b) = (l + w, t + h);
        for node in nodes {
            l = l.min(node.x);
            t = t.min(node.y);
            r = r.max(node.x + node.width);
            b = b.max(node.y + node.height);
        }
        Rect::new(l, t, r - l, b - t).ok()
    }

    pub fn encode(&self) -> String {
        let mut text = String::from("diagram 1\n");
        let _ = writeln!(text, "next {}", self.next_id);
        for node in &self.nodes {
            let _ = writeln!(
                text,
                "node {} {} {} {} {} {} {} {}",
                node.id.0,
                node.shape.name(),
                node.x,
                node.y,
                node.width,
                node.height,
                node.fill.to_hex(),
                escape(&node.label),
            );
        }
        for edge in &self.edges {
            let _ = writeln!(text, "edge {} {} {}", edge.id.0, edge.from.0, edge.to.0);
        }
        text
    }

    pub fn decode(text: &str) -> Result<Self, DecodeError> {
        let mut lines = text.lines().enumerate();
        match lines.next() {
            Some((_, "diagram 1")) => {}
            Some((_, header)) if header.starts_with("diagram ") => {
                return Err(DecodeError::new(1, "unsupported version"));
            }
            _ => return Err(DecodeError::new(1, "not a diagram")),
        }
        let mut diagram = Self::default();
        let mut max_id = 0;
        for (index, line) in lines {
            let number = index + 1;
            let error = |reason: &str| DecodeError::new(number, reason);
            let mut words = line.splitn(9, ' ');
            let id_of = |word: Option<&str>| -> Result<u64, DecodeError> {
                word.and_then(|w| w.parse().ok())
                    .ok_or_else(|| error("bad id"))
            };
            let num = |word: Option<&str>| -> Result<f32, DecodeError> {
                word.and_then(|w| w.parse::<f32>().ok())
                    .filter(|v| v.is_finite() && v.abs() <= MAX_COORD)
                    .ok_or_else(|| error("bad number"))
            };
            match words.next() {
                Some("next") => {
                    diagram.next_id = id_of(words.next())?;
                    if diagram.next_id > MAX_ID {
                        return Err(error("id counter out of range"));
                    }
                }
                Some("node") => {
                    let id = id_of(words.next())?;
                    let shape = words
                        .next()
                        .and_then(NodeShape::parse)
                        .ok_or_else(|| error("bad shape"))?;
                    let (x, y, width, height) = (
                        num(words.next())?,
                        num(words.next())?,
                        num(words.next())?,
                        num(words.next())?,
                    );
                    if !(MIN_SIDE..=MAX_SIDE).contains(&width)
                        || !(MIN_SIDE..=MAX_SIDE).contains(&height)
                    {
                        return Err(error("bad size"));
                    }
                    let fill = words
                        .next()
                        .and_then(|w| Color::from_hex(w).ok())
                        .ok_or_else(|| error("bad color"))?;
                    let label =
                        unescape(words.next().unwrap_or("")).ok_or_else(|| error("bad escape"))?;
                    if label.chars().count() > MAX_LABEL {
                        return Err(error("label too long"));
                    }
                    if diagram.node(NodeId(id)).is_some() {
                        return Err(error("duplicate node"));
                    }
                    max_id = max_id.max(id);
                    diagram.nodes.push(Node {
                        id: NodeId(id),
                        shape,
                        x,
                        y,
                        width,
                        height,
                        fill,
                        label,
                    });
                }
                Some("edge") => {
                    let id = id_of(words.next())?;
                    let from = NodeId(id_of(words.next())?);
                    let to = NodeId(id_of(words.next())?);
                    if diagram.node(from).is_none() || diagram.node(to).is_none() {
                        return Err(error("edge to unknown node"));
                    }
                    if diagram.edges.iter().any(|e| e.id.0 == id) {
                        return Err(error("duplicate edge"));
                    }
                    max_id = max_id.max(id);
                    diagram.edges.push(Edge {
                        id: EdgeId(id),
                        from,
                        to,
                    });
                }
                Some("") | None => {}
                Some(_) => return Err(error("unknown record")),
            }
        }
        if diagram.next_id < max_id {
            return Err(DecodeError::new(2, "next id below a used id"));
        }
        Ok(diagram)
    }
}

/// A label on one line: a backslash as `\\`, a line break as `\n`.
fn escape(label: &str) -> String {
    let mut text = String::with_capacity(label.len());
    for character in label.chars() {
        match character {
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            other => text.push(other),
        }
    }
    text
}

/// Reverses `escape` in one pass; `None` for an escape it never writes.
fn unescape(text: &str) -> Option<String> {
    let mut label = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            label.push(character);
            continue;
        }
        match characters.next()? {
            '\\' => label.push('\\'),
            'n' => label.push('\n'),
            _ => return None,
        }
    }
    Some(label)
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodeError {
    pub line: usize,
    pub reason: String,
}
impl DecodeError {
    fn new(line: usize, reason: &str) -> Self {
        Self {
            line,
            reason: reason.to_owned(),
        }
    }
}
impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}
impl std::error::Error for DecodeError {}
