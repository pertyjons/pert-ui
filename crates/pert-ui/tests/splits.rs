//! Public constrained split and cancellation contracts.
use pert_ui::*;
mod support;
use std::time::Duration;
use support::find;
fn ui(width: f32) -> Runtime {
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
fn first(ui: &Runtime, root: &WidgetId) -> f32 {
    ui.bounds(&ui.children(root).unwrap()[0])
        .unwrap()
        .unwrap()
        .width()
}
fn pointer(ui: &mut Runtime, x: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, 20.0).unwrap(), kind))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
}
#[test]
fn default_drag_escape_context_loss_and_commit() {
    let mut ui = ui(208.0);
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(SplitAxis::LeftRight, leaf("first"), leaf("second")),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let children = ui.children(&split).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    assert_eq!(first(&ui, &split), 140.0);
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    pointer(&mut ui, 144.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split), 100.0);
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    ui.clear_focus();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    pointer(&mut ui, 144.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split), 140.0);
    assert_eq!(ui.children(&split).unwrap(), children);
}
#[test]
fn lock_mid_drag_restores_preference_and_resize_preserves_fixed_side() {
    let mut ui = ui(208.0);
    let enabled = ui.signal(true);
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(SplitAxis::LeftRight, leaf("first"), leaf("second"))
                .sizing(SplitSizing::second(Length::from(80)))
                .resizable(enabled.read_only()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 120.0);
    pointer(&mut ui, 124.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 154.0, PointerKind::Move);
    assert_eq!(first(&ui, &split), 150.0);
    enabled.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 120.0);
    assert_eq!(ui.focused(), None);
    pointer(&mut ui, 154.0, PointerKind::Release(PointerButton::Primary));
    ui.set_viewport(Viewport::new(308.0, 80.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 220.0);
    ui.focus(&split).unwrap();
    ui.key_input(KeyCode::ArrowLeft, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 220.0);
}
#[test]
fn keyboard_repeat_multiple_keys_cancel_and_release_commit() {
    let mut ui = ui(208.0);
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(SplitAxis::LeftRight, leaf("first"), leaf("second")),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&split).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Repeat)
        .unwrap();
    ui.key_input(KeyCode::ArrowLeft, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!((first(&ui, &split) - 108.0).abs() < 0.001);
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    ui.clear_focus();
    ui.focus(&split).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Repeat)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!((first(&ui, &split) - 108.0).abs() < 0.001);
    ui.clear_focus();
    ui.flush(Duration::ZERO).unwrap();
    assert!((first(&ui, &split) - 108.0).abs() < 0.001);
}
#[test]
fn hard_limits_leave_trailing_space_and_preferences_restore_after_clamping() {
    let mut ui = ui(408.0);
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new(
                    "first",
                    Container::default()
                        .limits(Size::from([30, 0]), Some(Size::from([90, 100])))
                        .unwrap(),
                ),
                Element::new(
                    "second",
                    Container::default()
                        .limits(Size::from([40, 0]), Some(Size::from([80, 100])))
                        .unwrap(),
                ),
            )
            .sizing(SplitSizing::first(Length::from(70)))
            .locked(),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 90.0); // both maxima leave 230px trailing
    ui.set_viewport(Viewport::new(78.0, 80.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 30.0);
    ui.set_viewport(Viewport::new(158.0, 80.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 70.0);
    assert!(SplitSizing::ratio(f32::NAN).is_err());
    assert!(SplitSizing::ratio(1.01).is_err());
}
#[test]
fn child_removal_and_hiding_cancel_pending_resize() {
    let mut ui = ui(208.0);
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(SplitAxis::LeftRight, leaf("first"), leaf("second")),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let second = ui.children(&split).unwrap()[1].clone();
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    ui.set_visible(&second, false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.set_visible(&second, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    ui.unmount(&second).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 208.0);
    pointer(&mut ui, 144.0, PointerKind::Release(PointerButton::Primary));
}

#[test]
fn mixed_input_has_one_origin_and_child_arrows_do_not_resize_parent() {
    let mut ui = ui(208.0);
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new("button", Button::new("Action", |_| Ok(()))),
                leaf("second"),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let button = ui.children(&split).unwrap()[0].clone();
    ui.focus(&button).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    ui.clear_focus();
    ui.focus(&split).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(&mut ui, 112.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 150.0, PointerKind::Move);
    assert!((first(&ui, &split) - 108.0).abs() < 0.001);
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
}
#[test]
fn nested_split_minimums_reach_scroll_viewport() {
    let mut ui = ui(120.0);
    let min = |key| {
        Element::new(
            key,
            Container::default()
                .limits(Size::from([100, 20]), None)
                .unwrap(),
        )
    };
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "outer",
                SplitPane::new(
                    SplitAxis::LeftRight,
                    Element::new(
                        "inner",
                        SplitPane::new(SplitAxis::LeftRight, min("a"), min("b")),
                    ),
                    Element::new(
                        "last",
                        Container::default()
                            .limits(Size::from([40, 20]), None)
                            .unwrap(),
                    ),
                ),
            ))
            .axes(ScrollAxes::Both),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let outer = ui.children(&scroll).unwrap()[0].clone();
    let inner = ui.children(&outer).unwrap()[0].clone();
    assert_eq!(first(&ui, &outer), 208.0);
    assert_eq!(first(&ui, &inner), 100.0);
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 10.0).unwrap(),
        PointerKind::Scroll(Point::new(1000.0, 0.0).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let b = ui.children(&inner).unwrap()[1].clone();
    let [x, _, width, _] = ui.bounds(&b).unwrap().unwrap().values();
    assert!(x < 120.0 && x + width > 0.0);
}

#[test]
fn finite_large_upper_bounds_do_not_reject_small_allocations() {
    let mut ui = ui(208.0);
    let max = Size::new(f32::MAX, f32::MAX).unwrap();
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new(
                    "first",
                    Container::default().limits(Size::ZERO, Some(max)).unwrap(),
                ),
                Element::new(
                    "second",
                    Container::default().limits(Size::ZERO, Some(max)).unwrap(),
                ),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
}

#[test]
fn content_edge_becomes_fixed_only_after_committed_resize() {
    let mut ui = ui(408.0);
    let text = ui.signal("Short".to_owned());
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new("label", Label::new(text.read_only())),
                leaf("rest"),
            )
            .sizing(SplitSizing::CONTENT_FIRST),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let initial = first(&ui, &split);
    text.set("Longer content preference".to_owned()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let natural = first(&ui, &split);
    assert!(natural > initial);
    pointer(
        &mut ui,
        natural + 4.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, natural + 24.0, PointerKind::Move);
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), natural);
    text.set("Short".to_owned()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), initial);
    pointer(
        &mut ui,
        initial + 4.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, initial + 24.0, PointerKind::Move);
    pointer(
        &mut ui,
        initial + 24.0,
        PointerKind::Release(PointerButton::Primary),
    );
    let committed = first(&ui, &split);
    text.set("Longer content preference again".to_owned())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), committed);
}
#[test]
fn region_layout_edge_reuses_split_lock_and_cancellation() {
    let mut ui = ui(408.0);
    let enabled = ui.signal(true);
    let region_layout = ui
        .mount_tree(Element::new(
            "shell",
            RegionLayout::new()
                .left(leaf("navigation"))
                .left_width(Length::from(100))
                .left_resizable(enabled.read_only())
                .center(leaf("editor")),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let split = find(&ui, &region_layout, "split-left").unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    assert_eq!(first(&ui, &split), 140.0);
    enabled.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    pointer(&mut ui, 144.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split), 100.0);
}

#[test]
fn absent_region_layout_regions_never_create_resize_handles() {
    fn count_splits(ui: &Runtime, id: &WidgetId) -> usize {
        usize::from(ui.key(id).unwrap().starts_with("split-"))
            + ui.children(id)
                .unwrap()
                .iter()
                .map(|id| count_splits(ui, id))
                .sum::<usize>()
    }
    for slot in 0..4 {
        let mut ui = ui(408.0);
        let enabled = ui.signal(true);
        let edge = Element::new(
            "content",
            Container::default().preferred_size(Size::from([100, 40])),
        );
        let region_layout = match slot {
            0 => RegionLayout::new()
                .top(edge)
                .top_resizable(enabled.read_only()),
            1 => RegionLayout::new()
                .bottom(edge)
                .bottom_resizable(enabled.read_only()),
            2 => RegionLayout::new()
                .left(edge)
                .left_resizable(enabled.read_only()),
            _ => RegionLayout::new()
                .right(edge)
                .right_resizable(enabled.read_only()),
        };
        let root = ui.mount_tree(Element::new("shell", region_layout)).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(count_splits(&ui, &root), 0);
        ui.focus_next(false).unwrap();
        assert!(ui.focused().is_none());
    }
    let mut ui = ui(408.0);
    let root = ui
        .mount_tree(Element::new(
            "shell",
            RegionLayout::new()
                .top(leaf("top-content"))
                .bottom(leaf("bottom-content")),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(count_splits(&ui, &root), 1);
}

#[test]
fn opposing_region_layout_edges_share_permission_and_fixed_preference() {
    for vertical in [false, true] {
        let mut ui = ui(408.0);
        ui.set_viewport(Viewport::new(408.0, 408.0, 1.0).unwrap())
            .unwrap();
        let enabled = ui.signal(true);
        let region_layout = if vertical {
            RegionLayout::new()
                .top(leaf("a"))
                .bottom(leaf("b"))
                .bottom_height(100.into())
                .bottom_resizable(enabled.read_only())
        } else {
            RegionLayout::new()
                .left(leaf("a"))
                .right(leaf("b"))
                .right_width(100.into())
                .right_resizable(enabled.read_only())
        };
        let root = ui.mount_tree(Element::new("shell", region_layout)).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let viewport = ui.children(&root).unwrap()[0].clone();
        let regions = ui.children(&viewport).unwrap()[0].clone();
        let split = ui.children(&regions).unwrap()[0].clone();
        let second = ui.children(&split).unwrap()[1].clone();
        let extent = |ui: &Runtime| {
            ui.bounds(&second).unwrap().unwrap().values()[if vertical { 3 } else { 2 }]
        };
        assert_eq!(extent(&ui), 100.0);
        ui.focus(&split).unwrap();
        ui.key_input(
            if vertical {
                KeyCode::ArrowUp
            } else {
                KeyCode::ArrowLeft
            },
            KeyAction::Press,
        )
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(extent(&ui), 108.0);
        enabled.set(false).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(extent(&ui), 100.0);
        assert_ne!(ui.focused(), Some(split));
    }
}

#[test]
fn perpendicular_region_layout_edges_preserve_lone_side_extent_and_alignment() {
    for right in [false, true] {
        for bottom in [false, true] {
            let mut ui = ui(408.0);
            let region_layout = if right {
                RegionLayout::new()
                    .right(leaf("side"))
                    .right_width(100.into())
            } else {
                RegionLayout::new()
                    .left(leaf("side"))
                    .left_width(100.into())
            };
            let region_layout = if bottom {
                region_layout.bottom(leaf("bar"))
            } else {
                region_layout.top(leaf("bar"))
            };
            let root = ui.mount_tree(Element::new("shell", region_layout)).unwrap();
            ui.flush(Duration::ZERO).unwrap();
            let side = find(&ui, &root, "side").unwrap();
            let [x, _, width, _] = ui.bounds(&side).unwrap().unwrap().values();
            assert_eq!(width, 100.0);
            assert_eq!(x, if right { 308.0 } else { 0.0 });
        }
    }
}

#[test]
fn redescribed_split_applies_properties_and_keeps_user_position_until_its_rule_changes() {
    fn split(axis: SplitAxis, sizing: SplitSizing, locked: bool) -> Vec<Element> {
        let split = SplitPane::new(axis, leaf("first"), leaf("second")).sizing(sizing);
        vec![Element::new(
            "split",
            if locked { split.locked() } else { split },
        )]
    }
    let equal = SplitSizing::EQUAL;
    let mut ui = ui(208.0);
    let root = ui
        .mount_tree(Element::new(
            "root",
            View::new(split(SplitAxis::LeftRight, equal, false).remove(0)),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let split_id = ui.children(&root).unwrap()[0].clone();
    let children = ui.children(&split_id).unwrap();
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    pointer(&mut ui, 144.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split_id), 140.0);

    // The same rule keeps the user's committed position.
    ui.reconcile_children(&root, split(SplitAxis::LeftRight, equal, false))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split_id), 140.0);

    // A different rule replaces it.
    let sixty = SplitSizing::first(Length::from(60));
    ui.reconcile_children(&root, split(SplitAxis::LeftRight, sixty, false))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split_id), 60.0);

    // A new rule mid-drag cancels the drag; the rest of the press is ignored.
    pointer(&mut ui, 64.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 94.0, PointerKind::Move);
    assert_eq!(first(&ui, &split_id), 90.0);
    let quarter = SplitSizing::ratio(0.25).unwrap();
    ui.reconcile_children(&root, split(SplitAxis::LeftRight, quarter, false))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split_id), 50.0);
    pointer(&mut ui, 120.0, PointerKind::Move);
    pointer(&mut ui, 120.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split_id), 50.0);

    // Locking mid-drag restores the original preference, and the lock holds.
    pointer(&mut ui, 54.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 94.0, PointerKind::Move);
    assert_eq!(first(&ui, &split_id), 90.0);
    ui.reconcile_children(&root, split(SplitAxis::LeftRight, quarter, true))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split_id), 50.0);
    pointer(&mut ui, 94.0, PointerKind::Release(PointerButton::Primary));
    pointer(&mut ui, 54.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 94.0, PointerKind::Move);
    pointer(&mut ui, 94.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split_id), 50.0);

    // A new axis rearranges the same children.
    ui.reconcile_children(&root, split(SplitAxis::TopBottom, quarter, true))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split_id), 208.0);
    assert_eq!(ui.children(&split_id).unwrap(), children);

    // Unlocking again permits resizing.
    ui.reconcile_children(&root, split(SplitAxis::LeftRight, quarter, false))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split_id), 50.0);
    pointer(&mut ui, 54.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 74.0, PointerKind::Move);
    pointer(&mut ui, 74.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split_id), 70.0);
    assert_eq!(ui.children(&root).unwrap()[0], split_id);
}

#[test]
fn redescribed_region_layout_applies_extents_and_permissions() {
    fn shell(width: u16, resizable: Option<ReadSignal<bool>>) -> Vec<Element> {
        let layout = RegionLayout::new()
            .left(leaf("navigation"))
            .left_width(Length::from(width))
            .center(leaf("editor"));
        vec![Element::new(
            "shell",
            match resizable {
                Some(permission) => layout.left_resizable(permission),
                None => layout,
            },
        )]
    }
    let mut ui = ui(408.0);
    let enabled = ui.signal(true);
    let root = ui
        .mount_tree(Element::new("root", View::new(shell(100, None).remove(0))))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let split = find(&ui, &root, "split-left").unwrap();
    assert_eq!(first(&ui, &split), 100.0);
    // Locked: a drag does nothing.
    pointer(&mut ui, 104.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 144.0, PointerKind::Move);
    pointer(&mut ui, 144.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split), 100.0);

    // A new extent and a bound permission apply to the retained divider.
    ui.reconcile_children(&root, shell(120, Some(enabled.read_only())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(find(&ui, &root, "split-left"), Some(split.clone()));
    assert_eq!(first(&ui, &split), 120.0);
    pointer(&mut ui, 124.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 164.0, PointerKind::Move);
    pointer(&mut ui, 164.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(first(&ui, &split), 160.0);

    // The same extent keeps the user's position; a new one replaces it.
    ui.reconcile_children(&root, shell(120, Some(enabled.read_only())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 160.0);
    ui.reconcile_children(&root, shell(90, Some(enabled.read_only())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(first(&ui, &split), 90.0);
}

/// Focusable content that leaves every key to its ancestors.
struct Focusable;
impl Component for Focusable {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([40, 40]))
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
}

#[test]
fn escape_reaches_window_commands_through_idle_splits_and_region_layouts() {
    fn escape(ui: &mut Runtime) -> bool {
        let press = ui
            .command_key_input(CommandKeyInput::new(
                KeyCode::Escape,
                KeyAction::Press,
                CommandKey::Named(KeyCode::Escape),
            ))
            .unwrap();
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::Escape,
            KeyAction::Release,
            CommandKey::Named(KeyCode::Escape),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        press.handled
    }
    let mut ui = ui(408.0);
    let escapes = ui.signal(0);
    let written = escapes.clone();
    let _close = ui
        .register_command(
            "Close panel",
            CommandScope::Window(ui.primary_window()),
            vec![KeyChord::new(
                CommandKey::Named(KeyCode::Escape),
                Modifiers::default(),
            )],
            move |_, _| written.set(written.get()? + 1).map(|_| ()),
        )
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "split",
                    SplitPane::new(
                        SplitAxis::LeftRight,
                        Element::new("split-content", Focusable),
                        leaf("second"),
                    ),
                ),
                Element::new(
                    "shell",
                    RegionLayout::new()
                        .left(leaf("navigation"))
                        .center(Element::new("shell-content", Focusable)),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let split = find(&ui, &root, "split").unwrap();
    for target in [
        split.clone(),
        find(&ui, &root, "split-content").unwrap(),
        find(&ui, &root, "split-left").unwrap(),
        find(&ui, &root, "shell-content").unwrap(),
    ] {
        ui.focus(&target).unwrap();
        let before = escapes.get().unwrap();
        assert!(escape(&mut ui));
        assert_eq!(escapes.get().unwrap(), before + 1);
        assert_eq!(ui.focused(), Some(target));
    }
    // A keyboard resize in progress still owns Escape, which cancels it.
    ui.focus(&split).unwrap();
    let before = first(&ui, &split);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_ne!(first(&ui, &split), before);
    let count = escapes.get().unwrap();
    assert!(escape(&mut ui));
    assert_eq!(escapes.get().unwrap(), count);
    assert_eq!(first(&ui, &split), before);
}

#[test]
fn inspection_reports_the_committed_rule_and_extent() {
    fn sizing(sizing: SplitSizing) -> (Option<InspectValue>, Option<InspectValue>) {
        let mut ui = ui(400.0);
        ui.mount_tree(Element::new(
            "split",
            SplitPane::new(SplitAxis::LeftRight, leaf("a"), leaf("b")).sizing(sizing),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let details = ui
            .inspect()
            .components
            .into_iter()
            .find(|component| component.key == "split")
            .unwrap()
            .details;
        (
            details.properties.get("rule").cloned(),
            details.properties.get("extent").cloned(),
        )
    }
    assert_eq!(
        sizing(SplitSizing::second(Length::from(100))),
        (
            Some(InspectValue::Text("second".into())),
            Some(InspectValue::Number(100.0))
        )
    );
    assert_eq!(
        sizing(SplitSizing::ratio(0.25).unwrap()),
        (
            Some(InspectValue::Text("ratio".into())),
            Some(InspectValue::Number(0.25))
        )
    );
    assert_eq!(
        sizing(SplitSizing::CONTENT_FIRST),
        (Some(InspectValue::Text("content-first".into())), None)
    );
}
