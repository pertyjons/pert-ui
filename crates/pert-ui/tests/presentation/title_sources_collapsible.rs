//! `TextSource` titles of `Collapsible` and labels of `BreadcrumbSegment`
//! through the public facade: a binding is followed without redescription,
//! the same binding keeps its text across redescription, switching between
//! fixed and bound text applies at once, icon slots stay, a blank bound read
//! keeps the last title and fails the update with `LayoutError::BlankLabel`,
//! and the breadcrumb's overflow menu follows a bound label.
use crate::support;
use pert_ui::*;
use support::{find, frame, press, settle, widget};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime(width: f32) -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 240.0, 1.0)?)?;
    Ok(ui)
}

fn name(ui: &Runtime, key: &str) -> Option<String> {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .and_then(|c| c.details.semantics)
        .and_then(|s| s.name)
}

fn section(title: impl Into<TextSource>) -> Element {
    Element::new(
        "section",
        Collapsible::new(title, Element::new("body", Label::new("Body")))
            .leading_icon(Icon::Folder),
    )
}

#[test]
fn a_bound_collapsible_title_follows_its_binding_and_keeps_its_icon() -> TestResult {
    let mut ui = runtime(320.0)?;
    let title = ui.signal("Mix".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([section(title.read_only())]),
    ))?;
    frame(&mut ui);
    assert_eq!(name(&ui, "section").as_deref(), Some("Mix"));
    title.set("Mischen".to_owned())?;
    frame(&mut ui);
    assert_eq!(name(&ui, "section").as_deref(), Some("Mischen"));

    let mut reference = runtime(320.0)?;
    reference.mount_tree(Element::new("root", Column::new([section("Mischen")])))?;
    frame(&mut reference);
    assert_eq!(ui.paint()?, reference.paint()?, "the icon slot stays");
    Ok(())
}

#[test]
fn a_collapsible_keeps_or_replaces_its_title_on_redescription() -> TestResult {
    let mut ui = runtime(320.0)?;
    let title = ui.signal("Mix".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([section(title.read_only())]),
    ))?;
    frame(&mut ui);

    // The same binding keeps the text it has read.
    ui.reconcile_children(&root, vec![section(title.read_only())])?;
    assert_eq!(name(&ui, "section").as_deref(), Some("Mix"));
    frame(&mut ui);

    // Fixed text applies at once and the old binding no longer does.
    ui.reconcile_children(&root, vec![section("Fixed")])?;
    assert_eq!(name(&ui, "section").as_deref(), Some("Fixed"));
    title.set("Ignored".to_owned())?;
    frame(&mut ui);
    assert_eq!(name(&ui, "section").as_deref(), Some("Fixed"));

    // A new binding is empty until read, then shows its text.
    let other = ui.signal("Other".to_owned());
    ui.reconcile_children(&root, vec![section(other.read_only())])?;
    assert_eq!(name(&ui, "section").unwrap_or_default(), "", "not yet read");
    frame(&mut ui);
    assert_eq!(name(&ui, "section").as_deref(), Some("Other"));
    Ok(())
}

#[test]
fn a_blank_bound_collapsible_title_keeps_the_last_title_and_fails_the_update() -> TestResult {
    let mut ui = runtime(320.0)?;
    let title = ui.signal("Mix".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([section(title.read_only())]),
    ))?;
    frame(&mut ui);
    title.set("  ".to_owned())?;
    assert_eq!(ui.flush(ui.now()), Err(LayoutError::BlankLabel.into()));
    assert_eq!(name(&ui, "section").as_deref(), Some("Mix"));
    title.set("Mastering".to_owned())?;
    frame(&mut ui);
    assert_eq!(name(&ui, "section").as_deref(), Some("Mastering"));

    // A blank fixed title is still refused when it is described.
    let mut fixed = runtime(320.0)?;
    assert!(fixed.mount_tree(section(" ")).is_err());
    Ok(())
}

const PATH: [&str; 5] = [
    "/",
    "/home",
    "/home/user",
    "/home/user/music",
    "/home/user/music/project",
];

fn breadcrumb(user: &Signal<String>) -> Result<Breadcrumb, LayoutError> {
    Breadcrumb::new(
        PATH.iter().map(|key| match *key {
            "/" => BreadcrumbSegment::new(*key, "Root"),
            "/home/user" => BreadcrumbSegment::new(*key, user.read_only()),
            _ => BreadcrumbSegment::new(*key, key.rsplit('/').next().unwrap_or_default()),
        }),
        |_, _| Ok(()),
    )
}

#[test]
fn a_bound_breadcrumb_label_follows_its_binding_and_is_checked_when_read() -> TestResult {
    let mut ui = runtime(900.0)?;
    let user = ui.signal("user".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("path", breadcrumb(&user)?)]),
    ))?;
    frame(&mut ui);
    assert_eq!(name(&ui, "/home/user").as_deref(), Some("user"));
    user.set("Benutzer".to_owned())?;
    frame(&mut ui);
    assert_eq!(name(&ui, "/home/user").as_deref(), Some("Benutzer"));

    user.set(String::new())?;
    assert_eq!(ui.flush(ui.now()), Err(LayoutError::BlankLabel.into()));
    assert_eq!(name(&ui, "/home/user").as_deref(), Some("Benutzer"));

    // Only a blank fixed label is refused at construction.
    assert_eq!(
        Breadcrumb::new([BreadcrumbSegment::new("a", " ")], |_, _| Ok(())).err(),
        Some(LayoutError::BlankLabel)
    );
    let blank = ui.signal(String::new());
    assert!(
        Breadcrumb::new([BreadcrumbSegment::new("a", blank.read_only())], |_, _| Ok(
            ()
        ))
        .is_ok()
    );
    Ok(())
}

#[test]
fn an_open_overflow_menu_follows_a_bound_label() -> TestResult {
    let mut ui = runtime(900.0)?;
    let user = ui.signal("user".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("path", breadcrumb(&user)?)]),
    ))?;
    settle(&mut ui);
    let full = ui
        .bounds(&widget(&ui, &root, "path"))?
        .ok_or("no bounds")?
        .width();
    // Narrow enough that every middle segment collapses.
    ui.set_viewport(Viewport::new(full * 0.3, 240.0, 1.0)?)?;
    settle(&mut ui);
    let overflow = widget(&ui, &root, "overflow");
    let trigger = find(&ui, &overflow, "button").ok_or("no overflow button")?;
    ui.focus(&trigger)?;
    press(&mut ui, KeyCode::Enter);
    let rows = |ui: &Runtime| -> Vec<String> {
        ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "items" && c.visible)
            .and_then(|c| c.details.semantics)
            .map(|s| s.items.into_iter().filter_map(|item| item.name).collect())
            .unwrap_or_default()
    };
    assert!(rows(&ui).contains(&"user".to_owned()), "{:?}", rows(&ui));
    user.set("Benutzer".to_owned())?;
    settle(&mut ui);
    let shown = rows(&ui);
    assert!(
        shown.contains(&"Benutzer".to_owned()),
        "the open menu follows: {shown:?}"
    );
    assert!(!shown.contains(&"user".to_owned()));
    Ok(())
}
