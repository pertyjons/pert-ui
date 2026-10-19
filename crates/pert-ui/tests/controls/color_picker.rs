//! The color picker edits HSV locally and writes the bound linear color only
//! when an edit completes.
use pert_ui::*;
use std::time::Duration;

struct Setup {
    ui: Runtime,
    color: Signal<Color>,
    root: WidgetId,
}

fn setup(start: &str, picker: impl FnOnce(Signal<Color>) -> ColorPicker) -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(500.0, 600.0, 1.0).unwrap())
        .unwrap();
    let color = ui.signal(Color::from_hex(start).unwrap());
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([
                Element::new("picker", picker(color.clone())),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    Setup { ui, color, root }
}

fn settle(ui: &mut Runtime) {
    ui.flush(Duration::ZERO).unwrap();
}

fn find(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    ui.find_descendant(root, key).unwrap().unwrap()
}

/// A point at fractions of a part's allocation.
fn at(ui: &Runtime, part: &WidgetId, x: f32, y: f32) -> Point {
    let bounds = ui.bounds(part).unwrap().unwrap();
    Point::new(
        bounds.x() + bounds.width() * x,
        bounds.y() + bounds.height() * y,
    )
    .unwrap()
}

fn drag(ui: &mut Runtime, part: &WidgetId, x: f32, y: f32, kind: PointerKind) {
    let point = at(ui, part, x, y);
    pointer(ui, point, kind);
}

fn pointer(ui: &mut Runtime, point: Point, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
    settle(ui);
}

fn key(ui: &mut Runtime, code: KeyCode) {
    ui.key_input(code, KeyAction::Press).unwrap();
    ui.key_input(code, KeyAction::Release).unwrap();
    settle(ui);
}

fn property(ui: &Runtime, part: &str, name: &str) -> f64 {
    let item = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == part)
        .unwrap();
    match item.details.properties.get(name) {
        Some(InspectValue::Number(value)) => *value,
        other => panic!("{part}.{name} is {other:?}"),
    }
}

fn hex(color: &Signal<Color>) -> String {
    color.get().unwrap().to_hex()
}

#[test]
fn a_drag_previews_locally_and_its_release_writes_once() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#ff0000", ColorPicker::new);
    let area = find(&ui, &root, "area");
    drag(
        &mut ui,
        &area,
        0.5,
        0.5,
        PointerKind::Press(PointerButton::Primary),
    );
    drag(&mut ui, &area, 0.25, 0.5, PointerKind::Move);
    // The parts present the preview; the binding is untouched.
    assert!((property(&ui, "area", "saturation") - 0.25).abs() < 0.02);
    assert_eq!(hex(&color), "#ff0000");
    assert_eq!(ui.focused(), Some(area.clone()));
    drag(
        &mut ui,
        &area,
        0.25,
        0.5,
        PointerKind::Release(PointerButton::Primary),
    );
    let [red, green, blue, _] = color.get().unwrap().to_srgb8();
    // Hue 0, saturation 0.25, value 0.5: a muted dark red.
    assert!(red > green && green == blue, "{red} {green} {blue}");
    assert!((i32::from(red) - 128).abs() <= 2);
}

#[test]
fn escape_cancels_a_drag_and_restores_the_parts() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", ColorPicker::new);
    let hue = find(&ui, &root, "hue");
    let before = property(&ui, "hue", "hue");
    drag(
        &mut ui,
        &hue,
        0.9,
        0.5,
        PointerKind::Press(PointerButton::Primary),
    );
    assert!(property(&ui, "hue", "hue") > 300.0);
    // The area follows the previewed hue as well.
    assert!(property(&ui, "area", "hue") > 300.0);
    key(&mut ui, KeyCode::Escape);
    assert_eq!(property(&ui, "hue", "hue"), before);
    assert_eq!(property(&ui, "area", "hue"), before);
    drag(
        &mut ui,
        &hue,
        0.9,
        0.5,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(hex(&color), "#2090a0");
}

#[test]
fn focus_loss_hiding_and_removal_cancel_without_writing() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", ColorPicker::new);
    let area = find(&ui, &root, "area");
    let before = property(&ui, "area", "value");
    // Focus moving elsewhere cancels.
    drag(
        &mut ui,
        &area,
        0.1,
        0.9,
        PointerKind::Press(PointerButton::Primary),
    );
    let other = find(&ui, &root, "other");
    ui.focus(&other).unwrap();
    settle(&mut ui);
    assert_eq!(property(&ui, "area", "value"), before);
    drag(
        &mut ui,
        &area,
        0.1,
        0.9,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(hex(&color), "#2090a0");
    // Hiding the picker mid-drag cancels; shown again, it presents the bound value.
    let picker = find(&ui, &root, "picker");
    drag(
        &mut ui,
        &area,
        0.1,
        0.9,
        PointerKind::Press(PointerButton::Primary),
    );
    ui.set_visible(&picker, false).unwrap();
    settle(&mut ui);
    ui.set_visible(&picker, true).unwrap();
    settle(&mut ui);
    assert_eq!(property(&ui, "area", "value"), before);
    assert_eq!(hex(&color), "#2090a0");
    // Removal mid-drag writes nothing and leaves no capture behind.
    drag(
        &mut ui,
        &area,
        0.1,
        0.9,
        PointerKind::Press(PointerButton::Primary),
    );
    let point = at(&ui, &area, 0.1, 0.9);
    ui.unmount(&picker).unwrap();
    settle(&mut ui);
    pointer(&mut ui, point, PointerKind::Release(PointerButton::Primary));
    assert_eq!(hex(&color), "#2090a0");
}

#[test]
fn an_application_write_during_a_drag_wins() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", ColorPicker::new);
    let area = find(&ui, &root, "area");
    drag(
        &mut ui,
        &area,
        0.1,
        0.1,
        PointerKind::Press(PointerButton::Primary),
    );
    color.set(Color::from_hex("#00ff00").unwrap()).unwrap();
    settle(&mut ui);
    assert!((property(&ui, "area", "hue") - 120.0).abs() < 0.01);
    drag(&mut ui, &area, 0.9, 0.9, PointerKind::Move);
    drag(
        &mut ui,
        &area,
        0.9,
        0.9,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(hex(&color), "#00ff00");
}

#[test]
fn an_application_write_during_a_hue_drag_clears_the_preview_in_every_part() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", ColorPicker::new);
    let hue = find(&ui, &root, "hue");
    // A hue drag previews 324° in every part, including the area that is
    // updated before the strip.
    drag(
        &mut ui,
        &hue,
        0.9,
        0.5,
        PointerKind::Press(PointerButton::Primary),
    );
    assert!((property(&ui, "area", "hue") - 324.0).abs() < 0.5);
    color.set(Color::from_hex("#00ff00").unwrap()).unwrap();
    settle(&mut ui);
    // The area shows the application's color, not the abandoned preview.
    assert!((property(&ui, "area", "hue") - 120.0).abs() < 0.01);
    assert!((property(&ui, "hue", "hue") - 120.0).abs() < 0.01);
    drag(
        &mut ui,
        &hue,
        0.9,
        0.5,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(hex(&color), "#00ff00");
}

#[test]
fn hue_survives_zero_saturation_and_zero_value() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", ColorPicker::new);
    let hue = property(&ui, "hue", "hue");
    let area = find(&ui, &root, "area");
    // Drag to the bottom edge: black, which has no hue of its own.
    drag(
        &mut ui,
        &area,
        0.5,
        0.5,
        PointerKind::Press(PointerButton::Primary),
    );
    drag(
        &mut ui,
        &area,
        0.5,
        1.5,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(hex(&color), "#000000");
    assert_eq!(property(&ui, "hue", "hue"), hue);
    // Raising the value again brings the same hue back.
    ui.focus(&area).unwrap();
    key(&mut ui, KeyCode::PageUp);
    key(&mut ui, KeyCode::PageUp);
    assert_eq!(property(&ui, "area", "hue"), hue);
    let [red, green, blue, _] = color.get().unwrap().to_srgb8();
    assert!(blue > red && green > red, "{red} {green} {blue}");
}

#[test]
fn keys_step_each_part_and_shift_steps_finer() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#ff0000", |color| ColorPicker::new(color).alpha(true));
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    assert!((property(&ui, "hue", "hue") - 1.0).abs() < 1e-3);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert!((property(&ui, "hue", "hue") - 1.1).abs() < 1e-3);
    // A page steps a tenth of the 360° range.
    key(&mut ui, KeyCode::PageUp);
    assert!((property(&ui, "hue", "hue") - 37.1).abs() < 1e-3);
    key(&mut ui, KeyCode::Home);
    assert_eq!(hex(&color), "#ff0000");
    let alpha = find(&ui, &root, "alpha");
    ui.focus(&alpha).unwrap();
    key(&mut ui, KeyCode::PageDown);
    assert!((color.get().unwrap().alpha() - 0.9).abs() < 1e-6);
    key(&mut ui, KeyCode::Home);
    assert_eq!(color.get().unwrap().alpha(), 0.0);
    // Each key press is one edit: nothing waits for the release.
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    settle(&mut ui);
    assert_eq!(color.get().unwrap().alpha(), 1.0);
    ui.key_input(KeyCode::End, KeyAction::Release).unwrap();
}

#[test]
fn hex_and_channel_entry_commit_on_save() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#ff0000", ColorPicker::new);
    let hex_entry = find(&ui, &find(&ui, &root, "hex"), "input");
    ui.focus(&hex_entry).unwrap();
    key(&mut ui, KeyCode::End);
    for _ in 0.."#ff0000".len() {
        key(&mut ui, KeyCode::Backspace);
    }
    ui.text_input("0F8").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(hex(&color), "#00ff88");
    assert!((property(&ui, "hue", "hue") - 152.0).abs() < 0.5);
    // Translucent text is refused without an alpha strip.
    for _ in 0.."#00ff88".len() {
        key(&mut ui, KeyCode::Backspace);
    }
    ui.text_input("#00ff8880").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(hex(&color), "#00ff88");
    key(&mut ui, KeyCode::Escape);
    // A channel saves on focus loss and keeps the others.
    let red = find(&ui, &find(&ui, &root, "r"), "input");
    ui.focus(&red).unwrap();
    key(&mut ui, KeyCode::End);
    for _ in 0..3 {
        key(&mut ui, KeyCode::Backspace);
    }
    ui.text_input("255").unwrap();
    let other = find(&ui, &root, "other");
    ui.focus(&other).unwrap();
    settle(&mut ui);
    assert_eq!(hex(&color), "#ffff88");
    // The hex field shows the channel edit.
    let shown = ui
        .inspect()
        .components
        .into_iter()
        .filter_map(|item| item.details.semantics)
        .find(|semantics| semantics.role == SemanticRole::TextInput)
        .and_then(|semantics| semantics.value);
    assert_eq!(shown, Some(SemanticValue::Text("#ffff88".into())));
}

#[test]
fn an_application_write_gives_the_text_entry_fresh_bindings() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#ff0000", ColorPicker::new);
    let entry = find(&ui, &root, "entry");
    let hex_field = ui
        .find(&entry, &["hex", "input"])
        .unwrap()
        .expect("the hex field");
    ui.focus(&hex_field).unwrap();
    settle(&mut ui);
    ui.text_input("draft").unwrap();
    color.set(Color::from_hex("#123456").unwrap()).unwrap();
    settle(&mut ui);
    let shown = ui
        .inspect()
        .components
        .into_iter()
        .filter_map(|item| item.details.semantics)
        .find(|semantics| semantics.role == SemanticRole::TextInput)
        .and_then(|semantics| semantics.value);
    // The draft is gone and the field shows the written color, in place and
    // still focused.
    assert_eq!(shown, Some(SemanticValue::Text("#123456".into())));
    assert_eq!(find(&ui, &root, "entry"), entry);
    assert_eq!(ui.focused(), Some(hex_field.clone()));
    // The picker's own edits keep the same fields and bindings.
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(find(&ui, &root, "entry"), entry);
}

#[test]
fn a_disabled_picker_offers_no_tab_stops_and_ignores_input() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#ff0000", |color| ColorPicker::new(color).disabled());
    let area = find(&ui, &root, "area");
    drag(
        &mut ui,
        &area,
        0.1,
        0.1,
        PointerKind::Press(PointerButton::Primary),
    );
    drag(
        &mut ui,
        &area,
        0.1,
        0.1,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(hex(&color), "#ff0000");
    key(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "other")));
}

#[test]
fn the_popover_opens_from_its_swatch_and_escape_closes_it_mid_drag() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(500.0, 700.0, 1.0).unwrap())
        .unwrap();
    let color = ui.signal(Color::from_hex("#2090a0").unwrap());
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([Element::new(
                "color",
                ColorPicker::new(color.clone())
                    .name("Track color")
                    .into_popover(),
            )]),
        ))
        .unwrap();
    settle(&mut ui);
    let swatch = find(&ui, &root, "swatch");
    ui.focus(&swatch).unwrap();
    key(&mut ui, KeyCode::Enter);
    let area = find(&ui, &root, "area");
    assert_eq!(ui.focused(), Some(area.clone()));
    drag(
        &mut ui,
        &area,
        0.9,
        0.1,
        PointerKind::Press(PointerButton::Primary),
    );
    key(&mut ui, KeyCode::Escape);
    let popover = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == "color")
        .unwrap();
    assert_eq!(
        popover.details.properties.get("open"),
        Some(&InspectValue::Bool(false))
    );
    assert_eq!(hex(&color), "#2090a0");
    let name = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == "swatch")
        .and_then(|item| item.details.semantics)
        .and_then(|semantics| semantics.name);
    assert_eq!(name.as_deref(), Some("Track color"));
}

#[test]
fn inspection_reports_the_picker_and_its_parts() {
    let Setup { ui, .. } = setup("#ff8000", |color| ColorPicker::new(color).name("Fill"));
    let components = ui.inspect().components;
    let picker = components.iter().find(|item| item.key == "picker").unwrap();
    let semantics = picker.details.semantics.clone().unwrap();
    assert_eq!(semantics.role, SemanticRole::Group);
    assert_eq!(semantics.name.as_deref(), Some("Fill"));
    assert_eq!(semantics.value, Some(SemanticValue::Text("#ff8000".into())));
    let hue = components.iter().find(|item| item.key == "hue").unwrap();
    let semantics = hue.details.semantics.clone().unwrap();
    assert_eq!(semantics.role, SemanticRole::Slider);
    assert!(matches!(
        semantics.value,
        Some(SemanticValue::Number {
            minimum: Some(0.0),
            maximum: Some(360.0),
            ..
        })
    ));
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn on_save_reports_saved_values_of_fields() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal(String::new());
    let number = ui.signal(1_i64);
    let saves = ui.signal(Vec::<String>::new());
    let (a, b) = (saves.clone(), saves.clone());
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([
                Element::new(
                    "text",
                    TextField::new(text.clone())
                        .normalize(|text| Ok(text.trim().to_owned()))
                        .on_save(move |_, saved| {
                            let mut list = a.get()?;
                            list.push(format!("text {saved}"));
                            a.set(list)
                        }),
                ),
                Element::new(
                    "number",
                    IntegerInput::new(number.clone())
                        .stepper(true)
                        .on_save(move |_, saved| {
                            let mut list = b.get()?;
                            list.push(format!("number {saved}"));
                            b.set(list)
                        }),
                ),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    let field = find(&ui, &root, "text");
    ui.focus(&field).unwrap();
    ui.text_input(" hi ").unwrap();
    key(&mut ui, KeyCode::Enter);
    // An unchanged save and an application write report nothing.
    key(&mut ui, KeyCode::Enter);
    text.set("app".into()).unwrap();
    settle(&mut ui);
    let input = find(&ui, &root, "number");
    ui.focus(&input).unwrap();
    key(&mut ui, KeyCode::End);
    key(&mut ui, KeyCode::Backspace);
    ui.text_input("7").unwrap();
    // Focus loss saves.
    ui.focus(&field).unwrap();
    settle(&mut ui);
    assert_eq!(saves.get().unwrap(), ["text hi", "number 7"]);
    assert_eq!(number.get().unwrap(), 7);
}

#[test]
fn on_save_skips_a_save_that_normalizes_to_the_bound_text() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal(String::from("name"));
    let saves = ui.signal(0_usize);
    let counter = saves.clone();
    let root = ui
        .mount_tree(Element::new(
            "text",
            TextField::new(text.clone())
                .normalize(|text| Ok(text.trim().to_owned()))
                .on_save(move |_, _| {
                    let count = counter.get()?;
                    counter.set(count + 1)
                }),
        ))
        .unwrap();
    settle(&mut ui);
    ui.focus(&root).unwrap();
    key(&mut ui, KeyCode::End);
    ui.text_input(" ").unwrap();
    key(&mut ui, KeyCode::Enter);
    // "name " trims to the bound "name": nothing was written.
    assert_eq!(text.get().unwrap(), "name");
    assert_eq!(saves.get().unwrap(), 0);
}

#[test]
fn a_failing_rejection_report_does_not_suppress_on_save() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal(String::new());
    let saves = ui.signal(Vec::<String>::new());
    let list = saves.clone();
    let root = ui
        .mount_tree(Element::new(
            "text",
            TextField::new(text.clone())
                .normalize(|text| {
                    if text == "bad" {
                        Err(InputRejection::new("refused"))
                    } else {
                        Ok(text)
                    }
                })
                .on_rejection(|_, rejection| match rejection {
                    Some(_) => Ok(()),
                    // The report that a refusal ended fails.
                    None => Err(RuntimeError::BorrowConflict),
                })
                .on_save(move |_, saved| {
                    let mut saved_list = list.get()?;
                    saved_list.push(saved.to_owned());
                    list.set(saved_list)
                }),
        ))
        .unwrap();
    settle(&mut ui);
    ui.focus(&root).unwrap();
    ui.text_input("bad").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert!(text.get().unwrap().is_empty());
    for _ in 0..3 {
        key(&mut ui, KeyCode::Backspace);
    }
    ui.text_input("good").unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press)
        .expect_err("the failing report's error is returned");
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(text.get().unwrap(), "good");
    assert_eq!(saves.get().unwrap(), ["good"]);
}

#[test]
fn a_failing_rejection_report_does_not_suppress_a_number_save() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let number = ui.signal(1_i64);
    let saves = ui.signal(Vec::<i64>::new());
    let list = saves.clone();
    let root = ui
        .mount_tree(Element::new(
            "number",
            IntegerInput::new(number.clone())
                .normalize(|value| {
                    if value == 13 {
                        Err(InputRejection::new("refused"))
                    } else {
                        Ok(value)
                    }
                })
                .unwrap()
                .on_rejection(|_, rejection| match rejection {
                    Some(_) => Ok(()),
                    None => Err(RuntimeError::BorrowConflict),
                })
                .on_save(move |_, saved| {
                    let mut saved_list = list.get()?;
                    saved_list.push(saved);
                    list.set(saved_list)
                }),
        ))
        .unwrap();
    settle(&mut ui);
    ui.focus(&root).unwrap();
    key(&mut ui, KeyCode::End);
    key(&mut ui, KeyCode::Backspace);
    ui.text_input("13").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(number.get().unwrap(), 1);
    key(&mut ui, KeyCode::Backspace);
    key(&mut ui, KeyCode::Backspace);
    ui.text_input("7").unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press)
        .expect_err("the failing report's error is returned");
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(number.get().unwrap(), 7);
    assert_eq!(saves.get().unwrap(), [7]);
}

#[test]
fn rgb_channel_saves_preserve_exact_alpha_when_alpha_editing_is_disabled() {
    for channel in ["r", "g", "b"] {
        let Setup {
            mut ui,
            color,
            root,
        } = setup("#123456", ColorPicker::new);
        let original = Color::from_srgb(0.1, 0.2, 0.3, 0.1234567).unwrap();
        color.set(original).unwrap();
        settle(&mut ui);
        let input = find(&ui, &find(&ui, &root, channel), "input");
        ui.focus(&input).unwrap();
        key(&mut ui, KeyCode::End);
        for _ in 0..3 {
            key(&mut ui, KeyCode::Backspace);
        }
        ui.text_input("200").unwrap();
        key(&mut ui, KeyCode::Enter);
        let actual = color.get().unwrap();
        assert_eq!(actual.alpha(), original.alpha(), "channel {channel}");
        let index = match channel {
            "r" => 0,
            "g" => 1,
            "b" => 2,
            _ => unreachable!(),
        };
        let mut expected = original.to_srgb8();
        expected[index] = 200;
        assert_eq!(actual.to_srgb8(), expected);
    }
}

fn owned_popover_setup() -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(500.0, 700.0, 1.0).unwrap())
        .unwrap();
    let color = ui.signal(Color::from_hex("#2090a0").unwrap());
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([Element::new(
                "color",
                ColorPicker::new(color.clone())
                    .name("Mounted color")
                    .into_popover(),
            )]),
        ))
        .unwrap();
    settle(&mut ui);
    Setup { ui, color, root }
}
fn redescribe_owned_popover(ui: &mut Runtime, root: &WidgetId, color: &Signal<Color>) {
    ui.reconcile_children(
        root,
        vec![Element::new(
            "color",
            ColorPicker::new(color.clone())
                .name("Mounted color")
                .into_popover(),
        )],
    )
    .unwrap();
    settle(ui);
}
fn popover_open(ui: &Runtime) -> bool {
    ui.inspect()
        .components
        .into_iter()
        .find(|item| item.key == "color")
        .unwrap()
        .details
        .properties
        .get("open")
        == Some(&InspectValue::Bool(true))
}
fn first_text_value(ui: &Runtime) -> Option<SemanticValue> {
    ui.inspect()
        .components
        .into_iter()
        .filter_map(|item| item.details.semantics)
        .find(|semantics| semantics.role == SemanticRole::TextInput)
        .and_then(|semantics| semantics.value)
}

#[test]
fn compatible_owned_popover_redescription_keeps_open_focus_and_hex_draft() {
    let Setup {
        mut ui,
        color,
        root,
    } = owned_popover_setup();
    let swatch = find(&ui, &root, "swatch");
    ui.focus(&swatch).unwrap();
    key(&mut ui, KeyCode::Enter);
    let entry = find(&ui, &find(&ui, &root, "hex"), "input");
    ui.focus(&entry).unwrap();
    ui.text_input("draft").unwrap();
    settle(&mut ui);
    let draft = first_text_value(&ui);
    assert!(matches!(&draft, Some(SemanticValue::Text(text)) if text.contains("draft")));
    redescribe_owned_popover(&mut ui, &root, &color);
    assert!(popover_open(&ui));
    assert_eq!(ui.focused(), Some(entry.clone()));
    assert_eq!(first_text_value(&ui), draft);
    assert_eq!(find(&ui, &root, "swatch"), swatch);
    assert_eq!(hex(&color), "#2090a0");
    key(&mut ui, KeyCode::Escape); // Cancel the draft before dismissing the surface.
    key(&mut ui, KeyCode::Escape);
    assert!(!popover_open(&ui));
    ui.focus(&swatch).unwrap();
    key(&mut ui, KeyCode::Enter);
    assert!(popover_open(&ui));
    assert_eq!(ui.focused(), Some(find(&ui, &root, "area")));
}

#[test]
fn compatible_owned_popover_redescription_preserves_drag_until_release_commit() {
    let Setup {
        mut ui,
        color,
        root,
    } = owned_popover_setup();
    let swatch = find(&ui, &root, "swatch");
    ui.focus(&swatch).unwrap();
    key(&mut ui, KeyCode::Enter);
    let area = find(&ui, &root, "area");
    drag(
        &mut ui,
        &area,
        0.9,
        0.1,
        PointerKind::Press(PointerButton::Primary),
    );
    let saturation = property(&ui, "area", "saturation");
    let value = property(&ui, "area", "value");
    assert_eq!(hex(&color), "#2090a0");
    redescribe_owned_popover(&mut ui, &root, &color);
    assert!(popover_open(&ui));
    assert_eq!(find(&ui, &root, "area"), area);
    assert_eq!(property(&ui, "area", "saturation"), saturation);
    assert_eq!(property(&ui, "area", "value"), value);
    assert_eq!(hex(&color), "#2090a0");
    drag(
        &mut ui,
        &area,
        0.9,
        0.1,
        PointerKind::Release(PointerButton::Primary),
    );
    let committed = color.get().unwrap();
    assert_ne!(committed.to_hex(), "#2090a0");
    drag(&mut ui, &area, 0.1, 0.9, PointerKind::Move);
    assert_eq!(color.get().unwrap(), committed);
}

#[test]
fn owned_popover_keeps_anchor_and_editor_binding_options_until_a_new_key() {
    let Setup {
        mut ui,
        color,
        root,
    } = owned_popover_setup();
    let replacement = ui.signal(Color::from_hex("#ff0000").unwrap());
    let describe = |key| {
        Element::new(
            key,
            ColorPicker::new(replacement.clone())
                .name("Replacement")
                .alpha(true)
                .text_entry(false)
                .disabled()
                .into_popover(),
        )
    };
    ui.reconcile_children(&root, vec![describe("color")])
        .unwrap();
    settle(&mut ui);
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == "swatch")
        .unwrap()
        .details
        .semantics
        .unwrap();
    assert_eq!(semantics.name.as_deref(), Some("Mounted color"));
    assert_eq!(semantics.value, Some(SemanticValue::Text("#2090a0".into())));
    assert!(!semantics.disabled);
    let swatch = find(&ui, &root, "swatch");
    ui.focus(&swatch).unwrap();
    key(&mut ui, KeyCode::Enter);
    let area = find(&ui, &root, "area");
    drag(
        &mut ui,
        &area,
        0.9,
        0.1,
        PointerKind::Press(PointerButton::Primary),
    );
    drag(
        &mut ui,
        &area,
        0.9,
        0.1,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_ne!(hex(&color), "#2090a0");
    assert_eq!(hex(&replacement), "#ff0000");
    assert!(ui.find_descendant(&root, "alpha").unwrap().is_none());
    assert!(ui.find_descendant(&root, "hex").unwrap().is_some());
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "replacement",
            ColorPicker::new(replacement.clone())
                .name("Replacement")
                .alpha(true)
                .text_entry(false)
                .into_popover(),
        )],
    )
    .unwrap();
    settle(&mut ui);
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == "swatch")
        .unwrap()
        .details
        .semantics
        .unwrap();
    assert_eq!(semantics.name.as_deref(), Some("Replacement"));
    assert_eq!(semantics.value, Some(SemanticValue::Text("#ff0000".into())));
    assert!(!semantics.disabled);
    let swatch = find(&ui, &root, "swatch");
    ui.focus(&swatch).unwrap();
    key(&mut ui, KeyCode::Enter);
    assert!(ui.find_descendant(&root, "alpha").unwrap().is_some());
    assert!(ui.find_descendant(&root, "hex").unwrap().is_none());
    ui.reconcile_children(&root, vec![describe("disabled-replacement")])
        .unwrap();
    settle(&mut ui);
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == "swatch")
        .unwrap()
        .details
        .semantics
        .unwrap();
    assert!(semantics.disabled);
}

#[test]
fn ordinary_popover_replaces_external_open_binding_and_child_content() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(500.0, 700.0, 1.0).unwrap())
        .unwrap();
    let first = ui.signal(false);
    let second = ui.signal(false);
    let describe = |open: Signal<bool>, label| {
        Element::new(
            "color",
            Popover::new(
                open,
                Element::new("anchor-label", Label::new("Anchor")),
                Element::new("content-label", Label::new(label)),
            ),
        )
    };
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([describe(first.clone(), "Original")]),
        ))
        .unwrap();
    settle(&mut ui);
    first.set(true).unwrap();
    settle(&mut ui);
    assert!(popover_open(&ui));
    let content = find(&ui, &root, "content-label");
    ui.reconcile_children(&root, vec![describe(second.clone(), "Replacement")])
        .unwrap();
    settle(&mut ui);
    assert!(!popover_open(&ui));
    assert!(first.get().unwrap());
    assert!(!second.get().unwrap());
    assert_eq!(find(&ui, &root, "content-label"), content);
    second.set(true).unwrap();
    settle(&mut ui);
    assert!(popover_open(&ui));
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == "content-label")
        .unwrap()
        .details
        .semantics
        .unwrap();
    assert_eq!(semantics.name.as_deref(), Some("Replacement"));
    first.set(false).unwrap();
    settle(&mut ui);
    assert!(popover_open(&ui));
    second.set(false).unwrap();
    settle(&mut ui);
    assert!(!popover_open(&ui));
    // Dismissal writes the replacement binding, never the original one.
    first.set(true).unwrap();
    second.set(true).unwrap();
    settle(&mut ui);
    assert!(popover_open(&ui));
    key(&mut ui, KeyCode::Escape);
    assert!(!popover_open(&ui), "Escape closes the replaced popover");
    assert!(!second.get().unwrap());
    assert!(first.get().unwrap(), "the old binding is left alone");
    second.set(true).unwrap();
    settle(&mut ui);
    assert!(popover_open(&ui));
    pointer(
        &mut ui,
        Point::new(480.0, 680.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(
        &mut ui,
        Point::new(480.0, 680.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(!popover_open(&ui), "an outside press closes it");
    assert!(!second.get().unwrap());
    assert!(first.get().unwrap(), "the old binding is left alone");
}

#[test]
fn modified_part_keys_yield_to_application_commands() {
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#ff0000", ColorPicker::new);
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    let modifiers = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    let hits = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = hits.clone();
    let _command = ui
        .register_command(
            "Chord",
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Named(KeyCode::ArrowRight),
                modifiers,
            )],
            move |_, _| {
                counted.set(counted.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    ui.native_modifiers(modifiers).unwrap();
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::ArrowRight,
            action,
            CommandKey::Named(KeyCode::ArrowRight),
        ))
        .unwrap();
    }
    settle(&mut ui);
    assert_eq!(hits.get(), 1);
    assert_eq!(hex(&color), "#ff0000");
}

#[test]
fn a_held_part_key_keeps_stepping_when_a_modifier_is_added() {
    let Setup { mut ui, root, .. } = setup("#ff0000", ColorPicker::new);
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    settle(&mut ui);
    assert!((property(&ui, "hue", "hue") - 1.0).abs() < 1e-3);
    ui.native_modifiers(Modifiers::PRIMARY).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Repeat)
        .unwrap();
    settle(&mut ui);
    assert!(
        (property(&ui, "hue", "hue") - 2.0).abs() < 1e-3,
        "the owned press keeps repeating"
    );
    ui.native_modifiers(Modifiers::default()).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
}

/// The colors `on_change` reported, each with the bound color the handler
/// saw when it ran.
type Calls = std::rc::Rc<std::cell::RefCell<Vec<(Color, Color)>>>;

fn recording(
    calls: &Calls,
    color: &Signal<Color>,
) -> impl FnMut(&mut EventContext, Color) -> Result<(), RuntimeError> + 'static {
    let (calls, color) = (calls.clone(), color.clone());
    move |_, written| {
        calls.borrow_mut().push((written, color.get()?));
        Ok(())
    }
}

fn reported(calls: &Calls) -> Vec<String> {
    calls
        .borrow()
        .iter()
        .map(|(color, _)| color.to_hex())
        .collect()
}

fn replace_hex(ui: &mut Runtime, root: &WidgetId, text: &str) -> WidgetId {
    let entry = find(ui, &find(ui, root, "hex"), "input");
    ui.focus(&entry).unwrap();
    key(ui, KeyCode::End);
    for _ in 0..12 {
        key(ui, KeyCode::Backspace);
    }
    ui.text_input(text).unwrap();
    settle(ui);
    entry
}

#[test]
fn on_change_reports_each_committed_user_write_once() {
    let calls = Calls::default();
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", |color| {
        let handler = recording(&calls, &color);
        ColorPicker::new(color).alpha(true).on_change(handler)
    });
    // An area drag reports once, at its release.
    let area = find(&ui, &root, "area");
    drag(
        &mut ui,
        &area,
        0.2,
        0.2,
        PointerKind::Press(PointerButton::Primary),
    );
    drag(&mut ui, &area, 0.4, 0.3, PointerKind::Move);
    assert!(calls.borrow().is_empty(), "nothing while previewing");
    drag(
        &mut ui,
        &area,
        0.4,
        0.3,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(reported(&calls), [hex(&color)]);
    // A key step reports on its press, not its release.
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    assert_eq!(calls.borrow().len(), 2);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Repeat)
        .unwrap();
    assert_eq!(calls.borrow().len(), 3, "a repeat is an edit");
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    assert_eq!(calls.borrow().len(), 3);
    // The alpha strip by pointer and by Home.
    let alpha = find(&ui, &root, "alpha");
    drag(
        &mut ui,
        &alpha,
        0.5,
        0.5,
        PointerKind::Press(PointerButton::Primary),
    );
    drag(
        &mut ui,
        &alpha,
        0.5,
        0.5,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(calls.borrow().len(), 4);
    ui.focus(&alpha).unwrap();
    key(&mut ui, KeyCode::Home);
    assert_eq!(calls.borrow().len(), 5);
    assert_eq!(color.get().unwrap().alpha(), 0.0);
    // A hex save and a channel save report the saved color.
    replace_hex(&mut ui, &root, "#112233");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(reported(&calls).last().unwrap(), "#112233");
    let red = find(&ui, &find(&ui, &root, "r"), "input");
    ui.focus(&red).unwrap();
    key(&mut ui, KeyCode::End);
    for _ in 0..3 {
        key(&mut ui, KeyCode::Backspace);
    }
    ui.text_input("255").unwrap();
    let other = find(&ui, &root, "other");
    ui.focus(&other).unwrap();
    settle(&mut ui);
    assert_eq!(reported(&calls).last().unwrap(), "#ff2233");
    assert_eq!(calls.borrow().len(), 7);
    // Every call saw the binding already holding exactly its color, and a
    // hex save writes the parsed color itself.
    assert!(
        calls
            .borrow()
            .iter()
            .all(|(written, bound)| written == bound)
    );
    assert_eq!(calls.borrow()[5].0, Color::from_hex("#112233").unwrap());
}

#[test]
fn on_change_ignores_what_the_user_did_not_commit() {
    let calls = Calls::default();
    let allowed = std::cell::RefCell::new(None);
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#ff0000", |color| {
        let handler = recording(&calls, &color);
        let permission = color.sibling(true);
        *allowed.borrow_mut() = Some(permission.clone());
        ColorPicker::new(color)
            .enabled(permission)
            .on_change(handler)
    });
    let allowed = allowed.into_inner().unwrap();
    // An application write.
    color.set(Color::from_hex("#00ff00").unwrap()).unwrap();
    settle(&mut ui);
    color.set(Color::from_hex("#ff0000").unwrap()).unwrap();
    settle(&mut ui);
    // A drag cancelled by Escape.
    let area = find(&ui, &root, "area");
    drag(
        &mut ui,
        &area,
        0.3,
        0.3,
        PointerKind::Press(PointerButton::Primary),
    );
    drag(&mut ui, &area, 0.5, 0.5, PointerKind::Move);
    key(&mut ui, KeyCode::Escape);
    drag(
        &mut ui,
        &area,
        0.5,
        0.5,
        PointerKind::Release(PointerButton::Primary),
    );
    // Hue End and Home on a red: the retained hue moves between 0° and
    // 360°, and the bound value stays exactly as it was. This red does not
    // survive a round trip through HSV unchanged.
    let red = Color::from_hex("#802020").unwrap();
    color.set(red).unwrap();
    settle(&mut ui);
    let revision = color.revision();
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    key(&mut ui, KeyCode::End);
    key(&mut ui, KeyCode::End);
    key(&mut ui, KeyCode::Home);
    assert_eq!(color.get().unwrap(), red);
    assert_eq!(color.revision(), revision, "the binding was not rewritten");
    // A hue step on grey keeps the bound value exactly.
    let grey = Color::from_hex("#808080").unwrap();
    color.set(grey).unwrap();
    settle(&mut ui);
    let revision = color.revision();
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(color.get().unwrap(), grey);
    assert_eq!(color.revision(), revision, "the binding was not rewritten");
    // Refused hex text.
    replace_hex(&mut ui, &root, "nonsense");
    key(&mut ui, KeyCode::Enter);
    key(&mut ui, KeyCode::Escape);
    // A drag ended by revoking the permission, and its stale release after
    // the permission returns.
    drag(
        &mut ui,
        &area,
        0.3,
        0.3,
        PointerKind::Press(PointerButton::Primary),
    );
    drag(&mut ui, &area, 0.6, 0.6, PointerKind::Move);
    allowed.set(false).unwrap();
    settle(&mut ui);
    allowed.set(true).unwrap();
    settle(&mut ui);
    drag(
        &mut ui,
        &area,
        0.6,
        0.6,
        PointerKind::Release(PointerButton::Primary),
    );
    // Hiding mid-drag, showing and releasing.
    let picker = find(&ui, &root, "picker");
    drag(
        &mut ui,
        &area,
        0.3,
        0.3,
        PointerKind::Press(PointerButton::Primary),
    );
    ui.set_visible(&picker, false).unwrap();
    settle(&mut ui);
    ui.set_visible(&picker, true).unwrap();
    settle(&mut ui);
    drag(
        &mut ui,
        &area,
        0.3,
        0.3,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(hex(&color), "#808080");
    assert!(calls.borrow().is_empty(), "{:?}", calls.borrow());
    // Hiding with a typed hex draft saves nothing.
    replace_hex(&mut ui, &root, "#123456");
    ui.set_visible(&picker, false).unwrap();
    settle(&mut ui);
    ui.set_visible(&picker, true).unwrap();
    settle(&mut ui);
    assert_eq!(hex(&color), "#808080");
    assert!(calls.borrow().is_empty(), "{:?}", calls.borrow());
}

#[test]
fn a_failing_handler_keeps_the_write_and_is_called_again() {
    let calls = Calls::default();
    let failing = std::rc::Rc::new(std::cell::Cell::new(true));
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", |color| {
        let mut record = recording(&calls, &color);
        let failing = failing.clone();
        ColorPicker::new(color).on_change(move |cx, written| {
            record(cx, written)?;
            if failing.get() {
                Err(ApplicationError::new(std::io::Error::other("refused")).into())
            } else {
                Ok(())
            }
        })
    });
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    assert!(ui.key_input(KeyCode::ArrowRight, KeyAction::Press).is_err());
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release).ok();
    settle(&mut ui);
    let after_key = hex(&color);
    assert_ne!(after_key, "#2090a0", "the color stays written");
    assert_eq!(reported(&calls), std::slice::from_ref(&after_key));
    // A failing text save propagates too and keeps its write.
    replace_hex(&mut ui, &root, "#445566");
    assert!(ui.key_input(KeyCode::Enter, KeyAction::Press).is_err());
    ui.key_input(KeyCode::Enter, KeyAction::Release).ok();
    settle(&mut ui);
    assert_eq!(hex(&color), "#445566");
    assert_eq!(calls.borrow().len(), 2);
    // The next edit calls it again.
    failing.set(false);
    ui.focus(&hue).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(calls.borrow().len(), 3);
}

#[test]
fn a_handler_that_writes_and_then_fails_keeps_both_writes() {
    let calls = Calls::default();
    let replacement = Color::from_hex("#010203").unwrap();
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", |color| {
        let mut record = recording(&calls, &color);
        let bound = color.clone();
        ColorPicker::new(color).on_change(move |cx, written| {
            record(cx, written)?;
            bound.set(replacement)?;
            Err(ApplicationError::new(std::io::Error::other("refused")).into())
        })
    });
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    assert!(ui.key_input(KeyCode::ArrowRight, KeyAction::Press).is_err());
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release).ok();
    settle(&mut ui);
    assert_eq!(color.get().unwrap(), replacement);
    replace_hex(&mut ui, &root, "#778899");
    assert!(ui.key_input(KeyCode::Enter, KeyAction::Press).is_err());
    ui.key_input(KeyCode::Enter, KeyAction::Release).ok();
    settle(&mut ui);
    assert_eq!(color.get().unwrap(), replacement);
    assert_eq!(reported(&calls).len(), 2);
    assert_eq!(reported(&calls)[1], "#778899");
}

#[test]
fn a_handler_that_writes_another_color_wins_without_a_second_call() {
    let calls = Calls::default();
    let replacement = Color::from_hex("#010203").unwrap();
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", |color| {
        let mut record = recording(&calls, &color);
        let bound = color.clone();
        ColorPicker::new(color).on_change(move |cx, written| {
            record(cx, written)?;
            bound.set(replacement)
        })
    });
    // A part commit.
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(hex(&color), "#010203");
    assert_eq!(calls.borrow().len(), 1);
    assert_ne!(
        reported(&calls)[0],
        "#010203",
        "the user's color was reported"
    );
    // A text save: the field then shows the application's color.
    replace_hex(&mut ui, &root, "#778899");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(reported(&calls).last().unwrap(), "#778899");
    assert_eq!(calls.borrow().len(), 2);
    assert_eq!(hex(&color), "#010203");
    let other = find(&ui, &root, "other");
    ui.focus(&other).unwrap();
    settle(&mut ui);
    assert_eq!(
        first_text_value(&ui),
        Some(SemanticValue::Text("#010203".into()))
    );
    assert_eq!(calls.borrow().len(), 2);
}

#[test]
fn the_handler_follows_redescription_in_both_forms() {
    // The plain picker.
    let first = Calls::default();
    let second = Calls::default();
    let Setup {
        mut ui,
        color,
        root,
    } = setup("#2090a0", |color| {
        let handler = recording(&first, &color);
        ColorPicker::new(color).on_change(handler)
    });
    let describe = |handler: Option<&Calls>| {
        let mut picker = ColorPicker::new(color.clone());
        if let Some(calls) = handler {
            picker = picker.on_change(recording(calls, &color));
        }
        vec![
            Element::new("picker", picker),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ]
    };
    ui.reconcile_children(&root, describe(Some(&second)))
        .unwrap();
    settle(&mut ui);
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!((first.borrow().len(), second.borrow().len()), (0, 1));
    ui.reconcile_children(&root, describe(None)).unwrap();
    settle(&mut ui);
    ui.focus(&hue).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!((first.borrow().len(), second.borrow().len()), (0, 1));

    // The popover form keeps its open state, focus and draft.
    let Setup {
        mut ui,
        color,
        root,
    } = owned_popover_setup();
    let calls = Calls::default();
    let swatch = find(&ui, &root, "swatch");
    ui.focus(&swatch).unwrap();
    key(&mut ui, KeyCode::Enter);
    let entry = replace_hex(&mut ui, &root, "#abcdef");
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "color",
            ColorPicker::new(color.clone())
                .name("Mounted color")
                .on_change(recording(&calls, &color))
                .into_popover(),
        )],
    )
    .unwrap();
    settle(&mut ui);
    assert!(popover_open(&ui));
    assert_eq!(ui.focused(), Some(entry));
    key(&mut ui, KeyCode::Enter);
    assert_eq!(reported(&calls), ["#abcdef"]);
    redescribe_owned_popover(&mut ui, &root, &color);
    let hue = find(&ui, &root, "hue");
    ui.focus(&hue).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(
        calls.borrow().len(),
        1,
        "a description without one removes it"
    );
}
