//! ToolWindows center focus must not cover retained floating panels.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
struct Body {
    name: &'static str,
    hits: Rc<RefCell<Vec<&'static str>>>,
    color: Color,
    hovered: Rc<std::cell::Cell<bool>>,
}
impl Component for Body {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn hover_changed(&mut self, _: &mut EventContext, hovered: bool) {
        self.hovered.set(hovered);
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            a.width().unwrap_or(800.0),
            a.height().unwrap_or(600.0),
        )?)
    }
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.kind() == PointerKind::Press(PointerButton::Primary) {
            self.hits.borrow_mut().push(self.name);
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::Enter && event.transition == KeyTransition::Released {
            self.hits.borrow_mut().push(self.name);
        }
        Ok(InputOutcome::Handled)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(Rect::from_size(p.size().unwrap_or(Size::ZERO)), self.color);
        Ok(())
    }
}
fn body(name: &'static str, hits: &Rc<RefCell<Vec<&'static str>>>, color: Color) -> Element {
    Element::new(
        name,
        Body {
            name,
            hits: hits.clone(),
            color,
            hovered: Rc::new(std::cell::Cell::new(false)),
        },
    )
}
fn click(ui: &mut Runtime, point: Point) {
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
        ui.flush(Duration::ZERO).unwrap();
    }
}
#[test]
fn center_focus_and_pointer_raise_leave_floating_content_above_it() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let hits = Rc::new(RefCell::new(vec![]));
    let controller = ToolWindowController::new();
    let root = ui
        .mount_tree(Element::new(
            "tools",
            ToolWindowLayout::new(body("main", &hits, Color::WHITE))
                .controller(&controller)
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(
                    ToolWindow::new(
                        "floating",
                        "Floating",
                        body("floating-body", &hits, Color::from_srgb8(0, 0, 0, 255)),
                    )
                    .mode(ToolWindowMode::Floating)
                    .expanded(true),
                ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let main = ui.find_descendant(&root, "main").unwrap().unwrap();
    let floating = ui.find_descendant(&root, "floating-body").unwrap().unwrap();
    let area = ui.bounds(&floating).unwrap().unwrap();
    let point = Point::new(
        area.x() + area.width() / 2.0,
        area.y() + area.height() / 2.0,
    )
    .unwrap();
    ui.focus(&main).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let order: Vec<_> = scene
        .commands()
        .iter()
        .filter_map(|cmd| match cmd {
            DrawCommand::FillRect { color, .. }
                if *color == Color::WHITE || *color == Color::from_srgb8(0, 0, 0, 255) =>
            {
                Some(*color)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        order.last(),
        Some(&Color::from_srgb8(0, 0, 0, 255)),
        "floating body must draw above center"
    );
    click(&mut ui, point);
    assert_eq!(hits.borrow().last(), Some(&"floating-body"));
    assert_eq!(ui.focused(), Some(floating.clone()));
    // Pointer focus on the exposed center must respect the same tier.
    click(&mut ui, Point::new(790.0, 590.0).unwrap());
    assert_eq!(ui.focused(), Some(main.clone()));
    click(&mut ui, point);
    assert_eq!(ui.focused(), Some(floating.clone()));
    ui.focus(&main).unwrap();
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.key_input(KeyCode::Enter, action).unwrap();
    }
    assert_eq!(hits.borrow().last(), Some(&"main"));
    click(&mut ui, point);
    assert_eq!(hits.borrow().last(), Some(&"floating-body"));
    controller
        .set_mode("floating", ToolWindowMode::Docked)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.find_descendant(&root, "floating-body").unwrap(),
        Some(floating.clone())
    );
    controller
        .set_mode("floating", ToolWindowMode::Floating)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.find_descendant(&root, "floating-body").unwrap(),
        Some(floating.clone())
    );
    ui.focus(&main).unwrap();
    click(&mut ui, point);
    assert_eq!(ui.focused(), Some(floating));
}

struct Layers {
    children: Vec<Element>,
    priority: Rc<std::cell::Cell<i32>>,
    stacking: Rc<std::cell::Cell<bool>>,
    attempted_write: Signal<bool>,
    refused: Rc<RefCell<Option<RuntimeError>>>,
}
impl Component for Layers {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn stacks_children(&self) -> bool {
        self.stacking.get()
    }
    fn child_stacking_priority(&self, key: &str) -> i32 {
        // Deliberately invalid write exercises the runtime's read-only guard.
        *self.refused.borrow_mut() = self.attempted_write.set(true).err();
        if key == "front" {
            self.priority.get()
        } else {
            0
        }
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
#[test]
fn custom_stacking_priorities_keep_recency_and_logical_identity_independent() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(120.0, 80.0, 1.0).unwrap())
        .unwrap();
    let hits = Rc::new(RefCell::new(vec![]));
    let priority = Rc::new(std::cell::Cell::new(1));
    let stacking = Rc::new(std::cell::Cell::new(true));
    let front_hover = Rc::new(std::cell::Cell::new(false));
    let base_hover = Rc::new(std::cell::Cell::new(false));
    let write = ui.signal(false);
    let refused = Rc::new(RefCell::new(None));
    let root = ui
        .mount_tree(Element::new(
            "layers",
            Layers {
                children: vec![
                    Element::new(
                        "front",
                        Body {
                            name: "front",
                            hits: hits.clone(),
                            color: Color::WHITE,
                            hovered: front_hover.clone(),
                        },
                    ),
                    Element::new(
                        "base",
                        Body {
                            name: "base",
                            hits: hits.clone(),
                            color: Color::ACCENT,
                            hovered: base_hover.clone(),
                        },
                    ),
                ],
                priority: priority.clone(),
                stacking: stacking.clone(),
                attempted_write: write.clone(),
                refused: refused.clone(),
            },
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let logical = ui.children(&root).unwrap();
    let front = logical[0].clone();
    let base = logical[1].clone();
    let point = Point::new(10.0, 10.0).unwrap();
    // Priority wins before either child has been raised, even in reverse order.
    click(&mut ui, point);
    assert_eq!(hits.borrow().last(), Some(&"front"));
    assert!(!write.get().unwrap());
    assert_eq!(*refused.borrow(), Some(RuntimeError::WriteDuringInspection));
    assert!(front_hover.get());
    assert!(!base_hover.get());
    priority.set(-1);
    ui.invalidate(&root, Invalidation::Layout).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // No new pointer event: layout refreshes the stationary pointer.
    assert!(!front_hover.get());
    assert!(base_hover.get());
    // Equal priorities keep the existing focus raise policy.
    priority.set(0);
    ui.invalidate(&root, Invalidation::Layout).unwrap();
    ui.focus(&base).unwrap();
    click(&mut ui, point);
    assert_eq!(hits.borrow().last(), Some(&"base"));
    ui.focus(&front).unwrap();
    click(&mut ui, point);
    assert_eq!(hits.borrow().last(), Some(&"front"));
    // Full signed priorities are order values, not array positions or sums.
    for (value, expected, focused) in [
        (i32::MIN, "base", &front),
        (-1, "base", &front),
        (i32::MAX, "front", &base),
    ] {
        priority.set(value);
        ui.invalidate(&root, Invalidation::Layout).unwrap();
        ui.focus(focused).unwrap();
        click(&mut ui, point);
        assert_eq!(hits.borrow().last(), Some(&expected));
    }
    // A nonstacking parent uses logical order even if it provides priorities.
    stacking.set(false);
    ui.invalidate(&root, Invalidation::Layout).unwrap();
    click(&mut ui, point);
    assert_eq!(hits.borrow().last(), Some(&"base"));
    assert_eq!(ui.children(&root).unwrap(), logical);
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(ui.focused(), Some(front));
}
#[test]
fn floating_frames_raise_among_peers_after_center_focus_and_mode_changes() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let hits = Rc::new(RefCell::new(vec![]));
    let controller = ToolWindowController::new();
    let root = ui
        .mount_tree(Element::new(
            "tools",
            ToolWindowLayout::new(body("main", &hits, Color::WHITE))
                .controller(&controller)
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(
                    ToolWindow::new("a", "A", body("a-body", &hits, Color::ACCENT))
                        .mode(ToolWindowMode::Floating)
                        .expanded(true),
                )
                .window(
                    ToolWindow::new("b", "B", body("b-body", &hits, Color::SURFACE))
                        .mode(ToolWindowMode::Floating)
                        .expanded(true),
                ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let main = ui.find_descendant(&root, "main").unwrap().unwrap();
    let a = ui.find_descendant(&root, "a-body").unwrap().unwrap();
    let b = ui.find_descendant(&root, "b-body").unwrap().unwrap();
    let rect = ui.bounds(&a).unwrap().unwrap();
    let point = Point::new(
        rect.x() + rect.width() / 2.0,
        rect.y() + rect.height() / 2.0,
    )
    .unwrap();
    for (peer, expected) in [(&a, "a-body"), (&b, "b-body"), (&a, "a-body")] {
        ui.focus(peer).unwrap();
        ui.focus(&main).unwrap();
        click(&mut ui, point);
        assert_eq!(hits.borrow().last(), Some(&expected));
    }
    controller.collapse("a").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&main).unwrap();
    click(&mut ui, point);
    assert_eq!(hits.borrow().last(), Some(&"b-body"));
    controller.expand("a").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.find_descendant(&root, "a-body").unwrap(), Some(a));
    assert_eq!(ui.find_descendant(&root, "b-body").unwrap(), Some(b));
}
