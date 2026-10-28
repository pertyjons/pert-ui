use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

use super::*;
use crate::{BoxModelInspection, KeyCode, Modifiers};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NodeData {
    /// A virtualized collection's extent, for `UnboundedCollection`.
    pub(crate) collection: Option<CollectionExtent>,
    pub(crate) window: u64,
    pub(crate) parent: Option<u64>,
    pub(crate) key: String,
    pub(crate) visible: bool,
    pub(crate) focused: bool,
    pub(crate) box_model: BoxModelInspection,
    pub(crate) hints: ValidationHints,
}

/// A virtualized collection's rows and extents along its scrolling axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CollectionExtent {
    pub(crate) rows: usize,
    /// The whole content's extent.
    pub(crate) content: f64,
    /// The extent the collection was allocated as its viewport.
    pub(crate) allocated: f64,
    pub(crate) horizontal: bool,
}

/// What a validation refresh changed, so the runtime can request exactly one
/// repaint for an overlay that shows it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Refreshed {
    /// A validation pass ran, so observed geometry may have changed.
    pub(crate) ran: bool,
    /// The pass produced different markers than the previous one.
    pub(crate) markers_changed: bool,
}

/// One component's visible boxes for the box-model overlay.
pub(crate) struct OverlayBox {
    /// Border box intersected with the effective clip.
    pub(crate) border: [f32; 4],
    /// Content box intersected with the effective clip, where valid padding is
    /// reported and leaves a nonempty area.
    pub(crate) content: Option<[f32; 4]>,
}

type IssueKey = (ValidationCode, Vec<u64>, String);

#[derive(Default)]
pub(crate) struct Validator {
    nodes: BTreeMap<u64, NodeData>,
    attempts: BTreeMap<u64, ValidationIssue>,
    update_limit: Option<ValidationIssue>,
    report: ValidationReport,
    logged: BTreeSet<IssueKey>,
    /// Native windows whose viewport has area. Only their components' geometry
    /// is validated: a window without area presents nothing, and layout gives
    /// every box in it an empty extent.
    measured: BTreeSet<u64>,
    dirty: bool,
    /// Validation passes run, so tests can tell when a pass happened.
    #[cfg(test)]
    pub(crate) passes: usize,
    /// Every issue line written, including ones a later pass cleared.
    #[cfg(test)]
    pub(crate) logged_lines: usize,
}

impl Validator {
    pub(crate) fn relocate(&mut self, id: u64, window: u64, parent: Option<u64>) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.window = window;
            node.parent = parent;
            node.box_model.border_box = None;
            node.hints.clip = None;
            self.dirty = true;
        }
    }

    pub(crate) fn update(&mut self, id: u64, data: NodeData) {
        // Compare bitwise so a persistent NaN observation does not force a global
        // validation pass on every live tick. The numeric rule still rejects it.
        let unchanged = self.nodes.get(&id).is_some_and(|old| same_node(old, &data));
        if !unchanged {
            self.nodes.insert(id, data);
            self.dirty = true;
        }
    }

    pub(crate) fn unmeasured(&mut self, id: u64) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.box_model.border_box = None;
            node.hints.clip = None;
            self.dirty = true;
        }
    }
    /// Commits a node's arranged geometry, with a collection's extents as
    /// that arrangement left them: an arrangement can change its viewport or
    /// estimated content without another update.
    pub(crate) fn geometry(
        &mut self,
        id: u64,
        bounds: [f32; 4],
        clip: [f32; 4],
        collection: Option<CollectionExtent>,
    ) {
        if let Some(node) = self.nodes.get_mut(&id) {
            if node.box_model.border_box == Some(bounds)
                && node.hints.clip == Some(clip)
                && same_collection(node.collection, collection)
            {
                return;
            }
            node.box_model.border_box = Some(bounds);
            node.hints.clip = Some(clip);
            node.collection = collection;
            self.dirty = true;
        }
    }
    /// Replaces the windows whose geometry is validated, before a refresh.
    pub(crate) fn set_measured(&mut self, measured: BTreeSet<u64>) {
        if self.measured != measured {
            self.measured = measured;
            self.dirty = true;
        }
    }
    /// The nodes as validation sees them: without geometry where their window
    /// is not measured, so no geometry rule applies there.
    fn validated_nodes(&self) -> std::borrow::Cow<'_, BTreeMap<u64, NodeData>> {
        if self
            .nodes
            .values()
            .all(|node| self.measured.contains(&node.window))
        {
            return std::borrow::Cow::Borrowed(&self.nodes);
        }
        let mut nodes = self.nodes.clone();
        for node in nodes.values_mut() {
            if !self.measured.contains(&node.window) {
                node.box_model.border_box = None;
                node.hints.clip = None;
            }
        }
        std::borrow::Cow::Owned(nodes)
    }
    pub(crate) fn flags(&mut self, id: u64, visible: bool, focused: bool) {
        if let Some(node) = self.nodes.get_mut(&id)
            && (node.visible != visible || node.focused != focused)
        {
            node.visible = visible;
            node.focused = focused;
            self.dirty = true;
        }
    }

    pub(crate) fn remove(&mut self, id: u64) {
        self.nodes.remove(&id);
        self.attempts.remove(&id);
        self.dirty = true;
    }

    pub(crate) fn duplicate(&mut self, id: u64, key: &str) {
        self.attempts.insert(id, ValidationIssue { code: ValidationCode::DuplicateKey, components: vec![id],
            message: format!("Component key {key:?} is already mounted as #{id}; give the new component a distinct stable key.") });
        self.dirty = true;
    }

    /// `refreshed` lists the targets whose refresh moves after layout left
    /// new work scheduled at least twice in the failing flush.
    pub(crate) fn update_limit(
        &mut self,
        pending: Vec<(u64, String)>,
        refreshed: Vec<(u64, String, usize)>,
    ) {
        let mut message = format!(
            "Updates did not settle; pending components (not necessarily the cause): {pending:?}. Repair scheduling/focus feedback before retrying."
        );
        for (id, key, count) in &refreshed {
            message.push_str(&format!(
                " The pointer move the runtime repeats after each layout reached {key:?} (#{id}) and left new work scheduled {count} times in this flush: a pointer handler that invalidates layout, or writes state that leads to another layout, although nothing it derives from the event changed makes every layout produce the next move (see PointerEvent::is_refresh)."
            ));
        }
        let mut components: Vec<u64> = pending.iter().map(|(id, _)| *id).collect();
        for (id, _, _) in &refreshed {
            if !components.contains(id) {
                components.push(*id);
            }
        }
        self.update_limit = Some(ValidationIssue {
            code: ValidationCode::UpdateLimit,
            components,
            message,
        });
        self.dirty = true;
    }
    pub(crate) fn clear_update_limit(&mut self) {
        if self.update_limit.take().is_some() {
            self.dirty = true;
        }
    }

    pub(crate) fn mounted(&mut self) {
        // A successful corrected mount ends retained failed-admission markers.
        if !self.attempts.is_empty() {
            self.attempts.clear();
        }
        self.dirty = true;
    }

    pub(crate) fn report(&self, pending: bool) -> ValidationReport {
        let mut report = self.report.clone();
        report.enabled = true;
        report.pending = pending || self.dirty;
        report
    }

    pub(crate) fn refresh(&mut self) -> Refreshed {
        let ran = self.dirty;
        if !ran && self.report.logging_error.is_none() {
            return Refreshed::default();
        }
        let before = self.report.markers.clone();
        with_log(|log| self.refresh_to(log));
        Refreshed {
            ran,
            markers_changed: before != self.report.markers,
        }
    }

    /// Current markers paired with the window of the component they name, where
    /// that component is still known.
    pub(crate) fn markers(&self) -> impl Iterator<Item = (Option<u64>, &ValidationMarker)> {
        self.report.markers.iter().map(|marker| {
            (
                self.nodes.get(&marker.component).map(|node| node.window),
                marker,
            )
        })
    }

    /// Visible boxes of `window`'s measured components, in incarnation order.
    pub(crate) fn boxes(&self, window: u64) -> impl Iterator<Item = OverlayBox> + '_ {
        let measured = self.measured.contains(&window);
        self.nodes
            .values()
            .filter(move |node| measured && node.window == window)
            .filter_map(|node| {
                let border = visible_rect(node)?;
                let content = node
                    .box_model
                    .padding
                    .filter(|padding| padding.iter().all(|side| side.is_finite() && *side >= 0.0))
                    .and_then(|[top, right, bottom, left]| {
                        let outer = Rect::new(node.box_model.border_box?)?;
                        let inner = Rect {
                            x: outer.x + f64::from(left),
                            y: outer.y + f64::from(top),
                            w: outer.w - f64::from(left) - f64::from(right),
                            h: outer.h - f64::from(top) - f64::from(bottom),
                        };
                        (inner.w > 0.0 && inner.h > 0.0).then_some(inner)
                    })
                    .and_then(|inner| inner.intersect(border));
                Some(OverlayBox {
                    border: border.array(),
                    content: content.map(Rect::array),
                })
            })
    }

    fn refresh_to(&mut self, writer: &mut (impl Write + ?Sized)) {
        match self.refresh_to_result(writer) {
            Ok(()) => self.report.logging_error = None,
            Err(error) => self.report.logging_error = Some(error.to_string()),
        }
    }

    fn refresh_to_result(&mut self, writer: &mut (impl Write + ?Sized)) -> io::Result<()> {
        if !self.dirty && self.report.logging_error.is_none() {
            return Ok(());
        }
        if self.dirty {
            #[cfg(test)]
            {
                self.passes += 1;
            }
            let nodes = self.validated_nodes();
            let mut issues = check(&nodes);
            issues.extend(self.attempts.values().cloned());
            issues.extend(self.update_limit.iter().cloned());
            issues.sort_by_key(issue_key);
            issues.dedup();
            let mut markers: BTreeMap<u64, ValidationMarker> = BTreeMap::new();
            for issue in &issues {
                for &id in &issue.components {
                    let marker = markers.entry(id).or_insert_with(|| ValidationMarker {
                        component: id,
                        bounds: nodes.get(&id).and_then(visible_rect).map(Rect::array),
                        color: [1.0, 0.0, 0.0, 1.0],
                        codes: Vec::new(),
                    });
                    if !marker.codes.contains(&issue.code) {
                        marker.codes.push(issue.code);
                    }
                }
            }
            drop(nodes);
            self.report = ValidationReport {
                enabled: true,
                pending: false,
                issues,
                markers: markers.into_values().collect(),
                logging_error: None,
            };
            let current: BTreeSet<_> = self.report.issues.iter().map(issue_key).collect();
            // An issue withheld only because its window is not measured right
            // now stays logged, so it is not written again once validated.
            let nodes = &self.nodes;
            let measured = &self.measured;
            self.logged.retain(|key| {
                current.contains(key)
                    || (geometric(key.0)
                        && !key.1.is_empty()
                        && key.1.iter().all(|id| {
                            nodes
                                .get(id)
                                .is_some_and(|node| !measured.contains(&node.window))
                        }))
            });
            self.dirty = false;
        }
        for issue in &self.report.issues {
            let key = issue_key(issue);
            if !self.logged.contains(&key) {
                writeln!(
                    writer,
                    "[pert-ui validation ERROR {:?}] {}",
                    issue.code, issue.message
                )?;
                #[cfg(test)]
                {
                    self.logged_lines += 1;
                }
                self.logged.insert(key);
            }
        }
        Ok(())
    }
}

/// Runs `write` with the sink issue lines go to: standard error.
///
/// Direct writes, not `eprint!`, so a failing standard error is recorded in
/// [`ValidationReport::logging_error`] and retried instead of panicking.
#[cfg(not(test))]
fn with_log<R>(write: impl FnOnce(&mut dyn Write) -> R) -> R {
    write(&mut io::stderr().lock())
}

#[cfg(test)]
thread_local! {
    /// This crate's unit tests collect issue lines here instead: libtest
    /// captures only the `print!` family, so direct standard-error writes from
    /// tests that build invalid layouts on purpose would interleave with its
    /// output. Per thread, so each test sees only its own lines.
    static TEST_LOG: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Runs `write` with this thread's test log in place of standard error.
#[cfg(test)]
fn with_log<R>(write: impl FnOnce(&mut dyn Write) -> R) -> R {
    TEST_LOG.with_borrow_mut(|log| write(log))
}

/// Takes the issue lines this thread's validators wrote so far.
#[cfg(test)]
pub(crate) fn take_test_log() -> String {
    TEST_LOG.with_borrow_mut(|log| String::from_utf8_lossy_owned(std::mem::take(log)))
}

/// Whether `code` judges measured geometry, which is withheld in a window
/// that is not measured.
fn geometric(code: ValidationCode) -> bool {
    matches!(
        code,
        ValidationCode::InvalidBounds
            | ValidationCode::InvalidClip
            | ValidationCode::PaddingExceedsBounds
            | ValidationCode::EmptyHitArea
            | ValidationCode::UnexpectedOverlap
            | ValidationCode::EscapesParent
            | ValidationCode::CollapsedParent
            | ValidationCode::UnboundedCollection
    )
}

fn issue_key(issue: &ValidationIssue) -> IssueKey {
    (issue.code, issue.components.clone(), issue.message.clone())
}

fn same_collection(a: Option<CollectionExtent>, b: Option<CollectionExtent>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.rows == b.rows
                && a.horizontal == b.horizontal
                && a.content.to_bits() == b.content.to_bits()
                && a.allocated.to_bits() == b.allocated.to_bits()
        }
        _ => false,
    }
}

fn same_array(a: Option<[f32; 4]>, b: Option<[f32; 4]>) -> bool {
    a.map(|v| v.map(f32::to_bits)) == b.map(|v| v.map(f32::to_bits))
}

fn same_border(a: Option<crate::BorderInspection>, b: Option<crate::BorderInspection>) -> bool {
    a.map(|v| (v.width.to_bits(), v.color.map(f32::to_bits)))
        == b.map(|v| (v.width.to_bits(), v.color.map(f32::to_bits)))
}

fn same_node(a: &NodeData, b: &NodeData) -> bool {
    same_collection(a.collection, b.collection)
        && a.window == b.window
        && a.parent == b.parent
        && a.key == b.key
        && a.visible == b.visible
        && a.focused == b.focused
        && same_array(a.box_model.border_box, b.box_model.border_box)
        && same_array(a.box_model.margin, b.box_model.margin)
        && same_array(a.box_model.padding, b.box_model.padding)
        && same_array(a.box_model.foreground, b.box_model.foreground)
        && same_array(a.box_model.background, b.box_model.background)
        && same_border(a.box_model.border, b.box_model.border)
        && same_array(a.hints.clip, b.hints.clip)
        && a.hints.overlap == b.hints.overlap
        && a.hints.interactive == b.hints.interactive
        && a.hints.shortcuts == b.hints.shortcuts
}

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
impl Rect {
    fn new(value: [f32; 4]) -> Option<Self> {
        if !value.iter().all(|v| v.is_finite()) || value[2] < 0.0 || value[3] < 0.0 {
            return None;
        }
        Some(Self {
            x: value[0].into(),
            y: value[1].into(),
            w: value[2].into(),
            h: value[3].into(),
        })
    }
    fn intersect(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let w = (self.x + self.w).min(other.x + other.w) - x;
        let h = (self.y + self.h).min(other.y + other.h) - y;
        (w > 0.0 && h > 0.0).then_some(Self { x, y, w, h })
    }
    fn array(self) -> [f32; 4] {
        [self.x as f32, self.y as f32, self.w as f32, self.h as f32]
    }
}

/// How much two peers must share before it counts as an overlap, in logical
/// pixels.
///
/// Neighbours that tile a row or a column touch at a shared edge, and the two
/// sides of that edge are computed from different sums. When the coordinate has
/// no exact `f32` representation — a row at y = 208.8 under a fractional display
/// scale, say — one side can land a single step above the other, and the
/// rectangles then intersect by around 1.5e-5 logical pixels. That is not a
/// layout error, so the check ignores anything that cannot become a pixel: this
/// tolerance is roughly a thousand times the `f32` spacing at those coordinates
/// and at most an eighth of a physical pixel at the highest scale we support.
const OVERLAP_TOLERANCE: f64 = 1.0 / 32.0;

/// How far a child may leave its parent before it is reported, in logical pixels. The reasoning is the
/// one for [`OVERLAP_TOLERANCE`]: sums of fractional coordinates differ by
/// amounts that can never become a pixel.
const GEOMETRY_TOLERANCE: f64 = OVERLAP_TOLERANCE;

fn visible_rect(node: &NodeData) -> Option<Rect> {
    if !node.visible {
        return None;
    }
    let rect = Rect::new(node.box_model.border_box?)?;
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return None;
    }
    match node.hints.clip {
        Some(clip) => rect.intersect(Rect::new(clip)?),
        None => Some(rect),
    }
}

fn check(nodes: &BTreeMap<u64, NodeData>) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for (&id, node) in nodes {
        let mut emit = |code, message: &str| {
            issues.push(ValidationIssue {
                code,
                components: vec![id],
                message: format!("Component {:?} (#{id}): {message}", node.key),
            })
        };
        if node
            .box_model
            .border_box
            .is_some_and(|rect| Rect::new(rect).is_none())
        {
            emit(
                ValidationCode::InvalidBounds,
                "bounds must be finite with nonnegative width/height.",
            );
        }
        if node
            .hints
            .clip
            .is_some_and(|rect| Rect::new(rect).is_none())
        {
            emit(
                ValidationCode::InvalidClip,
                "effective clip must be finite with nonnegative width/height.",
            );
        }
        if node
            .box_model
            .margin
            .is_some_and(|v| v.iter().any(|x| !x.is_finite()))
        {
            emit(
                ValidationCode::InvalidMargin,
                "margin must be finite; finite negative margins are allowed.",
            );
        }
        if let Some(padding) = node.box_model.padding {
            if padding.iter().any(|x| !x.is_finite() || *x < 0.0) {
                emit(
                    ValidationCode::InvalidPadding,
                    "padding must be finite and nonnegative.",
                );
            } else if let Some(rect) = node.box_model.border_box.and_then(Rect::new)
                && (f64::from(padding[0]) + f64::from(padding[2]) > rect.h
                    || f64::from(padding[1]) + f64::from(padding[3]) > rect.w)
            {
                emit(
                    ValidationCode::PaddingExceedsBounds,
                    "padding exceeds the measured box and leaves negative content space.",
                );
            }
        }
        if node
            .box_model
            .border
            .is_some_and(|border| !border.width.is_finite() || border.width < 0.0)
        {
            emit(
                ValidationCode::InvalidBorderWidth,
                "border width must be finite and nonnegative.",
            );
        }
        for (name, color) in [
            ("foreground", node.box_model.foreground),
            ("background", node.box_model.background),
            ("border", node.box_model.border.map(|border| border.color)),
        ] {
            if color.is_some_and(|v| v.iter().any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))) {
                emit(
                    ValidationCode::InvalidColor,
                    &format!("{name} RGBA channels must be finite and within 0..=1."),
                );
            }
        }
        if node.visible
            && node.hints.interactive
            && node
                .box_model
                .border_box
                .and_then(Rect::new)
                .is_some_and(|rect| rect.w == 0.0 || rect.h == 0.0)
        {
            emit(
                ValidationCode::EmptyHitArea,
                "visible interactive component has zero measured width or height.",
            );
        }
        let bounds = node.box_model.border_box.and_then(Rect::new);
        if node.visible
            && bounds.is_some_and(|rect| rect.w == 0.0 || rect.h == 0.0)
            && nodes
                .values()
                .any(|child| child.parent == Some(id) && visible_rect(child).is_some())
        {
            emit(
                ValidationCode::CollapsedParent,
                "has zero measured width or height but visible children; its layout must reserve their space.",
            );
        }
        if node.hints.overlap == OverlapPolicy::Disallow
            && let Some(parent) = node.parent.and_then(|parent| nodes.get(&parent))
            && parent.window == node.window
            && parent.visible
            && let Some(outer) = parent.box_model.border_box.and_then(Rect::new)
            && outer.w > 0.0
            && outer.h > 0.0
            && let Some(inner) = visible_rect(node)
            && (inner.x < outer.x - GEOMETRY_TOLERANCE
                || inner.y < outer.y - GEOMETRY_TOLERANCE
                || inner.x + inner.w > outer.x + outer.w + GEOMETRY_TOLERANCE
                || inner.y + inner.h > outer.y + outer.h + GEOMETRY_TOLERANCE)
        {
            emit(
                ValidationCode::EscapesParent,
                &format!(
                    "visible bounds leave its parent {:?} without a clip; correct the parent's arrangement, clip its children, or allow an intentional overlay: report OverlapPolicy::Allow in the component's ComponentDetails::validation.",
                    parent.key
                ),
            );
        }
        if let Some(collection) = node.collection
            && collection.rows >= 100
            && collection.allocated > 0.0
            && collection.content <= collection.allocated + GEOMETRY_TOLERANCE
        {
            // Both in window coordinates, so a scaling ancestor cancels out.
            let along = |rect: Rect| {
                if collection.horizontal {
                    rect.w
                } else {
                    rect.h
                }
            };
            let shown = visible_rect(node).map_or(0.0, along);
            let whole = node
                .box_model
                .border_box
                .and_then(Rect::new)
                .map_or(0.0, along);
            if node.visible && whole > 0.0 && shown < whole / 2.0 {
                let owner = node
                    .parent
                    .and_then(|parent| nodes.get(&parent))
                    .map_or(String::new(), |parent| format!(" in {:?}", parent.key));
                emit(
                    ValidationCode::UnboundedCollection,
                    &format!(
                        "this collection{owner} was given its whole content extent ({:.0} px), so it mounts all {} rows and cannot scroll itself, while only {:.0}% of it is visible; give it a bounded extent (a SplitPane pane, Tab::fixed, RegionLayout::fixed, View::fixed or a grown child of a bounded Column, or of a bounded Row for a horizontal collection) instead of a scrolling ancestor; see VirtualList's bounded extent.",
                        collection.allocated,
                        collection.rows,
                        100.0 * shown / whole
                    ),
                );
            }
        }
        for binding in &node.hints.shortcuts {
            if binding.action.trim().is_empty() {
                emit(
                    ValidationCode::InvalidShortcut,
                    "shortcut action description must not be empty.",
                );
            }
        }
    }
    let peers: Vec<_> = nodes
        .iter()
        .filter(|(_, node)| node.hints.overlap == OverlapPolicy::Disallow)
        .filter_map(|(&id, node)| visible_rect(node).map(|rect| (id, node, rect)))
        .collect();
    for (index, &(a, first, first_rect)) in peers.iter().enumerate() {
        for &(b, second, second_rect) in &peers[index + 1..] {
            if first.window == second.window
                && first.parent == second.parent
                && first_rect.intersect(second_rect).is_some_and(|shared| {
                    shared.w > OVERLAP_TOLERANCE && shared.h > OVERLAP_TOLERANCE
                })
            {
                issues.push(ValidationIssue { code: ValidationCode::UnexpectedOverlap, components: vec![a, b],
                    message: format!("Components {:?} (#{a}) and {:?} (#{b}) overlap; correct their layout, or allow an intentional overlay: report OverlapPolicy::Allow in ComponentDetails::validation, or let the parent stack its children (Component::stacks_children).", first.key, second.key) });
            }
        }
    }
    type Chord = (u64, KeyCode, bool, bool, bool, bool);
    let mut chords: BTreeMap<Chord, Vec<(u64, &str, &str)>> = BTreeMap::new();
    for (&id, node) in nodes.iter().filter(|(_, node)| node.visible) {
        for binding in node.hints.shortcuts.iter().filter(|binding| {
            binding.enabled && (binding.scope == ShortcutScope::Window || node.focused)
        }) {
            let Modifiers {
                shift,
                control,
                alt,
                super_key,
            } = binding.modifiers;
            let chord = (node.window, binding.key, shift, control, alt, super_key);
            let previous = chords.entry(chord).or_default();
            for &(other, key, action) in previous.iter() {
                let components = if other == id {
                    vec![id]
                } else {
                    vec![other, id]
                };
                issues.push(ValidationIssue { code: ValidationCode::ShortcutConflict, components,
                    message: format!("Shortcut {:?} {:?} is active for both {action:?} on {key:?} (#{other}) and {:?} on {:?} (#{id}); change the binding or its active scope.", binding.key, binding.modifiers, binding.action, node.key) });
            }
            previous.push((id, &node.key, &binding.action));
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> NodeData {
        NodeData {
            collection: None,
            window: 1,
            parent: None,
            key: "control".into(),
            visible: true,
            focused: false,
            box_model: BoxModelInspection::default(),
            hints: ValidationHints::default(),
        }
    }

    #[test]
    fn unchanged_invalid_data_reuses_validation_and_errors_log_once_until_fixed() {
        let mut validator = Validator::default();
        let mut data = node();
        data.box_model.padding = Some([f32::NAN; 4]);
        let mut log = Vec::new();
        validator.update(1, data.clone());
        validator.refresh_to(&mut log);
        let first_len = log.len();
        assert!(first_len > 0);
        validator.update(1, data.clone());
        validator.refresh_to(&mut log);
        assert_eq!(validator.passes, 1);
        assert_eq!(log.len(), first_len);
        validator.flags(1, true, true);
        validator.refresh_to(&mut log);
        assert_eq!(log.len(), first_len);
        validator.update(1, node());
        validator.refresh_to(&mut log);
        assert!(validator.report(false).markers.is_empty());
        validator.update(1, data);
        validator.refresh_to(&mut log);
        assert_eq!(log.len(), first_len * 2);
    }

    #[test]
    fn refresh_writes_issue_lines_to_the_test_log() {
        let _ = take_test_log();
        let mut validator = Validator::default();
        validator.duplicate(1, "control");
        assert!(validator.refresh().ran);
        let log = take_test_log();
        assert!(
            log.starts_with("[pert-ui validation ERROR DuplicateKey]"),
            "{log}"
        );
        assert_eq!(validator.report(false).logging_error, None);
        // A logged issue is not written again.
        validator.duplicate(1, "control");
        validator.refresh();
        assert_eq!(take_test_log(), "");
    }

    #[test]
    fn failed_logging_remains_observable_and_retries() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("log unavailable"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut validator = Validator::default();
        validator.duplicate(1, "control");
        validator.refresh_to(&mut Broken);
        let report = validator.report(false);
        assert_eq!(report.logging_error.as_deref(), Some("log unavailable"));
        assert!(!report.pending);
        assert_eq!(validator.passes, 1);
        assert_eq!(report.markers.len(), 1);
        let mut log = Vec::new();
        validator.refresh_to(&mut log);
        assert!(!log.is_empty());
        assert!(validator.report(false).logging_error.is_none());
        assert!(!validator.report(false).pending);
        assert_eq!(validator.passes, 1);
    }

    struct UnavailableLog;
    impl Write for UnavailableLog {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("log unavailable"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_log_retry_reuses_the_finished_report_without_a_validation_pass() {
        let _ = take_test_log();
        let mut validator = Validator::default();
        validator.duplicate(1, "control");
        validator.refresh_to(&mut UnavailableLog);
        let before = validator.report(false);
        assert!(!before.pending);
        assert_eq!(validator.passes, 1);
        let refreshed = validator.refresh();
        assert!(!refreshed.ran);
        assert!(!refreshed.markers_changed);
        assert_eq!(validator.passes, 1);
        assert_eq!(validator.report(false).markers, before.markers);
        assert_eq!(validator.report(false).issues, before.issues);
        assert!(validator.report(false).logging_error.is_none());
        assert!(take_test_log().contains("DuplicateKey"));
        assert!(!validator.refresh().ran);
        assert!(take_test_log().is_empty());
    }

    #[test]
    fn partial_log_recovery_retries_only_the_unwritten_issue_without_rechecking() {
        struct OneLine(Vec<u8>);
        impl Write for OneLine {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.0.contains(&b'\n') {
                    return Err(io::Error::other("second line unavailable"));
                }
                self.0.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut validator = Validator::default();
        validator.duplicate(1, "first");
        validator.duplicate(2, "second");
        let mut partial = OneLine(Vec::new());
        validator.refresh_to(&mut partial);
        assert_eq!(validator.passes, 1);
        assert_eq!(validator.logged_lines, 1);
        assert!(!validator.report(false).pending);
        assert!(validator.report(false).logging_error.is_some());
        let mut retry = Vec::new();
        validator.refresh_to(&mut retry);
        let text = String::from_utf8(retry).unwrap();
        assert!(text.contains("second"));
        assert!(!text.contains("first"));
        assert_eq!(validator.passes, 1);
        assert_eq!(validator.logged_lines, 2);
        assert!(validator.report(false).logging_error.is_none());
    }

    #[test]
    fn a_corrected_issue_is_removed_instead_of_retrying_its_old_log() {
        let mut validator = Validator::default();
        validator.duplicate(1, "control");
        validator.refresh_to(&mut UnavailableLog);
        validator.remove(1);
        let mut log = Vec::new();
        validator.refresh_to(&mut log);
        assert!(log.is_empty());
        assert_eq!(validator.passes, 2);
        let report = validator.report(false);
        assert!(report.issues.is_empty());
        assert!(report.markers.is_empty());
        assert!(!report.pending);
        assert!(report.logging_error.is_none());
        validator.duplicate(1, "control");
        validator.refresh_to(&mut log);
        assert_eq!(validator.passes, 3);
        assert!(!log.is_empty());
    }

    #[test]
    fn geometry_is_validated_only_in_a_measured_window() {
        let mut empty = node();
        empty.hints.interactive = true;
        empty.box_model.border_box = Some([0.0, 0.0, 0.0, 20.0]);
        empty.box_model.padding = Some([f32::NAN; 4]);
        let codes = |validator: &Validator| -> Vec<_> {
            validator
                .report(false)
                .issues
                .iter()
                .map(|issue| issue.code)
                .collect()
        };
        let mut validator = Validator::default();
        validator.update(1, empty);
        validator.refresh_to(&mut Vec::new());
        // Padding values are checked anywhere; the empty box is not.
        assert_eq!(codes(&validator), [ValidationCode::InvalidPadding]);
        assert_eq!(validator.boxes(1).count(), 0);
        validator.set_measured(BTreeSet::from([1]));
        validator.refresh_to(&mut Vec::new());
        assert_eq!(
            codes(&validator),
            [ValidationCode::InvalidPadding, ValidationCode::EmptyHitArea]
        );
        validator.set_measured(BTreeSet::new());
        validator.refresh_to(&mut Vec::new());
        assert_eq!(codes(&validator), [ValidationCode::InvalidPadding]);
    }

    #[test]
    fn an_issue_withheld_while_its_window_is_unmeasured_is_not_logged_again() {
        let mut empty = node();
        empty.hints.interactive = true;
        empty.box_model.border_box = Some([0.0, 0.0, 0.0, 20.0]);
        let mut validator = Validator::default();
        validator.update(1, empty);
        validator.set_measured(BTreeSet::from([1]));
        let mut log = Vec::new();
        validator.refresh_to(&mut log);
        assert_eq!(validator.logged_lines, 1);
        validator.set_measured(BTreeSet::new());
        validator.refresh_to(&mut log);
        assert!(validator.report(false).issues.is_empty());
        validator.set_measured(BTreeSet::from([1]));
        validator.refresh_to(&mut log);
        assert_eq!(validator.report(false).issues.len(), 1);
        assert_eq!(
            validator.logged_lines, 1,
            "the unchanged issue is not logged twice"
        );
        // Once validation confirms it fixed, it is logged again if it returns.
        validator.update(1, node());
        validator.refresh_to(&mut log);
        let mut empty = node();
        empty.hints.interactive = true;
        empty.box_model.border_box = Some([0.0, 0.0, 0.0, 20.0]);
        validator.update(1, empty);
        validator.refresh_to(&mut log);
        assert_eq!(validator.logged_lines, 2);
    }

    #[test]
    fn peers_that_only_touch_are_not_overlapping() {
        // Two rows of a grid at a fractional coordinate: the first row's end and
        // the second row's start are the same edge, but f32 rounds them apart.
        let stacked = |y: f32| {
            let mut data = node();
            data.box_model.border_box = Some([24.0, y, 128.0, 48.0]);
            data
        };
        let mut validator = Validator::default();
        validator.set_measured(BTreeSet::from([1]));
        validator.update(1, stacked(208.8));
        validator.update(2, stacked(256.8));
        validator.refresh_to(&mut Vec::new());
        assert!(
            validator.report(false).issues.is_empty(),
            "{:?}",
            validator.report(false).issues,
        );

        // A shared edge that a pixel could show is still an overlap.
        let mut validator = Validator::default();
        validator.set_measured(BTreeSet::from([1]));
        validator.update(1, stacked(208.8));
        validator.update(2, stacked(256.8 - 0.5));
        validator.refresh_to(&mut Vec::new());
        assert_eq!(
            validator
                .report(false)
                .issues
                .iter()
                .map(|issue| issue.code)
                .collect::<Vec<_>>(),
            [ValidationCode::UnexpectedOverlap],
        );
    }

    #[test]
    fn malformed_geometry_and_colors_do_not_cascade_into_overlap() {
        let mut invalid = node();
        invalid.box_model.border_box = Some([0.0, 0.0, f32::NAN, 1.0]);
        invalid.box_model.margin = Some([f32::INFINITY; 4]);
        invalid.box_model.background = Some([2.0; 4]);
        invalid.hints.clip = Some([0.0, 0.0, -1.0, 1.0]);
        let codes: BTreeSet<_> = check(&BTreeMap::from([(1, invalid)]))
            .iter()
            .map(|i| i.code)
            .collect();
        assert_eq!(
            codes,
            BTreeSet::from([
                ValidationCode::InvalidBounds,
                ValidationCode::InvalidMargin,
                ValidationCode::InvalidColor,
                ValidationCode::InvalidClip,
            ])
        );
        let mut bordered = node();
        bordered.box_model.border = Some(crate::BorderInspection {
            width: -1.0,
            color: [f32::NAN, 0.0, 0.0, 1.0],
        });
        let codes: BTreeSet<_> = check(&BTreeMap::from([(1, bordered)]))
            .iter()
            .map(|i| i.code)
            .collect();
        assert_eq!(
            codes,
            BTreeSet::from([
                ValidationCode::InvalidColor,
                ValidationCode::InvalidBorderWidth,
            ])
        );
        let mut padded = node();
        padded.box_model.border_box = Some([0.0, 0.0, 10.0, 10.0]);
        padded.box_model.padding = Some([6.0; 4]);
        assert_eq!(
            check(&BTreeMap::from([(1, padded)]))[0].code,
            ValidationCode::PaddingExceedsBounds
        );
    }
}
