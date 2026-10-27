//! Input entry points settle in a flush of their own: a change notification a
//! component requests while handling the input is delivered before the entry
//! point returns.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

/// Requests one notification for every input it receives and counts the
/// deliveries.
struct Reporter {
    delivered: Rc<Cell<usize>>,
}
impl Component for Reporter {
    fn focusable(&self) -> bool {
        true
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 40.0)?)
    }
    fn key_input(
        &mut self,
        cx: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<(), RuntimeError> {
        cx.notifier().request();
        Ok(())
    }
    fn text_input(&mut self, cx: &mut EventContext, _: &str) -> Result<(), RuntimeError> {
        cx.notifier().request();
        Ok(())
    }
    fn text_preedit(&mut self, cx: &mut EventContext, _: &TextPreedit) -> Result<(), RuntimeError> {
        cx.notifier().request();
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        _: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        cx.notifier().request();
        Ok(InputOutcome::Handled)
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.delivered.set(self.delivered.get() + 1);
        Ok(())
    }
}

#[test]
fn every_input_entry_point_delivers_the_notifications_it_causes() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let delivered = Rc::new(Cell::new(0));
    let id = ui.mount_tree(Element::new(
        "reporter",
        Reporter {
            delivered: delivered.clone(),
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&id)?;
    let pending = std::cell::RefCell::new(Vec::new());
    let expect = |ui: &mut Runtime, entry: &'static str| {
        if delivered.get() != 1 {
            pending.borrow_mut().push(entry);
        }
        ui.flush(ui.now()).unwrap();
        delivered.set(0);
    };
    ui.key_input(KeyCode::KeyA, KeyAction::Press)?;
    expect(&mut ui, "key_input");
    ui.key_input(KeyCode::KeyA, KeyAction::Release)?;
    expect(&mut ui, "key_input release");
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::KeyB,
        KeyAction::Press,
        CommandKey::Character('b'),
    ))?;
    expect(&mut ui, "command_key_input");
    ui.text_input("b")?;
    expect(&mut ui, "text_input");
    ui.text_preedit(&TextPreedit::new("c", None)?)?;
    expect(&mut ui, "text_preedit");
    let center = Point::new(50.0, 20.0)?;
    ui.pointer_input(PointerEvent::new(
        center,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    expect(&mut ui, "pointer_input");
    assert_eq!(
        *pending.borrow(),
        Vec::<&str>::new(),
        "left notifications pending"
    );
    Ok(())
}

/// Requests a notification on a key press or pointer press, capturing the
/// pointer as a drag would, fails its delivery and counts how often its input
/// is cancelled.
struct FailingReporter {
    cancelled: Rc<Cell<usize>>,
}
impl Component for FailingReporter {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 40.0)?)
    }
    fn key_input(
        &mut self,
        cx: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<(), RuntimeError> {
        cx.notifier().request();
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        _: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        cx.capture_pointer()?;
        cx.notifier().request();
        Ok(InputOutcome::Handled)
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        Err(RuntimeError::EmptyKey)
    }
    fn cancel_input(&mut self, _: &mut EventContext) {
        self.cancelled.set(self.cancelled.get() + 1);
    }
}

#[test]
fn a_failing_notification_fails_the_input_and_cancels_it() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let cancelled = Rc::new(Cell::new(0));
    let id = ui.mount_tree(Element::new(
        "reporter",
        FailingReporter {
            cancelled: cancelled.clone(),
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&id)?;
    assert_eq!(
        ui.key_input(KeyCode::KeyA, KeyAction::Press),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(cancelled.replace(0), 1, "key_input");
    assert_eq!(
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::KeyB,
            KeyAction::Press,
            CommandKey::Character('b'),
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(cancelled.replace(0), 1, "command_key_input");
    assert!(
        !ui.keyboard().is_pressed(KeyCode::KeyB),
        "cancelling releases the key"
    );
    assert_eq!(
        ui.pointer_input(PointerEvent::new(
            Point::new(50.0, 20.0)?,
            PointerKind::Press(PointerButton::Primary),
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(cancelled.replace(0), 1, "pointer_input");
    Ok(())
}

/// Starts a drag on a press and requests a notification from every drag
/// event, whose delivery fails once `fail` is set.
struct Dragger {
    fail: Rc<Cell<bool>>,
}
impl Component for Dragger {
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
        if let PointerKind::Press(PointerButton::Primary) = event.kind() {
            cx.start_drag(
                DragPayload::new(()),
                DropOperations::MOVE,
                DragPreview::none(),
            )?;
        }
        Ok(InputOutcome::Handled)
    }
    fn drag_event(
        &mut self,
        cx: &mut EventContext,
        _: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        cx.notifier().request();
        Ok(DropResponse::Rejected)
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        if self.fail.get() {
            Err(RuntimeError::EmptyKey)
        } else {
            Ok(())
        }
    }
}

#[test]
fn a_failing_notification_during_a_drag_ends_the_drag() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let fail = Rc::new(Cell::new(false));
    ui.mount_tree(Element::new("dragger", Dragger { fail: fail.clone() }))?;
    ui.flush(Duration::ZERO)?;
    ui.pointer_input(PointerEvent::new(
        Point::new(50.0, 20.0)?,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    assert!(ui.drag_active());
    fail.set(true);
    assert_eq!(
        ui.pointer_input(PointerEvent::new(
            Point::new(60.0, 25.0)?,
            PointerKind::Move
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert!(!ui.drag_active(), "the failure ended the session");
    Ok(())
}
