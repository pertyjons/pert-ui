//! Overflowing tool stripes retain reachable, disjoint buttons.
use pert_ui::*;
use std::time::Duration;

fn mount(
    edge: ToolEdge,
    width: f32,
    height: f32,
) -> (Runtime, WidgetId, ToolWindowController, Signal<bool>) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0).unwrap())
        .unwrap();
    let controller = ToolWindowController::new();
    let allowed = ui.signal(true);
    let mut layout = ToolWindowLayout::new(Element::new("main", Label::new("main")))
        .controller(&controller)
        .toggles(allowed.read_only())
        .edge(edge, EdgeMode::Stripe)
        .show_names(true);
    for (key, group) in [
        ("a", StripeGroup::Start),
        ("b", StripeGroup::Start),
        ("c", StripeGroup::End),
        ("d", StripeGroup::End),
    ] {
        layout = layout.window(
            ToolWindow::new(
                key,
                format!("A long title for {key}"),
                Element::new(format!("body-{key}"), Label::new(key)),
            )
            .edge(edge)
            .group(group),
        );
    }
    let root = ui.mount_tree(Element::new("tools", layout)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    (ui, root, controller, allowed)
}
fn stripe(ui: &Runtime, root: &WidgetId, edge: ToolEdge) -> WidgetId {
    ui.find_descendant(
        root,
        if edge == ToolEdge::Left {
            "stripe-left"
        } else {
            "stripe-top"
        },
    )
    .unwrap()
    .unwrap()
}
fn key(ui: &mut Runtime, code: KeyCode) {
    ui.key_input(code, KeyAction::Press).unwrap();
    ui.key_input(code, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
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
fn keyboard_end_reveals_end_group_for_pointer_activation() {
    for (edge, width, height) in [(ToolEdge::Left, 400.0, 64.0), (ToolEdge::Top, 96.0, 300.0)] {
        let (mut ui, root, controller, _) = mount(edge, width, height);
        let stripe = stripe(&ui, &root, edge);
        ui.focus(&stripe).unwrap();
        key(&mut ui, KeyCode::End);
        let rect = ui.bounds(&stripe).unwrap().unwrap();
        // Oversized labels reveal their leading edge; ordinary buttons fit entirely.
        let point = if edge == ToolEdge::Left {
            Point::new(rect.x() + 10.0, rect.y() + rect.height() - 10.0).unwrap()
        } else {
            Point::new(rect.x() + 10.0, rect.y() + 10.0).unwrap()
        };
        click(&mut ui, point);
        assert!(
            controller.is_expanded("d").unwrap(),
            "end button must be reachable on {edge:?}"
        );
        assert!(!controller.is_expanded("a").unwrap());
    }
}

fn wheel(ui: &mut Runtime, point: Point, amount: f32) {
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Scroll(Point::new(0.0, amount).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
}
#[test]
fn wheel_reaches_both_groups_and_endpoints_reuse_layout() {
    for (edge, width, height) in [(ToolEdge::Left, 400.0, 64.0), (ToolEdge::Top, 96.0, 300.0)] {
        let (mut ui, root, controller, _) = mount(edge, width, height);
        let id = stripe(&ui, &root, edge);
        let rect = ui.bounds(&id).unwrap().unwrap();
        let point = Point::new(rect.x() + 10.0, rect.y() + 10.0).unwrap();
        ui.set_profiling(true).unwrap();
        for (amount, expected) in [(100_000.0, "d"), (-100_000.0, "a")] {
            wheel(&mut ui, point, amount);
            ui.paint().unwrap();
            ui.clear_profile().unwrap();
            for _ in 0..3 {
                wheel(&mut ui, point, amount);
            }
            ui.paint().unwrap();
            assert!(
                ui.profile()
                    .stats
                    .iter()
                    .filter(|stat| stat.key
                        == if edge == ToolEdge::Left {
                            "stripe-left"
                        } else {
                            "stripe-top"
                        })
                    .all(|stat| !matches!(
                        stat.operation,
                        ProfileOperation::Measure
                            | ProfileOperation::Arrange
                            | ProfileOperation::Paint
                    ))
            );
            let target = if edge == ToolEdge::Left && amount > 0.0 {
                Point::new(point.x(), rect.y() + rect.height() - 10.0).unwrap()
            } else {
                point
            };
            click(&mut ui, target);
            assert!(
                controller.is_expanded(expected).unwrap(),
                "edge {edge:?}, expected {expected}, flags {:?}",
                ["a", "b", "c", "d"].map(|key| controller.is_expanded(key).unwrap())
            );
            controller.collapse(expected).unwrap();
            ui.flush(Duration::ZERO).unwrap();
        }
    }
}
#[test]
fn resizing_a_focused_stripe_keeps_active_button_revealed() {
    let (mut ui, root, controller, _) = mount(ToolEdge::Left, 400.0, 400.0);
    let id = stripe(&ui, &root, ToolEdge::Left);
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::End);
    ui.set_viewport(Viewport::new(400.0, 64.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let rect = ui.bounds(&id).unwrap().unwrap();
    click(
        &mut ui,
        Point::new(rect.x() + 10.0, rect.y() + rect.height() - 10.0).unwrap(),
    );
    assert!(controller.is_expanded("d").unwrap());
}

#[test]
fn repeated_end_after_scrolling_reveals_unchanged_keyboard_selection() {
    let (mut ui, root, controller, _) = mount(ToolEdge::Left, 400.0, 64.0);
    let id = stripe(&ui, &root, ToolEdge::Left);
    let rect = ui.bounds(&id).unwrap().unwrap();
    let point = Point::new(rect.x() + 10.0, rect.y() + 10.0).unwrap();
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::End);
    wheel(&mut ui, point, -100_000.0);
    key(&mut ui, KeyCode::End);
    click(
        &mut ui,
        Point::new(point.x(), rect.y() + rect.height() - 10.0).unwrap(),
    );
    assert!(controller.is_expanded("d").unwrap());
}
#[test]
fn scrolling_cancels_an_armed_pointer_activation() {
    let (mut ui, root, controller, _) = mount(ToolEdge::Left, 400.0, 64.0);
    let id = stripe(&ui, &root, ToolEdge::Left);
    let rect = ui.bounds(&id).unwrap().unwrap();
    let point = Point::new(rect.x() + 10.0, rect.y() + 10.0).unwrap();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    wheel(&mut ui, point, 100_000.0);
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        ["a", "b", "c", "d"]
            .iter()
            .all(|key| !controller.is_expanded(key).unwrap())
    );
}

#[test]
fn overflow_drop_can_move_start_button_into_end_group() {
    let (mut ui, root, controller, _) = mount(ToolEdge::Left, 400.0, 64.0);
    let id = stripe(&ui, &root, ToolEdge::Left);
    let rect = ui.bounds(&id).unwrap().unwrap();
    let x = rect.x() + 10.0;
    wheel(&mut ui, Point::new(x, rect.y() + 10.0).unwrap(), 32.0);
    // B is now at 0..32 and C at 32..64: the groups meet without a gap.
    for (y, kind) in [
        (10.0, PointerKind::Press(PointerButton::Primary)),
        (50.0, PointerKind::Move),
        (54.0, PointerKind::Move),
        (54.0, PointerKind::Release(PointerButton::Primary)),
    ] {
        ui.pointer_input(PointerEvent::new(
            Point::new(x, rect.y() + y).unwrap(),
            kind,
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
    }
    assert_eq!(controller.placement("b").unwrap().1, StripeGroup::End);
}

#[test]
fn controller_reorder_refreshes_reveal_without_remounting_panels() {
    let (mut ui, root, controller, _) = mount(ToolEdge::Left, 400.0, 64.0);
    let before = ui.find_descendant(&root, "body-c").unwrap().unwrap();
    controller.reorder("d", 0).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let id = stripe(&ui, &root, ToolEdge::Left);
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::End);
    let rect = ui.bounds(&id).unwrap().unwrap();
    click(
        &mut ui,
        Point::new(rect.x() + 10.0, rect.y() + rect.height() - 10.0).unwrap(),
    );
    assert!(controller.is_expanded("c").unwrap());
    assert_eq!(ui.find_descendant(&root, "body-c").unwrap(), Some(before));
}

#[test]
fn capability_and_expansion_updates_reuse_stripe_geometry() {
    let (mut ui, _root, controller, allowed) = mount(ToolEdge::Left, 400.0, 64.0);
    ui.set_profiling(true).unwrap();
    ui.paint().unwrap();
    for step in 0..3 {
        ui.clear_profile().unwrap();
        match step {
            0 => {
                allowed.set(false).unwrap();
            }
            1 => {
                allowed.set(true).unwrap();
            }
            _ => {
                controller.expand("d").unwrap();
            }
        }
        ui.flush(Duration::ZERO).unwrap();
        ui.paint().unwrap();
        assert!(
            ui.profile()
                .stats
                .iter()
                .filter(|stat| stat.key == "stripe-left")
                .all(|stat| !matches!(
                    stat.operation,
                    ProfileOperation::Measure | ProfileOperation::Arrange
                )),
            "step {step} must not rebuild unchanged stripe geometry"
        );
    }
}

#[test]
fn drag_wheel_reaches_an_initially_offscreen_group() {
    let (mut ui, root, controller, _) = mount(ToolEdge::Left, 400.0, 64.0);
    let id = stripe(&ui, &root, ToolEdge::Left);
    let rect = ui.bounds(&id).unwrap().unwrap();
    let point = |y| Point::new(rect.x() + 10.0, rect.y() + y).unwrap();
    ui.pointer_input(PointerEvent::new(
        point(10.0),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(point(26.0), PointerKind::Move))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    wheel(&mut ui, point(54.0), 100_000.0);
    ui.pointer_input(PointerEvent::new(
        point(54.0),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(controller.placement("a").unwrap().1, StripeGroup::End);
    assert!(!controller.is_expanded("a").unwrap());
}

#[test]
fn a_drag_resting_near_an_edge_scrolls_the_stripe() {
    let (mut ui, root, controller, _) = mount(ToolEdge::Left, 400.0, 64.0);
    let id = stripe(&ui, &root, ToolEdge::Left);
    let rect = ui.bounds(&id).unwrap().unwrap();
    let point = |y| Point::new(rect.x() + 10.0, rect.y() + y).unwrap();
    let offset = |ui: &Runtime| {
        let key = ui.key(&id).unwrap().to_owned();
        match ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == key)
            .unwrap()
            .details
            .properties["scroll_offset"]
        {
            InspectValue::Number(offset) => offset,
            ref other => panic!("{other:?}"),
        }
    };
    ui.pointer_input(PointerEvent::new(
        point(10.0),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(point(26.0), PointerKind::Move))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // Rest at the far edge; ticks scroll without further motion.
    ui.pointer_input(PointerEvent::new(point(63.0), PointerKind::Move))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(offset(&ui), 0.0);
    let mut now = Duration::ZERO;
    for _ in 0..200 {
        let Some(deadline) = ui.next_deadline() else {
            break;
        };
        now = now.max(deadline);
        ui.flush(now).unwrap();
    }
    assert!(
        offset(&ui) > 0.0,
        "the stripe scrolled under the resting drag"
    );
    assert_eq!(ui.next_deadline(), None, "autoscroll stops at the end");
    ui.pointer_input(PointerEvent::new(
        point(63.0),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    ui.flush(now).unwrap();
    assert_eq!(controller.placement("a").unwrap().1, StripeGroup::End);
}
