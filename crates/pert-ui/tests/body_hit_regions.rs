//! Public body hit regions preserve capture, children, foreground and drag routing.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
struct Layer(Vec<Element>);
impl Component for Layer {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.0)
    }
    fn measure(&self, _: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            a.width().unwrap_or(120.0),
            a.height().unwrap_or(80.0),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, Rect::from_size(size))?;
        }
        Ok(())
    }
}
#[derive(Default)]
struct Seen {
    presses: Cell<usize>,
    releases: Cell<usize>,
    moves: Cell<usize>,
    drops: Cell<usize>,
    wheels: Cell<usize>,
}
struct Region {
    seen: Rc<Seen>,
    holes: bool,
    capture: bool,
    dragging: bool,
    children: Vec<Element>,
    writes: Option<Signal<bool>>,
    refused: Rc<RefCell<Option<RuntimeError>>>,
}
impl Component for Region {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(120.0, 80.0)?)
    }
    fn hit_test(&self, point: Point) -> bool {
        if let Some(signal) = &self.writes {
            *self.refused.borrow_mut() = signal.set(true).err();
        }
        !self.holes || point.x() < 20.0
    }
    fn hit_test_foreground(&self, point: Point) -> bool {
        self.holes && (40.0..60.0).contains(&point.x())
    }
    fn paint_foreground(&self, p: &mut Painter) -> Result<(), DrawError> {
        if self.holes {
            p.fill_rect(Rect::new(40.0, 0.0, 20.0, 80.0)?, Color::ACCENT);
        }
        Ok(())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            cx.place(0, Rect::new(80.0, 0.0, 20.0, 30.0)?)?;
        }
        Ok(())
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        Some(if self.holes {
            CursorIcon::Pointer
        } else {
            CursorIcon::Text
        })
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match e.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                self.seen.presses.set(self.seen.presses.get() + 1);
                if self.dragging {
                    cx.start_drag(
                        DragPayload::new(()),
                        DropOperations::MOVE,
                        DragPreview::none(),
                    )?;
                } else if self.capture {
                    cx.capture_pointer()?;
                }
            }
            PointerKind::Release(PointerButton::Primary) => {
                self.seen.releases.set(self.seen.releases.get() + 1);
                cx.release_pointer();
            }
            PointerKind::Move => self.seen.moves.set(self.seen.moves.get() + 1),
            PointerKind::Scroll(_) => self.seen.wheels.set(self.seen.wheels.get() + 1),
            _ => {}
        }
        Ok(InputOutcome::Handled)
    }
    fn drag_event(
        &mut self,
        _: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        if event.phase() == DragPhase::Drop {
            self.seen.drops.set(self.seen.drops.get() + 1);
        }
        Ok(DropResponse::Accepted(DropOperation::Move))
    }
}
fn region(seen: &Rc<Seen>, holes: bool, capture: bool) -> Region {
    Region {
        seen: seen.clone(),
        holes,
        capture,
        dragging: false,
        children: vec![],
        writes: None,
        refused: Rc::default(),
    }
}
fn send(ui: &mut Runtime, x: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, 10.0).unwrap(), kind))
        .unwrap();
}
#[test]
fn body_holes_keep_children_and_foreground_independent_and_capture_unrestricted() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(120.0, 80.0, 1.5).unwrap())
        .unwrap();
    let lower = Rc::new(Seen::default());
    let upper = Rc::new(Seen::default());
    let child = Rc::new(Seen::default());
    let mut top = region(&upper, true, true);
    top.children
        .push(Element::new("child", region(&child, false, false)));
    let root = ui
        .mount_tree(Element::new(
            "layers",
            Layer(vec![
                Element::new("lower", region(&lower, false, false)),
                Element::new("upper", top),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    for (x, expected) in [(30.0, "lower"), (50.0, "upper"), (90.0, "child")] {
        send(&mut ui, x, PointerKind::Move);
        assert_eq!(
            ui.cursor_icon(),
            if expected == "upper" {
                CursorIcon::Pointer
            } else {
                CursorIcon::Text
            }
        );
        send(&mut ui, x, PointerKind::Press(PointerButton::Primary));
        assert_eq!(ui.focused(), ui.find_descendant(&root, expected).unwrap());
        send(&mut ui, x, PointerKind::Release(PointerButton::Primary));
    }
    assert_eq!(lower.presses.get(), 1);
    assert_eq!(upper.presses.get(), 1);
    assert_eq!(child.presses.get(), 1);
    send(&mut ui, 10.0, PointerKind::Press(PointerButton::Primary));
    let before = upper.moves.get();
    send(&mut ui, 30.0, PointerKind::Move);
    assert_eq!(upper.moves.get(), before + 1);
    assert_eq!(ui.cursor_icon(), CursorIcon::Pointer);
    send(&mut ui, 30.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(upper.releases.get(), 2);
    assert_eq!(lower.releases.get(), 1);
    send(&mut ui, 30.0, PointerKind::Move);
    assert_eq!(ui.cursor_icon(), CursorIcon::Text);
}
#[test]
fn body_query_runs_read_only_and_modal_holes_do_not_expose_blocked_content() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(false);
    let lower = Rc::new(Seen::default());
    let upper = Rc::new(Seen::default());
    let mut top = region(&upper, true, false);
    let refused = top.refused.clone();
    top.writes = Some(value.clone());
    let dialog = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "layers",
            Layer(vec![
                Element::new("lower", region(&lower, false, false)),
                Element::new(
                    "dialog",
                    Dialog::new(
                        dialog.clone(),
                        Element::new("anchor", Label::new("Open")),
                        Element::new("upper", top),
                    ),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    dialog.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let upper_id = ui.find_descendant(&root, "upper").unwrap().unwrap();
    let bounds = ui.bounds(&upper_id).unwrap().unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(bounds.x() + 30.0, bounds.y() + 10.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    assert_eq!(lower.presses.get(), 0);
    assert!(!value.get().unwrap());
    assert_eq!(upper.presses.get(), 0);
    ui.unmount(&root).unwrap();
    let mut top = region(&upper, true, false);
    top.writes = Some(value.clone());
    top.refused = refused.clone();
    ui.mount_tree(Element::new(
        "root",
        Layer(vec![Element::new("upper", top)]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    send(&mut ui, 10.0, PointerKind::Move);
    assert_eq!(*refused.borrow(), Some(RuntimeError::WriteDuringInspection));
    assert!(!value.get().unwrap());
}
#[test]
fn drag_target_and_drag_wheel_resolve_through_body_holes() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(120.0, 80.0, 1.0).unwrap())
        .unwrap();
    let lower = Rc::new(Seen::default());
    let upper = Rc::new(Seen::default());
    let mut top = region(&upper, true, false);
    top.dragging = true;
    ui.mount_tree(Element::new(
        "layers",
        Layer(vec![
            Element::new("lower", region(&lower, false, false)),
            Element::new("upper", top),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    send(&mut ui, 10.0, PointerKind::Press(PointerButton::Primary));
    send(&mut ui, 30.0, PointerKind::Move);
    send(
        &mut ui,
        30.0,
        PointerKind::Scroll(Point::new(0.0, 20.0).unwrap()),
    );
    send(&mut ui, 30.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(lower.drops.get(), 1);
    assert_eq!(upper.drops.get(), 0);
    assert_eq!(lower.wheels.get(), 1);
    assert_eq!(upper.wheels.get(), 0);
}
