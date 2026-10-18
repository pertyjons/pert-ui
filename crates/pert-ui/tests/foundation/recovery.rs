//! Public contracts for bounded scheduling and host-driven input recovery.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

struct Bounce {
    active: Rc<Cell<bool>>,
    updates: Rc<Cell<usize>>,
}
impl Component for Bounce {
    fn focusable(&self) -> bool {
        true
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.updates.set(self.updates.get() + 1);
        Ok(())
    }
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.active.get() {
            cx.request_focus();
        }
        Ok(())
    }
}

#[test]
fn focus_feedback_is_bounded_diagnosed_and_repairable() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let active = Rc::new(Cell::new(true));
    let updates = Rc::new(Cell::new(0));
    let a = ui.mount(
        "first",
        Bounce {
            active: active.clone(),
            updates: updates.clone(),
        },
    )?;
    let b = ui.mount(
        "second",
        Bounce {
            active: active.clone(),
            updates: updates.clone(),
        },
    )?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&a)?;
    ui.focus(&b)?;
    updates.set(0);
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::UpdateLimit));
    assert!(updates.get() <= 2 * Runtime::MAX_FLUSH_WAVES);
    assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
    let report = ui.validation_report();
    if cfg!(debug_assertions) {
        let issue = report
            .issues
            .iter()
            .find(|issue| issue.code == ValidationCode::UpdateLimit)
            .unwrap();
        assert!(issue.message.contains("first") && issue.message.contains("second"));
        assert_eq!(issue.components.len(), 2);
    } else {
        assert!(!report.enabled);
    }
    // Phase/budget guards restore on failure. Writes are legal and pending work
    // can be drained after fixing the component, without remounting identities.
    let value = ui.signal(0);
    value.set(1)?;
    active.set(false);
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.next_deadline(), None);
    assert!(
        ui.validation_report()
            .issues
            .iter()
            .all(|i| i.code != ValidationCode::UpdateLimit)
    );
    assert_eq!(ui.key(&a)?, "first");
    Ok(())
}

#[test]
fn wave_limit_does_not_limit_number_of_independent_components() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let updates = Rc::new(Cell::new(0));
    let active = Rc::new(Cell::new(false));
    for index in 0..1000 {
        ui.mount(
            format!("static-{index}"),
            Bounce {
                active: active.clone(),
                updates: updates.clone(),
            },
        )?;
    }
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 1000);
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    Ok(())
}

#[test]
fn host_error_cleanup_preserves_numeric_draft_without_saving() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 100.0, 1.0).unwrap())?;
    let value = ui.signal(1_i64);
    let id = ui.mount_tree(Element::new("number", IntegerInput::new(value.clone())))?;
    ui.focus(&id)?;
    ui.text_input("2")?;
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    ui.cancel_input()?;
    assert_eq!(ui.focused(), Some(id));
    assert!(ui.keyboard().pressed().next().is_none());
    assert_eq!(value.get()?, 1);
    ui.flush(Duration::ZERO)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    assert_eq!(value.get()?, 21);
    Ok(())
}

struct Revealer;
impl Component for Revealer {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for i in 0..cx.len() {
            cx.place(
                i,
                Rect::new(i as f32 * 50.0, 0.0, 50.0, size.height()).unwrap(),
            )?;
        }
        Ok(())
    }
    fn reveal(&mut self, cx: &mut EventContext, _: Rect) -> Result<bool, RuntimeError> {
        cx.invalidate(Invalidation::Layout);
        Ok(true)
    }
}
#[test]
fn nested_focus_reveal_flushes_share_the_outer_budget() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())?;
    let active = Rc::new(Cell::new(false));
    let updates = Rc::new(Cell::new(0));
    let root = ui.mount_tree(Element::new("reveal", Revealer).children((0..2).map(|i| {
        Element::new(
            format!("focus-{i}"),
            Bounce {
                active: active.clone(),
                updates: updates.clone(),
            },
        )
    })))?;
    ui.flush(Duration::ZERO)?;
    let children = ui.children(&root)?;
    ui.focus(&children[0])?;
    updates.set(0);
    active.set(true);
    assert_eq!(ui.focus(&children[1]), Err(RuntimeError::UpdateLimit));
    assert!(updates.get() <= 2 * Runtime::MAX_FLUSH_WAVES);
    active.set(false);
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.next_deadline(), None);
    Ok(())
}

struct IgnoringChild;
impl Component for IgnoringChild {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
}
struct HeldParent {
    held: Rc<Cell<bool>>,
    resets: Rc<Cell<usize>>,
}
impl Component for HeldParent {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::KeyA {
            self.held.set(event.transition != KeyTransition::Released);
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn input_reset(&mut self) {
        self.held.set(false);
        self.resets.set(self.resets.get() + 1);
    }
}
#[test]
fn recovery_and_window_focus_loss_cancel_bubbling_keyboard_recipients() -> Result<(), RuntimeError>
{
    for focus_loss in [false, true] {
        let mut ui = Runtime::new();
        let held = Rc::new(Cell::new(false));
        let resets = Rc::new(Cell::new(0));
        let root = ui.mount_tree(
            Element::new(
                "parent",
                HeldParent {
                    held: held.clone(),
                    resets: resets.clone(),
                },
            )
            .children([Element::new("child", IgnoringChild)]),
        )?;
        ui.focus(&ui.children(&root)?[0])?;
        ui.key_input(KeyCode::KeyA, KeyAction::Press)?;
        assert!(held.get());
        let before = resets.get();
        if focus_loss {
            ui.clear_focus();
        } else {
            ui.cancel_input()?;
        }
        assert!(
            !held.get(),
            "the ancestor that handled the key must release its action"
        );
        assert_eq!(resets.get(), before + 1);
        assert!(ui.keyboard().pressed().next().is_none());
    }
    Ok(())
}
