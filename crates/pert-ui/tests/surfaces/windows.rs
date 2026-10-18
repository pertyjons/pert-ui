//! Public in-view window identity, geometry, input, and lifecycle contracts.
use crate::support;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use support::{settle, widget};
fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn leaf() -> Element {
    Element::new("body", Label::new("Window content"))
}
fn window(ui: &Runtime, key: &str, x: f32, y: f32) -> FloatingWindow {
    FloatingWindow::new(ui, key, key, leaf())
        .position(Point::new(x, y).unwrap())
        .unwrap()
        .initial_size(Size::from([300, 200]))
        .unwrap()
        .minimizable(true)
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
    settle(ui);
}
fn down(ui: &mut Runtime, x: f32, y: f32) {
    pointer(ui, x, y, PointerKind::Press(PointerButton::Primary));
}
fn up(ui: &mut Runtime, x: f32, y: f32) {
    pointer(ui, x, y, PointerKind::Release(PointerButton::Primary));
}
fn area(ui: &Runtime, id: &WidgetId) -> [f32; 4] {
    ui.bounds(id).unwrap().unwrap().values()
}
fn role_count(ui: &mut Runtime, role: ColorRole) -> usize {
    let color = ui.theme().palette(ThemeMode::Dark).unwrap().color(role);
    ui.paint()
        .unwrap()
        .commands()
        .iter()
        .filter(|command| {
            matches!(command,
                DrawCommand::FillRect { color: fill, .. }
                | DrawCommand::DrawMask { color: fill, .. }
                if *fill == color)
        })
        .count()
}
fn activate(ui: &mut Runtime, id: &WidgetId) {
    ui.focus(id).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    settle(ui);
}

#[test]
fn window_drag_and_resize_targets_show_their_respective_hover_roles() {
    let mut ui = runtime();
    let first = window(&ui, "one", 20.0, 20.0);
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([first])))
        .unwrap();
    settle(&mut ui);
    let window = widget(&ui, &root, "one");
    let [x, y, w, h] = area(&ui, &window);
    let neutral = role_count(&mut ui, ColorRole::SurfaceHover);
    let resize = role_count(&mut ui, ColorRole::ResizeHandleHover);
    pointer(&mut ui, x + 30.0, y + 15.0, PointerKind::Move);
    assert!(role_count(&mut ui, ColorRole::SurfaceHover) > neutral);
    pointer(&mut ui, x + w - 6.0, y + h - 6.0, PointerKind::Move);
    assert!(role_count(&mut ui, ColorRole::ResizeHandleHover) > resize);
    pointer(&mut ui, 700.0, 500.0, PointerKind::Move);
    assert_eq!(role_count(&mut ui, ColorRole::ResizeHandleHover), resize);
}

#[test]
fn workspace_restore_refuses_a_drag_and_restores_committed_geometry() {
    let mut ui = runtime();
    let window = window(&ui, "one", 20.0, 20.0);
    let state = window.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([window])))
        .unwrap();
    settle(&mut ui);
    let saved = ui.capture_workspace().unwrap();
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    assert_eq!(
        ui.capture_workspace().unwrap(),
        saved,
        "tentative drag is not persisted"
    );
    assert!(
        ui.restore_workspace(&saved)
            .unwrap()
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    up(&mut ui, 100.0, 85.0);
    assert_ne!(ui.capture_workspace().unwrap(), saved);
    state.minimize().unwrap();
    settle(&mut ui);
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    settle(&mut ui);
    assert_eq!(state.status().get().unwrap(), WindowStatus::Open);
    assert_eq!(
        area(&ui, &widget(&ui, &root, "one")),
        [20.0, 20.0, 300.0, 200.0]
    );
}
#[test]
fn dragging_commit_escape_focus_loss_and_viewport_clamping_retain_identity() {
    let mut ui = runtime();
    let w = window(&ui, "one", 20.0, 20.0);
    let state = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    let body = widget(&ui, &one, "body");
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    assert_eq!(&area(&ui, &one)[..2], &[80.0, 70.0]);
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    settle(&mut ui);
    assert_eq!(&area(&ui, &one)[..2], &[20.0, 20.0]);
    up(&mut ui, 100.0, 85.0);
    ui.key_input(KeyCode::Escape, KeyAction::Release).unwrap();
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    ui.clear_focus();
    settle(&mut ui);
    assert_eq!(&area(&ui, &one)[..2], &[20.0, 20.0]);
    up(&mut ui, 100.0, 85.0);
    down(&mut ui, 40.0, 35.0);
    up(&mut ui, 100.0, 85.0);
    assert_eq!(&area(&ui, &one)[..2], &[80.0, 70.0]);
    assert_eq!(widget(&ui, &one, "body"), body);
    state
        .set_position(Point::new(700.0, 500.0).unwrap())
        .unwrap();
    settle(&mut ui);
    ui.set_viewport(Viewport::new(200.0, 120.0, 1.25).unwrap())
        .unwrap();
    settle(&mut ui);
    assert_eq!(&area(&ui, &one)[..2], &[152.0, 72.0]);
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert_eq!(&area(&ui, &one)[..2], &[700.0, 500.0]);
}
#[test]
fn resizing_clamps_and_lock_mid_drag_cancels_without_replacing_content() {
    let mut ui = runtime();
    let enabled = ui.signal(true);
    let w = window(&ui, "one", 20.0, 20.0)
        .resizable(enabled.read_only())
        .size_limits(
            SizeLimits::new(
                Size::from([240, 150]),
                Constraints::bounded(Size::from([360, 260])),
            )
            .unwrap(),
        );
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    let body = widget(&ui, &one, "body");
    down(&mut ui, 314.0, 214.0);
    pointer(&mut ui, 500.0, 400.0, PointerKind::Move);
    assert_eq!(&area(&ui, &one)[2..], &[360.0, 260.0]);
    assert_eq!(ui.cursor_icon(), CursorIcon::ResizeDiagonal);
    enabled.set(false).unwrap();
    settle(&mut ui);
    assert_eq!(&area(&ui, &one)[2..], &[300.0, 200.0]);
    up(&mut ui, 500.0, 400.0);
    enabled.set(true).unwrap();
    settle(&mut ui);
    down(&mut ui, 314.0, 214.0);
    up(&mut ui, 250.0, 150.0);
    assert_eq!(&area(&ui, &one)[2..], &[240.0, 150.0]);
    assert_eq!(widget(&ui, &one, "body"), body);
}

#[test]
fn reactive_resize_permission_cannot_lift_a_static_denial() {
    for signal_first in [false, true] {
        for fixed in [false, true] {
            let mut ui = runtime();
            let enabled = ui.signal(true);
            let mut w = window(&ui, "one", 20.0, 20.0);
            if signal_first {
                w = w.resizable(enabled.read_only());
            }
            w = if fixed {
                w.fixed_size(Size::from([300, 200])).unwrap()
            } else {
                w.resizable(false)
            };
            if !signal_first {
                w = w.resizable(enabled.read_only());
            }
            w = w.resizable(true);
            let root = ui
                .mount_tree(Element::new("layer", WindowLayer::new([w])))
                .unwrap();
            settle(&mut ui);
            let one = widget(&ui, &root, "one");
            let before = area(&ui, &one);
            down(&mut ui, 314.0, 214.0);
            up(&mut ui, 370.0, 260.0);
            assert_eq!(area(&ui, &one), before);
            let inspected = ui.inspect();
            assert_eq!(
                inspected
                    .components
                    .iter()
                    .find(|c| c.key == "one")
                    .unwrap()
                    .details
                    .properties["resizable"],
                InspectValue::Bool(false)
            );
        }
    }
}

#[test]
fn resize_release_rechecks_permission_before_an_explicit_flush() {
    let mut ui = runtime();
    let permission = ui.signal(true);
    let root = ui
        .mount_tree(Element::new(
            "layer",
            WindowLayer::new([window(&ui, "one", 20.0, 20.0).resizable(permission.read_only())]),
        ))
        .unwrap();
    settle(&mut ui);
    let before = ui.capture_workspace().unwrap();
    down(&mut ui, 314.0, 214.0);
    pointer(&mut ui, 360.0, 260.0, PointerKind::Move);
    permission.set(false).unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(400.0, 300.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    assert_eq!(ui.capture_workspace().unwrap(), before);
    assert_eq!(
        area(&ui, &widget(&ui, &root, "one")),
        [20.0, 20.0, 300.0, 200.0]
    );
}

#[test]
fn redescription_revokes_an_active_move_and_restores_original_preferences() {
    for (removes_bar, explicit_flush) in [false, true]
        .into_iter()
        .flat_map(|bar| [(bar, false), (bar, true)])
    {
        let mut ui = runtime();
        let root = ui
            .mount_tree(Element::new(
                "layer",
                WindowLayer::new([window(&ui, "one", 20.0, 20.0)]),
            ))
            .unwrap();
        settle(&mut ui);
        let before = ui.capture_workspace().unwrap();
        down(&mut ui, 40.0, 30.0);
        pointer(&mut ui, 90.0, 80.0, PointerKind::Move);
        let next = if removes_bar {
            window(&ui, "one", 20.0, 20.0).decorated(false)
        } else {
            window(&ui, "one", 20.0, 20.0).movable(false)
        };
        ui.reconcile_children(&root, vec![next.into_element()])
            .unwrap();
        if explicit_flush {
            settle(&mut ui);
        }
        up(&mut ui, 140.0, 130.0);
        assert_eq!(ui.capture_workspace().unwrap(), before);
        assert_eq!(&area(&ui, &widget(&ui, &root, "one"))[..2], &[20.0, 20.0]);
    }
}

#[test]
fn multiple_bound_resize_restrictions_all_remain_active() {
    let mut ui = runtime();
    let locked = ui.signal(false);
    let other = ui.signal(true);
    let root = ui
        .mount_tree(Element::new(
            "layer",
            WindowLayer::new([window(&ui, "one", 20.0, 20.0)
                .resizable(locked.read_only())
                .resizable(other.read_only())]),
        ))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    down(&mut ui, 314.0, 214.0);
    up(&mut ui, 360.0, 260.0);
    assert_eq!(area(&ui, &one), [20.0, 20.0, 300.0, 200.0]);
    locked.set(true).unwrap();
    settle(&mut ui);
    down(&mut ui, 314.0, 214.0);
    up(&mut ui, 360.0, 260.0);
    assert!(area(&ui, &one)[2] > 300.0);
}

#[test]
fn zero_motion_title_and_grip_clicks_preserve_clamped_and_intrinsic_preferences() {
    for intrinsic in [false, true] {
        let mut ui = runtime();
        let w = if intrinsic {
            FloatingWindow::new(&ui, "one", "One", leaf())
                .position(Point::new(700.0, 500.0).unwrap())
                .unwrap()
        } else {
            window(&ui, "one", 700.0, 500.0)
        };
        let root = ui
            .mount_tree(Element::new("layer", WindowLayer::new([w])))
            .unwrap();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
            .unwrap();
        settle(&mut ui);
        let one = widget(&ui, &root, "one");
        let before = ui.capture_workspace().unwrap();
        let [x, y, w, h] = area(&ui, &one);
        for (px, py) in [(x + 30.0, y + 15.0), (x + w - 6.0, y + h - 6.0)] {
            down(&mut ui, px, py);
            up(&mut ui, px, py);
            assert_eq!(ui.capture_workspace().unwrap(), before);
            down(&mut ui, px, py);
            pointer(&mut ui, px + 10.0, py + 10.0, PointerKind::Move);
            up(&mut ui, px, py);
            assert_eq!(ui.capture_workspace().unwrap(), before);
        }
    }
}
#[test]
fn fixed_size_and_undecorated_drag_bar_are_independent() {
    let mut ui = runtime();
    let w = window(&ui, "one", 20.0, 20.0)
        .decorated(false)
        .drag_area(Element::new("custom-title", Label::new("Drag here")))
        .fixed_size(Size::from([280, 100]))
        .unwrap();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    down(&mut ui, 294.0, 114.0);
    up(&mut ui, 350.0, 180.0);
    assert_eq!(area(&ui, &one), [20.0, 20.0, 280.0, 100.0]);
    down(&mut ui, 30.0, 30.0);
    up(&mut ui, 60.0, 70.0);
    assert_eq!(area(&ui, &one), [50.0, 60.0, 280.0, 100.0]);
}
#[test]
fn child_consumed_click_raises_window_without_changing_logical_order_or_clickthrough() {
    let mut ui = runtime();
    let a = ui.signal(0);
    let b = ui.signal(0);
    let button = |key: &str, count: Signal<i32>| {
        Element::new(
            key,
            Button::new("Increment", move |_| {
                count.set(count.get()? + 1)?;
                Ok(())
            }),
        )
    };
    let first = FloatingWindow::new(&ui, "first", "First", button("a", a.clone()))
        .position(Point::new(20.0, 20.0).unwrap())
        .unwrap()
        .fixed_size(Size::from([300, 200]))
        .unwrap();
    let second = FloatingWindow::new(&ui, "second", "Second", button("b", b.clone()))
        .position(Point::new(160.0, 60.0).unwrap())
        .unwrap()
        .fixed_size(Size::from([300, 200]))
        .unwrap();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([first, second])))
        .unwrap();
    settle(&mut ui);
    let logical = ui.children(&root).unwrap();
    down(&mut ui, 60.0, 85.0);
    up(&mut ui, 60.0, 85.0);
    assert_eq!(a.get().unwrap(), 1);
    assert_eq!(b.get().unwrap(), 0);
    // The first button now covers this part of the second window's button.
    down(&mut ui, 200.0, 110.0);
    up(&mut ui, 200.0, 110.0);
    assert_eq!(a.get().unwrap(), 2);
    assert_eq!(b.get().unwrap(), 0);
    assert_eq!(ui.children(&root).unwrap(), logical);
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    // Stable traversal reaches both windows despite activation changing paint order.
    let mut seen = Vec::new();
    for _ in 0..20 {
        ui.focus_next(false).unwrap();
        settle(&mut ui);
        seen.push(ui.focused().unwrap());
    }
    assert!(seen.contains(&widget(&ui, &root, "a")));
    assert!(seen.contains(&widget(&ui, &root, "b")));
}
struct Live {
    count: Rc<Cell<usize>>,
}
impl Component for Live {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.count.set(self.count.get() + 1);
        cx.animate()
    }
}
#[test]
fn minimize_suspends_content_deadlines_and_restores_identity_and_size() {
    let mut ui = runtime();
    let count = Rc::new(Cell::new(0));
    let w = FloatingWindow::new(
        &ui,
        "one",
        "One",
        Element::new(
            "live",
            Live {
                count: count.clone(),
            },
        ),
    )
    .initial_size(Size::from([300, 200]))
    .unwrap()
    .minimizable(true);
    let state = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    let live = widget(&ui, &one, "live");
    let n = count.get();
    state.minimize().unwrap();
    settle(&mut ui);
    assert!(area(&ui, &one)[3] < 100.0);
    assert!(ui.next_deadline().is_none());
    ui.flush(Duration::from_secs(2)).unwrap();
    assert_eq!(count.get(), n);
    state.restore().unwrap();
    settle(&mut ui);
    assert_eq!(area(&ui, &one)[3], 200.0);
    assert_eq!(widget(&ui, &one, "live"), live);
    assert!(count.get() > n);
    ui.unmount(&root).unwrap();
    assert_eq!(state.status().get().unwrap(), WindowStatus::Closed);
    assert_eq!(state.restore(), Err(RuntimeError::Unmounted));
}
#[test]
fn closing_tears_down_and_stale_handle_does_not_target_reused_key() {
    let mut ui = runtime();
    let w = window(&ui, "one", 0.0, 0.0);
    let old = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    let close = widget(&ui, &one, "window-close");
    let button = widget(&ui, &close, "button");
    activate(&mut ui, &button);
    assert!(ui.children(&root).unwrap().is_empty());
    assert_eq!(ui.bounds(&one), Err(RuntimeError::Unmounted));
    let w = window(&ui, "one", 0.0, 0.0);
    let fresh = w.controller();
    ui.reconcile_children(&root, vec![w.into_element()])
        .unwrap();
    settle(&mut ui);
    assert_ne!(widget(&ui, &root, "one"), one);
    old.close().unwrap();
    settle(&mut ui);
    assert_eq!(fresh.status().get().unwrap(), WindowStatus::Open);
    assert!(ui.next_deadline().is_none());
}
#[test]
fn close_veto_and_callback_error_keep_window_and_do_not_replay() {
    let mut ui = runtime();
    let calls = Rc::new(Cell::new(0));
    let n = calls.clone();
    // The application's own refusal, carried through the library callback.
    let refusal = RuntimeError::from(pert_ui::ApplicationError::from(Box::<
        dyn std::error::Error + Send + Sync,
    >::from("close refused")));
    let failure = refusal.clone();
    let w = window(&ui, "one", 0.0, 0.0).on_close(move |_| {
        n.set(n.get() + 1);
        if n.get() == 1 {
            Ok(WindowCloseAction::KeepOpen)
        } else {
            Err(failure.clone())
        }
    });
    let state = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let button = widget(&ui, &widget(&ui, &root, "window-close"), "button");
    activate(&mut ui, &button);
    assert_eq!(calls.get(), 1);
    assert_eq!(state.status().get().unwrap(), WindowStatus::Open);
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    assert_eq!(
        ui.key_input(KeyCode::Enter, KeyAction::Release),
        Err(refusal)
    );
    settle(&mut ui);
    assert_eq!(calls.get(), 2);
    assert_eq!(state.status().get().unwrap(), WindowStatus::Open);
    state.close().unwrap();
    settle(&mut ui);
    assert!(ui.children(&root).unwrap().is_empty());
}
#[test]
fn programmatic_placement_wins_during_drag_and_removal_cancels_capture() {
    let mut ui = runtime();
    let w = window(&ui, "one", 20.0, 20.0);
    let state = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    state
        .set_position(Point::new(400.0, 300.0).unwrap())
        .unwrap();
    settle(&mut ui);
    up(&mut ui, 100.0, 85.0);
    assert_eq!(&area(&ui, &one)[..2], &[400.0, 300.0]);
    down(&mut ui, 420.0, 315.0);
    ui.unmount(&one).unwrap();
    settle(&mut ui);
    up(&mut ui, 600.0, 400.0);
    assert!(ui.children(&root).unwrap().is_empty());
}
struct CountPaint {
    paints: Rc<Cell<u32>>,
    measures: Rc<Cell<u32>>,
}
impl Component for CountPaint {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        self.measures.set(self.measures.get() + 1);
        Ok(Size::from([100, 80]))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        self.paints.set(self.paints.get() + 1);
        p.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0)?, Color::ACCENT);
        Ok(())
    }
}
#[test]
fn pure_move_reuses_content_measurement_and_drawing() {
    let mut ui = runtime();
    let paints = Rc::new(Cell::new(0));
    let measures = Rc::new(Cell::new(0));
    let w = FloatingWindow::new(
        &ui,
        "one",
        "One",
        Element::new(
            "custom",
            CountPaint {
                paints: paints.clone(),
                measures: measures.clone(),
            },
        ),
    )
    .initial_size(Size::from([300, 200]))
    .unwrap();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    ui.paint().unwrap();
    let before = (paints.get(), measures.get());
    down(&mut ui, 20.0, 15.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    ui.paint().unwrap();
    up(&mut ui, 100.0, 85.0);
    ui.paint().unwrap();
    assert_eq!((paints.get(), measures.get()), before);
    assert_eq!(&area(&ui, &widget(&ui, &root, "one"))[..2], &[80.0, 70.0]);
    assert!(ui.next_deadline().is_none());
}
#[test]
fn foreign_runtime_and_invalid_chrome_limits_fail_without_backend() {
    let other = runtime();
    let mut ui = runtime();
    let w = window(&other, "foreign", 0.0, 0.0);
    ui.mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::ForeignRuntime));
    let mut ui = runtime();
    let w = window(&ui, "invalid", 0.0, 0.0).size_limits(
        SizeLimits::new(Size::ZERO, Constraints::bounded(Size::from([10, 10]))).unwrap(),
    );
    ui.mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Geometry(GeometryError::InvertedBounds))
    );
}

#[test]
fn popup_and_view_dialog_inside_window_obey_ownership_and_modality() {
    let mut ui = runtime();
    let open = ui.signal(false);
    let show = open.clone();
    let dialog = DialogController::new(&ui);
    let show_dialog = dialog.clone();
    let content = Element::new(
        "owned-dialog",
        Dialog::new(
            dialog.clone(),
            Element::new(
                "popup-owner",
                Popover::new(
                    open.clone(),
                    Element::new(
                        "open-popup",
                        Button::new("Popover", move |_| show.set(true).map(|_| ())),
                    ),
                    Element::new(
                        "popup-actions",
                        Button::new("Open dialog", move |_| show_dialog.open()),
                    ),
                ),
            ),
            Element::new(
                "modal-editor",
                TextField::new(ui.signal("Draft".to_owned())),
            ),
        )
        .view_modal(),
    );
    let first = FloatingWindow::new(&ui, "first", "First", content)
        .initial_size(Size::from([300, 220]))
        .unwrap()
        .minimizable(true);
    let state = first.controller();
    let other = FloatingWindow::new(
        &ui,
        "second",
        "Second",
        Element::new("other-button", Button::new("Other", |_| Ok(()))),
    )
    .position(Point::new(350.0, 30.0).unwrap())
    .unwrap()
    .initial_size(Size::from([300, 220]))
    .unwrap();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([first, other])))
        .unwrap();
    settle(&mut ui);
    let first = widget(&ui, &root, "first");
    let opener = widget(&ui, &first, "open-popup");
    activate(&mut ui, &opener);
    assert!(open.get().unwrap());
    let action = widget(&ui, &first, "popup-actions");
    activate(&mut ui, &action);
    assert_eq!(dialog.status().get().unwrap(), DialogStatus::Open);
    assert_eq!(ui.focus(&opener), Err(RuntimeError::ModalBlocked));
    let other = widget(&ui, &root, "other-button");
    ui.focus(&other).unwrap();
    settle(&mut ui);
    ui.paint().unwrap();
    state.minimize().unwrap();
    settle(&mut ui);
    assert!(!open.get().unwrap());
    assert_ne!(dialog.status().get().unwrap(), DialogStatus::Open);
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn hidden_parent_stops_work_and_retains_windows_until_reactivated() {
    let mut ui = runtime();
    let shown = ui.signal(true);
    let w = window(&ui, "one", 20.0, 20.0);
    let state = w.controller();
    let layer = Element::new("layer", WindowLayer::new([w]));
    let root = ui
        .mount_tree(Element::new(
            "view",
            View::new(layer).visible_signal(shown.read_only()),
        ))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    shown.set(false).unwrap();
    settle(&mut ui);
    up(&mut ui, 100.0, 85.0);
    assert!(ui.next_deadline().is_none());
    shown.set(true).unwrap();
    settle(&mut ui);
    assert_eq!(&area(&ui, &one)[..2], &[20.0, 20.0]);
    shown.set(false).unwrap();
    settle(&mut ui);
    state.close().unwrap();
    settle(&mut ui);
    assert!(ui.next_deadline().is_none());
    shown.set(true).unwrap();
    settle(&mut ui);
    assert_eq!(ui.bounds(&one), Err(RuntimeError::Unmounted));
}

struct HoverProbe(Rc<Cell<bool>>);
impl Component for HoverProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn hover_changed(&mut self, _: &mut EventContext, hovered: bool) {
        self.0.set(hovered);
    }
    fn focusable(&self) -> bool {
        true
    }
}
#[test]
fn focus_raising_refreshes_stationary_pointer_hover() {
    let mut ui = runtime();
    let a = Rc::new(Cell::new(false));
    let b = Rc::new(Cell::new(false));
    let first = FloatingWindow::new(
        &ui,
        "first",
        "First",
        Element::new("probe-a", HoverProbe(a.clone())),
    )
    .fixed_size(Size::from([300, 200]))
    .unwrap();
    let second = FloatingWindow::new(
        &ui,
        "second",
        "Second",
        Element::new("probe-b", HoverProbe(b.clone())),
    )
    .fixed_size(Size::from([300, 200]))
    .unwrap();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([first, second])))
        .unwrap();
    settle(&mut ui);
    pointer(&mut ui, 40.0, 80.0, PointerKind::Move);
    assert!(!a.get());
    assert!(b.get());
    ui.focus(&widget(&ui, &root, "probe-a")).unwrap();
    settle(&mut ui);
    assert!(a.get());
    assert!(!b.get());
}

#[test]
fn keyboard_resize_does_not_move_locked_windows_or_change_minimized_restore_size() {
    let mut ui = runtime();
    let w = window(&ui, "one", 20.0, 20.0);
    let state = w.controller();
    let fixed = window(&ui, "fixed", 350.0, 20.0)
        .fixed_size(Size::from([280, 160]))
        .unwrap();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w, fixed])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    let fixed = widget(&ui, &root, "fixed");
    ui.focus(&one).unwrap();
    state.minimize().unwrap();
    settle(&mut ui);
    ui.key_input(KeyCode::AltLeft, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowDown, KeyAction::Release)
        .unwrap();
    state.restore().unwrap();
    settle(&mut ui);
    assert_eq!(area(&ui, &one), [20.0, 20.0, 300.0, 200.0]);
    ui.focus(&fixed).unwrap();
    let before = area(&ui, &fixed);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    settle(&mut ui);
    assert_eq!(area(&ui, &fixed), before);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    settle(&mut ui);
    assert_eq!(area(&ui, &fixed)[0], before[0] + 8.0);
}

#[test]
fn keyboard_movement_starts_at_actual_clamped_position() {
    let mut ui = runtime();
    let w = window(&ui, "one", 700.0, 500.0);
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    ui.set_viewport(Viewport::new(200.0, 120.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    ui.focus(&one).unwrap();
    ui.key_input(KeyCode::AltLeft, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowLeft, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowLeft, KeyAction::Release)
        .unwrap();
    settle(&mut ui);
    assert_eq!(area(&ui, &one)[0], 144.0);
}

#[test]
fn blank_title_is_distinct_from_invalid_key_and_custom_bar_minima_are_respected() {
    let mut ui = runtime();
    let w = FloatingWindow::new(&ui, "valid-key", "  ", leaf());
    assert_eq!(
        ui.mount_tree(Element::new("layer", WindowLayer::new([w]))),
        Err(RuntimeError::EmptyWindowTitle)
    );
    let mut ui = runtime();
    let w = window(&ui, "one", 0.0, 0.0)
        .decorated(false)
        .drag_area(Element::new(
            "custom-button",
            Button::new("Drag bar action", |_| Ok(())),
        ))
        .fixed_size(Size::ZERO)
        .unwrap();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    let button = widget(&ui, &one, "custom-button");
    assert!(area(&ui, &one)[2] >= area(&ui, &button)[2]);
    // User action keys live separately from the standard action keys.
    let mut ui = runtime();
    let w = window(&ui, "one", 0.0, 0.0).title_action(Element::new(
        "window-close",
        Button::new("Custom", |_| Ok(())),
    ));
    ui.mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
}

#[test]
fn minimization_cancels_armed_content_and_late_release_cannot_activate() {
    let mut ui = runtime();
    let count = ui.signal(0);
    let n = count.clone();
    let w = FloatingWindow::new(
        &ui,
        "one",
        "One",
        Element::new(
            "action",
            Button::new("Action", move |_| n.set(n.get()? + 1).map(|_| ())),
        ),
    )
    .initial_size(Size::from([300, 200]))
    .unwrap();
    let state = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    down(&mut ui, 40.0, 80.0);
    state.minimize().unwrap();
    settle(&mut ui);
    up(&mut ui, 40.0, 80.0);
    assert_eq!(count.get().unwrap(), 0);
    state.restore().unwrap();
    settle(&mut ui);
    let button = widget(&ui, &root, "action");
    activate(&mut ui, &button);
    assert_eq!(count.get().unwrap(), 1);
}
#[test]
fn custom_drag_bar_controls_consume_input_and_empty_bar_space_moves() {
    let mut ui = runtime();
    let count = ui.signal(0);
    let n = count.clone();
    let w = window(&ui, "one", 20.0, 20.0)
        .decorated(false)
        .drag_area(Element::new(
            "bar",
            Row::new([
                Element::new(
                    "action",
                    Button::new("Action", move |_| n.set(n.get()? + 1).map(|_| ())),
                ),
                Element::new("label", Label::new("Drag")),
            ]),
        ));
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    pointer(&mut ui, 40.0, 35.0, PointerKind::Move);
    assert_eq!(ui.cursor_icon(), CursorIcon::Default);
    down(&mut ui, 40.0, 35.0);
    up(&mut ui, 40.0, 35.0);
    assert_eq!(count.get().unwrap(), 1);
    assert_eq!(&area(&ui, &one)[..2], &[20.0, 20.0]);
    down(&mut ui, 290.0, 35.0);
    up(&mut ui, 320.0, 65.0);
    assert_eq!(&area(&ui, &one)[..2], &[50.0, 50.0]);
}
#[test]
fn zero_viewport_and_large_finite_preferences_are_bounded_without_panics() {
    let mut ui = runtime();
    let w = window(&ui, "one", f32::MAX, f32::MAX)
        .initial_size(Size::new(f32::MAX, f32::MAX).unwrap())
        .unwrap()
        .size_limits(
            SizeLimits::new(Size::ZERO, Constraints::bounded(Size::from([500, 400]))).unwrap(),
        );
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    assert_eq!(area(&ui, &one), [752.0, 552.0, 500.0, 400.0]);
    ui.paint().unwrap();
    ui.set_viewport(Viewport::new(0.0, 0.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    ui.paint().unwrap();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert_eq!(area(&ui, &one), [752.0, 552.0, 500.0, 400.0]);
}

#[test]
fn frameless_icon_has_no_resting_fill_and_keeps_activation_and_hover() {
    let mut ui = runtime();
    let clicks = ui.signal(0);
    let n = clicks.clone();
    ui.mount_tree(Element::new(
        "button",
        Button::icon(Icon::Info, "Information", move |_| {
            n.set(n.get()? + 1).map(|_| ())
        })
        .frameless(),
    ))
    .unwrap();
    settle(&mut ui);
    let before = ui.paint().unwrap();
    assert!(
        !before
            .commands()
            .iter()
            .any(|c| matches!(c, DrawCommand::FillRect { .. }))
    );
    pointer(&mut ui, 20.0, 20.0, PointerKind::Move);
    let hovered = ui.paint().unwrap();
    assert!(
        hovered
            .commands()
            .iter()
            .any(|c| matches!(c, DrawCommand::FillRect { .. }))
    );
    down(&mut ui, 20.0, 20.0);
    up(&mut ui, 20.0, 20.0);
    assert_eq!(clicks.get().unwrap(), 1);
}
#[test]
fn icon_menu_opens_and_disabled_menu_does_not_activate() {
    let mut ui = runtime();
    let menu = Menu::icon(
        &ui,
        Icon::MoreHorizontal,
        "More",
        vec![MenuItem::new("item", "Item", |_| Ok(()))],
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("menu", menu)]),
        ))
        .unwrap();
    settle(&mut ui);
    let button = widget(&ui, &root, "button");
    activate(&mut ui, &button);
    assert!(
        ui.inspect()
            .components
            .iter()
            .any(|c| c.visible && c.details.properties.contains_key("presentation_layer"))
    );
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Escape, KeyAction::Release).unwrap();
    settle(&mut ui);
    ui.unmount(&root).unwrap();
    let menu = Menu::icon(
        &ui,
        Icon::MoreHorizontal,
        "Unavailable",
        vec![MenuItem::new("item", "Item", |_| Ok(())).disabled()],
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("menu", menu)]),
        ))
        .unwrap();
    settle(&mut ui);
    let button = widget(&ui, &root, "button");
    activate(&mut ui, &button);
    assert!(
        !ui.inspect()
            .components
            .iter()
            .any(|c| c.visible && c.details.properties.contains_key("presentation_layer"))
    );
}

#[test]
fn disabled_drag_bar_button_defers_cursor_and_drag_to_window() {
    let mut ui = runtime();
    let w = window(&ui, "one", 20.0, 20.0)
        .decorated(false)
        .drag_area(Element::new(
            "disabled",
            Button::new("Unavailable", |_| Ok(())).disabled(),
        ));
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    pointer(&mut ui, 40.0, 35.0, PointerKind::Move);
    assert_eq!(ui.cursor_icon(), CursorIcon::Move);
    down(&mut ui, 40.0, 35.0);
    up(&mut ui, 80.0, 75.0);
    assert_eq!(&area(&ui, &one)[..2], &[60.0, 60.0]);
}

#[test]
fn placement_builders_work_during_update_and_keep_an_extracted_controller() {
    struct Owner {
        built: bool,
        controller: Rc<std::cell::RefCell<Option<WindowController>>>,
    }
    impl Component for Owner {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            if !self.built {
                let window = FloatingWindow::new(cx, "window", "Window", leaf());
                *self.controller.borrow_mut() = Some(window.controller());
                let window = window
                    .position(Point::new(10.0, 20.0)?)?
                    .position(Point::new(40.0, 50.0)?)?
                    .initial_size(Size::from([200, 120]))?
                    .fixed_size(Size::from([250, 150]))?;
                cx.request_children(vec![Element::new("layer", WindowLayer::new([window]))]);
                self.built = true;
            }
            Ok(())
        }
    }
    let mut ui = runtime();
    let controller = Rc::new(std::cell::RefCell::new(None));
    let root = ui
        .mount_tree(Element::new(
            "owner",
            Owner {
                built: false,
                controller: controller.clone(),
            },
        ))
        .unwrap();
    settle(&mut ui);
    let window = widget(&ui, &root, "window");
    assert_eq!(area(&ui, &window), [40.0, 50.0, 250.0, 150.0]);
    controller
        .borrow()
        .as_ref()
        .unwrap()
        .set_position(Point::new(70.0, 80.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert_eq!(area(&ui, &window), [70.0, 80.0, 250.0, 150.0]);
}
