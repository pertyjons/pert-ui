//! Builders that replace a library label take a `TextSource`: fixed text, a
//! binding the label follows, or a string-table role that follows
//! `UiSettings::set_strings`. Without one, the library default follows a
//! replaced table, and describing the component again with the same binding
//! keeps the text it has read. Covers `ToastHost::dismiss_label`,
//! `Toolbar::more_label` and `Chip::remove_name`.
use crate::support;
use pert_ui::*;
use support::frame;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 300.0, 1.0)?)?;
    Ok(ui)
}

fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("no component {key}"))
}

/// A table that renames `role` to `text`.
fn table(role: StringRole, text: &str) -> Result<UiStrings, Box<dyn std::error::Error>> {
    Ok(UiStrings::english().with_text(role, text)?)
}

// ToastHost::dismiss_label

fn dismiss_label(ui: &Runtime) -> String {
    match component(ui, "toasts")
        .details
        .properties
        .get("dismiss_label")
    {
        Some(InspectValue::Text(text)) => text.clone(),
        other => panic!("unexpected dismiss_label {other:?}"),
    }
}

fn host(queue: &ToastController, label: Option<TextSource>) -> Element {
    let host = ToastHost::new(queue.clone(), Element::new("body", Label::new("Body")));
    Element::new(
        "host",
        match label {
            Some(label) => host.dismiss_label(label),
            None => host,
        },
    )
}

#[test]
fn a_toast_dismiss_label_takes_fixed_bound_role_and_default_text() -> TestResult {
    let mut ui = runtime()?;
    let queue = ToastController::new(&ui);
    let bound = ui.signal("Stäng".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([host(&queue, Some("Close it".into()))]),
    ))?;
    frame(&mut ui);
    assert_eq!(dismiss_label(&ui), "Close it");

    // A binding is followed without describing the host again, and the same
    // binding keeps the text it has read.
    ui.reconcile_children(&root, vec![host(&queue, Some(bound.read_only().into()))])?;
    frame(&mut ui);
    assert_eq!(dismiss_label(&ui), "Stäng");
    bound.set("Avvisa".to_owned())?;
    frame(&mut ui);
    assert_eq!(dismiss_label(&ui), "Avvisa");
    ui.reconcile_children(&root, vec![host(&queue, Some(bound.read_only().into()))])?;
    assert_eq!(dismiss_label(&ui), "Avvisa", "kept before the next update");

    // A role follows the installed table.
    ui.reconcile_children(
        &root,
        vec![host(&queue, Some(StringRole::WindowClose.into()))],
    )?;
    frame(&mut ui);
    assert_eq!(dismiss_label(&ui), "Close window");
    ui.ui_settings()
        .set_strings(table(StringRole::WindowClose, "Fönster stängs")?)?;
    frame(&mut ui);
    assert_eq!(dismiss_label(&ui), "Fönster stängs");

    // Without a label the library default follows the table too.
    ui.reconcile_children(&root, vec![host(&queue, None)])?;
    frame(&mut ui);
    assert_eq!(dismiss_label(&ui), "Dismiss");
    ui.ui_settings()
        .set_strings(table(StringRole::ToastDismiss, "Ta bort")?)?;
    frame(&mut ui);
    assert_eq!(dismiss_label(&ui), "Ta bort");
    Ok(())
}

// Toolbar::more_label

/// The overflow button's name and its tooltip's text.
fn more_names(ui: &Runtime) -> (Option<String>, InspectValue) {
    let components = ui.inspect().components;
    let name = components
        .iter()
        .find(|c| c.key == "button")
        .and_then(|c| c.details.semantics.as_ref())
        .and_then(|s| s.name.clone());
    let help = components
        .iter()
        .find(|c| c.key == "help")
        .and_then(|c| c.details.properties.get("text").cloned())
        .unwrap_or(InspectValue::Text(String::new()));
    (name, help)
}

fn tool(key: &str, icon: Icon) -> Element {
    Element::new(key, Button::icon(icon, key, |_| Ok(())).frameless())
}

/// A toolbar too wide for the narrow viewport, so its overflow button shows.
fn toolbar(ui: &Runtime, label: Option<TextSource>) -> Result<Element, RuntimeError> {
    let toolbar = Toolbar::new(
        ui,
        [
            ToolGroup::new(
                "transport",
                [
                    tool("play", Icon::PlayArrow),
                    tool("stop", Icon::Stop),
                    tool("record", Icon::Record),
                ],
            )
            .essential(),
            ToolGroup::new("edit", [tool("undo", Icon::Undo), tool("redo", Icon::Redo)]),
            ToolGroup::new("tempo", [tool("timer", Icon::Timer)]),
        ],
    )?;
    Ok(Element::new(
        "toolbar",
        match label {
            Some(label) => toolbar.more_label(label),
            None => toolbar,
        },
    ))
}

fn named(text: &str) -> (Option<String>, InspectValue) {
    (Some(text.to_owned()), InspectValue::Text(text.to_owned()))
}

#[test]
fn a_toolbar_more_label_takes_fixed_bound_role_and_default_text() -> TestResult {
    let mut ui = runtime()?;
    ui.set_viewport(Viewport::new(160.0, 300.0, 1.0)?)?;
    let bound = ui.signal("Mer".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([toolbar(&ui, Some("More tools".into()))?]),
    ))?;
    frame(&mut ui);
    assert_eq!(more_names(&ui), named("More tools"));

    ui.reconcile_children(&root, vec![toolbar(&ui, Some(bound.read_only().into()))?])?;
    frame(&mut ui);
    // A bound or role label is read where it shows: the button's name. The
    // tooltip's help is hidden until it shows, so it reads its text then.
    assert_eq!(more_names(&ui).0.as_deref(), Some("Mer"));
    bound.set("Fler".to_owned())?;
    frame(&mut ui);
    assert_eq!(more_names(&ui).0.as_deref(), Some("Fler"));
    ui.reconcile_children(&root, vec![toolbar(&ui, Some(bound.read_only().into()))?])?;
    frame(&mut ui);
    assert_eq!(
        more_names(&ui).0.as_deref(),
        Some("Fler"),
        "the same binding keeps its text"
    );

    ui.reconcile_children(
        &root,
        vec![toolbar(&ui, Some(StringRole::TextCopy.into()))?],
    )?;
    frame(&mut ui);
    assert_eq!(more_names(&ui).0.as_deref(), Some("Copy"));
    ui.ui_settings()
        .set_strings(table(StringRole::TextCopy, "Kopiera")?)?;
    frame(&mut ui);
    assert_eq!(more_names(&ui).0.as_deref(), Some("Kopiera"));

    ui.reconcile_children(&root, vec![toolbar(&ui, None)?])?;
    frame(&mut ui);
    assert_eq!(more_names(&ui), named("More"));
    ui.ui_settings()
        .set_strings(table(StringRole::ToolbarMore, "Mera")?)?;
    frame(&mut ui);
    assert_eq!(more_names(&ui).0.as_deref(), Some("Mera"));
    Ok(())
}

// Chip::remove_name

fn remove_name(ui: &Runtime) -> Option<String> {
    component(ui, "chip")
        .details
        .semantics
        .and_then(|s| s.description)
}

fn chip(name: Option<TextSource>) -> Element {
    let chip = Chip::removable("Kick", |_| Ok(()));
    Element::new(
        "chip",
        match name {
            Some(name) => chip.remove_name(name),
            None => chip,
        },
    )
}

#[test]
fn a_chip_remove_name_takes_fixed_bound_role_and_default_text() -> TestResult {
    let mut ui = runtime()?;
    let bound = ui.signal("Ta bort trumma".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([chip(Some("Drop the kick".into()))]),
    ))?;
    frame(&mut ui);
    assert_eq!(remove_name(&ui).as_deref(), Some("Drop the kick"));

    ui.reconcile_children(&root, vec![chip(Some(bound.read_only().into()))])?;
    frame(&mut ui);
    assert_eq!(remove_name(&ui).as_deref(), Some("Ta bort trumma"));
    bound.set("Släng trumman".to_owned())?;
    frame(&mut ui);
    assert_eq!(remove_name(&ui).as_deref(), Some("Släng trumman"));
    ui.reconcile_children(&root, vec![chip(Some(bound.read_only().into()))])?;
    assert_eq!(
        remove_name(&ui).as_deref(),
        Some("Släng trumman"),
        "kept before the next update"
    );

    ui.reconcile_children(&root, vec![chip(Some(StringRole::WindowClose.into()))])?;
    frame(&mut ui);
    assert_eq!(remove_name(&ui).as_deref(), Some("Close window"));
    ui.ui_settings()
        .set_strings(table(StringRole::WindowClose, "Stäng fönstret")?)?;
    frame(&mut ui);
    assert_eq!(remove_name(&ui).as_deref(), Some("Stäng fönstret"));

    // The default names the chip through the table's message.
    ui.reconcile_children(&root, vec![chip(None)])?;
    frame(&mut ui);
    assert_eq!(remove_name(&ui).as_deref(), Some("Remove Kick"));
    ui.ui_settings()
        .set_strings(table(StringRole::RemoveItem, "Ta bort {item}")?)?;
    frame(&mut ui);
    assert_eq!(remove_name(&ui).as_deref(), Some("Ta bort Kick"));
    Ok(())
}
