//! What an application callback reaches for: `?` on the library's
//! higher-level errors, the address lookup, typed capture targets, command
//! buttons and the named-key chord.
use pert_ui::*;
use std::{cell::RefCell, error::Error, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn Error>>;

fn ui() -> Result<Runtime, Box<dyn Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    Ok(ui)
}

fn press(ui: &mut Runtime, code: KeyCode) -> Result<(), RuntimeError> {
    ui.key_input(code, KeyAction::Press)?;
    ui.key_input(code, KeyAction::Release)?;
    Ok(())
}

#[test]
fn a_command_action_propagates_a_workspace_error_with_question_mark() -> TestResult {
    let mut ui = ui()?;
    let restore = ui.register_command(
        "Restore layout",
        CommandScope::Application,
        vec![],
        |ui, _| {
            let workspace = Workspace::decode("not a workspace")?;
            let _report = ui.restore_workspace(&workspace)?;
            Ok(())
        },
    )?;
    let context = ui.command_context(CommandSource::Programmatic);
    let error = ui.invoke_command(&restore, context).unwrap_err();
    let RuntimeError::Application(failure) = &error else {
        panic!("an application failure: {error:?}");
    };
    let workspace = failure
        .downcast_ref::<WorkspaceError>()
        .ok_or("a workspace error")?;
    assert!(matches!(workspace, WorkspaceError::Malformed(_)));
    // Transparent: the message is the workspace error's.
    assert_eq!(error.to_string(), workspace.to_string());
    // Through the facade's application it is a runtime failure.
    #[cfg(feature = "native")]
    assert!(matches!(
        AppError::from(error.clone()),
        AppError::Runtime(RuntimeError::Application(_))
    ));
    Ok(())
}

#[test]
fn a_held_runtime_failure_keeps_its_one_form() {
    let runtime = RuntimeError::Closed;
    assert_eq!(
        RuntimeError::from(WorkspaceError::Runtime(runtime.clone())),
        runtime
    );
    assert_eq!(
        RuntimeError::from(WindowError::Runtime(runtime.clone())),
        runtime
    );
    assert_eq!(
        RuntimeError::from(CaptureError::Runtime(runtime.clone())),
        runtime
    );
    // A drawing failure inside a window error stays the whole window error.
    let window = RuntimeError::from(WindowError::Drawing(DrawError::InvalidColor));
    let RuntimeError::Application(failure) = window else {
        panic!("an application failure");
    };
    assert_eq!(
        failure.downcast_ref::<WindowError>(),
        Some(&WindowError::Drawing(DrawError::InvalidColor))
    );
    let capture = RuntimeError::from(CaptureError::Busy);
    let RuntimeError::Application(failure) = capture else {
        panic!("an application failure");
    };
    assert_eq!(
        failure.downcast_ref::<CaptureError>(),
        Some(&CaptureError::Busy)
    );
}

#[test]
fn find_takes_the_path_inspection_reports() -> TestResult {
    let mut ui = ui()?;
    let root = ui.mount_tree(Element::new(
        "workspace",
        RegionLayout::new()
            .top(Element::new("tools", Label::new("Tools")))
            .center(Element::new(
                "mixer",
                Column::new([
                    Element::new(
                        "left",
                        Row::new([Element::new("fader", Label::new("Left"))]),
                    ),
                    Element::new(
                        "right",
                        Row::new([Element::new("fader", Label::new("Right"))]),
                    ),
                ]),
            )),
    ))?;
    ui.flush(Duration::ZERO)?;
    let path = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| {
            component.key == "fader"
                && component
                    .path
                    .as_ref()
                    .is_some_and(|path| path.contains(&"right".to_owned()))
        })
        .and_then(|component| component.path)
        .ok_or("inspected")?;
    // The inspection path starts with the root's key and names the region.
    assert_eq!(path[0], "workspace");
    assert!(path.contains(&"center".to_owned()));
    let below: Vec<&str> = path[1..].iter().map(String::as_str).collect();
    let fader = ui.find(&root, &below)?.ok_or("found")?;
    assert_eq!(ui.key(&fader)?, "fader");
    // The full internal path needs the structure the address skips.
    assert!(ui.find_path(&root, &below)?.is_none());
    assert!(ui.find(&root, &["center", "mixer", "missing"])?.is_none());
    // Two faders: one key alone is ambiguous, an address is not.
    assert_eq!(
        ui.find_descendant(&root, "fader"),
        Err(RuntimeError::AmbiguousKey)
    );
    Ok(())
}

#[test]
fn a_window_target_is_built_from_its_handle() -> TestResult {
    let mut ui = ui()?;
    let main = ui.primary_window();
    assert_eq!(
        CaptureTarget::window(&main),
        CaptureTarget::Window(main.id())
    );
    let area = Rect::new(10.0, 20.0, 30.0, 40.0)?;
    assert_eq!(
        CaptureTarget::region(&main, area),
        CaptureTarget::Region {
            window: main.id(),
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0
        }
    );
    // A window that has closed is unknown.
    let view = ui.mount_tree(Element::new("panel", Label::new("Panel")))?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Panel")?)?;
    let panel = ui.commit_detach(prepared)?;
    ui.flush_windows(Duration::ZERO)?;
    let target = CaptureTarget::window(&panel);
    ui.redock(&panel)?;
    ui.flush_windows(Duration::ZERO)?;
    ui.set_capture_support(true);
    let reply = ui.request_capture(CaptureRequest::new(target));
    assert_eq!(reply.try_reply()?.value, Err(CaptureError::UnknownTarget));
    Ok(())
}

#[test]
fn a_named_chord_routes_like_its_long_form() -> TestResult {
    let mut ui = ui()?;
    assert_eq!(
        KeyChord::named(KeyCode::Space),
        KeyChord::new(CommandKey::Named(KeyCode::Space), Modifiers::default())
    );
    let ran = ui.signal(0u32);
    let count = ran.clone();
    ui.register_command(
        "Play",
        CommandScope::Application,
        vec![KeyChord::named(KeyCode::F5)],
        move |_, _| count.set(count.get()? + 1),
    )?;
    ui.mount_tree(Element::new("content", Label::new("Content")))?;
    ui.flush(Duration::ZERO)?;
    // Commands route keys that a host delivers with their layout identity.
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::F5,
            action,
            CommandKey::Named(KeyCode::F5),
        ))?;
    }
    ui.flush(Duration::ZERO)?;
    assert_eq!(ran.get()?, 1);
    Ok(())
}

type Receipts = Rc<RefCell<Vec<CommandReceipt>>>;

fn recording(
    receipts: &Receipts,
) -> impl FnMut(&mut EventContext, CommandReceipt) -> Result<(), RuntimeError> + 'static {
    let receipts = receipts.clone();
    move |_, receipt| {
        receipts.borrow_mut().push(receipt);
        Ok(())
    }
}

fn finished(receipt: &CommandReceipt) -> Option<Result<CommandOutcome, RuntimeError>> {
    match receipt.status() {
        CommandStatus::Finished(result) => Some(result),
        _ => None,
    }
}

#[test]
fn a_command_button_invokes_its_command_by_pointer_and_keyboard() -> TestResult {
    let mut ui = ui()?;
    let ran = ui.signal(0u32);
    let count = ran.clone();
    let play = ui.register_command("Play", CommandScope::Application, vec![], move |_, _| {
        count.set(count.get()? + 1)
    })?;
    let receipts = Receipts::default();
    let button = ui.mount_tree(Element::new(
        "play",
        Button::command(&ui, &play, recording(&receipts))?,
    ))?;
    ui.flush(Duration::ZERO)?;
    // The caption is the command's name.
    let name = ui.inspect().components[0]
        .details
        .semantics
        .clone()
        .and_then(|semantics| semantics.name);
    assert_eq!(name.as_deref(), Some("Play"));
    ui.focus(&button)?;
    press(&mut ui, KeyCode::Enter)?;
    assert_eq!(receipts.borrow().len(), 1);
    assert_eq!(receipts.borrow()[0].status(), CommandStatus::Pending);
    ui.flush(Duration::ZERO)?;
    assert_eq!(ran.get()?, 1);
    assert_eq!(
        finished(&receipts.borrow()[0]),
        Some(Ok(CommandOutcome::Invoked))
    );
    let [x, y, w, h] = ui.bounds(&button)?.ok_or("laid out")?.values();
    let at = Point::new(x + w / 2.0, y + h / 2.0)?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ran.get()?, 2);
    assert_eq!(receipts.borrow().len(), 2);
    Ok(())
}

#[test]
fn a_command_button_follows_the_command_binding_and_its_own_restriction() -> TestResult {
    let mut ui = ui()?;
    let available = ui.signal(true);
    let allowed = ui.signal(true);
    let play = ui.register_command("Play", CommandScope::Application, vec![], |_, _| Ok(()))?;
    ui.command_enabled_signal(&play, available.clone())?;
    let receipts = Receipts::default();
    let button = Button::command(&ui, &play, recording(&receipts))?.enabled(allowed.clone());
    let id = ui.mount_tree(Element::new("play", button))?;
    ui.flush(Duration::ZERO)?;
    let disabled = |ui: &Runtime| {
        ui.inspect().components[0]
            .details
            .semantics
            .as_ref()
            .is_some_and(|semantics| semantics.disabled)
    };
    assert!(!disabled(&ui));
    available.set(false)?;
    ui.flush(Duration::ZERO)?;
    assert!(disabled(&ui));
    available.set(true)?;
    allowed.set(false)?;
    ui.flush(Duration::ZERO)?;
    assert!(disabled(&ui), "both restrictions apply");
    allowed.set(true)?;
    ui.flush(Duration::ZERO)?;
    // Revoking the binding while the button is armed cancels the activation.
    let [x, y, w, h] = ui.bounds(&id)?.ok_or("laid out")?.values();
    let at = Point::new(x + w / 2.0, y + h / 2.0)?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    available.set(false)?;
    ui.flush(Duration::ZERO)?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert!(receipts.borrow().is_empty());
    Ok(())
}

#[test]
fn refusals_and_failures_arrive_in_receipts_once() -> TestResult {
    let mut ui = ui()?;
    let calls = ui.signal(0u32);
    let count = calls.clone();
    let failing = ui.register_command("Fail", CommandScope::Application, vec![], move |_, _| {
        count.set(count.get()? + 1)?;
        Err(ApplicationError::from("the engine is offline").into())
    })?;
    let contextual =
        ui.register_command("Contextual", CommandScope::Application, vec![], |_, _| {
            Ok(())
        })?;
    ui.command_enabled(&contextual, |_, _| Ok(false))?;
    let receipts = Receipts::default();
    let root = ui.mount_tree(Element::new(
        "tools",
        Column::new([
            Element::new(
                "fail",
                Button::command(&ui, &failing, recording(&receipts))?,
            ),
            Element::new(
                "contextual",
                Button::command(&ui, &contextual, recording(&receipts))?,
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let fail = ui.find(&root, &["fail"])?.ok_or("mounted")?;
    ui.focus(&fail)?;
    press(&mut ui, KeyCode::Enter)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        finished(&receipts.borrow()[0])
            .ok_or("finished")?
            .unwrap_err()
            .to_string(),
        "the engine is offline"
    );
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(calls.get()?, 1, "not replayed");
    // A contextual predicate leaves the button enabled and refuses in the receipt.
    let refused = ui.find(&root, &["contextual"])?.ok_or("mounted")?;
    ui.focus(&refused)?;
    press(&mut ui, KeyCode::Enter)?;
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(
        finished(&receipts.borrow()[1]),
        Some(Ok(CommandOutcome::Disabled))
    );
    // An unregistered command answers through the receipt; a stale one cannot
    // build a button.
    ui.unregister_command(&failing)?;
    ui.focus(&fail)?;
    press(&mut ui, KeyCode::Enter)?;
    ui.flush(Duration::from_millis(1))?;
    assert!(matches!(
        finished(&receipts.borrow()[2]),
        Some(Err(RuntimeError::StaleCommand))
    ));
    assert!(matches!(
        Button::command(&ui, &failing, |_, _| Ok(())),
        Err(RuntimeError::StaleCommand)
    ));
    Ok(())
}

#[test]
fn a_failing_receipt_handler_is_reported_once_and_a_new_description_replaces_it() -> TestResult {
    let mut ui = ui()?;
    let play = ui.register_command("Play", CommandScope::Application, vec![], |_, _| Ok(()))?;
    let receipts = Receipts::default();
    let describe = |ui: &Runtime, fail: bool| -> Result<Vec<Element>, RuntimeError> {
        let receipts = receipts.clone();
        Ok(vec![Element::new(
            "play",
            Button::command(ui, &play, move |_, receipt| {
                if fail {
                    return Err(ApplicationError::from("the readout is gone").into());
                }
                receipts.borrow_mut().push(receipt);
                Ok(())
            })?,
        )])
    };
    let root = ui.mount_tree(Element::new("tools", Column::new(describe(&ui, true)?)))?;
    ui.flush(Duration::ZERO)?;
    let button = ui.find(&root, &["play"])?.ok_or("mounted")?;
    ui.focus(&button)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    let error = ui
        .key_input(KeyCode::Enter, KeyAction::Release)
        .unwrap_err();
    assert_eq!(error.to_string(), "the readout is gone");
    ui.flush(Duration::ZERO)?;
    ui.reconcile_children(&root, describe(&ui, false)?)?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&button)?;
    press(&mut ui, KeyCode::Enter)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(receipts.borrow().len(), 1);
    Ok(())
}

#[test]
fn a_modal_dialog_refuses_a_command_through_the_receipt() -> TestResult {
    let mut ui = ui()?;
    let play = ui.register_command("Play", CommandScope::Application, vec![], |_, _| Ok(()))?;
    let receipts = Receipts::default();
    let dialog = DialogController::new(&ui);
    let root = ui.mount_tree(Element::new(
        "owner",
        Dialog::new(
            dialog.clone(),
            Element::new("anchor", Label::new("Main")),
            Element::new("play", Button::command(&ui, &play, recording(&receipts))?),
        ),
    ))?;
    dialog.open()?;
    ui.flush(Duration::ZERO)?;
    let button = ui.find_descendant(&root, "play")?.ok_or("mounted")?;
    ui.focus(&button)?;
    press(&mut ui, KeyCode::Enter)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        finished(&receipts.borrow()[0]),
        Some(Ok(CommandOutcome::Blocked))
    );
    Ok(())
}

#[test]
fn a_command_button_captures_the_context_at_activation() -> TestResult {
    let mut ui = ui()?;
    let seen = Rc::new(RefCell::new(None));
    let record = seen.clone();
    let play = ui.register_command(
        "Play",
        CommandScope::Application,
        vec![],
        move |ui, context| {
            let target = context
                .target
                .as_ref()
                .map(|id| ui.key(id).map(str::to_owned))
                .transpose()?;
            let focused = context
                .focused
                .as_ref()
                .map(|id| ui.key(id).map(str::to_owned))
                .transpose()?;
            *record.borrow_mut() = Some((context.source, target, focused));
            Ok(())
        },
    )?;
    let root = ui.mount_tree(Element::new(
        "window",
        Column::new([
            Element::new("document", Label::new("Document")),
            Element::new("play", Button::command(&ui, &play, |_, _| Ok(()))?),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let document = ui.find(&root, &["document"])?.ok_or("mounted")?;
    ui.set_command_target(Some(&document))?;
    let button = ui.find(&root, &["play"])?.ok_or("mounted")?;
    ui.focus(&button)?;
    press(&mut ui, KeyCode::Enter)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        seen.borrow().clone(),
        Some((
            CommandSource::Button,
            Some("document".to_owned()),
            Some("play".to_owned())
        ))
    );
    Ok(())
}

#[test]
fn a_new_description_replaces_the_buttons_own_restriction() -> TestResult {
    let mut ui = ui()?;
    let play = ui.register_command("Play", CommandScope::Application, vec![], |_, _| Ok(()))?;
    let describe = |ui: &Runtime, allowed: bool| -> Result<Vec<Element>, RuntimeError> {
        Ok(vec![Element::new(
            "play",
            Button::command(ui, &play, |_, _| Ok(()))?.enabled(allowed),
        )])
    };
    let root = ui.mount_tree(Element::new("tools", Column::new(describe(&ui, false)?)))?;
    ui.flush(Duration::ZERO)?;
    let disabled = |ui: &Runtime| {
        ui.inspect()
            .components
            .iter()
            .find(|component| component.key == "play")
            .and_then(|component| component.details.semantics.as_ref())
            .is_some_and(|semantics| semantics.disabled)
    };
    assert!(disabled(&ui));
    ui.reconcile_children(&root, describe(&ui, true)?)?;
    ui.flush(Duration::ZERO)?;
    assert!(
        !disabled(&ui),
        "the denial was the description's own, and is replaced"
    );
    Ok(())
}

/// An application error with a cause of its own.
#[derive(Debug)]
struct SaveFailed(std::io::Error);
impl std::fmt::Display for SaveFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("saving failed")
    }
}
impl Error for SaveFailed {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
fn an_error_with_a_cause_of_its_own_keeps_it_in_the_chain() -> TestResult {
    let mut ui = ui()?;
    let save = ui.register_command("Save", CommandScope::Application, vec![], |_, _| {
        Err(ApplicationError::new(SaveFailed(std::io::Error::other("disk full"))).into())
    })?;
    let context = ui.command_context(CommandSource::Programmatic);
    let error = ui.invoke_command(&save, context).unwrap_err();
    let mut chain = Vec::new();
    let mut link: Option<&(dyn Error + 'static)> = Some(&error);
    while let Some(current) = link {
        chain.push(current.to_string());
        link = current.source();
    }
    assert!(
        chain.iter().any(|message| message == "disk full"),
        "{chain:?}"
    );
    let RuntimeError::Application(failure) = &error else {
        panic!("an application failure: {error:?}");
    };
    assert!(failure.downcast_ref::<SaveFailed>().is_some());
    Ok(())
}

#[test]
fn a_full_command_queue_is_the_command_buttons_error() -> TestResult {
    let mut ui = ui()?;
    let play = ui.register_command("Play", CommandScope::Application, vec![], |_, _| Ok(()))?;
    let receipts: Receipts = Rc::default();
    let root = ui.mount_tree(Element::new(
        "window",
        Column::new([Element::new(
            "play",
            Button::command(&ui, &play, recording(&receipts))?,
        )]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let button = ui.find(&root, &["play"])?.ok_or("mounted")?;
    ui.focus(&button)?;
    // Arm the button, then fill the queue from an event on the same button,
    // which keeps its focus and so the press.
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.flush(Duration::ZERO)?;
    ui.try_event(&button, |cx| {
        for _ in 0..Runtime::MAX_QUEUED_COMMANDS {
            let _queued =
                cx.invoke_command(&play, cx.command_context(CommandSource::Programmatic))?;
        }
        Ok::<_, RuntimeError>(())
    })??;
    assert_eq!(
        ui.key_input(KeyCode::Enter, KeyAction::Release).err(),
        Some(RuntimeError::CommandQueueFull)
    );
    assert!(
        receipts.borrow().is_empty(),
        "no receipt for a refused activation"
    );
    Ok(())
}
