//! The pointer move the runtime repeats after layout (`PointerEvent::is_refresh`),
//! how a custom drag converges on it, and the `UpdateLimit` evidence for one that
//! does not.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type Log = Rc<RefCell<Vec<(PointerKind, bool, Point)>>>;

/// A board that drags its one child; `guarded` compares the derived offset
/// before invalidating, as a converging handler must.
struct Board {
    item: Vec<Element>,
    at: (f32, f32),
    drag: Option<(Point, (f32, f32))>,
    guarded: Rc<std::cell::Cell<bool>>,
    log: Log,
}
impl Board {
    fn new(guarded: bool, log: &Log) -> Self {
        Self {
            item: vec![Element::new("item", Label::new("Item"))],
            at: (0.0, 0.0),
            drag: None,
            guarded: Rc::new(std::cell::Cell::new(guarded)),
            log: log.clone(),
        }
    }
}
impl Component for Board {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.item)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::new(self.at.0, self.at.1, 40.0, 20.0)?)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(local) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        self.log
            .borrow_mut()
            .push((event.kind(), event.is_refresh(), local));
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                cx.capture_pointer()?;
                self.drag = Some((local, self.at));
            }
            PointerKind::Move => {
                let Some((start, from)) = self.drag else {
                    return Ok(InputOutcome::Ignored);
                };
                let at = (
                    from.0 + local.x() - start.x(),
                    from.1 + local.y() - start.y(),
                );
                if !self.guarded.get() || at != self.at {
                    self.at = at;
                    cx.invalidate(Invalidation::Layout);
                }
            }
            PointerKind::Release(PointerButton::Primary) => {
                self.drag = None;
                cx.release_pointer();
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
}

fn point(x: f32, y: f32) -> Point {
    Point::new(x, y).unwrap()
}
fn send(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) -> Result<InputOutcome, RuntimeError> {
    ui.pointer_input(PointerEvent::new(point(x, y), kind))
}
fn mount(ui: &mut Runtime, board: Board) -> WidgetId {
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui.mount_tree(Element::new("board", board)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    root
}
#[cfg(debug_assertions)]
fn update_limit_message(ui: &Runtime) -> Option<String> {
    ui.validation_report()
        .issues
        .into_iter()
        .find(|issue| issue.code == ValidationCode::UpdateLimit)
        .map(|issue| issue.message)
}
/// Presses and drags, returning the first failure of the drag's input or flush.
fn drag(ui: &mut Runtime) -> Result<(), RuntimeError> {
    send(ui, 10.0, 10.0, PointerKind::Press(PointerButton::Primary))?;
    send(ui, 60.0, 30.0, PointerKind::Move)?;
    ui.flush(Duration::ZERO)?;
    Ok(())
}

#[test]
fn an_unguarded_drag_never_settles_and_the_report_names_it() {
    let log = Log::default();
    let mut ui = Runtime::new();
    mount(&mut ui, Board::new(false, &log));
    assert_eq!(drag(&mut ui).unwrap_err(), RuntimeError::UpdateLimit);
    assert!(
        log.borrow()
            .iter()
            .filter(|(_, refresh, _)| *refresh)
            .count()
            > 2,
        "each layout repeated the move"
    );
    #[cfg(debug_assertions)]
    {
        let message = update_limit_message(&ui).expect("an UpdateLimit issue");
        assert!(message.contains("\"board\""), "{message}");
        assert!(message.contains("PointerEvent::is_refresh"), "{message}");
    }
}

#[test]
fn a_drag_that_compares_its_derived_offset_settles() {
    let log = Log::default();
    let mut ui = Runtime::new();
    let root = mount(&mut ui, Board::new(true, &log));
    drag(&mut ui).unwrap();
    send(
        &mut ui,
        60.0,
        30.0,
        PointerKind::Release(PointerButton::Primary),
    )
    .unwrap();
    let item = ui.find_descendant(&root, "item").unwrap().unwrap();
    assert_eq!(
        ui.bounds(&item).unwrap().map(|b| (b.x(), b.y())),
        Some((50.0, 20.0))
    );
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
    #[cfg(debug_assertions)]
    assert_eq!(update_limit_message(&ui), None);
}

#[test]
fn only_the_runtime_repeat_is_a_refresh() {
    let log = Log::default();
    let mut ui = Runtime::new();
    let board = mount(&mut ui, Board::new(true, &log));
    // Hover: a host move, then the same position again from the host.
    send(&mut ui, 10.0, 10.0, PointerKind::Move).unwrap();
    send(&mut ui, 10.0, 10.0, PointerKind::Move).unwrap();
    assert!(log.borrow().iter().all(|(_, refresh, _)| !refresh));
    assert!(!PointerEvent::new(point(1.0, 1.0), PointerKind::Move).is_refresh());
    // A layout with the pointer resting over the board repeats the move to
    // the hover target.
    log.borrow_mut().clear();
    ui.invalidate(&board, Invalidation::Layout).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        *log.borrow(),
        [(PointerKind::Move, true, point(10.0, 10.0))],
        "one refresh to the hover target"
    );
    // Under capture, the capture owner receives it.
    log.borrow_mut().clear();
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    send(&mut ui, 400.0, 10.0, PointerKind::Move).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let refreshes: Vec<_> = log
        .borrow()
        .iter()
        .filter(|(_, refresh, _)| *refresh)
        .map(|(kind, _, local)| (*kind, *local))
        .collect();
    assert_eq!(
        refreshes,
        [(PointerKind::Move, point(400.0, 10.0))],
        "the captured owner, outside its bounds"
    );
}

/// A capture owner whose own origin moves under a stationary pointer.
struct Shifting {
    child: Vec<Element>,
    offset: Signal<f32>,
    shown: f32,
}
impl Component for Shifting {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let offset = cx.read(&self.offset.read_only())?;
        if offset != self.shown {
            self.shown = offset;
            cx.request_layout();
        }
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.child)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::new(self.shown, 0.0, 100.0, 100.0)?)
    }
}
#[test]
fn a_capture_owner_moved_under_a_stationary_pointer_sees_its_new_local_position() {
    let log = Log::default();
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let offset = ui.signal(0.0_f32);
    let inner = Board::new(true, &log);
    ui.mount_tree(Element::new(
        "shifting",
        Shifting {
            child: vec![Element::new("board", inner)],
            offset: offset.clone(),
            shown: 0.0,
        },
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    send(
        &mut ui,
        30.0,
        30.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    log.borrow_mut().clear();
    // The board moves 20 pixels right while the pointer rests.
    offset.set(20.0).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // The first refresh moves the dragged item, the second derives the same
    // offset and changes nothing: the flush settled.
    assert!(!log.borrow().is_empty());
    assert!(
        log.borrow()
            .iter()
            .all(|entry| *entry == (PointerKind::Move, true, point(10.0, 30.0))),
        "the refresh carries the new local position: {:?}",
        log.borrow()
    );
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
}

/// Arrangement that always asks for another update while update always asks
/// for layout: a loop that has nothing to do with the pointer.
struct Restless;
impl Component for Restless {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.request_layout();
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(20.0, 20.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        cx.request_update();
        Ok(())
    }
}
#[test]
fn an_unrelated_loop_under_a_resting_pointer_keeps_the_generic_report() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let restless = ui.signal(false);
    let shown = restless.clone();
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([
                Element::new("button", Button::new("Inert", |_| Ok(()))),
                Element::new(
                    "loop",
                    Show::new(&shown, |_| Element::new("restless", Restless)),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let button = ui.find_descendant(&root, "button").unwrap().unwrap();
    let [x, y, w, h] = ui.bounds(&button).unwrap().unwrap().values();
    send(&mut ui, x + w / 2.0, y + h / 2.0, PointerKind::Move).unwrap();
    restless.set(true).unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::UpdateLimit
    );
    #[cfg(debug_assertions)]
    {
        let message = update_limit_message(&ui).expect("an UpdateLimit issue");
        assert!(!message.contains("is_refresh"), "{message}");
    }
}

#[test]
fn evidence_belongs_to_the_flush_that_failed() {
    let log = Log::default();
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let board = Board::new(false, &log);
    let guarded = board.guarded.clone();
    let restless = ui.signal(false);
    let shown = restless.clone();
    ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new("board", board),
            Element::new(
                "loop",
                Show::new(&shown, |_| Element::new("restless", Restless)),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(drag(&mut ui).unwrap_err(), RuntimeError::UpdateLimit);
    #[cfg(debug_assertions)]
    assert!(update_limit_message(&ui).unwrap().contains("\"board\""));
    // The same board, now converging, settles while the drag goes on.
    guarded.set(true);
    ui.flush(Duration::ZERO).unwrap();
    #[cfg(debug_assertions)]
    assert_eq!(update_limit_message(&ui), None, "a settled flush clears it");
    // An unrelated loop elsewhere carries no evidence about the board.
    restless.set(true).unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::UpdateLimit
    );
    #[cfg(debug_assertions)]
    {
        let message = update_limit_message(&ui).expect("an UpdateLimit issue");
        assert!(
            !message.contains("is_refresh"),
            "no stale evidence: {message}"
        );
    }
}

/// A board whose handler writes a counter on every move; its offset comes
/// from a mapped signal, so the loop runs through a derived computation.
struct Derived {
    item: Vec<Element>,
    moves: Signal<u32>,
    offset: ReadSignal<f32>,
    at: f32,
    dragging: bool,
}
impl Component for Derived {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let at = cx.read(&self.offset)?;
        if at != self.at {
            self.at = at;
            cx.request_layout();
        }
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.item)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::new(self.at, 0.0, 40.0, 20.0)?)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                cx.capture_pointer()?;
                self.dragging = true;
            }
            PointerKind::Move if self.dragging => {
                self.moves.update(|moves| *moves += 1)?;
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
}
#[test]
fn a_loop_through_a_derived_signal_is_named_too() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let moves = ui.signal(0_u32);
    let offset = moves.map(|moves| (*moves % 50) as f32).unwrap();
    ui.mount_tree(Element::new(
        "derived",
        Derived {
            item: vec![Element::new("item", Label::new("Item"))],
            moves,
            offset,
            at: 0.0,
            dragging: false,
        },
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(drag(&mut ui).unwrap_err(), RuntimeError::UpdateLimit);
    #[cfg(debug_assertions)]
    {
        let message = update_limit_message(&ui).expect("an UpdateLimit issue");
        assert!(message.contains("\"derived\""), "{message}");
    }
}

/// Starts a drag session on its first move after a press and records every
/// pointer event it receives.
struct Source {
    armed: bool,
    log: Log,
}
impl Component for Source {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 40.0)?)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let local = cx.to_local(event.position()).unwrap_or(event.position());
        self.log
            .borrow_mut()
            .push((event.kind(), event.is_refresh(), local));
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => self.armed = true,
            PointerKind::Move if self.armed => {
                self.armed = false;
                cx.start_drag(
                    DragPayload::new(1_u32),
                    DropOperations::MOVE,
                    DragPreview::element(Element::new("preview", Label::new("Dragged"))),
                )?;
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
}
#[test]
fn a_drag_session_receives_no_refresh_while_it_runs() {
    let log = Log::default();
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal("Short".to_owned());
    ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new(
                "source",
                Source {
                    armed: false,
                    log: log.clone(),
                },
            ),
            Element::new("label", Label::new(&text)),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    send(&mut ui, 30.0, 10.0, PointerKind::Move).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.drag_active(), "the session runs");
    log.borrow_mut().clear();
    // A layout while the session runs and the pointer rests.
    text.set("A much longer label than before".to_owned())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        log.borrow().iter().all(|(_, refresh, _)| !refresh),
        "{:?}",
        log.borrow()
    );
}
