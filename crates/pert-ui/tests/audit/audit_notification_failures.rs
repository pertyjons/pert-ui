//! Global change delivery preserves ownership, errors and bounded followups.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
type Action = Box<dyn FnMut(&mut EventContext) -> Result<(), RuntimeError>>;
struct State {
    notifier: RefCell<Option<Notifier>>,
    value: Signal<u32>,
    fail_update: Signal<bool>,
    unmount: Signal<bool>,
    allow_focus: Signal<bool>,
    focused_allowed: Cell<bool>,
    focus_override: Cell<Option<bool>>,
    observed: Cell<u32>,
    fail_callback: Cell<bool>,
    action: RefCell<Option<Action>>,
    cancel_action: RefCell<Option<Action>>,
    updated_notifier: RefCell<Option<Notifier>>,
    input_extra: RefCell<Option<Notifier>>,
    cancelled: Cell<usize>,
    pointer_events: Cell<usize>,
    start_drag: Cell<bool>,
    fail_key: Cell<bool>,
    text_editor: Cell<bool>,
    fail_text: Cell<bool>,
    unfocused_keys: Cell<bool>,
    held_action: Cell<bool>,
    child_feedback: Cell<bool>,
    feedback_serial: Cell<u32>,
}
impl State {
    fn new(ui: &Runtime) -> Rc<Self> {
        Rc::new(Self {
            notifier: RefCell::new(None),
            value: ui.signal(0),
            fail_update: ui.signal(false),
            unmount: ui.signal(false),
            allow_focus: ui.signal(true),
            focused_allowed: Cell::new(true),
            focus_override: Cell::new(None),
            observed: Cell::new(0),
            fail_callback: Cell::new(false),
            action: RefCell::new(None),
            cancel_action: RefCell::new(None),
            updated_notifier: RefCell::new(None),
            input_extra: RefCell::new(None),
            cancelled: Cell::new(0),
            pointer_events: Cell::new(0),
            start_drag: Cell::new(false),
            fail_key: Cell::new(false),
            text_editor: Cell::new(false),
            fail_text: Cell::new(false),
            unfocused_keys: Cell::new(false),
            held_action: Cell::new(false),
            child_feedback: Cell::new(false),
            feedback_serial: Cell::new(0),
        })
    }
    fn request(&self) {
        self.notifier.borrow().as_ref().unwrap().request();
    }
}
struct Reporter {
    state: Rc<State>,
    tag: &'static str,
    log: Rc<RefCell<Vec<&'static str>>>,
}
impl Component for Reporter {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        *self.state.notifier.borrow_mut() = Some(cx.notifier());
        if cx.read_paint(&self.state.fail_update)? {
            return Err(RuntimeError::EmptyKey);
        }
        if cx.read_paint(&self.state.unmount)? {
            cx.request_unmount();
        }
        self.state
            .focused_allowed
            .set(cx.read_paint(&self.state.allow_focus)?);
        let value = cx.read_paint(&self.state.value)?;
        if self.state.observed.replace(value) != value
            && let Some(notifier) = self.state.updated_notifier.borrow().as_ref()
        {
            notifier.request();
        }
        if self.state.child_feedback.get() {
            let next = self.state.feedback_serial.get() + 1;
            self.state.feedback_serial.set(next);
            cx.request_children(vec![Element::new(
                format!("feedback-{next}"),
                Label::new("Child"),
            )]);
        }
        Ok(())
    }
    fn receives_unfocused_keys(&self) -> bool {
        self.state.unfocused_keys.get()
    }
    fn focusable(&self) -> bool {
        self.state
            .focus_override
            .get()
            .unwrap_or(self.state.focused_allowed.get())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 60.0)?)
    }
    fn key_input(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<(), RuntimeError> {
        if event.code == KeyCode::KeyA {
            match event.transition {
                KeyTransition::Pressed => self.state.held_action.set(true),
                KeyTransition::Released | KeyTransition::UnmatchedRelease => {
                    self.state.held_action.set(false)
                }
                _ => {}
            }
        }
        if self.state.fail_key.get() {
            return Err(RuntimeError::EmptyKey);
        }
        self.state.request();
        if let Some(notifier) = self.state.input_extra.borrow().as_ref() {
            notifier.request();
        }
        Ok(())
    }
    fn text_key_event(
        &mut self,
        _: &mut EventContext,
        _: CommandKeyInput,
        _: KeyTransition,
        _: &KeyboardState,
    ) -> Result<TextKeyOutcome, RuntimeError> {
        if !self.state.text_editor.get() {
            return Ok(TextKeyOutcome::NotAnEditor);
        }
        self.state.held_action.set(true);
        if self.state.fail_text.get() {
            return Err(RuntimeError::EmptyKey);
        }
        Ok(TextKeyOutcome::Ignored)
    }
    fn text_preedit(&mut self, _: &mut EventContext, _: &TextPreedit) -> Result<(), RuntimeError> {
        if self.state.text_editor.get() {
            self.state.held_action.set(true);
        }
        Ok(())
    }
    fn text_input(&mut self, _: &mut EventContext, _: &str) -> Result<(), RuntimeError> {
        if self.state.text_editor.get() {
            self.state.held_action.set(true);
        }
        Ok(())
    }
    fn text_command(
        &mut self,
        _: &mut EventContext,
        _: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        if self.state.text_editor.get() {
            self.state.held_action.set(true);
            Ok(TextCommandOutcome::Handled)
        } else {
            Ok(TextCommandOutcome::NotAnEditor)
        }
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if matches!(event.kind(), PointerKind::Press(_) | PointerKind::Move) {
            self.state
                .pointer_events
                .set(self.state.pointer_events.get() + 1);
        }
        if matches!(event.kind(), PointerKind::Press(_)) {
            if self.state.start_drag.get() {
                cx.start_drag(
                    DragPayload::new(()),
                    DropOperations::MOVE,
                    DragPreview::none(),
                )?;
            } else {
                cx.capture_pointer()?;
            }
        }
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.state.cancelled.set(self.state.cancelled.get() + 1);
        self.state.held_action.set(false);
        if let Some(action) = self.state.cancel_action.borrow_mut().as_mut() {
            action(cx).unwrap();
        }
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.log.borrow_mut().push(self.tag);
        if let Some(action) = self.state.action.borrow_mut().as_mut() {
            action(cx)?;
        }
        if self.state.fail_callback.replace(false) {
            return Err(RuntimeError::EmptyCommandName);
        }
        Ok(())
    }
}
struct Stage {
    ui: Runtime,
    a: Rc<State>,
    b: Rc<State>,
    c: Rc<State>,
    a_id: WidgetId,
    b_id: WidgetId,
    c_id: WidgetId,
    b_window: NativeWindowId,
    log: Rc<RefCell<Vec<&'static str>>>,
}
fn stage() -> Stage {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let a = State::new(&ui);
    let b = State::new(&ui);
    let c = State::new(&ui);
    let log = Rc::new(RefCell::new(vec![]));
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([("a", &a), ("b", &b), ("c", &c)].map(|(tag, state)| {
                Element::new(
                    tag,
                    Reporter {
                        state: state.clone(),
                        tag,
                        log: log.clone(),
                    },
                )
            })),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let a_id = ui.find_descendant(&root, "a").unwrap().unwrap();
    let b_id = ui.find_descendant(&root, "b").unwrap().unwrap();
    let c_id = ui.find_descendant(&root, "c").unwrap().unwrap();
    let prepared = ui
        .prepare_detach(&b_id, NativeWindowOptions::new("B").unwrap())
        .unwrap();
    let b_window = ui.commit_detach(prepared).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    ui.select_window(&ui.primary_window()).unwrap();
    Stage {
        ui,
        a,
        b,
        c,
        a_id,
        b_id,
        c_id,
        b_window,
        log,
    }
}
#[test]
fn callback_failure_consumes_one_request_and_continues_ready_deliveries() {
    let mut s = stage();
    s.a.fail_callback.set(true);
    s.a.request();
    s.b.request();
    s.a.request();
    assert_eq!(
        s.ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::EmptyCommandName
    );
    assert_eq!(*s.log.borrow(), ["a", "b", "a"]);
    assert_eq!(s.ui.current_window(), s.ui.primary_window());
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*s.log.borrow(), ["a", "b", "a"]);
}
#[test]
fn failed_other_window_preparation_retains_entire_frontier_in_order() {
    let mut s = stage();
    s.b.fail_update.set(true).unwrap();
    s.a.request();
    s.b.request();
    s.a.request();
    assert_eq!(
        s.ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::EmptyKey
    );
    assert!(s.log.borrow().is_empty());
    assert_eq!(s.ui.current_window(), s.ui.primary_window());
    s.b.fail_update.set(false).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*s.log.borrow(), ["a", "b", "a"]);
}
#[test]
fn cross_window_callback_writes_settle_and_contribute_to_report() {
    let mut s = stage();
    let value = s.b.value.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |_| value.set(7)));
    s.a.request();
    let report = s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(s.b.observed.get(), 7);
    assert!(report.components_updated >= 1);
    assert_eq!(s.ui.current_window(), s.ui.primary_window());
}
#[test]
fn request_only_reentry_waits_for_next_flush_and_stays_behind_frontier() {
    let mut s = stage();
    let notifier = s.a.notifier.borrow().clone().unwrap();
    let once = Cell::new(true);
    *s.a.action.borrow_mut() = Some(Box::new(move |_| {
        if once.replace(false) {
            notifier.request();
        }
        Ok(())
    }));
    s.a.request();
    s.b.request();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*s.log.borrow(), ["a", "b"]);
    assert_eq!(s.ui.application_deadline(), Some(Duration::ZERO));
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*s.log.borrow(), ["a", "b", "a"]);
}
#[test]
fn pre_callback_settlement_notifications_deliver_before_input_return() {
    let mut s = stage();
    let value = s.b.value.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |_| value.set(7)));
    *s.b.updated_notifier.borrow_mut() = s.c.notifier.borrow().clone();
    *s.a.input_extra.borrow_mut() = s.b.notifier.borrow().clone();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    assert_eq!(*s.log.borrow(), ["a", "b", "c"]);
}
#[test]
fn post_callback_input_cleanup_can_invalidate_an_earlier_window() {
    let mut s = stage();
    let value = s.a.value.clone();
    *s.b.cancel_action.borrow_mut() = Some(Box::new(move |_| value.set(9)));
    s.ui.select_window(&s.b_window).unwrap();
    s.ui.focus(&s.b_id).unwrap();
    s.ui.select_window(&s.ui.primary_window()).unwrap();
    let allow = s.b.allow_focus.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |_| allow.set(false)));
    s.a.request();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(s.a.observed.get(), 9);
    assert_eq!(s.ui.current_window(), s.ui.primary_window());
}
#[test]
fn callback_feedback_uses_one_shared_wave_budget_and_remains_retryable() {
    let mut s = stage();
    let value = s.a.value.clone();
    let notifier = s.a.notifier.borrow().clone().unwrap();
    *s.a.action.borrow_mut() = Some(Box::new(move |_| {
        value.set(value.get()? + 1)?;
        notifier.request();
        Ok(())
    }));
    s.a.request();
    assert_eq!(
        s.ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::UpdateLimit
    );
    assert!(s.log.borrow().len() <= Runtime::MAX_FLUSH_WAVES);
    s.a.action.borrow_mut().take();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(s.a.observed.get(), s.a.value.get().unwrap());
    assert_eq!(s.ui.current_window(), s.ui.primary_window());
}

#[test]
fn presentation_preparation_does_not_replay_mailbox_or_task_frontiers() {
    let mut s = stage();
    let delivered = Rc::new(Cell::new(0));
    let captured = delivered.clone();
    let mailbox =
        s.ui.mailbox(Capacity::new(4).unwrap(), move |_: u32| {
            captured.set(captured.get() + 1);
            Ok(())
        })
        .unwrap();
    let sender = mailbox.sender();
    let task_ran = Rc::new(Cell::new(false));
    let task_flag = task_ran.clone();
    let value = s.b.value.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |cx| {
        sender.send(1).unwrap();
        let flag = task_flag.clone();
        let _receipt = cx.defer_while_shown(move |_| {
            flag.set(true);
            Ok(())
        })?;
        value.set(7)?;
        Ok(())
    }));
    s.a.request();
    s.b.request();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(s.b.observed.get(), 7);
    assert_eq!(delivered.get(), 0);
    assert!(!task_ran.get());
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(delivered.get(), 1);
    assert!(task_ran.get());
}

#[test]
fn retirement_during_preparation_discards_only_unmounted_requests() {
    let mut s = stage();
    s.b.unmount.set(true).unwrap();
    s.a.request();
    s.b.request();
    s.a.request();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*s.log.borrow(), ["a", "a"]);
    assert!(!s.ui.contains_window(&s.b_window));
    assert_eq!(s.ui.current_window(), s.ui.primary_window());
}
#[test]
fn callback_context_routes_focus_to_its_own_window_even_on_error() {
    let mut s = stage();
    let window = s.b_window.clone();
    *s.b.action.borrow_mut() = Some(Box::new(move |cx| {
        assert_eq!(cx.native_window(), window);
        cx.request_focus();
        Ok(())
    }));
    s.b.fail_callback.set(true);
    s.b.request();
    assert_eq!(
        s.ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::EmptyCommandName
    );
    assert_eq!(s.ui.current_window(), s.ui.primary_window());
    s.ui.select_window(&s.b_window).unwrap();
    assert_eq!(s.ui.focused(), Some(s.b_id));
}

#[test]
fn initially_dirty_recipient_allows_another_reentrant_frontier() {
    let mut s = stage();
    let notifier = s.a.notifier.borrow().clone().unwrap();
    let once = Cell::new(true);
    *s.a.action.borrow_mut() = Some(Box::new(move |_| {
        if once.replace(false) {
            notifier.request();
        }
        Ok(())
    }));
    s.b.value.set(7).unwrap();
    s.a.request();
    s.b.request();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*s.log.borrow(), ["a", "b", "a"]);
    assert_eq!(s.b.observed.get(), 7);
    assert_eq!(s.ui.application_deadline(), None);
}

#[test]
fn retained_other_window_failure_preserves_key_focus_and_pointer_capture() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 20.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    s.b.fail_update.set(true).unwrap();
    s.b.request();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(s.a.cancelled.get(), 0);
    assert_eq!(s.ui.focused(), Some(s.a_id.clone()));
    assert!(s.ui.keyboard().is_pressed(KeyCode::KeyA));
    let delivered_before_move = s.a.pointer_events.get();
    assert_eq!(
        s.ui.pointer_input(PointerEvent::new(
            Point::new(350.0, 250.0).unwrap(),
            PointerKind::Move
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(s.a.cancelled.get(), 0);
    assert_eq!(
        s.a.pointer_events.get(),
        delivered_before_move + 1,
        "capture still delivers outside the component"
    );
    s.b.fail_update.set(false).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    s.ui.pointer_input(PointerEvent::new(
        Point::new(350.0, 250.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    assert_eq!(s.a.pointer_events.get(), delivered_before_move + 2);
}
#[test]
fn unrelated_preparation_failure_does_not_end_current_drag() {
    let mut s = stage();
    s.a.start_drag.set(true);
    s.ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 20.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    assert!(s.ui.drag_active());
    s.b.fail_update.set(true).unwrap();
    s.b.request();
    assert_eq!(
        s.ui.pointer_input(PointerEvent::new(
            Point::new(25.0, 25.0).unwrap(),
            PointerKind::Move
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert!(s.ui.drag_active());
    assert_eq!(s.a.cancelled.get(), 0);
    s.b.fail_update.set(false).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    assert!(s.ui.drag_active());
}
#[test]
fn native_preflush_notifications_do_not_cancel_held_keys() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.command_key_input(CommandKeyInput::new(
        KeyCode::KeyA,
        KeyAction::Press,
        CommandKey::Character('a'),
    ))
    .unwrap();
    // The upcoming key's preflush generates a new notification from older work.
    *s.a.updated_notifier.borrow_mut() = s.b.notifier.borrow().clone();
    s.a.value.set(1).unwrap();
    s.b.fail_callback.set(true);
    assert_eq!(
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::KeyB,
            KeyAction::Press,
            CommandKey::Character('b')
        )),
        Err(RuntimeError::EmptyCommandName)
    );
    assert_eq!(s.a.cancelled.get(), 0);
    assert!(s.ui.keyboard().is_pressed(KeyCode::KeyA));
    assert!(
        !s.ui.keyboard().is_pressed(KeyCode::KeyB),
        "the preflush failed before dispatch"
    );
    assert_eq!(s.ui.focused(), Some(s.a_id.clone()));
}
#[test]
fn older_callback_descendants_do_not_adopt_current_input_provenance() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    // Make only the second B callback fail, after its old request creates a successor.
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let value = s.b.value.clone();
    let next = s.b.notifier.borrow().as_ref().unwrap().clone();
    *s.b.action.borrow_mut() = Some(Box::new(move |_| {
        counted.set(counted.get() + 1);
        if counted.get() == 1 {
            value.set(1)?;
            next.request();
            Ok(())
        } else {
            Err(RuntimeError::EmptyCommandName)
        }
    }));
    s.b.request();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press),
        Err(RuntimeError::EmptyCommandName)
    );
    assert_eq!(calls.get(), 2);
    assert_eq!(s.a.cancelled.get(), 0);
    assert!(s.ui.keyboard().is_pressed(KeyCode::KeyA));
}

#[test]
fn caught_nested_notification_error_cannot_poison_later_direct_input_cleanup() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.b.fail_callback.set(true);
    s.b.request();
    s.ui.select_window(&s.b_window).unwrap();
    s.ui.set_window_visible(false).unwrap();
    s.ui.select_window(&s.ui.primary_window()).unwrap();
    let a = s.a.clone();
    let b_window = s.b_window.clone();
    let command =
        s.ui.register_command(
            "Nested cleanup",
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Named(KeyCode::F1),
                Modifiers::default(),
            )],
            move |ui, _| {
                let previous = ui.current_window();
                ui.select_window(&b_window)?;
                ui.set_window_visible(true)?;
                ui.select_window(&previous)?;
                assert_eq!(
                    ui.key_input(KeyCode::KeyB, KeyAction::Press),
                    Err(RuntimeError::EmptyCommandName)
                );
                assert_eq!(a.cancelled.get(), 0, "the old hidden request is unrelated");
                a.fail_key.set(true);
                assert_eq!(
                    ui.key_input(KeyCode::KeyC, KeyAction::Press),
                    Err(RuntimeError::EmptyKey)
                );
                assert_eq!(
                    a.cancelled.get(),
                    1,
                    "the second call's direct error needs cleanup"
                );
                a.fail_key.set(false);
                Ok(())
            },
        )
        .unwrap();
    s.ui.command_key_input(CommandKeyInput::new(
        KeyCode::F1,
        KeyAction::Press,
        CommandKey::Named(KeyCode::F1),
    ))
    .unwrap();
    assert_eq!(s.a.cancelled.get(), 1);
    drop(command);
}
#[test]
fn command_action_propagating_nested_delivery_error_is_an_explicit_command_failure() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    s.b.fail_callback.set(true);
    s.b.request();
    s.ui.select_window(&s.b_window).unwrap();
    s.ui.set_window_visible(false).unwrap();
    s.ui.select_window(&s.ui.primary_window()).unwrap();
    let b_window = s.b_window.clone();
    s.ui.register_command(
        "Fail action",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Named(KeyCode::F1),
            Modifiers::default(),
        )],
        move |ui, _| {
            let previous = ui.current_window();
            ui.select_window(&b_window)?;
            ui.set_window_visible(true)?;
            ui.select_window(&previous)?;
            // Choosing to return this error explicitly fails the command action.
            ui.flush(ui.now()).map(|_| ())
        },
    )
    .unwrap();
    assert_eq!(
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::F1,
            KeyAction::Press,
            CommandKey::Named(KeyCode::F1)
        )),
        Err(RuntimeError::EmptyCommandName)
    );
    assert_eq!(s.a.cancelled.get(), 1);
    assert!(!s.ui.keyboard().is_pressed(KeyCode::KeyA));
}

#[test]
fn failed_native_preflush_cleans_up_a_blocked_held_key_release() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.command_key_input(CommandKeyInput::new(
        KeyCode::KeyA,
        KeyAction::Press,
        CommandKey::Character('a'),
    ))
    .unwrap();
    s.b.fail_update.set(true).unwrap();
    s.b.request();
    assert_eq!(
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::KeyA,
            KeyAction::Release,
            CommandKey::Character('a')
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert!(!s.ui.keyboard().is_pressed(KeyCode::KeyA));
    assert_eq!(s.a.cancelled.get(), 1);
}
#[test]
fn failed_headless_preflush_cleans_up_a_blocked_component_key_release() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    s.b.fail_update.set(true).unwrap();
    s.b.request();
    s.a.value.set(1).unwrap();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Release),
        Err(RuntimeError::EmptyKey)
    );
    assert!(!s.ui.keyboard().is_pressed(KeyCode::KeyA));
    assert_eq!(s.a.cancelled.get(), 1);
}
#[test]
fn failed_pointer_preflush_cleans_up_a_blocked_capture_release() {
    let mut s = stage();
    s.ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 20.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    s.b.fail_update.set(true).unwrap();
    s.b.request();
    s.a.value.set(1).unwrap();
    assert_eq!(
        s.ui.pointer_input(PointerEvent::new(
            Point::new(350.0, 250.0).unwrap(),
            PointerKind::Release(PointerButton::Primary)
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(s.a.cancelled.get(), 1);
    s.b.fail_update.set(false).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let events = s.a.pointer_events.get();
    s.ui.pointer_input(PointerEvent::new(
        Point::new(350.0, 250.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    assert_eq!(
        s.a.pointer_events.get(),
        events,
        "capture ended with physical release and cleanup"
    );
}

#[test]
fn unrelated_drag_preflush_failure_preserves_motion_but_cleans_up_blocked_release() {
    let mut s = stage();
    s.a.start_drag.set(true);
    s.ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 20.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    s.b.fail_update.set(true).unwrap();
    s.b.request();
    s.a.value.set(1).unwrap();
    assert_eq!(
        s.ui.pointer_input(PointerEvent::new(
            Point::new(25.0, 25.0).unwrap(),
            PointerKind::Move
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert!(
        s.ui.drag_active(),
        "a blocked motion must preserve the held drag"
    );
    s.a.value.set(2).unwrap();
    assert_eq!(
        s.ui.pointer_input(PointerEvent::new(
            Point::new(25.0, 25.0).unwrap(),
            PointerKind::Release(PointerButton::Primary)
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert!(
        !s.ui.drag_active(),
        "a blocked terminal release must finish the drag"
    );
}

#[test]
fn blocked_headless_release_cleans_up_an_unfocused_key_recipient() {
    let mut s = stage();
    s.a.unfocused_keys.set(true);
    s.a.allow_focus.set(false).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    assert!(s.ui.focused().is_none());
    let cancelled_before = s.a.cancelled.get();
    s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    assert!(s.a.held_action.get());
    s.b.fail_update.set(true).unwrap();
    s.b.request();
    s.a.value.set(1).unwrap();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Release),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(s.a.cancelled.get(), cancelled_before + 1);
    assert!(!s.a.held_action.get());
    assert!(!s.ui.keyboard().is_pressed(KeyCode::KeyA));
    s.b.fail_update.set(false).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    assert!(s.ui.focused().is_none());
    s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    assert!(s.a.held_action.get());
    s.b.fail_update.set(true).unwrap();
    s.b.request();
    s.a.value.set(2).unwrap();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Release),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(s.a.cancelled.get(), cancelled_before + 2);
    assert!(!s.a.held_action.get());
    assert!(!s.ui.keyboard().is_pressed(KeyCode::KeyA));
}

#[test]
fn caught_action_flush_error_does_not_suppress_later_update_failure_cleanup() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.b.fail_callback.set(true);
    s.b.request();
    s.ui.select_window(&s.b_window).unwrap();
    s.ui.set_window_visible(false).unwrap();
    s.ui.select_window(&s.ui.primary_window()).unwrap();
    let a = s.a.clone();
    let b_window = s.b_window.clone();
    s.ui.register_command(
        "Catch then update",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Named(KeyCode::F1),
            Modifiers::default(),
        )],
        move |ui, _| {
            let previous = ui.current_window();
            ui.select_window(&b_window)?;
            ui.set_window_visible(true)?;
            ui.select_window(&previous)?;
            assert_eq!(
                ui.flush(ui.now()).unwrap_err(),
                RuntimeError::EmptyCommandName
            );
            a.fail_update.set(true)?;
            a.request();
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::F1,
            KeyAction::Press,
            CommandKey::Named(KeyCode::F1)
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(s.a.cancelled.get(), 1);
    assert!(!s.ui.keyboard().is_pressed(KeyCode::F1));
}
struct RevealedProbe {
    shown: bool,
    callbacks: Rc<Cell<usize>>,
}
impl Component for RevealedProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 40.0)?)
    }
    fn child_visible(&self, _: &str) -> bool {
        self.shown
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        _: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        self.shown = true;
        cx.invalidate(Invalidation::Layout);
        cx.notifier().request();
        Ok(ChildReveal::Changed)
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let calls = self.callbacks.get() + 1;
        self.callbacks.set(calls);
        if calls == 1 {
            cx.notifier().request();
            Ok(())
        } else {
            Err(RuntimeError::EmptyCommandName)
        }
    }
}
#[test]
fn older_notification_reveal_successors_do_not_adopt_current_key() {
    let mut s = stage();
    let callbacks = Rc::new(Cell::new(0));
    let root =
        s.ui.mount_tree(
            Element::new(
                "reveal-probe",
                RevealedProbe {
                    shown: false,
                    callbacks: callbacks.clone(),
                },
            )
            .children([Element::new("hidden", Label::new("Hidden content"))]),
        )
        .unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let hidden = s.ui.children(&root).unwrap().remove(0);
    *s.b.action.borrow_mut() = Some(Box::new(move |cx| {
        let _receipt = cx.request_reveal(&hidden)?;
        Ok(())
    }));
    s.ui.focus(&s.a_id).unwrap();
    s.b.request();
    s.a.value.set(1).unwrap();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press),
        Err(RuntimeError::EmptyCommandName)
    );
    assert_eq!(callbacks.get(), 2);
    assert_eq!(s.a.cancelled.get(), 0);
    assert!(s.ui.keyboard().is_pressed(KeyCode::KeyA));
}

#[test]
fn caught_task_flush_error_does_not_suppress_later_update_failure_cleanup() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    let a = s.a.clone();
    let b_window = s.b_window.clone();
    let receipt = Rc::new(RefCell::new(None));
    let saved = receipt.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |cx| {
        let a = a.clone();
        let window = b_window.clone();
        *saved.borrow_mut() = Some(cx.defer_while_shown(move |ui| {
            let previous = ui.current_window();
            ui.select_window(&window)?;
            ui.set_window_visible(true)?;
            ui.select_window(&previous)?;
            assert_eq!(
                ui.flush(ui.now()).unwrap_err(),
                RuntimeError::EmptyCommandName
            );
            a.fail_update.set(true)?;
            Ok(())
        })?);
        Ok(())
    }));
    s.a.request();
    s.ui.flush(Duration::ZERO).unwrap();
    s.a.action.borrow_mut().take();
    s.b.fail_callback.set(true);
    s.b.request();
    s.ui.select_window(&s.b_window).unwrap();
    s.ui.set_window_visible(false).unwrap();
    s.ui.select_window(&s.ui.primary_window()).unwrap();
    assert_eq!(
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::F1,
            KeyAction::Press,
            CommandKey::Named(KeyCode::F1)
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(s.a.cancelled.get(), 1);
    assert_eq!(
        receipt.borrow().as_ref().unwrap().status(),
        TaskStatus::Succeeded
    );
}

#[test]
fn caught_nested_command_failure_cannot_suppress_fresh_input_cleanup() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    let inner =
        s.ui.register_command(
            "Inner failure",
            CommandScope::Application,
            vec![],
            |_, _| Err(RuntimeError::EmptyKey),
        )
        .unwrap();
    let a = s.a.clone();
    s.ui.register_command(
        "Catch inner then begin",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Named(KeyCode::F1),
            Modifiers::default(),
        )],
        move |ui, context| {
            assert_eq!(
                ui.invoke_command(&inner, context.clone()),
                Err(RuntimeError::EmptyKey)
            );
            assert_eq!(a.cancelled.get(), 1);
            ui.key_input(KeyCode::KeyA, KeyAction::Press)?;
            assert!(a.held_action.get());
            a.fail_callback.set(true);
            a.request();
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::F1,
            KeyAction::Press,
            CommandKey::Named(KeyCode::F1)
        )),
        Err(RuntimeError::EmptyCommandName)
    );
    assert_eq!(
        s.a.cancelled.get(),
        2,
        "the freshly begun action needs its own cleanup"
    );
    assert!(!s.a.held_action.get());
    assert!(!s.ui.keyboard().is_pressed(KeyCode::KeyA));
}

#[test]
fn own_notification_error_precedes_an_older_reveal_successor_error() {
    let mut s = stage();
    let callbacks = Rc::new(Cell::new(0));
    let root =
        s.ui.mount_tree(
            Element::new(
                "reveal-probe",
                RevealedProbe {
                    shown: false,
                    callbacks: callbacks.clone(),
                },
            )
            .children([Element::new("hidden", Label::new("Hidden content"))]),
        )
        .unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let hidden = s.ui.children(&root).unwrap().remove(0);
    *s.b.action.borrow_mut() = Some(Box::new(move |cx| {
        let _receipt = cx.request_reveal(&hidden)?;
        Ok(())
    }));
    *s.a.action.borrow_mut() = Some(Box::new(|_| Err(RuntimeError::EmptyKey)));
    s.ui.focus(&s.a_id).unwrap();
    // Settle focus/layout first so B is delivered alongside this key's own request.
    s.ui.flush(Duration::ZERO).unwrap();
    s.b.request();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press),
        Err(RuntimeError::EmptyKey)
    );
    assert_eq!(*s.log.borrow(), ["b", "a"]);
    assert_eq!(
        callbacks.get(),
        2,
        "reveals still run despite the earlier callback failure"
    );
    assert_eq!(s.a.cancelled.get(), 1);
    assert!(!s.a.held_action.get());
}

#[test]
fn queued_command_failure_cannot_suppress_cleanup_of_the_following_key() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    let command =
        s.ui.register_command(
            "Queued failure",
            CommandScope::Application,
            vec![],
            |_, _| Err(RuntimeError::EmptyCommandName),
        )
        .unwrap();
    let context = s.ui.command_context(CommandSource::Menu);
    let saved = Rc::new(RefCell::new(None));
    let receipt = saved.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |cx| {
        *saved.borrow_mut() = Some(cx.invoke_command(&command, context.clone())?);
        Ok(())
    }));
    s.a.request();
    s.ui.flush(Duration::ZERO).unwrap();
    s.a.action.borrow_mut().take();
    s.a.fail_key.set(true);
    assert_eq!(
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::KeyA,
            KeyAction::Press,
            CommandKey::Character('a')
        )),
        Err(RuntimeError::EmptyKey)
    );
    assert!(matches!(
        receipt.borrow().as_ref().unwrap().status(),
        CommandStatus::Finished(Err(RuntimeError::EmptyCommandName))
    ));
    assert_eq!(
        s.a.cancelled.get(),
        2,
        "queued command and the following failed key need separate cleanup"
    );
    assert!(!s.a.held_action.get());
    assert!(!s.ui.keyboard().is_pressed(KeyCode::KeyA));
}
#[test]
fn pending_reveal_cannot_restart_exhausted_notification_wave_budget() {
    let mut s = stage();
    let value = s.a.value.clone();
    let notifier = s.a.notifier.borrow().as_ref().unwrap().clone();
    let target = s.c.notifier.borrow().as_ref().unwrap().clone();
    // The target is already visible; its reveal still runs a nested flush.
    let root =
        s.ui.mount_tree(Element::new("reveal-budget", Label::new("Target")))
            .unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let queued = Rc::new(Cell::new(false));
    let reveal_receipt = Rc::new(RefCell::new(None));
    let receipt = reveal_receipt.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |cx| {
        if !queued.replace(true) {
            *receipt.borrow_mut() = Some(cx.request_reveal(&root)?);
        }
        value.set(value.get()? + 1)?;
        notifier.request();
        // Keep a second recipient in the frontier without producing feedback there.
        target.request();
        Ok(())
    }));
    s.a.request();
    assert_eq!(
        s.ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::UpdateLimit
    );
    let a_callbacks = s.log.borrow().iter().filter(|tag| **tag == "a").count();
    assert!(a_callbacks <= Runtime::MAX_FLUSH_WAVES);
    #[cfg(debug_assertions)]
    assert!(
        s.ui.validation_report()
            .issues
            .iter()
            .any(|issue| issue.code == ValidationCode::UpdateLimit)
    );
    assert!(matches!(
        reveal_receipt.borrow().as_ref().unwrap().status(),
        RevealStatus::Finished(Err(RuntimeError::UpdateLimit))
    ));
    s.a.action.borrow_mut().take();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(s.a.observed.get(), s.a.value.get().unwrap());
}

#[test]
fn first_callback_error_cannot_mask_exhausted_notification_wave_budget() {
    let mut s = stage();
    let value = s.a.value.clone();
    let state = s.a.clone();
    let notifier = s.a.notifier.borrow().as_ref().unwrap().clone();
    let target = s.c.notifier.borrow().as_ref().unwrap().clone();
    // The target is already visible; its reveal still runs a nested flush.
    let root =
        s.ui.mount_tree(Element::new("reveal-budget", Label::new("Target")))
            .unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let queued = Rc::new(Cell::new(false));
    let reveal_receipt = Rc::new(RefCell::new(None));
    let receipt = reveal_receipt.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |cx| {
        if !queued.replace(true) {
            *receipt.borrow_mut() = Some(cx.request_reveal(&root)?);
        }
        state.child_feedback.set(true);
        value.set(value.get()? + 1)?;
        notifier.request();
        // Keep a second recipient in the frontier without producing feedback there.
        target.request();
        Err(RuntimeError::EmptyKey)
    }));
    s.a.request();
    assert_eq!(
        s.ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::EmptyKey
    );
    let a_callbacks = s.log.borrow().iter().filter(|tag| **tag == "a").count();
    assert!(a_callbacks <= Runtime::MAX_FLUSH_WAVES);
    #[cfg(debug_assertions)]
    assert!(
        s.ui.validation_report()
            .issues
            .iter()
            .any(|issue| issue.code == ValidationCode::UpdateLimit)
    );
    assert!(matches!(
        reveal_receipt.borrow().as_ref().unwrap().status(),
        RevealStatus::Finished(Err(RuntimeError::UpdateLimit))
    ));
    s.a.child_feedback.set(false);
    s.a.action.borrow_mut().take();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(s.a.observed.get(), s.a.value.get().unwrap());
}

#[test]
fn a_full_reveal_queue_opens_deep_independent_paths_without_false_feedback() {
    let mut s = stage();
    s.ui.set_viewport(Viewport::new(400.0, 6000.0, 1.0).unwrap())
        .unwrap();
    let descriptions = (0..Runtime::MAX_REVEAL_REQUESTS).map(|index| {
        let mut content = Element::new(format!("leaf-{index}"), Label::new("Target"));
        for depth in 0..3 {
            content = Element::new(
                format!("tabs-{index}-{depth}"),
                Tabs::new([
                    Tab::new(
                        "start",
                        "Start",
                        Element::new("placeholder", Label::new("Start")),
                    ),
                    Tab::new("show", "Show", content),
                ]),
            );
        }
        content
    });
    let root =
        s.ui.mount_tree(Element::new("deep-targets", Column::new(descriptions)))
            .unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let targets: Vec<_> = (0..Runtime::MAX_REVEAL_REQUESTS)
        .map(|index| {
            s.ui.find_descendant(&root, &format!("leaf-{index}"))
                .unwrap()
                .unwrap()
        })
        .collect();
    assert!(
        s.ui.inspect()
            .components
            .iter()
            .filter(|item| item.key.starts_with("leaf-"))
            .all(|item| !item.visible)
    );
    let receipts = Rc::new(RefCell::new(Vec::new()));
    let saved = receipts.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |cx| {
        for target in &targets {
            saved.borrow_mut().push(cx.request_reveal(target)?);
        }
        Ok(())
    }));
    s.a.request();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(receipts.borrow().len(), Runtime::MAX_REVEAL_REQUESTS);
    assert!(receipts.borrow().iter().all(|receipt| matches!(receipt.status(), RevealStatus::Finished(Ok(ref outcome)) if outcome.is_revealed())));
    assert!(
        s.ui.inspect()
            .components
            .iter()
            .filter(|item| item.key.starts_with("leaf-"))
            .all(|item| item.visible)
    );
}

#[test]
fn headless_failure_cleans_the_recipient_selected_after_preflush_focus_transfer() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let target = s.c_id.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |cx| cx.request_focus_for(&target)));
    s.c.fail_key.set(true);
    s.a.value.set(1).unwrap();
    s.a.request();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap_err(),
        RuntimeError::EmptyKey
    );
    assert_eq!(s.ui.focused(), Some(s.c_id));
    assert_eq!(s.a.cancelled.get(), 1, "focus lifecycle runs once");
    assert_eq!(
        s.c.cancelled.get(),
        1,
        "actual failing key recipient is cleaned"
    );
    assert!(!s.c.held_action.get());
}

#[test]
fn headless_failure_cleans_the_unfocused_recipient_selected_after_preflush() {
    let mut s = stage();
    s.a.allow_focus.set(false).unwrap();
    s.c.allow_focus.set(false).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    s.a.unfocused_keys.set(true);
    let a = Rc::downgrade(&s.a);
    let c = Rc::downgrade(&s.c);
    *s.a.action.borrow_mut() = Some(Box::new(move |_| {
        a.upgrade().unwrap().unfocused_keys.set(false);
        c.upgrade().unwrap().unfocused_keys.set(true);
        Ok(())
    }));
    s.c.fail_key.set(true);
    s.a.value.set(1).unwrap();
    s.a.request();
    let a_before = s.a.cancelled.get();
    let c_before = s.c.cancelled.get();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap_err(),
        RuntimeError::EmptyKey
    );
    assert!(s.ui.focused().is_none());
    assert_eq!(s.a.cancelled.get(), a_before);
    assert_eq!(s.c.cancelled.get(), c_before + 1);
    assert!(!s.c.held_action.get());
}

#[test]
fn a_postdispatch_focus_transfer_does_not_cancel_the_new_focus_for_an_old_key_error() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let target = s.c_id.clone();
    *s.a.action.borrow_mut() = Some(Box::new(move |cx| {
        cx.request_focus_for(&target)?;
        Err(RuntimeError::EmptyKey)
    }));
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap_err(),
        RuntimeError::EmptyKey
    );
    assert_eq!(s.ui.focused(), Some(s.c_id));
    assert_eq!(s.a.cancelled.get(), 1);
    assert_eq!(s.c.cancelled.get(), 0);
    assert!(!s.a.held_action.get());
}

fn bubbling_fixture(parent: &Rc<State>, child: &Rc<State>) -> Element {
    let log = Rc::new(RefCell::new(Vec::new()));
    Element::new(
        "bubble-parent",
        Reporter {
            state: parent.clone(),
            tag: "parent",
            log: log.clone(),
        },
    )
    .children([Element::new(
        "bubble-child",
        Reporter {
            state: child.clone(),
            tag: "child",
            log,
        },
    )])
}

#[test]
fn headless_bubbling_failure_cleans_every_actual_keyboard_recipient() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let parent = State::new(&ui);
    let child = State::new(&ui);
    let description = bubbling_fixture(&parent, &child);
    let root = ui.mount_tree(description).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let focused = ui.find_descendant(&root, "bubble-child").unwrap().unwrap();
    ui.focus(&focused).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    parent.fail_key.set(true);
    assert_eq!(
        ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap_err(),
        RuntimeError::EmptyKey
    );
    assert_eq!(child.cancelled.get(), 1);
    assert_eq!(parent.cancelled.get(), 1);
    assert!(!child.held_action.get());
    assert!(!parent.held_action.get());
}

#[test]
fn headless_escape_text_bubbling_failure_cleans_every_actual_editor_recipient() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let parent = State::new(&ui);
    let child = State::new(&ui);
    parent.text_editor.set(true);
    child.text_editor.set(true);
    let content = bubbling_fixture(&parent, &child);
    let open = ui.signal(true);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "popup",
                Popover::new(open, Element::new("anchor", Label::new("Anchor")), content),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let focused = ui.find_descendant(&root, "bubble-child").unwrap().unwrap();
    ui.focus(&focused).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let parent_before = parent.cancelled.get();
    let child_before = child.cancelled.get();
    parent.fail_text.set(true);
    assert_eq!(
        ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap_err(),
        RuntimeError::EmptyKey
    );
    assert_eq!(child.cancelled.get(), child_before + 1);
    assert_eq!(parent.cancelled.get(), parent_before + 1);
    assert!(!child.held_action.get());
    assert!(!parent.held_action.get());
}

struct CloseObserver(ReadSignal<bool>);
impl Component for CloseObserver {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if !cx.read_paint(&self.0)? {
            return Err(RuntimeError::EmptyCommandName);
        }
        Ok(())
    }
}

#[test]
fn escape_hiding_traced_editors_before_update_failure_cancels_them_once() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let parent = State::new(&ui);
    let child = State::new(&ui);
    parent.text_editor.set(true);
    child.text_editor.set(true);
    let content = bubbling_fixture(&parent, &child);
    let open = ui.signal(true);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "popup",
                    Popover::new(
                        open.clone(),
                        Element::new("anchor", Label::new("Anchor")),
                        content,
                    ),
                ),
                Element::new("observer", CloseObserver(open.read_only())),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let focused = ui.find_descendant(&root, "bubble-child").unwrap().unwrap();
    ui.focus(&focused).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let parent_before = parent.cancelled.get();
    let child_before = child.cancelled.get();
    assert_eq!(
        ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap_err(),
        RuntimeError::EmptyCommandName
    );
    assert!(!open.get().unwrap());
    assert_eq!(child.cancelled.get(), child_before + 1);
    assert_eq!(parent.cancelled.get(), parent_before + 1);
    assert!(!child.held_action.get());
    assert!(!parent.held_action.get());
}

#[test]
fn focusability_cleanup_is_not_repeated_by_an_attributable_key_failure() {
    for (from_update, delayed_settlement) in
        [(true, false), (false, false), (true, true), (false, true)]
    {
        let mut s = stage();
        s.ui.focus(&s.a_id).unwrap();
        s.ui.flush(Duration::ZERO).unwrap();
        let state = Rc::downgrade(&s.a);
        *s.a.action.borrow_mut() = Some(Box::new(move |cx| {
            let state = state.upgrade().unwrap();
            if from_update {
                state.allow_focus.set(false)?;
            } else {
                state.focus_override.set(Some(false));
                cx.invalidate(Invalidation::Paint);
            }
            if delayed_settlement {
                state.fail_update.set(true)?;
            }
            Err(RuntimeError::EmptyCommandName)
        }));
        assert_eq!(
            s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap_err(),
            RuntimeError::EmptyCommandName
        );
        assert_eq!(
            s.a.cancelled.get(),
            1,
            "update: {from_update}, delayed: {delayed_settlement}"
        );
        assert!(!s.a.held_action.get());
        s.a.action.borrow_mut().take();
        if delayed_settlement {
            s.a.fail_update.set(false).unwrap();
            s.ui.flush(Duration::ZERO).unwrap();
            assert_eq!(
                s.a.cancelled.get(),
                1,
                "retry must not repeat completed cancellation"
            );
        }
        assert_ne!(s.ui.focused(), Some(s.a_id.clone()));
        s.a.focus_override.set(None);
        s.a.allow_focus.set(true).unwrap();
        s.ui.flush(Duration::ZERO).unwrap();
        s.ui.focus(&s.a_id).unwrap();
        s.ui.flush(Duration::ZERO).unwrap();
        s.ui.key_input(KeyCode::KeyA, KeyAction::Release).unwrap();
        s.a.fail_key.set(true);
        assert_eq!(
            s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap_err(),
            RuntimeError::EmptyKey
        );
        assert_eq!(
            s.a.cancelled.get(),
            2,
            "fresh dispatch can be cleaned again"
        );
        assert!(!s.a.held_action.get());
    }
}

#[test]
fn regained_focus_can_end_again_without_an_intervening_key_event() {
    for from_update in [true, false] {
        let mut s = stage();
        s.ui.focus(&s.a_id).unwrap();
        s.ui.flush(Duration::ZERO).unwrap();
        s.ui.clear_focus();
        let before = s.a.cancelled.get();
        s.ui.focus(&s.a_id).unwrap();
        s.ui.flush(Duration::ZERO).unwrap();
        if from_update {
            s.a.allow_focus.set(false).unwrap();
        } else {
            s.a.focus_override.set(Some(false));
        }
        s.ui.flush(Duration::ZERO).unwrap();
        assert_eq!(s.a.cancelled.get(), before + 1);
        assert_ne!(s.ui.focused(), Some(s.a_id));
    }
}

#[test]
fn delivered_input_rearms_cleanup_after_a_previous_keyboard_failure() {
    for (path, from_update) in
        (0..5).flat_map(|path| [true, false].map(|from_update| (path, from_update)))
    {
        let mut s = stage();
        s.ui.focus(&s.a_id).unwrap();
        s.ui.flush(Duration::ZERO).unwrap();
        s.a.fail_key.set(true);
        assert_eq!(
            s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap_err(),
            RuntimeError::EmptyKey
        );
        assert_eq!(s.a.cancelled.get(), 1);
        s.a.fail_key.set(false);
        s.a.text_editor.set(true);
        match path {
            0 => {
                s.ui.command_key_input(CommandKeyInput::new(
                    KeyCode::KeyB,
                    KeyAction::Press,
                    CommandKey::Character('b'),
                ))
                .unwrap();
            }
            1 => {
                s.ui.text_preedit(&TextPreedit::new("draft", None).unwrap())
                    .unwrap();
            }
            2 => {
                s.ui.text_input("committed").unwrap();
            }
            3 => {
                s.ui.text_command(&s.a_id, TextCommand::Copy).unwrap();
            }
            _ => {
                let state = s.a.clone();
                s.ui.event(&s.a_id, move |_| state.held_action.set(true))
                    .unwrap();
            }
        }
        assert!(s.a.held_action.get());
        if from_update {
            s.a.allow_focus.set(false).unwrap();
        } else {
            s.a.focus_override.set(Some(false));
        }
        s.ui.flush(Duration::ZERO).unwrap();
        assert_eq!(
            s.a.cancelled.get(),
            2,
            "text path: {path}, update: {from_update}"
        );
        assert!(!s.a.held_action.get());
        assert_ne!(s.ui.focused(), Some(s.a_id));
    }
}

#[test]
fn older_callback_failure_does_not_mask_current_callback_cleanup() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    s.b.fail_callback.set(true);
    *s.a.action.borrow_mut() = Some(Box::new(|_| Err(RuntimeError::EmptyKey)));
    s.b.request();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press),
        Err(RuntimeError::EmptyCommandName)
    );
    assert_eq!(*s.log.borrow(), ["b", "a"]);
    assert_eq!(s.a.cancelled.get(), 1);
    assert!(!s.a.held_action.get());
    assert!(s.ui.keyboard().is_pressed(KeyCode::KeyA));
    s.a.action.borrow_mut().take();
    // Cleanup ends the component-owned action, not physical pressed-key state.
    s.ui.key_input(KeyCode::KeyA, KeyAction::Release).unwrap();
    s.ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    assert!(s.a.held_action.get());
    s.ui.key_input(KeyCode::KeyA, KeyAction::Release).unwrap();
    assert!(!s.a.held_action.get());
}

#[test]
fn older_callback_failure_does_not_mask_current_recipient_preparation_cleanup() {
    let mut s = stage();
    s.ui.focus(&s.a_id).unwrap();
    s.ui.flush(Duration::ZERO).unwrap();
    let fail = s.a.fail_update.clone();
    *s.b.action.borrow_mut() = Some(Box::new(move |_| {
        fail.set(true)?;
        Err(RuntimeError::EmptyCommandName)
    }));
    s.b.request();
    assert_eq!(
        s.ui.key_input(KeyCode::KeyA, KeyAction::Press),
        Err(RuntimeError::EmptyCommandName)
    );
    assert_eq!(*s.log.borrow(), ["b"]);
    assert_eq!(s.a.cancelled.get(), 1);
    assert!(!s.a.held_action.get());
    assert!(s.ui.keyboard().is_pressed(KeyCode::KeyA));
    s.a.fail_update.set(false).unwrap();
    s.b.action.borrow_mut().take();
    s.ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*s.log.borrow(), ["b", "a"]);
}
