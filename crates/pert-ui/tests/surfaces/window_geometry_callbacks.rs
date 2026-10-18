//! Committed geometry reports through the public facade: `FloatingWindow`
//! movement restricted by a Permission, `FloatingWindow::on_frame` with its
//! origins, and `ToolWindowChangeKind::Geometry` for docked and floating tool
//! windows. Previews and cancelled gestures report nothing.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};
use support::{settle, widget};

type Frames = Rc<RefCell<Vec<(WindowPlacement, ChangeOrigin)>>>;

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    ui
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
fn press(ui: &mut Runtime, code: KeyCode) {
    ui.key_input(code, KeyAction::Press).unwrap();
    ui.key_input(code, KeyAction::Release).unwrap();
    settle(ui);
}
fn recorder(
    frames: &Frames,
) -> impl FnMut(&mut EventContext, WindowPlacement, ChangeOrigin) -> Result<(), RuntimeError> + 'static
{
    let frames = frames.clone();
    move |_, placement, origin| {
        frames.borrow_mut().push((placement, origin));
        Ok(())
    }
}
fn point(x: f32, y: f32) -> Point {
    Point::new(x, y).unwrap()
}
fn window(ui: &Runtime, frames: Option<&Frames>) -> FloatingWindow {
    let window = FloatingWindow::new(
        ui,
        "one",
        "One",
        Element::new("body", Label::new("Content")),
    )
    .position(point(20.0, 20.0))
    .unwrap()
    .initial_size(Size::from([300, 200]))
    .unwrap();
    match frames {
        Some(frames) => window.on_frame(recorder(frames)),
        None => window,
    }
}

#[test]
fn a_revoked_move_restores_the_position_and_reports_nothing() {
    let mut ui = runtime();
    let movable = ui.signal(true);
    let frames = Frames::default();
    let w = window(&ui, Some(&frames)).movable(movable.read_only());
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    assert_eq!(&area(&ui, &one)[..2], &[80.0, 70.0], "the preview moves");
    assert!(frames.borrow().is_empty(), "a preview is not reported");
    movable.set(false).unwrap();
    settle(&mut ui);
    assert_eq!(
        &area(&ui, &one)[..2],
        &[20.0, 20.0],
        "the move is cancelled"
    );
    up(&mut ui, 100.0, 85.0);
    assert_eq!(&area(&ui, &one)[..2], &[20.0, 20.0]);
    // The title no longer starts a move while denied.
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    up(&mut ui, 100.0, 85.0);
    assert_eq!(&area(&ui, &one)[..2], &[20.0, 20.0]);
    assert!(frames.borrow().is_empty());
    // Granted again, a move commits.
    movable.set(true).unwrap();
    settle(&mut ui);
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    up(&mut ui, 100.0, 85.0);
    assert_eq!(&area(&ui, &one)[..2], &[80.0, 70.0]);
    assert_eq!(frames.borrow().len(), 1);
}

#[test]
fn a_fixed_denial_is_not_lifted_by_a_bound_permission() {
    let mut ui = runtime();
    let movable = ui.signal(true);
    let w = window(&ui, None)
        .movable(false)
        .movable(movable.read_only());
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    let one = widget(&ui, &root, "one");
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    up(&mut ui, 100.0, 85.0);
    assert_eq!(&area(&ui, &one)[..2], &[20.0, 20.0]);
}

#[test]
fn on_frame_reports_committed_moves_resizes_restores_and_controller_writes() {
    let mut ui = runtime();
    let frames = Frames::default();
    let w = window(&ui, Some(&frames));
    let state = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    assert!(frames.borrow().is_empty(), "mounting reports nothing");
    let saved = ui.capture_workspace().unwrap();
    let one = widget(&ui, &root, "one");

    // A move reports once, at release.
    down(&mut ui, 40.0, 35.0);
    pointer(&mut ui, 70.0, 55.0, PointerKind::Move);
    pointer(&mut ui, 100.0, 85.0, PointerKind::Move);
    assert!(frames.borrow().is_empty());
    up(&mut ui, 100.0, 85.0);
    {
        let frames = frames.borrow();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].0.position, point(80.0, 70.0));
        assert_eq!(frames[0].1, ChangeOrigin::User);
    }

    // A resize at the corner grip reports the new size.
    let [x, y, w, h] = area(&ui, &one);
    down(&mut ui, x + w - 4.0, y + h - 4.0);
    pointer(&mut ui, x + w + 36.0, y + h + 16.0, PointerKind::Move);
    up(&mut ui, x + w + 36.0, y + h + 16.0);
    {
        let frames = frames.borrow();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].0.size, Some(Size::from([340, 220])));
        assert_eq!(frames[1].1, ChangeOrigin::User);
    }

    // Escape cancels a move without a report.
    down(&mut ui, 100.0, 85.0);
    pointer(&mut ui, 200.0, 185.0, PointerKind::Move);
    press(&mut ui, KeyCode::Escape);
    up(&mut ui, 200.0, 185.0);
    assert_eq!(frames.borrow().len(), 2);

    // An Alt+arrow step on the focused frame commits as the user's.
    ui.focus(&one).unwrap();
    ui.key_input(KeyCode::AltLeft, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::ArrowRight);
    ui.key_input(KeyCode::AltLeft, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(frames.borrow().len(), 3);
    assert_eq!(frames.borrow()[2].0.position, point(88.0, 70.0));
    assert_eq!(frames.borrow()[2].1, ChangeOrigin::User);

    // A controller write is the application's.
    state.set_position(point(10.0, 12.0)).unwrap();
    settle(&mut ui);
    assert_eq!(
        frames
            .borrow()
            .last()
            .copied()
            .map(|(p, o)| (p.position, o)),
        Some((point(10.0, 12.0), ChangeOrigin::Application))
    );
    // A write that changes nothing calls nothing.
    state.set_position(point(10.0, 12.0)).unwrap();
    settle(&mut ui);
    assert_eq!(frames.borrow().len(), 4);

    // Writes within one flush coalesce to the last placement and origin.
    state.set_position(point(30.0, 30.0)).unwrap();
    state.set_position(point(40.0, 40.0)).unwrap();
    settle(&mut ui);
    assert_eq!(frames.borrow().len(), 5);
    assert_eq!(frames.borrow()[4].0.position, point(40.0, 40.0));

    // A restore reports its origin.
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    settle(&mut ui);
    let last = *frames.borrow().last().unwrap();
    assert_eq!(last.0.position, point(20.0, 20.0));
    assert_eq!(last.1, ChangeOrigin::Restore);
}

#[test]
fn a_redescription_replaces_or_removes_the_frame_callback() {
    let mut ui = runtime();
    let first = Frames::default();
    let second = Frames::default();
    let w = window(&ui, Some(&first));
    let state = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    // The layer's children are reconciled by key; the mounted window keeps
    // its controller.
    ui.reconcile_children(&root, vec![window(&ui, Some(&second)).into_element()])
        .unwrap();
    settle(&mut ui);
    state.set_position(point(60.0, 60.0)).unwrap();
    settle(&mut ui);
    assert!(first.borrow().is_empty());
    assert_eq!(second.borrow().len(), 1);

    ui.reconcile_children(&root, vec![window(&ui, None).into_element()])
        .unwrap();
    settle(&mut ui);
    state.set_position(point(70.0, 70.0)).unwrap();
    settle(&mut ui);
    assert_eq!(second.borrow().len(), 1, "a removed callback is not called");
}

#[test]
fn a_failing_frame_callback_keeps_the_placement_and_is_not_replayed() {
    let mut ui = runtime();
    let calls = Rc::new(std::cell::Cell::new(0));
    let counter = calls.clone();
    let w = window(&ui, None).on_frame(move |_, _, _| {
        counter.set(counter.get() + 1);
        Err(RuntimeError::BorrowConflict)
    });
    let state = w.controller();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([w])))
        .unwrap();
    settle(&mut ui);
    state.set_position(point(90.0, 90.0)).unwrap();
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::BorrowConflict));
    settle(&mut ui);
    assert_eq!(calls.get(), 1);
    assert_eq!(&area(&ui, &widget(&ui, &root, "one"))[..2], &[90.0, 90.0]);
}

type Changes = Rc<RefCell<Vec<ToolWindowChange>>>;

fn tool_layout(changes: &Changes) -> ToolWindowLayout {
    let changes = changes.clone();
    ToolWindowLayout::new(Element::new("main", Button::new("Main", |_| Ok(()))))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(ToolWindow::new(
            "files",
            "Files",
            Element::new("files-body", Button::new("Files", |_| Ok(()))),
        ))
        .on_change(move |_, change| {
            changes.borrow_mut().push(change.clone());
            Ok(())
        })
}
fn rect(ui: &Runtime, root: &WidgetId, key: &str) -> Rect {
    ui.bounds(&widget(ui, root, key)).unwrap().unwrap()
}
fn geometry(changes: &Changes) -> Vec<ToolWindowChange> {
    changes
        .borrow()
        .iter()
        .filter(|change| change.kind == ToolWindowChangeKind::Geometry)
        .cloned()
        .collect()
}

#[test]
fn a_docked_panel_resize_reports_its_extent_once_it_ends() {
    let mut ui = runtime();
    let changes = Changes::default();
    let controller = ToolWindowController::new();
    let root = ui
        .mount_tree(Element::new(
            "frame",
            tool_layout(&changes).controller(&controller),
        ))
        .unwrap();
    settle(&mut ui);
    controller.expand("files").unwrap();
    settle(&mut ui);
    let before = rect(&ui, &root, "files").width();
    let divider = rect(&ui, &root, "divider-left");
    let (dx, dy) = (
        divider.x() + divider.width() / 2.0,
        divider.y() + divider.height() / 2.0,
    );
    down(&mut ui, dx, dy);
    pointer(&mut ui, dx + 20.0, dy, PointerKind::Move);
    pointer(&mut ui, dx + 40.0, dy, PointerKind::Move);
    assert!(
        geometry(&changes).is_empty(),
        "a resize in progress is not reported"
    );
    up(&mut ui, dx + 40.0, dy);
    let reported = geometry(&changes);
    assert_eq!(reported.len(), 1);
    assert_eq!(reported[0].key, "files");
    assert_eq!(reported[0].extent, Some(before + 40.0));
    assert_eq!(reported[0].origin, ChangeOrigin::User);

    // Escape cancels a resize without a report.
    let divider = rect(&ui, &root, "divider-left");
    let (dx, dy) = (
        divider.x() + divider.width() / 2.0,
        divider.y() + divider.height() / 2.0,
    );
    down(&mut ui, dx, dy);
    pointer(&mut ui, dx + 30.0, dy, PointerKind::Move);
    ui.focus(&widget(&ui, &root, "divider-left")).unwrap();
    press(&mut ui, KeyCode::Escape);
    up(&mut ui, dx + 30.0, dy);
    assert_eq!(geometry(&changes).len(), 1);
}

#[test]
fn a_floating_frame_reports_moves_and_a_restore() {
    let mut ui = runtime();
    let changes = Changes::default();
    let controller = ToolWindowController::new();
    let root = ui
        .mount_tree(Element::new(
            "frame",
            tool_layout(&changes).controller(&controller),
        ))
        .unwrap();
    settle(&mut ui);
    controller
        .set_mode("files", ToolWindowMode::Floating)
        .unwrap();
    controller.expand("files").unwrap();
    settle(&mut ui);
    let saved = ui.capture_workspace().unwrap();
    let start = rect(&ui, &root, "files");
    let (x, y) = (start.x() + 24.0, start.y() + 8.0);
    down(&mut ui, x, y);
    pointer(&mut ui, x + 40.0, y + 30.0, PointerKind::Move);
    assert!(geometry(&changes).is_empty());
    up(&mut ui, x + 40.0, y + 30.0);
    let reported = geometry(&changes);
    assert_eq!(reported.len(), 1);
    let frame = reported[0].floating.expect("a floating frame");
    assert_eq!((frame.x(), frame.y()), (start.x() + 40.0, start.y() + 30.0));
    assert_eq!(reported[0].origin, ChangeOrigin::User);
    assert_eq!(reported[0].mode, ToolWindowMode::Floating);

    // A click on the header without motion commits nothing new.
    let moved = rect(&ui, &root, "files");
    down(&mut ui, moved.x() + 24.0, moved.y() + 8.0);
    up(&mut ui, moved.x() + 24.0, moved.y() + 8.0);
    assert_eq!(geometry(&changes).len(), 1);

    // Restoring the saved arrangement reports the frame it applies.
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    settle(&mut ui);
    let reported = geometry(&changes);
    assert_eq!(reported.len(), 2);
    assert_eq!(reported[1].origin, ChangeOrigin::Restore);
    assert_eq!(
        reported[1].floating, None,
        "the saved arrangement had no frame of its own"
    );
}
