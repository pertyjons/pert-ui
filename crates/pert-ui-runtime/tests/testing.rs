//! Contracts of the headless test driver, `testing::Driver`, and its
//! `TestError`: each action's effect, each refusal, and the cleanup that
//! releases what an action pressed when a step fails.
use pert_ui_runtime::testing::*;
use pert_ui_runtime::{
    ApplicationError, CommandKey, KeyEvent, KeyTransition, KeyboardState, NativeWindowOptions,
    ReadSignal, RevealRefusal, SemanticValue, Show, Signal, Tab, Tabs, TextCommand,
    TextCommandOutcome, component::*, prelude::*,
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
    let _ticker = ui.mount("ticker", Ticker(ticks.clone()))?;
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
fn click_refuses_another_window_a_refused_reveal_and_no_bounds() -> TestResult {
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
    let legacy = ui.mount(
        "legacy",
        Target {
            releases: Rc::default(),
            fail_release: false,
        },
    )?;
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
    // A root mounted without a tree has no allocation to click.
    assert_eq!(
        driver.click(&legacy),
        Err(TestError::NotLaidOut {
            key: "legacy".into()
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
