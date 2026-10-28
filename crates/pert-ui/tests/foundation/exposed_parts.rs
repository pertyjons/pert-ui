//! Public composite parts are scoped metadata, independent of address paths.
use pert_ui::prelude::*;
use pert_ui::testing::{Driver, TestError};
use pert_ui::{Detail, InspectionQuery, RuntimeError, Selector};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn leaf(key: &str, name: &str) -> Element {
    Label::new(key).key(key).expose_part(name)
}

fn assert_clean(ui: &Runtime, ids: &[&WidgetId]) {
    let snapshot = ui.inspect();
    for id in ids {
        let serial = ui.inspection_id(id).expect("mounted identity");
        let entry = snapshot
            .components
            .iter()
            .find(|entry| entry.id == serial)
            .expect("component record");
        assert_eq!(entry.dirty, None, "{} was invalidated", entry.key);
    }
}

#[test]
fn scopes_forward_internal_structure_and_stop_at_application_children_and_parts() -> TestResult {
    let mut ui = Runtime::new();
    let root = ui.mount_tree(
        Column::new([
            Container::new(leaf("button", "literal/a"))
                .key("wrapper")
                .structural(),
            Container::new(leaf("nested", "inside")).key("application"),
            Container::new(leaf("nested", "inside"))
                .key("panel")
                .expose_part("panel"),
        ])
        .key("root"),
    )?;
    let wrapper_child = ui.find(&root, &["button"])?.ok_or("no forwarded address")?;
    let panel = ui.find(&root, &["panel"])?.ok_or("no panel address")?;
    let driver = Driver::new(&mut ui);
    let parts = driver.exposed_parts(&root)?;
    assert_eq!(
        parts
            .iter()
            .map(|part| part.name.as_str())
            .collect::<Vec<_>>(),
        ["literal/a", "panel"]
    );
    assert_eq!(driver.exposed_part(&root, "literal/a")?, wrapper_child);
    assert!(
        matches!(driver.exposed_part(&root, "inside"), Err(TestError::ExposedPartNotFound { available, .. }) if available == ["literal/a", "panel"])
    );
    assert_eq!(driver.exposed_parts(&panel)?.len(), 1);
    assert_eq!(
        driver.exposed_part(&panel, "inside")?,
        driver.find(&panel, &["nested"])?
    );
    Ok(())
}

#[test]
fn duplicates_are_enumerated_but_never_silently_selected() -> TestResult {
    let mut ui = Runtime::new();
    let root =
        ui.mount_tree(Column::new([leaf("first", "same"), leaf("second", "same")]).key("root"))?;
    assert_eq!(Driver::new(&mut ui).exposed_parts(&root)?.len(), 2);
    assert_eq!(
        ui.find_exposed_part(&root, "same"),
        Err(RuntimeError::AmbiguousExposedPart {
            owner: ui.inspection_id(&root)?,
            name: "same".into(),
        })
    );
    Ok(())
}

#[test]
fn names_validate_before_adoption_and_owner_errors_take_priority() -> TestResult {
    let mut ui = Runtime::new();
    assert_eq!(
        ui.mount_tree(leaf("invalid", " \t")).err(),
        Some(RuntimeError::EmptyExposedPartName)
    );
    assert!(ui.inspect().components.is_empty());
    let root = ui.mount_tree(Column::new([leaf("child", "valid")]).key("root"))?;
    assert_eq!(
        ui.find_exposed_part(&root, " "),
        Err(RuntimeError::EmptyExposedPartName)
    );
    let mut other = Runtime::new();
    let foreign = other.mount_tree(Label::new("Other").key("root"))?;
    assert_eq!(
        ui.find_exposed_part(&foreign, " "),
        Err(RuntimeError::ForeignRuntime)
    );
    assert_eq!(
        ui.reconcile_children(&root, vec![leaf("child", " ")]),
        Err(RuntimeError::EmptyExposedPartName)
    );
    assert!(ui.find_exposed_part(&root, "valid")?.is_some());
    ui.unmount(&root)?;
    assert_eq!(
        ui.find_exposed_part(&root, " "),
        Err(RuntimeError::Unmounted)
    );
    Ok(())
}

#[test]
fn names_replace_and_remove_without_remounting_or_scheduling_layout() -> TestResult {
    use pert_ui::testing::HostIntegration;
    let mut ui = Runtime::new();
    let root = ui.mount_tree(Column::new([leaf("child", "first")]).key("root"))?;
    let child = ui.find_exposed_part(&root, "first")?.ok_or("no part")?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.runtime_mut().paint()?;
    driver
        .runtime_mut()
        .reconcile_children(&root, vec![leaf("child", "second")])?;
    assert_eq!(driver.exposed_part(&root, "second")?, child);
    assert_eq!(driver.runtime().find_exposed_part(&root, "first")?, None);
    assert_clean(driver.runtime(), &[&root, &child]);
    assert!(!driver.runtime().needs_paint());
    assert_eq!(driver.runtime().next_deadline(), None);
    assert!(driver.quiet_for(Duration::from_millis(10))?.is_idle());
    driver
        .runtime_mut()
        .reconcile_children(&root, vec![Label::new("child").key("child")])?;
    assert!(driver.exposed_parts(&root)?.is_empty());
    assert_eq!(driver.find(&root, &["child"])?, child);
    assert_clean(driver.runtime(), &[&root, &child]);
    assert!(!driver.runtime().needs_paint());
    assert_eq!(driver.runtime().next_deadline(), None);
    assert!(driver.quiet_for(Duration::from_millis(10))?.is_idle());
    Ok(())
}

#[test]
fn select_part_metadata_keeps_field_naming_and_application_addresses() -> TestResult {
    let mut ui = Runtime::new();
    let value = ui.signal(0_u8);
    let root = ui.mount_tree(
        Field::new(
            Select::new(&ui, value, [Choice::new(0, "A"), Choice::new(1, "B")])?.key("mode"),
        )
        .label("Mode")
        .key("field"),
    )?;
    ui.flush(Duration::ZERO)?;
    let mode = ui.find(&root, &["mode"])?.ok_or("mode address")?;
    let control = ui.find_exposed_part(&mode, "control")?.ok_or("control")?;
    let inspected = ui.query(
        &InspectionQuery::components(Selector::id(ui.inspection_id(&control)?))
            .detail(Detail::Semantics),
    );
    let component = inspected.components.first().ok_or("inspection")?;
    assert_eq!(component.exposed_part.as_deref(), Some("control"));
    assert_eq!(
        component
            .details
            .semantics
            .as_ref()
            .and_then(|s| s.name.as_deref()),
        Some("Mode")
    );
    assert_eq!(component.labelled_by, Some(ui.inspection_id(&root)?));
    assert_eq!(ui.find(&mode, &["control"])?, None);
    assert!(ui.find_exposed_part(&mode, "options")?.is_some());
    Ok(())
}

#[test]
fn built_in_internal_composition_does_not_leak_clipboard_parts() -> TestResult {
    use pert_ui::{PickChoice, PickItem, QuickPick, QuickPickController};
    let mut ui = Runtime::new();
    let query = ui.signal(String::new());
    let search = ui.mount_tree(SearchField::new(query).key("plain"))?;
    assert_eq!(
        Driver::new(&mut ui)
            .exposed_parts(&search)?
            .iter()
            .map(|part| part.name.as_str())
            .collect::<Vec<_>>(),
        ["clipboard"]
    );
    // QuickPick builds its search during update; the search is a scope of
    // its own, even though its layout element is structural.
    let controller = QuickPickController::new(&ui);
    let pick = ui.mount_tree(
        QuickPick::new(
            controller.clone(),
            Label::new("Content").key("content"),
            |_, _| Ok(PickChoice::Done),
        )
        .key("pick"),
    )?;
    ui.flush(Duration::ZERO)?;
    controller.open(vec![PickItem::new("a", "A")])?;
    ui.flush(Duration::ZERO)?;
    let driver = Driver::new(&mut ui);
    assert!(
        driver.exposed_parts(&pick)?.is_empty(),
        "the existing surface address is an application-child boundary"
    );
    let surface = driver.find(&pick, &["surface"])?;
    assert_eq!(
        driver
            .exposed_parts(&surface)?
            .iter()
            .map(|part| part.name.as_str())
            .collect::<Vec<_>>(),
        ["search"]
    );
    let search = driver.exposed_part(&surface, "search")?;
    assert!(driver.exposed_part(&search, "clipboard").is_ok());
    Ok(())
}

#[test]
fn a_part_in_another_window_can_be_found_but_input_requires_that_window() -> TestResult {
    use pert_ui::testing::HostIntegration;
    use pert_ui::windowing::*;
    let mut ui = Runtime::new();
    let tabs = ui.mount_tree(
        Tabs::new([Tab::new(
            "view",
            "View",
            Container::new(leaf("button", "action")).key("panel"),
        )])
        .key("tabs"),
    )?;
    ui.flush(Duration::ZERO)?;
    let panel = ui.find(&tabs, &["view", "panel"])?.ok_or("panel")?;
    let view = ui.find(&tabs, &["view"])?.ok_or("view")?;
    let primary = ui.primary_window();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("View")?)?;
    let window = ui.commit_detach(prepared)?;
    ui.select_window(&primary)?;
    let mut driver = Driver::new(&mut ui);
    let part = driver.exposed_part(&panel, "action")?;
    assert_eq!(driver.runtime().window_of(&part)?, window);
    assert!(matches!(driver.click(&part), Err(TestError::OtherWindow { key }) if key == "button"));
    assert_eq!(
        driver
            .exposed_parts(&panel)?
            .first()
            .map(|part| part.id.clone()),
        Some(part)
    );
    Ok(())
}

#[test]
fn exposed_menu_names_do_not_collide_with_application_keys_or_workspace_paths() -> TestResult {
    use pert_ui::{ContextMenu, MenuBar, MenuItem};
    fn tabs(key: &str) -> Element {
        Tabs::new([Tab::new("one", "One", Label::new("Body").key("body"))]).key(key)
    }
    let mut ui = Runtime::new();
    let context = ContextMenu::new(
        &ui,
        tabs("items"),
        vec![MenuItem::new("run", "Run", |_| Ok(()))],
    )?
    .key("context");
    let bar = MenuBar::new(
        &ui,
        vec![MenuItem::submenu(
            "file",
            "File",
            vec![MenuItem::new("run", "Run", |_| Ok(()))],
        )],
    )?
    .content(tabs("title/file"))
    .leading(tabs("leading-tabs"))
    .trailing(tabs("trailing-tabs"))
    .key("bar");
    let root = ui.mount_tree(Column::new([context, bar]).key("root"))?;
    ui.flush(Duration::ZERO)?;
    let context = ui.find(&root, &["context"])?.ok_or("context")?;
    let content = ui.find(&context, &["items"])?.ok_or("app items")?;
    let list = ui
        .find_exposed_part(&context, "items")?
        .ok_or("menu items")?;
    assert_ne!(content, list);
    let bar = ui.find(&root, &["bar"])?.ok_or("bar")?;
    let content = ui.find(&bar, &["title/file"])?.ok_or("app title/file")?;
    let title = ui.find_exposed_part(&bar, "title/file")?.ok_or("title")?;
    assert_ne!(content, title);
    let workspace = ui.capture_workspace()?;
    let paths = workspace
        .entries()
        .map(|(path, _)| path.to_string())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"root/context/items".into()), "{paths:?}");
    assert!(paths.contains(&"root/bar/title\\/file".into()), "{paths:?}");
    assert!(paths.contains(&"root/bar/leading-tabs".into()), "{paths:?}");
    assert!(
        paths.contains(&"root/bar/trailing-tabs".into()),
        "{paths:?}"
    );
    assert!(ui.restore_workspace(&workspace)?.is_complete());
    assert_eq!(ui.capture_workspace()?, workspace);
    Ok(())
}

#[test]
fn multi_select_search_bounds_its_clipboard_scope() -> TestResult {
    let mut ui = Runtime::new();
    let values = ui.signal(vec![0_u8]);
    let root = ui.mount_tree(
        Column::new([
            MultiSelect::new(&ui, values, [Choice::new(0, "A"), Choice::new(1, "B")])?
                .key("chooser"),
        ])
        .key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    let chooser = ui.find(&root, &["chooser"])?.ok_or("chooser")?;
    let trigger = ui.find_descendant(&chooser, "anchor")?.ok_or("anchor")?;
    ui.focus(&trigger)?;
    ui.key_input(pert_ui::KeyCode::Enter, pert_ui::KeyAction::Press)?;
    ui.key_input(pert_ui::KeyCode::Enter, pert_ui::KeyAction::Release)?;
    ui.flush(ui.now())?;
    let driver = Driver::new(&mut ui);
    assert_eq!(
        driver
            .exposed_parts(&chooser)?
            .iter()
            .map(|part| part.name.as_str())
            .collect::<Vec<_>>(),
        ["search"]
    );
    let search = driver.exposed_part(&chooser, "search")?;
    assert!(driver.exposed_part(&search, "clipboard").is_ok());
    Ok(())
}

#[test]
fn workspace_parts_and_slots_forward_names_without_becoming_exposed_scopes() -> TestResult {
    let mut ui = Runtime::new();
    let root = ui.mount_tree(
        Column::new([
            Container::new(leaf("first", "first-action"))
                .key("part-wrapper")
                .part("saved"),
            Container::new(leaf("second", "second-action"))
                .key("slot-wrapper")
                .slot("slot"),
        ])
        .key("root"),
    )?;
    let driver = Driver::new(&mut ui);
    assert_eq!(
        driver
            .exposed_parts(&root)?
            .iter()
            .map(|part| part.name.as_str())
            .collect::<Vec<_>>(),
        ["first-action", "second-action"]
    );
    assert!(driver.find(&root, &["saved"]).is_ok());
    assert!(driver.find(&root, &["slot"]).is_ok());
    assert_eq!(
        driver
            .runtime()
            .key(&driver.exposed_part(&root, "first-action")?)?,
        "first"
    );
    assert_eq!(
        driver
            .runtime()
            .key(&driver.exposed_part(&root, "second-action")?)?,
        "second"
    );
    Ok(())
}

#[test]
fn clipboard_list_is_nested_and_the_palette_forwards_the_existing_surface_scope() -> TestResult {
    use pert_ui::CommandSource;
    use pert_ui::commands::{CommandPalette, CommandPaletteController};
    let mut ui = Runtime::new();
    let controller = CommandPaletteController::new(&ui);
    let palette = ui.mount_tree(
        CommandPalette::new(controller.clone(), Label::new("Content").key("content"))
            .key("palette"),
    )?;
    ui.flush(Duration::ZERO)?;
    let context = ui.command_context(CommandSource::Programmatic);
    controller.open(&mut ui, &context)?;
    ui.flush(ui.now())?;
    let driver = Driver::new(&mut ui);
    assert!(driver.exposed_parts(&palette)?.is_empty());
    let surface = driver.find(&palette, &["surface"])?;
    let search = driver.exposed_part(&surface, "search")?;
    let clipboard = driver.exposed_part(&search, "clipboard")?;
    let rows = driver.exposed_part(&clipboard, "items")?;
    assert_eq!(driver.semantics(&rows)?.role, pert_ui::SemanticRole::Menu);
    assert!(
        driver
            .semantics(&rows)?
            .items
            .iter()
            .all(|item| item.bounds.is_none())
    );
    Ok(())
}
