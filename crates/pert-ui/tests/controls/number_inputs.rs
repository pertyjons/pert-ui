//! Numeric editing contracts through the public facade.
use crate::support;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use support::{frame, key};

fn command(ui: &mut Runtime, code: KeyCode) {
    let modifier = KeyCode::primary();
    ui.key_input(modifier, KeyAction::Press).unwrap();
    key(ui, code);
    ui.key_input(modifier, KeyAction::Release).unwrap();
}
fn replace(ui: &mut Runtime, text: &str) {
    command(ui, KeyCode::KeyA);
    key(ui, KeyCode::Backspace);
    ui.text_input(text).unwrap();
    frame(ui);
}
fn property(ui: &Runtime, name: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "field")
        .unwrap()
        .details
        .properties[name]
        .clone()
}
fn mount(ui: &mut Runtime, field: impl Component + 'static) -> WidgetId {
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let id = ui.mount_tree(Element::new("field", field)).unwrap();
    ui.focus(&id).unwrap();
    frame(ui);
    id
}

#[test]
fn integer_saves_on_enter_tab_pointer_focus_and_window_focus_loss() {
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("field", IntegerInput::new(value.clone())),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    let children = ui.children(&root).unwrap();
    let field = children[0].clone();
    let other = children[1].clone();
    ui.focus(&field).unwrap();
    frame(&mut ui);
    replace(&mut ui, "42");
    assert_eq!(value.get().unwrap(), 7);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 42);
    replace(&mut ui, "43");
    ui.focus(&other).unwrap();
    assert_eq!(value.get().unwrap(), 43);
    ui.focus(&field).unwrap();
    replace(&mut ui, "44");
    key(&mut ui, KeyCode::Tab);
    assert_eq!(value.get().unwrap(), 44);
    assert_eq!(ui.focused(), Some(other));
    ui.focus(&field).unwrap();
    replace(&mut ui, "45");
    ui.clear_focus();
    assert_eq!(value.get().unwrap(), 45);
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn escape_hide_and_removal_discard_drafts_and_release_subscriptions() {
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    let field = mount(&mut ui, IntegerInput::new(value.clone()));
    replace(&mut ui, "42");
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), InspectValue::Text("7".into()));
    assert_eq!(value.get().unwrap(), 7);
    replace(&mut ui, "99");
    ui.set_visible(&field, false).unwrap();
    assert_eq!(value.get().unwrap(), 7);
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
    ui.set_visible(&field, true).unwrap();
    ui.focus(&field).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), InspectValue::Text("7".into()));
    replace(&mut ui, "88");
    ui.unmount(&field).unwrap();
    assert_eq!(value.get().unwrap(), 7);
    value.set(8).unwrap();
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
}

#[test]
fn invalid_drafts_remain_visible_after_enter_and_blur_without_writes() {
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    let field = mount(
        &mut ui,
        IntegerInput::new(value.clone()).range(-10, 10).unwrap(),
    );
    for text in [
        "",
        "-",
        "+",
        "12",
        "1.5",
        "1e1",
        "1,000",
        "9223372036854775808",
        "abc",
    ] {
        ui.focus(&field).unwrap();
        replace(&mut ui, text);
        assert_eq!(property(&ui, "valid"), InspectValue::Bool(false));
        key(&mut ui, KeyCode::Enter);
        ui.clear_focus();
        frame(&mut ui);
        assert_eq!(value.get().unwrap(), 7);
        assert_eq!(property(&ui, "text"), InspectValue::Text(text.into()));
    }
    ui.focus(&field).unwrap();
    replace(&mut ui, " -8 ");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), -8);
}

#[test]
fn all_integer_bits_survive_editing_and_steps_saturate_without_overflow() {
    let mut ui = Runtime::new();
    let value = ui.signal(0_i64);
    mount(
        &mut ui,
        IntegerInput::new(value.clone()).step(i64::MAX).unwrap(),
    );
    for number in [9_007_199_254_740_993, i64::MIN, i64::MAX] {
        replace(&mut ui, &number.to_string());
        key(&mut ui, KeyCode::Enter);
        assert_eq!(value.get().unwrap(), number);
    }
    key(&mut ui, KeyCode::ArrowUp);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), i64::MAX);
    value.set(i64::MIN).unwrap();
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), i64::MIN);
}

#[test]
fn decimal_separators_exponents_and_finite_validation() {
    let mut ui = Runtime::new();
    let value = ui.signal(0.0);
    mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .range(-100.0, 100.0)
            .unwrap(),
    );
    for (text, expected) in [
        ("1,25", 1.25),
        ("-2.5", -2.5),
        (" +1.5e1 ", 15.0),
        (".5", 0.5),
    ] {
        replace(&mut ui, text);
        key(&mut ui, KeyCode::Enter);
        assert_eq!(value.get().unwrap(), expected);
    }
    for text in [
        "NaN", "inf", "-inf", "1e309", "1,234.5", "1.2.3", "101", "1e", "-",
    ] {
        replace(&mut ui, text);
        key(&mut ui, KeyCode::Enter);
        assert_eq!(value.get().unwrap(), 0.5);
        assert_eq!(property(&ui, "valid"), InspectValue::Bool(false));
    }
}

#[test]
fn precision_is_only_presentation_and_steps_edit_the_full_value() {
    let mut ui = Runtime::new();
    let value = ui.signal(1.234567);
    let field = mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .decimal_places(2)
            .unwrap()
            .step(0.000001)
            .unwrap(),
    );
    assert_eq!(property(&ui, "text"), InspectValue::Text("1.23".into()));
    key(&mut ui, KeyCode::Enter);
    ui.clear_focus();
    assert_eq!(value.get().unwrap(), 1.234567);
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::ArrowUp);
    assert_eq!(value.get().unwrap(), 1.234567);
    ui.clear_focus();
    assert!((value.get().unwrap() - 1.234568).abs() < 1e-12);
    // A rounded display outside a narrow allowed range is still a valid resting value.
    let mut ui = Runtime::new();
    let value = ui.signal(0.125);
    mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .range(0.125, 0.126)
            .unwrap()
            .decimal_places(1)
            .unwrap(),
    );
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 0.125);
    assert_eq!(property(&ui, "valid"), InspectValue::Bool(true));
}

#[test]
fn repeated_decimal_steps_stay_exact_decimals() {
    // Binary addition of 0.1 accumulates error: 56 steps down from zero gave
    // -5.600000000000001, which a field without a precision showed as typed.
    let mut ui = Runtime::new();
    let value = ui.signal(0.0);
    mount(
        &mut ui,
        DecimalInput::new(value.clone()).range(-24.0, 24.0).unwrap(),
    );
    for _ in 0..56 {
        key(&mut ui, KeyCode::ArrowDown);
    }
    assert_eq!(property(&ui, "text"), InspectValue::Text("-5.6".into()));
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), -5.6);
    // Steps keep a value's own digits, and stepping back to zero is not -0.
    let mut ui = Runtime::new();
    let value = ui.signal(2.3456789);
    mount(
        &mut ui,
        DecimalInput::new(value.clone()).decimal_places(2).unwrap(),
    );
    for _ in 0..22 {
        key(&mut ui, KeyCode::ArrowDown);
    }
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 0.1456789);
    let mut ui = Runtime::new();
    let value = ui.signal(-0.3);
    mount(&mut ui, DecimalInput::new(value.clone()));
    for _ in 0..3 {
        key(&mut ui, KeyCode::ArrowUp);
    }
    key(&mut ui, KeyCode::Enter);
    assert!(value.get().unwrap() == 0.0 && value.get().unwrap().is_sign_positive());
}

#[test]
fn external_changes_win_even_before_a_flush_or_focus_loss() {
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    let field = mount(&mut ui, IntegerInput::new(value.clone()));
    replace(&mut ui, "42");
    value.set(99).unwrap();
    ui.clear_focus();
    assert_eq!(value.get().unwrap(), 99);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), InspectValue::Text("99".into()));
    ui.focus(&field).unwrap();
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(property(&ui, "text"), InspectValue::Text("99".into()));
    replace(&mut ui, "5");
    value.set(123).unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 123);
}

#[test]
fn ime_enter_does_not_save_preedit_and_escape_cancels_composition_before_draft() {
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    mount(&mut ui, IntegerInput::new(value.clone()));
    replace(&mut ui, "42");
    ui.text_preedit(&TextPreedit::new("3", None).unwrap())
        .unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 7);
    frame(&mut ui);
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(true));
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), InspectValue::Text("42".into()));
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(false));
    key(&mut ui, KeyCode::Escape);
    assert_eq!(property(&ui, "text"), InspectValue::Text("7".into()));
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(false));
    replace(&mut ui, "12");
    ui.text_preedit(&TextPreedit::new("3", None).unwrap())
        .unwrap();
    ui.clear_focus();
    assert_eq!(value.get().unwrap(), 12); // uncommitted composition is excluded
}

#[test]
fn native_numeric_escape_preserves_the_draft_until_composition_is_cancelled() {
    for decimal in [false, true] {
        let mut ui = Runtime::new();
        let integer = ui.signal(7_i64);
        let fraction = ui.signal(7.0_f64);
        if decimal {
            mount(&mut ui, DecimalInput::new(fraction.clone()));
        } else {
            mount(&mut ui, IntegerInput::new(integer.clone()));
        }
        replace(&mut ui, "42");
        ui.text_preedit(&TextPreedit::new("3", None).unwrap())
            .unwrap();
        let native = |ui: &mut Runtime, code| {
            for action in [KeyAction::Press, KeyAction::Release] {
                ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
                    .unwrap();
            }
        };
        native(&mut ui, KeyCode::Enter);
        assert_eq!(integer.get().unwrap(), 7);
        assert_eq!(fraction.get().unwrap(), 7.0);
        native(&mut ui, KeyCode::Escape);
        assert_eq!(property(&ui, "text"), InspectValue::Text("42".into()));
        assert_eq!(property(&ui, "composing"), InspectValue::Bool(false));
        native(&mut ui, KeyCode::Escape);
        assert_eq!(property(&ui, "text"), InspectValue::Text("7".into()));
    }
}

#[test]
fn readonly_disabled_foreign_and_invalid_external_values() {
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    let id = mount(&mut ui, IntegerInput::new(value.clone()).read_only());
    ui.text_input("42").unwrap();
    key(&mut ui, KeyCode::ArrowUp);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 7);
    assert!(ui.text_input_state().is_none());
    ui.unmount(&id).unwrap();
    let id = mount(&mut ui, IntegerInput::new(value.clone()).disabled());
    ui.text_input("42").unwrap();
    assert_eq!(value.get().unwrap(), 7);
    ui.unmount(&id).unwrap();
    let foreign = Runtime::new();
    ui.mount_tree(Element::new(
        "foreign",
        IntegerInput::new(foreign.signal(0)),
    ))
    .unwrap();
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::ForeignRuntime));
    let mut ui = Runtime::new();
    let value = ui.signal(0.0);
    mount(&mut ui, DecimalInput::new(value.clone()));
    value.set(f64::NAN).unwrap();
    // The field refuses the value; the runtime keeps working.
    frame(&mut ui);
    assert!(value.get().unwrap().is_nan());
    assert_eq!(property(&ui, "text"), InspectValue::Text("0".into()));
    assert_eq!(
        property(&ui, "binding_error"),
        InspectValue::Text(NumberInputError::NonFinite.to_string())
    );
    value.set(2.0).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), InspectValue::Text("2".into()));
}

fn has_property(ui: &Runtime, name: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "field")
        .is_some_and(|c| c.details.properties.contains_key(name))
}

#[test]
fn a_refused_external_value_keeps_the_field_and_the_runtime_working() {
    for inline in [false, true] {
        let mut ui = Runtime::new();
        let value = ui.signal(5_i64);
        let id = if inline {
            mount(
                &mut ui,
                InlineInteger::new(value.clone()).range(0, 10).unwrap(),
            )
        } else {
            mount(
                &mut ui,
                IntegerInput::new(value.clone()).range(0, 10).unwrap(),
            )
        };
        if inline {
            key(&mut ui, KeyCode::F2);
        }
        replace(&mut ui, "7");
        // An out-of-range value written by the application while a draft is open.
        value.set(42).unwrap();
        frame(&mut ui);
        frame(&mut ui);
        assert!(matches!(
            property(&ui, "binding_error"),
            InspectValue::Text(_)
        ));
        assert_eq!(property(&ui, "value"), InspectValue::Text("5".into()));
        let semantics = ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "field")
            .and_then(|c| c.details.semantics)
            .unwrap();
        assert!(
            semantics.read_only,
            "inline: {inline}: a refused field is read-only"
        );
        assert!(!semantics.actions.contains(&SemanticAction::SetValue));
        // The draft is gone and nothing can overwrite the application's value:
        // typing, stepping, saving and focus loss all leave it alone.
        ui.text_input("3").unwrap();
        key(&mut ui, KeyCode::ArrowUp);
        key(&mut ui, KeyCode::Enter);
        ui.pointer_input(PointerEvent::new(
            Point::new(20.0, 10.0).unwrap(),
            PointerKind::Scroll(Point::new(0.0, -40.0).unwrap()),
        ))
        .unwrap();
        ui.clear_focus();
        frame(&mut ui);
        assert_eq!(value.get().unwrap(), 42, "inline: {inline}");
        // Paint shows the refusal with the error outline.
        let theme = ui.theme();
        let error = theme
            .palette(theme.mode().unwrap())
            .unwrap()
            .color(ColorRole::Error);
        ui.flush(Duration::ZERO).unwrap();
        assert!(
            ui.paint()
                .unwrap()
                .commands()
                .iter()
                .any(|command| matches!(
                    command,
                    DrawCommand::FillRect { color, .. } | DrawCommand::DrawMask { color, .. }
                        if *color == error
                ))
        );
        // The next acceptable value ends the refusal and editing resumes.
        value.set(8).unwrap();
        frame(&mut ui);
        assert!(!has_property(&ui, "binding_error"));
        assert_eq!(property(&ui, "value"), InspectValue::Text("8".into()));
        ui.focus(&id).unwrap();
        if inline {
            key(&mut ui, KeyCode::F2);
        }
        replace(&mut ui, "9");
        key(&mut ui, KeyCode::Enter);
        assert_eq!(value.get().unwrap(), 9, "inline: {inline}");
    }
    // A value refused from the first update leaves the field empty and locked.
    let mut ui = Runtime::new();
    let value = ui.signal(f64::INFINITY);
    mount(&mut ui, InlineDecimal::new(value.clone()));
    key(&mut ui, KeyCode::F2);
    ui.text_input("1").unwrap();
    ui.clear_focus();
    frame(&mut ui);
    assert!(has_property(&ui, "binding_error"));
    assert_eq!(value.get().unwrap(), f64::INFINITY);
}

#[test]
fn builders_reject_bad_bounds_steps_and_precision() {
    let ui = Runtime::new();
    assert!(IntegerInput::new(ui.signal(0)).range(2, 1).is_err());
    assert!(IntegerInput::new(ui.signal(0)).step(0).is_err());
    assert!(IntegerInput::new(ui.signal(0)).step(-1).is_err());
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(DecimalInput::new(ui.signal(0.0)).range(value, 1.0).is_err());
        assert!(DecimalInput::new(ui.signal(0.0)).range(0.0, value).is_err());
        assert!(DecimalInput::new(ui.signal(0.0)).step(value).is_err());
    }
    let max = NumberInputError::MAX_PRECISION;
    assert!(
        DecimalInput::new(ui.signal(0.0))
            .decimal_places(max)
            .is_ok()
    );
    assert!(
        InlineDecimal::new(ui.signal(0.0))
            .decimal_places(max)
            .is_ok()
    );
    assert_eq!(
        DecimalInput::new(ui.signal(0.0))
            .decimal_places(max + 1)
            .err(),
        Some(NumberInputError::InvalidPrecision)
    );
    assert_eq!(
        InlineDecimal::new(ui.signal(0.0))
            .decimal_places(max + 1)
            .err(),
        Some(NumberInputError::InvalidPrecision)
    );
    // The former name keeps its rules.
    #[allow(deprecated)]
    let renamed = DecimalInput::new(ui.signal(0.0)).precision(max + 1).err();
    assert_eq!(renamed, Some(NumberInputError::InvalidPrecision));
    // Sliders take the same setting under the same name and limit.
    let mut ui = ui;
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 9, "limit").unwrap(),
            ParameterSpec::new("Limit", 0.0, 1.0, 0.5).unwrap(),
        )
        .unwrap();
    assert!(Slider::new(parameter.clone()).decimal_places(max).is_ok());
    assert_eq!(
        Slider::new(parameter).decimal_places(max + 1).err(),
        Some(NumberInputError::InvalidPrecision)
    );
}

struct FailingFocus(Rc<Cell<usize>>);
impl Component for FailingFocus {
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 30.0)?)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if cx.focused() {
            cx.wake_after(Duration::from_secs(1))?;
        }
        Ok(())
    }
    fn focus_lost(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.0.set(self.0.get() + 1);
        Err(RuntimeError::Exhausted)
    }
}
#[test]
fn public_focus_hook_errors_do_not_block_cleanup_and_window_errors_are_delivered() {
    let mut ui = Runtime::new();
    let calls = Rc::new(Cell::new(0));
    let first = ui.mount("first", FailingFocus(calls.clone())).unwrap();
    let second = ui
        .mount("second", Button::new("Other", |_| Ok(())))
        .unwrap();
    ui.focus(&first).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "first")
            .unwrap()
            .next_deadline
            .is_some()
    );
    assert_eq!(ui.focus(&second), Err(RuntimeError::Exhausted));
    assert!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "first")
            .unwrap()
            .next_deadline
            .is_none()
    );
    assert_eq!(ui.focused(), Some(second));
    for _ in 0..2 {
        ui.focus(&first).unwrap();
        ui.clear_focus();
    }
    assert!(ui.focused().is_none());
    assert_eq!(calls.get(), 3);
    assert!(ui.needs_paint());
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::Exhausted));
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::Exhausted));
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&first).unwrap();
    ui.set_visible(&first, false).unwrap();
    ui.unmount(&first).unwrap();
    assert_eq!(calls.get(), 3); // cancellation is not focus-save
}

#[test]
fn focus_error_still_delivers_the_click_and_preserves_new_capture() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let clicked = ui.signal(0);
    let clicks = clicked.clone();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("first", FailingFocus(Rc::new(Cell::new(0)))),
                Element::new("second", Button::new("Click", move |_| clicks.set(1))),
            ]),
        ))
        .unwrap();
    let children = ui.children(&root).unwrap();
    ui.focus(&children[0]).unwrap();
    frame(&mut ui);
    let [x, y, w, h] = ui.bounds(&children[1]).unwrap().unwrap().values();
    let position = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    assert_eq!(
        ui.pointer_input(PointerEvent::new(
            position,
            PointerKind::Press(PointerButton::Primary)
        )),
        Err(RuntimeError::Exhausted)
    );
    assert_eq!(ui.focused(), Some(children[1].clone()));
    ui.pointer_input(PointerEvent::new(
        position,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    assert_eq!(clicked.get().unwrap(), 1);
}

#[test]
fn undo_is_local_to_the_draft_and_committing_resets_its_history() {
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    mount(&mut ui, IntegerInput::new(value.clone()));
    replace(&mut ui, "42");
    ui.text_input("3").unwrap();
    command(&mut ui, KeyCode::KeyZ);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), InspectValue::Text("42".into()));
    assert_eq!(value.get().unwrap(), 7);
    key(&mut ui, KeyCode::Enter);
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), 42);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), InspectValue::Text("42".into()));
}

#[test]
fn failed_clipboard_operations_preserve_the_entire_unsaved_draft() {
    struct FailingClipboard(bool);
    impl Clipboard for FailingClipboard {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            if self.0 {
                Ok("9".repeat(16385))
            } else {
                Err(ClipboardError::Unavailable)
            }
        }
        fn write_text(&mut self, _: &str) -> Result<(), ClipboardError> {
            Err(ClipboardError::Unavailable)
        }
    }
    for draft in ["42", "-"] {
        for (code, oversized) in [
            (KeyCode::KeyC, false),
            (KeyCode::KeyX, false),
            (KeyCode::KeyV, false),
            (KeyCode::KeyV, true),
        ] {
            let mut ui = Runtime::new();
            let value = ui.signal(7_i64);
            mount(&mut ui, IntegerInput::new(value.clone()));
            replace(&mut ui, draft);
            command(&mut ui, KeyCode::KeyA);
            ui.set_clipboard(FailingClipboard(oversized)).unwrap();
            let modifier = KeyCode::primary();
            ui.key_input(modifier, KeyAction::Press).unwrap();
            // A clipboard failure is an error; an oversized paste is refused
            // without one, like typing into a full field. Neither edits the draft.
            assert_eq!(ui.key_input(code, KeyAction::Press).is_err(), !oversized);
            ui.key_input(code, KeyAction::Release).unwrap();
            ui.key_input(modifier, KeyAction::Release).unwrap();
            frame(&mut ui);
            assert_eq!(property(&ui, "text"), InspectValue::Text(draft.into()));
            assert_eq!(
                property(&ui, "selection"),
                InspectValue::Text(format!("0..{}", draft.len()))
            );
            assert_eq!(value.get().unwrap(), 7);
        }
    }
}

#[test]
fn pointer_focus_save_refreshes_shared_fields_before_caret_placement() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(7_i64);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("field", IntegerInput::new(value.clone())),
                Element::new("mirror", IntegerInput::new(value.clone())),
            ]),
        ))
        .unwrap();
    let fields = ui.children(&root).unwrap();
    ui.focus(&fields[0]).unwrap();
    replace(&mut ui, "99");
    let [x, y, w, h] = ui.bounds(&fields[1]).unwrap().unwrap().values();
    let point = Point::new(x + w - 5.0, y + h / 2.0).unwrap();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    assert_eq!(value.get().unwrap(), 99);
    ui.text_input("1").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 991);
}

fn wheel(ui: &mut Runtime, vertical: f32) -> InputOutcome {
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 20.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, vertical).unwrap()),
    ))
    .unwrap()
}
#[test]
fn focused_wheel_steps_integer_drafts_with_bounds_and_escape() {
    let mut ui = Runtime::new();
    let value = ui.signal(5_i64);
    let field = mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .range(0, 10)
            .unwrap()
            .step(3)
            .unwrap(),
    );
    assert_eq!(wheel(&mut ui, -40.0), InputOutcome::Handled);
    assert_eq!(property(&ui, "text"), InspectValue::Text("8".into()));
    assert_eq!(value.get().unwrap(), 5);
    wheel(&mut ui, -40.0);
    assert_eq!(property(&ui, "text"), InspectValue::Text("10".into()));
    wheel(&mut ui, 40.0);
    assert_eq!(property(&ui, "text"), InspectValue::Text("7".into()));
    key(&mut ui, KeyCode::Escape);
    assert_eq!(property(&ui, "text"), InspectValue::Text("5".into()));
    ui.clear_focus();
    assert_eq!(wheel(&mut ui, -40.0), InputOutcome::Ignored);
    assert_eq!(value.get().unwrap(), 5);
    ui.focus(&field).unwrap();
    wheel(&mut ui, -40.0);
    ui.clear_focus();
    assert_eq!(value.get().unwrap(), 8);
}
#[test]
fn stepping_obeys_display_precision_and_undo_preserves_invisible_numeric_changes() {
    let mut ui = Runtime::new();
    let value = ui.signal(1.234567_f64);
    mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .decimal_places(2)
            .unwrap()
            .step(0.000001)
            .unwrap(),
    );
    key(&mut ui, KeyCode::ArrowUp);
    wheel(&mut ui, -40.0);
    assert_eq!(property(&ui, "text"), InspectValue::Text("1.23".into()));
    assert_eq!(property(&ui, "dirty"), InspectValue::Bool(true));
    command(&mut ui, KeyCode::KeyZ);
    key(&mut ui, KeyCode::Enter);
    assert!((value.get().unwrap() - 1.234568).abs() < 1e-12);
    assert_eq!(property(&ui, "text"), InspectValue::Text("1.23".into()));

    // Free typing stays exact; stepping formats the draft without rounding its value.
    replace(&mut ui, "2.6456789");
    assert_eq!(
        property(&ui, "text"),
        InspectValue::Text("2.6456789".into())
    );
    wheel(&mut ui, -40.0);
    assert_eq!(property(&ui, "text"), InspectValue::Text("2.65".into()));
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(
        property(&ui, "text"),
        InspectValue::Text("2.6456789".into())
    );
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    command(&mut ui, KeyCode::KeyZ);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert_eq!(property(&ui, "text"), InspectValue::Text("2.65".into()));
    key(&mut ui, KeyCode::Enter);
    assert!((value.get().unwrap() - 2.6456799).abs() < 1e-12);
}
#[test]
fn wheel_respects_decimal_step_and_does_not_edit_invalid_readonly_disabled_or_composing_fields() {
    let mut ui = Runtime::new();
    let value = ui.signal(1.234567_f64);
    let field = mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .decimal_places(2)
            .unwrap()
            .step(0.25)
            .unwrap(),
    );
    wheel(&mut ui, -40.0);
    assert_eq!(property(&ui, "text"), InspectValue::Text("1.48".into()));
    wheel(&mut ui, 40.0);
    key(&mut ui, KeyCode::Enter);
    assert!((value.get().unwrap() - 1.234567).abs() < 1e-12);
    replace(&mut ui, "-");
    wheel(&mut ui, -40.0);
    assert_eq!(property(&ui, "text"), InspectValue::Text("-".into()));
    key(&mut ui, KeyCode::Escape);
    ui.text_preedit(&TextPreedit::new("2", None).unwrap())
        .unwrap();
    wheel(&mut ui, -40.0);
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(true));
    assert_eq!(property(&ui, "text"), InspectValue::Text("1.23".into()));
    ui.unmount(&field).unwrap();
    let field = mount(&mut ui, DecimalInput::new(value.clone()).read_only());
    assert_eq!(wheel(&mut ui, -40.0), InputOutcome::Ignored);
    ui.unmount(&field).unwrap();
    mount(&mut ui, DecimalInput::new(value.clone()).disabled());
    assert_eq!(wheel(&mut ui, -40.0), InputOutcome::Ignored);
    assert!((value.get().unwrap() - 1.234567).abs() < 1e-12);
}

#[test]
fn shift_fines_keyboard_and_wheel_steps_without_rounding_numeric_drafts() {
    let mut ui = Runtime::new();
    let value = ui.signal(1.234567_f64);
    mount(
        &mut ui,
        DecimalInput::new(value.clone()).decimal_places(2).unwrap(),
    );
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    wheel(&mut ui, -40.0);
    assert_eq!(property(&ui, "text"), InspectValue::Text("1.24".into()));
    key(&mut ui, KeyCode::ArrowUp);
    assert_eq!(property(&ui, "text"), InspectValue::Text("1.25".into()));
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    key(&mut ui, KeyCode::Enter);
    assert!((value.get().unwrap() - 1.254567).abs() < 1e-12);
    wheel(&mut ui, 40.0);
    key(&mut ui, KeyCode::Enter);
    assert!((value.get().unwrap() - 1.154567).abs() < 1e-12);

    for (step, fine) in [(1, 1), (3, 1), (25, 2), (100, 10)] {
        let mut ui = Runtime::new();
        let value = ui.signal(100_i64);
        mount(
            &mut ui,
            IntegerInput::new(value.clone()).step(step).unwrap(),
        );
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
        wheel(&mut ui, -1.0);
        key(&mut ui, KeyCode::ArrowUp);
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
            .unwrap();
        key(&mut ui, KeyCode::Enter);
        assert_eq!(value.get().unwrap(), 100 + fine * 2);
    }
}

#[test]
fn a_decimal_input_edits_a_continuous_parameter_at_its_resolution() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let (gain, host) = ui
        .parameter(
            ParameterId::new(1, 1, "gain").unwrap(),
            ParameterSpec::new("Gain", 0.0, 1.0, 0.1).unwrap(),
        )
        .unwrap();
    assert_eq!(
        DecimalInput::for_parameter(gain.clone())
            .range(0.0, 2.0)
            .err(),
        Some(NumberInputError::ParameterDomain),
        "the specification supplies the bounds"
    );
    let tap = |ui: &mut Runtime, code: KeyCode| {
        ui.key_input(code, KeyAction::Press).unwrap();
        ui.key_input(code, KeyAction::Release).unwrap();
    };
    let field = ui
        .mount_tree(Element::new(
            "gain",
            DecimalInput::for_parameter(gain.clone()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let text = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "gain")
            .and_then(|c| c.details.properties.get("text").cloned())
    };
    assert_eq!(
        text(&ui),
        Some(InspectValue::Text("0.1".into())),
        "the shortest decimal of the f32 is shown"
    );
    ui.focus(&field).unwrap();
    tap(&mut ui, KeyCode::End);
    tap(&mut ui, KeyCode::Backspace);
    tap(&mut ui, KeyCode::Backspace);
    tap(&mut ui, KeyCode::Backspace);
    ui.text_input("1.5").unwrap();
    tap(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        gain.snapshot().get().unwrap().base,
        0.1,
        "out of range is refused"
    );
    tap(&mut ui, KeyCode::Escape);
    host.write_base(0.25).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(text(&ui), Some(InspectValue::Text("0.25".into())));
    tap(&mut ui, KeyCode::End);
    for _ in 0..4 {
        tap(&mut ui, KeyCode::Backspace);
    }
    ui.text_input("0.75").unwrap();
    tap(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(gain.snapshot().get().unwrap().base, 0.75);
}
