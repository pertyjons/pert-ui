//! The Drag & drop page, composed through the public facade.
//!
//! The custom components here are written as a consumer would write them: a drag
//! source and two drop targets, using only public extension points.
use crate::gallery::{label, page, section};
use pert_ui::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

/// An application's ordered track names, shown as padded rows.
struct Tracks(Vec<String>);
impl ListItems for Tracks {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<Element> {
        let key = self.key(index)?;
        Some(Element::new(
            key.clone(),
            Container::new(Element::new("name", Label::new(key))).padding(Length::from(8)),
        ))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.0.iter().position(|candidate| candidate == key)
    }
}

/// Removes the dropped keys wherever they are and inserts them at the position:
/// the one edit a move is.
fn apply_drop(lists: &mut [Vec<String>], target: usize, drop: &ListDrop<'_>) {
    let moved = drop.items().to_vec();
    // The position is resolved before anything is removed, because it may name
    // one of the dragged rows: dropping a row beside itself changes nothing.
    let list = &lists[target];
    let boundary = match drop.position() {
        DropPosition::Before(key) => list.iter().position(|k| k == key).unwrap_or(list.len()),
        DropPosition::After(key) => list
            .iter()
            .position(|k| k == key)
            .map_or(list.len(), |index| index + 1),
        _ => list.len(),
    };
    let moving = drop.operation() == DropOperation::Move;
    let leaving = list[..boundary]
        .iter()
        .filter(|key| moving && moved.contains(key))
        .count();
    let index = boundary - leaving;
    if moving {
        for list in lists.iter_mut() {
            list.retain(|key| !moved.contains(key));
        }
    }
    let list = &mut lists[target];
    // A copy of a key the list already holds would duplicate an identity.
    let incoming: Vec<_> = moved
        .into_iter()
        .filter(|key| !list.contains(key))
        .collect();
    let index = index.min(list.len());
    list.splice(index..index, incoming);
}

fn track_lists(ui: &Runtime, status: &Signal<String>) -> Result<Element, RuntimeError> {
    let names = |prefix: &str, count: usize| -> Vec<String> {
        (1..=count).map(|n| format!("{prefix} {n}")).collect()
    };
    let model = Rc::new(RefCell::new(vec![names("Drums", 12), names("Synth", 4)]));
    let sources: Vec<_> = model
        .borrow()
        .iter()
        .map(|list| ui.signal(ListSource::new(Tracks(list.clone()))))
        .collect();
    let mut columns = Vec::new();
    for (index, (key, title)) in [("list-a", "Bus A"), ("list-b", "Bus B")]
        .into_iter()
        .enumerate()
    {
        let (model, sources, status) = (model.clone(), sources.clone(), status.clone());
        let list = VirtualList::new(sources[index].read_only(), 34.0)?
            .selection(ListSelection::new(ui, SelectionMode::Multiple))
            .drag_operations(DropOperations::MOVE | DropOperations::COPY)
            .reorderable(move |_, drop| {
                // The outline below takes the same payload type but keeps its
                // own model; a move is one edit, so its nodes are refused
                // rather than copied here and left there.
                if !drop
                    .items()
                    .iter()
                    .all(|key| model.borrow().iter().any(|list| list.contains(key)))
                {
                    return Ok(false);
                }
                apply_drop(&mut model.borrow_mut(), index, drop);
                for (list, source) in model.borrow().iter().zip(&sources) {
                    source.set(ListSource::new(Tracks(list.clone())))?;
                }
                status.set(format!(
                    "{:?} of {} to {title}, {:?}",
                    drop.operation(),
                    drop.items().join(", "),
                    drop.position()
                ))?;
                Ok(true)
            });
        columns.push(Element::new(
            format!("{key}-column"),
            Group::titled(
                title,
                Element::new(
                    "frame",
                    Container::new(Element::new(key, list).with_label(title))
                        .preferred_size(Size::from([200, 220])),
                ),
            ),
        ));
    }
    Ok(Element::new("lists", Wrap::new(columns)))
}

/// A session's folders and tracks: children per folder, `""` holding the top
/// level. A name ending in `/` is a folder, which takes dropped tracks.
#[derive(Clone)]
struct Session(BTreeMap<String, Vec<String>>);
impl TreeItems for Session {
    fn roots(&self) -> Vec<String> {
        self.children("")
    }
    fn children(&self, key: &str) -> Vec<String> {
        self.0.get(key).cloned().unwrap_or_default()
    }
    fn can_expand(&self, key: &str) -> bool {
        key.ends_with('/')
    }
    fn can_drop_into(&self, key: &str) -> bool {
        key.ends_with('/')
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key.trim_end_matches('/'))))
    }
}

/// Moves the dropped nodes as one edit. A node dragged together with its
/// folder travels inside the folder, so only the topmost dragged nodes move.
fn apply_tree_drop(model: &mut BTreeMap<String, Vec<String>>, drop: &ListDrop<'_>) {
    let parent_of = |model: &BTreeMap<String, Vec<String>>, key: &str| {
        model
            .iter()
            .find(|(_, children)| children.iter().any(|child| child == key))
            .map(|(parent, _)| parent.clone())
    };
    let dragged = drop.items();
    let moved: Vec<String> = dragged
        .iter()
        .filter(|key| {
            let mut parent = parent_of(model, key);
            while let Some(folder) = parent.filter(|folder| !folder.is_empty()) {
                if dragged.contains(&folder) {
                    return false;
                }
                parent = parent_of(model, &folder);
            }
            true
        })
        .cloned()
        .collect();
    let named = match drop.position() {
        DropPosition::Before(key) | DropPosition::After(key) | DropPosition::Into(key) => {
            Some(key.clone())
        }
        _ => None,
    };
    // Beside or into a dragged node itself, nothing moves.
    if named.as_ref().is_some_and(|key| moved.contains(key)) {
        return;
    }
    for children in model.values_mut() {
        children.retain(|key| !moved.contains(key));
    }
    match drop.position() {
        DropPosition::Before(key) | DropPosition::After(key) => {
            let parent = parent_of(model, key).unwrap_or_default();
            let siblings = model.entry(parent).or_default();
            let at = siblings
                .iter()
                .position(|sibling| sibling == key)
                .map_or(siblings.len(), |at| {
                    at + usize::from(matches!(drop.position(), DropPosition::After(_)))
                });
            siblings.splice(at..at, moved);
        }
        DropPosition::Into(key) => model.entry(key.clone()).or_default().extend(moved),
        _ => model.entry(String::new()).or_default().extend(moved),
    }
}

fn session_outline(ui: &Runtime, status: &Signal<String>) -> Result<Element, RuntimeError> {
    let model: BTreeMap<String, Vec<String>> = [
        ("", vec!["Drums/", "Keys/", "Archive/", "Vocal"]),
        ("Drums/", vec!["Kick", "Snare", "Hat"]),
        ("Keys/", vec!["Piano", "Organ"]),
        ("Archive/", vec![]),
    ]
    .into_iter()
    .map(|(folder, names)| {
        (
            folder.to_owned(),
            names.into_iter().map(str::to_owned).collect(),
        )
    })
    .collect();
    let model = Rc::new(RefCell::new(model));
    let state = TreeController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Multiple),
        TreeSource::new(Session(model.borrow().clone())),
    )?;
    state.set_expanded("Drums/", true)?;
    state.set_expanded("Keys/", true)?;
    let (published, status) = (state.clone(), status.clone());
    let tree = Tree::new(&state, 30.0)?.reorderable(move |_, drop| {
        // Rows from the lists above belong to their model, which this one
        // cannot remove them from, so only the outline's own nodes move.
        if !drop.is_internal() {
            return Ok(false);
        }
        apply_tree_drop(&mut model.borrow_mut(), drop);
        published.set_source(TreeSource::new(Session(model.borrow().clone())))?;
        if let DropPosition::Into(folder) = drop.position() {
            published.set_expanded(folder, true)?;
        }
        status.set(format!(
            "Moved {} {:?}",
            drop.items().join(", "),
            drop.position()
        ))?;
        Ok(true)
    });
    Ok(Element::new(
        "outline-frame",
        Container::new(Element::new("outline", tree).with_label("Outline"))
            .preferred_size(Size::from([260, 260])),
    ))
}

/// What a chip carries: the application's name for it, never a position.
struct Swatch(&'static str);

/// A draggable chip: a press that moves far enough starts a session.
struct Chip {
    name: &'static str,
    role: ColorRole,
    armed: Option<Point>,
    dragging: bool,
}
impl Chip {
    fn new(name: &'static str, role: ColorRole) -> Self {
        Self {
            name,
            role,
            armed: None,
            dragging: false,
        }
    }
    fn face(name: &str, role: ColorRole) -> Container {
        Container::new(Element::new(
            "name",
            Label::new(name).color_role(ColorRole::OnSurfaceContainer),
        ))
        .padding(Length::from(10))
        .background_role(role)
    }
}
impl Component for Chip {
    fn take_children(&mut self) -> Vec<Element> {
        vec![Element::new("face", Self::face(self.name, self.role))]
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        Some(CursorIcon::Move)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                self.armed = Some(event.position());
                cx.capture_pointer()?;
            }
            PointerKind::Move => {
                if let Some(start) = self.armed
                    && (event.position().x() - start.x()).hypot(event.position().y() - start.y())
                        >= DRAG_THRESHOLD
                {
                    self.armed = None;
                    self.dragging = true;
                    cx.invalidate(Invalidation::Paint);
                    cx.start_drag(
                        DragPayload::new(Swatch(self.name)),
                        // All three, so that the open bin shows each
                        // operation's cursor as the modifiers ask for it.
                        DropOperations::MOVE | DropOperations::COPY | DropOperations::LINK,
                        DragPreview::element(Element::new(
                            "chip",
                            Self::face(self.name, self.role),
                        ))
                        .hot_spot(Point::new(12.0, 12.0)?),
                    )?;
                }
            }
            PointerKind::Release(_) => {
                self.armed = None;
                cx.release_pointer();
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn drag_finished(&mut self, cx: &mut EventContext, _: DragOutcome) -> Result<(), RuntimeError> {
        // Presentation only: the bin recorded the drop as its own edit.
        self.dragging = false;
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.armed = None;
        cx.invalidate(Invalidation::Paint);
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        // The chip left behind is outlined while its drag is in progress.
        if self.dragging
            && let Some(size) = painter.size()
        {
            let width = Length::new(painter.metric(MetricRole::FocusWidth))?;
            painter.stroke_shape(
                Rect::from_size(size),
                Shape::Rectangle,
                width,
                painter.color(ColorRole::Primary),
            )?;
        }
        Ok(())
    }
}

/// A drop zone for chips that reports what arrived, or refuses everything.
struct Bin {
    title: &'static str,
    accepts: bool,
    received: Signal<String>,
    over: Option<DropResponse>,
}
impl Bin {
    fn new(title: &'static str, accepts: bool, received: Signal<String>) -> Self {
        Self {
            title,
            accepts,
            received,
            over: None,
        }
    }
}
impl Component for Bin {
    fn take_children(&mut self) -> Vec<Element> {
        vec![Element::new(
            "content",
            Container::new(Element::new(
                "rows",
                Column::new([
                    Element::new("title", Label::new(self.title).style(TextStyle::Heading)),
                    Element::new("received", Label::new(self.received.read_only())),
                ]),
            ))
            .padding(Length::from(12))
            .background_role(ColorRole::SurfaceContainer)
            .preferred_size(Size::from([200, 96])),
        )]
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn drag_event(
        &mut self,
        cx: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        let Some(swatch) = event.payload().get::<Swatch>() else {
            return Ok(DropResponse::Rejected);
        };
        let response = match event.phase() {
            DragPhase::Leave => None,
            // A locked bin claims the drag so that it can say no: the cursor
            // shows that nothing can be dropped here.
            _ if !self.accepts => Some(DropResponse::Forbidden),
            DragPhase::Drop => {
                self.received
                    .set(format!("{:?}: {}", event.preferred(), swatch.0))?;
                None
            }
            _ => Some(DropResponse::Accepted(event.preferred())),
        };
        if self.over != response {
            self.over = response;
            cx.invalidate(Invalidation::Paint);
        }
        Ok(match (event.phase(), self.accepts) {
            (DragPhase::Leave, _) => DropResponse::Rejected,
            (_, false) => DropResponse::Forbidden,
            _ => DropResponse::Accepted(event.preferred()),
        })
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let (Some(over), Some(size)) = (self.over, painter.size()) else {
            return Ok(());
        };
        let role = match over {
            DropResponse::Accepted(_) => ColorRole::Primary,
            DropResponse::Rejected | DropResponse::Forbidden => ColorRole::Error,
        };
        let width = Length::new(painter.metric(MetricRole::FocusWidth))?;
        painter.stroke_shape(
            Rect::from_size(size),
            Shape::Rectangle,
            width,
            painter.color(role),
        )
    }
}

pub(crate) fn drag_and_drop(ui: &Runtime) -> Result<Element, RuntimeError> {
    let status = ui.signal("Drag a row, or press Alt with an arrow on the active row.".to_owned());
    let open = ui.signal("Nothing received yet.".to_owned());
    let locked = ui.signal("Refuses every drop.".to_owned());
    let outline = ui.signal("Drag a track or a folder.".to_owned());
    Ok(page([
        section(
            "reorder",
            "Reorderable lists",
            [
                label(
                    "help",
                    "Drag rows within a list or between the two. Shift or Control selects several rows, which are dragged together. Hold the copy modifier to copy instead of move. Resting near an edge scrolls, Escape cancels, and the wheel still scrolls under a drag.",
                ),
                track_lists(ui, &status)?,
                Element::new("drop-status", Label::new(status.read_only())),
            ],
        ),
        section(
            "custom",
            "Custom source and targets",
            [
                label(
                    "help",
                    "The chips start a session with their own payload type. One bin accepts it, the other claims the drag to refuse it, and the lists above do not recognize it at all. Over the open bin the copy modifier asks for a copy and adding Shift (Command on macOS) for a link, each with its own cursor.",
                ),
                Element::new(
                    "chips",
                    Wrap::new([
                        Element::new("chip-kick", Chip::new("Kick", ColorRole::SurfaceContainer)),
                        Element::new(
                            "chip-snare",
                            Chip::new("Snare", ColorRole::SurfaceContainer),
                        ),
                        Element::new("chip-hat", Chip::new("Hat", ColorRole::SurfaceContainer)),
                    ]),
                ),
                Element::new(
                    "bins",
                    Wrap::new([
                        Element::new("bin-open", Bin::new("Accepts chips", true, open)),
                        Element::new("bin-locked", Bin::new("Locked", false, locked)),
                    ]),
                ),
            ],
        ),
        section(
            "outline-section",
            "Reorderable tree",
            [
                label(
                    "help",
                    "Drag a track before or after another, or onto the middle of a folder to put it inside, even an empty or collapsed one. After the last track of a folder, how far right the pointer is decides whether the track stays in the folder. A folder cannot go inside itself. With the keyboard, Alt with Up or Down moves the active node among its siblings, Left moves it out of its folder and Right into the folder above it.",
                ),
                session_outline(ui, &outline)?,
                Element::new("outline-status", Label::new(outline.read_only())),
            ],
        ),
    ]))
}
