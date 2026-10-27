//! Owned popup placement, routing, focus and delayed help through the public facade.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
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
#[test]
fn popover_escapes_clip_focuses_and_consumes_outside_click() {
    let mut ui = ui();
    let open = ui.signal(false);
    let clicked = ui.signal(0);
    let writer = clicked.clone();
    let show = open.clone();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "clipped",
                    Container::new(Element::new(
                        "popover",
                        Popover::new(
                            open.clone(),
                            Element::new("trigger", Button::new("Open", move |_| show.set(true))),
                            Element::new("action", Button::new("Popup action", |_| Ok(()))),
                        ),
                    ))
                    .preferred_size(Size::from([150, 32])),
                ),
                Element::new(
                    "under",
                    Button::new("Underlying", move |_| writer.set(writer.get()? + 1)),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(&ui, &root, "trigger").unwrap();
    let action = find(&ui, &root, "action").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert!(open.get().unwrap());
    assert_eq!(ui.focused(), Some(action.clone()));
    let [x, y, w, h] = ui.bounds(&action).unwrap().unwrap().values();
    assert!(y >= 32.0);
    assert!(w > 0.0 && h > 0.0);
    ui.paint().unwrap();
    assert_eq!(
        point(
            &mut ui,
            390.0,
            290.0,
            PointerKind::Press(PointerButton::Primary)
        ),
        InputOutcome::Handled
    );
    point(
        &mut ui,
        390.0,
        290.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO).unwrap();
    assert!(!open.get().unwrap());
    assert_eq!(clicked.get().unwrap(), 0);
    assert_eq!(ui.focused(), Some(trigger.clone()));
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Escape);
    assert!(!open.get().unwrap());
    assert_eq!(ui.focused(), Some(trigger));
    let _ = x;
}
#[test]
fn select_highlight_is_tentative_and_menu_skips_disabled() {
    let mut ui = ui();
    let value = ui.signal(1);
    let count = ui.signal(0);
    let writer = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "dropdown",
                    Select::new(
                        &ui,
                        value.clone(),
                        [
                            Choice::new(1, "One"),
                            Choice::new(2, "Two").disabled(),
                            Choice::new(3, "Three"),
                        ],
                    )
                    .unwrap(),
                ),
                Element::new(
                    "menu",
                    Menu::new(
                        &ui,
                        "Actions",
                        vec![
                            MenuItem::new("disabled", "Disabled", |_| panic!("disabled action"))
                                .disabled(),
                            MenuItem::new("apply", "Apply", move |_| writer.set(writer.get()? + 1)),
                        ],
                    )
                    .unwrap(),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let anchor = find(&ui, &root, "select").unwrap();
    ui.focus(&anchor).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(value.get().unwrap(), 1);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(value.get().unwrap(), 1);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 3);
    assert_eq!(ui.focused(), Some(anchor));
    let trigger = find(&ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(count.get().unwrap(), 1);
    assert_eq!(ui.focused(), Some(trigger));
}
#[test]
fn the_menu_key_and_shift_f10_open_a_select_list_instead_of_an_enclosing_context_menu() {
    let mut ui = ui();
    let value = ui.signal(1);
    let inline = ui.signal(1);
    let count = ui.signal(0);
    let writer = count.clone();
    let choices = || {
        [
            Choice::new(1, "One"),
            Choice::new(2, "Two").disabled(),
            Choice::new(3, "Three"),
        ]
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "context",
                ContextMenu::new(
                    &ui,
                    Element::new(
                        "column",
                        Column::new([
                            Element::new(
                                "dropdown",
                                Select::new(&ui, value.clone(), choices()).unwrap(),
                            ),
                            Element::new(
                                "inline",
                                InlineSelect::new(&ui, inline.clone(), choices())
                                    .unwrap()
                                    .label("Inline"),
                            ),
                        ]),
                    ),
                    vec![MenuItem::new("apply", "Apply", move |_| {
                        writer.set(writer.get()? + 1)
                    })],
                )
                .unwrap(),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let dropdown = find(&ui, &root, "dropdown").unwrap();
    let anchor = find(&ui, &dropdown, "select").unwrap();
    ui.focus(&anchor).unwrap();
    // The Menu key opens the list; the next arrow moves past the disabled row.
    press(&mut ui, KeyCode::ContextMenu);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 3);
    assert_eq!(ui.focused(), Some(anchor));
    let inline_owner = find(&ui, &root, "inline").unwrap();
    let inline_anchor = find(&ui, &inline_owner, "select").unwrap();
    ui.focus(&inline_anchor).unwrap();
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::F10);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(inline.get().unwrap(), 3);
    assert_eq!(count.get().unwrap(), 0, "the wrapper's menu never opened");
}
#[test]
fn tooltip_uses_one_deadline_and_never_steals_focus() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "tooltip",
                Tooltip::new(
                    Element::new("button", Button::new("Help", |_| Ok(()))),
                    "Details",
                ),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    point(&mut ui, 10.0, 10.0, PointerKind::Move);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.next_deadline(), Some(Duration::from_millis(500)));
    ui.flush(Duration::from_millis(499)).unwrap();
    let _help = find(&ui, &root, "help").unwrap();
    assert!(
        !ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "help")
            .unwrap()
            .visible
    );
    ui.flush(Duration::from_millis(500)).unwrap();
    assert!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "help")
            .unwrap()
            .visible
    );
    assert!(ui.focused().is_none());
    assert!(ui.next_deadline().is_none());
    ui.paint().unwrap();
    point(&mut ui, 390.0, 290.0, PointerKind::Move);
    ui.flush(Duration::from_millis(500)).unwrap();
    assert!(
        !ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "help")
            .unwrap()
            .visible
    );
    assert!(ui.next_deadline().is_none());
}
#[test]
fn hiding_and_removal_close_owned_signals() {
    let mut ui = ui();
    let open = ui.signal(false);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "popover",
                Popover::new(
                    open.clone(),
                    Element::new("anchor", Button::new("Open", |_| Ok(()))),
                    Element::new("content", Button::new("Action", |_| Ok(()))),
                ),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(open.get().unwrap());
    ui.set_visible(&root, false).unwrap();
    assert!(!open.get().unwrap());
    ui.set_visible(&root, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.unmount(&root).unwrap();
    assert!(!open.get().unwrap());
}

#[test]
fn nested_escape_closes_top_layer_and_restores_each_opener() {
    let mut ui = ui();
    let outer = ui.signal(false);
    let inner = ui.signal(false);
    let show_outer = outer.clone();
    let show_inner = inner.clone();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "outer",
                Popover::new(
                    outer.clone(),
                    Element::new(
                        "outer-trigger",
                        Button::new("Outer", move |_| show_outer.set(true)),
                    ),
                    Element::new(
                        "inner",
                        Popover::new(
                            inner.clone(),
                            Element::new(
                                "inner-trigger",
                                Button::new("Inner", move |_| show_inner.set(true)),
                            ),
                            Element::new("inner-action", Button::new("Action", |_| Ok(()))),
                        ),
                    ),
                ),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let outer_trigger = find(&ui, &root, "outer-trigger").unwrap();
    let inner_trigger = find(&ui, &root, "inner-trigger").unwrap();
    ui.focus(&outer_trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ui.focused(), Some(inner_trigger.clone()));
    press(&mut ui, KeyCode::Enter);
    assert!(inner.get().unwrap());
    assert!(outer.get().unwrap());
    press(&mut ui, KeyCode::Escape);
    assert!(!inner.get().unwrap());
    assert!(outer.get().unwrap());
    assert_eq!(ui.focused(), Some(inner_trigger));
    press(&mut ui, KeyCode::Escape);
    assert!(!outer.get().unwrap());
    assert_eq!(ui.focused(), Some(outer_trigger));
}
#[test]
fn constrained_long_select_navigates_and_dynamic_choices_cancel_clicks() {
    let mut ui = ui();
    ui.set_viewport(Viewport::new(160.0, 160.0, 1.25).unwrap())
        .unwrap();
    let value = ui.signal(0);
    let choices = ui.signal(
        (0..30)
            .map(|i| Choice::new(i, format!("Option {i}")))
            .collect::<Vec<_>>(),
    );
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "dropdown",
                Select::bound(&ui, value.clone(), choices.read_only()),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let anchor = find(&ui, &root, "select").unwrap();
    ui.focus(&anchor).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 29);
    ui.paint().unwrap();
    press(&mut ui, KeyCode::Enter);
    let options = find(&ui, &root, "options").unwrap();
    let [x, y, _, _] = ui.bounds(&options).unwrap().unwrap().values();
    point(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    choices.set(vec![Choice::new(100, "Replacement")]).unwrap();
    point(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(value.get().unwrap(), 29);
}
#[test]
fn bottom_anchor_flips_and_clipped_anchor_dismisses() {
    let mut ui = ui();
    let open = ui.signal(false);
    let show = open.clone();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "space",
                    Container::default().preferred_size(Size::from([100, 210])),
                ),
                Element::new(
                    "popover",
                    Popover::new(
                        open.clone(),
                        Element::new("trigger", Button::new("Open", move |_| show.set(true))),
                        Element::new("content", Button::new("Action", |_| Ok(()))),
                    ),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(&ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    let content = find(&ui, &root, "content").unwrap();
    let [_, ay, _, _] = ui.bounds(&trigger).unwrap().unwrap().values();
    let [_, py, _, _] = ui.bounds(&content).unwrap().unwrap().values();
    assert!(py < ay);
    ui.set_viewport(Viewport::new(400.0, 100.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!open.get().unwrap());
}

#[test]
fn directly_hiding_owner_closes_signal_and_does_not_reopen() {
    let mut ui = ui();
    let open = ui.signal(false);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "owner",
                Popover::new(
                    open.clone(),
                    Element::new("trigger", Button::new("Open", |_| Ok(()))),
                    Element::new("action", Button::new("Action", |_| Ok(()))),
                ),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let owner = find(&ui, &root, "owner").unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.set_visible(&owner, false).unwrap();
    assert!(!open.get().unwrap());
    ui.set_visible(&owner, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!open.get().unwrap());
}

struct ExternalSurface {
    open: Signal<bool>,
    fail: Signal<bool>,
    shown: bool,
    children: Vec<Element>,
}
impl Component for ExternalSurface {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.shown = cx.read(&self.open)?;
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
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        if self.fail.get()? {
            return Err(RuntimeError::BorrowConflict);
        }
        self.open.set(false)?;
        self.shown = false;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([100, 32]))
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for i in 0..cx.len() {
            cx.place(
                i,
                Rect::new(0.0, if i == 0 { 0.0 } else { size.height() }, 100.0, 32.0).unwrap(),
            )?;
        }
        Ok(())
    }
}
#[test]
fn custom_surface_failure_preserves_routing_and_consumed_release() {
    let mut ui = ui();
    let open = ui.signal(false);
    let fail = ui.signal(false);
    let show = open.clone();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "custom",
                    ExternalSurface {
                        open: open.clone(),
                        fail: fail.clone(),
                        shown: false,
                        children: vec![
                            Element::new("trigger", Button::new("Open", move |_| show.set(true))),
                            Element::new("popup", Button::new("Action", |_| Ok(()))),
                        ],
                    },
                ),
                Element::new("outside", Button::new("Outside", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(&ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    fail.set(true).unwrap();
    assert_eq!(
        ui.pointer_input(PointerEvent::new(
            Point::new(390.0, 290.0).unwrap(),
            PointerKind::Press(PointerButton::Primary)
        )),
        Err(RuntimeError::BorrowConflict)
    );
    assert_eq!(
        point(
            &mut ui,
            390.0,
            290.0,
            PointerKind::Release(PointerButton::Primary)
        ),
        InputOutcome::Handled
    );
    let outside = find(&ui, &root, "outside").unwrap();
    ui.focus(&outside).unwrap();
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::BorrowConflict));
    // Recovery retries the retained registry; it neither loses Escape ownership
    // nor leaves a visible orphan surface after the callback becomes available.
    fail.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!open.get().unwrap());
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Escape);
    assert!(!open.get().unwrap());
}
#[test]
fn no_space_dismisses_without_focusing_an_invisible_surface() {
    let mut ui = ui();
    let open = ui.signal(true);
    ui.mount_tree(Element::new(
        "owner",
        Popover::new(
            open.clone(),
            Element::new("anchor", Button::new("Full viewport anchor", |_| Ok(()))),
            Element::new("action", Button::new("Invisible", |_| Ok(()))),
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!open.get().unwrap());
    assert!(ui.focused().is_none());
}
#[test]
fn list_wheel_scroll_is_retained_until_a_selection_is_made() {
    let mut ui = ui();
    let value = ui.signal(0);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "dropdown",
                Select::new(
                    &ui,
                    value.clone(),
                    (0..30).map(|i| Choice::new(i, format!("Option {i}"))),
                )
                .unwrap(),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let anchor = find(&ui, &root, "select").unwrap();
    ui.focus(&anchor).unwrap();
    press(&mut ui, KeyCode::Enter);
    let list = find(&ui, &root, "options").unwrap();
    let [x, y, _, _] = ui.bounds(&list).unwrap().unwrap().values();
    point(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Scroll(Point::new(0.0, 96.0).unwrap()),
    );
    ui.flush(Duration::ZERO).unwrap();
    point(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    point(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(value.get().unwrap(), 3);
}
#[test]
fn outside_press_over_an_actual_button_never_clicks_through() {
    let mut ui = ui();
    let open = ui.signal(false);
    let count = ui.signal(0);
    let show = open.clone();
    let writer = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "row",
                Row::new([
                    Element::new(
                        "owner",
                        Popover::new(
                            open.clone(),
                            Element::new("trigger", Button::new("Open", move |_| show.set(true))),
                            Element::new("action", Button::new("Action", |_| Ok(()))),
                        ),
                    ),
                    Element::new(
                        "underlying",
                        Button::new("Underlying", move |_| writer.set(writer.get()? + 1)),
                    ),
                ]),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(&ui, &root, "trigger").unwrap();
    let underlying = find(&ui, &root, "underlying").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    let [x, y, _, _] = ui.bounds(&underlying).unwrap().unwrap().values();
    point(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    point(
        &mut ui,
        x + 5.0,
        y + 5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(count.get().unwrap(), 0);
    assert!(!open.get().unwrap());
}

#[test]
fn failed_window_focus_dismissal_retries_the_original_reason_without_readmission() {
    let mut ui = ui();
    let open = ui.signal(false);
    let fail = ui.signal(false);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "owner",
                ExternalSurface {
                    open: open.clone(),
                    fail: fail.clone(),
                    shown: false,
                    children: vec![
                        Element::new("trigger", Button::new("Open", |_| Ok(()))),
                        Element::new("popup", Button::new("Action", |_| Ok(()))),
                    ],
                },
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&find(&ui, &root, "trigger").unwrap()).unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    fail.set(true).unwrap();
    ui.clear_focus();
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::BorrowConflict));
    fail.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!open.get().unwrap());
    assert!(ui.focused().is_none());
}

#[test]
fn deferred_error_before_admission_still_blocks_a_new_visible_modal() {
    let mut ui = ui();
    let open = ui.signal(false);
    let fail = ui.signal(false);
    let dialog = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "custom",
                    ExternalSurface {
                        open: open.clone(),
                        fail: fail.clone(),
                        shown: false,
                        children: vec![
                            Element::new("trigger", Button::new("Open", |_| Ok(()))),
                            Element::new("popup", Button::new("Action", |_| Ok(()))),
                        ],
                    },
                ),
                Element::new(
                    "dialog-owner",
                    AlertDialog::new(
                        dialog.clone(),
                        Element::new("dialog-trigger", Button::new("Confirm", |_| Ok(()))),
                        "Confirm",
                        "Proceed?",
                        "Accept",
                    ),
                ),
                Element::new("outside", Button::new("Outside", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    fail.set(true).unwrap();
    let custom = find(&ui, &root, "custom").unwrap();
    ui.set_visible(&custom, false).unwrap();
    dialog.open().unwrap();
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::BorrowConflict));
    let outside = find(&ui, &root, "outside").unwrap();
    assert_eq!(ui.focus(&outside), Err(RuntimeError::ModalBlocked));
    assert_eq!(
        ui.event(&outside, |_| panic!("blocked")),
        Err(RuntimeError::ModalBlocked)
    );
    fail.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.key(&ui.focused().unwrap()).unwrap(), "cancel");
}
#[test]
fn failed_hidden_or_removed_surface_does_not_leave_stale_dismissal_routing() {
    for remove in [false, true] {
        let mut ui = ui();
        let open = ui.signal(false);
        let fail = ui.signal(false);
        let count = ui.signal(0);
        let c = count.clone();
        let root = ui
            .mount_tree(Element::new(
                "column",
                Column::new([
                    Element::new(
                        "custom",
                        ExternalSurface {
                            open: open.clone(),
                            fail: fail.clone(),
                            shown: false,
                            children: vec![
                                Element::new("trigger", Button::new("Open", |_| Ok(()))),
                                Element::new("popup", Button::new("Action", |_| Ok(()))),
                            ],
                        },
                    ),
                    Element::new(
                        "outside",
                        Button::new("Outside", move |_| c.set(c.get()? + 1)),
                    ),
                ]),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        open.set(true).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        fail.set(true).unwrap();
        let custom = find(&ui, &root, "custom").unwrap();
        if remove {
            ui.unmount(&custom).unwrap();
        } else {
            ui.set_visible(&custom, false).unwrap();
        }
        assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::BorrowConflict));
        fail.set(false).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let outside = find(&ui, &root, "outside").unwrap();
        let [x, y, _, _] = ui.bounds(&outside).unwrap().unwrap().values();
        point(
            &mut ui,
            x + 4.0,
            y + 4.0,
            PointerKind::Press(PointerButton::Primary),
        );
        point(
            &mut ui,
            x + 4.0,
            y + 4.0,
            PointerKind::Release(PointerButton::Primary),
        );
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(count.get().unwrap(), 1);
    }
}

/// Whether the current scene contains a PanelBorder edge at the theme border width.
fn framed(ui: &mut Runtime) -> bool {
    let border = Palette::dark().color(ColorRole::PanelBorder);
    ui.paint().unwrap().commands().iter().any(|c| {
        matches!(c, DrawCommand::FillRect { rect, color } if *color == border && rect.height() == 1.0)
    })
}
#[test]
fn transient_surfaces_are_framed_only_while_presented() {
    let mut ui = ui();
    let value = ui.signal(1);
    let dialog = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "tooltip",
                    Tooltip::new(
                        Element::new("button", Button::new("Help", |_| Ok(()))),
                        "Details",
                    ),
                ),
                Element::new(
                    "dropdown",
                    Select::new(
                        &ui,
                        value.clone(),
                        [Choice::new(1, "One"), Choice::new(2, "Two")],
                    )
                    .unwrap(),
                ),
                Element::new(
                    "dialog-owner",
                    AlertDialog::new(
                        dialog.clone(),
                        Element::new("dialog-trigger", Button::new("Confirm", |_| Ok(()))),
                        "Confirm",
                        "Proceed?",
                        "Accept",
                    ),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!framed(&mut ui), "no transient surface is presented yet");

    let anchor = find(&ui, &root, "select").unwrap();
    ui.focus(&anchor).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    assert!(framed(&mut ui), "the select list has a PanelBorder frame");
    press(&mut ui, KeyCode::Escape);
    assert!(!framed(&mut ui));

    dialog.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(framed(&mut ui), "the dialog panel has a PanelBorder frame");
    press(&mut ui, KeyCode::Escape);
    assert!(!framed(&mut ui));

    // The runtime clock only advances, so the delayed help comes last.
    point(&mut ui, 10.0, 10.0, PointerKind::Move);
    ui.flush(Duration::ZERO).unwrap();
    ui.flush(Duration::from_millis(500)).unwrap();
    assert!(framed(&mut ui), "the tooltip has a PanelBorder frame");
    point(&mut ui, 390.0, 290.0, PointerKind::Move);
    ui.flush(Duration::from_millis(1000)).unwrap();
    assert!(!framed(&mut ui));
}

#[test]
fn a_surface_whose_anchor_leaves_no_room_is_refused_and_settles_closed() {
    fn popover(open: &Signal<bool>) -> Element {
        let opener = open.clone();
        Element::new(
            "popover",
            Popover::new(
                open.clone(),
                Element::new("opener", Button::new("Open", move |_| opener.set(true))),
                Element::new("content", Label::new("Surface")),
            ),
        )
    }
    fn activate(ui: &mut Runtime, root: &WidgetId) {
        let opener = find(ui, root, "opener").unwrap();
        ui.focus(&opener).unwrap();
        press(ui, KeyCode::Enter);
    }
    // A root is allocated the whole viewport, so its anchor spans the window and
    // there is no side to present on. The request is refused, not left pending.
    let mut ui = ui();
    let open = ui.signal(false);
    let root = ui.mount_tree(popover(&open)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.bounds(&root).unwrap().unwrap().height(),
        ui.viewport().height()
    );
    activate(&mut ui, &root);
    assert!(!open.get().unwrap(), "NoSpace returns the signal to false");
    assert_eq!(ui.next_deadline(), None, "a refused surface leaves no work");

    // The same popover inside a layout container keeps its own height and opens.
    let mut ui = self::ui();
    let open = ui.signal(false);
    let root = ui
        .mount_tree(Element::new("column", Column::new([popover(&open)])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    activate(&mut ui, &root);
    assert!(open.get().unwrap());
}

/// A popover whose content holds two actions; with `neighbors`, buttons
/// before and after it. Without them the anchor is the only focusable
/// component outside the popup, so Tab and Shift+Tab out of the popup reach it.
fn anchored_popover(ui: &mut Runtime, neighbors: bool) -> (WidgetId, Signal<bool>) {
    let open = ui.signal(false);
    let show = open.clone();
    let popover = Element::new(
        "popover",
        Popover::new(
            open.clone(),
            Element::new("trigger", Button::new("Open", move |_| show.set(true))),
            Element::new(
                "content",
                Column::new([
                    Element::new("first", Button::new("First", |_| Ok(()))),
                    Element::new("second", Button::new("Second", |_| Ok(()))),
                ]),
            ),
        ),
    );
    let children = if neighbors {
        vec![
            Element::new("before", Button::new("Before", |_| Ok(()))),
            popover,
            Element::new("after", Button::new("After", |_| Ok(()))),
        ]
    } else {
        vec![popover]
    };
    let root = ui
        .mount_tree(Element::new("root", Column::new(children)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    (root, open)
}

/// Presses Tab, with Shift held when `back`, and flushes.
fn tab(ui: &mut Runtime, back: bool) {
    if back {
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    }
    press(ui, KeyCode::Tab);
    if back {
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
            .unwrap();
    }
    ui.flush(Duration::ZERO).unwrap();
}

/// Tab through the native keyboard path, as the App delivers it.
fn native_tab(ui: &mut Runtime, back: bool) {
    let key = |ui: &mut Runtime, code: KeyCode, action: KeyAction| {
        ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
            .unwrap();
    };
    if back {
        key(ui, KeyCode::ShiftLeft, KeyAction::Press);
    }
    key(ui, KeyCode::Tab, KeyAction::Press);
    key(ui, KeyCode::Tab, KeyAction::Release);
    if back {
        key(ui, KeyCode::ShiftLeft, KeyAction::Release);
    }
    ui.flush(Duration::ZERO).unwrap();
}

fn focused_key(ui: &Runtime) -> Option<String> {
    ui.focused().map(|id| ui.key(&id).unwrap().to_owned())
}

#[test]
fn focus_moving_from_a_popover_to_its_own_anchor_closes_it() {
    for neighbors in [false, true] {
        for exit in [
            "tab",
            "shift-tab",
            "native-tab",
            "native-shift-tab",
            "focus",
        ] {
            let case = format!("{exit}, neighbors {neighbors}");
            let mut ui = ui();
            let (root, open) = anchored_popover(&mut ui, neighbors);
            let trigger = find(&ui, &root, "trigger").unwrap();
            ui.focus(&trigger).unwrap();
            press(&mut ui, KeyCode::Enter);
            ui.flush(Duration::ZERO).unwrap();
            let first = find(&ui, &root, "first").unwrap();
            let second = find(&ui, &root, "second").unwrap();
            assert_eq!(ui.focused(), Some(first.clone()), "{case}: admission focus");
            // Moving inside the popup keeps it open.
            tab(&mut ui, false);
            assert_eq!(ui.focused(), Some(second.clone()), "{case}: inside");
            assert!(open.get().unwrap(), "{case}: open while focus stays inside");
            match exit {
                "tab" => tab(&mut ui, false),
                "native-tab" => native_tab(&mut ui, false),
                "shift-tab" | "native-shift-tab" => {
                    ui.focus(&first).unwrap();
                    ui.flush(Duration::ZERO).unwrap();
                    if exit == "shift-tab" {
                        tab(&mut ui, true);
                    } else {
                        native_tab(&mut ui, true);
                    }
                }
                _ => {
                    ui.focus(&trigger).unwrap();
                    ui.flush(Duration::ZERO).unwrap();
                }
            }
            let landed = focused_key(&ui);
            assert!(
                !open.get().unwrap(),
                "{case}: focus left the popup for {landed:?} but it stayed open"
            );
            // Dismissal keeps focus where the user moved it.
            assert_ne!(landed.as_deref(), Some("first"), "{case}");
            assert_ne!(landed.as_deref(), Some("second"), "{case}");
            if !neighbors || exit == "focus" {
                assert_eq!(
                    landed.as_deref(),
                    Some("trigger"),
                    "{case}: reached the anchor"
                );
            }
        }
    }
}

#[test]
fn a_brief_departure_to_the_anchor_before_a_flush_still_closes_the_popover() {
    let mut ui = ui();
    let (root, open) = anchored_popover(&mut ui, false);
    let trigger = find(&ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    let first = find(&ui, &root, "first").unwrap();
    assert_eq!(ui.focused(), Some(first.clone()));
    // Focus leaves for the anchor and returns before anything synchronizes
    // the surfaces: the departure still counts.
    ui.focus(&trigger).unwrap();
    ui.focus(&first).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!open.get().unwrap(), "the popover stayed open");
}

/// Whether a Select's option list is presented.
fn options_shown(ui: &Runtime) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|c| c.key == "options" && c.visible)
}

#[test]
fn focus_moving_from_a_select_list_to_its_own_anchor_closes_it() {
    for exit in ["tab", "shift-tab", "focus"] {
        let mut ui = ui();
        let value = ui.signal(1);
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([Element::new(
                    "dropdown",
                    Select::new(
                        &ui,
                        value.clone(),
                        [Choice::new(1, "One"), Choice::new(3, "Three")],
                    )
                    .unwrap(),
                )]),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let anchor = find(&ui, &root, "select").unwrap();
        ui.focus(&anchor).unwrap();
        press(&mut ui, KeyCode::ArrowDown);
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(
            focused_key(&ui).as_deref(),
            Some("options"),
            "{exit}: list focused"
        );
        match exit {
            "tab" => tab(&mut ui, false),
            "shift-tab" => tab(&mut ui, true),
            _ => {
                ui.focus(&anchor).unwrap();
                ui.flush(Duration::ZERO).unwrap();
            }
        }
        assert_eq!(
            focused_key(&ui).as_deref(),
            Some("select"),
            "{exit}: reached the anchor"
        );
        assert!(
            !options_shown(&ui),
            "{exit}: the list stayed open on its anchor"
        );
        assert_eq!(value.get().unwrap(), 1, "{exit}: leaving chooses nothing");
    }
}

#[test]
fn short_tooltip_uses_content_width_and_stays_inside_the_window() {
    for left in [0, 300] {
        let mut ui = ui();
        let root = ui
            .mount_tree(Element::new(
                "column",
                Column::new([Element::new(
                    "row",
                    Row::new([
                        Element::new(
                            "spacer",
                            Container::default().preferred_size(Size::from([left, 32])),
                        ),
                        Element::new(
                            "tooltip",
                            Tooltip::new(
                                Element::new(
                                    "anchor",
                                    Container::default().preferred_size(Size::from([
                                        if left == 0 { 350 } else { 80 },
                                        32,
                                    ])),
                                ),
                                "Hi",
                            ),
                        ),
                    ])
                    .gap(Length::from(0)),
                )]),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        point(&mut ui, f32::from(left) + 10.0, 10.0, PointerKind::Move);
        ui.flush(Duration::ZERO).unwrap();
        ui.flush(Duration::from_millis(500)).unwrap();
        let tooltip = find(&ui, &root, "tooltip").unwrap();
        let popup = find(&ui, &tooltip, "popup").unwrap();
        let shown = ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "popup")
            .unwrap();
        assert!(shown.visible);
        let bounds = ui.bounds(&popup).unwrap().unwrap();
        assert!(bounds.width() < 100.0, "{bounds:?}");
        assert!(bounds.x() >= 0.0 && bounds.x() + bounds.width() <= 400.0);
        assert!(bounds.y() >= 32.0 && bounds.y() + bounds.height() <= 300.0);
        assert!(ui.focused().is_none());
        assert!(ui.next_deadline().is_none());
    }
}

#[test]
fn ordinary_popover_still_uses_anchor_width_as_its_minimum() {
    let mut ui = ui();
    let open = ui.signal(true);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "popover",
                Popover::new(
                    open.clone(),
                    Element::new(
                        "anchor",
                        Container::default().preferred_size(Size::from([350, 32])),
                    ),
                    Element::new("label", Label::new("Hi")),
                ),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(open.get().unwrap());
    let popup = find(&ui, &root, "popup").unwrap();
    assert_eq!(ui.bounds(&popup).unwrap().unwrap().width(), 350.0);
}
#[test]
fn tooltip_content_can_be_wider_than_its_anchor_with_viewport_clamping() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "tooltip",
                Tooltip::new(
                    Element::new(
                        "anchor",
                        Container::default().preferred_size(Size::from([80, 32])),
                    ),
                    "Longer help follows its text rather than the anchor width. ".repeat(3),
                ),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    point(&mut ui, 10.0, 10.0, PointerKind::Move);
    ui.flush(Duration::ZERO).unwrap();
    ui.flush(Duration::from_millis(500)).unwrap();
    let popup = find(&ui, &root, "popup").unwrap();
    let bounds = ui.bounds(&popup).unwrap().unwrap();
    assert!(bounds.width() > 80.0 && bounds.width() <= 400.0);
    assert!(bounds.x() >= 0.0 && bounds.x() + bounds.width() <= 400.0);
    assert!(bounds.y() >= 32.0 && bounds.y() + bounds.height() <= 300.0);
    assert!(ui.next_deadline().is_none());
}
#[test]
fn an_open_select_list_refuses_choices_after_the_dropdown_is_disabled() {
    let mut ui = ui();
    let value = ui.signal(1);
    let select = |ui: &Runtime, disabled: bool| {
        let select = Select::new(
            ui,
            value.clone(),
            [
                Choice::new(1, "One"),
                Choice::new(2, "Two"),
                Choice::new(3, "Three"),
            ],
        )
        .unwrap();
        if disabled { select.disabled() } else { select }
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("dropdown", select(&ui, false))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let anchor = find(&ui, &root, "select").unwrap();
    ui.focus(&anchor).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    let open = |ui: &Runtime| {
        ui.inspect()
            .components
            .iter()
            .any(|component| component.key == "options" && component.visible)
    };
    assert!(open(&ui));
    ui.reconcile_children(&root, vec![Element::new("dropdown", select(&ui, true))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(open(&ui), "the open list stays until dismissed");
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 1, "a keyboard choice is refused");
    let options = find(&ui, &root, "options").unwrap();
    let [x, y, w, h] = ui.bounds(&options).unwrap().unwrap().values();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        point(&mut ui, x + w / 2.0, y + h - 4.0, kind);
        ui.flush(Duration::ZERO).unwrap();
    }
    assert_eq!(value.get().unwrap(), 1, "a pointer choice is refused");
    if open(&ui) {
        press(&mut ui, KeyCode::Escape);
    }
    assert!(!open(&ui), "Escape closes the list");
}

/// A popover in a column whose anchor reports its dismissals.
fn reporting_popover(
    ui: &mut Runtime,
    open: &Signal<bool>,
    reasons: Option<&Signal<Vec<SurfaceDismissal>>>,
) -> Element {
    let mut popover = Popover::new(
        open.clone(),
        Element::new("trigger", Button::new("Open", |_| Ok(()))),
        Element::new("action", Button::new("Action", |_| Ok(()))),
    );
    if let Some(reasons) = reasons {
        let log = reasons.clone();
        popover = popover.on_dismiss(move |_, reason| {
            let mut seen = log.get()?;
            seen.push(reason);
            log.set(seen)
        });
    }
    let _ = ui;
    Element::new("owner", popover)
}

#[test]
fn on_dismiss_reports_framework_closes_but_not_the_applications_own() {
    let mut ui = ui();
    let open = ui.signal(false);
    let reasons = ui.signal(Vec::new());
    let popover = reporting_popover(&mut ui, &open, Some(&reasons));
    let root = ui
        .mount_tree(Element::new("column", Column::new([popover])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // Escape is the framework's close: reported after the signal is false.
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::Escape);
    assert!(!open.get().unwrap());
    assert_eq!(reasons.get().unwrap(), [SurfaceDismissal::Escape]);

    // The application's own close through the signal reports nothing.
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(reasons.get().unwrap().len(), 1);

    // Hiding the owner is the framework's close as well.
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let owner = find(&ui, &root, "owner").unwrap();
    ui.set_visible(&owner, false).unwrap();
    assert_eq!(
        reasons.get().unwrap(),
        [SurfaceDismissal::Escape, SurfaceDismissal::OwnerHidden]
    );
    ui.set_visible(&owner, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // A description without the handler removes it.
    let plain = reporting_popover(&mut ui, &open, None);
    ui.reconcile_children(&root, vec![plain]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::Escape);
    assert!(!open.get().unwrap());
    assert_eq!(reasons.get().unwrap().len(), 2);

    // And a description with another handler replaces it.
    let other = ui.signal(Vec::new());
    let replaced = reporting_popover(&mut ui, &open, Some(&other));
    ui.reconcile_children(&root, vec![replaced]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::Escape);
    assert_eq!(other.get().unwrap(), [SurfaceDismissal::Escape]);
    assert_eq!(reasons.get().unwrap().len(), 2);
}

#[test]
fn on_dismiss_is_silent_when_the_owner_of_a_closed_popover_is_hidden_or_removed() {
    let mut ui = ui();
    let open = ui.signal(false);
    let reasons = ui.signal(Vec::new());
    let popover = reporting_popover(&mut ui, &open, Some(&reasons));
    let root = ui
        .mount_tree(Element::new("column", Column::new([popover])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // Hiding the owner of a surface that was never opened reports nothing.
    let owner = find(&ui, &root, "owner").unwrap();
    ui.set_visible(&owner, false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.set_visible(&owner, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(reasons.get().unwrap().is_empty());

    // The application closes the surface through the signal, then hides the
    // owner: the close was its own, so the handler stays silent.
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.set_visible(&owner, false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(reasons.get().unwrap().is_empty());
    ui.set_visible(&owner, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // Closing through the signal and removing the owner in the same frame,
    // before any update read the signal, reports nothing either.
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(false).unwrap();
    ui.reconcile_children(&root, Vec::new()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(reasons.get().unwrap().is_empty());
    assert!(!open.get().unwrap());

    // An open surface whose owner is removed is still the framework's close.
    let popover = reporting_popover(&mut ui, &open, Some(&reasons));
    ui.reconcile_children(&root, vec![popover]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.reconcile_children(&root, Vec::new()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(reasons.get().unwrap(), [SurfaceDismissal::OwnerRemoved]);
    assert!(!open.get().unwrap());
}

#[test]
fn a_failed_dismissal_handler_is_not_replayed_when_the_runtime_retries() {
    let mut ui = ui();
    let open = ui.signal(false);
    let calls = ui.signal(Vec::new());
    let fail = ui.signal(true);
    let popover = {
        let calls = calls.clone();
        let fail = fail.clone();
        Popover::new(
            open.clone(),
            Element::new("trigger", Button::new("Open", |_| Ok(()))),
            Element::new("action", Button::new("Action", |_| Ok(()))),
        )
        .on_dismiss(move |_, reason| {
            let mut seen = calls.get()?;
            seen.push(reason);
            calls.set(seen)?;
            if fail.get()? {
                Err(RuntimeError::EmptyCommandName)
            } else {
                Ok(())
            }
        })
    };
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("owner", popover)]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    open.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // Hiding the owner defers the dismissal; the handler fails once and the
    // failure is reported, with the signal already false.
    let owner = find(&ui, &root, "owner").unwrap();
    ui.set_visible(&owner, false).unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::EmptyCommandName)
    );
    assert_eq!(calls.get().unwrap(), [SurfaceDismissal::OwnerHidden]);
    assert!(!open.get().unwrap());

    // The retried dismissal finds the surface closed and does not report the
    // same close again, whether or not the handler would now succeed.
    ui.flush(Duration::ZERO).unwrap();
    fail.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(calls.get().unwrap().len(), 1);
    assert!(!open.get().unwrap());

    // Showing the owner again does not reopen the surface or report anything.
    ui.set_visible(&owner, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(calls.get().unwrap().len(), 1);
    assert!(
        !ui.inspect()
            .components
            .iter()
            .any(|c| c.key == "action" && c.visible)
    );
}
