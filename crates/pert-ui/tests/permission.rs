//! The shared permission type and the resizing permissions built on it.
use pert_ui::*;
mod support;
use std::time::Duration;
use support::find;

fn runtime(width: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 80.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn leaf(key: &str) -> Element {
    Element::new(
        key,
        Container::default().preferred_size(Size::from([40, 40])),
    )
}
/// The first child's width, rounded to hundredths: a committed ratio
/// resolves to a width a few ulps away from the dragged position.
fn first(ui: &Runtime, split: &WidgetId) -> f32 {
    let width = ui
        .bounds(&ui.children(split).unwrap()[0])
        .unwrap()
        .unwrap()
        .width();
    (width * 100.0).round() / 100.0
}
fn pointer(ui: &mut Runtime, x: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, 20.0).unwrap(), kind))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
}
/// Drags the divider of a 208-wide equal split from its center by `delta`
/// and releases, returning the first child's width afterwards.
fn drag(ui: &mut Runtime, split: &WidgetId, from: f32, delta: f32) -> f32 {
    pointer(ui, from, PointerKind::Press(PointerButton::Primary));
    pointer(ui, from + delta, PointerKind::Move);
    pointer(
        ui,
        from + delta,
        PointerKind::Release(PointerButton::Primary),
    );
    first(ui, split)
}
fn resizable(ui: &Runtime, key: &str) -> InspectValue {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .unwrap()
        .details
        .properties["resizable"]
        .clone()
}
fn split(ui: &mut Runtime, pane: SplitPane) -> WidgetId {
    let id = ui.mount_tree(Element::new("split", pane)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    id
}
fn pane() -> SplitPane {
    SplitPane::new(SplitAxis::LeftRight, leaf("first"), leaf("second"))
}

#[test]
fn permission_values_compose_with_the_stricter_winning() {
    let ui = Runtime::new();
    let a = ui.signal(true);
    let b = ui.signal(true);
    assert_eq!(Permission::allowed().fixed(), Some(true));
    assert_eq!(Permission::default().fixed(), Some(true));
    assert_eq!(Permission::denied().fixed(), Some(false));
    assert_eq!(Permission::from(true).fixed(), Some(true));
    assert_eq!(Permission::from(false).fixed(), Some(false));
    // A fixed denial on either side wins over any signal.
    assert_eq!(
        Permission::from(false).and(a.read_only()).fixed(),
        Some(false)
    );
    assert_eq!(
        Permission::from(a.read_only()).and(false).fixed(),
        Some(false)
    );
    // Allowing never adds a condition.
    let bound = Permission::from(a.read_only()).and(true);
    assert_eq!(bound.fixed(), None);
    // Two signals: both must allow.
    let shared = b.read_only();
    let both = Permission::from(a.read_only()).and(&shared);
    assert!(both.get().unwrap());
    b.set(false).unwrap();
    assert!(!both.get().unwrap());
    a.set(false).unwrap();
    b.set(true).unwrap();
    assert!(!both.get().unwrap());
    a.set(true).unwrap();
    assert!(both.get().unwrap());
    // The same description compares equal regardless of order and repeats.
    let reversed = Permission::from(shared)
        .and(a.read_only())
        .and(b.read_only());
    assert!(both.same_as(&reversed));
    assert!(!both.same_as(&Permission::from(a.read_only())));
    assert!(Permission::denied().same_as(&Permission::from(a.read_only()).and(false)));
    assert!(!Permission::denied().same_as(&Permission::allowed()));
}

#[test]
fn every_split_permission_state_governs_pointer_and_keyboard() {
    // Allowed by default.
    let mut ui = runtime(208.0);
    let id = split(&mut ui, pane());
    assert_eq!(resizable(&ui, "split"), InspectValue::Bool(true));
    assert_eq!(drag(&mut ui, &id, 104.0, 20.0), 120.0);
    // Denied: neither pointer nor keys resize.
    for locked in [pane().locked(), pane().resizable(false)] {
        let mut ui = runtime(208.0);
        let id = split(&mut ui, locked);
        assert_eq!(resizable(&ui, "split"), InspectValue::Bool(false));
        assert_eq!(drag(&mut ui, &id, 104.0, 20.0), 100.0);
        assert!(ui.focus(&id).is_ok());
        ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(first(&ui, &id), 100.0);
    }
    // Bound: follows the signal at run time.
    let mut ui = runtime(208.0);
    let allowed = ui.signal(false);
    let id = split(&mut ui, pane().resizable(allowed.read_only()));
    assert_eq!(drag(&mut ui, &id, 104.0, 20.0), 100.0);
    allowed.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(resizable(&ui, "split"), InspectValue::Bool(true));
    assert_eq!(drag(&mut ui, &id, 104.0, 20.0), 120.0);
}

#[test]
fn a_lock_and_a_bound_permission_on_one_description_compose() {
    // Before foundations item 4 the later call replaced the earlier one; now
    // the stricter wins, so a later bound permission never lifts a lock.
    let mut ui = runtime(208.0);
    let allowed = ui.signal(true);
    let id = split(&mut ui, pane().locked().resizable(allowed.read_only()));
    assert_eq!(resizable(&ui, "split"), InspectValue::Bool(false));
    assert_eq!(drag(&mut ui, &id, 104.0, 20.0), 100.0);
    // Two bound permissions: either one can deny.
    let mut ui = runtime(208.0);
    let layout = ui.signal(true);
    let item = ui.signal(true);
    let id = split(
        &mut ui,
        pane()
            .resizable(layout.read_only())
            .resizable(item.read_only()),
    );
    assert_eq!(drag(&mut ui, &id, 104.0, 20.0), 120.0);
    item.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(drag(&mut ui, &id, 124.0, 20.0), 120.0);
    item.set(true).unwrap();
    layout.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(drag(&mut ui, &id, 124.0, 20.0), 120.0);
    layout.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(drag(&mut ui, &id, 124.0, 20.0), 140.0);
}

#[test]
fn revoking_during_a_held_drag_cancels_it_and_granting_again_allows_the_next_press() {
    let mut ui = runtime(208.0);
    let allowed = ui.signal(true);
    let id = split(&mut ui, pane().resizable(allowed.read_only()));
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 134.0, PointerKind::Move);
    assert_eq!(first(&ui, &id), 130.0);
    allowed.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &id), 100.0);
    // Granting again mid-press does not resume the cancelled drag.
    allowed.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(&mut ui, 154.0, PointerKind::Move);
    pointer(&mut ui, 154.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &id), 100.0);
    // The next press resizes.
    assert_eq!(drag(&mut ui, &id, 104.0, 20.0), 120.0);
}

#[test]
fn a_region_layout_restricts_every_edge_but_enables_none() {
    let mut ui = runtime(408.0);
    let editing = ui.signal(true);
    let shell = ui
        .mount_tree(Element::new(
            "shell",
            RegionLayout::new()
                .left(leaf("navigation"))
                .left_width(Length::from(100))
                .left_resizable(true)
                .right(leaf("inspector"))
                .right_width(Length::from(100))
                .center(leaf("editor"))
                .resizable(editing.read_only()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let left = find(&ui, &shell, "split-left").unwrap();
    assert_eq!(resizable(&ui, "split-left"), InspectValue::Bool(true));
    // The right edge has no permission of its own and stays locked.
    assert_eq!(resizable(&ui, "split-right"), InspectValue::Bool(false));
    assert_eq!(drag(&mut ui, &left, 104.0, 20.0), 120.0);
    // The layout-wide signal locks the left edge at run time.
    editing.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(resizable(&ui, "split-left"), InspectValue::Bool(false));
    assert_eq!(drag(&mut ui, &left, 124.0, 20.0), 120.0);
    editing.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(drag(&mut ui, &left, 124.0, 20.0), 140.0);
}

#[test]
fn a_fixed_layout_lock_wins_over_edge_permissions() {
    let mut ui = runtime(408.0);
    let edge = ui.signal(true);
    let shell = ui
        .mount_tree(Element::new(
            "shell",
            RegionLayout::new()
                .left(leaf("navigation"))
                .left_width(Length::from(100))
                .left_resizable(edge.read_only())
                .center(leaf("editor"))
                .resizable(false),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let left = find(&ui, &shell, "split-left").unwrap();
    assert_eq!(resizable(&ui, "split-left"), InspectValue::Bool(false));
    assert_eq!(drag(&mut ui, &left, 104.0, 20.0), 100.0);
}
