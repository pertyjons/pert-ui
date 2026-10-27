//! Public text, focus and activation contracts without native backend imports.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn send(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) -> Result<InputOutcome, RuntimeError> {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
}
#[test]
fn text_is_shaped_rasterized_scaled_and_retained() {
    let mut ui = ui();
    let value = ui.signal("Hello, åäö!".to_owned());
    let root = ui
        .mount_tree(Element::new("label", Label::new(value.read_only())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let first = ui.paint().unwrap();
    assert!(
        first
            .masks()
            .iter()
            .any(|mask| mask.pixels().iter().any(|alpha| *alpha > 0))
    );
    let ptr = first.masks()[0].pixels().as_ptr();
    assert_eq!(ui.paint().unwrap().masks()[0].pixels().as_ptr(), ptr);
    let width = first.masks()[0].width();
    ui.set_viewport(Viewport::new(400.0, 200.0, 2.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let second = ui.paint().unwrap();
    assert!(second.masks()[0].width() >= width * 2 - 2);
    value.set("Changed".to_owned()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_ne!(
        ui.paint().unwrap().masks()[0].pixels().as_ptr(),
        second.masks()[0].pixels().as_ptr()
    );
    ui.unmount(&root).unwrap();
    assert!(ui.next_deadline().is_none());
}
#[test]
fn button_capture_release_escape_and_keyboard_repeat() {
    let mut ui = ui();
    let count = Rc::new(Cell::new(0));
    let action = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new(
                    "first",
                    Button::new("Apply", move |_| {
                        action.set(action.get() + 1);
                        Ok(())
                    }),
                ),
                Element::new("second", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let buttons = ui.children(&root).unwrap();
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    send(&mut ui, 300.0, 150.0, PointerKind::Move).unwrap();
    send(
        &mut ui,
        300.0,
        150.0,
        PointerKind::Release(PointerButton::Primary),
    )
    .unwrap();
    assert_eq!(count.get(), 0);
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    )
    .unwrap();
    assert_eq!(count.get(), 1);
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    )
    .unwrap();
    assert_eq!(count.get(), 1);
    ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    assert!(ui.keyboard().pressed().count() >= 2);
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    assert_eq!(count.get(), 2);
    ui.key_input(KeyCode::Tab, KeyAction::Press).unwrap();
    assert_eq!(ui.focused(), Some(buttons[1].clone()));
    ui.focus(&buttons[0]).unwrap();
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    ui.clear_focus();
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    )
    .unwrap();
    assert_eq!(count.get(), 2);
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    ui.set_visible(&root, false).unwrap();
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    )
    .unwrap();
    assert_eq!(count.get(), 2);
}
#[test]
fn action_errors_do_not_leave_capture_or_replay_activation() {
    let mut ui = ui();
    let count = Rc::new(Cell::new(0));
    let action = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new(
                    "fail",
                    Button::new("Fail", |_| Err(RuntimeError::BorrowConflict)),
                ),
                Element::new(
                    "ok",
                    Button::new("OK", move |_| {
                        action.set(action.get() + 1);
                        Ok(())
                    }),
                ),
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
    assert_eq!(
        send(
            &mut ui,
            10.0,
            10.0,
            PointerKind::Release(PointerButton::Primary)
        ),
        Err(RuntimeError::BorrowConflict)
    );
    let second = ui.children(&root).unwrap()[1].clone();
    let [x, y, _, _] = ui.bounds(&second).unwrap().unwrap().values();
    send(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    send(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Release(PointerButton::Primary),
    )
    .unwrap();
    assert_eq!(count.get(), 1);
}
#[test]
fn text_size_admission_and_mask_validation_are_explicit() {
    let mut ui = ui();
    ui.mount_tree(Element::new("large", Label::new("x".repeat(16 * 1024 + 1))))
        .unwrap();
    // A label refuses text over the limit without failing the flush.
    ui.flush(Duration::ZERO).unwrap();
    let large = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "large")
        .unwrap();
    assert_eq!(
        large.details.properties.get("text_error"),
        Some(&InspectValue::Text(
            RuntimeError::TextEdit(TextEditError::TooLarge).to_string()
        ))
    );
    assert!(MaskImage::new(0, 1, vec![]).is_err());
    assert!(MaskImage::new(2, 2, vec![255]).is_err());
}

#[test]
fn outside_release_and_repeat_after_focus_loss_never_activate() {
    let mut ui = ui();
    let count = Rc::new(Cell::new(0));
    let action = count.clone();
    let id = ui
        .mount_tree(Element::new(
            "button",
            Button::new("Apply", move |_| {
                action.set(action.get() + 1);
                Ok(())
            }),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let point = Point::new(10.0, 10.0).unwrap();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(point, PointerKind::Leave))
        .unwrap();
    ui.pointer_input(
        PointerEvent::new(point, PointerKind::Release(PointerButton::Primary)).outside(),
    )
    .unwrap();
    assert_eq!(count.get(), 0);
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    ui.clear_focus();
    ui.focus(&id).unwrap();
    assert_eq!(
        ui.key_input(KeyCode::Space, KeyAction::Repeat)
            .unwrap()
            .transition,
        KeyTransition::Repeated
    );
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    assert_eq!(count.get(), 0);
}

struct Faulty {
    fail_layout: ReadSignal<bool>,
    fail: bool,
    resets: Rc<Cell<usize>>,
}
impl Component for Faulty {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.fail = cx.read(&self.fail_layout)?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        if self.fail {
            Err(LayoutError::InvalidChild.into())
        } else {
            Ok(Size::from([64, 32]))
        }
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if matches!(event.kind(), PointerKind::Press(_)) {
            cx.capture_pointer()?;
        }
        Ok(InputOutcome::Handled)
    }
    fn input_reset(&mut self) {
        self.resets.set(self.resets.get() + 1);
    }
}
#[test]
fn nonfocusable_capture_resets_on_window_loss_and_failed_release_layout() {
    let mut ui = ui();
    let fail = ui.signal(false);
    let resets = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new(
        "capture",
        Faulty {
            fail_layout: fail.read_only(),
            fail: false,
            resets: resets.clone(),
        },
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
    ui.clear_focus();
    assert_eq!(resets.get(), 1);
    send(
        &mut ui,
        10.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    )
    .unwrap();
    fail.set(true).unwrap();
    assert!(
        send(
            &mut ui,
            10.0,
            10.0,
            PointerKind::Release(PointerButton::Primary)
        )
        .is_err()
    );
    assert_eq!(resets.get(), 2);
    fail.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.clear_focus();
    assert_eq!(resets.get(), 2);
}

#[test]
fn stationary_pointer_layout_and_visibility_flush_is_ready_to_paint() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "wrap",
            Wrap::new([
                Element::new("one", Button::new("One", |_| Ok(()))),
                Element::new("two", Button::new("Two", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    send(&mut ui, 80.0, 10.0, PointerKind::Move).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    ui.set_viewport(Viewport::new(70.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    send(&mut ui, 10.0, 10.0, PointerKind::Move).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let before = ui.paint().unwrap();
    ui.set_visible(&root, false).unwrap();
    ui.set_visible(&root, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(before.commands(), ui.paint().unwrap().commands());
}

#[test]
fn failed_press_cancels_new_capture() {
    struct FailedPress(Rc<Cell<usize>>);
    impl Component for FailedPress {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn pointer_input(
            &mut self,
            cx: &mut EventContext,
            _: PointerEvent,
        ) -> Result<InputOutcome, RuntimeError> {
            cx.capture_pointer()?;
            Err(RuntimeError::BorrowConflict)
        }
        fn input_reset(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let mut ui = ui();
    let resets = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new("fail", FailedPress(resets.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        send(
            &mut ui,
            10.0,
            10.0,
            PointerKind::Press(PointerButton::Primary)
        )
        .is_err()
    );
    assert_eq!(resets.get(), 1);
    ui.clear_focus();
    assert_eq!(resets.get(), 1);
}

#[test]
fn explicit_focus_changes_cancel_older_deferred_requests() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("a", Button::new("A", |_| Ok(()))),
                Element::new("b", Button::new("B", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let children = ui.children(&root).unwrap();
    ui.event(&children[0], |cx| cx.request_focus()).unwrap();
    ui.clear_focus();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.focused().is_none());
    ui.event(&children[0], |cx| cx.request_focus()).unwrap();
    ui.focus(&children[1]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.focused(), Some(children[1].clone()));
}

#[test]
fn window_focus_loss_resets_unfocused_hover_presentation() {
    let mut ui = ui();
    ui.mount_tree(Element::new("button", Button::new("Hover", |_| Ok(()))))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let resting = ui.paint().unwrap().commands().to_vec();
    send(&mut ui, 10.0, 10.0, PointerKind::Move).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_ne!(ui.paint().unwrap().commands(), resting);
    assert!(ui.focused().is_none());
    ui.clear_focus();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.paint().unwrap().commands(), resting);
}
#[test]
fn window_loss_does_not_redeliver_failed_parent_input_through_hover_child() {
    struct Parent {
        children: Vec<Element>,
        calls: Rc<Cell<usize>>,
    }
    impl Component for Parent {
        fn take_children(&mut self) -> Vec<Element> {
            std::mem::take(&mut self.children)
        }
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
            cx.measure(0, a)
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
            cx.place(0, Rect::from_size(s))
        }
        fn pointer_input(
            &mut self,
            cx: &mut EventContext,
            _: PointerEvent,
        ) -> Result<InputOutcome, RuntimeError> {
            self.calls.set(self.calls.get() + 1);
            cx.capture_pointer()?;
            Err(RuntimeError::BorrowConflict)
        }
    }
    let mut ui = ui();
    let calls = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new(
        "parent",
        Parent {
            children: vec![Element::new("child", Label::new("Hit child"))],
            calls: calls.clone(),
        },
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        send(
            &mut ui,
            10.0,
            10.0,
            PointerKind::Press(PointerButton::Primary)
        )
        .is_err()
    );
    assert_eq!(calls.get(), 1);
    ui.clear_focus();
    assert_eq!(calls.get(), 1);
}
