//! Adaptive layout preserves the controls' identity and editing contracts.
use crate::support;
use pert_ui::*;
use std::time::Duration;
use support::frame;

fn viewport(ui: &Runtime, width: f32, height: f32) {
    ui.set_viewport(Viewport::new(width, height, 1.0).unwrap())
        .unwrap();
}
fn bounds(ui: &Runtime, id: &WidgetId) -> Rect {
    ui.bounds(id).unwrap().unwrap()
}
fn columns(ui: &Runtime) -> f64 {
    match ui
        .inspect()
        .components
        .iter()
        .find(|c| c.key == "form")
        .unwrap()
        .details
        .properties["columns"]
    {
        InspectValue::Number(n) => n,
        ref other => panic!("{other:?}"),
    }
}
fn field(ui: &Runtime, key: &str) -> Element {
    Element::new(
        key,
        Field::new(Element::new(
            "input",
            TextField::new(ui.signal(String::new())),
        ))
        .label(key),
    )
}

#[test]
fn exact_fit_reflows_by_local_width_and_hidden_fields_leave_no_cells() {
    let mut ui = Runtime::new();
    viewport(&ui, 1000.0, 500.0);
    let outer = ui
        .mount_tree(Element::new(
            "outer",
            Container::new(Element::new(
                "form",
                Form::new([field(&ui, "a"), field(&ui, "b"), field(&ui, "c")]),
            ))
            .size_limits(
                SizeLimits::new(Size::ZERO, Constraints::new(Some(520.0), None).unwrap()).unwrap(),
            ),
        ))
        .unwrap();
    frame(&mut ui);
    let form = ui.children(&outer).unwrap()[0].clone();
    let fields = ui.children(&form).unwrap();
    assert_eq!(columns(&ui), 2.0, "uses panel width, not 1000-pixel window");
    assert_eq!(bounds(&ui, &fields[0]).y(), bounds(&ui, &fields[1]).y());
    assert!(bounds(&ui, &fields[2]).y() > bounds(&ui, &fields[0]).y());
    viewport(&ui, 519.0, 500.0);
    frame(&mut ui);
    assert_eq!(columns(&ui), 1.0);
    viewport(&ui, 520.0, 500.0);
    ui.set_visible(&fields[1], false).unwrap();
    frame(&mut ui);
    assert_eq!(bounds(&ui, &fields[0]).y(), bounds(&ui, &fields[2]).y());
    assert_eq!(ui.children(&form).unwrap(), fields);
    ui.clear_focus();
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
    assert_eq!(
        ui.flush(Duration::from_secs(5)).unwrap().components_updated,
        0
    );
}

#[test]
fn reflow_preserves_focus_numeric_draft_and_tab_order() {
    let mut ui = Runtime::new();
    viewport(&ui, 800.0, 500.0);
    let value = ui.signal(7_i64);
    let form = ui
        .mount_tree(Element::new(
            "form",
            Form::new([
                Element::new(
                    "number",
                    Field::new(Element::new("input", IntegerInput::new(value.clone())))
                        .label("Copies"),
                ),
                field(&ui, "next"),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    let fields = ui.children(&form).unwrap();
    let input = ui.children(&fields[0]).unwrap()[0].clone();
    let next = ui.children(&fields[1]).unwrap()[0].clone();
    ui.focus(&input).unwrap();
    let modifier = KeyCode::primary();
    ui.key_input(modifier, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::KeyA, KeyAction::Release).unwrap();
    ui.key_input(modifier, KeyAction::Release).unwrap();
    ui.text_input("42").unwrap();
    assert_eq!(value.get().unwrap(), 7);
    viewport(&ui, 300.0, 500.0);
    frame(&mut ui);
    assert_eq!(columns(&ui), 1.0);
    assert_eq!(ui.focused(), Some(input.clone()));
    assert_eq!(ui.children(&form).unwrap(), fields);
    assert_eq!(value.get().unwrap(), 7, "reflow must not commit a draft");
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    assert_eq!(value.get().unwrap(), 42);
    ui.focus_next(false).unwrap();
    assert_eq!(ui.focused(), Some(next));
}

#[test]
fn small_viewports_scroll_and_density_zoom_recompute_columns() {
    let mut ui = Runtime::new();
    viewport(&ui, 560.0, 400.0);
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "form",
                Form::new((0..6).map(|i| field(&ui, &format!("field-{i}")))),
            ))
            .axes(ScrollAxes::Both),
        ))
        .unwrap();
    frame(&mut ui);
    assert_eq!(columns(&ui), 2.0);
    ui.theme().set_density(Density::Comfortable).unwrap();
    frame(&mut ui);
    assert_eq!(columns(&ui), 1.0);
    ui.theme().set_density(Density::Standard).unwrap();
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.5).unwrap())
        .unwrap();
    frame(&mut ui);
    assert_eq!(columns(&ui), 1.0);
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.0).unwrap())
        .unwrap();
    viewport(&ui, 180.0, 90.0);
    frame(&mut ui);
    let form = ui.children(&scroll).unwrap()[0].clone();
    assert!(bounds(&ui, &form).width() >= 256.0);
    let fields = ui.children(&form).unwrap();
    let last = ui.children(fields.last().unwrap()).unwrap()[0].clone();
    ui.focus(&last).unwrap();
    frame(&mut ui);
    let rect = bounds(&ui, &last);
    assert!(
        rect.y() >= 0.0 && rect.y() < 90.0,
        "focus reveals the last row: {rect:?}"
    );
    assert_eq!(ui.children(&form).unwrap(), fields);
    // Reveal the clipped right edge with horizontal scrolling, not just the row.
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 20.0).unwrap(),
        PointerKind::Scroll(Point::new(1000.0, 0.0).unwrap()),
    ))
    .unwrap();
    frame(&mut ui);
    let rect = bounds(&ui, &last);
    assert!(rect.x() < 0.0, "horizontal overflow must be reachable");
    assert!(rect.x() + rect.width() <= 180.01);
}

#[test]
fn empty_and_unbounded_forms_have_defined_layout() {
    let mut ui = Runtime::new();
    viewport(&ui, 800.0, 500.0);
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("form", Form::new([field(&ui, "a"), field(&ui, "b")])),
                Element::new("empty", Form::default()),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    assert_eq!(
        columns(&ui),
        1.0,
        "a row measures children with unbounded width"
    );
    let empty = ui.children(&root).unwrap()[1].clone();
    assert_eq!(bounds(&ui, &empty).width(), 0.0);
    assert_eq!(bounds(&ui, &empty).height(), 0.0);
}

#[test]
fn field_messages_wrap_without_overlapping_the_next_row() {
    let mut ui = Runtime::new();
    viewport(&ui, 520.0, 500.0);
    let status = ui.signal(FieldStatus::Normal);
    let form = ui
        .mount_tree(Element::new(
            "form",
            Form::new([
                Element::new(
                    "a",
                    Field::new(Element::new(
                        "input",
                        TextField::new(ui.signal(String::new())),
                    ))
                    .label("Name")
                    .status(status.read_only()),
                ),
                field(&ui, "b"),
                field(&ui, "c"),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    let fields = ui.children(&form).unwrap();
    status.set(FieldStatus::error("A long error message that wraps over several lines while keeping the label and control together.")).unwrap();
    frame(&mut ui);
    let first = bounds(&ui, &fields[0]);
    assert!(bounds(&ui, &fields[2]).y() >= first.y() + first.height());
    assert_eq!(columns(&ui), 2.0);
}
