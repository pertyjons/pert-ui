//! Toast identity, scheduling and transparent notification gaps through the facade.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

struct Backdrop {
    presses: Rc<Cell<usize>>,
    wheels: Rc<Cell<usize>>,
}
impl Component for Backdrop {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
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
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match e.kind() {
            PointerKind::Press(PointerButton::Primary) => self.presses.set(self.presses.get() + 1),
            PointerKind::Scroll(_) => self.wheels.set(self.wheels.get() + 1),
            _ => {}
        }
        Ok(InputOutcome::Handled)
    }
}
struct Stage {
    ui: Runtime,
    receipts: Vec<ToastReceipt>,
    chosen: Rc<Cell<usize>>,
    presses: Rc<Cell<usize>>,
    wheels: Rc<Cell<usize>>,
    stack: WidgetId,
    queue: ToastQueue,
}
fn fixture(count: usize) -> Stage {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let queue = ToastQueue::new(&ui);
    let chosen = Rc::new(Cell::new(usize::MAX));
    let receipts = (0..count)
        .map(|index| {
            let chosen = chosen.clone();
            queue
                .push(Toast::new("Saved").action("Apply", move |_| {
                    chosen.set(index);
                    Ok(())
                }))
                .unwrap()
        })
        .collect();
    let presses = Rc::new(Cell::new(0));
    let wheels = Rc::new(Cell::new(0));
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(
                queue.clone(),
                Element::new(
                    "backdrop",
                    Backdrop {
                        presses: presses.clone(),
                        wheels: wheels.clone(),
                    },
                ),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let stack = ui.find_descendant(&root, "toasts").unwrap().unwrap();
    Stage {
        ui,
        receipts,
        chosen,
        presses,
        wheels,
        stack,
        queue,
    }
}
fn key(ui: &mut Runtime, code: KeyCode) {
    ui.key_input(code, KeyAction::Press).unwrap();
    ui.key_input(code, KeyAction::Release).unwrap();
}
fn dismiss(ui: &mut Runtime, stack: &WidgetId, count: usize, index: usize) {
    let bounds = ui.bounds(stack).unwrap().unwrap();
    let metrics = ui.theme().metrics(ui.theme().density().unwrap()).unwrap();
    let gap = metrics.get(MetricRole::GroupPadding);
    let height = (bounds.height() - gap * count.saturating_sub(1) as f32) / count as f32;
    let point = Point::new(
        bounds.x() + bounds.width() - metrics.get(MetricRole::ControlPaddingX) - 10.0,
        bounds.y() + index as f32 * (height + gap) + height
            - metrics.get(MetricRole::ControlPaddingX)
            - metrics.get(MetricRole::ControlHeight) / 2.0,
    )
    .unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
    }
}
#[test]
fn earlier_pointer_dismissal_keeps_keyboard_selection_on_the_same_toast() {
    for terminal in [KeyCode::Enter, KeyCode::Delete] {
        let Stage {
            mut ui,
            receipts,
            chosen,
            stack,
            ..
        } = fixture(3);
        ui.focus(&stack).unwrap();
        key(&mut ui, KeyCode::ArrowDown);
        dismiss(&mut ui, &stack, 3, 0);
        assert_eq!(receipts[0].status(), ToastStatus::Dismissed);
        key(&mut ui, terminal);
        if terminal == KeyCode::Enter {
            assert_eq!(chosen.get(), 1);
            assert_eq!(receipts[1].status(), ToastStatus::ActionSucceeded);
        } else {
            assert_eq!(chosen.get(), usize::MAX);
            assert_eq!(receipts[1].status(), ToastStatus::Dismissed);
        }
        assert_eq!(receipts[2].status(), ToastStatus::Visible);
    }
}
#[test]
fn removing_selected_toast_chooses_following_neighbor_then_previous_at_end() {
    for index in [1, 2] {
        let Stage {
            mut ui,
            receipts,
            chosen,
            stack,
            ..
        } = fixture(3);
        ui.focus(&stack).unwrap();
        for _ in 0..index {
            key(&mut ui, KeyCode::ArrowDown);
        }
        dismiss(&mut ui, &stack, 3, index);
        assert_eq!(receipts[index].status(), ToastStatus::Dismissed);
        key(&mut ui, KeyCode::Enter);
        assert_eq!(chosen.get(), if index == 1 { 2 } else { 1 });
    }
}
#[test]
fn unchanged_hover_and_fitting_wheel_leave_toast_layout_and_paint_cached() {
    let Stage {
        mut ui,
        wheels,
        stack,
        ..
    } = fixture(1);
    let bounds = ui.bounds(&stack).unwrap().unwrap();
    let point = Point::new(bounds.x() + 5.0, bounds.y() + 5.0).unwrap();
    ui.set_profiling(true).unwrap();
    ui.pointer_input(PointerEvent::new(point, PointerKind::Move))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    ui.clear_profile().unwrap();
    for _ in 0..3 {
        ui.pointer_input(PointerEvent::new(point, PointerKind::Move))
            .unwrap();
    }
    for delta in [-30.0, 30.0] {
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Scroll(Point::new(0.0, delta).unwrap()),
        ))
        .unwrap();
    }
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    assert!(
        ui.profile()
            .stats
            .iter()
            .filter(|s| s.key == "toasts")
            .all(|s| !matches!(
                s.operation,
                ProfileOperation::Measure | ProfileOperation::Arrange | ProfileOperation::Paint
            )),
        "{:?}",
        ui.profile()
    );
    // Scroll yielding follows ancestor routing; underlying siblings only receive gap hits.
    assert_eq!(wheels.get(), 0);
}
#[test]
fn transparent_toast_gap_delivers_press_and_wheel_to_the_backdrop() {
    let Stage {
        mut ui,
        presses,
        wheels,
        stack,
        ..
    } = fixture(2);
    let bounds = ui.bounds(&stack).unwrap().unwrap();
    let gap = ui
        .theme()
        .metrics(ui.theme().density().unwrap())
        .unwrap()
        .get(MetricRole::GroupPadding);
    let point = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap();
    assert!(gap > 0.0);
    for kind in [
        PointerKind::Move,
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
        PointerKind::Scroll(Point::new(0.0, 20.0).unwrap()),
    ] {
        ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
    }
    assert_eq!(presses.get(), 1);
    assert_eq!(wheels.get(), 1);
    assert_eq!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "toasts")
            .unwrap()
            .details
            .properties
            .get("paused"),
        Some(&InspectValue::Bool(false))
    );
}

#[test]
fn batched_expiry_prefers_surviving_neighbor_over_a_newly_promoted_toast() {
    let Stage {
        mut ui,
        queue,
        receipts,
        chosen,
        stack,
        ..
    } = fixture(2);
    let outside = ui
        .mount("outside", Button::new("Outside", |_| Ok(())))
        .unwrap();
    ui.focus(&outside).unwrap();
    ui.flush(Duration::from_secs(1)).unwrap();
    let callback = chosen.clone();
    let c = queue
        .push(Toast::new("C").action("Apply", move |_| {
            callback.set(2);
            Ok(())
        }))
        .unwrap();
    let callback = chosen.clone();
    let d = queue
        .push(Toast::new("D").action("Apply", move |_| {
            callback.set(3);
            Ok(())
        }))
        .unwrap();
    ui.flush(Duration::from_secs(1)).unwrap();
    assert_eq!(c.status(), ToastStatus::Visible);
    assert_eq!(d.status(), ToastStatus::Pending);
    ui.focus(&stack).unwrap();
    key(&mut ui, KeyCode::ArrowDown);
    ui.focus(&outside).unwrap();
    ui.flush(Duration::from_secs(5)).unwrap();
    assert_eq!(receipts[0].status(), ToastStatus::Expired);
    assert_eq!(receipts[1].status(), ToastStatus::Expired);
    assert_eq!(c.status(), ToastStatus::Visible);
    assert_eq!(d.status(), ToastStatus::Visible);
    ui.focus(&stack).unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get(), 2);
    assert_eq!(c.status(), ToastStatus::ActionSucceeded);
    assert_eq!(d.status(), ToastStatus::Visible);
}

fn scroll_offset(ui: &Runtime) -> f64 {
    match ui
        .inspect()
        .components
        .iter()
        .find(|c| c.key == "toasts")
        .unwrap()
        .details
        .properties["scroll_offset"]
    {
        InspectValue::Number(offset) => offset,
        _ => panic!("scroll offset must be numeric"),
    }
}
#[test]
fn overflowing_toasts_clamp_both_wheel_boundaries_without_repeating_work() {
    let Stage {
        mut ui,
        queue,
        stack,
        ..
    } = fixture(1);
    let _long = queue
        .push(Toast::new("Long feedback.\n".repeat(80)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let bounds = ui.bounds(&stack).unwrap().unwrap();
    let point = Point::new(bounds.x() + 5.0, bounds.y() + 5.0).unwrap();
    ui.set_profiling(true).unwrap();
    for delta in [100_000.0, -100_000.0] {
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Scroll(Point::new(0.0, delta).unwrap()),
        ))
        .unwrap();
        for _ in 0..3 {
            ui.flush(Duration::ZERO).unwrap();
        }
        if delta > 0.0 {
            assert!(scroll_offset(&ui) > 0.0);
        } else {
            assert_eq!(scroll_offset(&ui), 0.0);
        }
        let offset = scroll_offset(&ui);
        ui.paint().unwrap();
        ui.clear_profile().unwrap();
        for _ in 0..3 {
            ui.pointer_input(PointerEvent::new(
                point,
                PointerKind::Scroll(Point::new(0.0, delta).unwrap()),
            ))
            .unwrap();
            ui.flush(Duration::ZERO).unwrap();
        }
        assert_eq!(scroll_offset(&ui), offset);
        ui.paint().unwrap();
        assert!(
            ui.profile()
                .stats
                .iter()
                .filter(|s| s.key == "toasts")
                .all(|s| !matches!(
                    s.operation,
                    ProfileOperation::Measure | ProfileOperation::Arrange | ProfileOperation::Paint
                )),
            "{:?}",
            ui.profile()
        );
    }
}

#[test]
fn a_fitting_toast_arrangement_uses_no_more_text_work_than_its_measurement() {
    let mut s = fixture(1);
    s.ui.set_profiling(true).unwrap();
    s.ui.clear_profile().unwrap();
    s.ui.invalidate(&s.stack, Invalidation::Layout).unwrap();
    s.ui.flush(s.ui.now()).unwrap();
    let profile = s.ui.profile();
    assert_eq!(profile.dropped_samples, 0);
    let incarnation = profile
        .stats
        .iter()
        .find(|stat| stat.key == "toasts" && stat.operation == ProfileOperation::Arrange)
        .unwrap()
        .component;
    let work = |operation| {
        profile
            .recent
            .iter()
            .enumerate()
            .filter(|(_, sample)| sample.component == incarnation && sample.operation == operation)
            .map(|(index, _)| {
                profile.recent[..index]
                    .iter()
                    .rev()
                    .take_while(|sample| sample.operation == ProfileOperation::Text)
                    .count()
            })
            .collect::<Vec<_>>()
    };
    let measured = work(ProfileOperation::Measure);
    let arranged = work(ProfileOperation::Arrange);
    let expected = measured.iter().copied().max().unwrap();
    assert!(
        expected > 0,
        "measurement must prepare the fixture's actual text"
    );
    assert!(
        !arranged.is_empty(),
        "the fixture must arrange the toast host"
    );
    assert!(
        arranged.iter().all(|count| *count <= expected),
        "measure={measured:?}, arrange={arranged:?}"
    );
}

#[test]
fn expanding_the_viewport_clamps_toast_scroll_and_restores_first_control_hits() {
    let mut s = fixture(3);
    s.ui.set_viewport(Viewport::new(260.0, 120.0, 1.25).unwrap())
        .unwrap();
    s.ui.flush(s.ui.now()).unwrap();
    let bounds = s.ui.bounds(&s.stack).unwrap().unwrap();
    s.ui.pointer_input(PointerEvent::new(
        Point::new(bounds.x() + 3.0, bounds.y() + 3.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, 10000.0).unwrap()),
    ))
    .unwrap();
    s.ui.flush(s.ui.now()).unwrap();
    assert!(scroll_offset(&s.ui) > 0.0);
    s.ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    s.ui.flush(s.ui.now()).unwrap();
    assert_eq!(scroll_offset(&s.ui), 0.0);
    dismiss(&mut s.ui, &s.stack, 3, 0);
    assert_eq!(s.receipts[0].status(), ToastStatus::Dismissed);
    assert_eq!(s.receipts[1].status(), ToastStatus::Visible);
    assert_eq!(s.receipts[2].status(), ToastStatus::Visible);
}
#[test]
fn modified_toast_keys_yield_to_commands_and_held_arrows_keep_stepping() {
    for code in [KeyCode::ArrowDown, KeyCode::Delete] {
        let Stage {
            mut ui,
            receipts,
            stack,
            ..
        } = fixture(3);
        ui.focus(&stack).unwrap();
        let modifiers = Modifiers {
            alt: true,
            ..Modifiers::default()
        };
        let hits = Rc::new(Cell::new(0));
        let counted = hits.clone();
        let _command = ui
            .register_command(
                "Chord",
                CommandScope::Application,
                vec![KeyChord::new(CommandKey::Named(code), modifiers)],
                move |_, _| {
                    counted.set(counted.get() + 1);
                    Ok(())
                },
            )
            .unwrap();
        ui.native_modifiers(modifiers).unwrap();
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
                .unwrap();
        }
        ui.flush(ui.now()).unwrap();
        assert_eq!(hits.get(), 1, "{code:?}");
        assert!(
            receipts
                .iter()
                .all(|receipt| receipt.status() == ToastStatus::Visible),
            "{code:?}"
        );
        ui.native_modifiers(Modifiers::default()).unwrap();
    }
    // A held arrow steps once per repeat.
    let Stage {
        mut ui,
        chosen,
        stack,
        ..
    } = fixture(3);
    ui.focus(&stack).unwrap();
    for action in [KeyAction::Press, KeyAction::Repeat, KeyAction::Release] {
        ui.key_input(KeyCode::ArrowDown, action).unwrap();
    }
    key(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get(), 2);
}
