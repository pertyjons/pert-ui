//! `TextSource` contracts through the public facade: fixed, rich and bound
//! captions share one argument, a binding is followed without redescription,
//! the same binding keeps its text across redescription, icon slots stay
//! independent of the text, and parameter-named controls take icon decoration
//! and a replacing label.
use crate::support;
use pert_ui::*;
use support::{frame, settle};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 240.0, 1.0)?)?;
    Ok(ui)
}

fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("no component {key}"))
}

fn name(ui: &Runtime, key: &str) -> Option<String> {
    component(ui, key).details.semantics.and_then(|s| s.name)
}

fn text_property(ui: &Runtime, key: &str, property: &str) -> String {
    match &component(ui, key).details.properties[property] {
        InspectValue::Text(text) => text.clone(),
        other => panic!("unexpected {property} {other:?}"),
    }
}

/// Caption-bearing controls built from one text source.
fn controls(text: impl Into<TextSource> + Clone) -> Vec<Element> {
    vec![
        Element::new("label", Label::new(text.clone())),
        Element::new("button", Button::new(text.clone(), |_| Ok(()))),
        Element::new("link", Link::new(text.clone(), |_| Ok(()))),
        Element::new("chip", Chip::new(text.clone())),
        Element::new(
            "icon",
            Button::icon(Icon::Settings, text.clone(), |_| Ok(())),
        ),
    ]
}

#[test]
fn bound_captions_follow_their_binding_and_keep_it_across_redescription() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal("Mixer".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new(controls(text.read_only())),
    ))?;
    frame(&mut ui);
    for key in ["label", "button", "link", "chip", "icon"] {
        assert_eq!(name(&ui, key).as_deref(), Some("Mixer"), "{key}");
    }
    text.set("Mischpult".to_owned())?;
    frame(&mut ui);
    for key in ["label", "button", "link", "chip", "icon"] {
        assert_eq!(name(&ui, key).as_deref(), Some("Mischpult"), "{key}");
    }

    // Describing again with the same binding keeps the text it has read.
    ui.reconcile_children(&root, controls(text.read_only()))?;
    for key in ["label", "button", "link", "chip", "icon"] {
        assert_eq!(name(&ui, key).as_deref(), Some("Mischpult"), "{key}");
    }
    frame(&mut ui);

    // Fixed text replaces the binding, which no longer applies.
    ui.reconcile_children(&root, controls("Fixed"))?;
    frame(&mut ui);
    text.set("Ignored".to_owned())?;
    frame(&mut ui);
    for key in ["label", "button", "link", "chip", "icon"] {
        assert_eq!(name(&ui, key).as_deref(), Some("Fixed"), "{key}");
    }
    Ok(())
}

#[test]
fn a_bound_rich_caption_matches_the_same_fixed_caption() -> TestResult {
    let content = RichText::from("Gain").leading_icon(Icon::Info);
    let mut bound = runtime()?;
    let source = bound.signal(RichText::from("Before"));
    bound.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "chip",
            Chip::new(source.read_only()).leading_icon(Icon::Settings),
        )]),
    ))?;
    frame(&mut bound);
    source.set(content.clone())?;
    frame(&mut bound);
    let mut fixed = runtime()?;
    fixed.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "chip",
            Chip::new(content).leading_icon(Icon::Settings),
        )]),
    ))?;
    frame(&mut fixed);
    assert_eq!(bound.paint()?, fixed.paint()?);
    assert_eq!(name(&bound, "chip").as_deref(), Some("Gain"));
    Ok(())
}

#[test]
fn a_removable_chip_names_its_removal_after_its_bound_text() -> TestResult {
    let mut ui = runtime()?;
    let text = ui.signal("Kick".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "chip",
            Chip::removable(text.read_only(), |_| Ok(())),
        )]),
    ))?;
    frame(&mut ui);
    text.set("Snare".to_owned())?;
    frame(&mut ui);
    assert_eq!(text_property(&ui, "chip", "text"), "Snare");
    let semantics = component(&ui, "chip").details.semantics.unwrap();
    assert_eq!(semantics.name.as_deref(), Some("Snare"));
    assert_eq!(semantics.description.as_deref(), Some("Remove Snare"));
    Ok(())
}

#[test]
fn menu_bar_titles_follow_bound_labels() -> TestResult {
    let mut ui = runtime()?;
    let title = ui.signal("File".to_owned());
    let bar = MenuBar::new(
        &ui,
        vec![MenuItem::submenu(
            "file",
            title.read_only(),
            vec![MenuItem::new("open", "Open", |_| Ok(()))],
        )],
    )?;
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("bar", bar)]),
    ))?;
    frame(&mut ui);
    assert_eq!(name(&ui, "title/file").as_deref(), Some("File"));
    title.set("Datei".to_owned())?;
    frame(&mut ui);
    assert_eq!(name(&ui, "title/file").as_deref(), Some("Datei"));

    Ok(())
}

#[test]
fn a_binding_from_another_runtime_fails_the_update() -> TestResult {
    let mut ui = runtime()?;
    let other = Runtime::new();
    let foreign = other.signal("Elsewhere".to_owned());
    ui.mount_tree(Element::new("label", Label::new(foreign.read_only())))?;
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::ForeignRuntime),
        "a foreign binding is rejected when read"
    );
    Ok(())
}

fn bypass(
    ui: &mut Runtime,
) -> Result<(Parameter<bool>, ParameterHost<bool>), Box<dyn std::error::Error>> {
    Ok(ui.parameter(
        ParameterId::new(1, 1, "bypass")?,
        ParameterSpec::boolean("Bypass", false)?,
    )?)
}

#[test]
fn icon_slots_decorate_a_followed_parameter_name() -> TestResult {
    let mut ui = runtime()?;
    let (parameter, host) = bypass(&mut ui)?;
    let describe = |parameter: Parameter<bool>| {
        Element::new(
            "control",
            Checkbox::for_parameter(parameter)
                .leading_icon(Icon::Settings)
                .trailing_icon(Icon::Info),
        )
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([describe(parameter.clone())]),
    ))?;
    frame(&mut ui);
    host.replace_spec(ParameterSpec::boolean("Bypass all", false)?)?;
    frame(&mut ui);
    // Describing again keeps following the name and adopts the slots.
    ui.reconcile_children(&root, vec![describe(parameter.clone())])?;
    frame(&mut ui);
    assert_eq!(name(&ui, "control").as_deref(), Some("Bypass all"));

    let mut reference = runtime()?;
    let (fresh, _host) = reference.parameter(
        ParameterId::new(1, 1, "bypass")?,
        ParameterSpec::boolean("Bypass all", false)?,
    )?;
    reference.mount_tree(Element::new("root", Column::new([describe(fresh)])))?;
    frame(&mut reference);
    assert_eq!(ui.paint()?, reference.paint()?);
    Ok(())
}

#[test]
fn a_label_replaces_the_parameter_name_and_can_follow_a_binding() -> TestResult {
    let mut ui = runtime()?;
    let (parameter, host) = bypass(&mut ui)?;
    let translated = ui.signal("Umgehen".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "control",
            Toggle::for_parameter(parameter.clone()).label(translated.read_only()),
        )]),
    ))?;
    frame(&mut ui);
    assert_eq!(name(&ui, "control").as_deref(), Some("Umgehen"));
    host.replace_spec(ParameterSpec::boolean("Bypass all", false)?)?;
    frame(&mut ui);
    assert_eq!(
        name(&ui, "control").as_deref(),
        Some("Umgehen"),
        "the specification no longer names it"
    );
    translated.set("Contourner".to_owned())?;
    frame(&mut ui);
    assert_eq!(name(&ui, "control").as_deref(), Some("Contourner"));

    // Without the override the control follows the name again.
    ui.reconcile_children(
        &root,
        vec![Element::new("control", Toggle::for_parameter(parameter))],
    )?;
    frame(&mut ui);
    assert_eq!(name(&ui, "control").as_deref(), Some("Bypass all"));
    Ok(())
}

#[test]
fn an_inline_toggle_name_overrides_the_parameter_and_moves_its_glyph() -> TestResult {
    let mut ui = runtime()?;
    let (parameter, host) = bypass(&mut ui)?;
    let local = ui.signal(false);
    let bound = ui.signal("Mute".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("named", InlineToggle::for_parameter(parameter).name("Thru")),
            Element::new("bound", InlineToggle::new(local, bound.read_only())),
        ]),
    ))?;
    frame(&mut ui);
    assert_eq!(text_property(&ui, "named", "text"), "Thru");
    assert_eq!(text_property(&ui, "named", "glyph"), "T");
    host.replace_spec(ParameterSpec::boolean("Bypass all", false)?)?;
    frame(&mut ui);
    assert_eq!(text_property(&ui, "named", "text"), "Thru");

    assert_eq!(text_property(&ui, "bound", "text"), "Mute");
    assert_eq!(text_property(&ui, "bound", "glyph"), "M");
    bound.set("Solo".to_owned())?;
    settle(&mut ui);
    assert_eq!(text_property(&ui, "bound", "text"), "Solo");
    assert_eq!(text_property(&ui, "bound", "glyph"), "S");
    Ok(())
}

#[test]
fn a_new_binding_shows_nothing_until_it_is_read() -> TestResult {
    let mut ui = runtime()?;
    let first = ui.signal("First".to_owned());
    let second = ui.signal("Second".to_owned());
    let local = ui.signal(false);
    let describe = |text: TextSource, name: TextSource| {
        vec![
            Element::new("label", Label::new(text.clone())),
            Element::new("chip", Chip::new(text)),
            Element::new("toggle", InlineToggle::new(local.clone(), name)),
        ]
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new(describe(first.read_only().into(), "Fixed".into())),
    ))?;
    frame(&mut ui);
    assert_eq!(name(&ui, "label").as_deref(), Some("First"));
    ui.reconcile_children(
        &root,
        describe(second.read_only().into(), second.read_only().into()),
    )?;
    // The old text never stands in for a binding that has not been read.
    assert_eq!(text_property(&ui, "label", "text"), "");
    assert_eq!(text_property(&ui, "chip", "text"), "");
    assert_eq!(text_property(&ui, "toggle", "text"), "");
    frame(&mut ui);
    assert_eq!(text_property(&ui, "label", "text"), "Second");
    assert_eq!(text_property(&ui, "chip", "text"), "Second");
    assert_eq!(text_property(&ui, "toggle", "text"), "Second");
    assert_eq!(text_property(&ui, "toggle", "glyph"), "S");
    Ok(())
}

#[test]
fn a_menu_row_described_with_the_same_binding_keeps_its_text_and_press() -> TestResult {
    let mut ui = runtime()?;
    let label = ui.signal("Copy".to_owned());
    let open = ui.signal(false);
    let ran = std::rc::Rc::new(std::cell::Cell::new(0));
    let describe = |label: ReadSignal<String>| {
        let ran = ran.clone();
        Menu::with_state(
            open.clone(),
            "Edit",
            vec![MenuItem::new("copy", label, move |_| {
                ran.set(ran.get() + 1);
                Ok(())
            })],
        )
        .map(|menu| Element::new("menu", menu))
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([describe(label.read_only())?]),
    ))?;
    frame(&mut ui);
    ui.focus(&support::widget(&ui, &root, "trigger"))?;
    support::key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    label.set("Kopiera".to_owned())?;
    frame(&mut ui);
    let rows = |ui: &Runtime| -> Vec<String> {
        ui.inspect()
            .components
            .into_iter()
            .filter(|c| c.visible)
            .filter_map(|c| c.details.semantics)
            .flat_map(|s| s.items)
            .filter_map(|item| item.name)
            .collect()
    };
    assert_eq!(rows(&ui), ["Kopiera"]);
    // A key press on the highlighted row survives describing the menu again.
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.reconcile_children(&root, vec![describe(label.read_only())?])?;
    assert_eq!(rows(&ui), ["Kopiera"], "the read text is kept");
    frame(&mut ui);
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    frame(&mut ui);
    assert_eq!(rows(&ui).len(), 0, "choosing the row closed the menu");
    assert_eq!(ran.get(), 1, "the press was not cancelled");
    Ok(())
}

#[test]
fn an_empty_plain_label_takes_no_height_until_it_has_text() -> TestResult {
    let mut ui = runtime()?;
    let message = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("message", Label::new(message.read_only())),
            Element::new("icon", Label::new("").leading_icon(Icon::Info)),
        ]),
    ))?;
    frame(&mut ui);
    let bounds = |ui: &Runtime, key: &str| {
        let id = support::widget(ui, &root, key);
        ui.bounds(&id).unwrap().unwrap()
    };
    assert_eq!(
        bounds(&ui, "message").height(),
        0.0,
        "empty text takes no space"
    );
    assert!(bounds(&ui, "icon").height() > 0.0, "an icon is content");
    let issues = ui.validation_report().issues;
    assert!(issues.is_empty(), "{issues:?}");
    message.set("Saved".into())?;
    frame(&mut ui);
    assert!(
        bounds(&ui, "message").height() > 0.0,
        "text takes its line again"
    );
    Ok(())
}
