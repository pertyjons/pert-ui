//! The knob shares the slider's edit protocol with a rotary presentation.
use pert_ui::*;
use std::time::Duration;

struct Setup {
    ui: Runtime,
    parameter: Parameter<f32>,
    host: ParameterHost<f32>,
    root: WidgetId,
    knob: WidgetId,
    center: Point,
}

fn setup(knob: impl FnOnce(Parameter<f32>) -> Knob) -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
        .unwrap();
    let (parameter, host) = ui
        .parameter(
            ParameterId::new(1, 1, "value").unwrap(),
            ParameterSpec::new("Value", -100.0, 100.0, 0.0).unwrap(),
        )
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([Element::new("knob", knob(parameter.clone()))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let knob = ui.children(&root).unwrap()[0].clone();
    let bounds = ui.bounds(&knob).unwrap().unwrap();
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.width() / 2.0,
    )
    .unwrap();
    Setup {
        ui,
        parameter,
        host,
        root,
        knob,
        center,
    }
}

fn pointer(ui: &mut Runtime, point: Point, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
}

fn offset(point: Point, dy: f32) -> Point {
    Point::new(point.x(), point.y() + dy).unwrap()
}

fn base(parameter: &Parameter<f32>) -> f32 {
    parameter.snapshot().get().unwrap().base
}

fn assert_near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-3, "{actual} != {expected}");
}

fn travel() -> f32 {
    ThemeMetrics::default().get(MetricRole::KnobTravel)
}

#[test]
fn default_size_is_the_themed_diameter_and_grows_for_the_readout() {
    let plain = setup(Knob::new);
    let diameter = ThemeMetrics::default().get(MetricRole::KnobDiameter);
    let bounds = plain.ui.bounds(&plain.knob).unwrap().unwrap();
    assert_eq!((bounds.width(), bounds.height()), (diameter, diameter));
    let readout = setup(|p| Knob::new(p).show_value(true));
    let bounds = readout.ui.bounds(&readout.knob).unwrap().unwrap();
    assert!(bounds.height() > diameter);
    assert!(bounds.width() >= diameter);
}

#[test]
fn press_keeps_the_value_and_vertical_drag_moves_relative_to_it() {
    let Setup {
        mut ui,
        parameter,
        center,
        host: _host,
        ..
    } = setup(Knob::new);
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    let snapshot = parameter.snapshot().get().unwrap();
    // The press begins an edit without jumping to the pointer position.
    assert_eq!((snapshot.base, snapshot.preview), (0.0, None));
    // Upward by a quarter of the travel adds a quarter of the 200-wide range.
    let up = offset(center, -travel() / 4.0);
    pointer(&mut ui, up, PointerKind::Move);
    assert_near(parameter.snapshot().get().unwrap().preview.unwrap(), 50.0);
    // Horizontal movement does not change a knob.
    pointer(
        &mut ui,
        Point::new(up.x() + 80.0, up.y()).unwrap(),
        PointerKind::Move,
    );
    assert_near(parameter.snapshot().get().unwrap().preview.unwrap(), 50.0);
    pointer(&mut ui, up, PointerKind::Release(PointerButton::Primary));
    assert_near(base(&parameter), 50.0);
    assert_eq!(parameter.snapshot().get().unwrap().revision.get(), 1);
}

#[test]
fn shift_divides_drag_sensitivity_by_ten() {
    let Setup {
        mut ui,
        parameter,
        center,
        host: _host,
        ..
    } = setup(Knob::new);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    let down = offset(center, travel() / 4.0);
    pointer(&mut ui, down, PointerKind::Move);
    pointer(&mut ui, down, PointerKind::Release(PointerButton::Primary));
    assert_near(base(&parameter), -5.0);
}

#[test]
fn escape_cancels_a_drag_and_keeps_the_base() {
    let Setup {
        mut ui,
        parameter,
        knob,
        center,
        host: _host,
        ..
    } = setup(Knob::new);
    ui.focus(&knob).unwrap();
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, offset(center, -40.0), PointerKind::Move);
    assert!(parameter.snapshot().get().unwrap().preview.is_some());
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    let snapshot = parameter.snapshot().get().unwrap();
    assert_eq!((snapshot.base, snapshot.preview), (0.0, None));
    // The release after a cancellation commits nothing.
    pointer(
        &mut ui,
        offset(center, -40.0),
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(base(&parameter), 0.0);
    assert_eq!(parameter.snapshot().get().unwrap().revision.get(), 0);
}

#[test]
fn double_click_resets_to_the_default_in_one_edit() {
    let Setup {
        mut ui,
        parameter,
        host,
        center,
        ..
    } = setup(Knob::new);
    host.write_base(60.0).unwrap();
    ui.flush(Duration::from_millis(10)).unwrap();
    for time in [20, 200] {
        ui.flush(Duration::from_millis(time)).unwrap();
        pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
        pointer(
            &mut ui,
            center,
            PointerKind::Release(PointerButton::Primary),
        );
    }
    assert_eq!(base(&parameter), 0.0);
    // A third press starts a new sequence instead of resetting again.
    host.write_base(30.0).unwrap();
    ui.flush(Duration::from_millis(300)).unwrap();
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        center,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(base(&parameter), 30.0);
}

#[test]
fn a_click_records_no_edit_and_a_double_click_exactly_one() {
    let Setup {
        mut ui,
        parameter,
        host,
        center,
        ..
    } = setup(Knob::new);
    host.write_base(60.0).unwrap();
    ui.flush(Duration::from_millis(10)).unwrap();
    let revision = |p: &Parameter<f32>| p.snapshot().get().unwrap().revision.get();
    let before = revision(&parameter);
    let click = |ui: &mut Runtime| {
        pointer(ui, center, PointerKind::Press(PointerButton::Primary));
        pointer(ui, center, PointerKind::Release(PointerButton::Primary));
    };
    ui.flush(Duration::from_millis(20)).unwrap();
    click(&mut ui);
    assert_eq!(revision(&parameter), before, "a lone click commits nothing");
    ui.flush(Duration::from_millis(200)).unwrap();
    click(&mut ui);
    assert_eq!(base(&parameter), 0.0);
    assert_eq!(
        revision(&parameter),
        before + 1,
        "the double-click is one committed edit"
    );
}

#[test]
fn rebinding_a_knob_ends_its_click_sequence() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
        .unwrap();
    let spec = || ParameterSpec::new("Value", -100.0, 100.0, 0.0).unwrap();
    let (first, _first_host) = ui
        .parameter(ParameterId::new(1, 1, "first").unwrap(), spec())
        .unwrap();
    let (second, second_host) = ui
        .parameter(ParameterId::new(1, 2, "second").unwrap(), spec())
        .unwrap();
    second_host.write_base(40.0).unwrap();
    let row = |p: &Parameter<f32>| vec![Element::new("knob", Knob::new(p.clone()))];
    let root = ui
        .mount_tree(Element::new("row", Row::new(row(&first))))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let knob = ui.children(&root).unwrap()[0].clone();
    let bounds = ui.bounds(&knob).unwrap().unwrap();
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.width() / 2.0,
    )
    .unwrap();
    ui.flush(Duration::from_millis(20)).unwrap();
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        center,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.reconcile_children(&root, row(&second)).unwrap();
    ui.flush(Duration::from_millis(100)).unwrap();
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        center,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(base(&second), 40.0, "no reset across parameters");
}

#[test]
fn slow_presses_and_a_drag_before_a_press_are_not_a_double_click() {
    let Setup {
        mut ui,
        parameter,
        host,
        center,
        ..
    } = setup(Knob::new);
    host.write_base(60.0).unwrap();
    ui.flush(Duration::from_millis(10)).unwrap();
    for time in [20, 800] {
        ui.flush(Duration::from_millis(time)).unwrap();
        pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
        pointer(
            &mut ui,
            center,
            PointerKind::Release(PointerButton::Primary),
        );
    }
    assert_eq!(base(&parameter), 60.0);
    // A press that moved the value was a drag; a quick press after it is not
    // the second half of a double-click.
    ui.flush(Duration::from_millis(2000)).unwrap();
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    let up = offset(center, -travel() / 20.0);
    pointer(&mut ui, up, PointerKind::Move);
    pointer(&mut ui, up, PointerKind::Release(PointerButton::Primary));
    assert_near(base(&parameter), 70.0);
    ui.flush(Duration::from_millis(2100)).unwrap();
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        center,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_near(base(&parameter), 70.0);
}

#[test]
fn keyboard_and_wheel_match_the_slider() {
    let Setup {
        mut ui,
        parameter,
        knob,
        center,
        host: _host,
        ..
    } = setup(Knob::new);
    ui.focus(&knob).unwrap();
    let tap = |ui: &mut Runtime, code| {
        ui.key_input(code, KeyAction::Press).unwrap();
        ui.key_input(code, KeyAction::Release).unwrap();
    };
    tap(&mut ui, KeyCode::ArrowUp);
    assert_near(base(&parameter), 2.0);
    tap(&mut ui, KeyCode::PageDown);
    assert_near(base(&parameter), -18.0);
    tap(&mut ui, KeyCode::End);
    assert_near(base(&parameter), 100.0);
    tap(&mut ui, KeyCode::Home);
    assert_near(base(&parameter), -100.0);
    tap(&mut ui, KeyCode::Backspace);
    assert_eq!(base(&parameter), 0.0);
    pointer(
        &mut ui,
        center,
        PointerKind::Scroll(Point::new(0.0, -1.0).unwrap()),
    );
    assert_near(base(&parameter), 2.0);
}

#[test]
fn inspection_reports_slider_semantics() {
    let Setup { mut ui, host, .. } = setup(Knob::new);
    host.write_base(25.0).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let snapshot = ui.inspect();
    let component = snapshot
        .components
        .iter()
        .find(|c| c.key == "knob")
        .unwrap();
    let semantics = component.details.semantics.as_ref().unwrap();
    assert_eq!(semantics.role, SemanticRole::Slider);
    assert_eq!(semantics.name.as_deref(), Some("Value"));
    assert_eq!(
        component.details.properties["value"],
        InspectValue::Number(25.0)
    );
}

#[test]
fn arc_origin_changes_in_place_and_only_the_painted_arc() {
    let Setup {
        mut ui,
        parameter,
        root,
        knob,
        host: _host,
        ..
    } = setup(Knob::new);
    // At the default value the minimum origin fills a value arc.
    let minimum = ui.paint().unwrap().commands().len();
    let bounds = ui.bounds(&knob).unwrap();
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "knob",
            Knob::new(parameter.clone()).arc_origin(ArcOrigin::Default),
        )],
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.children(&root).unwrap(), vec![knob.clone()]);
    assert_eq!(ui.bounds(&knob).unwrap(), bounds);
    // The default origin at the default value draws no value arc.
    assert_eq!(ui.paint().unwrap().commands().len(), minimum - 1);
    assert_eq!(base(&parameter), 0.0);
}

#[test]
fn removal_during_a_drag_cancels_the_edit() {
    let Setup {
        mut ui,
        parameter,
        root,
        center,
        host: _host,
        ..
    } = setup(Knob::new);
    pointer(&mut ui, center, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, offset(center, -40.0), PointerKind::Move);
    assert!(parameter.snapshot().get().unwrap().preview.is_some());
    ui.reconcile_children(&root, Vec::new()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let snapshot = parameter.snapshot().get().unwrap();
    assert_eq!((snapshot.base, snapshot.preview), (0.0, None));
    assert_eq!(snapshot.revision.get(), 0);
}

#[test]
fn a_press_without_movement_commits_the_exact_starting_value() {
    // Shared with the slider: a Shift press keeps a slider relative, and
    // releasing it unmoved must not commit a normalized round trip.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let (parameter, host) = ui
        .parameter(
            ParameterId::new(1, 1, "value").unwrap(),
            ParameterSpec::new("Value", 0.0, 100.0, 50.0).unwrap(),
        )
        .unwrap();
    host.write_base(60.0).unwrap();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([Element::new("slider", Slider::new(parameter.clone()))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let slider = ui.children(&root).unwrap()[0].clone();
    let bounds = ui.bounds(&slider).unwrap().unwrap();
    let point = Point::new(
        bounds.x() + bounds.width() / 3.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap();
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    pointer(&mut ui, point, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, point, PointerKind::Release(PointerButton::Primary));
    assert_eq!(base(&parameter), 60.0);
}
