//! Navigation input admission and cancellation regressions through the facade.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn send(ui: &mut Runtime, code: KeyCode, action: KeyAction) {
    ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
        .unwrap();
    ui.flush(ui.now()).unwrap();
}
#[test]
fn modified_tab_navigation_yields_to_application_commands() {
    for code in [
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::Home,
        KeyCode::End,
    ] {
        for modifiers in [
            Modifiers::PRIMARY,
            Modifiers {
                alt: true,
                ..Modifiers::default()
            },
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
        ] {
            let mut ui = runtime();
            let root = ui
                .mount_tree(Element::new(
                    "tabs",
                    Tabs::new([
                        Tab::new("a", "A", Element::new("a-body", Label::new("A"))),
                        Tab::new("b", "B", Element::new("b-body", Label::new("B"))),
                    ]),
                ))
                .unwrap();
            ui.flush(Duration::ZERO).unwrap();
            ui.focus(&root).unwrap();
            let hits = Rc::new(Cell::new(0));
            let counted = hits.clone();
            ui.register_command(
                "Navigation",
                CommandScope::Application,
                vec![KeyChord::new(CommandKey::Named(code), modifiers)],
                move |_, _| {
                    counted.set(counted.get() + 1);
                    Ok(())
                },
            )
            .unwrap();
            ui.native_modifiers(modifiers).unwrap();
            send(&mut ui, code, KeyAction::Press);
            send(&mut ui, code, KeyAction::Release);
            assert_eq!(hits.get(), 1, "{code:?} {modifiers:?}");
            assert!(
                ui.inspect()
                    .components
                    .iter()
                    .any(|c| c.key == "a-body" && c.visible)
            );
            ui.native_modifiers(Modifiers::default()).unwrap();
            send(&mut ui, KeyCode::ArrowRight, KeyAction::Press);
            send(&mut ui, KeyCode::ArrowRight, KeyAction::Release);
            assert!(
                ui.inspect()
                    .components
                    .iter()
                    .any(|c| c.key == "b-body" && c.visible)
            );
        }
    }
}

#[test]
fn history_side_buttons_keep_the_first_press_and_discard_revoked_arming() {
    for reverse_release in [false, true] {
        let mut ui = runtime();
        let history = NavigationHistory::new(&ui).unwrap();
        for value in [1, 2, 3] {
            history.visit(value, value.to_string()).unwrap();
        }
        let enabled = ui.signal(true);
        let hits = Rc::new(Cell::new(0));
        let counted = hits.clone();
        ui.mount_tree(Element::new(
            "history",
            HistoryButtons::new(
                history.clone(),
                Element::new("body", Label::new("Content")),
                move |_, _| {
                    counted.set(counted.get() + 1);
                    Ok(())
                },
            )
            .enabled(enabled.read_only()),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let point = Point::new(10.0, 10.0).unwrap();
        let pointer =
            |ui: &mut Runtime, kind| ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
        pointer(&mut ui, PointerKind::Press(PointerButton::Back));
        pointer(&mut ui, PointerKind::Press(PointerButton::Forward));
        assert_eq!(hits.get(), 1);
        let order = if reverse_release {
            [PointerButton::Forward, PointerButton::Back]
        } else {
            [PointerButton::Back, PointerButton::Forward]
        };
        for button in order {
            let outcome = pointer(&mut ui, PointerKind::Release(button));
            assert_eq!(
                outcome,
                if button == PointerButton::Back {
                    InputOutcome::Handled
                } else {
                    InputOutcome::Ignored
                }
            );
        }
        pointer(&mut ui, PointerKind::Press(PointerButton::Back));
        enabled.set(false).unwrap();
        ui.flush(ui.now()).unwrap();
        enabled.set(true).unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(
            pointer(&mut ui, PointerKind::Release(PointerButton::Back)),
            InputOutcome::Ignored
        );
        for kind in [
            PointerKind::Press(PointerButton::Forward),
            PointerKind::Release(PointerButton::Forward),
        ] {
            pointer(&mut ui, kind);
        }
        assert_eq!(hits.get(), 3);
    }
}

#[test]
fn an_unowned_repeat_cannot_start_a_command_after_focus_moves() {
    struct MoveFocus(Rc<RefCell<Option<WidgetId>>>);
    impl Component for MoveFocus {
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
            if event.code == KeyCode::Enter && event.transition == KeyTransition::Pressed {
                cx.request_focus_for(self.0.borrow().as_ref().unwrap())?;
                return Ok(InputOutcome::Handled);
            }
            Ok(InputOutcome::Ignored)
        }
    }
    let mut ui = runtime();
    let target = Rc::new(RefCell::new(None));
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("source", MoveFocus(target.clone())),
                Element::new("target", Button::new("Target", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let source = ui.find_descendant(&root, "source").unwrap().unwrap();
    let destination = ui.find_descendant(&root, "target").unwrap().unwrap();
    *target.borrow_mut() = Some(destination.clone());
    ui.focus(&source).unwrap();
    let hits = Rc::new(Cell::new(0));
    let counted = hits.clone();
    let command = ui
        .register_command(
            "Repeat",
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Named(KeyCode::Enter),
                Modifiers::PRIMARY,
            )],
            move |_, _| {
                counted.set(counted.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    ui.command_repeats(&command, true).unwrap();
    send(&mut ui, KeyCode::Enter, KeyAction::Press);
    assert_eq!(ui.focused(), Some(destination));
    ui.native_modifiers(Modifiers::PRIMARY).unwrap();
    let dispatch = ui
        .command_key_input(CommandKeyInput::new(
            KeyCode::Enter,
            KeyAction::Repeat,
            CommandKey::Named(KeyCode::Enter),
        ))
        .unwrap();
    assert!(!dispatch.handled);
    assert!(!dispatch.deliver_text);
    send(&mut ui, KeyCode::Enter, KeyAction::Release);
    assert_eq!(hits.get(), 0);
    send(&mut ui, KeyCode::Enter, KeyAction::Press);
    send(&mut ui, KeyCode::Enter, KeyAction::Repeat);
    send(&mut ui, KeyCode::Enter, KeyAction::Release);
    assert_eq!(hits.get(), 2);
}

#[test]
fn zoom_focus_outline_tracks_hide_reveal_and_cancel_with_focus() {
    let mut ui = runtime();
    let root = ui
        .mount_tree(Element::new("zoom", ZoomView::new([])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.paint().unwrap().commands().is_empty());
    ui.focus(&root).unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(!ui.paint().unwrap().commands().is_empty());
    ui.cancel_input().unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(ui.focused(), Some(root.clone()));
    assert!(!ui.paint().unwrap().commands().is_empty());
    ui.set_visible(&root, false).unwrap();
    ui.flush(ui.now()).unwrap();
    ui.set_visible(&root, true).unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(ui.focused(), None);
    assert!(ui.paint().unwrap().commands().is_empty());
    ui.focus(&root).unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(!ui.paint().unwrap().commands().is_empty());
}

#[test]
fn tab_reorder_and_cycling_require_their_explicit_modifier_chords() {
    let mut ui = runtime();
    let moves = Rc::new(Cell::new(0));
    let counted = moves.clone();
    let root = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([
                Tab::new("a", "A", Element::new("a-body", Label::new("A"))),
                Tab::new("b", "B", Element::new("b-body", Label::new("B"))),
            ])
            .reorderable(move |_, _| {
                counted.set(counted.get() + 1);
                Ok(false)
            }),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&root).unwrap();
    let hits = Rc::new(Cell::new(0));
    let counted = hits.clone();
    let chords = [
        (
            KeyCode::ArrowRight,
            Modifiers {
                alt: true,
                shift: true,
                ..Modifiers::default()
            },
        ),
        (
            KeyCode::Tab,
            Modifiers {
                control: true,
                alt: true,
                ..Modifiers::default()
            },
        ),
    ];
    ui.register_command(
        "Mixed navigation",
        CommandScope::Application,
        chords
            .iter()
            .map(|(code, modifiers)| KeyChord::new(CommandKey::Named(*code), *modifiers))
            .collect(),
        move |_, _| {
            counted.set(counted.get() + 1);
            Ok(())
        },
    )
    .unwrap();
    for (code, modifiers) in chords {
        ui.native_modifiers(modifiers).unwrap();
        send(&mut ui, code, KeyAction::Press);
        send(&mut ui, code, KeyAction::Release);
    }
    assert_eq!(hits.get(), 2);
    assert_eq!(moves.get(), 0);
    ui.native_modifiers(Modifiers {
        alt: true,
        ..Modifiers::default()
    })
    .unwrap();
    send(&mut ui, KeyCode::ArrowRight, KeyAction::Press);
    send(&mut ui, KeyCode::ArrowRight, KeyAction::Release);
    assert_eq!(moves.get(), 1);
    ui.native_modifiers(Modifiers {
        control: true,
        shift: true,
        ..Modifiers::default()
    })
    .unwrap();
    send(&mut ui, KeyCode::Tab, KeyAction::Press);
    send(&mut ui, KeyCode::Tab, KeyAction::Release);
    assert!(
        ui.inspect()
            .components
            .iter()
            .any(|c| c.key == "b-body" && c.visible)
    );
    assert_eq!(hits.get(), 2);
}
