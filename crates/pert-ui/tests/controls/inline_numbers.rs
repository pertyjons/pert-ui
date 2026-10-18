//! Inline numeric presentation and editing through the public facade.
use crate::support;
use pert_ui::*;
use support::{frame, key};
fn pointer(ui: &mut Runtime, point: Point, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
}
fn mount(ui: &mut Runtime, control: impl Component + 'static) -> WidgetId {
    ui.set_viewport(Viewport::new(600.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("field", control),
                Element::new("next", Button::new("Next", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    frame(ui);
    ui.children(&root).unwrap()[0].clone()
}
fn point(ui: &Runtime, id: &WidgetId) -> Point {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    Point::new(x + w - 12.0, y + h / 2.0).unwrap()
}
fn property(ui: &Runtime, key: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "field")
        .unwrap()
        .details
        .properties[key]
        .clone()
}
fn click(ui: &mut Runtime, id: &WidgetId) {
    let p = point(ui, id);
    pointer(ui, p, PointerKind::Press(PointerButton::Primary));
    pointer(ui, p, PointerKind::Release(PointerButton::Primary));
    frame(ui);
}
#[test]
fn inline_integer_click_text_precision_keys_wheel_and_idle() {
    let mut ui = Runtime::new();
    let value = ui.signal(8_i64);
    let id = mount(
        &mut ui,
        InlineInteger::new(value.clone())
            .range(0, 100)
            .unwrap()
            .label("Vol")
            .unit("%"),
    );
    let bounds = ui.bounds(&id).unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
    assert!(ui.text_input_state().is_none());
    click(&mut ui, &id);
    assert!(ui.text_input_state().is_some());
    ui.text_input("42").unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 8);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 42);
    assert!(ui.next_deadline().is_none());
    assert!(ui.text_input_state().is_none());
    key(&mut ui, KeyCode::ArrowUp);
    assert_eq!(value.get().unwrap(), 43);
    let p = point(&ui, &id);
    pointer(
        &mut ui,
        p,
        PointerKind::Scroll(Point::new(0.0, -1.0).unwrap()),
    );
    assert_eq!(value.get().unwrap(), 44);
    frame(&mut ui);
    assert_eq!(ui.bounds(&id).unwrap(), bounds);
    key(&mut ui, KeyCode::F2);
    ui.text_input("999").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 44);
    assert_eq!(property(&ui, "valid"), InspectValue::Bool(false));
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "display_value"),
        InspectValue::Text("44".into())
    );
    assert!(ui.next_deadline().is_none());
}
#[test]
fn integer_drag_threshold_exact_large_values_and_cancellation() {
    let mut ui = Runtime::new();
    let original = i64::MAX - 10;
    let value = ui.signal(original);
    let id = mount(&mut ui, InlineInteger::new(value.clone()));
    let start = point(&ui, &id);
    pointer(&mut ui, start, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        Point::new(start.x() + 2.0, start.y()).unwrap(),
        PointerKind::Move,
    );
    assert_eq!(value.get().unwrap(), original);
    pointer(
        &mut ui,
        Point::new(start.x() + 8.0, start.y()).unwrap(),
        PointerKind::Move,
    );
    frame(&mut ui);
    assert_eq!(
        property(&ui, "display_value"),
        InspectValue::Text((original + 2).to_string())
    );
    assert_eq!(value.get().unwrap(), original);
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "display_value"),
        InspectValue::Text(original.to_string())
    );
    pointer(&mut ui, start, PointerKind::Release(PointerButton::Primary));
    pointer(&mut ui, start, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        Point::new(start.x() + 8.0, start.y()).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(value.get().unwrap(), original + 2);
    assert!(ui.text_input_state().is_none());
}

#[test]
fn revoked_numeric_input_discards_drag_before_reenabling_integer_and_decimal() {
    for (decimal, flush_revocation, disables) in [false, true]
        .into_iter()
        .flat_map(|decimal| [(decimal, false), (decimal, true)])
        .flat_map(|(decimal, flush)| [(decimal, flush, false), (decimal, flush, true)])
    {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(600.0, 200.0, 1.0).unwrap())
            .unwrap();
        let integer = ui.signal(5_i64);
        let fraction = ui.signal(5.0_f64);
        let control = |read_only| {
            if decimal {
                let field = InlineDecimal::new(fraction.clone());
                Element::new(
                    "field",
                    if read_only {
                        if disables {
                            field.disabled()
                        } else {
                            field.read_only()
                        }
                    } else {
                        field
                    },
                )
            } else {
                let field = InlineInteger::new(integer.clone());
                Element::new(
                    "field",
                    if read_only {
                        if disables {
                            field.disabled()
                        } else {
                            field.read_only()
                        }
                    } else {
                        field
                    },
                )
            }
        };
        let root = ui
            .mount_tree(Element::new("root", Column::new([control(false)])))
            .unwrap();
        frame(&mut ui);
        let field = ui.children(&root).unwrap()[0].clone();
        let p = point(&ui, &field);
        let moved = Point::new(p.x() + 12.0, p.y()).unwrap();
        pointer(&mut ui, p, PointerKind::Press(PointerButton::Primary));
        pointer(&mut ui, moved, PointerKind::Move);
        ui.reconcile_children(&root, vec![control(true)]).unwrap();
        if flush_revocation {
            frame(&mut ui);
            pointer(&mut ui, moved, PointerKind::Release(PointerButton::Primary));
        }
        ui.reconcile_children(&root, vec![control(false)]).unwrap();
        frame(&mut ui);
        pointer(&mut ui, moved, PointerKind::Release(PointerButton::Primary));
        pointer(
            &mut ui,
            Point::new(p.x() + 24.0, p.y()).unwrap(),
            PointerKind::Move,
        );
        assert_eq!(integer.get().unwrap(), 5);
        assert_eq!(fraction.get().unwrap(), 5.0);
        pointer(&mut ui, p, PointerKind::Press(PointerButton::Primary));
        pointer(&mut ui, moved, PointerKind::Move);
        pointer(&mut ui, moved, PointerKind::Release(PointerButton::Primary));
        if decimal {
            assert!(fraction.get().unwrap() > 5.0);
        } else {
            assert!(integer.get().unwrap() > 5);
        }
    }
}
#[test]
fn decimal_custom_format_converts_units_preserves_unchanged_precision_and_width() {
    let mut ui = Runtime::new();
    let value = ui.signal(0.00123456789_f64);
    let format = NumberFormat::new(
        "9999.00 ms",
        |v: f64| format!("{:.2}", v * 1000.0),
        |s| {
            s.parse::<f64>()
                .map(|v| v / 1000.0)
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    )
    .edit_text(|v| (v * 1000.0).to_string());
    let id = mount(
        &mut ui,
        InlineDecimal::new(value.clone())
            .range(0.0, 9.0)
            .unwrap()
            .label("Delay")
            .unit("ms")
            .format(format),
    );
    let bounds = ui.bounds(&id).unwrap();
    click(&mut ui, &id);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 0.00123456789);
    click(&mut ui, &id);
    ui.text_input("250").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 0.25);
    assert_eq!(
        property(&ui, "display_value"),
        InspectValue::Text("250.00".into())
    );
    assert_eq!(ui.bounds(&id).unwrap(), bounds);
}
#[test]
fn external_signal_update_wins_over_drag_and_text_before_next_frame() {
    let mut ui = Runtime::new();
    let value = ui.signal(5_i64);
    let id = mount(&mut ui, InlineInteger::new(value.clone()));
    click(&mut ui, &id);
    ui.text_input("7").unwrap();
    value.set(20).unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 20);
    key(&mut ui, KeyCode::Escape);
    let p = point(&ui, &id);
    pointer(&mut ui, p, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        Point::new(p.x() + 8.0, p.y()).unwrap(),
        PointerKind::Move,
    );
    value.set(30).unwrap();
    pointer(&mut ui, p, PointerKind::Release(PointerButton::Primary));
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 30);
    assert!(ui.text_input_state().is_none());
}
#[test]
fn slider_typed_values_use_parameter_session_and_cancel_on_focus_loss() {
    let mut ui = Runtime::new();
    let (parameter, host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 100.0, 12.34567).unwrap(),
        )
        .unwrap();
    let id = mount(
        &mut ui,
        InlineSlider::new(parameter.clone())
            .label("Amount")
            .unit("%"),
    );
    click(&mut ui, &id);
    assert!(ui.text_input_state().is_some());
    key(&mut ui, KeyCode::Enter);
    assert_eq!(parameter.snapshot().get().unwrap().base, 12.34567);
    click(&mut ui, &id);
    ui.text_input("42.5").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(parameter.snapshot().get().unwrap().base, 42.5);
    assert!(ui.next_deadline().is_none());
    click(&mut ui, &id);
    ui.text_input("66").unwrap();
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(parameter.snapshot().get().unwrap().base, 42.5);
    click(&mut ui, &id);
    ui.text_input("77").unwrap();
    host.write_base(20.0).unwrap();
    frame(&mut ui);
    assert!(ui.text_input_state().is_none());
    key(&mut ui, KeyCode::Enter);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(parameter.snapshot().get().unwrap().base, 20.0);
    assert!(ui.text_input_state().is_none());
}
#[test]
fn slider_custom_parser_rejects_nonfinite_and_out_of_range() {
    let mut ui = Runtime::new();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 1.0, 0.5).unwrap(),
        )
        .unwrap();
    let format = NumberFormat::new(
        "999 %",
        |v: f32| format!("{:.0}", v * 100.0),
        |s| {
            s.parse::<f32>()
                .map(|v| v / 100.0)
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    );
    let id = mount(
        &mut ui,
        InlineSlider::new(parameter.clone())
            .unit("%")
            .format(format),
    );
    for text in ["NaN", "200", "bad"] {
        click(&mut ui, &id);
        ui.text_input(text).unwrap();
        key(&mut ui, KeyCode::Enter);
        frame(&mut ui);
        assert_eq!(parameter.snapshot().get().unwrap().base, 0.5);
        assert_eq!(property(&ui, "text_editing"), InspectValue::Bool(true));
        key(&mut ui, KeyCode::Escape);
        frame(&mut ui);
    }
    click(&mut ui, &id);
    ui.text_input("25").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(parameter.snapshot().get().unwrap().base, 0.25);
}
#[test]
fn slider_value_scrub_commits_once_and_external_changes_do_not_replay() {
    let mut ui = Runtime::new();
    let (parameter, host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 100.0, 50.0).unwrap(),
        )
        .unwrap();
    let id = mount(&mut ui, InlineSlider::new(parameter.clone()));
    let p = point(&ui, &id);
    pointer(&mut ui, p, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        Point::new(p.x() + 16.0, p.y()).unwrap(),
        PointerKind::Move,
    );
    assert_eq!(parameter.snapshot().get().unwrap().base, 50.0);
    assert!((parameter.snapshot().get().unwrap().preview.unwrap() - 60.0).abs() < 0.0001);
    pointer(
        &mut ui,
        Point::new(p.x() + 16.0, p.y()).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(parameter.snapshot().get().unwrap().revision.get(), 1);
    pointer(&mut ui, p, PointerKind::Press(PointerButton::Primary));
    pointer(
        &mut ui,
        Point::new(p.x() + 16.0, p.y()).unwrap(),
        PointerKind::Move,
    );
    host.write_base(15.0).unwrap();
    frame(&mut ui);
    pointer(&mut ui, p, PointerKind::Release(PointerButton::Primary));
    assert_eq!(parameter.snapshot().get().unwrap().base, 15.0);
}
#[test]
fn hidden_inline_editors_cancel_and_stop_caret_work() {
    let mut ui = Runtime::new();
    let value = ui.signal(5_i64);
    let id = mount(&mut ui, InlineInteger::new(value.clone()));
    click(&mut ui, &id);
    ui.text_input("8").unwrap();
    frame(&mut ui);
    assert!(ui.next_deadline().is_some());
    ui.set_visible(&id, false).unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 5);
    assert!(ui.next_deadline().is_none());
    ui.set_visible(&id, true).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "text_editing"), InspectValue::Bool(false));
    assert_eq!(
        property(&ui, "display_value"),
        InspectValue::Text("5".into())
    );
}

#[test]
fn read_only_inline_value_allows_selection_copy_and_preserves_explicit_style() {
    use std::{cell::RefCell, rc::Rc};
    struct ClipboardText(Rc<RefCell<String>>);
    impl Clipboard for ClipboardText {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            Ok(self.0.borrow().clone())
        }
        fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
            *self.0.borrow_mut() = text.into();
            Ok(())
        }
    }
    let copied = Rc::new(RefCell::new(String::new()));
    let mut ui = Runtime::new();
    ui.set_clipboard(ClipboardText(copied.clone())).unwrap();
    let value = ui.signal(123_i64);
    let id = mount(
        &mut ui,
        InlineInteger::new(value.clone())
            .style(TextStyle::Body)
            .label("Read-only")
            .unit("units")
            .read_only(),
    );
    assert_eq!(
        property(&ui, "typography"),
        InspectValue::Text("Body".into())
    );
    click(&mut ui, &id);
    assert_eq!(property(&ui, "text_editing"), InspectValue::Bool(true));
    let draft_style = ui
        .inspect()
        .components
        .iter()
        .find(|component| component.key == "editor")
        .unwrap()
        .details
        .properties["typography"]
        .clone();
    assert_eq!(draft_style, InspectValue::Text("Body".into()));
    assert!(ui.text_input_state().is_none());
    let command = KeyCode::primary();
    ui.key_input(command, KeyAction::Press).unwrap();
    key(&mut ui, KeyCode::KeyC);
    ui.key_input(command, KeyAction::Release).unwrap();
    assert_eq!(&*copied.borrow(), "123");
    ui.text_input("999").unwrap();
    key(&mut ui, KeyCode::ArrowUp);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 123);
    key(&mut ui, KeyCode::Escape);
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    assert_eq!(property(&ui, "text_editing"), InspectValue::Bool(true));
    assert!(ui.next_deadline().is_none());
}
#[test]
fn inline_slider_retains_layout_and_remeasures_changed_default_format_bounds() {
    let mut ui = Runtime::new();
    ui.set_profiling(true).unwrap();
    let (parameter, host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 9.0, 5.0).unwrap(),
        )
        .unwrap();
    let id = mount(&mut ui, InlineSlider::new(parameter));
    ui.clear_profile().unwrap();
    for value in [1.0, 9.0, 0.0] {
        host.write_base(value).unwrap();
        frame(&mut ui);
    }
    assert!(ui.profile().stats.iter().all(|s| !matches!(
        s.operation,
        ProfileOperation::Measure | ProfileOperation::Arrange
    )));
    host.replace_spec(ParameterSpec::new("Amount", 0.0, 10000.0, 0.0).unwrap())
        .unwrap();
    frame(&mut ui);
    assert!(
        ui.profile()
            .stats
            .iter()
            .any(|s| s.operation == ProfileOperation::Measure)
    );
    ui.focus(&id).unwrap();
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
}
#[test]
fn inline_ime_draft_undo_and_caret_stay_inside_the_value_area() {
    let mut ui = Runtime::new();
    let value = ui.signal(12_i64);
    let id = mount(
        &mut ui,
        InlineInteger::new(value.clone())
            .range(0, 999)
            .unwrap()
            .label("Count")
            .unit("items"),
    );
    click(&mut ui, &id);
    ui.text_preedit(&TextPreedit::new("34", Some(0..2)).unwrap())
        .unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 12);
    ui.text_input("34").unwrap();
    frame(&mut ui);
    let (_, state) = ui.text_input_state().unwrap();
    let [_, _, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    let [cx, cy, cw, ch] = state.caret.values();
    assert!(cx > 0.0 && cx + cw <= w && cy + ch <= h);
    let command = KeyCode::primary();
    ui.key_input(command, KeyAction::Press).unwrap();
    key(&mut ui, KeyCode::KeyZ);
    ui.key_input(command, KeyAction::Release).unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 12);
}
#[test]
fn invalid_custom_display_is_rejected_before_saving() {
    let mut ui = Runtime::new();
    let value = ui.signal(1_i64);
    let format = NumberFormat::new(
        "999",
        |v: i64| {
            if v == 2 {
                "invalid\ntext".into()
            } else {
                v.to_string()
            }
        },
        |s| {
            s.parse::<i64>()
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    );
    let id = mount(&mut ui, InlineInteger::new(value.clone()).format(format));
    click(&mut ui, &id);
    ui.text_input("2").unwrap();
    assert!(ui.key_input(KeyCode::Enter, KeyAction::Press).is_err());
    assert_eq!(value.get().unwrap(), 1);
}

#[test]
fn slider_text_steps_are_drafts_and_bad_format_cannot_commit() {
    let mut ui = Runtime::new();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "value").unwrap(),
            ParameterSpec::new("Value", 0.0, 100.0, 10.0).unwrap(),
        )
        .unwrap();
    let format = NumberFormat::new(
        "999",
        |v: f32| {
            if v == 12.0 {
                "invalid\ntext".into()
            } else {
                v.to_string()
            }
        },
        |s| {
            s.parse::<f32>()
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    )
    .edit_text(|v| v.to_string());
    let id = mount(&mut ui, InlineSlider::new(parameter.clone()).format(format));
    click(&mut ui, &id);
    key(&mut ui, KeyCode::ArrowUp);
    assert_eq!(parameter.snapshot().get().unwrap().base, 10.0);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(parameter.snapshot().get().unwrap().base, 11.0);
    click(&mut ui, &id);
    ui.text_input("12").unwrap();
    assert!(ui.key_input(KeyCode::Enter, KeyAction::Press).is_err());
    assert_eq!(parameter.snapshot().get().unwrap().base, 11.0);
}

#[test]
fn dynamic_unit_overrides_literal_suffix_and_uses_explicit_editing_units() {
    let mut ui = Runtime::new();
    let value = ui.signal(0.25_f64);
    let format = NumberFormat::new(
        "9999 ms",
        |v: f64| {
            if v < 1.0 {
                format!("{:.0}", v * 1000.0)
            } else {
                format!("{v:.2}")
            }
        },
        |s| {
            s.parse::<f64>()
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    )
    .unit(|v| if v < 1.0 { "ms".into() } else { "s".into() })
    .edit_text(|v| v.to_string())
    .edit_unit("s");
    let id = mount(
        &mut ui,
        InlineDecimal::new(value.clone())
            .unit("ignored")
            .format(format),
    );
    assert_eq!(property(&ui, "unit"), InspectValue::Text("ms".into()));
    click(&mut ui, &id);
    assert_eq!(property(&ui, "unit"), InspectValue::Text("s".into()));
    ui.text_input("2.5").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 2.5);
    assert_eq!(property(&ui, "unit"), InspectValue::Text("s".into()));
    assert_eq!(
        property(&ui, "display_value"),
        InspectValue::Text("2.50".into())
    );
}

#[test]
fn inline_height_and_secondary_caption_follow_theme_caption_metrics() {
    let mut ui = Runtime::new();
    let value = ui.signal(8_i64);
    let id = mount(&mut ui, InlineInteger::new(value).label("Vol").unit("%"));
    assert_eq!(
        property(&ui, "typography"),
        InspectValue::Text("Monospace".into())
    );
    let before = ui.bounds(&id).unwrap().unwrap().values();
    let pixels_before = ui
        .paint()
        .unwrap()
        .masks()
        .iter()
        .map(|m| m.height())
        .max()
        .unwrap();
    let metrics = ui
        .theme()
        .metrics(Density::Standard)
        .unwrap()
        .with_metric(MetricRole::CaptionSize, Length::new(24.0).unwrap())
        .unwrap();
    ui.theme().set_metrics(Density::Standard, metrics).unwrap();
    frame(&mut ui);
    assert!(ui.bounds(&id).unwrap().unwrap().height() > before[3]);
    assert!(
        ui.paint()
            .unwrap()
            .masks()
            .iter()
            .map(|m| m.height())
            .max()
            .unwrap()
            > pixels_before
    );
    click(&mut ui, &id);
    let (_, state) = ui.text_input_state().unwrap();
    assert!(state.caret.height() > 12.0);
}

#[test]
fn inline_slider_pointer_endpoints_match_the_painted_track() {
    let mut ui = Runtime::new();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "value").unwrap(),
            ParameterSpec::new("Value", 0.0, 100.0, 50.0).unwrap(),
        )
        .unwrap();
    mount(&mut ui, InlineSlider::new(parameter.clone()).label("Value"));
    let scene = ui.paint().unwrap();
    let outline = ui
        .theme()
        .palette(ThemeMode::Dark)
        .unwrap()
        .color(ColorRole::Outline);
    let [x, y, w, h] = scene
        .commands()
        .iter()
        .find_map(|cmd| match cmd {
            DrawCommand::FillRect { rect, color }
                if *color == outline && rect.height() == 2.0 && rect.width() > 2.0 =>
            {
                Some(rect.values())
            }
            _ => None,
        })
        .unwrap();
    // Rounded track caps add one logical pixel at either end of this central fill.
    for (x, value) in [(x - 1.0, 0.0), (x + w + 1.0, 100.0)] {
        let p = Point::new(x, y + h / 2.0).unwrap();
        pointer(&mut ui, p, PointerKind::Press(PointerButton::Primary));
        pointer(&mut ui, p, PointerKind::Release(PointerButton::Primary));
        assert_eq!(parameter.snapshot().get().unwrap().base, value);
    }
}

/// While a value is typed, the three inline numeric rows route keys to the draft
/// before application key commands, and text commands act on the draft.
#[test]
fn inline_rows_in_text_mode_route_plain_keys_and_text_commands_to_the_draft() {
    #[derive(Clone, Default)]
    struct Memory(std::rc::Rc<std::cell::RefCell<String>>);
    impl Clipboard for Memory {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            Ok(self.0.borrow().clone())
        }
        fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
            *self.0.borrow_mut() = text.to_owned();
            Ok(())
        }
    }
    fn type_five(ui: &mut Runtime) -> KeyDispatch {
        let mut input = CommandKeyInput::new(
            KeyCode::Digit5,
            KeyAction::Press,
            CommandKey::Character('5'),
        );
        input.has_text = true;
        let dispatch = ui.command_key_input(input).unwrap();
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::Digit5,
            KeyAction::Release,
            CommandKey::Character('5'),
        ))
        .unwrap();
        dispatch
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 200.0, 1.0).unwrap())
        .unwrap();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 100.0, 10.0).unwrap(),
        )
        .unwrap();
    let integer = ui.signal(1_i64);
    let decimal = ui.signal(1.0_f64);
    let hits = std::rc::Rc::new(std::cell::Cell::new(0_u32));
    let counter = hits.clone();
    let _command = ui
        .register_command(
            "Five",
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Character('5'),
                Modifiers::default(),
            )],
            move |_, _| {
                counter.set(counter.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    let clipboard = Memory::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    let controls = [
        (
            "slider",
            Element::new("field", InlineSlider::new(parameter)),
        ),
        (
            "integer",
            Element::new("field", InlineInteger::new(integer)),
        ),
        (
            "decimal",
            Element::new("field", InlineDecimal::new(decimal)),
        ),
    ];
    for (name, element) in controls {
        let root = ui
            .mount_tree(Element::new(name, Column::new([element])))
            .unwrap();
        frame(&mut ui);
        let id = ui.children(&root).unwrap()[0].clone();
        ui.focus(&id).unwrap();
        frame(&mut ui);
        // At rest the row is no text editor: the key command runs.
        let before = hits.get();
        type_five(&mut ui);
        assert_eq!(hits.get(), before + 1, "{name} at rest");
        assert_eq!(
            ui.text_command(&id, TextCommand::Copy).unwrap(),
            TextCommandOutcome::NotAnEditor,
            "{name} at rest"
        );
        key(&mut ui, KeyCode::F2);
        frame(&mut ui);
        let dispatch = type_five(&mut ui);
        assert!(dispatch.deliver_text, "{name} claims the typed text");
        assert_eq!(hits.get(), before + 1, "{name} in text mode");
        // The selected value copies like the chord would.
        clipboard.0.borrow_mut().clear();
        assert_eq!(
            ui.text_command(&id, TextCommand::Copy).unwrap(),
            TextCommandOutcome::Handled,
            "{name} copies"
        );
        assert!(!clipboard.0.borrow().is_empty(), "{name} copies");
        key(&mut ui, KeyCode::Escape);
        ui.unmount(&root).unwrap();
        frame(&mut ui);
    }
}
