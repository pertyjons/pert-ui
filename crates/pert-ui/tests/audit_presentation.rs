//! Presentation regression contracts: visibility, scroll axes and reported errors.
use pert_ui::*;
mod support;
use support::{frame, key, widget};

fn ui(width: f32, height: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0).unwrap())
        .unwrap();
    ui
}
fn details(ui: &Runtime, key: &str) -> ComponentDetails {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap()
        .details
}
fn number(ui: &Runtime, key: &str, name: &str) -> f64 {
    match details(ui, key).properties[name] {
        InspectValue::Number(value) => value,
        ref other => panic!("expected number, got {other:?}"),
    }
}
fn wheel(ui: &mut Runtime, point: Point, delta: Point) {
    ui.pointer_input(PointerEvent::new(point, PointerKind::Scroll(delta)))
        .unwrap();
    frame(ui);
}

#[test]
fn removed_container_border_disappears_from_the_next_painted_inspection() {
    let mut ui = ui(300.0, 200.0);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "panel",
                Container::default()
                    .preferred_size(Size::from([120, 40]))
                    .border_role(ColorRole::PanelBorder),
            )]),
        ))
        .unwrap();
    frame(&mut ui);
    assert!(details(&ui, "panel").box_model.border.is_some());
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "panel",
            Container::default().preferred_size(Size::from([120, 40])),
        )],
    )
    .unwrap();
    frame(&mut ui);
    assert!(details(&ui, "panel").box_model.border.is_none());
}

fn oversized(axes: ScrollAxes) -> Element {
    Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "content",
            Container::default()
                .limits(Size::from([800, 700]), None)
                .unwrap(),
        ))
        .axes(axes),
    )
}

#[test]
fn disabled_scroll_axes_clear_offsets_with_oversized_minimum_content_and_restore() {
    for axes in [
        ScrollAxes::Vertical,
        ScrollAxes::Horizontal,
        ScrollAxes::None,
    ] {
        let mut ui = ui(300.0, 200.0);
        let root = ui
            .mount_tree(Element::new(
                "root",
                Container::new(oversized(ScrollAxes::Both)),
            ))
            .unwrap();
        frame(&mut ui);
        wheel(
            &mut ui,
            Point::new(50.0, 50.0).unwrap(),
            Point::new(50.0, 60.0).unwrap(),
        );
        assert!(number(&ui, "scroll", "scroll_x") > 0.0);
        assert!(number(&ui, "scroll", "scroll_y") > 0.0);
        let saved = ui.capture_workspace().unwrap();
        ui.reconcile_children(&root, vec![oversized(axes)]).unwrap();
        frame(&mut ui);
        let forbidden = |ui: &Runtime| {
            if axes != ScrollAxes::Horizontal {
                assert_eq!(number(ui, "scroll", "scroll_x"), 0.0);
            }
            if axes != ScrollAxes::Vertical {
                assert_eq!(number(ui, "scroll", "scroll_y"), 0.0);
            }
        };
        forbidden(&ui);
        if axes == ScrollAxes::Vertical {
            assert_eq!(number(&ui, "scroll", "scroll_y"), 60.0);
        }
        if axes == ScrollAxes::Horizontal {
            assert_eq!(number(&ui, "scroll", "scroll_x"), 50.0);
        }
        let report = ui.restore_workspace(&saved).unwrap();
        assert!(report.rejected().next().is_none());
        frame(&mut ui);
        forbidden(&ui);
    }
}

#[test]
fn hiding_lone_and_perpendicular_regions_keeps_every_layout_phase_valid() {
    for (top, left) in [(true, false), (false, true), (true, true)] {
        let mut ui = ui(300.0, 200.0);
        let mut layout = RegionLayout::new().fixed();
        if top {
            layout = layout
                .top(Element::new("top-content", Label::new("Top")))
                .top_height(Length::new(36.0).unwrap());
        }
        if left {
            layout = layout
                .left(Element::new("left-content", Label::new("Left")))
                .left_width(Length::new(90.0).unwrap());
        }
        let root = ui.mount_tree(Element::new("regions", layout)).unwrap();
        frame(&mut ui);
        let edge = widget(&ui, &root, if left { "left" } else { "top" });
        ui.set_visible(&edge, false).unwrap();
        frame(&mut ui);
        ui.set_visible(&edge, true).unwrap();
        frame(&mut ui);
        assert!(ui.bounds(&edge).unwrap().is_some());
    }
}

#[test]
fn wheel_over_fitting_tabs_reaches_the_outer_scroll_area() {
    let mut ui = ui(300.0, 200.0);
    let root = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "tabs",
                Tabs::new([Tab::new(
                    "one",
                    "One",
                    Element::new(
                        "content",
                        Container::default()
                            .limits(Size::from([100, 700]), None)
                            .unwrap(),
                    ),
                )]),
            )),
        ))
        .unwrap();
    frame(&mut ui);
    let tabs = widget(&ui, &root, "tabs");
    let bounds = ui.bounds(&tabs).unwrap().unwrap();
    wheel(
        &mut ui,
        Point::new(bounds.x() + 30.0, bounds.y() + 10.0).unwrap(),
        Point::new(0.0, 50.0).unwrap(),
    );
    assert!(number(&ui, "scroll", "scroll_y") > 0.0);
    assert_eq!(number(&ui, "tabs", "scroll_x"), 0.0);
}

#[test]
fn overflowing_tab_wheels_scroll_the_strip_and_yield_at_the_end() {
    let mut ui = ui(300.0, 200.0);
    let tabs = Tabs::new((0..5).map(|index| {
        Tab::new(
            format!("page-{index}"),
            format!("Long page title {index}"),
            Element::new(
                "content",
                Container::default()
                    .limits(Size::from([100, 700]), None)
                    .unwrap(),
            ),
        )
    }));
    let root = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new("tabs", tabs)),
        ))
        .unwrap();
    frame(&mut ui);
    let strip = widget(&ui, &root, "tabs");
    let bounds = ui.bounds(&strip).unwrap().unwrap();
    let over = Point::new(bounds.x() + 30.0, bounds.y() + 10.0).unwrap();
    wheel(&mut ui, over, Point::new(0.0, 50.0).unwrap());
    assert!(number(&ui, "tabs", "scroll_x") > 0.0);
    assert_eq!(number(&ui, "scroll", "scroll_y"), 0.0);
    wheel(&mut ui, over, Point::new(0.0, 10000.0).unwrap());
    let end = number(&ui, "tabs", "scroll_x");
    assert_eq!(number(&ui, "scroll", "scroll_y"), 0.0);
    wheel(&mut ui, over, Point::new(0.0, 50.0).unwrap());
    assert_eq!(number(&ui, "tabs", "scroll_x"), end);
    assert_eq!(number(&ui, "scroll", "scroll_y"), 50.0);
}

#[test]
fn dynamic_readout_units_can_be_empty_and_remeasure_without_a_value_change() {
    let mut ui = ui(600.0, 200.0);
    let source = ui.signal(1.0_f64);
    let format = NumberFormat::new(
        "000",
        |_| "100".into(),
        |_| Err(NumberInputError::InvalidNumber),
    )
    .unit(|value: f64| match value as i32 {
        0 => String::new(),
        1 => "%".into(),
        _ => "long unit".into(),
    });
    let root = ui
        .mount_tree(Element::new(
            "root",
            Row::new([Element::new(
                "readout",
                InlineReadout::formatted(source.read_only(), format).unit("fallback"),
            )]),
        ))
        .unwrap();
    frame(&mut ui);
    let readout = widget(&ui, &root, "readout");
    let first = ui.bounds(&readout).unwrap().unwrap().width();
    assert_eq!(
        details(&ui, "readout").properties["unit"],
        InspectValue::Text("%".into())
    );
    source.set(0.0).unwrap();
    frame(&mut ui);
    let empty = ui.bounds(&readout).unwrap().unwrap().width();
    assert_eq!(
        details(&ui, "readout").properties["unit"],
        InspectValue::Text(String::new())
    );
    assert!(empty < first);
    source.set(2.0).unwrap();
    frame(&mut ui);
    assert!(ui.bounds(&readout).unwrap().unwrap().width() > first);
    let text = ui.signal("100".to_owned());
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "readout",
            InlineReadout::new(text.read_only()).unit("literal"),
        )],
    )
    .unwrap();
    frame(&mut ui);
    assert_eq!(
        details(&ui, "readout").properties["unit"],
        InspectValue::Text("literal".into())
    );
    let no_dynamic = NumberFormat::new(
        "000",
        |_: f64| "100".into(),
        |_| Err(NumberInputError::InvalidNumber),
    );
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "readout",
            InlineReadout::formatted(source.read_only(), no_dynamic).unit("fallback"),
        )],
    )
    .unwrap();
    frame(&mut ui);
    assert_eq!(
        details(&ui, "readout").properties["unit"],
        InspectValue::Text("fallback".into())
    );
}

#[test]
fn visibly_invalid_numeric_drafts_report_invalid_slider_semantics() {
    let mut ui = ui(600.0, 200.0);
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 100.0, 10.0).unwrap(),
        )
        .unwrap();
    let row = ui
        .mount_tree(Element::new("field", InlineSlider::new(parameter.clone())))
        .unwrap();
    frame(&mut ui);
    ui.focus(&row).unwrap();
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    ui.text_input("invalid").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert!(
        details(&ui, "field")
            .properties
            .contains_key("validation_error")
    );
    assert!(details(&ui, "field").semantics.unwrap().invalid);
    assert_eq!(parameter.snapshot().get().unwrap().base, 10.0);
}
