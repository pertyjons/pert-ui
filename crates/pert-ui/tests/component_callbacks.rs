//! User-write callbacks added by the component API review.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
mod support;
use support::{key, settle};

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(500.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn boolean(kind: usize, value: Signal<bool>, log: Rc<RefCell<Vec<bool>>>, fail: bool) -> Element {
    let handler = move |_: &mut EventContext, value| {
        log.borrow_mut().push(value);
        if fail {
            Err(RuntimeError::EmptyWindowTitle)
        } else {
            Ok(())
        }
    };
    match kind {
        0 => Element::new("control", Checkbox::new("Value", value).on_change(handler)),
        1 => Element::new("control", Toggle::new("Value", value).on_change(handler)),
        _ => Element::new(
            "control",
            InlineToggle::new(value, "Value").on_change(handler),
        ),
    }
}
#[test]
fn boolean_callbacks_follow_user_writes_replace_and_never_replay_failures() {
    for kind in 0..3 {
        let mut ui = runtime();
        let value = ui.signal(false);
        let first = Rc::new(RefCell::new(Vec::new()));
        let second = Rc::new(RefCell::new(Vec::new()));
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([boolean(kind, value.clone(), first.clone(), false)]),
            ))
            .unwrap();
        settle(&mut ui);
        let control = ui.children(&root).unwrap()[0].clone();
        ui.focus(&control).unwrap();
        key(&mut ui, KeyCode::Space);
        assert_eq!(*first.borrow(), [true]);
        value.set(false).unwrap();
        settle(&mut ui);
        assert_eq!(first.borrow().len(), 1);
        ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
        ui.reconcile_children(
            &root,
            vec![boolean(kind, value.clone(), second.clone(), true)],
        )
        .unwrap();
        assert!(ui.key_input(KeyCode::Space, KeyAction::Release).is_err());
        assert!(value.get().unwrap());
        assert_eq!(*second.borrow(), [true]);
        settle(&mut ui);
        assert_eq!(second.borrow().len(), 1);
        assert_eq!(first.borrow().len(), 1);
    }
}
#[test]
fn boolean_callbacks_report_remote_submission_not_response() {
    for kind in 0..3 {
        let mut ui = runtime();
        let (parameter, host) = ui
            .remote_parameter(
                ParameterId::new(1, 1, "enabled").unwrap(),
                ParameterSpec::boolean("Enabled", false).unwrap(),
            )
            .unwrap();
        let log = Rc::new(RefCell::new(Vec::new()));
        let sink = log.clone();
        let handler = move |_: &mut EventContext, value| {
            sink.borrow_mut().push(value);
            Ok(())
        };
        let element = match kind {
            0 => Element::new(
                "control",
                Checkbox::for_parameter(parameter.clone()).on_change(handler),
            ),
            1 => Element::new(
                "control",
                Toggle::for_parameter(parameter.clone()).on_change(handler),
            ),
            _ => Element::new(
                "control",
                InlineToggle::for_parameter(parameter.clone()).on_change(handler),
            ),
        };
        let control = ui.mount_tree(element).unwrap();
        settle(&mut ui);
        ui.focus(&control).unwrap();
        key(&mut ui, KeyCode::Space);
        assert_eq!(*log.borrow(), [true]);
        assert!(!parameter.snapshot().get().unwrap().base);
        let request = host.request().unwrap();
        host.respond(&request, AuthorityResponse::Rejected).unwrap();
        settle(&mut ui);
        assert_eq!(log.borrow().len(), 1);
        ui.focus(&control).unwrap();
        key(&mut ui, KeyCode::Space);
        let request = host.request().unwrap();
        host.respond(&request, AuthorityResponse::Accepted(true))
            .unwrap();
        settle(&mut ui);
        assert_eq!(*log.borrow(), [true, true]);
    }
}
fn select(
    ui: &Runtime,
    inline: bool,
    value: Signal<&'static str>,
    log: Rc<RefCell<Vec<&'static str>>>,
    fail: bool,
) -> Element {
    let handler = move |cx: &mut EventContext, value: &&'static str| {
        let log = log.clone();
        let value = *value;
        let _ = cx.defer_detached(move |_| {
            log.borrow_mut().push(value);
            Ok(())
        })?;
        if fail {
            Err(RuntimeError::EmptyWindowTitle)
        } else {
            Ok(())
        }
    };
    let options = [Choice::new("a", "A"), Choice::new("b", "B")];
    if inline {
        Element::new(
            "control",
            InlineSelect::new(ui, value, options)
                .unwrap()
                .on_change(handler),
        )
    } else {
        Element::new(
            "control",
            Select::new(ui, value, options).unwrap().on_change(handler),
        )
    }
}
fn open(ui: &mut Runtime, root: &WidgetId) {
    let anchor = ui.find_descendant(root, "select").unwrap().unwrap();
    ui.focus(&anchor).unwrap();
    key(ui, KeyCode::Enter);
}
#[test]
fn select_callbacks_close_replace_and_skip_same_value_and_application_writes() {
    for inline in [false, true] {
        let mut ui = runtime();
        let value = ui.signal("a");
        let first = Rc::new(RefCell::new(Vec::new()));
        let second = Rc::new(RefCell::new(Vec::new()));
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([select(&ui, inline, value.clone(), first.clone(), false)]),
            ))
            .unwrap();
        settle(&mut ui);
        open(&mut ui, &root);
        key(&mut ui, KeyCode::Enter);
        assert!(first.borrow().is_empty());
        open(&mut ui, &root);
        key(&mut ui, KeyCode::ArrowDown);
        key(&mut ui, KeyCode::Enter);
        settle(&mut ui);
        assert_eq!(value.get().unwrap(), "b");
        assert_eq!(*first.borrow(), ["b"]);
        value.set("a").unwrap();
        settle(&mut ui);
        open(&mut ui, &root);
        ui.reconcile_children(
            &root,
            vec![select(&ui, inline, value.clone(), second.clone(), true)],
        )
        .unwrap();
        key(&mut ui, KeyCode::ArrowDown);
        ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
        assert!(ui.key_input(KeyCode::Enter, KeyAction::Release).is_err());
        settle(&mut ui);
        assert_eq!(value.get().unwrap(), "b");
        assert_eq!(*second.borrow(), ["b"]);
        assert_eq!(first.borrow().len(), 1);
        assert!(
            !ui.inspect()
                .components
                .iter()
                .any(|c| c.key == "options" && c.visible)
        );
        ui.flush(Duration::from_secs(1)).unwrap();
        assert_eq!(second.borrow().len(), 1);
    }
}
#[test]
fn choice_adornment_slots_replace_and_survive_caption_changes() {
    let expected = RichText::from("Final")
        .leading_icon(Icon::Check)
        .trailing_icon(Icon::Close);
    let choice = Choice::new(1, "Old")
        .leading_icon(Icon::Close)
        .leading_icon(Icon::Check)
        .trailing_icon(Icon::Check)
        .trailing_icon(Icon::Close)
        .rich_label(RichText::from("Final"));
    assert_eq!(choice.content(), &expected);
    assert_eq!(choice.label(), "Final");
    assert_eq!(choice.value(), &1);
}

#[test]
fn select_remote_callback_reports_only_proposals() {
    for inline in [false, true] {
        let mut ui = runtime();
        let a = ChoiceId::new("a").unwrap();
        let b = ChoiceId::new("b").unwrap();
        let spec = ParameterSpec::choices(
            "Choice",
            [
                ChoiceOption::new(a.clone(), "A"),
                ChoiceOption::new(b.clone(), "B"),
            ],
            a.clone(),
        )
        .unwrap();
        let (parameter, host) = ui
            .remote_parameter(ParameterId::new(1, 1, "choice").unwrap(), spec)
            .unwrap();
        let log = Rc::new(RefCell::new(Vec::new()));
        let sink = log.clone();
        let handler = move |_: &mut EventContext, value: &ChoiceId| {
            sink.borrow_mut().push(value.clone());
            Ok(())
        };
        let control = if inline {
            Element::new(
                "control",
                InlineSelect::for_parameter(&ui, parameter.clone()).on_change(handler),
            )
        } else {
            Element::new(
                "control",
                Select::for_parameter(&ui, parameter.clone()).on_change(handler),
            )
        };
        let root = ui
            .mount_tree(Element::new("root", Column::new([control])))
            .unwrap();
        settle(&mut ui);
        for accepted in [false, true] {
            open(&mut ui, &root);
            key(&mut ui, KeyCode::ArrowDown);
            key(&mut ui, KeyCode::Enter);
            assert_eq!(parameter.snapshot().get().unwrap().base, a);
            let request = host.request().unwrap();
            let count = log.borrow().len();
            host.respond(
                &request,
                if accepted {
                    AuthorityResponse::Accepted(b.clone())
                } else {
                    AuthorityResponse::Rejected
                },
            )
            .unwrap();
            settle(&mut ui);
            assert_eq!(log.borrow().len(), count);
        }
        assert_eq!(*log.borrow(), [b.clone(), b]);
    }
}

#[test]
fn removing_handlers_suppresses_later_user_notifications() {
    let mut ui = runtime();
    let boolean_value = ui.signal(false);
    let selected = ui.signal("a");
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "boolean",
                    Toggle::new("Toggle", boolean_value.clone())
                        .on_change(|_, _| panic!("removed handler")),
                ),
                Element::new(
                    "control",
                    Select::new(
                        &ui,
                        selected.clone(),
                        [Choice::new("a", "A"), Choice::new("b", "B")],
                    )
                    .unwrap()
                    .on_change(|_, _| panic!("removed handler")),
                ),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    open(&mut ui, &root);
    ui.reconcile_children(
        &root,
        vec![
            Element::new("boolean", Toggle::new("Toggle", boolean_value.clone())),
            Element::new(
                "control",
                Select::new(
                    &ui,
                    selected.clone(),
                    [Choice::new("a", "A"), Choice::new("b", "B")],
                )
                .unwrap(),
            ),
        ],
    )
    .unwrap();
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(selected.get().unwrap(), "b");
    let toggle = ui.find_descendant(&root, "boolean").unwrap().unwrap();
    ui.focus(&toggle).unwrap();
    key(&mut ui, KeyCode::Space);
    assert!(boolean_value.get().unwrap());
}

#[test]
fn removing_and_reinstalling_navigation_callbacks_preserves_global_order() {
    for tabs in [false, true] {
        let mut ui = runtime();
        let log = Rc::new(RefCell::new(Vec::new()));
        let describe = |a_enabled| {
            ["a", "b"]
                .into_iter()
                .map(|owner| {
                    let enabled = owner != "a" || a_enabled;
                    let log = log.clone();
                    if tabs {
                        let control = Tabs::new([
                            Tab::new(
                                "first",
                                "First",
                                Element::new("first-body", Label::new("First")),
                            ),
                            Tab::new(
                                "second",
                                "Second",
                                Element::new("second-body", Label::new("Second")),
                            ),
                        ]);
                        Element::new(
                            owner,
                            if enabled {
                                control.on_select(move |_, _| {
                                    log.borrow_mut().push(owner);
                                    Ok(())
                                })
                            } else {
                                control
                            },
                        )
                    } else {
                        let control = Accordion::new([
                            Section::new(
                                "first",
                                "First",
                                Element::new("first-body", Label::new("First")),
                            ),
                            Section::new(
                                "second",
                                "Second",
                                Element::new("second-body", Label::new("Second")),
                            ),
                        ])
                        .mode(ExpandMode::Single)
                        .expanded(["first"]);
                        Element::new(
                            owner,
                            if enabled {
                                control.on_change(move |_, _, _| {
                                    log.borrow_mut().push(owner);
                                    Ok(())
                                })
                            } else {
                                control
                            },
                        )
                    }
                })
                .collect::<Vec<_>>()
        };
        let saved = |owner: &str, selected: &str| {
            let state = if tabs {
                ViewState::new("tabs")
                    .unwrap()
                    .with("selected", WorkspaceValue::text(selected).unwrap())
                    .unwrap()
            } else {
                ViewState::new("accordion")
                    .unwrap()
                    .with(
                        "open",
                        WorkspaceValue::map([(selected.to_owned(), WorkspaceValue::flag(true))])
                            .unwrap(),
                    )
                    .unwrap()
            };
            Workspace::new(
                WorkspaceSettings::default(),
                [(WorkspacePath::new(["root", owner]).unwrap(), state)],
            )
            .unwrap()
        };
        let root = ui
            .mount_tree(Element::new("root", Column::new(describe(true))))
            .unwrap();
        settle(&mut ui);
        assert!(
            ui.restore_workspace(&saved("a", "second"))
                .unwrap()
                .is_complete()
        );
        ui.reconcile_children(&root, describe(false)).unwrap();
        ui.reconcile_children(&root, describe(true)).unwrap();
        assert!(
            ui.restore_workspace(&saved("b", "second"))
                .unwrap()
                .is_complete()
        );
        assert!(
            ui.restore_workspace(&saved("a", "first"))
                .unwrap()
                .is_complete()
        );
        settle(&mut ui);
        assert_eq!(*log.borrow(), ["b", "a"]);
    }
}
