//! The drag-and-drop contract, through the facade.
//!
//! These pin the rules in [drag and drop](../../../docs/contracts.md): how a
//! session starts, how its target is found while the source holds the capture,
//! what each side is told and in which order, and every way a session ends.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

#[derive(Debug, PartialEq)]
struct Item(&'static str);

/// Everything the participants were told, in order.
#[derive(Default)]
struct Journal(RefCell<Vec<String>>);
impl Journal {
    fn push(&self, line: impl Into<String>) {
        self.0.borrow_mut().push(line.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut self.0.borrow_mut())
    }
}

fn point(x: f32, y: f32) -> Point {
    Point::new(x, y).unwrap()
}
fn size(width: u16, height: u16) -> Size {
    Size::from([width, height])
}

type FinishHook = Box<dyn FnMut(&mut EventContext) -> Result<(), RuntimeError>>;

/// Starts a drag of `Item(name)` once an armed press has moved far enough.
struct Source {
    name: &'static str,
    size: Size,
    permitted: DropOperations,
    preview: bool,
    broken_preview: bool,
    focusable: bool,
    armed: Option<Point>,
    journal: Rc<Journal>,
    start_error: Rc<Cell<Option<RuntimeError>>>,
    finish: Option<FinishHook>,
}
impl Source {
    fn new(name: &'static str, journal: &Rc<Journal>) -> Self {
        Self {
            name,
            size: size(40, 40),
            permitted: DropOperations::MOVE | DropOperations::COPY,
            preview: false,
            broken_preview: false,
            focusable: true,
            armed: None,
            journal: journal.clone(),
            start_error: Rc::default(),
            finish: None,
        }
    }
    /// A source that never holds keyboard focus, so focus moving elsewhere
    /// does not take its input context.
    fn unfocusable(mut self) -> Self {
        self.focusable = false;
        self
    }
    /// A preview whose element tree cannot mount: two children share a key.
    fn broken_preview(mut self) -> Self {
        self.broken_preview = true;
        self
    }
    fn permitted(mut self, permitted: DropOperations) -> Self {
        self.permitted = permitted;
        self
    }
    fn preview(mut self) -> Self {
        self.preview = true;
        self
    }
}
impl Component for Source {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        self.focusable
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(self.size)
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
                    let preview = if self.broken_preview {
                        DragPreview::element(Element::new(
                            "broken",
                            Column::new([
                                Element::new("same", Label::new("a")),
                                Element::new("same", Label::new("b")),
                            ]),
                        ))
                    } else if self.preview {
                        DragPreview::element(Element::new("chip", Painted::new(&self.journal)))
                            .hot_spot(point(5.0, 5.0))
                    } else {
                        DragPreview::none()
                    };
                    match cx.start_drag(DragPayload::new(Item(self.name)), self.permitted, preview)
                    {
                        Ok(()) => self.journal.push(format!("{}: started", self.name)),
                        Err(error) => self.start_error.set(Some(error)),
                    }
                }
            }
            PointerKind::Release(_) => {
                if self.armed.take().is_some() {
                    self.journal.push(format!("{}: clicked", self.name));
                }
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.journal
            .push(format!("{}: key {:?}", self.name, event.code));
        Ok(InputOutcome::Handled)
    }
    fn drag_event(
        &mut self,
        _: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        // A source may be its own target, which it is only for the link operation
        // here so the other tests see it reject.
        if event.permitted() == DropOperations::LINK {
            self.journal
                .push(format!("{}: own {:?}", self.name, event.phase()));
            return Ok(DropResponse::Accepted(DropOperation::Link));
        }
        Ok(DropResponse::Rejected)
    }
    fn drag_finished(
        &mut self,
        cx: &mut EventContext,
        outcome: DragOutcome,
    ) -> Result<(), RuntimeError> {
        self.journal
            .push(format!("{}: finished {outcome:?}", self.name));
        match &mut self.finish {
            Some(finish) => finish(cx),
            None => Ok(()),
        }
    }
}

/// A fixed-size piece of preview content that counts how often it is prepared.
struct Painted {
    journal: Rc<Journal>,
}
impl Painted {
    fn new(journal: &Rc<Journal>) -> Self {
        Self {
            journal: journal.clone(),
        }
    }
}
impl Component for Painted {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.journal.push("preview: update");
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(size(30, 20))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        self.journal.push("preview: paint");
        p.fill_rect(Rect::new(0.0, 0.0, 30.0, 20.0)?, Color::ACCENT);
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Answer {
    Accept,
    AcceptLink,
    Reject,
    RefuseDrop,
    Fail,
    /// Accepts, then fails when told to let go.
    FailLeave,
    /// Accepts, and fails when the wheel is turned over it.
    FailScroll,
}

/// Answers drags of `Item` and records everything it is told.
struct Target {
    name: &'static str,
    size: Size,
    answer: Answer,
    children: Vec<Element>,
    journal: Rc<Journal>,
}
impl Target {
    fn new(name: &'static str, answer: Answer, journal: &Rc<Journal>) -> Self {
        Self {
            name,
            size: size(100, 60),
            answer,
            children: Vec::new(),
            journal: journal.clone(),
        }
    }
    fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
    fn children(mut self, children: Vec<Element>) -> Self {
        self.children = children;
        self
    }
}
impl Component for Target {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(self.size)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        // Children sit in the top-left corner at their preferred size.
        for index in 0..cx.len() {
            let size = cx.measure(index, Constraints::default())?;
            cx.place(index, Rect::from_size(size))?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if let PointerKind::Scroll(_) = event.kind() {
            if self.answer == Answer::FailScroll {
                return Err(RuntimeError::Hidden);
            }
            self.journal.push(format!("{}: scrolled", self.name));
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn drag_event(
        &mut self,
        cx: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        let item = event
            .payload()
            .downcast_ref::<Item>()
            .map_or("?", |item| item.0);
        let local = cx.to_local(event.position()).unwrap();
        self.journal.push(format!(
            "{}: {:?} {:?} {item} at {},{} prefers {:?}",
            self.name,
            event.role(),
            event.phase(),
            local.x(),
            local.y(),
            event.preferred()
        ));
        if event.payload().downcast_ref::<Item>().is_none() {
            return Ok(DropResponse::Rejected);
        }
        Ok(match (self.answer, event.phase()) {
            (Answer::Fail, _) | (Answer::FailLeave, DragPhase::Leave) => {
                return Err(RuntimeError::Hidden);
            }
            (Answer::Reject, _) | (Answer::RefuseDrop, DragPhase::Drop) => DropResponse::Rejected,
            (Answer::AcceptLink, _) => DropResponse::Accepted(DropOperation::Link),
            _ => DropResponse::Accepted(event.preferred()),
        })
    }
}

/// Source at the left, the supplied content to its right, in a 400 × 200 window.
struct Stage {
    ui: Runtime,
    root: WidgetId,
    journal: Rc<Journal>,
}
impl Stage {
    fn new(
        source: impl FnOnce(Source) -> Source,
        content: impl FnOnce(&Rc<Journal>) -> Vec<Element>,
    ) -> Self {
        let journal = Rc::new(Journal::default());
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
            .unwrap();
        let mut children = vec![Element::new(
            "source",
            source(Source::new("source", &journal)),
        )];
        children.extend(content(&journal));
        let root = ui
            .mount_tree(Element::new(
                "row",
                Row::new(children).gap(Length::new(0.0).unwrap()),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        journal.take();
        Self { ui, root, journal }
    }
    fn child(&self, index: usize) -> WidgetId {
        self.ui.children(&self.root).unwrap()[index].clone()
    }
    fn pointer(&mut self, x: f32, y: f32, kind: PointerKind) -> Result<InputOutcome, RuntimeError> {
        let result = self.ui.pointer_input(PointerEvent::new(point(x, y), kind));
        self.ui.flush(self.ui.now())?;
        result
    }
    fn press(&mut self, x: f32, y: f32) {
        self.pointer(x, y, PointerKind::Press(PointerButton::Primary))
            .unwrap();
    }
    fn moved(&mut self, x: f32, y: f32) {
        self.pointer(x, y, PointerKind::Move).unwrap();
    }
    fn release(&mut self, x: f32, y: f32) {
        self.pointer(x, y, PointerKind::Release(PointerButton::Primary))
            .unwrap();
    }
    /// Presses the source and crosses the threshold, staying over the source.
    fn start(&mut self) {
        self.press(10.0, 10.0);
        self.moved(16.0, 10.0);
        assert!(self.ui.drag_active());
        assert_eq!(self.journal.take()[0], "source: started");
    }
}

fn one_target(answer: Answer) -> impl FnOnce(&Rc<Journal>) -> Vec<Element> {
    move |journal| {
        vec![Element::new(
            "target",
            Target::new("target", answer, journal),
        )]
    }
}

#[test]
fn a_press_without_movement_stays_a_click() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.press(10.0, 10.0);
    stage.moved(12.0, 11.0);
    assert!(!stage.ui.drag_active(), "below the threshold");
    stage.release(12.0, 11.0);
    assert_eq!(stage.journal.take(), ["source: clicked"]);
    assert_eq!(stage.ui.next_deadline(), None);
}

#[test]
fn a_drop_tells_the_target_first_and_then_the_source() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    // The source holds the capture, and the target is still found.
    stage.moved(60.0, 10.0);
    stage.moved(70.0, 20.0);
    stage.release(70.0, 20.0);
    assert_eq!(
        stage.journal.take(),
        [
            "target: Candidate Enter source at 20,10 prefers Move",
            "target: Candidate Over source at 30,20 prefers Move",
            "target: Candidate Drop source at 30,20 prefers Move",
            "source: finished Dropped(Move)",
        ]
    );
    assert!(!stage.ui.drag_active());
    assert_eq!(
        stage.ui.next_deadline(),
        None,
        "a session leaves nothing scheduled"
    );
    // The capture ended with the session: the next press is an ordinary press.
    stage.press(10.0, 10.0);
    stage.release(10.0, 10.0);
    assert_eq!(stage.journal.take(), ["source: clicked"]);
}

#[test]
fn a_rejecting_component_defers_to_the_ancestor_that_accepts() {
    let mut stage = Stage::new(
        |s| s,
        |journal| {
            vec![Element::new(
                "list",
                Target::new("list", Answer::Accept, journal).children(vec![Element::new(
                    "row",
                    Target::new("row", Answer::Reject, journal).size(size(50, 30)),
                )]),
            )]
        },
    );
    stage.start();
    stage.moved(50.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        [
            "row: Candidate Enter source at 10,10 prefers Move",
            "list: Candidate Enter source at 10,10 prefers Move",
        ]
    );
    // The row is asked again; the list, now the target, is told the pointer moved.
    stage.moved(55.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        [
            "row: Candidate Enter source at 15,10 prefers Move",
            "list: Candidate Over source at 15,10 prefers Move",
        ]
    );
    // Off the row but still on the list.
    stage.moved(120.0, 50.0);
    assert_eq!(
        stage.journal.take(),
        ["list: Candidate Over source at 80,50 prefers Move"]
    );
    stage.release(120.0, 50.0);
    assert_eq!(
        stage.journal.take(),
        [
            "list: Candidate Drop source at 80,50 prefers Move",
            "source: finished Dropped(Move)",
        ]
    );
}

#[test]
fn a_descendant_that_accepts_takes_the_drag_from_its_ancestor() {
    let mut stage = Stage::new(
        |s| s,
        |journal| {
            vec![Element::new(
                "list",
                Target::new("list", Answer::Accept, journal).children(vec![Element::new(
                    "row",
                    Target::new("row", Answer::Accept, journal).size(size(50, 30)),
                )]),
            )]
        },
    );
    stage.start();
    stage.moved(120.0, 50.0);
    stage.journal.take();
    stage.moved(50.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        [
            "row: Candidate Enter source at 10,10 prefers Move",
            "list: Candidate Leave source at 10,10 prefers Move",
            // The list is now an ancestor of the target, and is told as one.
            "list: Observer Over source at 10,10 prefers Move",
        ]
    );
    stage.release(50.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        [
            "row: Candidate Drop source at 10,10 prefers Move",
            "list: Observer Leave source at 10,10 prefers Move",
            "source: finished Dropped(Move)",
        ]
    );
}

#[test]
fn leaving_a_target_and_releasing_over_nothing_is_unclaimed() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.moved(300.0, 150.0);
    assert_eq!(
        stage.journal.take(),
        [
            "target: Candidate Enter source at 20,10 prefers Move",
            "target: Candidate Leave source at 260,150 prefers Move",
        ]
    );
    stage.release(300.0, 150.0);
    assert_eq!(stage.journal.take(), ["source: finished Unclaimed"]);
}

#[test]
fn a_release_outside_the_window_is_unclaimed_even_over_a_last_known_target() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let outside = PointerEvent::new(
        point(60.0, -20.0),
        PointerKind::Release(PointerButton::Primary),
    )
    .outside();
    stage.ui.pointer_input(outside).unwrap();
    assert_eq!(
        stage.journal.take(),
        [
            "target: Candidate Leave source at 20,10 prefers Move",
            "source: finished Unclaimed",
        ]
    );
}

#[test]
fn a_refused_drop_is_not_a_cancellation() {
    let mut stage = Stage::new(|s| s, one_target(Answer::RefuseDrop));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.release(60.0, 10.0);
    assert_eq!(
        stage.journal.take()[1..],
        [
            "target: Candidate Drop source at 20,10 prefers Move",
            "source: finished Refused",
        ]
    );
}

#[test]
fn escape_cancels_the_session_before_any_other_key_routing() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    stage
        .ui
        .key_input(KeyCode::Escape, KeyAction::Press)
        .unwrap();
    assert_eq!(
        stage.journal.take(),
        [
            "target: Candidate Leave source at 20,10 prefers Move",
            "source: finished Cancelled",
        ],
        "the focused source never saw the key"
    );
    assert!(!stage.ui.drag_active());
    // The button is still down; its release is nobody's click and nobody's drop.
    stage.release(60.0, 10.0);
    assert_eq!(stage.journal.take(), Vec::<String>::new());
}

#[test]
fn focus_loss_and_input_cancellation_cancel_the_session() {
    for cancel in [
        (|ui: &mut Runtime| ui.clear_focus()) as fn(&mut Runtime),
        |ui: &mut Runtime| ui.cancel_input().unwrap(),
    ] {
        let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
        stage.start();
        stage.moved(60.0, 10.0);
        stage.journal.take();
        cancel(&mut stage.ui);
        assert_eq!(
            stage.journal.take(),
            [
                "target: Candidate Leave source at 20,10 prefers Move",
                "source: finished Cancelled",
            ]
        );
        assert!(!stage.ui.drag_active());
    }
}

#[test]
fn removing_the_source_ends_the_session_and_tells_the_target() {
    let mut stage = Stage::new(|s| s.preview(), one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let source = stage.child(0);
    stage.ui.unmount(&source).unwrap();
    assert!(
        stage.ui.next_deadline().is_some(),
        "the host is asked to flush so the session can end"
    );
    stage.ui.flush(stage.ui.now()).unwrap();
    // The row closed the gap the source left, so only the phase is pinned.
    let told = stage.journal.take();
    assert_eq!(told.len(), 1, "nobody is left to tell on the source's side");
    assert!(told[0].starts_with("target: Candidate Leave source"));
    assert!(!stage.ui.drag_active());
    assert!(
        stage
            .ui
            .inspect()
            .components
            .iter()
            .all(|c| c.key != "chip")
    );
    assert_eq!(stage.ui.next_deadline(), None);
}

#[test]
fn hiding_the_source_cancels_like_any_lost_input_context() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let source = stage.child(0);
    stage.ui.set_visible(&source, false).unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    // The row closed the gap the hidden source left, so only the phase is pinned.
    assert!(stage.journal.take()[0].starts_with("target: Candidate Leave source"));
    assert!(!stage.ui.drag_active());
}

#[test]
fn a_removed_target_is_never_called_again_and_the_session_continues() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let target = stage.child(1);
    stage.ui.unmount(&target).unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    assert!(stage.ui.drag_active());
    stage.release(60.0, 10.0);
    assert_eq!(stage.journal.take(), ["source: finished Unclaimed"]);
}

#[test]
fn layout_under_a_still_pointer_resolves_the_target_again() {
    let mut stage = Stage::new(
        |s| s,
        |journal| {
            vec![
                Element::new("first", Target::new("first", Answer::Accept, journal)),
                Element::new("second", Target::new("second", Answer::Accept, journal)),
            ]
        },
    );
    stage.start();
    stage.moved(60.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        ["first: Candidate Enter source at 20,10 prefers Move"]
    );
    // The first target disappears; the second slides under the pointer.
    let first = stage.child(1);
    stage.ui.set_visible(&first, false).unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    assert_eq!(
        stage.journal.take(),
        ["second: Candidate Enter source at 20,10 prefers Move"],
        "a hidden component is not called, and nothing waited for the pointer to move"
    );
}

#[test]
fn a_payload_of_an_unknown_type_finds_no_target() {
    struct Other;
    struct Foreign(Option<Point>);
    impl Component for Foreign {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(size(40, 40))
        }
        fn pointer_input(
            &mut self,
            cx: &mut EventContext,
            event: PointerEvent,
        ) -> Result<InputOutcome, RuntimeError> {
            match event.kind() {
                PointerKind::Press(_) => {
                    self.0 = Some(event.position());
                    cx.capture_pointer()?;
                }
                PointerKind::Move if self.0.take().is_some() => {
                    cx.start_drag(
                        DragPayload::new(Other),
                        DropOperations::ALL,
                        DragPreview::none(),
                    )?;
                }
                _ => {}
            }
            Ok(InputOutcome::Handled)
        }
    }
    let journal = Rc::new(Journal::default());
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new("foreign", Foreign(None)),
            Element::new("target", Target::new("target", Answer::Accept, &journal)),
        ])
        .gap(Length::new(0.0).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let mut send = |x: f32, kind| {
        ui.pointer_input(PointerEvent::new(point(x, 10.0), kind))
            .unwrap();
    };
    send(10.0, PointerKind::Press(PointerButton::Primary));
    send(20.0, PointerKind::Move);
    send(60.0, PointerKind::Move);
    assert_eq!(
        journal.take(),
        ["target: Candidate Enter ? at 20,10 prefers Move"]
    );
    assert_eq!(
        ui.cursor_icon(),
        CursorIcon::Grabbing,
        "asked, and rejected"
    );
    assert_eq!(ui.inspect().drag.unwrap().target, None);
}

#[test]
fn modifiers_choose_the_operation_and_the_cursor_shows_it() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::Grabbing);
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let copy = if cfg!(target_os = "macos") {
        KeyCode::AltLeft
    } else {
        KeyCode::ControlLeft
    };
    stage.ui.key_input(copy, KeyAction::Press).unwrap();
    assert_eq!(
        stage.journal.take(),
        [
            "target: Candidate Over source at 20,10 prefers Copy",
            // Only Escape is taken by the session; other keys route as usual.
            format!("source: key {copy:?}").as_str(),
        ],
        "a still pointer is told about the modifier"
    );
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::Copy);
    stage.release(60.0, 10.0);
    assert_eq!(stage.journal.take()[1], "source: finished Dropped(Copy)");
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::Default);
}

#[test]
fn a_target_may_forbid_part_of_itself_and_shows_no_drop_there() {
    struct Picky(Rc<Journal>);
    impl Component for Picky {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(size(100, 60))
        }
        fn drag_event(
            &mut self,
            cx: &mut EventContext,
            event: &DragEvent<'_>,
        ) -> Result<DropResponse, RuntimeError> {
            self.0.push(format!("picky: {:?}", event.phase()));
            // Only its left half takes a drop; the rest is its own, and refuses.
            Ok(if cx.to_local(event.position()).unwrap().x() < 50.0 {
                DropResponse::Accepted(event.preferred())
            } else {
                DropResponse::Forbidden
            })
        }
    }
    let mut stage = Stage::new(
        |s| s,
        |journal| vec![Element::new("picky", Picky(journal.clone()))],
    );
    stage.start();
    stage.moved(60.0, 10.0);
    stage.moved(120.0, 10.0);
    assert_eq!(stage.journal.take(), ["picky: Enter", "picky: Over"]);
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::NoDrop);
    stage.release(120.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        ["picky: Leave", "source: finished Cancelled"]
    );
}

#[test]
fn a_target_that_rejects_gives_the_drag_up_to_its_ancestor() {
    struct Fickle(Rc<Journal>);
    impl Component for Fickle {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(size(80, 40))
        }
        fn drag_event(
            &mut self,
            cx: &mut EventContext,
            event: &DragEvent<'_>,
        ) -> Result<DropResponse, RuntimeError> {
            self.0.push(format!("fickle: {:?}", event.phase()));
            // Its left half is its own; its right half is not its business.
            Ok(if cx.to_local(event.position()).unwrap().x() < 40.0 {
                DropResponse::Accepted(event.preferred())
            } else {
                DropResponse::Rejected
            })
        }
    }
    let mut stage = Stage::new(
        |s| s,
        |journal| {
            vec![Element::new(
                "list",
                Target::new("list", Answer::Accept, journal)
                    .children(vec![Element::new("fickle", Fickle(journal.clone()))]),
            )]
        },
    );
    stage.start();
    stage.moved(50.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        [
            "fickle: Enter",
            "list: Observer Over source at 10,10 prefers Move"
        ]
    );
    stage.moved(100.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        [
            "fickle: Over",
            "list: Candidate Enter source at 60,10 prefers Move",
            "fickle: Leave",
        ],
        "the list stops observing and becomes the target"
    );
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::Grabbing);
}

#[test]
fn accepting_an_operation_the_source_forbids_is_an_error_that_ends_the_session() {
    let mut stage = Stage::new(
        |s| s.permitted(DropOperations::MOVE),
        one_target(Answer::AcceptLink),
    );
    stage.start();
    let error = stage
        .ui
        .pointer_input(PointerEvent::new(point(60.0, 10.0), PointerKind::Move))
        .unwrap_err();
    assert_eq!(error, RuntimeError::DropNotPermitted);
    assert!(!stage.ui.drag_active());
    assert_eq!(
        stage.journal.take().last().unwrap(),
        "source: finished Cancelled"
    );
}

#[test]
fn a_failing_participant_cancels_and_its_error_surfaces() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Fail));
    stage.start();
    let error = stage
        .ui
        .pointer_input(PointerEvent::new(point(60.0, 10.0), PointerKind::Move))
        .unwrap_err();
    assert_eq!(error, RuntimeError::Hidden);
    assert!(!stage.ui.drag_active());
    assert_eq!(
        stage.journal.take().last().unwrap(),
        "source: finished Cancelled"
    );
    // Ordinary hover resumes at the next flush, and then nothing is scheduled.
    stage.ui.flush(stage.ui.now()).unwrap();
    assert_eq!(stage.ui.next_deadline(), None);
}

#[test]
fn a_release_is_resolved_where_it_happens_not_where_the_pointer_last_moved() {
    let mut stage = Stage::new(
        |s| s,
        |journal| {
            vec![
                Element::new("first", Target::new("first", Answer::Accept, journal)),
                Element::new("second", Target::new("second", Answer::Accept, journal)),
            ]
        },
    );
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    // No motion is delivered between the two positions.
    stage.release(160.0, 20.0);
    assert_eq!(
        stage.journal.take(),
        [
            "second: Candidate Enter source at 20,20 prefers Move",
            "first: Candidate Leave source at 120,20 prefers Move",
            "second: Candidate Drop source at 20,20 prefers Move",
            "source: finished Dropped(Move)",
        ],
        "the first target never sees a drop meant for the second"
    );
}

#[test]
fn a_failure_while_handing_the_drag_over_still_ends_every_part() {
    let mut stage = Stage::new(
        |s| s,
        |journal| {
            vec![
                Element::new("first", Target::new("first", Answer::FailLeave, journal)),
                Element::new("second", Target::new("second", Answer::Accept, journal)),
            ]
        },
    );
    stage.ui.set_debug_mode(DebugMode::Inspect);
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let error = stage
        .ui
        .pointer_input(PointerEvent::new(point(160.0, 10.0), PointerKind::Move))
        .unwrap_err();
    assert_eq!(error, RuntimeError::Hidden);
    assert_eq!(
        stage.journal.take(),
        [
            "second: Candidate Enter source at 20,10 prefers Move",
            "first: Candidate Leave source at 120,10 prefers Move",
            // The first target was told once; the second, which had claimed the
            // drag, is told that it ended.
            "second: Candidate Leave source at 20,10 prefers Move",
            "source: finished Cancelled",
        ]
    );
    let ended = stage
        .ui
        .inspect()
        .events
        .into_iter()
        .find(|event| event.kind == EventKind::DragEnd)
        .unwrap();
    assert!(
        matches!(ended.status, EventStatus::Failed(_)),
        "history keeps the error that ended the session: {:?}",
        ended.status
    );
}

#[test]
fn an_error_from_the_wheel_ends_the_session_like_any_participant_error() {
    let mut stage = Stage::new(|s| s.preview(), one_target(Answer::FailScroll));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let error = stage
        .ui
        .pointer_input(PointerEvent::new(
            point(60.0, 10.0),
            PointerKind::Scroll(point(0.0, 10.0)),
        ))
        .unwrap_err();
    assert_eq!(error, RuntimeError::Hidden);
    assert!(!stage.ui.drag_active());
    assert_eq!(
        stage.journal.take(),
        [
            "target: Candidate Leave source at 20,10 prefers Move",
            "source: finished Cancelled",
        ]
    );
    assert!(
        stage
            .ui
            .inspect()
            .components
            .iter()
            .all(|c| c.key != "chip")
    );
}

#[test]
fn a_request_that_cannot_become_a_session_still_tells_its_source() {
    let mut stage = Stage::new(|s| s.broken_preview(), one_target(Answer::Accept));
    stage.press(10.0, 10.0);
    let error = stage
        .ui
        .pointer_input(PointerEvent::new(point(16.0, 10.0), PointerKind::Move))
        .unwrap_err();
    assert_eq!(error, RuntimeError::DuplicateKey("same".into()));
    assert!(!stage.ui.drag_active());
    assert_eq!(
        stage.journal.take(),
        ["source: started", "source: finished Cancelled"],
        "`start_drag` had succeeded, so the source is owed its ending"
    );
    // Nothing of the failed request is left: the next drag starts normally.
    stage.release(16.0, 10.0);
    stage.ui.flush(stage.ui.now()).unwrap();
    assert_eq!(stage.ui.next_deadline(), None);
}

#[test]
fn a_request_made_outside_pointer_input_is_adopted_by_the_next_flush() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    let source = stage.child(0);
    stage.press(10.0, 10.0);
    stage
        .ui
        .event(&source, |cx| {
            cx.start_drag(
                DragPayload::new(Item("later")),
                DropOperations::MOVE,
                DragPreview::none(),
            )
        })
        .unwrap()
        .unwrap();
    assert!(!stage.ui.drag_active(), "not yet adopted");
    assert!(stage.ui.next_deadline().is_some());
    assert!(
        stage.ui.application_deadline().is_some(),
        "a host scheduling all windows is asked to flush too"
    );
    stage.ui.flush(stage.ui.now()).unwrap();
    assert!(stage.ui.drag_active());
    stage.moved(60.0, 10.0);
    stage.release(60.0, 10.0);
    assert!(
        stage
            .journal
            .take()
            .contains(&"target: Candidate Drop later at 20,10 prefers Move".to_owned())
    );
}

#[test]
fn a_session_in_a_detached_window_ends_before_that_window_is_retired() {
    let journal = Rc::new(Journal::default());
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let owner = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([
                Tab::new(
                    "drag",
                    "Drag",
                    Element::new("source", Source::new("source", &journal).preview()),
                ),
                Tab::new("other", "Other", Element::new("label", Label::new("Other"))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let view = ui.children(&owner).unwrap()[0].clone();
    let prepared = ui
        .prepare_detach(&view, NativeWindowOptions::new("Detached").unwrap())
        .unwrap();
    let detached = ui.commit_detach(prepared).unwrap();
    ui.select_window(&detached).unwrap();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let source = ui
        .inspect()
        .components
        .iter()
        .find(|c| c.key == "source")
        .and_then(|c| c.details.box_model.border_box)
        .expect("the source is laid out in its own window");
    let (x, y) = (source[0] + 10.0, source[1] + 10.0);
    for (dx, kind) in [
        (0.0, PointerKind::Press(PointerButton::Primary)),
        (6.0, PointerKind::Move),
    ] {
        ui.pointer_input(PointerEvent::new(point(x + dx, y), kind))
            .unwrap();
    }
    assert!(ui.drag_active());
    assert!(ui.inspect().drag.unwrap().preview.is_some());
    // Removing the owner retires the detached window the session lives in.
    ui.select_window(&ui.primary_window()).unwrap();
    ui.unmount(&owner).unwrap();
    assert!(!ui.contains_window(&detached));
    assert!(!ui.drag_active());
    assert!(
        ui.inspect().components.is_empty(),
        "the preview root did not outlive its window: {:?}",
        ui.inspect()
            .components
            .iter()
            .map(|c| c.key.clone())
            .collect::<Vec<_>>()
    );
    ui.flush(ui.now()).unwrap();
    assert_eq!(ui.next_deadline(), None);
}

/// Places its children at the origin, each inside a bound clip, optionally
/// stacking them so a later or raised child covers an earlier one.
struct Frame {
    children: Vec<Element>,
    clip: ReadSignal<Rect>,
    current: Rect,
    stacks: bool,
}
impl Component for Frame {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.current = cx.read(&self.clip)?;
        Ok(())
    }
    fn stacks_children(&self) -> bool {
        self.stacks
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(size(100, 60))
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place_clipped(index, Rect::from_size(size), self.current)?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.validation.overlap = OverlapPolicy::Allow;
        details
    }
}

#[test]
fn a_clip_that_uncovers_the_pointer_resolves_the_target_again() {
    let mut stage = Stage::new(|s| s, |_| Vec::new());
    let signal = stage.ui.signal(Rect::new(0.0, 0.0, 100.0, 60.0).unwrap());
    let journal = stage.journal.clone();
    stage
        .ui
        .reconcile_children(
            &stage.root,
            vec![
                Element::new("source", Source::new("source", &journal)),
                Element::new(
                    "frame",
                    Frame {
                        children: vec![Element::new(
                            "inner",
                            Target::new("inner", Answer::Accept, &journal),
                        )],
                        clip: signal.read_only(),
                        current: Rect::new(0.0, 0.0, 100.0, 60.0).unwrap(),
                        stacks: false,
                    },
                ),
            ],
        )
        .unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    stage.journal.take();
    stage.start();
    stage.moved(120.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        ["inner: Candidate Enter source at 80,10 prefers Move"]
    );
    // The clip shrinks to the left half: the target keeps its bounds, but the
    // still pointer is no longer over anything it shows.
    signal
        .set(Rect::new(0.0, 0.0, 50.0, 60.0).unwrap())
        .unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    assert_eq!(
        stage.journal.take(),
        ["inner: Candidate Leave source at 80,10 prefers Move"]
    );
    assert_eq!(stage.ui.inspect().drag.unwrap().target, None);
}

#[test]
fn raising_a_covered_sibling_resolves_the_target_again() {
    // A source without keyboard focus keeps its input context when focus moves.
    let mut stage = Stage::new(|s| s.unfocusable(), |_| Vec::new());
    let signal = stage.ui.signal(Rect::new(0.0, 0.0, 100.0, 60.0).unwrap());
    let journal = stage.journal.clone();
    stage
        .ui
        .reconcile_children(
            &stage.root,
            vec![
                Element::new("source", Source::new("source", &journal).unfocusable()),
                Element::new(
                    "frame",
                    Frame {
                        children: vec![
                            Element::new("below", Target::new("below", Answer::Accept, &journal)),
                            Element::new("above", Target::new("above", Answer::Accept, &journal)),
                        ],
                        clip: signal.read_only(),
                        current: Rect::new(0.0, 0.0, 100.0, 60.0).unwrap(),
                        stacks: true,
                    },
                ),
            ],
        )
        .unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    stage.journal.take();
    stage.start();
    stage.moved(60.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        ["above: Candidate Enter source at 20,10 prefers Move"]
    );
    // Focusing the covered sibling raises it; nothing moved, yet it is now what
    // the still pointer is over.
    let frame = stage.child(1);
    let below = stage.ui.children(&frame).unwrap()[0].clone();
    stage.ui.focus(&below).unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    assert_eq!(
        stage.journal.take(),
        [
            "below: Candidate Enter source at 20,10 prefers Move",
            "above: Candidate Leave source at 20,10 prefers Move",
        ]
    );
}

#[test]
fn a_failing_observer_does_not_keep_the_others_from_being_told() {
    let mut stage = Stage::new(
        |s| s,
        |journal| {
            vec![Element::new(
                "outer",
                Target::new("outer", Answer::Reject, journal).children(vec![Element::new(
                    "inner",
                    Target::new("inner", Answer::FailLeave, journal)
                        .size(size(80, 50))
                        .children(vec![Element::new(
                            "row",
                            Target::new("row", Answer::Accept, journal).size(size(40, 30)),
                        )]),
                )]),
            )]
        },
    );
    stage.start();
    stage.moved(50.0, 10.0);
    // `inner` would accept, but `row` is asked first and claims the drag, so
    // `inner` and `outer` observe.
    assert_eq!(
        stage.journal.take(),
        [
            "row: Candidate Enter source at 10,10 prefers Move",
            "inner: Observer Over source at 10,10 prefers Move",
            "outer: Observer Over source at 10,10 prefers Move",
        ]
    );
    let error = stage
        .ui
        .pointer_input(PointerEvent::new(point(300.0, 150.0), PointerKind::Move))
        .unwrap_err();
    assert_eq!(error, RuntimeError::Hidden);
    assert_eq!(
        stage.journal.take(),
        [
            "row: Candidate Leave source at 260,150 prefers Move",
            "inner: Observer Leave source at 260,150 prefers Move",
            // `inner` failed; `outer` is still told, and nobody twice.
            "outer: Observer Leave source at 260,150 prefers Move",
            "source: finished Cancelled",
        ]
    );
    assert!(!stage.ui.drag_active());
}

#[test]
fn a_request_whose_press_ends_before_adoption_tells_its_source() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    let source = stage.child(0);
    stage.press(10.0, 10.0);
    stage.journal.take();
    stage
        .ui
        .event(&source, |cx| {
            cx.start_drag(
                DragPayload::new(Item("never")),
                DropOperations::MOVE,
                DragPreview::none(),
            )
        })
        .unwrap()
        .unwrap();
    // The button goes up before any flush could adopt the request.
    stage.release(10.0, 10.0);
    assert!(!stage.ui.drag_active());
    assert_eq!(
        stage.journal.take(),
        ["source: clicked", "source: finished Cancelled"]
    );
    assert_eq!(stage.ui.next_deadline(), None);
    // The flags are clear: an ordinary drag starts afterwards.
    stage.start();
}

#[test]
fn a_modal_surface_that_blocks_the_source_cancels_the_session() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    let dialog = DialogController::new(&stage.ui);
    let journal = stage.journal.clone();
    stage
        .ui
        .reconcile_children(
            &stage.root,
            vec![
                Element::new("source", Source::new("source", &journal)),
                Element::new("target", Target::new("target", Answer::Accept, &journal)),
                Element::new(
                    "dialog",
                    Dialog::new(
                        dialog.clone(),
                        Element::new("anchor", Label::new("Anchor")),
                        Element::new("content", Label::new("Blocking")),
                    ),
                ),
            ],
        )
        .unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    stage.journal.take();
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    dialog.open().unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    assert!(!stage.ui.drag_active());
    assert_eq!(
        stage.journal.take(),
        [
            "target: Candidate Leave source at 20,10 prefers Move",
            "source: finished Cancelled",
        ]
    );
}

/// Tabs whose first view holds a dialog and whose second holds a source and a
/// target; returns the root, the two views and the dialog's controller.
fn two_views(ui: &mut Runtime, journal: &Rc<Journal>) -> (WidgetId, WidgetId, DialogController) {
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let dialog = DialogController::new(ui);
    let root = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([
                Tab::new(
                    "modal",
                    "Modal",
                    Element::new(
                        "dialog",
                        Dialog::new(
                            dialog.clone(),
                            Element::new("anchor", Label::new("Anchor")),
                            Element::new("content", Label::new("Blocking")),
                        ),
                    ),
                ),
                Tab::new(
                    "drag",
                    "Drag",
                    Element::new(
                        "row",
                        Row::new([
                            Element::new("source", Source::new("source", journal).preview()),
                            Element::new("target", Target::new("target", Answer::Accept, journal)),
                        ])
                        .gap(Length::new(0.0).unwrap()),
                    ),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let view = ui.children(&root).unwrap()[0].clone();
    (root, view, dialog)
}
fn bounds_of(ui: &Runtime, key: &str) -> [f32; 4] {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == key)
        .and_then(|c| c.details.box_model.border_box)
        .unwrap_or_else(|| panic!("{key} is not laid out"))
}

#[test]
fn a_modal_surface_in_another_window_does_not_cancel_the_session() {
    let journal = Rc::new(Journal::default());
    let mut ui = Runtime::new();
    let (root, view, dialog) = two_views(&mut ui, &journal);
    let prepared = ui
        .prepare_detach(&view, NativeWindowOptions::new("Modal").unwrap())
        .unwrap();
    let detached = ui.commit_detach(prepared).unwrap();
    ui.select_window(&detached).unwrap();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    dialog.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // Back in the primary window, show the drag view and start a drag there.
    let primary = ui.primary_window();
    ui.select_window(&primary).unwrap();
    ui.focus(&root).unwrap();
    for key in [KeyCode::Home, KeyCode::ArrowRight] {
        ui.key_input(key, KeyAction::Press).unwrap();
        ui.key_input(key, KeyAction::Release).unwrap();
        ui.flush(ui.now()).unwrap();
    }
    let [sx, sy, ..] = bounds_of(&ui, "source");
    let [tx, ty, ..] = bounds_of(&ui, "target");
    for (x, y, kind) in [
        (
            sx + 10.0,
            sy + 10.0,
            PointerKind::Press(PointerButton::Primary),
        ),
        (sx + 16.0, sy + 10.0, PointerKind::Move),
        (tx + 20.0, ty + 10.0, PointerKind::Move),
    ] {
        ui.pointer_input(PointerEvent::new(point(x, y), kind))
            .unwrap();
        ui.flush(ui.now()).unwrap();
    }
    assert!(ui.drag_active());
    // Flushing the other window reads that window's modal surfaces, which say
    // nothing about a source that lives here.
    ui.select_window(&detached).unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(
        ui.drag_active(),
        "a dialog elsewhere is not this drag's business"
    );
    ui.select_window(&primary).unwrap();
    ui.pointer_input(PointerEvent::new(
        point(tx + 20.0, ty + 10.0),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    assert!(
        journal
            .take()
            .contains(&"source: finished Dropped(Move)".to_owned())
    );
}

#[test]
fn redocking_the_window_a_session_lives_in_ends_the_session_first() {
    let journal = Rc::new(Journal::default());
    let mut ui = Runtime::new();
    let (root, _, _) = two_views(&mut ui, &journal);
    ui.focus(&root).unwrap();
    for key in [KeyCode::Home, KeyCode::ArrowRight] {
        ui.key_input(key, KeyAction::Press).unwrap();
        ui.key_input(key, KeyAction::Release).unwrap();
        ui.flush(ui.now()).unwrap();
    }
    let view = ui.children(&root).unwrap()[1].clone();
    let prepared = ui
        .prepare_detach(&view, NativeWindowOptions::new("Drag").unwrap())
        .unwrap();
    let detached = ui.commit_detach(prepared).unwrap();
    ui.select_window(&detached).unwrap();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.flush(ui.now()).unwrap();
    let [sx, sy, ..] = bounds_of(&ui, "source");
    for (dx, kind) in [
        (10.0, PointerKind::Press(PointerButton::Primary)),
        (16.0, PointerKind::Move),
    ] {
        ui.pointer_input(PointerEvent::new(point(sx + dx, sy + 10.0), kind))
            .unwrap();
    }
    assert!(ui.drag_active());
    journal.take();
    ui.redock(&detached).unwrap();
    assert!(!ui.drag_active());
    assert!(
        journal
            .take()
            .contains(&"source: finished Cancelled".to_owned())
    );
    ui.flush(ui.now()).unwrap();
    assert!(
        ui.inspect().components.iter().all(|c| c.key != "chip"),
        "the preview went with the window it was mounted in"
    );
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn a_source_may_be_its_own_target() {
    let mut stage = Stage::new(
        |s| s.permitted(DropOperations::LINK),
        one_target(Answer::Reject),
    );
    stage.press(10.0, 10.0);
    stage.moved(16.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        ["source: started", "source: own Enter"]
    );
    stage.release(16.0, 10.0);
    assert_eq!(
        stage.journal.take(),
        ["source: own Drop", "source: finished Dropped(Link)"]
    );
}

#[test]
fn only_one_session_exists_and_it_needs_a_held_button() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    let source = stage.child(0);
    let refused = stage
        .ui
        .event(&source, |cx| {
            cx.start_drag(
                DragPayload::new(Item("loose")),
                DropOperations::MOVE,
                DragPreview::none(),
            )
        })
        .unwrap();
    assert_eq!(refused, Err(RuntimeError::CaptureUnavailable));
    stage.start();
    let second = stage
        .ui
        .event(&source, |cx| {
            assert!(cx.drag_active());
            cx.start_drag(
                DragPayload::new(Item("second")),
                DropOperations::MOVE,
                DragPreview::none(),
            )
        })
        .unwrap();
    assert_eq!(second, Err(RuntimeError::DragUnavailable));
    let empty = stage
        .ui
        .event(&source, |cx| {
            cx.start_drag(
                DragPayload::new(Item("none")),
                DropOperations::NONE,
                DragPreview::none(),
            )
        })
        .unwrap();
    assert_eq!(empty, Err(RuntimeError::DragUnavailable));
}

#[test]
fn the_preview_follows_the_pointer_above_everything_and_costs_no_preparation() {
    let mut stage = Stage::new(|s| s.preview(), one_target(Answer::Accept));
    stage.press(10.0, 10.0);
    stage.moved(16.0, 10.0);
    stage.ui.paint().unwrap();
    assert_eq!(
        stage.journal.take(),
        ["source: started", "preview: update", "preview: paint"]
    );
    // Over the target: the preview covers the pointer and is not hit itself.
    stage.moved(60.0, 10.0);
    let scene = stage.ui.paint().unwrap();
    let DrawCommand::FillRect { rect, .. } = *scene.commands().last().unwrap() else {
        panic!("the preview is composed last");
    };
    assert_eq!(
        rect.values(),
        [55.0, 5.0, 30.0, 20.0],
        "pointer less the hot spot"
    );
    assert_eq!(
        stage.journal.take(),
        ["target: Candidate Enter source at 20,10 prefers Move"],
        "moving it prepared nothing again"
    );
    let report = {
        stage
            .ui
            .pointer_input(PointerEvent::new(point(70.0, 30.0), PointerKind::Move))
            .unwrap();
        stage.ui.flush(stage.ui.now()).unwrap()
    };
    assert_eq!(report.components_updated, 0);
    stage.ui.paint().unwrap();
    assert_eq!(
        stage.journal.take(),
        ["target: Candidate Over source at 30,30 prefers Move"],
        "and a preview-only layout does not deliver the position twice"
    );
    let snapshot = stage.ui.inspect();
    let preview = snapshot.drag.as_ref().unwrap().preview.unwrap();
    assert!(snapshot.components.iter().any(|c| c.id == preview));
    stage.release(70.0, 30.0);
    let after = stage.ui.inspect();
    assert!(after.drag.is_none());
    assert!(
        after
            .components
            .iter()
            .all(|c| c.id != preview && c.key != "chip"),
        "the preview is unmounted with the session"
    );
    assert_eq!(stage.ui.next_deadline(), None);
}

#[test]
fn the_wheel_still_scrolls_what_is_under_the_pointer() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    stage
        .pointer(60.0, 10.0, PointerKind::Scroll(point(0.0, 30.0)))
        .unwrap();
    assert_eq!(
        stage.journal.take(),
        [
            "target: scrolled",
            "target: Candidate Over source at 20,10 prefers Move",
        ]
    );
}

#[test]
fn inspection_and_history_describe_the_session_without_its_payload() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.ui.set_debug_mode(DebugMode::Inspect);
    stage.start();
    stage.moved(60.0, 10.0);
    let snapshot = stage.ui.inspect();
    let drag = snapshot.drag.expect("an active session");
    let id = |key: &str| {
        snapshot
            .components
            .iter()
            .find(|c| c.key == key)
            .unwrap()
            .id
    };
    assert_eq!(drag.source, id("source"));
    assert_eq!(drag.target, Some(id("target")));
    assert_eq!(drag.accepted, Some(DropOperation::Move));
    assert_eq!(drag.permitted, DropOperations::MOVE | DropOperations::COPY);
    assert!(drag.payload_type.ends_with("Item"));
    stage.release(60.0, 10.0);
    let events = stage.ui.inspect().events;
    let drags: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::DragStart | EventKind::DragEnd))
        .map(|e| (e.kind, e.status.clone()))
        .collect();
    assert_eq!(
        drags,
        [
            (EventKind::DragStart, EventStatus::Succeeded),
            (EventKind::DragEnd, EventStatus::Succeeded),
        ]
    );
}

#[test]
fn a_cancelled_session_is_recorded_as_cancelled() {
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.ui.set_debug_mode(DebugMode::Inspect);
    stage.start();
    stage
        .ui
        .key_input(KeyCode::Escape, KeyAction::Press)
        .unwrap();
    let events = stage.ui.inspect().events;
    assert!(
        events
            .iter()
            .any(|e| e.kind == EventKind::DragEnd && e.status == EventStatus::Cancelled)
    );
}

#[test]
fn workspace_restore_is_refused_while_something_is_being_dragged() {
    let journal = Rc::new(Journal::default());
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new("source", Source::new("source", &journal)),
            Element::new(
                "tabs",
                Tabs::new([
                    Tab::new("a", "A", Element::new("a-body", Label::new("A"))),
                    Tab::new("b", "B", Element::new("b-body", Label::new("B"))),
                ]),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let mut saved = ui.capture_workspace().unwrap();
    let zoom = UiZoom::new(1.5).unwrap();
    saved
        .set_settings(
            WorkspaceSettings::default()
                .with_zoom(Some(zoom))
                .with_theme_mode(Some(ThemeMode::Light))
                .with_density(Some(Density::Compact)),
        )
        .unwrap();
    for (x, kind) in [
        (10.0, PointerKind::Press(PointerButton::Primary)),
        (16.0, PointerKind::Move),
    ] {
        ui.pointer_input(PointerEvent::new(point(x, 10.0), kind))
            .unwrap();
    }
    assert!(ui.drag_active());
    let report = ui.restore_workspace(&saved).unwrap();
    assert!(
        report
            .rejected()
            .all(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    assert!(report.rejected().count() > 0);
    // The entries wait for the drag to end; the settings do not.
    assert_eq!(ui.ui_settings().zoom().unwrap(), zoom);
    assert_eq!(ui.theme().mode().unwrap(), ThemeMode::Light);
    assert_eq!(ui.theme().density().unwrap(), Density::Compact);
    assert!(ui.drag_active());
}

#[test]
fn the_native_key_path_cancels_with_escape_and_follows_modifiers() {
    let native = |key: KeyCode| CommandKeyInput::new(key, KeyAction::Press, CommandKey::Named(key));
    // Escape through the entry point the native `App` uses.
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let dispatch = stage.ui.command_key_input(native(KeyCode::Escape)).unwrap();
    assert!(dispatch.handled);
    assert_eq!(
        stage.journal.take(),
        [
            "target: Candidate Leave source at 20,10 prefers Move",
            "source: finished Cancelled",
        ],
    );
    assert!(!stage.ui.drag_active());
    stage.release(60.0, 10.0);
    assert_eq!(stage.journal.take(), Vec::<String>::new());

    // Modifiers reported by the OS re-resolve the operation under a still pointer.
    let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
    stage.start();
    stage.moved(60.0, 10.0);
    stage.journal.take();
    let copy = if cfg!(target_os = "macos") {
        Modifiers {
            alt: true,
            ..Modifiers::default()
        }
    } else {
        Modifiers {
            control: true,
            ..Modifiers::default()
        }
    };
    stage.ui.native_modifiers(copy).unwrap();
    assert_eq!(
        stage.journal.take(),
        ["target: Candidate Over source at 20,10 prefers Copy"]
    );
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::Copy);
    stage.release(60.0, 10.0);
    assert_eq!(stage.journal.take()[1], "source: finished Dropped(Copy)");
}

#[test]
fn shutdown_delivers_terminal_drag_outcomes_before_unmounting() {
    for pending in [false, true] {
        let mut stage = Stage::new(|s| s, one_target(Answer::Accept));
        if pending {
            let source = stage.child(0);
            stage.press(10.0, 10.0);
            stage
                .ui
                .event(&source, |cx| {
                    cx.start_drag(
                        DragPayload::new(Item("pending")),
                        DropOperations::MOVE,
                        DragPreview::none(),
                    )
                })
                .unwrap()
                .unwrap();
        } else {
            stage.start();
            stage.moved(60.0, 10.0);
        }
        stage.journal.take();
        stage.ui.shutdown();
        let events = stage.journal.take();
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "source: finished Cancelled")
                .count(),
            1
        );
        if !pending {
            assert!(events[0].contains("Candidate Leave"));
            assert_eq!(events[1], "source: finished Cancelled");
        }
        stage.ui.shutdown();
        assert!(stage.journal.take().is_empty());
    }
}

#[test]
fn shutdown_retains_typed_cleanup_failure_without_skipping_other_participants() {
    let mut stage = Stage::new(
        |mut source| {
            source.finish = Some(Box::new(|_| Err(RuntimeError::Exhausted)));
            source
        },
        one_target(Answer::FailLeave),
    );
    stage.start();
    stage.moved(60.0, 10.0);
    assert!(stage.ui.take_shutdown_errors().is_empty());
    stage.journal.take();
    stage.ui.shutdown();
    let events = stage.journal.take();
    assert!(events[0].contains("Candidate Leave"));
    assert_eq!(events[1], "source: finished Cancelled");
    assert_eq!(stage.ui.take_shutdown_errors(), [RuntimeError::Hidden]);
    assert!(stage.ui.take_shutdown_errors().is_empty());
    assert!(matches!(
        stage.ui.flush(stage.ui.now()),
        Err(RuntimeError::Closed)
    ));
}

#[test]
fn shutdown_terminal_callback_cannot_admit_a_replacement_drag() {
    for pending in [false, true] {
        let error = Rc::new(Cell::new(None));
        let recorded = error.clone();
        let mut stage = Stage::new(
            move |mut source| {
                source.finish = Some(Box::new(move |cx| {
                    cx.capture_pointer()?;
                    recorded.set(
                        cx.start_drag(
                            DragPayload::new(Item("replacement")),
                            DropOperations::MOVE,
                            DragPreview::none(),
                        )
                        .err(),
                    );
                    Ok(())
                }));
                source
            },
            one_target(Answer::Accept),
        );
        if pending {
            stage.press(10.0, 10.0);
            stage
                .ui
                .event(&stage.child(0), |cx| {
                    cx.start_drag(
                        DragPayload::new(Item("pending")),
                        DropOperations::MOVE,
                        DragPreview::none(),
                    )
                })
                .unwrap()
                .unwrap();
        } else {
            stage.start();
        }
        stage.ui.shutdown();
        assert_eq!(error.take(), Some(RuntimeError::Closed));
        assert!(stage.ui.take_shutdown_errors().is_empty());
    }
}

#[test]
fn shutdown_retains_drag_failures_when_its_native_window_is_retired() {
    for parked in [false, true] {
        let mut stage = Stage::new(
            |mut source| {
                source.finish = Some(Box::new(|_| Err(RuntimeError::Exhausted)));
                source
            },
            one_target(Answer::Accept),
        );
        let primary = stage.ui.primary_window();
        let prepared = stage
            .ui
            .prepare_detach(&stage.root, NativeWindowOptions::new("Detached").unwrap())
            .unwrap();
        let detached = stage.ui.commit_detach(prepared).unwrap();
        stage
            .ui
            .set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
            .unwrap();
        stage.ui.flush(stage.ui.now()).unwrap();
        stage.start();
        stage.moved(60.0, 10.0);
        if parked {
            stage.ui.select_window(&primary).unwrap();
        }
        stage.journal.take();
        stage.ui.shutdown();
        let events = stage.journal.take();
        assert!(events[0].contains("Candidate Leave"));
        assert_eq!(events[1], "source: finished Cancelled");
        assert!(!stage.ui.contains_window(&detached));
        assert_eq!(stage.ui.take_shutdown_errors(), [RuntimeError::Exhausted]);
        assert!(stage.ui.take_shutdown_errors().is_empty());
    }
}

#[test]
fn shutdown_keeps_errors_queued_before_closing_and_notifies_observers() {
    for queued_before in [false, true] {
        let mut stage = Stage::new(
            |s| s.preview(),
            |journal| {
                vec![Element::new(
                    "outer",
                    Target::new("outer", Answer::Reject, journal).children(vec![Element::new(
                        "inner",
                        Target::new("inner", Answer::FailLeave, journal)
                            .size(size(80, 50))
                            .children(vec![Element::new(
                                "row",
                                Target::new("row", Answer::Accept, journal).size(size(40, 30)),
                            )]),
                    )]),
                )]
            },
        );
        stage.start();
        stage.moved(50.0, 10.0);
        assert!(
            stage
                .ui
                .inspect()
                .components
                .iter()
                .any(|c| c.key == "chip")
        );
        stage.journal.take();
        if queued_before {
            stage.ui.clear_focus();
        } else {
            stage.ui.shutdown();
        }
        let events = stage.journal.take();
        assert!(events[0].contains("row: Candidate Leave"));
        assert!(events[1].contains("inner: Observer Leave"));
        assert!(events[2].contains("outer: Observer Leave"));
        assert_eq!(events[3], "source: finished Cancelled");
        assert!(
            stage
                .ui
                .inspect()
                .components
                .iter()
                .all(|c| c.key != "chip")
        );
        if queued_before {
            assert!(stage.ui.take_shutdown_errors().is_empty());
            stage.ui.shutdown();
        }
        assert_eq!(stage.ui.take_shutdown_errors(), [RuntimeError::Hidden]);
        assert!(stage.journal.take().is_empty());
    }
}
