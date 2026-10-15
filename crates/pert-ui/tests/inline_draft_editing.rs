//! Inline rows editing through their child `DraftEditor`: the native key path,
//! presses on the row's chrome, the wheel during a composition, outcomes and
//! re-description.
use pert_ui::*;
mod support;
use std::{cell::Cell, rc::Rc};
use support::{find, frame};

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;

fn native(ui: &mut Runtime, code: KeyCode) -> Result<(), RuntimeError> {
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))?;
    }
    ui.flush(ui.now())?;
    Ok(())
}

fn property(ui: &Runtime, name: &str) -> InspectValue {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == "field")
        .map(|c| c.details.properties[name].clone())
        .expect("the row is mounted")
}

/// Mounts `control` as "field" and a capture-scope command on the keys a
/// row's editor leaves to it; returns the row and the command's hit count.
fn mount(ui: &mut Runtime, control: Element) -> Result<(WidgetId, WidgetId, Rc<Cell<u32>>), Error> {
    ui.set_viewport(Viewport::new(600.0, 200.0, 1.0)?)?;
    let hits = Rc::new(Cell::new(0));
    let counter = hits.clone();
    let chords = [
        KeyCode::Enter,
        KeyCode::Escape,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
    ]
    .map(|code| KeyChord::new(CommandKey::Named(code), Modifiers::default()))
    .to_vec();
    let command =
        ui.register_command("Transport", CommandScope::Capture, chords, move |_, _| {
            counter.set(counter.get() + 1);
            Ok(())
        })?;
    std::mem::forget(command);
    let root = ui.mount_tree(Element::new("root", Column::new([control])))?;
    frame(ui);
    let row = find(ui, &root, "field").ok_or("no row")?;
    ui.focus(&row)?;
    frame(ui);
    Ok((root, row, hits))
}

#[test]
fn a_typing_row_answers_its_keys_before_capture_commands() -> TestResult {
    // An integer row: Enter saves, and neither Enter nor the arrows run the command.
    let mut ui = Runtime::new();
    let value = ui.signal(8_i64);
    let (_, _, hits) = mount(
        &mut ui,
        Element::new("field", InlineInteger::new(value.clone()).range(0, 100)?),
    )?;
    native(&mut ui, KeyCode::F2)?;
    ui.text_input("42")?;
    native(&mut ui, KeyCode::ArrowUp)?;
    native(&mut ui, KeyCode::Enter)?;
    assert_eq!(value.get()?, 43);
    assert_eq!(hits.get(), 0, "the row took Enter and Up");

    // A label: Escape restores, and does not run the command.
    let mut ui = Runtime::new();
    let name = ui.signal("Lead".to_owned());
    let (_, _, hits) = mount(
        &mut ui,
        Element::new("field", InlineText::new(name.clone())),
    )?;
    native(&mut ui, KeyCode::F2)?;
    ui.text_input("Bass")?;
    native(&mut ui, KeyCode::Escape)?;
    assert_eq!(name.get()?, "Lead");
    assert_eq!(property(&ui, "editing"), InspectValue::Bool(false));
    native(&mut ui, KeyCode::F2)?;
    ui.text_input("Bass")?;
    native(&mut ui, KeyCode::Enter)?;
    assert_eq!(name.get()?, "Bass");
    assert_eq!(hits.get(), 0, "the label took Enter and Escape");

    // A readout: Escape leaves text mode without running the command.
    let mut ui = Runtime::new();
    let text = ui.signal("001:01:000".to_owned());
    let (_, _, hits) = mount(
        &mut ui,
        Element::new("field", InlineReadout::new(text.read_only())),
    )?;
    native(&mut ui, KeyCode::F2)?;
    assert_eq!(property(&ui, "text_editing"), InspectValue::Bool(true));
    native(&mut ui, KeyCode::Escape)?;
    assert_eq!(property(&ui, "text_editing"), InspectValue::Bool(false));
    assert_eq!(hits.get(), 0, "the readout took Escape");
    Ok(())
}

#[test]
fn a_press_on_a_typing_rows_caption_keeps_its_text_mode() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 60.0, 1.0)?)?;
    let value = ui.signal(8_i64);
    let rows: Vec<Element> = std::iter::once(Element::new(
        "field",
        InlineInteger::new(value.clone())
            .range(0, 100)?
            .label("Voices"),
    ))
    .chain((0..8).map(|index| Element::new(format!("filler{index}"), Label::new("filler"))))
    .collect();
    let root = ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new("rows", Column::new(rows))),
    ))?;
    frame(&mut ui);
    let row = find(&ui, &root, "field").ok_or("no row")?;
    ui.focus(&row)?;
    ui.key_input(KeyCode::F2, KeyAction::Press)?;
    ui.text_input("42")?;
    frame(&mut ui);
    let [x, y, _, h] = ui.bounds(&row)?.ok_or("unplaced")?.values();
    let caption = Point::new(x + 6.0, y + h / 2.0)?;
    ui.pointer_input(PointerEvent::new(
        caption,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        caption,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    frame(&mut ui);
    assert_eq!(property(&ui, "text_editing"), InspectValue::Bool(true));
    assert_eq!(value.get()?, 8, "nothing was saved");
    assert_eq!(
        ui.focused(),
        find(&ui, &row, "editor"),
        "focus stayed in the row"
    );
    Ok(())
}

#[test]
fn the_wheel_does_not_step_through_a_composition() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 200.0, 1.0)?)?;
    let value = ui.signal(8_i64);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "field",
            InlineInteger::new(value.clone()).range(0, 100)?,
        )]),
    ))?;
    frame(&mut ui);
    let row = find(&ui, &root, "field").ok_or("no row")?;
    ui.focus(&row)?;
    ui.key_input(KeyCode::F2, KeyAction::Press)?;
    frame(&mut ui);
    ui.text_preedit(&TextPreedit::new("5", None)?)?;
    let editor = find(&ui, &row, "editor").ok_or("no editor")?;
    let [x, y, w, h] = ui.bounds(&editor)?.ok_or("unplaced")?.values();
    let over = Point::new(x + w / 2.0, y + h / 2.0)?;
    ui.pointer_input(PointerEvent::new(
        over,
        PointerKind::Scroll(Point::new(0.0, -40.0)?),
    ))?;
    frame(&mut ui);
    let composing = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "editor")
        .map(|c| c.details.properties["composing"].clone());
    assert_eq!(
        composing,
        Some(InspectValue::Bool(true)),
        "the composition survived"
    );
    assert_eq!(
        property(&ui, "text"),
        InspectValue::Text("8".into()),
        "no step"
    );
    Ok(())
}

#[test]
fn a_slider_records_cancelled_when_text_mode_ends_unsaved() -> TestResult {
    for end in ["escape", "focus loss"] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(600.0, 200.0, 1.0)?)?;
        let (parameter, _host) = ui.parameter(
            ParameterId::new(1, 1, "amount")?,
            ParameterSpec::new("Amount", 0.0, 100.0, 10.0)?,
        )?;
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("field", InlineSlider::new(parameter.clone())),
                Element::new("next", Button::new("Next", |_| Ok(()))),
            ]),
        ))?;
        frame(&mut ui);
        let row = find(&ui, &root, "field").ok_or("no row")?;
        ui.focus(&row)?;
        ui.key_input(KeyCode::F2, KeyAction::Press)?;
        ui.text_input("42")?;
        frame(&mut ui);
        if end == "escape" {
            ui.key_input(KeyCode::Escape, KeyAction::Press)?;
        } else {
            ui.clear_focus();
        }
        frame(&mut ui);
        assert_eq!(
            property(&ui, "edit_outcome"),
            InspectValue::Text("Cancelled".into()),
            "{end}"
        );
        assert_eq!(parameter.snapshot().get()?.base, 10.0, "{end}");
    }
    Ok(())
}

#[test]
fn describing_a_typing_numeric_row_again_keeps_its_draft() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 200.0, 1.0)?)?;
    let value = ui.signal(8_i64);
    let row = || {
        InlineInteger::new(value.clone())
            .range(0, 100)
            .map(|row| row.label("Voices"))
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("field", row()?)]),
    ))?;
    frame(&mut ui);
    let field = find(&ui, &root, "field").ok_or("no row")?;
    ui.focus(&field)?;
    ui.key_input(KeyCode::F2, KeyAction::Press)?;
    ui.text_input("42")?;
    frame(&mut ui);
    let editor = find(&ui, &field, "editor");
    ui.reconcile_children(&root, vec![Element::new("field", row()?)])?;
    frame(&mut ui);
    assert_eq!(
        find(&ui, &field, "editor"),
        editor,
        "the editor stayed mounted"
    );
    assert_eq!(property(&ui, "text"), InspectValue::Text("42".into()));
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    frame(&mut ui);
    assert_eq!(value.get()?, 42);
    Ok(())
}

#[test]
fn a_press_on_other_content_still_moves_focus_as_before() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 60.0, 1.0)?)?;
    let text = ui.signal("draft".to_owned());
    let rows: Vec<Element> = [
        Element::new("text", TextField::new(text)),
        Element::new("label", Label::new("Label")),
    ]
    .into_iter()
    .chain((0..8).map(|index| Element::new(format!("filler{index}"), Label::new("filler"))))
    .collect();
    let root = ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new("rows", Column::new(rows))),
    ))?;
    frame(&mut ui);
    let field = find(&ui, &root, "text").ok_or("no field")?;
    let label = find(&ui, &root, "label").ok_or("no label")?;
    ui.focus(&field)?;
    frame(&mut ui);
    let [x, y, w, h] = ui.bounds(&label)?.ok_or("unplaced")?.values();
    let point = Point::new(x + w / 2.0, y + h / 2.0)?;
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    frame(&mut ui);
    assert_eq!(
        ui.focused(),
        Some(root),
        "the scroll area takes focus, as before"
    );
    Ok(())
}

#[test]
fn a_typing_slider_answers_its_keys_and_ignores_the_wheel_while_composing() -> TestResult {
    let mut ui = Runtime::new();
    let (parameter, _host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 100.0, 10.0)?,
    )?;
    let (_, row, hits) = mount(
        &mut ui,
        Element::new("field", InlineSlider::new(parameter.clone())),
    )?;
    native(&mut ui, KeyCode::F2)?;
    ui.text_preedit(&TextPreedit::new("5", None)?)?;
    let editor = find(&ui, &row, "editor").ok_or("no editor")?;
    let [x, y, w, h] = ui.bounds(&editor)?.ok_or("unplaced")?.values();
    let over = Point::new(x + w / 2.0, y + h / 2.0)?;
    ui.pointer_input(PointerEvent::new(
        over,
        PointerKind::Scroll(Point::new(0.0, -40.0)?),
    ))?;
    frame(&mut ui);
    let composing = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "editor")
        .map(|c| c.details.properties["composing"].clone());
    assert_eq!(
        composing,
        Some(InspectValue::Bool(true)),
        "the composition survived"
    );
    ui.text_preedit(&TextPreedit::new("", None)?)?;
    ui.text_input("42")?;
    native(&mut ui, KeyCode::ArrowUp)?;
    native(&mut ui, KeyCode::Escape)?;
    assert_eq!(parameter.snapshot().get()?.base, 10.0, "Escape cancelled");
    native(&mut ui, KeyCode::F2)?;
    ui.text_input("42")?;
    native(&mut ui, KeyCode::Enter)?;
    assert_eq!(parameter.snapshot().get()?.base, 42.0);
    assert_eq!(hits.get(), 0, "the slider took Enter, Escape and Up");
    Ok(())
}
