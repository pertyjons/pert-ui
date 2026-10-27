//! Public routing, identity and sustained-key contracts across window roots.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

fn key(
    ui: &mut Runtime,
    physical: KeyCode,
    logical: char,
    action: KeyAction,
) -> Result<KeyDispatch, RuntimeError> {
    ui.command_key_input(CommandKeyInput::new(
        physical,
        action,
        CommandKey::Character(logical),
    ))
}
fn primary(ui: &mut Runtime) -> Result<(), RuntimeError> {
    ui.native_modifiers(Modifiers::PRIMARY)
}
fn windows(ui: &mut Runtime) -> Result<Vec<(NativeWindowId, WidgetId)>, RuntimeError> {
    let mut result = Vec::new();
    for index in 0..3 {
        ui.select_window(&ui.primary_window())?;
        let view = ui.mount_tree(Element::new(
            format!("view-{index}"),
            View::new(Element::new("body", Label::new("Editor"))),
        ))?;
        ui.flush(Duration::ZERO)?;
        let window = if index == 0 {
            ui.primary_window()
        } else {
            let prepared =
                ui.prepare_detach(&view, NativeWindowOptions::new(format!("Editor {index}"))?)?;
            ui.commit_detach(prepared)?
        };
        result.push((window, view));
    }
    ui.flush_windows(Duration::ZERO)?;
    Ok(result)
}
#[test]
fn three_unfocused_windows_route_application_once_and_window_locally() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let roots = windows(&mut ui)?;
    let opened = ui.signal(0);
    let written = opened.clone();
    ui.register_command(
        "Open",
        CommandScope::Application,
        vec![KeyChord::primary('o')],
        move |_, _| written.set(written.get()? + 1),
    )?;
    for (window, _) in &roots {
        ui.select_window(window)?;
        assert!(ui.focused().is_none());
        primary(&mut ui)?;
        assert!(key(&mut ui, KeyCode::KeyO, 'o', KeyAction::Press)?.handled);
        key(&mut ui, KeyCode::KeyO, 'o', KeyAction::Release)?;
    }
    assert_eq!(opened.get()?, 3);
    let counts = ui.signal([0, 0, 0]);
    for (index, (window, _)) in roots.iter().enumerate() {
        let counts = counts.clone();
        ui.register_command(
            "Delete",
            CommandScope::Window(window.clone()),
            vec![KeyChord::new(
                CommandKey::Named(KeyCode::Delete),
                Modifiers::default(),
            )],
            move |_, _| {
                let mut value = counts.get()?;
                value[index] += 1;
                counts.set(value)
            },
        )?;
    }
    ui.native_modifiers(Modifiers::default())?;
    ui.select_window(&roots[1].0)?;
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::Delete,
        KeyAction::Press,
        CommandKey::Named(KeyCode::Delete),
    ))?;
    assert_eq!(counts.get()?, [0, 1, 0]);
    Ok(())
}
#[test]
fn layout_identity_disabled_reservation_rebinding_and_conflicts() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let roots = windows(&mut ui)?;
    ui.select_window(&roots[1].0)?;
    let count = ui.signal(0);
    let written = count.clone();
    let command = ui.register_command(
        "Undo",
        CommandScope::Application,
        vec![KeyChord::primary('z')],
        move |_, _| written.set(written.get()? + 1),
    )?;
    let local = ui.register_command(
        "Editor undo",
        CommandScope::View(roots[1].1.clone()),
        vec![KeyChord::primary('z')],
        |_, _| panic!("disabled action"),
    )?;
    ui.command_enabled(&local, |_, _| Ok(false))?;
    primary(&mut ui)?;
    key(&mut ui, KeyCode::KeyW, 'Z', KeyAction::Press)?; // AZERTY position, Caps Lock
    key(&mut ui, KeyCode::KeyW, 'Z', KeyAction::Release)?;
    assert_eq!(count.get()?, 0);
    ui.unregister_command(&local)?;
    key(&mut ui, KeyCode::KeyW, 'z', KeyAction::Press)?;
    key(&mut ui, KeyCode::KeyW, 'z', KeyAction::Release)?;
    assert_eq!(count.get()?, 1);
    assert_eq!(
        ui.register_command(
            "Duplicate",
            CommandScope::Application,
            vec![KeyChord::primary('Z')],
            |_, _| Ok(())
        ),
        Err(RuntimeError::ShortcutConflict(KeyChord::primary('Z')))
    );
    ui.rebind_command(&command, vec![KeyChord::primary('y')])?;
    key(&mut ui, KeyCode::KeyZ, 'y', KeyAction::Press)?;
    assert_eq!(count.get()?, 2);
    Ok(())
}
#[test]
fn empty_text_undo_reserves_layout_chord_and_custom_text_phase_runs_once()
-> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())?;
    let text = ui.signal("Empty history".to_owned());
    let field = ui.mount_tree(Element::new("text", TextField::new(text)))?;
    ui.register_command(
        "Host undo",
        CommandScope::Application,
        vec![KeyChord::primary('z')],
        |_, _| panic!("host undo escaped text"),
    )?;
    ui.focus(&field)?;
    primary(&mut ui)?;
    assert!(key(&mut ui, KeyCode::KeyW, 'z', KeyAction::Press)?.handled);
    Ok(())
}
#[test]
fn orphan_repeats_never_reestablish_pressed_state_or_trigger_new_command()
-> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.register_command(
        "Preview",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Character('a'),
            Modifiers::default(),
        )],
        |_, _| panic!("orphan repeat"),
    )?;
    ui.clear_focus();
    for _ in 0..4 {
        assert!(key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Repeat)?.handled);
        assert!(!ui.keyboard().is_pressed(KeyCode::KeyA));
    }
    Ok(())
}
#[test]
fn removed_command_consumes_its_remaining_repeats_and_release() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let id = ui.register_command(
        "Once",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Character('a'),
            Modifiers::default(),
        )],
        |_, _| Ok(()),
    )?;
    ui.command_repeats(&id, true)?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Press)?;
    ui.unregister_command(&id)?;
    ui.register_command(
        "Replacement",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Character('a'),
            Modifiers::default(),
        )],
        |_, _| panic!("stale repeat retargeted"),
    )?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Repeat)?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Release)?;
    Ok(())
}
#[test]
fn captured_menu_target_is_not_retargeted_after_movement_or_hiding() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let roots = windows(&mut ui)?;
    ui.select_window(&roots[1].0)?;
    let id = ui.register_command("Menu", CommandScope::Application, vec![], |_, _| Ok(()))?;
    let context = ui.command_context(CommandSource::Menu);
    ui.redock(&roots[1].0)?;
    assert_eq!(
        ui.invoke_command(&id, context),
        Err(RuntimeError::StaleCommandTarget)
    );
    ui.set_command_target(Some(&roots[1].1))?;
    let context = ui.command_context(CommandSource::Button);
    ui.set_visible(&roots[1].1, false)?;
    assert_eq!(
        ui.invoke_command(&id, context),
        Err(RuntimeError::StaleCommandTarget)
    );
    let mut other = Runtime::new();
    let other_id =
        other.register_command("Other", CommandScope::Application, vec![], |_, _| Ok(()))?;
    assert_eq!(
        ui.command_info(&other_id, &ui.command_context(CommandSource::Programmatic)),
        Err(RuntimeError::StaleCommand)
    );
    Ok(())
}
#[test]
fn held_input_has_one_terminal_outcome_and_never_leaks_after_focus_loss() -> Result<(), RuntimeError>
{
    let mut ui = Runtime::new();
    let roots = windows(&mut ui)?;
    ui.select_window(&roots[1].0)?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let cancellations = Rc::new(RefCell::new(Vec::new()));
    let written = events.clone();
    let cancelled = cancellations.clone();
    let registration = ui.register_held_input(
        &roots[1].1,
        vec![KeyCode::KeyA],
        Modifiers::default(),
        move |key, event, _| {
            written.borrow_mut().push((key, event));
            Ok(InputOutcome::Handled)
        },
        move |key, reason| cancelled.borrow_mut().push((key, reason)),
    )?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Press)?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Repeat)?;
    ui.clear_focus();
    ui.select_window(&roots[2].0)?;
    for _ in 0..3 {
        key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Repeat)?;
    }
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Release)?;
    ui.unregister_held_input(&registration)?;
    assert_eq!(
        *events.borrow(),
        vec![
            (KeyCode::KeyA, KeyTransition::Pressed),
            (KeyCode::KeyA, KeyTransition::Repeated)
        ]
    );
    assert_eq!(
        *cancellations.borrow(),
        vec![(KeyCode::KeyA, HeldCancellation::FocusLost)]
    );
    Ok(())
}
#[test]
fn held_release_removes_ownership_before_failing_callback() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let owner = ui.mount_tree(Element::new("view", Label::new("Owner")))?;
    let cancellations = ui.signal(0);
    let written = cancellations.clone();
    ui.register_held_input(
        &owner,
        vec![KeyCode::KeyA],
        Modifiers::default(),
        |_, event, _| {
            if event == KeyTransition::Released {
                Err(RuntimeError::StalePresentation)
            } else {
                Ok(InputOutcome::Handled)
            }
        },
        move |_, _| {
            written.set(written.get().unwrap() + 1).unwrap();
        },
    )?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Press)?;
    assert_eq!(
        key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Release),
        Err(RuntimeError::StalePresentation)
    );
    ui.clear_focus();
    assert_eq!(cancellations.get()?, 0);
    Ok(())
}

#[test]
fn application_modal_blocks_other_roots_and_preserves_its_editor_input() -> Result<(), RuntimeError>
{
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())?;
    let state = DialogController::new(&ui);
    let text = ui.signal("Dialog draft".to_owned());
    let root = ui.mount_tree(Element::new(
        "owner",
        Dialog::new(
            state.clone(),
            Element::new("anchor", Label::new("Main")),
            Element::new("editor", TextField::new(text.clone())),
        )
        .application_modal(),
    ))?;
    let view = ui.mount_tree(Element::new("other", Label::new("Other window")))?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Other")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.flush_windows(Duration::ZERO)?;
    let count = ui.signal(0);
    let written = count.clone();
    let command = ui.register_command(
        "Modal action",
        CommandScope::Application,
        vec![KeyChord::primary('o')],
        move |_, _| written.set(written.get()? + 1),
    )?;
    ui.command_in_modals(&command, true)?;
    let stale_menu = ui.command_context(CommandSource::Menu);
    ui.select_window(&ui.primary_window())?;
    state.open()?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(state.status().get()?, DialogStatus::Open);
    assert_eq!(
        ui.invoke_command(&command, stale_menu)?,
        CommandOutcome::Blocked
    );
    ui.select_window(&detached)?;
    primary(&mut ui)?;
    key(&mut ui, KeyCode::KeyO, 'o', KeyAction::Press)?;
    key(&mut ui, KeyCode::KeyO, 'o', KeyAction::Release)?;
    assert_eq!(count.get()?, 0);
    ui.select_window(&ui.primary_window())?;
    assert!(ui.focused().is_some());
    primary(&mut ui)?;
    key(&mut ui, KeyCode::KeyO, 'o', KeyAction::Press)?;
    key(&mut ui, KeyCode::KeyO, 'o', KeyAction::Release)?;
    assert_eq!(count.get()?, 1);
    ui.native_modifiers(Modifiers::default())?;
    let mut input =
        CommandKeyInput::new(KeyCode::KeyA, KeyAction::Press, CommandKey::Character('a'));
    input.has_text = true;
    assert!(ui.command_key_input(input)?.deliver_text);
    ui.text_input("a")?;
    assert!(text.get()?.contains('a'));
    assert_eq!(
        ui.prepare_detach(&root, NativeWindowOptions::new("Blocked")?)
            .err(),
        Some(RuntimeError::ModalBlocked)
    );
    state.cancel()?;
    ui.flush_windows(Duration::ZERO)?;
    ui.select_window(&detached)?;
    assert_eq!(
        ui.invoke_command(&command, ui.command_context(CommandSource::Button))?,
        CommandOutcome::Invoked
    );
    Ok(())
}
#[test]
fn view_pointer_selects_target_without_focus_or_registered_view_bindings()
-> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())?;
    let root = ui.mount_tree(Element::new(
        "rows",
        Column::new([
            Element::new(
                "first",
                View::new(Element::new("first-label", Label::new("First editor"))),
            ),
            Element::new(
                "second",
                View::new(Element::new("second-label", Label::new("Second editor"))),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let second = ui.children(&root)?[1].clone();
    let [x, y, w, h] = ui.bounds(&second)?.unwrap().values();
    ui.pointer_input(PointerEvent::new(
        Point::new(x + w / 2.0, y + h / 2.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))?;
    assert_eq!(ui.command_context(CommandSource::Menu).target, Some(second));
    Ok(())
}
#[test]
fn queued_button_uses_same_disabled_and_modal_admission_path() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let owner = ui.mount_tree(Element::new("owner", Label::new("Owner")))?;
    let id = ui.register_command("Disabled", CommandScope::Application, vec![], |_, _| {
        panic!("disabled button invoked")
    })?;
    let enabled = ui.signal(false);
    ui.command_enabled_signal(&id, enabled.read_only())?;
    let receipt = ui.try_event(&owner, |cx| {
        cx.invoke_command(&id, cx.command_context(CommandSource::Button))
    })??;
    assert_eq!(receipt.status_binding().get()?, CommandStatus::Pending);
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        receipt.status(),
        CommandStatus::Finished(Ok(CommandOutcome::Disabled))
    );
    Ok(())
}

#[test]
fn queued_commands_schedule_work_and_revalidate_modals_between_actions() -> Result<(), RuntimeError>
{
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())?;
    let state = DialogController::new(&ui);
    let owner = ui.mount_tree(Element::new(
        "owner",
        Dialog::new(
            state.clone(),
            Element::new("anchor", Label::new("Content")),
            Element::new("dialog-content", Label::new("Modal")),
        ),
    ))?;
    ui.flush(Duration::ZERO)?;
    let opening = state.clone();
    let open = ui.register_command("Open", CommandScope::Application, vec![], move |_, _| {
        opening.open()
    })?;
    let blocked = ui.register_command(
        "Old menu action",
        CommandScope::Application,
        vec![],
        |_, _| panic!("queued action escaped modal"),
    )?;
    let context = ui.command_context(CommandSource::Menu);
    let (first, second) = ui.try_event(&owner, |cx| {
        Ok::<_, RuntimeError>((
            cx.invoke_command(&open, context.clone())?,
            cx.invoke_command(&blocked, context)?,
        ))
    })??;
    assert_eq!(ui.application_deadline(), Some(ui.now()));
    ui.flush_windows(ui.application_deadline().unwrap())?;
    assert_eq!(
        first.status(),
        CommandStatus::Finished(Ok(CommandOutcome::Invoked))
    );
    assert_eq!(
        second.status(),
        CommandStatus::Finished(Ok(CommandOutcome::Blocked))
    );
    assert_eq!(state.status().get()?, DialogStatus::Open);
    Ok(())
}
#[test]
fn held_modal_admission_and_initial_callback_failure_cancel_once() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())?;
    let state = DialogController::new(&ui);
    ui.mount_tree(Element::new(
        "modal-owner",
        Dialog::new(
            state.clone(),
            Element::new("anchor", Label::new("Main")),
            Element::new("dialog-content", Label::new("Modal")),
        )
        .application_modal(),
    ))?;
    let owner = ui.mount_tree(Element::new("held", Label::new("Held owner")))?;
    let prepared = ui.prepare_detach(&owner, NativeWindowOptions::new("Held")?)?;
    let window = ui.commit_detach(prepared)?;
    let cancellations = Rc::new(RefCell::new(Vec::new()));
    let cancelled = cancellations.clone();
    let registration = ui.register_held_input(
        &owner,
        vec![KeyCode::KeyA],
        Modifiers::default(),
        |_, _, _| Ok(InputOutcome::Handled),
        move |_, reason| cancelled.borrow_mut().push(reason),
    )?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Press)?;
    state.open()?;
    ui.flush_windows(ui.now())?;
    assert_eq!(*cancellations.borrow(), vec![HeldCancellation::Modal]);
    ui.unregister_held_input(&registration)?;
    state.cancel()?;
    ui.flush_windows(ui.now())?;
    ui.select_window(&window)?;
    let cancelled = cancellations.clone();
    ui.register_held_input(
        &owner,
        vec![KeyCode::KeyA],
        Modifiers::default(),
        |_, _, _| Err(RuntimeError::StalePresentation),
        move |_, reason| cancelled.borrow_mut().push(reason),
    )?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Release)?;
    assert_eq!(
        key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Press),
        Err(RuntimeError::StalePresentation)
    );
    ui.clear_focus();
    assert_eq!(
        *cancellations.borrow(),
        vec![HeldCancellation::Modal, HeldCancellation::InputFailure]
    );
    Ok(())
}

#[test]
fn command_info_and_invocation_use_same_window_and_sanitized_optional_focus()
-> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let roots = windows(&mut ui)?;
    ui.select_window(&roots[1].0)?;
    let child = ui.children(&roots[1].1)?[0].clone();
    ui.focus(&child)?;
    let context = ui.command_context(CommandSource::Menu);
    let invoked = ui.signal(false);
    let written = invoked.clone();
    let id = ui.register_command(
        "Contextual",
        CommandScope::Application,
        vec![],
        move |ui, context| {
            assert_eq!(ui.current_window(), context.window);
            assert!(context.focused.is_none());
            written.set(true)?;
            Ok(())
        },
    )?;
    ui.command_enabled(&id, |ui, context| {
        Ok(ui.current_window() == context.window && context.focused.is_none())
    })?;
    ui.unmount(&child)?;
    ui.select_window(&roots[2].0)?;
    assert!(ui.command_info(&id, &context)?.enabled);
    assert_eq!(ui.current_window(), roots[2].0);
    assert_eq!(ui.invoke_command(&id, context)?, CommandOutcome::Invoked);
    assert_eq!(ui.current_window(), roots[2].0);
    assert!(invoked.get()?);
    Ok(())
}

#[test]
fn queued_command_finishes_when_every_native_root_is_hidden() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let owner = ui.mount_tree(Element::new("owner", Label::new("Owner")))?;
    let id = ui.register_command(
        "Hidden action",
        CommandScope::Application,
        vec![],
        |_, _| panic!("unavailable native target"),
    )?;
    let receipt = ui.try_event(&owner, |cx| {
        cx.invoke_command(&id, cx.command_context(CommandSource::Button))
    })??;
    ui.set_window_visible(false)?;
    assert!(ui.has_command_requests());
    ui.flush_commands()?;
    assert_eq!(
        receipt.status(),
        CommandStatus::Finished(Err(RuntimeError::StaleCommandTarget))
    );
    assert!(!ui.has_command_requests());
    Ok(())
}

#[test]
fn minimizing_the_owner_window_keeps_an_application_modal_prompt_blocking()
-> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())?;
    let state = DialogController::new(&ui);
    ui.mount_tree(Element::new(
        "owner",
        Dialog::new(
            state.clone(),
            Element::new("anchor", Label::new("Main")),
            Element::new("save", Label::new("Save changes?")),
        )
        .application_modal(),
    ))?;
    let view = ui.mount_tree(Element::new("other", Label::new("Other window")))?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Other")?)?;
    let detached = ui.commit_detach(prepared)?;
    let command = ui.register_command(
        "Other action",
        CommandScope::Application,
        vec![KeyChord::primary('o')],
        |_, _| Ok(()),
    )?;
    ui.select_window(&ui.primary_window())?;
    state.open()?;
    ui.flush_windows(Duration::ZERO)?;

    // Minimizing the owner window suspends the prompt, it does not end it.
    ui.set_window_visible(false)?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(state.status().get()?, DialogStatus::Open);
    ui.select_window(&detached)?;
    assert_eq!(
        ui.invoke_command(&command, ui.command_context(CommandSource::Button))?,
        CommandOutcome::Blocked
    );

    ui.select_window(&ui.primary_window())?;
    ui.set_window_visible(true)?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(state.status().get()?, DialogStatus::Open);
    state.cancel()?;
    ui.flush_windows(Duration::ZERO)?;
    ui.select_window(&detached)?;
    assert_eq!(
        ui.invoke_command(&command, ui.command_context(CommandSource::Button))?,
        CommandOutcome::Invoked
    );
    Ok(())
}

/// Starts a timer on `A` and ignores every other key, like a custom component
/// with a pending hold-to-repeat.
struct Timed;
impl Component for Timed {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::KeyA && event.transition == KeyTransition::Pressed {
            cx.set_timer(Duration::from_secs(1))?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
}

#[test]
fn escape_through_the_native_path_ends_the_focused_components_context() -> Result<(), RuntimeError>
{
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())?;
    let timed = ui.mount_tree(Element::new("timed", Timed))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&timed)?;
    key(&mut ui, KeyCode::KeyA, 'a', KeyAction::Press)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(1)));
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::Escape,
        KeyAction::Press,
        CommandKey::Named(KeyCode::Escape),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.next_deadline(), None, "as through `key_input`");
    Ok(())
}

#[test]
fn os_repeats_run_a_command_only_after_it_opts_in() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let count = ui.signal(0);
    let written = count.clone();
    let id = ui.register_command(
        "Nudge",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Character('a'),
            Modifiers::default(),
        )],
        move |_, _| written.set(written.get()? + 1),
    )?;
    let hold = |ui: &mut Runtime| -> Result<(), RuntimeError> {
        key(ui, KeyCode::KeyA, 'a', KeyAction::Press)?;
        for _ in 0..2 {
            // A suppressed repeat is still consumed, never delivered as text.
            assert!(key(ui, KeyCode::KeyA, 'a', KeyAction::Repeat)?.handled);
        }
        key(ui, KeyCode::KeyA, 'a', KeyAction::Release).map(|_| ())
    };
    hold(&mut ui)?;
    assert_eq!(count.get()?, 1, "repeats are suppressed by default");
    let mut repeat = ui.command_context(CommandSource::Programmatic);
    repeat.repeat = true;
    assert_eq!(
        ui.invoke_command(&id, repeat.clone())?,
        CommandOutcome::RepeatSuppressed
    );
    assert_eq!(count.get()?, 1);

    ui.command_repeats(&id, true)?;
    hold(&mut ui)?;
    assert_eq!(count.get()?, 4, "the press and each repeat run the command");
    assert_eq!(ui.invoke_command(&id, repeat)?, CommandOutcome::Invoked);
    Ok(())
}

#[test]
fn a_command_needs_a_nonblank_name() {
    let mut ui = Runtime::new();
    for name in ["", "   "] {
        assert_eq!(
            ui.register_command(name, CommandScope::Application, vec![], |_, _| Ok(())),
            Err(RuntimeError::EmptyCommandName)
        );
    }
}

#[test]
fn the_command_queue_refuses_beyond_its_capacity_and_receipts_are_identities()
-> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let owner = ui.mount_tree(Element::new("owner", Label::new("Owner")))?;
    let id = ui.register_command("Queued", CommandScope::Application, vec![], |_, _| Ok(()))?;
    let receipts = ui.try_event(&owner, |cx| {
        let mut receipts = Vec::new();
        for _ in 0..Runtime::MAX_QUEUED_COMMANDS {
            receipts.push(cx.invoke_command(&id, cx.command_context(CommandSource::Button))?);
        }
        assert_eq!(
            cx.invoke_command(&id, cx.command_context(CommandSource::Button)),
            Err(RuntimeError::CommandQueueFull)
        );
        Ok::<_, RuntimeError>(receipts)
    })??;
    assert_eq!(receipts[0], receipts[0].clone());
    assert_ne!(receipts[0], receipts[1]);
    ui.flush_commands()?;
    assert!(
        receipts
            .iter()
            .all(|receipt| receipt.status() == CommandStatus::Finished(Ok(CommandOutcome::Invoked)))
    );
    // The flush emptied the queue.
    let next = ui.try_event(&owner, |cx| {
        cx.invoke_command(&id, cx.command_context(CommandSource::Button))
    })??;
    assert_eq!(next.status(), CommandStatus::Pending);
    Ok(())
}

#[test]
fn a_batch_finishes_every_receipt_when_an_action_shuts_the_runtime_down() -> Result<(), RuntimeError>
{
    let mut ui = Runtime::new();
    let owner = ui.mount_tree(Element::new("owner", Label::new("Owner")))?;
    let quit = ui.register_command("Quit", CommandScope::Application, vec![], |ui, _| {
        ui.shutdown();
        Ok(())
    })?;
    let later = ui.register_command("Later", CommandScope::Application, vec![], |_, _| {
        panic!("ran after shutdown")
    })?;
    let (first, second, third) = ui.try_event(&owner, |cx| {
        let context = cx.command_context(CommandSource::Button);
        Ok::<_, RuntimeError>((
            cx.invoke_command(&quit, context.clone())?,
            cx.invoke_command(&later, context.clone())?,
            cx.invoke_command(&later, context)?,
        ))
    })??;
    let binding = second.status_binding();
    ui.flush_commands()?;
    assert_eq!(
        first.status(),
        CommandStatus::Finished(Ok(CommandOutcome::Invoked))
    );
    // Readable after shutdown, unlike the binding.
    assert_eq!(second.status(), CommandStatus::Cancelled);
    assert_eq!(third.status(), CommandStatus::Cancelled);
    assert_eq!(binding.get(), Err(RuntimeError::Closed));
    Ok(())
}

#[test]
fn shutdown_cancels_queued_invocations_observably() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let owner = ui.mount_tree(Element::new("owner", Label::new("Owner")))?;
    let id = ui.register_command("Never", CommandScope::Application, vec![], |_, _| {
        panic!("ran after shutdown")
    })?;
    let receipt = ui.try_event(&owner, |cx| {
        cx.invoke_command(&id, cx.command_context(CommandSource::Button))
    })??;
    ui.shutdown();
    assert_eq!(receipt.status(), CommandStatus::Cancelled);
    assert!(format!("{receipt:?}").contains("Cancelled"));
    Ok(())
}

fn listed(rows: &[CommandEntry]) -> Vec<(&str, Result<bool, RuntimeError>)> {
    rows.iter()
        .map(|row| (row.name.as_str(), row.enabled.clone()))
        .collect()
}

#[test]
fn listing_follows_scope_eligibility_and_sorts_by_name() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let roots = windows(&mut ui)?;
    let app_b = ui.register_command("Beta", CommandScope::Application, vec![], |_, _| Ok(()))?;
    ui.register_command("Alpha", CommandScope::Capture, vec![], |_, _| Ok(()))?;
    let twin = ui.register_command("Beta", CommandScope::Application, vec![], |_, _| Ok(()))?;
    ui.register_command(
        "Window here",
        CommandScope::Window(roots[1].0.clone()),
        vec![],
        |_, _| Ok(()),
    )?;
    ui.register_command(
        "Window there",
        CommandScope::Window(roots[2].0.clone()),
        vec![],
        |_, _| Ok(()),
    )?;
    ui.register_command(
        "In view",
        CommandScope::View(roots[1].1.clone()),
        vec![],
        |_, _| Ok(()),
    )?;
    ui.select_window(&roots[1].0)?;
    let body = ui.children(&roots[1].1)?[0].clone();
    ui.set_command_target(None)?;
    let untargeted = ui.command_context(CommandSource::Programmatic);
    assert!(untargeted.target.is_none());
    let names = |rows: &[CommandEntry]| rows.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
    assert_eq!(
        names(&ui.commands_for(&untargeted)?),
        ["Alpha", "Beta", "Beta", "Window here"],
        "a view command needs a target inside its view"
    );
    ui.set_command_target(Some(&body))?;
    let targeted = ui.command_context(CommandSource::Programmatic);
    let rows = ui.commands_for(&targeted)?;
    assert_eq!(
        names(&rows),
        ["Alpha", "Beta", "Beta", "In view", "Window here"]
    );
    assert_eq!(rows[1].id, app_b, "equal names keep registration order");
    assert_eq!(rows[2].id, twin);
    assert_eq!(rows[0].scope, CommandScope::Capture);
    ui.unregister_command(&twin)?;
    assert_eq!(ui.commands_for(&targeted)?.len(), 4);
    Ok(())
}

#[test]
fn listing_reports_current_shortcuts_and_enabled_state_like_command_info()
-> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let save = ui.register_command(
        "Save",
        CommandScope::Application,
        vec![KeyChord::primary('s')],
        |_, _| Ok(()),
    )?;
    let enabled = ui.signal(true);
    ui.command_enabled_signal(&save, enabled.read_only())?;
    ui.rebind_command(
        &save,
        vec![KeyChord::primary('s').shifted(), KeyChord::primary('w')],
    )?;
    let context = ui.command_context(CommandSource::Menu);
    let row = ui.commands_for(&context)?.remove(0);
    assert_eq!(
        row.shortcuts,
        [KeyChord::primary('s').shifted(), KeyChord::primary('w')]
    );
    assert_eq!(row.enabled, Ok(true));
    enabled.set(false)?;
    assert_eq!(listed(&ui.commands_for(&context)?), [("Save", Ok(false))]);
    assert_eq!(
        Ok(ui.command_info(&save, &context)?.enabled),
        ui.commands_for(&context)?[0].enabled
    );
    Ok(())
}

#[test]
fn listing_shows_modal_blocked_commands_as_disabled() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())?;
    let state = DialogController::new(&ui);
    ui.mount_tree(Element::new(
        "owner",
        Dialog::new(
            state.clone(),
            Element::new("anchor", Label::new("Main")),
            Element::new("body", Label::new("Dialog")),
        ),
    ))?;
    ui.flush(Duration::ZERO)?;
    let blocked =
        ui.register_command("Blocked", CommandScope::Application, vec![], |_, _| Ok(()))?;
    let allowed =
        ui.register_command("Allowed", CommandScope::Application, vec![], |_, _| Ok(()))?;
    ui.command_in_modals(&allowed, true)?;
    state.open()?;
    ui.flush(Duration::ZERO)?;
    let context = ui.command_context(CommandSource::Menu);
    assert_eq!(
        listed(&ui.commands_for(&context)?),
        [("Allowed", Ok(true)), ("Blocked", Ok(false))]
    );
    for id in [&allowed, &blocked] {
        let row = ui
            .commands_for(&context)?
            .into_iter()
            .find(|row| row.id == *id)
            .unwrap();
        assert_eq!(Ok(ui.command_info(id, &context)?.enabled), row.enabled);
    }
    Ok(())
}

#[test]
fn listing_runs_predicates_read_only_and_restores_the_selected_window() -> Result<(), RuntimeError>
{
    let mut ui = Runtime::new();
    let roots = windows(&mut ui)?;
    let probe = ui.signal(0);
    let written = probe.clone();
    let id = ui.register_command("Probe", CommandScope::Application, vec![], |_, _| Ok(()))?;
    ui.command_enabled(&id, move |ui, context| {
        assert_eq!(ui.current_window(), context.window);
        // Predicates are read-only: a write is refused.
        assert!(written.set(1).is_err());
        Ok(true)
    })?;
    ui.select_window(&roots[1].0)?;
    let context = ui.command_context(CommandSource::Menu);
    ui.select_window(&roots[2].0)?;
    assert_eq!(listed(&ui.commands_for(&context)?), [("Probe", Ok(true))]);
    assert_eq!(ui.current_window(), roots[2].0);
    assert_eq!(probe.get()?, 0);

    let failing =
        ui.register_command("Failing", CommandScope::Application, vec![], |_, _| Ok(()))?;
    ui.command_enabled(&failing, |_, _| Err(RuntimeError::BorrowConflict))?;
    assert_eq!(
        listed(&ui.commands_for(&context)?),
        [
            ("Failing", Err(RuntimeError::BorrowConflict)),
            ("Probe", Ok(true))
        ],
        "a failing predicate affects only its own row"
    );
    assert_eq!(ui.current_window(), roots[2].0);
    Ok(())
}

#[test]
fn listing_discards_stale_optional_focus_like_command_info() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let roots = windows(&mut ui)?;
    ui.select_window(&roots[1].0)?;
    let child = ui.children(&roots[1].1)?[0].clone();
    ui.focus(&child)?;
    let context = ui.command_context(CommandSource::Menu);
    assert!(context.focused.is_some());
    let id = ui.register_command("Focus", CommandScope::Application, vec![], |_, _| Ok(()))?;
    ui.command_enabled(&id, |_, context| Ok(context.focused.is_none()))?;
    ui.unmount(&child)?;
    let row = ui.commands_for(&context)?.remove(0);
    assert_eq!(row.enabled, Ok(true), "the unmounted focus was discarded");
    assert_eq!(Ok(ui.command_info(&id, &context)?.enabled), row.enabled);
    Ok(())
}

#[test]
fn listing_rejects_a_stale_context_and_rows_invoke_with_revalidation() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let root = ui.mount_tree(Element::new(
        "view",
        View::new(Element::new("body", Label::new("Editor"))),
    ))?;
    ui.flush(Duration::ZERO)?;
    let body = ui.children(&root)?[0].clone();
    let count = ui.signal(0);
    let written = count.clone();
    ui.register_command(
        "Count",
        CommandScope::View(root.clone()),
        vec![],
        move |_, _| written.set(written.get()? + 1),
    )?;
    ui.set_command_target(Some(&body))?;
    let context = ui.command_context(CommandSource::Menu);
    let row = ui.commands_for(&context)?.remove(0);
    assert_eq!(
        ui.invoke_command(&row.id, context.clone())?,
        CommandOutcome::Invoked
    );
    assert_eq!(count.get()?, 1);
    ui.unmount(&body)?;
    assert_eq!(
        ui.commands_for(&context).err(),
        Some(RuntimeError::StaleCommandTarget)
    );
    assert_eq!(
        ui.invoke_command(&row.id, context).err(),
        Some(RuntimeError::StaleCommandTarget),
        "a retained row revalidates when invoked"
    );
    Ok(())
}
