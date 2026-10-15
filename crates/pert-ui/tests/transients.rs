//! Owned popup placement, routing, focus and delayed help through the public facade.
use pert_ui::*;
mod support;
use std::time::Duration;
use support::{find, press};
fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn point(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) -> InputOutcome {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap()
}
fn alert(_ui: &Runtime, state: &DialogController, name: &str) -> Element {
    Element::new(
        name,
        AlertDialog::new(
            state.clone(),
            Element::new(
                format!("open-{name}"),
                Button::new("Open", {
                    let s = state.clone();
                    move |_| s.open()
                }),
            ),
            "Confirm",
            "Proceed?",
            "Delete",
        ),
    )
}
#[test]
fn alert_safe_focus_traps_navigation_and_retains_exactly_one_outcome() {
    let mut ui = ui();
    let state = DialogController::new(&ui);
    let count = ui.signal(0);
    let c = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                alert(&ui, &state, "alert"),
                Element::new(
                    "outside",
                    Button::new("Outside", move |_| c.set(c.get()? + 1).map(|_| ())),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(&ui, &root, "open-alert").unwrap();
    let outside = find(&ui, &root, "outside").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    let cancel = find(&ui, &root, "cancel").unwrap();
    let accept = find(&ui, &root, "accept").unwrap();
    assert_eq!(ui.focused(), Some(cancel.clone()));
    assert_eq!(ui.focus(&outside), Err(RuntimeError::ModalBlocked));
    press(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(accept));
    press(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(cancel));
    point(
        &mut ui,
        5.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    point(
        &mut ui,
        5.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(state.status().get().unwrap(), DialogStatus::Open);
    assert_eq!(count.get().unwrap(), 0);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(trigger));
    assert_eq!(state.open(), Err(RuntimeError::DialogBusy));
    assert_eq!(
        state.take_outcome().unwrap(),
        Some(DialogOutcome::Cancelled)
    );
    assert_eq!(state.take_outcome().unwrap(), None);
    state.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::Tab);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(state.take_outcome().unwrap(), Some(DialogOutcome::Accepted));
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
}
#[test]
fn modal_conflict_rejects_only_new_session_and_removal_retains_outcome() {
    let mut ui = ui();
    let first = DialogController::new(&ui);
    let second = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([alert(&ui, &first, "first"), alert(&ui, &second, "second")]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    first.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    second.open().unwrap();
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::ModalConflict));
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first.status().get().unwrap(), DialogStatus::Open);
    assert_eq!(
        second.take_outcome().unwrap(),
        Some(DialogOutcome::Failed(SurfaceDismissal::ModalConflict))
    );
    ui.clear_focus();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first.status().get().unwrap(), DialogStatus::Open);
    ui.unmount(&root).unwrap();
    assert_eq!(
        first.take_outcome().unwrap(),
        Some(DialogOutcome::Dismissed(SurfaceDismissal::OwnerRemoved))
    );
}
#[test]
fn nested_dialogs_restore_parent_then_original_opener() {
    let mut ui = ui();
    let outer = DialogController::new(&ui);
    let inner = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "outer",
            Dialog::new(
                outer.clone(),
                Element::new(
                    "trigger",
                    Button::new("Open", {
                        let s = outer.clone();
                        move |_| s.open()
                    }),
                ),
                alert(&ui, &inner, "inner"),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(&ui, &root, "trigger").unwrap();
    let inner_trigger = find(&ui, &root, "open-inner").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ui.focused(), Some(inner_trigger.clone()));
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(inner_trigger));
    assert_eq!(outer.status().get().unwrap(), DialogStatus::Open);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(trigger));
}
#[test]
fn view_modal_leaves_sibling_interactive_and_require_response_ignores_escape() {
    let mut ui = ui();
    let state = DialogController::new(&ui);
    let count = ui.signal(0);
    let c = count.clone();
    let dialog = Dialog::new(
        state.clone(),
        Element::new(
            "view-body",
            Container::default().preferred_size(Size::from([180, 240])),
        ),
        Element::new(
            "close",
            Button::new("Close", {
                let s = state.clone();
                move |_| s.cancel().map(|_| ())
            }),
        ),
    )
    .view_modal()
    .require_response();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Row::new([
                Element::new("view", dialog),
                Element::new(
                    "outside",
                    Button::new("Outside", move |_| c.set(c.get()? + 1).map(|_| ())),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    state.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::Escape);
    assert_eq!(state.status().get().unwrap(), DialogStatus::Open);
    let outside = find(&ui, &root, "outside").unwrap();
    ui.focus(&outside).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(count.get().unwrap(), 1);
    let view = find(&ui, &root, "view").unwrap();
    ui.set_visible(&view, false).unwrap();
    assert_eq!(
        state.take_outcome().unwrap(),
        Some(DialogOutcome::Dismissed(SurfaceDismissal::OwnerHidden))
    );
}
#[test]
fn submenu_right_left_escape_and_leaf_close_entire_chain() {
    let mut ui = ui();
    let count = ui.signal(0);
    let c = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "menu",
                Menu::new(
                    &ui,
                    "Actions",
                    vec![MenuItem::submenu(
                        "nested",
                        "Nested",
                        vec![
                            MenuItem::new("disabled", "Disabled", |_| panic!("disabled"))
                                .disabled(),
                            MenuItem::submenu(
                                "deep",
                                "Deep",
                                vec![MenuItem::new("leaf", "Apply", move |_| {
                                    c.set(c.get()? + 1).map(|_| ())
                                })],
                            ),
                        ],
                    )],
                )
                .unwrap(),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(&ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    let first = ui.focused();
    press(&mut ui, KeyCode::ArrowRight);
    assert_ne!(ui.focused(), first);
    let second = ui.focused();
    press(&mut ui, KeyCode::ArrowRight);
    assert_ne!(ui.focused(), second);
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(ui.focused(), second);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), first);
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(count.get().unwrap(), 1);
    assert_eq!(ui.focused(), Some(trigger));
    ui.paint().unwrap();
}
#[test]
fn context_menu_pointer_and_keyboard_open_and_validate_nested_keys() {
    let mut ui = ui();
    let count = ui.signal(0);
    let c = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "context",
            ContextMenu::new(
                &ui,
                Element::new("target", Button::new("Target", |_| Ok(()))),
                vec![MenuItem::new("action", "Apply", move |_| {
                    c.set(c.get()? + 1).map(|_| ())
                })],
            )
            .unwrap(),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    point(
        &mut ui,
        375.0,
        275.0,
        PointerKind::Press(PointerButton::Secondary),
    );
    point(
        &mut ui,
        375.0,
        275.0,
        PointerKind::Release(PointerButton::Secondary),
    );
    ui.flush(Duration::ZERO).unwrap();
    let items = find(&ui, &root, "items").unwrap();
    assert_eq!(ui.focused(), Some(items));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(count.get().unwrap(), 1);
    let target = find(&ui, &root, "target").unwrap();
    ui.focus(&target).unwrap();
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::F10);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(count.get().unwrap(), 2);
    assert!(
        Menu::new(
            &ui,
            "Invalid",
            vec![MenuItem::submenu(
                "branch",
                "Branch",
                vec![
                    MenuItem::new("same", "One", |_| Ok(())),
                    MenuItem::new("same", "Two", |_| Ok(()))
                ]
            )]
        )
        .is_err()
    );
}
#[test]
fn toast_capacity_expiry_pending_promotion_and_shutdown_are_observable() {
    let mut ui = ui();
    let queue = ToastQueue::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(
                queue.clone(),
                Element::new("content", Label::new("Content")),
            ),
        ))
        .unwrap();
    let mut receipts = Vec::new();
    for i in 0..19 {
        receipts.push(queue.push(Toast::new(format!("Message {i}"))).unwrap());
    }
    assert!(matches!(
        queue.push(Toast::new("Overflow")),
        Err(RuntimeError::ToastFull)
    ));
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.focused().is_none());
    assert_eq!(receipts[0].status(), ToastStatus::Visible);
    assert_eq!(receipts[3].status(), ToastStatus::Pending);
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(5)));
    ui.flush(Duration::from_secs(5)).unwrap();
    assert_eq!(receipts[0].status(), ToastStatus::Expired);
    assert_eq!(receipts[3].status(), ToastStatus::Visible);
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(10)));
    ui.unmount(&root).unwrap();
    assert_eq!(receipts[3].status(), ToastStatus::OwnerRemoved);
    assert!(matches!(
        queue.push(Toast::new("Late")),
        Err(RuntimeError::Unmounted)
    ));
}
#[test]
fn toast_controls_follow_their_labels_and_sit_at_the_trailing_edge() {
    let mut ui = ui();
    let queue = ToastQueue::new(&ui);
    let _receipt = queue
        .push(Toast::new("A message long enough to wrap inside the stack").action("OK", |_| Ok(())))
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(
                queue.clone(),
                Element::new("content", Label::new("Content")),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let theme = ui.theme();
    let button = theme
        .palette(theme.mode().unwrap())
        .unwrap()
        .color(ColorRole::Button);
    // A themed button is drawn as its two rounded edges and the span between
    // them, so the control is the union of the touching fills in its color.
    let mut parts: Vec<[f32; 4]> = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color } if *color == button => Some(rect.values()),
            _ => None,
        })
        .collect();
    parts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut buttons: Vec<[f32; 4]> = Vec::new();
    for part in parts {
        match buttons.last_mut() {
            Some(last) if part[0] <= last[0] + last[2] + 0.5 => {
                last[2] = (part[0] + part[2] - last[0]).max(last[2]);
            }
            _ => buttons.push(part),
        }
    }
    assert_eq!(buttons.len(), 2, "the action and Dismiss: {buttons:?}");
    let [action, dismiss] = [buttons[0], buttons[1]];
    let width = |rect: [f32; 4]| rect[2];
    let x = |rect: [f32; 4]| rect[0];
    let stack = ui
        .bounds(&find(&ui, &root, "toasts").unwrap())
        .unwrap()
        .unwrap();
    let metrics = theme.metrics(theme.density().unwrap()).unwrap();
    let pad = metrics.get(MetricRole::ControlPaddingX);
    let gap = metrics.get(MetricRole::ItemGap);
    let minimum = metrics.get(MetricRole::ControlWidth);

    // Each control follows its own label rather than half the stack.
    assert!(
        width(dismiss) > width(action),
        "Dismiss is the longer word: {action:?} {dismiss:?}",
    );
    for rect in [action, dismiss] {
        assert!(
            width(rect) >= minimum,
            "{rect:?} is narrower than a control"
        );
        assert!(
            width(rect) < stack.width() / 2.0,
            "{rect:?} still takes half the stack",
        );
    }
    // Both sit at the trailing edge, one ItemGap apart.
    assert!(
        (x(dismiss) + width(dismiss) - (stack.x() + stack.width() - pad)).abs() < 0.5,
        "Dismiss is not against the trailing margin: {dismiss:?} in {stack:?}",
    );
    assert!(
        (x(dismiss) - (x(action) + width(action)) - gap).abs() < 0.5,
        "the controls are not one ItemGap apart: {action:?} {dismiss:?}",
    );
}

#[test]
fn narrow_toasts_stack_controls_and_clip_their_labels() {
    let mut ui = ui();
    ui.set_viewport(Viewport::new(120.0, 300.0, 1.0).unwrap())
        .unwrap();
    let queue = ToastQueue::new(&ui);
    let _receipt = queue
        .push(Toast::new("Saved").action("Apply", |_| Ok(())))
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(queue, Element::new("content", Label::new("Content"))),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let stack = ui
        .bounds(&find(&ui, &root, "toasts").unwrap())
        .unwrap()
        .unwrap();
    let theme = ui.theme();
    let button_text = theme
        .palette(theme.mode().unwrap())
        .unwrap()
        .color(ColorRole::OnButton);
    let scene = ui.paint().unwrap();
    let mut clips: Vec<Rect> = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask { color, clip, .. } if *color == button_text => *clip,
            _ => None,
        })
        .collect();
    clips.retain(|clip| clip.width() < stack.width());
    clips.sort_by(|a, b| a.y().total_cmp(&b.y()));
    clips.dedup();
    assert_eq!(clips.len(), 2, "both controls remain visible: {clips:?}");
    assert!(clips[0].y() + clips[0].height() <= clips[1].y());
    assert!(clips.iter().all(|clip| {
        clip.x() >= stack.x() && clip.x() + clip.width() <= stack.x() + stack.width()
    }));
}

#[test]
fn toast_focus_pauses_at_event_time_before_delayed_flush_and_action_failure_is_terminal() {
    let mut ui = ui();
    let queue = ToastQueue::new(&ui);
    let receipt = queue
        .push(Toast::new("Try action").action("Fail", |_| Err(RuntimeError::BorrowConflict)))
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(
                queue.clone(),
                Element::new("content", Label::new("Content")),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.flush(Duration::from_secs(2)).unwrap();
    let stack = find(&ui, &root, "toasts").unwrap();
    ui.focus(&stack).unwrap();
    ui.flush(Duration::from_secs(20)).unwrap();
    assert!(ui.next_deadline().is_none());
    ui.flush(Duration::from_secs(30)).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    assert_eq!(
        ui.key_input(KeyCode::Enter, KeyAction::Release),
        Err(RuntimeError::BorrowConflict)
    );
    ui.flush(Duration::from_secs(30)).unwrap();
    assert_eq!(receipt.status(), ToastStatus::ActionFailed);
    assert!(ui.next_deadline().is_none());
}
#[test]
fn banner_dismissal_removes_its_extent_and_does_not_schedule_expiry() {
    let mut ui = ui();
    let visible = ui.signal(true);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "banner",
                    Banner::new("Feedback", []).visible_signal(visible.read_only()),
                ),
                Element::new("after", Label::new("After")),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let after = find(&ui, &root, "after").unwrap();
    let before = ui.bounds(&after).unwrap().unwrap().y();
    visible.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.bounds(&after).unwrap().unwrap().y() < before);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn finish_take_open_requires_observed_close_and_resets_safe_focus() {
    let mut ui = ui();
    let state = DialogController::new(&ui);
    let root = ui.mount_tree(alert(&ui, &state, "alert")).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    state.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let accept = find(&ui, &root, "accept").unwrap();
    ui.focus(&accept).unwrap();
    state.accept().unwrap();
    assert_eq!(state.take_outcome().unwrap(), Some(DialogOutcome::Accepted));
    assert_eq!(state.open(), Err(RuntimeError::DialogBusy));
    ui.flush(Duration::ZERO).unwrap();
    state.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.key(&ui.focused().unwrap()).unwrap(), "cancel");
}
#[test]
fn modal_admission_failure_keeps_earlier_admission_blocking_until_recovery() {
    let mut ui = ui();
    let first = DialogController::new(&ui);
    let second = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                alert(&ui, &first, "first"),
                alert(&ui, &second, "second"),
                Element::new("outside", Button::new("Outside", |_| panic!("blocked"))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    first.open().unwrap();
    second.open().unwrap();
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::ModalConflict));
    let accepted_owner = find(&ui, &root, "first").unwrap();
    let accepted_cancel = find(&ui, &accepted_owner, "cancel").unwrap();
    ui.focus(&accepted_cancel).unwrap();
    ui.event(&accepted_cancel, |_| ()).unwrap();
    let outside = find(&ui, &root, "outside").unwrap();
    assert_eq!(ui.focus(&outside), Err(RuntimeError::ModalBlocked));
    assert_eq!(
        ui.event(&outside, |_| panic!("blocked event")),
        Err(RuntimeError::ModalBlocked)
    );
    ui.flush(Duration::ZERO).unwrap();
}
#[test]
fn closing_modal_uses_surviving_owner_target_when_opener_hidden() {
    let mut ui = ui();
    let state = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("external", Button::new("External", |_| Ok(()))),
                Element::new(
                    "owner",
                    Dialog::new(
                        state.clone(),
                        Element::new(
                            "view",
                            Column::new([
                                Element::new("opener", Button::new("Open", |_| Ok(()))),
                                Element::new("fallback", Button::new("Fallback", |_| Ok(()))),
                            ]),
                        ),
                        Element::new("action", Button::new("Action", |_| Ok(()))),
                    ),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let opener = find(&ui, &root, "opener").unwrap();
    ui.focus(&opener).unwrap();
    state.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.set_visible(&opener, false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "fallback").unwrap()));
}
#[test]
fn submenu_parent_leaf_activates_with_one_pointer_click() {
    let mut ui = ui();
    let count = ui.signal(0);
    let c = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "menu",
                Menu::new(
                    &ui,
                    "Menu",
                    vec![
                        MenuItem::submenu(
                            "branch",
                            "Branch",
                            vec![MenuItem::new("nested", "Nested", |_| Ok(()))],
                        ),
                        MenuItem::new("leaf", "Leaf", move |_| c.set(c.get()? + 1).map(|_| ())),
                    ],
                )
                .unwrap(),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&find(&ui, &root, "trigger").unwrap()).unwrap();
    press(&mut ui, KeyCode::Enter);
    let parent = ui.focused().unwrap();
    press(&mut ui, KeyCode::ArrowRight);
    let [x, y, _, h] = ui.bounds(&parent).unwrap().unwrap().values();
    point(
        &mut ui,
        x + 5.0,
        y + h * 0.75,
        PointerKind::Press(PointerButton::Primary),
    );
    point(
        &mut ui,
        x + 5.0,
        y + h * 0.75,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(count.get().unwrap(), 1);
}
#[test]
fn toast_hidden_elapsed_expiry_preserves_pending_lifetime_and_receipts_survive_shutdown() {
    let mut ui = ui();
    let queue = ToastQueue::new(&ui);
    let mut receipts = Vec::new();
    for _ in 0..4 {
        receipts.push(queue.push(Toast::new("Timed")).unwrap());
    }
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(
                queue.clone(),
                Element::new("content", Label::new("Content")),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.flush(Duration::from_secs(2)).unwrap();
    ui.set_visible(&root, false).unwrap();
    ui.flush(Duration::from_secs(20)).unwrap();
    assert!(ui.next_deadline().is_none());
    ui.set_visible(&root, true).unwrap();
    ui.flush(Duration::from_secs(20)).unwrap();
    assert_eq!(receipts[0].status(), ToastStatus::Expired);
    assert_eq!(receipts[3].status(), ToastStatus::Visible);
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(25)));
    drop(ui);
    assert_eq!(receipts[3].status(), ToastStatus::OwnerRemoved);
    let ui = Runtime::new();
    let q = ToastQueue::new(&ui);
    let receipt = q.push(Toast::new("No host yet")).unwrap();
    drop(ui);
    assert_eq!(receipt.status(), ToastStatus::OwnerRemoved);
}
#[test]
fn toast_hover_pause_and_resume_account_event_time_even_when_flush_is_delayed() {
    let mut ui = ui();
    let q = ToastQueue::new(&ui);
    let receipt = q.push(Toast::new("Hover")).unwrap();
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(q, Element::new("content", Label::new("Content"))),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.flush(Duration::from_secs(2)).unwrap();
    let stack = find(&ui, &root, "toasts").unwrap();
    let [x, y, _, _] = ui.bounds(&stack).unwrap().unwrap().values();
    point(&mut ui, x + 5.0, y + 5.0, PointerKind::Move);
    ui.flush(Duration::from_secs(20)).unwrap();
    assert_eq!(receipt.status(), ToastStatus::Visible);
    assert!(ui.next_deadline().is_none());
    point(&mut ui, 0.0, 0.0, PointerKind::Move);
    ui.flush(Duration::from_secs(22)).unwrap();
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(23)));
    ui.flush(Duration::from_secs(23)).unwrap();
    assert_eq!(receipt.status(), ToastStatus::Expired);
}
#[test]
fn toast_rejects_oversized_text_and_scrolls_long_content_with_keyboard() {
    let mut ui = ui();
    let q = ToastQueue::new(&ui);
    assert!(matches!(
        q.push(Toast::new("x".repeat(16 * 1024 + 1))),
        Err(RuntimeError::TextEdit(TextEditError::TooLarge))
    ));
    let receipt = q
        .push(Toast::new("Long feedback line.\n".repeat(35)))
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(q, Element::new("content", Label::new("Content"))),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&find(&ui, &root, "toasts").unwrap()).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    let snapshot = ui.inspect();
    let stack = snapshot
        .components
        .iter()
        .find(|c| c.key == "toasts")
        .unwrap();
    assert!(
        matches!(stack.details.properties.get("scroll_offset"), Some(InspectValue::Number(value)) if *value > 0.0)
    );
    ui.paint().unwrap();
    press(&mut ui, KeyCode::Delete);
    assert_eq!(receipt.status(), ToastStatus::Dismissed);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn toast_blur_resumes_at_event_time_and_action_can_enqueue_followup() {
    let mut ui = ui();
    let q = ToastQueue::new(&ui);
    let post = q.clone();
    let followup = std::rc::Rc::new(std::cell::RefCell::new(None));
    let observed = followup.clone();
    let receipt = q
        .push(Toast::new("Initial").action("Next", move |_| {
            *observed.borrow_mut() = Some(post.push(Toast::new("Follow-up"))?);
            Ok(())
        }))
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(
                q,
                Element::new("outside", Button::new("Outside", |_| Ok(()))),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.flush(Duration::from_secs(2)).unwrap();
    let stack = find(&ui, &root, "toasts").unwrap();
    ui.focus(&stack).unwrap();
    ui.flush(Duration::from_secs(20)).unwrap();
    let outside = find(&ui, &root, "outside").unwrap();
    ui.focus(&outside).unwrap();
    ui.flush(Duration::from_secs(22)).unwrap();
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(23)));
    ui.focus(&stack).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    ui.flush(Duration::from_secs(22)).unwrap();
    assert_eq!(receipt.status(), ToastStatus::ActionSucceeded);
    assert_eq!(
        followup.borrow().as_ref().unwrap().status(),
        ToastStatus::Visible
    );
}
#[test]
fn toast_navigation_cancels_armed_activation_instead_of_retargeting() {
    let mut ui = ui();
    let q = ToastQueue::new(&ui);
    let first = q.push(Toast::new("First")).unwrap();
    let second = q.push(Toast::new("Second")).unwrap();
    let root = ui
        .mount_tree(Element::new(
            "host",
            ToastHost::new(q, Element::new("content", Label::new("Content"))),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&find(&ui, &root, "toasts").unwrap()).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first.status(), ToastStatus::Visible);
    assert_eq!(second.status(), ToastStatus::Visible);
}

fn context_menu() -> (Runtime, WidgetId) {
    let mut ui = ui();
    let menu = ContextMenu::new(
        &ui,
        Element::new("target", Label::new("Target")),
        vec![MenuItem::new("action", "Apply", |_| Ok(()))],
    )
    .unwrap();
    let root = ui.mount_tree(Element::new("context", menu)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    (ui, root)
}

#[test]
fn unmounting_a_surface_owner_leaves_no_deadline_behind() {
    // Teardown finishes inside `unmount`: a host that unmounts a root and then
    // asks when to wake must be allowed to sleep, exactly as for a plain container.
    let (mut ui, root) = context_menu();
    assert_eq!(ui.next_deadline(), None);
    ui.unmount(&root).unwrap();
    assert_eq!(ui.next_deadline(), None);

    // Removing whatever holds focus schedules one flush to restore it. That is
    // the focus contract, not surface teardown: an open surface whose item list
    // is focused asks for exactly what a plain container with a focused button
    // asks for, and one flush settles both.
    let (mut ui, root) = context_menu();
    for kind in [
        PointerKind::Press(PointerButton::Secondary),
        PointerKind::Release(PointerButton::Secondary),
    ] {
        point(&mut ui, 5.0, 5.0, kind);
    }
    ui.flush(Duration::ZERO).unwrap();
    assert!(find(&ui, &root, "items").is_some_and(|items| ui.focused() == Some(items)));
    let mut plain = self::ui();
    let column = plain
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("button", Button::new("Plain", |_| Ok(())))]),
        ))
        .unwrap();
    plain.flush(Duration::ZERO).unwrap();
    let button = find(&plain, &column, "button").unwrap();
    plain.focus(&button).unwrap();
    plain.flush(Duration::ZERO).unwrap();
    for (mut ui, root) in [(ui, root), (plain, column)] {
        ui.unmount(&root).unwrap();
        assert_eq!(ui.focused(), None);
        assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(ui.next_deadline(), None);
    }
}

#[test]
fn unmounting_a_nested_surface_owner_still_schedules_the_parent_layout() {
    let mut ui = ui();
    let menu = ContextMenu::new(
        &ui,
        Element::new("target", Label::new("Target")),
        vec![MenuItem::new("action", "Apply", |_| Ok(()))],
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new("context", menu),
                Element::new("sibling", Label::new("Sibling")),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let context = find(&ui, &root, "context").unwrap();
    ui.unmount(&context).unwrap();
    // The surviving parent has to arrange its remaining child, so this deadline
    // is real work rather than leftover teardown, and one flush settles it.
    assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.next_deadline(), None);
}
#[test]
fn hiding_the_native_window_suspends_an_open_dialog_instead_of_ending_it() {
    let mut ui = ui();
    let state = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([alert(&ui, &state, "alert")]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(&ui, &root, "open-alert").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    let cancel = find(&ui, &root, "cancel").unwrap();
    assert_eq!(ui.focused(), Some(cancel.clone()));

    // Minimizing reports a hidden root and a zero-sized viewport.
    ui.set_window_visible(false).unwrap();
    ui.set_viewport(Viewport::new(0.0, 0.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(state.status().get().unwrap(), DialogStatus::Open);
    assert_eq!(state.take_outcome().unwrap(), None);

    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui.set_window_visible(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(state.status().get().unwrap(), DialogStatus::Open);
    assert_eq!(state.take_outcome().unwrap(), None);
    // The restored prompt is admitted again: focus is inside it and it blocks.
    assert_eq!(ui.focused(), Some(cancel));
    assert_eq!(ui.focus(&trigger), Err(RuntimeError::ModalBlocked));
    press(&mut ui, KeyCode::Escape);
    assert_eq!(
        state.take_outcome().unwrap(),
        Some(DialogOutcome::Cancelled)
    );
    assert_eq!(ui.focused(), Some(trigger));
}

#[test]
fn an_alert_describes_itself_and_can_block_every_window() {
    let mut ui = ui();
    let state = DialogController::new(&ui);
    ui.mount_tree(Element::new(
        "root",
        Column::new([alert(&ui, &state, "alert")]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    state.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let details = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "alert")
        .unwrap()
        .details;
    let semantics = details.semantics.unwrap();
    assert_eq!(semantics.role, SemanticRole::AlertDialog);
    assert_eq!(semantics.name.as_deref(), Some("Confirm"));
    assert_eq!(semantics.expanded, Some(true));
    assert!(semantics.actions.contains(&SemanticAction::Dismiss));
    assert_eq!(
        details.properties.get("open"),
        Some(&InspectValue::Bool(true))
    );
    assert_eq!(
        details.properties.get("window_modal"),
        Some(&InspectValue::Bool(true))
    );
    assert!(format!("{state:?}").contains("Open"));

    // An application-modal confirmation, such as a quit prompt, blocks other windows.
    let mut ui = self::ui();
    let quit = DialogController::new(&ui);
    let prompt = AlertDialog::new(
        quit.clone(),
        Element::new("main", Label::new("Main")),
        "Quit",
        "Discard changes?",
        "Quit",
    )
    .application_modal();
    ui.mount_tree(Element::new(
        "owner",
        Column::new([Element::new("quit", prompt)]),
    ))
    .unwrap();
    let other = ui
        .mount_tree(Element::new("other", Button::new("Other", |_| Ok(()))))
        .unwrap();
    let prepared = ui
        .prepare_detach(&other, NativeWindowOptions::new("Other").unwrap())
        .unwrap();
    let _detached = ui.commit_detach(prepared).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    ui.select_window(&ui.primary_window()).unwrap();
    quit.open().unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    assert_eq!(ui.focus(&other), Err(RuntimeError::ModalBlocked));
    let modal = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "quit")
        .unwrap()
        .details;
    assert_eq!(
        modal.properties.get("window_modal"),
        Some(&InspectValue::Bool(false))
    );
}

/// A custom surface owner that records every dismissal reason it is given.
struct Recorder {
    children: Vec<Element>,
    open: Signal<bool>,
    shown: bool,
    reasons: std::rc::Rc<std::cell::RefCell<Vec<SurfaceDismissal>>>,
}
impl Recorder {
    fn new(
        open: &Signal<bool>,
        reasons: &std::rc::Rc<std::cell::RefCell<Vec<SurfaceDismissal>>>,
    ) -> Self {
        Self {
            children: vec![
                Element::new("anchor", Button::new("Anchor", |_| Ok(()))),
                Element::new("popup", Label::new("Surface")),
            ],
            open: open.clone(),
            shown: false,
            reasons: reasons.clone(),
        }
    }
}
impl Component for Recorder {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.shown = cx.read(&self.open.read_only())?;
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "popup" || self.shown
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "popup").then_some(SurfaceKind::Popover)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.reasons.borrow_mut().push(reason);
        self.shown = false;
        self.open.set(false).map(|_| ())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        cx.measure(0, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::from_size(size))?;
        if cx.len() == 2 {
            let popup = cx.measure(1, Constraints::new(None, None)?)?;
            cx.place(
                1,
                Rect::new(0.0, size.height(), popup.width(), popup.height())?,
            )?;
        }
        Ok(())
    }
}

#[test]
fn a_custom_surface_owner_is_told_why_its_surface_closed() {
    let mut ui = ui();
    let reasons = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let others = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let (first, second) = (ui.signal(false), ui.signal(false));
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("first", Recorder::new(&first, &reasons)),
                Element::new("second", Recorder::new(&second, &others)),
                Element::new("elsewhere", Button::new("Elsewhere", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let anchor = find(&ui, &root, "first")
        .and_then(|first| find(&ui, &first, "anchor"))
        .unwrap();
    let elsewhere = find(&ui, &root, "elsewhere").unwrap();
    let open = |ui: &mut Runtime| {
        first.set(true).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert!(first.get().unwrap());
    };
    let last = |reasons: &std::cell::RefCell<Vec<SurfaceDismissal>>| {
        reasons.borrow_mut().drain(..).collect::<Vec<_>>()
    };

    // A press outside the owner and its surface.
    open(&mut ui);
    let [x, y, w, h] = ui.bounds(&elsewhere).unwrap().unwrap().values();
    let (x, y) = (x + w / 2.0, y + h / 2.0);
    point(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    point(&mut ui, x, y, PointerKind::Release(PointerButton::Primary));
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(last(&reasons), [SurfaceDismissal::OutsidePress]);

    // Keyboard focus leaving the owner.
    ui.focus(&anchor).unwrap();
    open(&mut ui);
    ui.focus(&elsewhere).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(last(&reasons), [SurfaceDismissal::FocusLeft]);

    // The native window losing its input context.
    open(&mut ui);
    ui.clear_focus();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(last(&reasons), [SurfaceDismissal::WindowFocusLost]);

    // Another independent surface opening.
    open(&mut ui);
    second.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(last(&reasons), [SurfaceDismissal::Replaced]);
    second.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // The anchor scrolled out of the window.
    open(&mut ui);
    let [_, top, _, _] = ui.bounds(&anchor).unwrap().unwrap().values();
    ui.set_viewport(Viewport::new(400.0, top / 2.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(last(&reasons), [SurfaceDismissal::AnchorClipped]);
}

#[test]
fn a_context_menu_wrapper_is_not_a_tab_stop() {
    let mut ui = ui();
    let count = ui.signal(0);
    let c = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "context",
                ContextMenu::new(
                    &ui,
                    Element::new("target", Button::new("Target", |_| Ok(()))),
                    vec![MenuItem::new("action", "Apply", move |_| {
                        c.set(c.get()? + 1).map(|_| ())
                    })],
                )
                .unwrap(),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let target = find(&ui, &root, "target").unwrap();
    ui.focus_next(false).unwrap();
    assert_eq!(ui.focused(), Some(target.clone()));
    ui.focus_next(false).unwrap();
    assert_eq!(ui.focused(), Some(target), "the content is the only stop");
    // Shift+F10 still reaches the wrapper from its focused content.
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::F10);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(count.get().unwrap(), 1);
    // So does the ContextMenu (Menu) key, without a modifier.
    press(&mut ui, KeyCode::ContextMenu);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(count.get().unwrap(), 2);
}
