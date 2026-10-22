//! Default activation admission and later command scopes through native routing.
use crate::support;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

fn send(ui: &mut Runtime, code: KeyCode, action: KeyAction) {
    ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
        .unwrap();
    ui.flush(ui.now()).unwrap();
}

#[test]
fn modified_activation_yields_and_armed_release_survives_modifier_changes() {
    for kind in 0..7 {
        for code in [KeyCode::Enter, KeyCode::Space] {
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
                let mut ui = Runtime::new();
                ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
                    .unwrap();
                let activations = Rc::new(Cell::new(0));
                let hits = activations.clone();
                let element = match kind {
                    0 => Element::new(
                        "control",
                        Button::new("Action", move |_| {
                            hits.set(hits.get() + 1);
                            Ok(())
                        }),
                    ),
                    1 => Element::new(
                        "control",
                        Breadcrumb::new(
                            vec![
                                BreadcrumbSegment::new("a", "Parent"),
                                BreadcrumbSegment::new("b", "Current"),
                            ],
                            move |_, _| {
                                hits.set(hits.get() + 1);
                                Ok(())
                            },
                        )
                        .unwrap(),
                    ),
                    2 => Element::new(
                        "control",
                        Collapsible::new("Header", Element::new("body", Label::new("Body"))),
                    ),
                    3 => Element::new(
                        "control",
                        Accordion::new([Section::new(
                            "a",
                            "Header",
                            Element::new("body", Label::new("Body")),
                        )]),
                    ),
                    4 => {
                        let queue = ToastController::new(&ui);
                        let _receipt = queue
                            .push(Toast::new("Message").action("Action", move |_| {
                                hits.set(hits.get() + 1);
                                Ok(())
                            }))
                            .unwrap();
                        Element::new(
                            "control",
                            ToastHost::new(queue, Element::new("content", Label::new("Content"))),
                        )
                    }
                    5 => Element::new(
                        "control",
                        MenuBar::new(
                            &ui,
                            vec![MenuItem::new("action", "Action", move |_| {
                                hits.set(hits.get() + 1);
                                Ok(())
                            })],
                        )
                        .unwrap(),
                    ),
                    _ => Element::new(
                        "control",
                        ToolWindowLayout::new(Element::new("content", Label::new("Content")))
                            .edge(ToolEdge::Left, EdgeMode::Stripe)
                            .window(ToolWindow::new(
                                "panel",
                                "Panel",
                                Element::new("body", Label::new("Body")),
                            )),
                    ),
                };
                let root = ui.mount_tree(element).unwrap();
                ui.flush(Duration::ZERO).unwrap();
                let focus = match kind {
                    1 => support::find(&ui, &root, "a").unwrap(),
                    4 => support::find(&ui, &root, "toasts").unwrap(),
                    5 => support::find(&ui, &root, "title/action").unwrap(),
                    6 => support::find(&ui, &root, "stripe-left").unwrap(),
                    _ => root.clone(),
                };
                ui.focus(&focus).unwrap();
                let commands = Rc::new(Cell::new(0));
                let hits = commands.clone();
                let _command = ui
                    .register_command(
                        "Modified action",
                        CommandScope::Application,
                        vec![KeyChord::new(CommandKey::Named(code), modifiers)],
                        move |_, _| {
                            hits.set(hits.get() + 1);
                            Ok(())
                        },
                    )
                    .unwrap();
                ui.command_repeats(&_command, true).unwrap();
                ui.native_modifiers(modifiers).unwrap();
                send(&mut ui, code, KeyAction::Press);
                send(&mut ui, code, KeyAction::Release);
                assert_eq!(commands.get(), 1);
                assert_eq!(activations.get(), 0);
                if matches!(kind, 2 | 3 | 6) {
                    assert!(
                        ui.inspect()
                            .components
                            .iter()
                            .any(|c| c.key == "body" && !c.visible)
                    );
                }
                ui.native_modifiers(Modifiers::default()).unwrap();
                send(&mut ui, code, KeyAction::Press);
                send(&mut ui, code, KeyAction::Repeat);
                ui.native_modifiers(modifiers).unwrap();
                send(&mut ui, code, KeyAction::Repeat);
                send(&mut ui, code, KeyAction::Release);
                assert_eq!(commands.get(), 1);
                if !matches!(kind, 2 | 3 | 6) {
                    assert_eq!(activations.get(), 1);
                } else {
                    assert!(
                        ui.inspect()
                            .components
                            .iter()
                            .any(|c| c.key == "body" && c.visible)
                    );
                }
            }
        }
    }
}

#[test]
fn toast_modified_input_preserves_pointer_ownership_and_escape_cancels() {
    for pointer in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(120.0, 300.0, 1.0).unwrap())
            .unwrap();
        let queue = ToastController::new(&ui);
        let hits = Rc::new(Cell::new(0));
        let counted = hits.clone();
        let receipt = queue
            .push(Toast::new("Message").action("Action", move |_| {
                counted.set(counted.get() + 1);
                Ok(())
            }))
            .unwrap();
        let root = ui
            .mount_tree(Element::new(
                "host",
                ToastHost::new(queue, Element::new("body", Label::new("Body"))),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let stack = support::find(&ui, &root, "toasts").unwrap();
        ui.focus(&stack).unwrap();
        ui.flush(ui.now()).unwrap();
        let theme = ui.theme();
        let color = theme
            .palette(theme.mode().unwrap())
            .unwrap()
            .color(ColorRole::OnButton);
        let scene = ui.paint().unwrap();
        let clip = scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::DrawMask {
                    color: c,
                    clip: Some(clip),
                    ..
                } if *c == color => Some(*clip),
                _ => None,
            })
            .min_by(|a, b| a.y().total_cmp(&b.y()))
            .unwrap();
        let point = Point::new(
            clip.x() + clip.width() / 2.0,
            clip.y() + clip.height() / 2.0,
        )
        .unwrap();
        if pointer {
            ui.pointer_input(PointerEvent::new(
                point,
                PointerKind::Press(PointerButton::Primary),
            ))
            .unwrap();
        } else {
            send(&mut ui, KeyCode::Space, KeyAction::Press);
        }
        ui.native_modifiers(Modifiers::PRIMARY).unwrap();
        send(&mut ui, KeyCode::Enter, KeyAction::Press);
        send(&mut ui, KeyCode::Enter, KeyAction::Release);
        assert_eq!(hits.get(), 0);
        send(&mut ui, KeyCode::Escape, KeyAction::Press);
        send(&mut ui, KeyCode::Escape, KeyAction::Release);
        if pointer {
            ui.pointer_input(PointerEvent::new(
                point,
                PointerKind::Release(PointerButton::Primary),
            ))
            .unwrap();
        } else {
            send(&mut ui, KeyCode::Space, KeyAction::Release);
        }
        assert_eq!(hits.get(), 0);
        assert_eq!(receipt.status(), ToastStatus::Visible);
        ui.native_modifiers(Modifiers::default()).unwrap();
        send(&mut ui, KeyCode::Enter, KeyAction::Press);
        send(&mut ui, KeyCode::Enter, KeyAction::Release);
        assert_eq!(hits.get(), 1);
        assert_eq!(receipt.status(), ToastStatus::ActionSucceeded);
    }
}
