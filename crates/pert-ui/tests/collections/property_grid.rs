//! Property grids: aligned columns, keyed rows, the resizable label column,
//! the shared width binding and workspace persistence.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 600.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn flush(ui: &mut Runtime) {
    ui.flush(Duration::ZERO).unwrap();
}

fn find(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    ui.find_descendant(root, key)
        .unwrap()
        .unwrap_or_else(|| panic!("{key} is not mounted"))
}

fn details(ui: &Runtime, key: &str) -> ComponentDetails {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .unwrap_or_else(|| panic!("no component keyed {key}"))
        .details
}

fn label_width(ui: &Runtime, key: &str) -> f64 {
    match details(ui, key).properties["label_width"] {
        InspectValue::Number(width) => width,
        ref other => panic!("label_width is {other:?}"),
    }
}

fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
    flush(ui);
}

fn key(ui: &mut Runtime, code: KeyCode, action: KeyAction) {
    ui.key_input(code, action).unwrap();
    flush(ui);
}

fn button(key: &str) -> Element {
    Element::new(key, Button::new(key, |_| Ok(())))
}

fn rows(order: &[&str]) -> Vec<Property> {
    order
        .iter()
        .map(|name| {
            Property::new(
                *name,
                format!("Label {name}"),
                button(&format!("{name}-editor")),
            )
        })
        .collect()
}

fn grid(order: &[&str]) -> Element {
    Element::new("grid", PropertyGrid::new(rows(order)))
}

#[test]
fn labels_and_editors_sit_in_two_aligned_columns() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(grid(&["a", "b", "c"]))?;
    flush(&mut ui);
    // Short labels: the column takes its ControlWidth floor.
    assert_eq!(label_width(&ui, "grid"), 64.0);
    let mut previous_bottom = None;
    for name in ["a", "b", "c"] {
        let editor = ui
            .bounds(&find(&ui, &root, &format!("{name}-editor")))?
            .unwrap();
        // The editor column starts after the label column and the resize zone.
        assert_eq!(editor.x(), 72.0);
        assert_eq!(editor.width(), 480.0 - 72.0);
        // The row's field draws no label above the editor.
        let field = ui.bounds(&find(&ui, &root, name))?.unwrap();
        assert_eq!(field.y(), editor.y());
        if let Some(bottom) = previous_bottom {
            assert_eq!(field.y(), bottom + 8.0, "rows are separated by ItemGap");
        }
        previous_bottom = Some(field.y() + field.height());
    }
    // The label names the editor's field.
    let field = details(&ui, "b").semantics.unwrap();
    assert_eq!(field.role, SemanticRole::Field);
    assert_eq!(field.name.as_deref(), Some("Label b"));
    assert_eq!(
        details(&ui, "b").properties["label"],
        InspectValue::Text("Label b".into())
    );
    let grid = details(&ui, "grid").semantics.unwrap();
    assert_eq!(grid.role, SemanticRole::Group);
    assert!(grid.actions.contains(&SemanticAction::Resize));
    Ok(())
}

#[test]
fn a_long_label_widens_the_column_up_to_half_and_wraps_beyond() -> TestResult {
    let mut ui = ui();
    let long = "A considerably longer property label than the others in this grid";
    let root = ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new([
            Property::new("short", "Short", button("short-editor")),
            Property::new("long", long, button("long-editor")),
        ]),
    ))?;
    flush(&mut ui);
    assert_eq!(label_width(&ui, "grid"), 240.0, "at most half the grid");
    // The wrapped label makes its row taller than the editor, never cut off.
    let editor = ui.bounds(&find(&ui, &root, "long-editor"))?.unwrap();
    let grid = ui.bounds(&root)?.unwrap();
    assert!(grid.height() > editor.y() + editor.height());
    Ok(())
}

#[test]
fn editors_keep_identity_and_focus_by_key_under_reorder_relabel_and_removal() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(grid(&["a", "b", "c"]))?;
    flush(&mut ui);
    let b = find(&ui, &root, "b-editor");
    ui.focus(&b)?;
    flush(&mut ui);
    let described = |order: &[(&str, &str)]| {
        order
            .iter()
            .map(|(name, label)| {
                Property::new(*name, *label, button(&format!("{name}-editor"))).into_element()
            })
            .collect::<Vec<_>>()
    };
    ui.reconcile_children(
        &root,
        described(&[("c", "C"), ("b", "Renamed"), ("a", "A")]),
    )?;
    flush(&mut ui);
    assert_eq!(find(&ui, &root, "b-editor"), b);
    assert_eq!(ui.focused(), Some(b.clone()));
    assert_eq!(
        details(&ui, "b").semantics.unwrap().name.as_deref(),
        Some("Renamed")
    );
    let c = ui.bounds(&find(&ui, &root, "c-editor"))?.unwrap();
    let b_bounds = ui.bounds(&b)?.unwrap();
    assert!(c.y() < b_bounds.y(), "rows follow the new order");

    ui.reconcile_children(&root, described(&[("b", "Renamed")]))?;
    flush(&mut ui);
    assert_eq!(find(&ui, &root, "b-editor"), b);
    assert_eq!(ui.focused(), Some(b));
    Ok(())
}

#[test]
fn invalid_rows_are_rejected_before_the_grid_mounts() {
    let cases: [(Vec<Property>, LayoutError); 3] = [
        (
            vec![Property::new("a", "  ", button("a-editor"))],
            LayoutError::BlankLabel,
        ),
        (
            vec![Property::new(" ", "Label", button("a-editor"))],
            LayoutError::BlankKey,
        ),
        (
            vec![
                Property::new("a", "First", button("a-editor")),
                Property::new("a", "Second", button("b-editor")),
            ],
            LayoutError::DuplicateKey("a".into()),
        ),
    ];
    for (rows, expected) in cases {
        let mut ui = ui();
        assert_eq!(
            ui.mount_tree(Element::new("grid", PropertyGrid::new(rows)))
                .err(),
            Some(RuntimeError::Layout(expected))
        );
        assert!(ui.inspect().components.is_empty());
    }
    // A raw child without a label is only seen by the update that reads it.
    let mut ui = ui();
    ui.mount_tree(Element::new("grid", PropertyGrid::new([])).children([button("raw")]))
        .unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Layout(LayoutError::BlankLabel))
    );
}

#[test]
fn a_rejected_description_leaves_the_mounted_grid_unchanged() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new("root", Column::new([grid(&["a", "b"])])))?;
    flush(&mut ui);
    let grid_id = find(&ui, &root, "grid");
    let editors = ui.children(&grid_id)?;
    ui.focus(&find(&ui, &root, "b-editor"))?;
    let invalid = Element::new(
        "grid",
        PropertyGrid::new([
            Property::new("a", "Label a", button("a-editor")),
            Property::new("b", " ", button("b-editor")),
        ]),
    );
    assert_eq!(
        ui.reconcile_children(&root, vec![invalid]),
        Err(RuntimeError::Layout(LayoutError::BlankLabel))
    );
    flush(&mut ui);
    assert_eq!(ui.children(&grid_id)?, editors);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "b-editor")));
    assert_eq!(
        details(&ui, "b")
            .semantics
            .and_then(|semantics| semantics.name),
        Some("Label b".to_owned())
    );
    Ok(())
}

#[test]
fn required_marks_the_row_editor() -> TestResult {
    let mut ui = ui();
    ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new([
            Property::new("a", "Name", button("a-editor")).required(true),
            Property::new("b", "Note", button("b-editor")),
        ]),
    ))?;
    flush(&mut ui);
    assert_eq!(
        details(&ui, "a").properties["required"],
        InspectValue::Bool(true)
    );
    assert_eq!(
        details(&ui, "b").properties["required"],
        InspectValue::Bool(false)
    );
    Ok(())
}

#[test]
fn dragging_the_zone_previews_commits_on_release_and_escape_cancels() -> TestResult {
    let mut ui = ui();
    ui.mount_tree(grid(&["a", "b"]))?;
    flush(&mut ui);
    let zone = 64.0 + 4.0;
    pointer(
        &mut ui,
        zone,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, zone + 50.0, 5.0, PointerKind::Move);
    assert_eq!(label_width(&ui, "grid"), 114.0);
    key(&mut ui, KeyCode::Escape, KeyAction::Press);
    key(&mut ui, KeyCode::Escape, KeyAction::Release);
    assert_eq!(label_width(&ui, "grid"), 64.0, "Escape restores the width");
    pointer(
        &mut ui,
        zone + 50.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(label_width(&ui, "grid"), 64.0);

    pointer(
        &mut ui,
        zone,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, zone + 30.0, 5.0, PointerKind::Move);
    pointer(
        &mut ui,
        zone + 30.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(label_width(&ui, "grid"), 94.0);
    // Dragging far past either bound clamps the column.
    pointer(
        &mut ui,
        94.0 + 4.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 0.0, 5.0, PointerKind::Move);
    pointer(
        &mut ui,
        0.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(label_width(&ui, "grid"), 64.0, "never below ControlWidth");
    Ok(())
}

#[test]
fn a_press_outside_the_zone_is_left_to_the_editor() -> TestResult {
    let mut ui = ui();
    ui.mount_tree(grid(&["a"]))?;
    flush(&mut ui);
    pointer(
        &mut ui,
        20.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 120.0, 5.0, PointerKind::Move);
    pointer(
        &mut ui,
        120.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(label_width(&ui, "grid"), 64.0);
    Ok(())
}

#[test]
fn keys_resize_while_the_grid_has_focus_and_focus_loss_cancels() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(grid(&["a", "b"]))?;
    flush(&mut ui);
    ui.focus(&root)?;
    flush(&mut ui);
    key(&mut ui, KeyCode::ArrowRight, KeyAction::Press);
    key(&mut ui, KeyCode::ArrowRight, KeyAction::Repeat);
    assert_eq!(label_width(&ui, "grid"), 80.0);
    key(&mut ui, KeyCode::ArrowRight, KeyAction::Release);
    assert_eq!(label_width(&ui, "grid"), 80.0, "release commits");

    key(&mut ui, KeyCode::ShiftLeft, KeyAction::Press);
    key(&mut ui, KeyCode::ArrowRight, KeyAction::Press);
    assert_eq!(label_width(&ui, "grid"), 112.0);
    key(&mut ui, KeyCode::ShiftLeft, KeyAction::Release);
    // Focus moves to an editor while the arrow is held: the resize cancels.
    let editor = find(&ui, &root, "a-editor");
    ui.focus(&editor)?;
    flush(&mut ui);
    assert_eq!(label_width(&ui, "grid"), 80.0);
    key(&mut ui, KeyCode::ArrowRight, KeyAction::Release);
    assert_eq!(label_width(&ui, "grid"), 80.0);
    // Arrows in an editor do not resize the grid.
    key(&mut ui, KeyCode::ArrowLeft, KeyAction::Press);
    key(&mut ui, KeyCode::ArrowLeft, KeyAction::Release);
    assert_eq!(label_width(&ui, "grid"), 80.0);
    Ok(())
}

#[test]
fn hiding_during_a_drag_cancels_it() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new("column", Column::new([grid(&["a"])])))?;
    flush(&mut ui);
    let grid_id = find(&ui, &root, "grid");
    pointer(
        &mut ui,
        68.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 118.0, 5.0, PointerKind::Move);
    ui.set_visible(&grid_id, false)?;
    flush(&mut ui);
    ui.set_visible(&grid_id, true)?;
    flush(&mut ui);
    assert_eq!(label_width(&ui, "grid"), 64.0);
    pointer(
        &mut ui,
        118.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(label_width(&ui, "grid"), 64.0);
    Ok(())
}

#[test]
fn grids_bound_to_one_signal_stay_aligned_and_are_not_persisted() -> TestResult {
    let mut ui = ui();
    let width = ui.signal(Length::new(100.0)?);
    let root = ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new(
                "first",
                PropertyGrid::new(rows(&["a"])).bind_label_width(width.clone()),
            ),
            Element::new(
                "second",
                PropertyGrid::new(rows(&["b"])).bind_label_width(width.clone()),
            ),
        ]),
    ))?;
    flush(&mut ui);
    assert_eq!(label_width(&ui, "first"), 100.0);
    assert_eq!(label_width(&ui, "second"), 100.0);
    let first = ui.bounds(&find(&ui, &root, "first"))?.unwrap();
    let y = first.y() + 5.0;
    pointer(
        &mut ui,
        104.0,
        y,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 144.0, y, PointerKind::Move);
    // The drag previews on its own grid; the signal is written on release.
    assert_eq!(label_width(&ui, "first"), 140.0);
    assert_eq!(label_width(&ui, "second"), 100.0);
    assert_eq!(width.get()?, Length::new(100.0)?);
    pointer(
        &mut ui,
        144.0,
        y,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(width.get()?, Length::new(140.0)?);
    assert_eq!(label_width(&ui, "second"), 140.0);

    // An application write during a drag wins over it.
    pointer(
        &mut ui,
        144.0,
        y,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 184.0, y, PointerKind::Move);
    width.set(Length::new(90.0)?)?;
    flush(&mut ui);
    pointer(
        &mut ui,
        184.0,
        y,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(width.get()?, Length::new(90.0)?);
    assert_eq!(label_width(&ui, "first"), 90.0);
    assert_eq!(
        details(&ui, "first").properties["shared"],
        InspectValue::Bool(true)
    );
    assert_eq!(ui.capture_workspace()?.entries().count(), 0);
    Ok(())
}

#[test]
fn an_unbound_width_persists_once_resized_and_round_trips() -> TestResult {
    let mut ui = ui();
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            grid(&["a"]),
            Element::new("transient", PropertyGrid::new(rows(&["b"])).persist(false)),
        ]),
    ))?;
    flush(&mut ui);
    let untouched = ui.capture_workspace()?;
    let state = untouched.entries().next().unwrap().1.clone();
    assert_eq!(
        untouched.entries().count(),
        1,
        "the transient grid stays out"
    );
    assert!(
        state.get("label-width").is_none(),
        "the column follows its labels"
    );
    pointer(
        &mut ui,
        68.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 98.0, 5.0, PointerKind::Move);
    // A capture during a drag reports the committed width: none yet.
    assert_eq!(ui.capture_workspace()?, untouched);
    pointer(
        &mut ui,
        98.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    let saved = ui.capture_workspace()?;
    let entries: Vec<_> = saved
        .entries()
        .map(|(path, state)| (path.to_string(), state.clone()))
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "column/grid");
    assert_eq!(entries[0].1.kind(), "property-grid");
    assert_eq!(entries[0].1.number("label-width")?, 94.0);

    let mut other = ui_with_grid()?;
    assert!(other.restore_workspace(&saved)?.is_complete());
    flush(&mut other);
    assert_eq!(label_width(&other, "grid"), 94.0);
    assert_eq!(other.capture_workspace()?, saved);
    // Restoring the untouched document returns to the labels' width.
    assert!(other.restore_workspace(&untouched)?.is_complete());
    flush(&mut other);
    assert_eq!(label_width(&other, "grid"), 64.0);
    Ok(())
}

#[test]
fn a_restored_width_captures_exactly_as_the_document_held_it() -> TestResult {
    // 100.1 has no exact f32: a restore followed by a capture must still give
    // back the document's number, not its rounded layout value.
    let mut ui = ui_with_grid()?;
    let path = WorkspacePath::new(["column", "grid"])?;
    let state =
        ViewState::new("property-grid")?.with("label-width", WorkspaceValue::number(100.1)?)?;
    let document = Workspace::new(WorkspaceSettings::default(), [(path, state)])?;
    assert!(ui.restore_workspace(&document)?.is_complete());
    flush(&mut ui);
    let captured = ui.capture_workspace()?;
    let (_, captured_state) = captured.entries().next().unwrap();
    assert_eq!(captured_state.number("label-width")?, 100.1);
    let (_, restored_state) = document.entries().next().unwrap();
    assert_eq!(captured_state, restored_state);
    Ok(())
}

fn ui_with_grid() -> Result<Runtime, RuntimeError> {
    let mut ui = ui();
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            grid(&["a"]),
            Element::new("transient", PropertyGrid::new(rows(&["b"])).persist(false)),
        ]),
    ))?;
    flush(&mut ui);
    Ok(ui)
}

#[test]
fn restore_rejects_invalid_documents_and_a_resize_in_progress() -> TestResult {
    let mut ui = ui_with_grid()?;
    let path = WorkspacePath::new(["column", "grid"])?;
    let restore = |ui: &mut Runtime, state: ViewState| {
        ui.restore_workspace(
            &Workspace::new(WorkspaceSettings::default(), [(path.clone(), state)]).unwrap(),
        )
        .unwrap()
        .rejected()
        .map(|(_, reason)| reason.clone())
        .collect::<Vec<_>>()
    };
    let unknown = ViewState::new("property-grid")?
        .with("label-width", WorkspaceValue::number(90.0)?)?
        .with("ratio", WorkspaceValue::number(0.5)?)?;
    assert_eq!(
        restore(&mut ui, unknown),
        [RestoreRejection::UnknownValue("ratio".into())]
    );
    let negative =
        ViewState::new("property-grid")?.with("label-width", WorkspaceValue::number(-1.0)?)?;
    assert_eq!(
        restore(&mut ui, negative),
        [RestoreRejection::OutOfRange("label-width".into())]
    );
    let text =
        ViewState::new("property-grid")?.with("label-width", WorkspaceValue::text("wide")?)?;
    assert_eq!(restore(&mut ui, text).len(), 1);
    let valid =
        ViewState::new("property-grid")?.with("label-width", WorkspaceValue::number(90.0)?)?;
    pointer(
        &mut ui,
        68.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    assert_eq!(restore(&mut ui, valid.clone()), [RestoreRejection::Busy]);
    pointer(
        &mut ui,
        68.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    flush(&mut ui);
    assert_eq!(label_width(&ui, "grid"), 64.0, "rejections changed nothing");
    assert!(restore(&mut ui, valid).is_empty());
    flush(&mut ui);
    assert_eq!(label_width(&ui, "grid"), 90.0);
    Ok(())
}

#[test]
fn a_grid_that_is_not_resizable_takes_no_focus_and_ignores_the_zone() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new(rows(&["a"])).resizable(false),
    ))?;
    flush(&mut ui);
    // Tab goes straight to the editor; the grid takes no Tab stop.
    ui.focus_next(FocusDirection::Forward)?;
    flush(&mut ui);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "a-editor")));
    pointer(
        &mut ui,
        68.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 118.0, 5.0, PointerKind::Move);
    pointer(
        &mut ui,
        118.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(label_width(&ui, "grid"), 64.0);
    assert!(details(&ui, "grid").semantics.unwrap().actions.is_empty());
    Ok(())
}

#[test]
fn help_and_status_appear_in_the_editor_column() -> TestResult {
    let mut ui = ui();
    let status = ui.signal(FieldStatus::Normal);
    let root = ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new([Property::new("name", "Name", button("name-editor"))
            .help("Shown in lists")
            .status_signal(status.read_only())]),
    ))?;
    flush(&mut ui);
    assert_eq!(
        details(&ui, "name")
            .semantics
            .unwrap()
            .description
            .as_deref(),
        Some("Shown in lists")
    );
    let field = ui.bounds(&find(&ui, &root, "name"))?.unwrap();
    assert_eq!(field.x(), 72.0, "help text sits under the editor");
    status.set(FieldStatus::error("Required"))?;
    flush(&mut ui);
    let field = details(&ui, "name").semantics.unwrap();
    assert!(field.invalid);
    assert_eq!(field.description.as_deref(), Some("Required"));
    Ok(())
}

#[test]
fn a_row_with_a_described_status_describes_its_editor() -> TestResult {
    let mut ui = ui();
    ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new([Property::new("name", "Name", button("name-editor"))
            .help("Shown in lists")
            .status(FieldStatus::warning("Shortened in the mixer"))]),
    ))?;
    flush(&mut ui);
    let field = details(&ui, "name").semantics.unwrap();
    assert_eq!(field.description.as_deref(), Some("Shortened in the mixer"));
    assert!(field.warning && !field.invalid);
    Ok(())
}

#[test]
fn a_hidden_row_takes_no_space() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(grid(&["a", "b", "c"]))?;
    flush(&mut ui);
    let c_before = ui.bounds(&find(&ui, &root, "c"))?.unwrap().y();
    ui.set_visible(&find(&ui, &root, "b"), false)?;
    flush(&mut ui);
    let a = ui.bounds(&find(&ui, &root, "a"))?.unwrap();
    let c = ui.bounds(&find(&ui, &root, "c"))?.unwrap();
    assert!(c.y() < c_before);
    assert_eq!(c.y(), a.y() + a.height() + 8.0);
    Ok(())
}

#[test]
fn an_initial_width_is_kept_across_descriptions() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "holder",
        Column::new([Element::new(
            "grid",
            PropertyGrid::new(rows(&["a"])).label_width(Length::new(150.0)?),
        )]),
    ))?;
    flush(&mut ui);
    assert_eq!(label_width(&ui, "grid"), 150.0);
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "grid",
            PropertyGrid::new(rows(&["a"])).label_width(Length::new(80.0)?),
        )],
    )?;
    flush(&mut ui);
    assert_eq!(label_width(&ui, "grid"), 150.0);
    Ok(())
}

#[test]
fn a_revoked_resize_permission_cancels_a_drag_and_restores_the_width() -> TestResult {
    let mut ui = ui();
    let allowed = ui.signal(true);
    ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new(rows(&["a", "b"])).resizable(allowed.read_only()),
    ))?;
    flush(&mut ui);
    pointer(
        &mut ui,
        68.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 118.0, 5.0, PointerKind::Move);
    assert_eq!(label_width(&ui, "grid"), 114.0);
    allowed.set(false)?;
    flush(&mut ui);
    assert_eq!(
        label_width(&ui, "grid"),
        64.0,
        "revocation restores the width"
    );
    assert_eq!(
        details(&ui, "grid").properties["resizable"],
        InspectValue::Bool(false)
    );
    pointer(&mut ui, 140.0, 5.0, PointerKind::Move);
    pointer(
        &mut ui,
        140.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(
        label_width(&ui, "grid"),
        64.0,
        "the rest of the press is ignored"
    );
    allowed.set(true)?;
    flush(&mut ui);
    pointer(
        &mut ui,
        68.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 98.0, 5.0, PointerKind::Move);
    pointer(
        &mut ui,
        98.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(label_width(&ui, "grid"), 94.0, "a new press resizes again");
    // A fixed denial cannot be lifted by a later bound permission.
    let mut locked = ui_locked()?;
    pointer(
        &mut locked,
        68.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut locked, 118.0, 5.0, PointerKind::Move);
    assert_eq!(label_width(&locked, "grid"), 64.0);
    Ok(())
}

fn ui_locked() -> Result<Runtime, Box<dyn std::error::Error>> {
    let mut ui = ui();
    let allowed = ui.signal(true);
    ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new(rows(&["a"]))
            .resizable(false)
            .resizable(allowed.read_only()),
    ))?;
    flush(&mut ui);
    Ok(ui)
}
