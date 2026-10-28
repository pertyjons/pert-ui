//! Contracts of the headless test driver, `testing::Driver`, and its
//! `TestError`: each action's effect, each refusal, and the cleanup that
//! releases what an action pressed when a step fails.
use pert_ui_runtime::testing::*;
use pert_ui_runtime::{
    ApplicationError, CommandId, CommandKey, CommandSource, FailurePhase, HeldCancellation,
    KeyEvent, KeyTransition, KeyboardState, NativeWindowOptions, ReadSignal, RevealRefusal,
    SemanticValue, Show, Signal, Tab, Tabs, TextCommand, TextCommandOutcome, Toast,
    ToastController, component::*, prelude::*,
};
use std::{cell::Cell, cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn failure(message: &str) -> RuntimeError {
    ApplicationError::from(message).into()
}

/// Whether `error` is the runtime's report of the application failure
/// `message`; an `ApplicationError` compares by identity, not by message.
fn is_failure(error: &TestError, message: &str) -> bool {
    matches!(error, TestError::Runtime(RuntimeError::Application(inner)) if inner.to_string() == message)
}

/// A runtime with a viewport, so mounted trees are laid out.
fn runtime() -> Result<Runtime, RuntimeError> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    Ok(ui)
}

type KeyLog = Rc<RefCell<Vec<(KeyCode, KeyTransition, Modifiers)>>>;

/// A focusable leaf that records each key event it receives with the
/// modifiers reported then. On a press of `trigger_on`'s key it sets the
/// trigger and requests its own notification, so the flush after the key's
/// dispatch updates the [`Watcher`], whose update fails once. With
/// `fail_releases`, every release it receives fails, naming the key.
struct Recorder {
    log: KeyLog,
    trigger_on: Option<(KeyCode, Signal<bool>)>,
    fail_releases: bool,
}
impl Component for Recorder {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 20.0)?)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.log
            .borrow_mut()
            .push((event.code, event.transition, keyboard.modifiers()));
        if let Some((code, trigger)) = &self.trigger_on
            && event.code == *code
            && event.transition == KeyTransition::Pressed
        {
            trigger.set(true)?;
            cx.notifier().request();
        }
        if self.fail_releases
            && matches!(
                event.transition,
                KeyTransition::Released | KeyTransition::UnmatchedRelease
            )
        {
            return Err(ApplicationError::from(format!("release {:?}", event.code)).into());
        }
        Ok(InputOutcome::Ignored)
    }
    /// Answers every text command, so the driver treats it as an editor.
    fn text_command(
        &mut self,
        _: &mut EventContext,
        _: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        Ok(TextCommandOutcome::Handled)
    }
}

/// Fails its first update after the trigger is set; later updates succeed.
struct Watcher {
    trigger: ReadSignal<bool>,
    failed: bool,
}
impl Component for Watcher {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if cx.read(&self.trigger)? && !std::mem::replace(&mut self.failed, true) {
            return Err(failure("update failed"));
        }
        Ok(())
    }
}

/// A pointer target that counts releases and can fail them; it reports no
/// semantics.
struct Target {
    releases: Rc<Cell<u32>>,
    fail_release: bool,
}
impl Component for Target {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(60.0, 30.0)?)
    }
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if let PointerKind::Release(PointerButton::Primary) = event.kind() {
            self.releases.set(self.releases.get() + 1);
            if self.fail_release {
                return Err(failure("release failed"));
            }
        }
        Ok(InputOutcome::Handled)
    }
}

/// A focusable leaf whose focus loss fails.
struct FailingFocusLoss;
impl Component for FailingFocusLoss {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 20.0)?)
    }
    fn focus_lost(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        Err(failure("focus loss failed"))
    }
}

fn counter(
    ui: &mut Runtime,
) -> (
    Signal<u32>,
    impl FnMut(&mut EventContext) -> Result<(), RuntimeError> + 'static,
) {
    let count = ui.signal(0_u32);
    let target = count.clone();
    (count, move |_: &mut EventContext| {
        target.update(|n| *n += 1)
    })
}

#[test]
fn settle_and_advance_flush_once_at_a_later_clock() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal(String::from("ab"));
    let length = text.map(String::len)?;
    let mut driver = Driver::new(&mut ui);
    let report = driver.settle()?;
    assert_eq!(driver.runtime().now(), Driver::FRAME);
    assert_eq!(report.next_deadline, driver.runtime().next_deadline());
    text.set("abcd".to_owned())?;
    driver.advance(Duration::from_secs(2))?;
    assert_eq!(
        driver.runtime().now(),
        Driver::FRAME + Duration::from_secs(2)
    );
    assert_eq!(length.get()?, 4);
    assert_eq!(format!("{driver:?}"), "Driver { .. }");
    Ok(())
}

/// Updates once per wake and asks for another one frame later.
struct Ticker(Rc<Cell<u32>>);
impl Component for Ticker {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.set(self.0.get() + 1);
        cx.wake_after(Driver::FRAME)
    }
}

#[test]
fn settle_ticks_one_frame_and_advance_is_one_deadline_jump() -> TestResult {
    assert_eq!(Driver::FRAME, Duration::from_millis(16));
    let mut ui = runtime()?;
    let ticks = Rc::new(Cell::new(0));
    let _ticker = ui.mount_tree(Element::new("ticker", Ticker(ticks.clone())))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    assert_eq!(ticks.get(), 1);
    for frame in 2..=4 {
        let before = driver.runtime().now();
        driver.settle()?;
        assert_eq!(driver.runtime().now(), before + Duration::from_millis(16));
        assert_eq!(ticks.get(), frame, "each settle is one tick");
    }
    // Twenty frames' worth of time in one jump: missed deadlines produce one
    // update, not twenty.
    driver.advance(Driver::FRAME * 20)?;
    assert_eq!(ticks.get(), 5);
    assert_eq!(driver.runtime().now(), Driver::FRAME * 24);
    Ok(())
}

#[test]
fn an_advance_past_the_clock_limit_is_refused_before_it_flushes() -> TestResult {
    let mut ui = Runtime::new();
    ui.flush(Duration::MAX)?;
    let mut driver = Driver::new(&mut ui);
    assert_eq!(
        driver.advance(Duration::from_nanos(1)),
        Err(TestError::ClockOverflow)
    );
    assert_eq!(driver.settle(), Err(TestError::ClockOverflow));
    assert_eq!(driver.runtime().now(), Duration::MAX);
    Ok(())
}

#[test]
fn wait_until_flushes_without_moving_the_clock_until_the_condition_holds() -> TestResult {
    let mut ui = runtime()?;
    let total = ui.signal(None::<u64>);
    let target = total.clone();
    let _task = ui.spawn(|| (1..=10).sum::<u64>(), move |sum| target.set(Some(sum)))?;
    let mut driver = Driver::new(&mut ui);
    driver.wait_until(Duration::from_secs(10), |_| Ok(total.get()?.is_some()))?;
    assert_eq!(total.get()?, Some(55));
    assert_eq!(driver.runtime().now(), Duration::ZERO);

    let error = driver
        .wait_until(Duration::from_millis(5), |_| Ok(false))
        .unwrap_err();
    assert!(
        matches!(error, TestError::TimedOut { waited } if waited >= Duration::from_millis(5)),
        "{error:?}"
    );
    // A failing condition stops the wait with its error.
    let error = driver
        .wait_until(Duration::from_secs(1), |_| Err(failure("condition")))
        .unwrap_err();
    assert!(is_failure(&error, "condition"), "{error:?}");
    Ok(())
}

#[test]
fn find_names_a_missing_path_and_semantics_reads_one_component() -> TestResult {
    let mut ui = runtime()?;
    let name = ui.signal(String::from("Ada"));
    let releases = Rc::new(Cell::new(0));
    let root = ui.mount_tree(Element::new(
        "form",
        Column::new([
            Element::new("title", Label::new("Profile")),
            Element::new("name", TextField::new(name)),
            Element::new(
                "plain",
                Target {
                    releases,
                    fail_release: false,
                },
            ),
        ]),
    ))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let title = driver.find(&root, &["title"])?;
    assert_eq!(driver.runtime().key(&title)?, "title");
    assert_eq!(
        driver.find(&root, &["missing", "child"]),
        Err(TestError::NotFound {
            path: vec!["missing".into(), "child".into()]
        })
    );
    assert_eq!(driver.semantics(&title)?.name.as_deref(), Some("Profile"));
    let field = driver.find(&root, &["name"])?;
    assert_eq!(
        driver.semantics(&field)?.value,
        Some(SemanticValue::Text("Ada".into()))
    );
    let plain = driver.find(&root, &["plain"])?;
    assert_eq!(
        driver.semantics(&plain),
        Err(TestError::NoSemantics {
            key: "plain".into()
        })
    );
    Ok(())
}

#[test]
fn click_reveals_presses_and_releases_at_the_middle_and_settles() -> TestResult {
    let mut ui = runtime()?;
    let (clicks, action) = counter(&mut ui);
    let tabs = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new(
                "home",
                "Home",
                Element::new("welcome", Label::new("Welcome")),
            ),
            Tab::new(
                "more",
                "More",
                Element::new("add", Button::new("Add", action)),
            ),
        ]),
    ))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    // The button is on the hidden tab: the click reveals it first.
    let add = driver
        .runtime()
        .find_descendant(&tabs, "add")?
        .ok_or("add")?;
    let before = driver.runtime().now();
    driver.click(&add)?;
    assert_eq!(clicks.get()?, 1);
    assert_eq!(driver.runtime().now(), before + Driver::FRAME);
    Ok(())
}

#[test]
fn click_refuses_another_window_and_a_refused_reveal() -> TestResult {
    let mut ui = runtime()?;
    let (clicks, action) = counter(&mut ui);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("hidden", Button::new("Hidden", |_| Ok(()))),
            Element::new(
                "panel",
                View::new(Element::new("away", Button::new("Away", action))),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let panel = ui.find(&root, &["panel"])?.ok_or("panel")?;
    let prepared = ui.prepare_detach(&panel, NativeWindowOptions::new("Panel")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    ui.flush(Duration::ZERO)?;
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    let hidden = ui.find(&root, &["hidden"])?.ok_or("hidden")?;
    ui.set_visible(&hidden, false)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;

    let away = driver.find(&root, &["panel", "away"])?;
    assert_eq!(
        driver.click(&away),
        Err(TestError::OtherWindow { key: "away".into() })
    );
    assert_eq!(clicks.get()?, 0);
    assert_eq!(driver.runtime().current_window(), primary);
    // Selected, the detached window takes the click.
    driver.runtime_mut().select_window(&detached)?;
    driver.click(&away)?;
    assert_eq!(clicks.get()?, 1);
    driver.runtime_mut().select_window(&primary)?;

    assert_eq!(
        driver.click(&hidden),
        Err(TestError::NotRevealed {
            key: "hidden".into(),
            refusal: RevealRefusal::Hidden {
                component: hidden.clone()
            },
        })
    );
    Ok(())
}

#[test]
fn replace_text_selects_all_then_types_or_deletes() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal(String::from("Example"));
    let root = ui.mount_tree(Element::new(
        "form",
        Column::new([
            Element::new("field", TextField::new(text.clone())),
            Element::new("button", Button::new("Save", |_| Ok(()))),
        ]),
    ))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let field = driver.find(&root, &["field"])?;
    driver.replace_text(&field, "Replaced")?;
    assert_eq!(text.get()?, "Replaced");
    assert_eq!(driver.runtime().focused(), Some(field.clone()));
    // Empty text deletes the selection with Backspace.
    driver.replace_text(&field, "")?;
    assert_eq!(text.get()?, "");
    assert!(driver.runtime().keyboard().pressed().next().is_none());

    let button = driver.find(&root, &["button"])?;
    assert_eq!(
        driver.replace_text(&button, "x"),
        Err(TestError::NotAnEditor {
            key: "button".into()
        })
    );
    Ok(())
}

#[test]
fn replace_text_refuses_an_editor_in_another_window_before_focusing_it() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal(String::from("Kept"));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "panel",
            View::new(Element::new("field", TextField::new(text.clone()))),
        )]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let panel = ui.find(&root, &["panel"])?.ok_or("panel")?;
    let prepared = ui.prepare_detach(&panel, NativeWindowOptions::new("Panel")?)?;
    let _detached = ui.commit_detach(prepared)?;
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let field = driver.find(&root, &["panel", "field"])?;
    assert_eq!(
        driver.replace_text(&field, "Lost"),
        Err(TestError::OtherWindow {
            key: "field".into()
        })
    );
    assert_eq!(driver.runtime().current_window(), primary);
    assert_eq!(driver.runtime().focused(), None);
    assert_eq!(text.get()?, "Kept");
    Ok(())
}

#[test]
fn type_text_inserts_at_the_caret_and_settles() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal(String::from("World"));
    let count = text.map(String::len)?;
    let field = ui.mount_tree(Element::new("field", TextField::new(text.clone())))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.runtime_mut().focus(&field)?;
    driver.type_text("Hello ")?;
    assert_eq!(text.get()?, "Hello World");
    assert_eq!(count.get()?, 11);
    Ok(())
}

#[test]
fn key_and_chord_reach_commands_and_the_focused_component() -> TestResult {
    let mut ui = runtime()?;
    let log = KeyLog::default();
    let recorder = ui.mount_tree(Element::new(
        "recorder",
        Recorder {
            log: log.clone(),
            trigger_on: None,
            fail_releases: false,
        },
    ))?;
    let refreshes = ui.signal(0_u32);
    let target = refreshes.clone();
    ui.register_command(
        "Refresh",
        CommandScope::Application,
        vec![KeyChord::named(KeyCode::F5)],
        move |_, _| target.update(|n| *n += 1),
    )?;
    let saves = ui.signal(0_u32);
    let target = saves.clone();
    ui.register_command(
        "Save",
        CommandScope::Application,
        vec![KeyChord::primary('s')],
        move |_, _| target.update(|n| *n += 1),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.runtime_mut().focus(&recorder)?;

    driver.key(KeyCode::F5)?;
    assert_eq!(refreshes.get()?, 1);
    driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    assert_eq!(saves.get()?, 1);

    // A chord no command claims reaches the focused component, with the
    // modifiers pressed Control, Alt, Shift, Super and released in reverse.
    log.borrow_mut().clear();
    let all = Modifiers {
        shift: true,
        control: true,
        alt: true,
        super_key: true,
    };
    driver.chord(
        KeyChord::new(CommandKey::Character('k'), all),
        KeyCode::KeyK,
    )?;
    let sequence: Vec<_> = log
        .borrow()
        .iter()
        .map(|(code, transition, _)| (*code, *transition))
        .collect();
    use KeyTransition::{Pressed, Released};
    assert_eq!(
        sequence,
        [
            (KeyCode::ControlLeft, Pressed),
            (KeyCode::AltLeft, Pressed),
            (KeyCode::ShiftLeft, Pressed),
            (KeyCode::SuperLeft, Pressed),
            (KeyCode::KeyK, Pressed),
            (KeyCode::KeyK, Released),
            (KeyCode::SuperLeft, Released),
            (KeyCode::ShiftLeft, Released),
            (KeyCode::AltLeft, Released),
            (KeyCode::ControlLeft, Released),
        ]
    );
    let principal = log.borrow()[4].2;
    assert_eq!(principal, all);
    let keyboard = driver.runtime().keyboard();
    assert!(keyboard.pressed().next().is_none());
    assert_eq!(keyboard.modifiers(), Modifiers::default());

    // A key is sent without modifiers.
    log.borrow_mut().clear();
    driver.key(KeyCode::KeyQ)?;
    assert_eq!(
        *log.borrow(),
        [
            (KeyCode::KeyQ, Pressed, Modifiers::default()),
            (KeyCode::KeyQ, Released, Modifiers::default()),
        ]
    );
    Ok(())
}

#[test]
fn the_driver_refuses_held_keys_and_reported_modifiers() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal(String::from("Kept"));
    let (clicks, action) = counter(&mut ui);
    let root = ui.mount_tree(Element::new(
        "form",
        Column::new([
            Element::new("field", TextField::new(text.clone())),
            Element::new("button", Button::new("Add", action)),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let field = ui.find(&root, &["field"])?.ok_or("field")?;
    let button = ui.find(&root, &["button"])?.ok_or("button")?;
    ui.key_input(KeyCode::KeyA, KeyAction::Press)?;
    let mut driver = Driver::new(&mut ui);
    assert_eq!(driver.key(KeyCode::Enter), Err(TestError::InputHeld));
    assert_eq!(
        driver.chord(KeyChord::primary('s'), KeyCode::KeyS),
        Err(TestError::InputHeld)
    );
    assert_eq!(
        driver.replace_text(&field, "Lost"),
        Err(TestError::InputHeld)
    );
    assert_eq!(driver.click(&button), Err(TestError::InputHeld));
    // The press the test made is still held: the driver released nothing.
    assert!(driver.runtime().keyboard().is_pressed(KeyCode::KeyA));
    driver
        .runtime_mut()
        .key_input(KeyCode::KeyA, KeyAction::Release)?;

    driver.runtime_mut().native_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    })?;
    assert_eq!(driver.key(KeyCode::Enter), Err(TestError::InputHeld));
    driver
        .runtime_mut()
        .native_modifiers(Modifiers::default())?;
    driver.click(&button)?;
    assert_eq!(clicks.get()?, 1);
    assert_eq!(text.get()?, "Kept");
    Ok(())
}

/// A focused recorder whose press of `key` makes the flush after that key's
/// dispatch fail once: the key was admitted when its error is reported. The
/// runtime cancels input for that failure, so the driver's releases arrive
/// as `UnmatchedRelease`; the log shows that they were sent.
fn failing_after(key: KeyCode) -> Result<(Runtime, KeyLog), Box<dyn std::error::Error>> {
    let mut ui = runtime()?;
    let log = KeyLog::default();
    let trigger = ui.signal(false);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "watcher",
                Watcher {
                    trigger: trigger.read_only(),
                    failed: false,
                },
            ),
            Element::new(
                "recorder",
                Recorder {
                    log: log.clone(),
                    trigger_on: Some((key, trigger)),
                    fail_releases: false,
                },
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let recorder = ui.find(&root, &["recorder"])?.ok_or("recorder")?;
    ui.focus(&recorder)?;
    Ok((ui, log))
}

fn transitions(log: &KeyLog) -> Vec<(KeyCode, KeyTransition)> {
    log.borrow()
        .iter()
        .map(|(code, transition, _)| (*code, *transition))
        .collect()
}

#[test]
fn a_key_whose_press_reports_an_error_is_still_released() -> TestResult {
    let (mut ui, log) = failing_after(KeyCode::KeyQ)?;
    let mut driver = Driver::new(&mut ui);
    let error = driver.key(KeyCode::KeyQ).unwrap_err();
    assert!(is_failure(&error, "update failed"), "{error:?}");
    assert!(driver.runtime().keyboard().pressed().next().is_none());
    assert_eq!(
        transitions(&log),
        [
            (KeyCode::KeyQ, KeyTransition::Pressed),
            (KeyCode::KeyQ, KeyTransition::UnmatchedRelease)
        ]
    );
    Ok(())
}

#[test]
fn a_chord_releases_its_modifiers_after_a_failed_modifier_press() -> TestResult {
    let (mut ui, log) = failing_after(KeyCode::ControlLeft)?;
    let mut driver = Driver::new(&mut ui);
    let chord = KeyChord::new(
        CommandKey::Character('k'),
        Modifiers {
            control: true,
            alt: true,
            ..Modifiers::default()
        },
    );
    let error = driver.chord(chord, KeyCode::KeyK).unwrap_err();
    assert!(is_failure(&error, "update failed"), "{error:?}");
    // Control's press failed: nothing further was pressed, and Control was
    // released with the empty modifier snapshot.
    assert_eq!(
        transitions(&log),
        [
            (KeyCode::ControlLeft, KeyTransition::Pressed),
            (KeyCode::ControlLeft, KeyTransition::UnmatchedRelease)
        ]
    );
    let keyboard = driver.runtime().keyboard();
    assert!(keyboard.pressed().next().is_none());
    assert_eq!(keyboard.modifiers(), Modifiers::default());
    Ok(())
}

#[test]
fn a_chord_releases_its_key_and_modifiers_after_a_failed_key_press() -> TestResult {
    let (mut ui, log) = failing_after(KeyCode::KeyK)?;
    let mut driver = Driver::new(&mut ui);
    let chord = KeyChord::new(
        CommandKey::Character('k'),
        Modifiers {
            control: true,
            shift: true,
            ..Modifiers::default()
        },
    );
    let error = driver.chord(chord, KeyCode::KeyK).unwrap_err();
    assert!(is_failure(&error, "update failed"), "{error:?}");
    use KeyTransition::{Pressed, UnmatchedRelease};
    assert_eq!(
        transitions(&log),
        [
            (KeyCode::ControlLeft, Pressed),
            (KeyCode::ShiftLeft, Pressed),
            (KeyCode::KeyK, Pressed),
            (KeyCode::KeyK, UnmatchedRelease),
            (KeyCode::ShiftLeft, UnmatchedRelease),
            (KeyCode::ControlLeft, UnmatchedRelease),
        ]
    );
    let keyboard = driver.runtime().keyboard();
    assert!(keyboard.pressed().next().is_none());
    assert_eq!(keyboard.modifiers(), Modifiers::default());
    Ok(())
}

#[test]
fn a_click_whose_press_reports_an_error_is_still_released() -> TestResult {
    let mut ui = runtime()?;
    let releases = Rc::new(Cell::new(0));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("first", FailingFocusLoss),
            Element::new(
                "target",
                Target {
                    releases: releases.clone(),
                    fail_release: false,
                },
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let first = ui.find(&root, &["first"])?.ok_or("first")?;
    let target = ui.find(&root, &["target"])?.ok_or("target")?;
    ui.focus(&first)?;
    let mut driver = Driver::new(&mut ui);
    // The press moves focus, and the previous recipient's focus loss fails
    // after the press was delivered.
    let error = driver.click(&target).unwrap_err();
    assert!(is_failure(&error, "focus loss failed"), "{error:?}");
    assert_eq!(releases.get(), 1);
    Ok(())
}

#[test]
fn a_failed_release_is_cleanup_beside_the_press_error_or_alone() -> TestResult {
    let mut ui = runtime()?;
    let releases = Rc::new(Cell::new(0));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("first", FailingFocusLoss),
            Element::new(
                "target",
                Target {
                    releases: releases.clone(),
                    fail_release: true,
                },
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let first = ui.find(&root, &["first"])?.ok_or("first")?;
    let target = ui.find(&root, &["target"])?.ok_or("target")?;
    ui.focus(&first)?;
    let mut driver = Driver::new(&mut ui);
    let error = driver.click(&target).unwrap_err();
    assert!(
        matches!(
            &error,
            TestError::Cleanup { error, cleanup: RuntimeError::Application(cleanup) }
                if is_failure(error, "focus loss failed") && cleanup.to_string() == "release failed"
        ),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "focus loss failed; releasing the pressed input also failed"
    );
    assert_eq!(
        std::error::Error::source(&error).map(ToString::to_string),
        Some("release failed".to_owned())
    );
    // With the press succeeding, the release's failure is the result.
    let error = driver.click(&target).unwrap_err();
    assert!(is_failure(&error, "release failed"), "{error:?}");
    assert_eq!(releases.get(), 2);
    Ok(())
}

#[test]
fn cleanup_goes_on_after_a_failed_release_and_reports_the_first() -> TestResult {
    let mut ui = runtime()?;
    let log = KeyLog::default();
    let recorder = ui.mount_tree(Element::new(
        "recorder",
        Recorder {
            log: log.clone(),
            trigger_on: None,
            fail_releases: true,
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&recorder)?;
    let mut driver = Driver::new(&mut ui);
    let chord = KeyChord::new(
        CommandKey::Character('k'),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    );
    // Only cleanup failed: its first failure, the key's release, is the
    // result, and Control was still released after it.
    let error = driver.chord(chord, KeyCode::KeyK).unwrap_err();
    assert!(is_failure(&error, "release KeyK"), "{error:?}");
    assert_eq!(
        transitions(&log).last().map(|(code, _)| *code),
        Some(KeyCode::ControlLeft)
    );
    let keyboard = driver.runtime().keyboard();
    assert!(keyboard.pressed().next().is_none());
    assert_eq!(keyboard.modifiers(), Modifiers::default());
    Ok(())
}

/// Toggles `shown` when F6 or Control is released, the last key event of
/// `Driver::key(F6)` and of a Control chord.
struct ReleaseWriter(Signal<bool>);
impl Component for ReleaseWriter {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 20.0)?)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition == KeyTransition::Released
            && matches!(event.code, KeyCode::F6 | KeyCode::ControlLeft)
        {
            self.0.set(!self.0.get()?)?;
        }
        Ok(InputOutcome::Ignored)
    }
}

#[test]
fn key_and_chord_settle_so_their_effect_is_shown() -> TestResult {
    let mut ui = runtime()?;
    let shown = ui.signal(false);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("writer", ReleaseWriter(shown.clone())),
            Element::new(
                "panel",
                Show::new(&shown, |_| Element::new("note", Label::new("Shown"))),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let writer = ui.find(&root, &["writer"])?.ok_or("writer")?;
    let panel = ui.find(&root, &["panel"])?.ok_or("panel")?;
    ui.focus(&writer)?;
    let mut driver = Driver::new(&mut ui);
    // The binding is written by the last key event; only a flush after it
    // mounts the child.
    driver.key(KeyCode::F6)?;
    assert_eq!(driver.runtime().children(&panel)?.len(), 1);
    let control = KeyChord::new(
        CommandKey::Character('h'),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    );
    driver.chord(control, KeyCode::KeyH)?;
    assert!(driver.runtime().children(&panel)?.is_empty());
    Ok(())
}

#[test]
fn an_empty_replacements_backspace_is_released_after_its_press_fails() -> TestResult {
    let (mut ui, log) = failing_after(KeyCode::Backspace)?;
    let recorder = ui.focused().ok_or("the recorder is focused")?;
    let mut driver = Driver::new(&mut ui);
    let error = driver.replace_text(&recorder, "").unwrap_err();
    assert!(is_failure(&error, "update failed"), "{error:?}");
    assert_eq!(
        transitions(&log),
        [
            (KeyCode::Backspace, KeyTransition::Pressed),
            (KeyCode::Backspace, KeyTransition::UnmatchedRelease)
        ]
    );
    assert!(driver.runtime().keyboard().pressed().next().is_none());
    Ok(())
}

// `Driver::clear_focus`.

#[test]
fn clear_focus_saves_a_focused_draft_and_leaves_nothing_focused() -> TestResult {
    let mut ui = runtime()?;
    let tag = ui.signal(String::new());
    let field = ui.mount_tree(Element::new(
        "tag",
        TextField::new(tag.clone()).normalize(|text| Ok(text.trim().to_owned())),
    ))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.runtime_mut().focus(&field)?;
    driver.type_text(" draft ")?;
    assert_eq!(tag.get()?, "", "a draft until Enter or focus loss");
    let before = driver.runtime().now();
    driver.clear_focus()?;
    assert_eq!(tag.get()?, "draft");
    assert_eq!(driver.runtime().focused(), None);
    assert_eq!(driver.runtime().now(), before + Driver::FRAME, "it settles");
    Ok(())
}

#[test]
fn clear_focus_returns_a_focus_loss_error() -> TestResult {
    let mut ui = runtime()?;
    let leaf = ui.mount_tree(Element::new("leaf", FailingFocusLoss))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&leaf)?;
    let mut driver = Driver::new(&mut ui);
    let error = driver.clear_focus().unwrap_err();
    assert!(is_failure(&error, "focus loss failed"), "{error:?}");
    assert_eq!(driver.runtime().focused(), None);
    Ok(())
}

#[test]
fn clear_focus_needs_no_neutral_input_and_leaves_it_neutral() -> TestResult {
    let mut ui = runtime()?;
    let mut driver = Driver::new(&mut ui);
    driver
        .runtime_mut()
        .command_key_input(CommandKeyInput::new(
            KeyCode::KeyA,
            KeyAction::Press,
            CommandKey::Named(KeyCode::KeyA),
        ))?;
    assert_eq!(driver.key(KeyCode::Tab), Err(TestError::InputHeld));
    driver.clear_focus()?;
    assert!(driver.runtime().keyboard().pressed().next().is_none());
    assert_eq!(
        driver.runtime().keyboard().modifiers(),
        Modifiers::default()
    );
    driver.key(KeyCode::Tab)?;
    Ok(())
}

/// What a setup helper returns, or why it failed.
type Setup<T> = Result<T, Box<dyn std::error::Error>>;
/// A count a component or callback shares with the test.
type Count = Rc<Cell<u32>>;
/// The cancellations a held-input registration received.
type Cancellations = Rc<RefCell<Vec<HeldCancellation>>>;

/// A runtime with a detached window whose text field owns a held-input
/// registration for F8 that logs its cancellations. Returns the field and
/// the detached window.
fn held_in_a_detached_window() -> Setup<(Runtime, WidgetId, NativeWindowId, Cancellations)> {
    let mut ui = runtime()?;
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("here", Label::new("Here")),
            Element::new(
                "panel",
                View::new(Element::new(
                    "field",
                    TextField::new(ui.signal(String::new())),
                )),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let panel = ui.find(&root, &["panel"])?.ok_or("panel")?;
    let prepared = ui.prepare_detach(&panel, NativeWindowOptions::new("Panel")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    ui.flush(Duration::ZERO)?;
    let field = ui.find(&root, &["panel", "field"])?.ok_or("field")?;
    let cancellations = Rc::new(RefCell::new(Vec::new()));
    let log = cancellations.clone();
    ui.register_held_input(
        &field,
        vec![KeyCode::F8],
        Modifiers::default(),
        |_, _, _| Ok(InputOutcome::Handled),
        move |_, reason| log.borrow_mut().push(reason),
    )?;
    Ok((ui, field, detached, cancellations))
}

#[test]
fn clear_focus_keeps_another_windows_focus_and_cancels_every_held_key() -> TestResult {
    let (mut ui, field, detached, cancellations) = held_in_a_detached_window()?;
    ui.select_window(&detached)?;
    ui.focus(&field)?;
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::F8,
        KeyAction::Press,
        CommandKey::Named(KeyCode::F8),
    ))?;
    assert!(cancellations.borrow().is_empty());
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    let mut driver = Driver::new(&mut ui);
    driver.clear_focus()?;
    // The selected window's reach: the detached window keeps its focus.
    driver.runtime_mut().select_window(&detached)?;
    assert_eq!(driver.runtime().focused(), Some(field));
    // The runtime-wide reach: the held registration was cancelled.
    assert_eq!(*cancellations.borrow(), [HeldCancellation::FocusLost]);
    Ok(())
}

// `Driver::quiet_for`.

const QUARTER: Duration = Duration::from_millis(250);

#[test]
fn a_spinning_spinner_is_busy_and_a_resting_one_idle() -> TestResult {
    let mut ui = runtime()?;
    let busy = ui.signal(true);
    ui.mount_tree(Element::new("busy", Spinner::new(busy.clone())))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let report = driver.quiet_for(QUARTER)?;
    assert!(report.components_updated >= 2, "{report:?}");
    assert!(report.next_deadline.is_some(), "{report:?}");
    assert!(report.needs_paint, "{report:?}");
    assert!(report.baseline_painted);
    assert!(!report.is_idle());
    busy.set(false)?;
    let report = driver.quiet_for(QUARTER)?;
    assert!(report.is_idle(), "{report:?}");
    Ok(())
}

#[test]
fn a_timer_the_test_installed_is_busy_until_it_is_cancelled() -> TestResult {
    let mut ui = runtime()?;
    let count = ui.signal(0_u32);
    let shown = count.map(|n| n.to_string())?;
    ui.mount_tree(Element::new("count", Label::new(shown)))?;
    let target = count.clone();
    let timer = ui.start_interval(Duration::from_millis(100), move |_| {
        target.update(|n| *n += 1)
    })?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let report = driver.quiet_for(QUARTER)?;
    assert_eq!(count.get()?, 2);
    assert!(report.components_updated >= 2, "{report:?}");
    assert!(report.needs_paint, "{report:?}");
    assert!(report.next_deadline.is_some(), "{report:?}");
    timer.cancel();
    let report = driver.quiet_for(QUARTER)?;
    assert!(report.is_idle(), "{report:?}");
    Ok(())
}

#[test]
fn a_toast_waiting_to_expire_keeps_a_deadline() -> TestResult {
    let mut ui = runtime()?;
    let queue = ToastController::new(&ui);
    ui.mount_tree(Element::new(
        "host",
        ToastHost::new(queue.clone(), Element::new("body", Label::new("Body"))),
    ))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let _toast = queue.push(Toast::new("Saved"))?;
    driver.settle()?;
    let report = driver.quiet_for(Duration::from_secs(1))?;
    assert_eq!(report.components_updated, 0, "{report:?}");
    assert!(!report.needs_paint, "{report:?}");
    assert!(report.next_deadline.is_some(), "{report:?}");
    assert!(!report.is_idle());
    driver.advance(Duration::from_secs(10))?;
    let report = driver.quiet_for(Duration::from_secs(1))?;
    assert!(report.is_idle(), "{report:?}");
    Ok(())
}

#[test]
fn another_windows_spinner_does_not_make_the_selected_window_busy() -> TestResult {
    let mut ui = runtime()?;
    let busy = ui.signal(true);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("here", Label::new("Here")),
            Element::new(
                "panel",
                View::new(Element::new("spinner", Spinner::new(busy.clone()))),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let panel = ui.find(&root, &["panel"])?.ok_or("panel")?;
    let prepared = ui.prepare_detach(&panel, NativeWindowOptions::new("Panel")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    ui.flush(Duration::ZERO)?;
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let report = driver.quiet_for(QUARTER)?;
    assert!(report.is_idle(), "{report:?}");
    driver.runtime_mut().select_window(&detached)?;
    let report = driver.quiet_for(QUARTER)?;
    assert!(report.components_updated >= 2, "{report:?}");
    assert!(report.next_deadline.is_some(), "{report:?}");
    Ok(())
}

#[test]
fn work_due_when_the_call_starts_runs_in_the_baseline_and_is_not_counted() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal(String::from("Before"));
    ui.mount_tree(Element::new("text", Label::new(text.clone())))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    text.set("After".to_owned())?;
    let report = driver.quiet_for(Duration::from_millis(100))?;
    assert!(report.is_idle(), "{report:?}");
    Ok(())
}

#[test]
fn the_clock_moves_by_exactly_the_interval() -> TestResult {
    let mut ui = runtime()?;
    let ticks = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new("ticker", Ticker(ticks.clone())))?;
    let mut driver = Driver::new(&mut ui);
    assert_eq!(driver.runtime().now(), Duration::ZERO);
    let report = driver.quiet_for(Duration::from_millis(40))?;
    assert_eq!(driver.runtime().now(), Duration::from_millis(40));
    // Flushes at 16, 32 and a final partial frame at 40 ms: the ticker,
    // which wakes a frame after each update, ticks at 16 and 32 ms.
    assert_eq!(report.components_updated, 2, "{report:?}");
    assert_eq!(report.next_deadline, Some(Duration::from_millis(48)));
    let report = driver.quiet_for(Duration::ZERO)?;
    assert_eq!(driver.runtime().now(), Duration::from_millis(40));
    assert_eq!(report.components_updated, 0, "{report:?}");
    assert!(
        report.next_deadline.is_some(),
        "the ticker is still scheduled"
    );
    Ok(())
}

/// A leaf whose paint fails with a clone of its error.
struct FailingPaint(DrawError);
impl Component for FailingPaint {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 20.0)?)
    }
    fn paint(&self, _: &mut Painter) -> Result<(), DrawError> {
        Err(self.0.clone())
    }
}

#[test]
fn a_paint_failure_is_a_draw_error_and_a_runtime_failure_keeps_its_form() -> TestResult {
    let mut ui = runtime()?;
    ui.mount_tree(Element::new("leaf", FailingPaint(DrawError::TooComplex)))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    assert_eq!(
        driver.quiet_for(Duration::ZERO),
        Err(TestError::Draw(DrawError::TooComplex))
    );

    let mut ui = runtime()?;
    ui.mount_tree(Element::new(
        "leaf",
        FailingPaint(DrawError::Runtime(failure("paint failed"))),
    ))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let error = driver.quiet_for(Duration::ZERO).unwrap_err();
    assert!(is_failure(&error, "paint failed"), "{error:?}");
    assert_eq!(error.to_string(), "paint failed");
    Ok(())
}

#[test]
fn a_components_own_pending_updates_is_its_paint_error() -> TestResult {
    let mut ui = runtime()?;
    ui.mount_tree(Element::new(
        "leaf",
        FailingPaint(DrawError::PendingUpdates),
    ))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    assert_eq!(
        driver.quiet_for(Duration::ZERO),
        Err(TestError::Draw(DrawError::PendingUpdates))
    );
    let site = driver.runtime().last_failure().ok_or("a failure site")?;
    assert_eq!(site.path, ["leaf"]);
    assert_eq!(site.phase, FailurePhase::Paint);
    Ok(())
}

type Shared<T> = Rc<RefCell<Option<T>>>;

/// Keeps the notifier of its first update for the test, and queues `command`
/// each time a notification is delivered.
struct QueuesOnNotification {
    notifier: Shared<Notifier>,
    command: CommandId,
}
impl Component for QueuesOnNotification {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.notifier
            .borrow_mut()
            .get_or_insert_with(|| cx.notifier());
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let context = cx.command_context(CommandSource::Programmatic);
        let _receipt = cx.invoke_command(&self.command, context)?;
        Ok(())
    }
}

/// A settled runtime whose next flush delivers a notification that queues a
/// command counting its runs, after that flush dispatched its commands.
fn queued_after_the_baseline_flush() -> Setup<(Runtime, Notifier, Count)> {
    let mut ui = runtime()?;
    let runs = Rc::new(Cell::new(0));
    let counter = runs.clone();
    let command =
        ui.register_command("Count", CommandScope::Application, vec![], move |_, _| {
            counter.set(counter.get() + 1);
            Ok(())
        })?;
    let notifier = Shared::default();
    ui.mount_tree(Element::new(
        "queues",
        QueuesOnNotification {
            notifier: notifier.clone(),
            command,
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    let notifier = notifier.borrow_mut().take().ok_or("the notifier")?;
    Ok((ui, notifier, runs))
}

#[test]
fn a_refused_baseline_is_activity_not_an_error() -> TestResult {
    let (mut ui, notifier, runs) = queued_after_the_baseline_flush()?;
    // Requested outside an input entry point, so no flush delivers it yet.
    notifier.request();
    let mut driver = Driver::new(&mut ui);
    let now = driver.runtime().now();
    let report = driver.quiet_for(Duration::ZERO)?;
    assert!(!report.baseline_painted, "{report:?}");
    assert_eq!(report.next_deadline, Some(now), "{report:?}");
    assert!(!report.is_idle());
    assert_eq!(runs.get(), 0, "the command is still queued");
    Ok(())
}

#[test]
fn a_refused_baseline_is_never_idle_even_when_the_end_needs_no_paint() -> TestResult {
    let (mut ui, notifier, runs) = queued_after_the_baseline_flush()?;
    ui.paint()?;
    notifier.request();
    let mut driver = Driver::new(&mut ui);
    let report = driver.quiet_for(Duration::from_millis(100))?;
    assert_eq!(runs.get(), 1, "the first step ran the command");
    assert_eq!(report.components_updated, 0, "{report:?}");
    assert_eq!(report.next_deadline, None, "{report:?}");
    assert!(!report.needs_paint, "{report:?}");
    assert!(!report.baseline_painted, "{report:?}");
    assert!(!report.is_idle());
    Ok(())
}

#[test]
fn an_overflowing_interval_is_refused_before_anything_runs() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal(String::from("Before"));
    ui.mount_tree(Element::new("text", Label::new(text.clone())))?;
    let mut driver = Driver::new(&mut ui);
    driver.advance(Duration::from_millis(1))?;
    text.set("After".to_owned())?;
    assert!(driver.runtime().needs_paint());
    assert_eq!(
        driver.quiet_for(Duration::MAX),
        Err(TestError::ClockOverflow)
    );
    assert_eq!(driver.runtime().now(), Duration::from_millis(1));
    assert!(
        driver.runtime().needs_paint(),
        "nothing was flushed or painted"
    );
    Ok(())
}

#[test]
fn a_steps_flush_error_ends_the_call() -> TestResult {
    let mut ui = runtime()?;
    let trigger = ui.signal(false);
    ui.mount_tree(Element::new(
        "watcher",
        Watcher {
            trigger: trigger.read_only(),
            failed: false,
        },
    ))?;
    let target = trigger.clone();
    let _timer = ui.start_interval(Duration::from_millis(100), move |_| target.set(true))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let error = driver.quiet_for(QUARTER).unwrap_err();
    assert!(is_failure(&error, "update failed"), "{error:?}");
    Ok(())
}

/// What a primary-window component does when its event timer elapses: queue
/// a command, or reveal the component the test names once it is mounted.
enum OnTimer {
    Command(CommandId),
    Reveal(Shared<WidgetId>),
}

/// Arms a 50 ms event timer when a notification is delivered, and counts its
/// own updates.
struct Arming {
    notifier: Shared<Notifier>,
    updates: Rc<Cell<u32>>,
    on_timer: OnTimer,
}
impl Component for Arming {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.updates.set(self.updates.get() + 1);
        self.notifier
            .borrow_mut()
            .get_or_insert_with(|| cx.notifier());
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        cx.set_timer(Duration::from_millis(50))
    }
    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        match &self.on_timer {
            OnTimer::Command(command) => {
                let context = cx.command_context(CommandSource::Programmatic);
                let _receipt = cx.invoke_command(command, context)?;
            }
            OnTimer::Reveal(target) => {
                let target = target
                    .borrow()
                    .clone()
                    .ok_or_else(|| failure("no target"))?;
                let _receipt = cx.request_reveal(&target)?;
            }
        }
        Ok(())
    }
}

/// An `Arming` component in the primary window and a `Ticker` in a detached
/// one, the primary selected and settled and the timer armed, due 50 ms
/// after the clock's zero. Returns the ticker's ticks, the `Arming` component's
/// updates and the ticker.
fn nested_flush_setup(ui: &mut Runtime, on_timer: OnTimer) -> Setup<(Count, Count, WidgetId)> {
    let ticks = Rc::new(Cell::new(0));
    let notifier = Shared::default();
    let updates = Rc::new(Cell::new(0));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "arming",
                Arming {
                    notifier: notifier.clone(),
                    updates: updates.clone(),
                    on_timer,
                },
            ),
            Element::new(
                "panel",
                View::new(Element::new("ticker", Ticker(ticks.clone()))),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let panel = ui.find(&root, &["panel"])?.ok_or("panel")?;
    let ticker = ui.find(&root, &["panel", "ticker"])?.ok_or("ticker")?;
    let prepared = ui.prepare_detach(&panel, NativeWindowOptions::new("Panel")?)?;
    let _detached = ui.commit_detach(prepared)?;
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    ui.flush(Duration::ZERO)?;
    // The delivery arms the timer, and settles presentation in every visible
    // window, so it runs before the measured interval.
    notifier.borrow().as_ref().ok_or("the notifier")?.request();
    ui.flush(Duration::ZERO)?;
    Ok((ticks, updates, ticker))
}

#[test]
fn a_commands_admission_in_a_step_runs_another_windows_due_work_uncounted() -> TestResult {
    let mut ui = runtime()?;
    let runs = Rc::new(Cell::new(0));
    let counter = runs.clone();
    let command = ui.register_command("Noop", CommandScope::Application, vec![], move |_, _| {
        counter.set(counter.get() + 1);
        Ok(())
    })?;
    let (ticks, updates, _) = nested_flush_setup(&mut ui, OnTimer::Command(command))?;
    let primary = ui.primary_window();
    let mut driver = Driver::new(&mut ui);
    let (ticks_before, updates_before) = (ticks.get(), updates.get());
    let report = driver.quiet_for(QUARTER)?;
    assert_eq!(runs.get(), 1, "the timer queued the command once");
    assert!(
        ticks.get() > ticks_before,
        "the admission flushed the detached window"
    );
    assert_eq!(
        report.components_updated,
        (updates.get() - updates_before) as usize,
        "{report:?}"
    );
    assert_eq!(report.next_deadline, None, "{report:?}");
    assert_eq!(driver.runtime().current_window(), primary);
    Ok(())
}

#[test]
fn a_queued_reveal_in_a_step_runs_another_windows_due_work_uncounted() -> TestResult {
    let mut ui = runtime()?;
    let target = Shared::default();
    let (ticks, updates, ticker) = nested_flush_setup(&mut ui, OnTimer::Reveal(target.clone()))?;
    *target.borrow_mut() = Some(ticker);
    let primary = ui.primary_window();
    let mut driver = Driver::new(&mut ui);
    let (ticks_before, updates_before) = (ticks.get(), updates.get());
    let report = driver.quiet_for(QUARTER)?;
    assert!(
        ticks.get() > ticks_before,
        "the reveal flushed the detached window"
    );
    assert_eq!(
        report.components_updated,
        (updates.get() - updates_before) as usize,
        "{report:?}"
    );
    assert_eq!(report.next_deadline, None, "{report:?}");
    assert_eq!(driver.runtime().current_window(), primary);
    Ok(())
}
