//! Parameter controls preserve transaction and identity boundaries.
use pert_ui::*;
use std::time::Duration;
fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
}
#[test]
fn shared_controls_preview_commit_cancel_and_external_conflict() {
    let mut ui = ui();
    let (p, host) = ui
        .parameter(
            ParameterId::new(1, 1, "value").unwrap(),
            ParameterSpec::new("Value", 0.0, 100.0, 50.0).unwrap(),
        )
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("slider", Slider::new(p.clone())),
                Element::new(
                    "vertical",
                    Slider::new(p.clone()).orientation(Orientation::Vertical),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let ids = ui.children(&root).unwrap();
    ui.focus(&ids[0]).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    assert_eq!(p.snapshot().get().unwrap().base, 50.0);
    assert_eq!(p.snapshot().get().unwrap().preview, Some(51.0));
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release).unwrap();
    assert!(p.snapshot().get().unwrap().preview.is_some());
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    assert_eq!(p.snapshot().get().unwrap().base, 51.0);
    assert_eq!(p.snapshot().get().unwrap().revision.get(), 1);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    host.write_base(20.0).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    assert_eq!(p.snapshot().get().unwrap().base, 20.0);
    ui.flush(Duration::ZERO).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    assert_eq!(p.snapshot().get().unwrap().preview, Some(100.0));
    ui.clear_focus();
    assert_eq!(p.snapshot().get().unwrap().base, 20.0);
    assert_eq!(p.snapshot().get().unwrap().preview, None);
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
    assert!(ui.next_deadline().is_none());
    ui.focus(&ids[1]).unwrap();
    ui.key_input(KeyCode::Home, KeyAction::Press).unwrap();
    ui.unmount(&root).unwrap();
    assert_eq!(p.snapshot().get().unwrap().base, 20.0);
    assert_eq!(p.snapshot().get().unwrap().preview, None);
}
#[test]
fn logarithmic_drag_and_readonly_follow_the_parameter() {
    let mut ui = ui();
    let (p, host) = ui
        .parameter(
            ParameterId::new(1, 2, "value").unwrap(),
            ParameterSpec::new("Value", 1.0, 100.0, 10.0)
                .unwrap()
                .with_mapping(ParameterMapping::Logarithmic)
                .unwrap(),
        )
        .unwrap();
    let id = ui
        .mount_tree(Element::new("slider", Slider::new(p.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(
        &mut ui,
        200.0,
        100.0,
        PointerKind::Press(PointerButton::Primary),
    );
    assert!((p.snapshot().get().unwrap().preview.unwrap() - 10.0).abs() < 0.01);
    pointer(
        &mut ui,
        200.0,
        100.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    assert!(
        (p.spec()
            .unwrap()
            .normalize(&p.snapshot().get().unwrap().base)
            .unwrap()
            .get()
            - 0.501)
            .abs()
            < 0.0001
    );
    let before = p.snapshot().get().unwrap().base;
    host.replace_spec(p.spec().unwrap().read_only()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(
        &mut ui,
        399.0,
        100.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(
        &mut ui,
        399.0,
        100.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(p.snapshot().get().unwrap().base, before);
    ui.focus(&id).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    assert_eq!(p.snapshot().get().unwrap().base, before);
}
#[test]
fn vertical_pointer_gestures_cancel_on_escape_and_hide() {
    {
        let mut ui = ui();
        let (p, _host) = ui
            .parameter(
                ParameterId::new(1, 3, "value").unwrap(),
                ParameterSpec::new("Value", 0.0, 1.0, 0.5).unwrap(),
            )
            .unwrap();
        let id = ui
            .mount_tree(Element::new(
                "control",
                Slider::new(p.clone()).orientation(Orientation::Vertical),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        pointer(
            &mut ui,
            100.0,
            100.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(&mut ui, 100.0, 70.0, PointerKind::Move);
        assert!(p.snapshot().get().unwrap().preview.unwrap() > 0.5);
        ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
        assert_eq!(p.snapshot().get().unwrap().preview, None);
        pointer(
            &mut ui,
            100.0,
            70.0,
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(p.snapshot().get().unwrap().base, 0.5);
        pointer(
            &mut ui,
            100.0,
            100.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(&mut ui, 100.0, 70.0, PointerKind::Move);
        ui.set_visible(&id, false).unwrap();
        assert_eq!(p.snapshot().get().unwrap().preview, None);
    }
}

#[test]
fn pointer_conflict_retains_terminal_reason_and_fine_drag_uses_held_modifiers() {
    let mut ui = ui();
    let (p, host) = ui
        .parameter(
            ParameterId::new(1, 4, "value").unwrap(),
            ParameterSpec::new("Value", 0.0, 1.0, 0.5).unwrap(),
        )
        .unwrap();
    ui.mount_tree(Element::new(
        "vertical",
        Slider::new(p.clone()).orientation(Orientation::Vertical),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // A pressed Shift keeps the gesture relative instead of jumping to the press.
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    pointer(
        &mut ui,
        100.0,
        100.0,
        PointerKind::Press(PointerButton::Primary),
    );
    assert_eq!(p.snapshot().get().unwrap().preview, None);
    pointer(&mut ui, 100.0, 84.0, PointerKind::Move);
    let fine = p.snapshot().get().unwrap().preview.unwrap() - 0.5;
    assert!(fine > 0.0);
    host.write_base(0.2).unwrap();
    pointer(&mut ui, 100.0, 70.0, PointerKind::Move);
    pointer(
        &mut ui,
        100.0,
        70.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(p.snapshot().get().unwrap().base, 0.2);
    let snapshot = ui.inspect();
    let details = &snapshot
        .components
        .iter()
        .find(|c| c.key == "vertical")
        .unwrap()
        .details;
    assert!(
        matches!(details.properties.get("edit_outcome"),Some(InspectValue::Text(t)) if t.contains("Conflict"))
    );
    ui.clear_focus();
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    pointer(
        &mut ui,
        100.0,
        100.0,
        PointerKind::Press(PointerButton::Primary),
    );
    // Releasing Shift mid-gesture changes sensitivity without moving the value.
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert_eq!(p.snapshot().get().unwrap().preview, None);
    pointer(&mut ui, 100.0, 84.0, PointerKind::Move);
    let coarse = p.snapshot().get().unwrap().preview.unwrap() - 0.2;
    assert!((coarse / fine - 10.0).abs() < 0.0001);
    host.write_base(0.7).unwrap();
    pointer(
        &mut ui,
        100.0,
        84.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(p.snapshot().get().unwrap().base, 0.7);
}

#[test]
fn constrained_slider_geometry_matches_visible_track_endpoints() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let mut ui = Runtime::new();
        let (w, h) = if orientation == Orientation::Horizontal {
            (10.0, 8.0)
        } else {
            (8.0, 10.0)
        };
        ui.set_viewport(Viewport::new(w, h, 1.0).unwrap()).unwrap();
        let (p, _host) = ui
            .parameter(
                ParameterId::new(1, 5, "value").unwrap(),
                ParameterSpec::new("Value", 0.0, 1.0, 0.5).unwrap(),
            )
            .unwrap();
        ui.mount_tree(Element::new(
            "slider",
            Slider::new(p.clone()).orientation(orientation),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.paint().unwrap();
        for (position, expected) in [(4.0, 0.0), (5.0, 0.5), (6.0, 1.0)] {
            let (x, y) = if orientation == Orientation::Horizontal {
                (position, 4.0)
            } else {
                (4.0, 10.0 - position)
            };
            pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
            pointer(&mut ui, x, y, PointerKind::Release(PointerButton::Primary));
            assert!((p.snapshot().get().unwrap().base - expected).abs() < 0.0001);
        }
    }
}

fn readout_rects(scene: &DisplayList) -> Vec<(Rect, Option<Rect>)> {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask {
                rect, clip, color, ..
            } if *color == Palette::dark().color(ColorRole::OnSurface) => Some((*rect, *clip)),
            _ => None,
        })
        .collect()
}

#[test]
fn readouts_reserve_signed_rounded_endpoints_across_values_and_scales() {
    for scale in [1.0, 1.25, 2.0] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 500.0, scale).unwrap())
            .unwrap();
        let (parameter, host) = ui
            .parameter(
                ParameterId::new(1, 6, "value").unwrap(),
                ParameterSpec::new("Value", -999.999, 99.999, 0.0).unwrap(),
            )
            .unwrap();
        let root = ui
            .mount_tree(Element::new(
                "controls",
                Row::new([
                    Element::new(
                        "horizontal",
                        Slider::new(parameter.clone()).show_value(true).unit("%"),
                    ),
                    Element::new(
                        "vertical",
                        Slider::new(parameter.clone())
                            .orientation(Orientation::Vertical)
                            .show_value(true)
                            .unit("%"),
                    ),
                ]),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let ids = ui.children(&root).unwrap();
        let bounds: Vec<_> = ids.iter().map(|id| ui.bounds(id).unwrap()).collect();
        let original = ui.paint().unwrap();
        let text = readout_rects(&original);
        ui.set_profiling(true).unwrap();
        assert_eq!(text.len(), 2); // one readout per slider
        assert!(text[0].0.x() > bounds[0].unwrap().x());
        for value in [-999.999, -100.0, -10.0, -0.001, -0.0, 9.99, 10.0, 99.999] {
            host.write_base(value).unwrap();
            ui.flush(Duration::ZERO).unwrap();
            let scene = ui.paint().unwrap();
            let pixels = |scene: &DisplayList| {
                scene
                    .commands()
                    .iter()
                    .find_map(|c| match c {
                        DrawCommand::DrawMask { mask, color, .. }
                            if *color == Palette::dark().color(ColorRole::OnSurface) =>
                        {
                            Some(scene.mask(*mask).unwrap().pixels().to_vec())
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            if value.abs() >= 0.01 {
                assert_ne!(
                    pixels(&scene),
                    pixels(&original),
                    "visible number must refresh"
                );
            } else {
                assert_eq!(
                    pixels(&scene),
                    pixels(&original),
                    "rounded negative zero has no sign"
                );
            }
            assert_eq!(
                readout_rects(&scene),
                text,
                "value {value} at scale {scale}"
            );
            assert_eq!(
                ids.iter()
                    .map(|id| ui.bounds(id).unwrap())
                    .collect::<Vec<_>>(),
                bounds
            );
        }
        assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
        assert!(ui.next_deadline().is_none());
        assert!(
            ui.profile().stats.iter().all(|s| !matches!(
                s.operation,
                ProfileOperation::Measure | ProfileOperation::Arrange
            )),
            "ordinary values must not trigger any layout callbacks"
        );
        host.replace_spec(ParameterSpec::new("Value", -10000.0, 10000.0, 0.0).unwrap())
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert!(ui.bounds(&ids[0]).unwrap().unwrap().width() > bounds[0].unwrap().width());
        assert!(
            ui.profile()
                .stats
                .iter()
                .any(|s| s.operation == ProfileOperation::Measure),
            "changed reserved columns must remeasure"
        );
    }
}

#[test]
fn labeled_slider_endpoints_follow_the_painted_track_and_text_does_not_edit() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let mut ui = ui();
        let (parameter, _host) = ui
            .parameter(
                ParameterId::new(1, 7, "value").unwrap(),
                ParameterSpec::new("Value", 0.0, 100.0, 50.0).unwrap(),
            )
            .unwrap();
        ui.mount_tree(Element::new(
            "slider",
            Slider::new(parameter.clone())
                .orientation(orientation)
                .show_value(true)
                .unit("%"),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let scene = ui.paint().unwrap();
        // Rounded tracks compose clipped corner masks and solid interior bands.
        let bounds = scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color }
                    if *color == Palette::dark().color(ColorRole::Outline) =>
                {
                    Some(*rect)
                }
                DrawCommand::DrawMask {
                    rect, color, clip, ..
                } if *color == Palette::dark().color(ColorRole::Outline) => {
                    Some(clip.map_or(*rect, |clip| rect.intersection(clip)))
                }
                _ => None,
            })
            .fold(
                [
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                ],
                |b, rect| {
                    let [x, y, w, h] = rect.values();
                    [b[0].min(x), b[1].min(y), b[2].max(x + w), b[3].max(y + h)]
                },
            );
        let [x, y, right, bottom] = bounds;
        let (w, h) = (right - x, bottom - y);
        let points = if orientation == Orientation::Horizontal {
            [(x, y + h / 2.0, 0.0), (x + w, y + h / 2.0, 100.0)]
        } else {
            [(x + w / 2.0, y + h, 0.0), (x + w / 2.0, y, 100.0)]
        };
        for (x, y, value) in points {
            pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
            ui.flush(Duration::ZERO).unwrap();
            assert_eq!(parameter.snapshot().get().unwrap().preview, Some(value));
            pointer(&mut ui, x, y, PointerKind::Release(PointerButton::Primary));
            assert_eq!(parameter.snapshot().get().unwrap().base, value);
        }
        let [x, y, w, h] = readout_rects(&scene)[0].0.values();
        pointer(
            &mut ui,
            x + w / 2.0,
            y + h / 2.0,
            PointerKind::Press(PointerButton::Primary),
        );
        assert!(parameter.snapshot().get().unwrap().preview.is_none());
        pointer(
            &mut ui,
            x + w / 2.0,
            y + h / 2.0,
            PointerKind::Release(PointerButton::Primary),
        );
    }
}

#[test]
fn optional_readouts_and_tiny_allocations_remain_valid() {
    for show in [false, true] {
        let mut ui = ui();
        let (p, _host) = ui
            .parameter(
                ParameterId::new(1, 8, "value").unwrap(),
                ParameterSpec::new("Value", 0.0, 100.0, 50.0).unwrap(),
            )
            .unwrap();
        ui.mount_tree(Element::new("slider", Slider::new(p).show_value(show)))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(readout_rects(&ui.paint().unwrap()).len(), usize::from(show));
        for (w, h) in [(8.0, 8.0), (1.0, 20.0), (20.0, 1.0), (0.0, 0.0)] {
            ui.set_viewport(Viewport::new(w, h, 1.0).unwrap()).unwrap();
            ui.flush(Duration::ZERO).unwrap();
            ui.paint().unwrap();
        }
    }
}

#[test]
fn slider_without_text_does_not_subscribe_to_text_color() {
    let mut ui = ui();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 11, "value").unwrap(),
            ParameterSpec::new("Value", 0.0, 1.0, 0.5).unwrap(),
        )
        .unwrap();
    ui.mount_tree(Element::new("slider", Slider::new(parameter)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    ui.set_profiling(true).unwrap();
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::OnSurface, Color::ACCENT)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    assert!(
        ui.profile()
            .stats
            .iter()
            .all(|s| s.operation != ProfileOperation::Paint)
    );
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::Primary, Color::MUTED)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    assert!(
        ui.profile()
            .stats
            .iter()
            .any(|s| s.operation == ProfileOperation::Paint)
    );
}

#[test]
fn focused_wheel_steps_commit_once_with_mapping_fine_adjustment_and_limits() {
    for kind in 0..2 {
        let mut ui = ui();
        let spec = ParameterSpec::new("Value", 1.0, 100.0, 10.0)
            .unwrap()
            .with_mapping(ParameterMapping::Logarithmic)
            .unwrap();
        let (p, host) = ui
            .parameter(ParameterId::new(1, 20, "wheel").unwrap(), spec)
            .unwrap();
        let element = if kind == 0 {
            Element::new("control", Slider::new(p.clone()))
        } else {
            Element::new(
                "control",
                Slider::new(p.clone()).orientation(Orientation::Vertical),
            )
        };
        let id = ui.mount_tree(element).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let wheel = |ui: &mut Runtime, dy| {
            ui.pointer_input(PointerEvent::new(
                Point::new(200.0, 100.0).unwrap(),
                PointerKind::Scroll(Point::new(0.0, dy).unwrap()),
            ))
            .unwrap()
        };
        assert_eq!(wheel(&mut ui, -40.0), InputOutcome::Ignored);
        assert_eq!(p.snapshot().get().unwrap().revision.get(), 0);
        ui.focus(&id).unwrap();
        for (dy, expected) in [(-40.0, 0.51), (-0.5, 0.52), (120.0, 0.51)] {
            assert_eq!(wheel(&mut ui, dy), InputOutcome::Handled);
            let snapshot = p.snapshot().get().unwrap();
            assert!(snapshot.preview.is_none());
            let actual = p.spec().unwrap().normalize(&snapshot.base).unwrap().get();
            assert!((actual - expected).abs() < 0.00001);
        }
        assert_eq!(p.snapshot().get().unwrap().revision.get(), 3);
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
        wheel(&mut ui, -1.0);
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
            .unwrap();
        let actual = p
            .spec()
            .unwrap()
            .normalize(&p.snapshot().get().unwrap().base)
            .unwrap()
            .get();
        assert!((actual - 0.511).abs() < 0.00001);
        for (base, dy, expected) in [(99.9, -1.0, 100.0), (1.001, 1.0, 1.0)] {
            host.write_base(base).unwrap();
            wheel(&mut ui, dy);
            assert_eq!(p.snapshot().get().unwrap().base, expected);
        }
        host.replace_spec(p.spec().unwrap().read_only()).unwrap();
        assert_eq!(wheel(&mut ui, -1.0), InputOutcome::Ignored);
        assert_eq!(p.snapshot().get().unwrap().base, 1.0);
        assert!(p.snapshot().get().unwrap().preview.is_none());
        ui.clear_focus();
        ui.flush(Duration::ZERO).unwrap();
        assert!(ui.next_deadline().is_none());
    }
}

#[test]
fn wheel_preserves_active_key_and_pointer_edits_and_disabled_controls() {
    {
        let mut ui = ui();
        let (p, _host) = ui
            .parameter(
                ParameterId::new(1, 21, "wheel").unwrap(),
                ParameterSpec::new("Value", 0.0, 1.0, 0.5).unwrap(),
            )
            .unwrap();
        let id = ui
            .mount_tree(Element::new("control", Slider::new(p.clone())))
            .unwrap();
        ui.focus(&id).unwrap();
        for key in [true, false] {
            if key {
                ui.key_input(KeyCode::ArrowUp, KeyAction::Press).unwrap();
            } else {
                pointer(
                    &mut ui,
                    200.0,
                    100.0,
                    PointerKind::Press(PointerButton::Primary),
                );
            }
            let before = p.snapshot().get().unwrap();
            pointer(
                &mut ui,
                200.0,
                100.0,
                PointerKind::Scroll(Point::new(0.0, -40.0).unwrap()),
            );
            assert_eq!(p.snapshot().get().unwrap(), before);
            ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
            ui.key_input(KeyCode::Escape, KeyAction::Release).unwrap();
            ui.key_input(KeyCode::ArrowUp, KeyAction::Release).unwrap();
            pointer(
                &mut ui,
                200.0,
                100.0,
                PointerKind::Release(PointerButton::Primary),
            );
            assert_eq!(p.snapshot().get().unwrap().base, 0.5);
            assert!(p.snapshot().get().unwrap().preview.is_none());
        }
        ui.unmount(&id).unwrap();
        let disabled = ui
            .mount_tree(Element::new("disabled", Slider::new(p.clone()).disabled()))
            .unwrap();
        ui.focus(&disabled).unwrap();
        pointer(
            &mut ui,
            200.0,
            100.0,
            PointerKind::Scroll(Point::new(0.0, -1.0).unwrap()),
        );
        assert_eq!(p.snapshot().get().unwrap().base, 0.5);
    }
}

#[test]
fn horizontal_slider_drags_left_right_and_shift_reduces_that_motion() {
    let mut distances = Vec::new();
    for fine in [false, true] {
        let mut ui = ui();
        let (p, _host) = ui
            .parameter(
                ParameterId::new(1, 22, "horizontal").unwrap(),
                ParameterSpec::new("Value", 0.0, 1.0, 0.5).unwrap(),
            )
            .unwrap();
        ui.mount_tree(Element::new("slider", Slider::new(p.clone())))
            .unwrap();
        pointer(
            &mut ui,
            200.0,
            100.0,
            PointerKind::Press(PointerButton::Primary),
        );
        let start = p.snapshot().get().unwrap().preview.unwrap();
        if fine {
            ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
        }
        pointer(&mut ui, 200.0, 60.0, PointerKind::Move);
        assert_eq!(p.snapshot().get().unwrap().preview, Some(start));
        pointer(&mut ui, 240.0, 60.0, PointerKind::Move);
        let right = p.snapshot().get().unwrap().preview.unwrap();
        assert!(right > start);
        distances.push(right - start);
        pointer(&mut ui, 160.0, 60.0, PointerKind::Move);
        assert!(p.snapshot().get().unwrap().preview.unwrap() < start);
        ui.clear_focus();
        assert_eq!(p.snapshot().get().unwrap().base, 0.5);
    }
    assert!((distances[0] / distances[1] - 10.0).abs() < 0.0001);
}

#[test]
fn slider_fill_follows_logarithmic_normalized_values_without_layout_or_idle_work() {
    let mut ui = Runtime::new();
    ui.set_profiling(true).unwrap();
    ui.set_viewport(Viewport::new(100.0, 100.0, 1.25).unwrap())
        .unwrap();
    let (parameter, host) = ui
        .parameter(
            ParameterId::new(1, 11, "fill").unwrap(),
            ParameterSpec::new("Value", 1.0, 100.0, 1.0)
                .unwrap()
                .with_mapping(ParameterMapping::Logarithmic)
                .unwrap(),
        )
        .unwrap();
    let id = ui
        .mount_tree(Element::new("slider", Slider::new(parameter)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let primary = ui
        .theme()
        .palette(ThemeMode::Dark)
        .unwrap()
        .color(ColorRole::Primary);
    // The filled part of the track is the only Primary geometry in the scene.
    let coverage = |scene: &DisplayList| {
        scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::DrawMask { mask, color, .. } if *color == primary => Some(
                    scene
                        .mask(*mask)
                        .unwrap()
                        .pixels()
                        .iter()
                        .map(|value| u64::from(*value))
                        .sum::<u64>(),
                ),
                DrawCommand::FillRect { rect, color, .. } if *color == primary => {
                    let [_, _, w, h] = rect.values();
                    Some((w * h) as u64)
                }
                _ => None,
            })
            .sum::<u64>()
    };
    let minimum = ui.paint().unwrap();
    let bounds = ui.bounds(&id).unwrap();
    ui.clear_profile().unwrap();
    host.write_base(10.0).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let middle = ui.paint().unwrap();
    host.write_base(100.0).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let maximum = ui.paint().unwrap();
    assert!(coverage(&minimum) < coverage(&middle) && coverage(&middle) < coverage(&maximum));
    assert_eq!(ui.bounds(&id).unwrap(), bounds);
    assert!(ui.next_deadline().is_none());
    assert!(ui.profile().stats.iter().all(|s| !matches!(
        s.operation,
        ProfileOperation::Measure | ProfileOperation::Arrange
    )));
    assert_eq!(maximum, ui.paint().unwrap());
}

#[test]
fn a_power_mapped_slider_follows_its_curve_and_new_metadata_ends_its_edit() {
    let mut ui = ui();
    let curve = ParameterMapping::Power(PowerExponent::new(2.0).unwrap());
    let (p, host) = ui
        .parameter(
            ParameterId::new(1, 9, "gain").unwrap(),
            ParameterSpec::new("Gain", -60.0, 12.0, 0.0)
                .unwrap()
                .with_mapping(curve)
                .unwrap(),
        )
        .unwrap();
    let id = ui
        .mount_tree(Element::new("gain", Slider::new(p.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // The normalized position follows the curve, not the linear fraction.
    let normalized = ui.inspect().components[0].details.properties["normalized"].clone();
    let InspectValue::Number(normalized) = normalized else {
        panic!("a number");
    };
    assert!(
        (normalized - (60.0f64 / 72.0).sqrt()).abs() < 1e-4,
        "{normalized}"
    );
    let [x, y, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    pointer(
        &mut ui,
        x + w / 2.0,
        y + h / 2.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, x + w * 0.6, y + h / 2.0, PointerKind::Move);
    assert!(p.snapshot().get().unwrap().preview.is_some());
    // Replacing the metadata during the drag ends the edit as a conflict.
    host.replace_spec(ParameterSpec::new("Gain", -60.0, 12.0, 0.0).unwrap())
        .unwrap();
    assert_eq!(p.snapshot().get().unwrap().preview, None);
    pointer(
        &mut ui,
        x + w * 0.6,
        y + h / 2.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(p.snapshot().get().unwrap().base, 0.0);
}
