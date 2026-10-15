//! Live restrictions preserve structural capabilities and cancel collection gestures.
use pert_ui::*;
use std::{cell::Cell, rc::Rc};

struct Rows;
impl TableItems for Rows {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("row{index}"))
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        TableItems::key(self, index).map(|_| Element::new("value", Label::new("Cell")))
    }
}
impl ListItems for Rows {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("row{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        ListItems::key(self, index).map(|key| Element::new(key, Label::new("Row")))
    }
}
impl TreeItems for Rows {
    fn roots(&self) -> Vec<String> {
        (0..3).map(|i| format!("row{i}")).collect()
    }
    fn children(&self, _: &str) -> Vec<String> {
        vec![]
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}
fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 240.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
    ui.flush(ui.now()).unwrap();
}
fn property(ui: &Runtime, key: &str, property: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == key)
        .unwrap()
        .details
        .properties[property]
        .clone()
}
fn table(ui: &Runtime, first: TableColumn) -> TableController {
    TableController::new(
        ui,
        [first, TableColumn::new("b", "B").movable(true)],
        TableSource::new(Rows),
        ListSelection::new(ui, SelectionMode::Single),
    )
    .unwrap()
}
#[test]
fn revoking_resize_restores_fixed_width_or_flex_weight_and_blocks_stale_release() {
    for fixed in [false, true] {
        let mut ui = runtime();
        let permission = ui.signal(true);
        let mut first = TableColumn::new("a", "A").resizable(true);
        if fixed {
            first = first.width(Length::new(100.0).unwrap());
        } else {
            first = first.flex(3.0).unwrap();
        }
        let state = table(&ui, first);
        let id = ui
            .mount_tree(Element::new(
                "table",
                Table::new(&state, 24.0)
                    .unwrap()
                    .resize_permission(permission.read_only()),
            ))
            .unwrap();
        ui.flush(ui.now()).unwrap();
        let saved = ui.capture_workspace().unwrap();
        let widths = property(&ui, "table", "column_widths");
        let row = ui.find_descendant(&id, "row0").unwrap().unwrap();
        let cell = ui.find_descendant(&row, "a").unwrap().unwrap();
        let cell_bounds = ui.bounds(&cell).unwrap().unwrap();
        let edge = if fixed { 100.0 } else { 300.0 };
        pointer(
            &mut ui,
            edge,
            10.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(&mut ui, edge + 30.0, 10.0, PointerKind::Move);
        assert!(state.columns()[0].fixed_width().is_some());
        assert_ne!(ui.bounds(&cell).unwrap().unwrap(), cell_bounds);
        permission.set(false).unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(ui.capture_workspace().unwrap(), saved);
        assert_eq!(property(&ui, "table", "column_widths"), widths);
        assert_eq!(ui.bounds(&cell).unwrap().unwrap(), cell_bounds);
        let restored = &state.columns()[0];
        assert_eq!(restored.fixed_width(), fixed.then_some(100.0));
        assert_eq!(restored.flex_weight(), (!fixed).then_some(3.0));
        pointer(
            &mut ui,
            edge + 80.0,
            10.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(state.columns()[0], *restored);
        if !fixed {
            ui.set_viewport(Viewport::new(800.0, 240.0, 1.0).unwrap())
                .unwrap();
            ui.flush(ui.now()).unwrap();
            assert_eq!(state.columns()[0].flex_weight(), Some(3.0));
        } else {
            // A completed resize creates a user preference; revoking a later
            // resize must retain that membership as well as its exact width.
            permission.set(true).unwrap();
            pointer(
                &mut ui,
                100.0,
                10.0,
                PointerKind::Press(PointerButton::Primary),
            );
            pointer(&mut ui, 120.0, 10.0, PointerKind::Move);
            pointer(
                &mut ui,
                120.0,
                10.0,
                PointerKind::Release(PointerButton::Primary),
            );
            let preferred = ui.capture_workspace().unwrap();
            pointer(
                &mut ui,
                120.0,
                10.0,
                PointerKind::Press(PointerButton::Primary),
            );
            pointer(&mut ui, 150.0, 10.0, PointerKind::Move);
            permission.set(false).unwrap();
            ui.flush(ui.now()).unwrap();
            assert_eq!(state.columns()[0].fixed_width(), Some(120.0));
            assert_eq!(ui.capture_workspace().unwrap(), preferred);
            pointer(
                &mut ui,
                150.0,
                10.0,
                PointerKind::Release(PointerButton::Primary),
            );
        }
    }
}
#[test]
fn fixed_resize_denial_cannot_be_reenabled_by_a_bound_permission() {
    for binding_first in [false, true] {
        let mut ui = runtime();
        let allowed = ui.signal(true);
        let state = table(
            &ui,
            TableColumn::new("a", "A")
                .width(Length::new(100.0).unwrap())
                .resizable(true),
        );
        let widget = Table::new(&state, 24.0).unwrap();
        let widget = if binding_first {
            widget
                .resize_permission(allowed.read_only())
                .resize_permission(false)
        } else {
            widget
                .resize_permission(false)
                .resize_permission(allowed.read_only())
        };
        ui.mount_tree(Element::new("table", widget)).unwrap();
        ui.flush(ui.now()).unwrap();
        pointer(
            &mut ui,
            100.0,
            10.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(&mut ui, 150.0, 10.0, PointerKind::Move);
        pointer(
            &mut ui,
            150.0,
            10.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(state.columns()[0].fixed_width(), Some(100.0));
    }
}
#[test]
fn revoking_table_reordering_cancels_the_active_drag_without_changing_order() {
    let mut ui = runtime();
    let allowed = ui.signal(true);
    let state = table(
        &ui,
        TableColumn::new("a", "A")
            .width(Length::new(100.0).unwrap())
            .movable(true),
    );
    ui.mount_tree(Element::new(
        "table",
        Table::new(&state, 24.0)
            .unwrap()
            .reorder_permission(allowed.read_only()),
    ))
    .unwrap();
    ui.flush(ui.now()).unwrap();
    pointer(
        &mut ui,
        30.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 160.0, 10.0, PointerKind::Move);
    assert!(ui.drag_active());
    allowed.set(false).unwrap();
    // A move's own preflush must cancel ownership before release.
    pointer(&mut ui, 350.0, 10.0, PointerKind::Move);
    assert!(!ui.drag_active());
    pointer(
        &mut ui,
        350.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(!ui.drag_active());
    assert_eq!(
        state
            .columns()
            .iter()
            .map(TableColumn::key)
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}
#[test]
fn list_and_tree_reordering_revocation_cancels_owned_drag_and_can_be_reenabled() {
    for tree in [false, true] {
        let mut ui = runtime();
        let allowed = ui.signal(true);
        let calls = Rc::new(Cell::new(0));
        let sink = calls.clone();
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let widget = if tree {
            let controller = TreeController::new(&ui, TreeSource::new(Rows), selection).unwrap();
            Element::new(
                "collection",
                Tree::new(&controller, 40.0)
                    .unwrap()
                    .reorderable(move |_, _| {
                        sink.set(sink.get() + 1);
                        Ok(true)
                    })
                    .reorder_permission(allowed.read_only()),
            )
        } else {
            let rows = ui.signal(ListSource::new(Rows));
            Element::new(
                "collection",
                VirtualList::new(rows.read_only(), 40.0)
                    .unwrap()
                    .selection(selection)
                    .reorderable(move |_, _| {
                        sink.set(sink.get() + 1);
                        Ok(true)
                    })
                    .reorder_permission(allowed.read_only()),
            )
        };
        ui.mount_tree(widget).unwrap();
        ui.flush(ui.now()).unwrap();
        pointer(
            &mut ui,
            80.0,
            20.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(&mut ui, 80.0, 70.0, PointerKind::Move);
        assert!(ui.drag_active());
        allowed.set(false).unwrap();
        ui.flush(ui.now()).unwrap();
        assert!(!ui.drag_active());
        pointer(
            &mut ui,
            80.0,
            90.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(calls.get(), 0);
        allowed.set(true).unwrap();
        ui.flush(ui.now()).unwrap();
        pointer(
            &mut ui,
            80.0,
            20.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(&mut ui, 80.0, 90.0, PointerKind::Move);
        assert!(ui.drag_active());
        pointer(
            &mut ui,
            80.0,
            90.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn denied_table_reorder_does_not_lift_column_capabilities_and_live_permission_recovers() {
    for movable in [false, true] {
        let mut ui = runtime();
        let allowed = ui.signal(false);
        let state = table(&ui, TableColumn::new("a", "A").movable(movable));
        let id = ui
            .mount_tree(Element::new(
                "table",
                Table::new(&state, 24.0)
                    .unwrap()
                    .reorder_permission(allowed.read_only()),
            ))
            .unwrap();
        ui.flush(ui.now()).unwrap();
        let body = ui.find_descendant(&id, "body").unwrap().unwrap();
        ui.focus(&body).unwrap();
        state.set_active_column(Some("a".into())).unwrap();
        let alt_right = |ui: &mut Runtime| {
            for (key, action) in [
                (KeyCode::AltLeft, KeyAction::Press),
                (KeyCode::ArrowRight, KeyAction::Press),
                (KeyCode::ArrowRight, KeyAction::Release),
                (KeyCode::AltLeft, KeyAction::Release),
            ] {
                ui.command_key_input(CommandKeyInput::new(key, action, CommandKey::Named(key)))
                    .unwrap();
            }
            ui.flush(ui.now()).unwrap();
        };
        alt_right(&mut ui);
        assert_eq!(state.columns()[0].key(), "a");
        state.set_active_column(Some("a".into())).unwrap();
        allowed.set(true).unwrap();
        alt_right(&mut ui);
        assert_eq!(state.columns()[0].key(), if movable { "b" } else { "a" });
    }
}

#[test]
fn fixed_list_and_tree_denial_cannot_be_lifted_and_selection_still_works() {
    for tree in [false, true] {
        let mut ui = runtime();
        let allowed = ui.signal(true);
        let calls = Rc::new(Cell::new(0));
        let sink = calls.clone();
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let item = if tree {
            let controller =
                TreeController::new(&ui, TreeSource::new(Rows), selection.clone()).unwrap();
            Element::new(
                "collection",
                Tree::new(&controller, 40.0)
                    .unwrap()
                    .reorderable(move |_, _| {
                        sink.set(sink.get() + 1);
                        Ok(true)
                    })
                    .reorder_permission(false)
                    .reorder_permission(allowed.read_only()),
            )
        } else {
            let source = ui.signal(ListSource::new(Rows));
            Element::new(
                "collection",
                VirtualList::new(source.read_only(), 40.0)
                    .unwrap()
                    .selection(selection.clone())
                    .reorderable(move |_, _| {
                        sink.set(sink.get() + 1);
                        Ok(true)
                    })
                    .reorder_permission(false)
                    .reorder_permission(allowed.read_only()),
            )
        };
        ui.mount_tree(item).unwrap();
        ui.flush(ui.now()).unwrap();
        pointer(
            &mut ui,
            80.0,
            20.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(&mut ui, 80.0, 90.0, PointerKind::Move);
        assert!(!ui.drag_active());
        pointer(
            &mut ui,
            80.0,
            90.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(selection.active().get().unwrap(), Some("row0".into()));
    }
}

#[test]
fn revoking_movement_preserves_armed_sorting_without_admitting_a_column_drag() {
    for movable in [false, true] {
        let mut ui = runtime();
        let allowed = ui.signal(true);
        let calls = Rc::new(Cell::new(0));
        let sink = calls.clone();
        let state = table(
            &ui,
            TableColumn::new("a", "A")
                .width(Length::new(100.0).unwrap())
                .sortable(true)
                .movable(movable),
        );
        ui.mount_tree(Element::new(
            "table",
            Table::new(&state, 24.0)
                .unwrap()
                .reorder_permission(allowed.read_only())
                .on_sort(move |_, sort| {
                    assert_eq!(sort.column, "a");
                    sink.set(sink.get() + 1);
                    Ok(())
                }),
        ))
        .unwrap();
        ui.flush(ui.now()).unwrap();
        pointer(
            &mut ui,
            30.0,
            10.0,
            PointerKind::Press(PointerButton::Primary),
        );
        allowed.set(false).unwrap();
        ui.flush(ui.now()).unwrap();
        // Passing the drag threshold cannot turn the preserved sorting press
        // into a move once permission is denied.
        pointer(&mut ui, 180.0, 10.0, PointerKind::Move);
        assert!(!ui.drag_active());
        pointer(
            &mut ui,
            30.0,
            10.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            state
                .columns()
                .iter()
                .map(TableColumn::key)
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }
}

#[test]
fn allowing_permissions_does_not_create_missing_structural_capabilities() {
    let mut ui = runtime();
    let state = table(
        &ui,
        TableColumn::new("a", "A").width(Length::new(100.0).unwrap()),
    );
    ui.mount_tree(Element::new(
        "table",
        Table::new(&state, 24.0).unwrap().resize_permission(true),
    ))
    .unwrap();
    ui.flush(ui.now()).unwrap();
    pointer(
        &mut ui,
        100.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 150.0, 10.0, PointerKind::Move);
    pointer(
        &mut ui,
        150.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(state.columns()[0].fixed_width(), Some(100.0));
    for tree in [false, true] {
        let mut ui = runtime();
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let element = if tree {
            let state = TreeController::new(&ui, TreeSource::new(Rows), selection.clone()).unwrap();
            Element::new(
                "collection",
                Tree::new(&state, 40.0).unwrap().reorder_permission(true),
            )
        } else {
            let rows = ui.signal(ListSource::new(Rows));
            Element::new(
                "collection",
                VirtualList::new(rows.read_only(), 40.0)
                    .unwrap()
                    .selection(selection.clone())
                    .reorder_permission(true),
            )
        };
        ui.mount_tree(element).unwrap();
        ui.flush(ui.now()).unwrap();
        pointer(
            &mut ui,
            80.0,
            20.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(&mut ui, 80.0, 90.0, PointerKind::Move);
        assert!(!ui.drag_active());
        pointer(
            &mut ui,
            80.0,
            90.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(selection.active().get().unwrap(), Some("row0".into()));
        assert_eq!(
            property(&ui, "collection", "reorderable"),
            InspectValue::Bool(false)
        );
    }
}

#[test]
fn revoking_a_foreign_drop_target_preserves_the_sources_owned_drag() {
    let mut ui = runtime();
    let allowed = ui.signal(true);
    let rows = ui.signal(ListSource::new(Rows));
    let calls = Rc::new(Cell::new(0));
    let list = |key: &str, permission: Permission| {
        let sink = calls.clone();
        Element::new(
            key,
            Container::new(Element::new(
                format!("{key}-list"),
                VirtualList::new(rows.read_only(), 40.0)
                    .unwrap()
                    .selection(ListSelection::new(&ui, SelectionMode::Single))
                    .reorderable(move |_, _| {
                        sink.set(sink.get() + 1);
                        Ok(true)
                    })
                    .reorder_permission(permission),
            ))
            .preferred_size(Size::new(160.0, 200.0).unwrap()),
        )
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Row::new([
                list("source", Permission::allowed()),
                list("target", Permission::from(allowed.read_only())),
            ]),
        ))
        .unwrap();
    ui.flush(ui.now()).unwrap();
    let source = ui.find_descendant(&root, "source-list").unwrap().unwrap();
    let target = ui.find_descendant(&root, "target-list").unwrap().unwrap();
    let from = ui.bounds(&source).unwrap().unwrap();
    let to = ui.bounds(&target).unwrap().unwrap();
    pointer(
        &mut ui,
        from.x() + 40.0,
        from.y() + 20.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, from.x() + 40.0, from.y() + 70.0, PointerKind::Move);
    assert!(ui.drag_active());
    pointer(&mut ui, to.x() + 40.0, to.y() + 70.0, PointerKind::Move);
    let has_indicator = |ui: &Runtime| {
        let snapshot = ui.inspect();
        let target = snapshot
            .components
            .iter()
            .find(|c| c.key == "target-list")
            .unwrap();
        target.details.properties.contains_key("drop_boundary")
            || target.details.properties.contains_key("drop_into")
    };
    assert!(has_indicator(&ui), "the target must first admit the drag");
    allowed.set(false).unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(!has_indicator(&ui));
    assert!(
        ui.drag_active(),
        "a target cannot revoke the source's ownership"
    );
    pointer(
        &mut ui,
        to.x() + 40.0,
        to.y() + 70.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(!ui.drag_active());
    assert_eq!(calls.get(), 0);
}

#[test]
fn revoking_reorder_permission_does_not_cancel_an_active_scrollbar_drag() {
    for tree in [false, true] {
        let mut ui = runtime();
        ui.set_viewport(Viewport::new(400.0, 80.0, 1.0).unwrap())
            .unwrap();
        let allowed = ui.signal(true);
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let element = if tree {
            let state = TreeController::new(&ui, TreeSource::new(Rows), selection).unwrap();
            Element::new(
                "collection",
                Tree::new(&state, 40.0)
                    .unwrap()
                    .reorderable(|_, _| Ok(true))
                    .reorder_permission(allowed.read_only()),
            )
        } else {
            let rows = ui.signal(ListSource::new(Rows));
            Element::new(
                "collection",
                VirtualList::new(rows.read_only(), 40.0)
                    .unwrap()
                    .selection(selection)
                    .reorderable(|_, _| Ok(true))
                    .reorder_permission(allowed.read_only()),
            )
        };
        ui.mount_tree(element).unwrap();
        ui.flush(ui.now()).unwrap();
        let thumb_color = Palette::dark().color(ColorRole::ScrollbarThumb);
        let thumb = ui
            .paint()
            .unwrap()
            .commands()
            .iter()
            .find_map(|cmd| match cmd {
                DrawCommand::FillRect { rect, color } if *color == thumb_color => Some(*rect),
                _ => None,
            })
            .expect("overflowing collection draws a scrollbar");
        let x = thumb.x() + thumb.width() / 2.0;
        let y = thumb.y() + thumb.height() / 2.0;
        let before = property(&ui, "collection", "scroll_y");
        pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
        allowed.set(false).unwrap();
        ui.flush(ui.now()).unwrap();
        pointer(&mut ui, 410.0, 75.0, PointerKind::Move);
        assert_ne!(
            property(&ui, "collection", "scroll_y"),
            before,
            "capture must continue outside the collection"
        );
        pointer(
            &mut ui,
            410.0,
            75.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert!(!ui.drag_active());
    }
}

#[test]
fn compatible_redescription_retains_collection_permission_bindings() {
    for kind in 0..3 {
        let mut ui = runtime();
        let original = ui.signal(false);
        let replacement = ui.signal(true);
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let table = table(&ui, TableColumn::new("a", "A").resizable(true));
        let rows = ui.signal(ListSource::new(Rows));
        let tree = TreeController::new(&ui, TreeSource::new(Rows), selection.clone()).unwrap();
        let describe = |binding: ReadSignal<bool>| match kind {
            0 => Element::new(
                "collection",
                Table::new(&table, 24.0).unwrap().resize_permission(binding),
            ),
            1 => Element::new(
                "collection",
                VirtualList::new(rows.read_only(), 40.0)
                    .unwrap()
                    .selection(selection.clone())
                    .reorderable(|_, _| Ok(true))
                    .reorder_permission(binding),
            ),
            _ => Element::new(
                "collection",
                Tree::new(&tree, 40.0)
                    .unwrap()
                    .reorderable(|_, _| Ok(true))
                    .reorder_permission(binding),
            ),
        };
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([describe(original.read_only())]),
            ))
            .unwrap();
        ui.flush(ui.now()).unwrap();
        let before = ui.find_descendant(&root, "collection").unwrap().unwrap();
        ui.reconcile_children(&root, vec![describe(replacement.read_only())])
            .unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(
            ui.find_descendant(&root, "collection").unwrap().unwrap(),
            before
        );
        let name = if kind == 0 {
            "resize_allowed"
        } else {
            "reorderable"
        };
        assert_eq!(property(&ui, "collection", name), InspectValue::Bool(false));
        replacement.set(false).unwrap();
        original.set(true).unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(property(&ui, "collection", name), InspectValue::Bool(true));
    }
}

#[test]
fn list_and_tree_keyboard_reordering_obeys_live_permission_in_native_key_routing() {
    for tree in [false, true] {
        let mut ui = runtime();
        let allowed = ui.signal(false);
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let calls = Rc::new(Cell::new(0));
        let sink = calls.clone();
        let element = if tree {
            let state = TreeController::new(&ui, TreeSource::new(Rows), selection).unwrap();
            Element::new(
                "collection",
                Tree::new(&state, 40.0)
                    .unwrap()
                    .reorderable(move |_, _| {
                        sink.set(sink.get() + 1);
                        Ok(true)
                    })
                    .reorder_permission(allowed.read_only()),
            )
        } else {
            let rows = ui.signal(ListSource::new(Rows));
            Element::new(
                "collection",
                VirtualList::new(rows.read_only(), 40.0)
                    .unwrap()
                    .selection(selection)
                    .reorderable(move |_, _| {
                        sink.set(sink.get() + 1);
                        Ok(true)
                    })
                    .reorder_permission(allowed.read_only()),
            )
        };
        ui.mount_tree(element).unwrap();
        ui.flush(ui.now()).unwrap();
        pointer(
            &mut ui,
            80.0,
            20.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(
            &mut ui,
            80.0,
            20.0,
            PointerKind::Release(PointerButton::Primary),
        );
        for enabled in [false, true] {
            allowed.set(enabled).unwrap();
            for (key, action) in [
                (KeyCode::AltLeft, KeyAction::Press),
                (KeyCode::ArrowDown, KeyAction::Press),
                (KeyCode::ArrowDown, KeyAction::Release),
                (KeyCode::AltLeft, KeyAction::Release),
            ] {
                ui.command_key_input(CommandKeyInput::new(key, action, CommandKey::Named(key)))
                    .unwrap();
            }
            ui.flush(ui.now()).unwrap();
            assert_eq!(calls.get(), usize::from(enabled));
        }
    }
}
