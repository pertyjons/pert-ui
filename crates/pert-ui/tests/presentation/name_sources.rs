//! Names and texts that replace library strings take a `TextSource`: fixed
//! text, a binding the component follows, or a `StringRole` that follows a
//! replaced string table, with the library's default following the table too
//! and the same source kept across redescription. Covers QuickPick,
//! CommandPalette, ColorSwatch, ColorPicker, HistoryNavigator and
//! CollectionFilter.
use crate::support;
use pert_ui::*;
use support::settle;

/// Every visible semantic name, `placeholder` and `text` property.
fn reported(ui: &Runtime) -> Vec<String> {
    let mut out = Vec::new();
    for component in ui.inspect().components {
        if !component.visible {
            continue;
        }
        let details = component.details;
        for key in ["text", "placeholder"] {
            if let Some(InspectValue::Text(text)) = details.properties.get(key) {
                out.push(text.clone());
            }
        }
        if let Some(semantics) = details.semantics {
            out.extend(semantics.name);
        }
    }
    out
}

fn semantic_name(ui: &Runtime, key: &str) -> Option<String> {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key && c.visible)
        .and_then(|c| c.details.semantics)
        .and_then(|s| s.name)
}

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn install(ui: &mut Runtime, replacements: &[(StringRole, &str)]) {
    let mut table = UiStrings::english();
    for (role, text) in replacements {
        table = table.with(*role, *text).unwrap();
    }
    ui.ui_settings().set_strings(table).unwrap();
    settle(ui);
}

#[track_caller]
fn assert_reported(ui: &Runtime, text: &str) {
    let all = reported(ui);
    assert!(all.iter().any(|t| t == text), "{text} not in {all:?}");
}

#[test]
fn quick_pick_texts_follow_their_sources_and_the_table() {
    let mut ui = runtime();
    let controller = QuickPickController::new(&ui);
    let name = ui.signal("Go to".to_owned());
    let picker = QuickPick::new(
        controller.clone(),
        Element::new("body", Label::new("Body")),
        |_, _| Ok(PickChoice::Done),
    )
    .name(name.read_only())
    .list_name(StringRole::FilterName)
    .placeholder("Find a page");
    ui.mount_tree(Element::new("picker", picker)).unwrap();
    settle(&mut ui);
    controller.open(vec![PickItem::new("a", "Alpha")]).unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "picker").as_deref(), Some("Go to"));
    assert_reported(&ui, "Filter");
    assert_reported(&ui, "Find a page");

    // A binding relabels the open picker.
    name.set("Gehe zu".to_owned()).unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "picker").as_deref(), Some("Gehe zu"));

    // A role and the default empty text follow a replaced table.
    install(
        &mut ui,
        &[
            (StringRole::FilterName, "Filtrera"),
            (StringRole::NoMatches, "Inga träffar"),
        ],
    );
    assert_reported(&ui, "Filtrera");
    ui.text_input("zzz").unwrap();
    settle(&mut ui);
    assert_reported(&ui, "Inga träffar");
}

#[test]
fn command_palette_texts_take_a_text_source() {
    let mut ui = runtime();
    let palette = CommandPaletteController::new(&ui);
    let hint = ui.signal("Run".to_owned());
    ui.mount_tree(Element::new(
        "palette",
        CommandPalette::new(
            palette.clone(),
            Element::new("content", Button::new("Edit", |_| Ok(()))),
        )
        .placeholder(hint.read_only())
        .empty_text(StringRole::FilterName),
    ))
    .unwrap();
    settle(&mut ui);
    let context = ui.command_context(CommandSource::KeyBinding);
    palette.open(&mut ui, &context).unwrap();
    settle(&mut ui);
    assert_reported(&ui, "Run");
    hint.set("Ausführen".to_owned()).unwrap();
    settle(&mut ui);
    assert_reported(&ui, "Ausführen");
    ui.text_input("zzzz").unwrap();
    settle(&mut ui);
    assert_reported(&ui, "Filter");
    install(&mut ui, &[(StringRole::FilterName, "Filtrera")]);
    assert_reported(&ui, "Filtrera");
}

#[test]
fn a_swatch_name_follows_its_source_and_falls_back_to_hex() {
    let mut ui = runtime();
    let name = ui.signal("Track color".to_owned());
    let color = Color::from_hex("#2090a0").unwrap();
    let describe = |name: Option<TextSource>| {
        let swatch = ColorSwatch::new(color);
        let swatch = match name {
            Some(name) => swatch.name(name),
            None => swatch,
        };
        vec![Element::new("swatch", swatch)]
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new(describe(Some(name.read_only().into()))),
        ))
        .unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "swatch").as_deref(), Some("Track color"));
    name.set("Spurfarbe".to_owned()).unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "swatch").as_deref(), Some("Spurfarbe"));
    // The same binding keeps the read name at once.
    ui.reconcile_children(&root, describe(Some(name.read_only().into())))
        .unwrap();
    assert_eq!(semantic_name(&ui, "swatch").as_deref(), Some("Spurfarbe"));
    // An empty read and fixed empty text fall back to the hex text.
    name.set(String::new()).unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "swatch").as_deref(), Some("#2090a0"));
    ui.reconcile_children(&root, describe(Some("".into())))
        .unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "swatch").as_deref(), Some("#2090a0"));
    // A role follows a replaced table.
    ui.reconcile_children(&root, describe(Some(StringRole::ColorName.into())))
        .unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "swatch").as_deref(), Some("Color"));
    install(&mut ui, &[(StringRole::ColorName, "Färg")]);
    assert_eq!(semantic_name(&ui, "swatch").as_deref(), Some("Färg"));
}

#[test]
fn a_color_picker_name_reaches_its_swatch_and_follows_its_source() {
    let mut ui = runtime();
    let name = ui.signal("Track color".to_owned());
    let bound = ui.signal(Color::from_hex("#a02090").unwrap());
    let plain = ui.signal(Color::from_hex("#2090a0").unwrap());
    ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new(
                "named",
                ColorPicker::new(bound)
                    .name(name.read_only())
                    .into_popover(),
            ),
            Element::new("plain", ColorPicker::new(plain).into_popover()),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let swatches = |ui: &Runtime| -> Vec<String> {
        ui.inspect()
            .components
            .into_iter()
            .filter(|c| c.key == "swatch")
            .filter_map(|c| c.details.semantics?.name)
            .collect()
    };
    assert_eq!(swatches(&ui), ["Track color", "Color"]);
    name.set("Spurfarbe".to_owned()).unwrap();
    settle(&mut ui);
    install(&mut ui, &[(StringRole::ColorName, "Färg")]);
    assert_eq!(swatches(&ui), ["Spurfarbe", "Färg"]);
}

#[test]
fn a_history_navigator_name_follows_its_source_and_the_default_the_table() {
    let mut ui = runtime();
    let history = NavigationHistory::<&'static str>::new(&ui).unwrap();
    history.visit("home", "Home").unwrap();
    let name = ui.signal("Pages".to_owned());
    let describe = |ui: &Runtime, name: Option<TextSource>| {
        let navigator = HistoryNavigator::new(ui, history.clone(), |_, _| Ok(())).unwrap();
        let navigator = match name {
            Some(name) => navigator.name(name),
            None => navigator,
        };
        vec![Element::new("history", navigator)]
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new(describe(&ui, Some(name.read_only().into()))),
        ))
        .unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "history").as_deref(), Some("Pages"));
    name.set("Seiten".to_owned()).unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "history").as_deref(), Some("Seiten"));
    let again = describe(&ui, Some(name.read_only().into()));
    ui.reconcile_children(&root, again).unwrap();
    assert_eq!(semantic_name(&ui, "history").as_deref(), Some("Seiten"));
    // Without a name the default role follows a replaced table.
    let default = describe(&ui, None);
    ui.reconcile_children(&root, default).unwrap();
    settle(&mut ui);
    assert_eq!(
        semantic_name(&ui, "history").as_deref(),
        Some("Navigation history")
    );
    install(&mut ui, &[(StringRole::HistoryName, "Verlauf")]);
    assert_eq!(semantic_name(&ui, "history").as_deref(), Some("Verlauf"));
}

struct Words(Vec<&'static str>);
impl ListItems for Words {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|word| (*word).to_owned())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let word = *self.0.get(index)?;
        Some(Element::new(word, Label::new(word)))
    }
    fn text(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|word| (*word).to_owned())
    }
}

#[test]
fn a_collection_filter_follows_its_name_and_placeholder_sources() {
    let mut ui = runtime();
    let source = ui.signal(ListSource::new(Words(vec!["a"]))).read_only();
    let target = ListFilter::new(&ui, source).unwrap();
    let query = ui.signal(String::new());
    let name = ui.signal("Find tracks".to_owned());
    let describe = |name: TextSource, placeholder: Option<TextSource>| {
        let filter = CollectionFilter::with_query(
            query.clone(),
            target.clone(),
            Element::new("list", Label::new("List")),
        )
        .name(name);
        let filter = match placeholder {
            Some(placeholder) => filter.placeholder(placeholder),
            None => filter,
        };
        vec![Element::new("filter-demo", filter)]
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new(describe(name.read_only().into(), None)),
        ))
        .unwrap();
    settle(&mut ui);
    assert_eq!(
        semantic_name(&ui, "filter-demo").as_deref(),
        Some("Find tracks")
    );
    assert_reported(&ui, "Find tracks");
    name.set("Spuren suchen".to_owned()).unwrap();
    settle(&mut ui);
    assert_eq!(
        semantic_name(&ui, "filter-demo").as_deref(),
        Some("Spuren suchen")
    );
    assert_reported(&ui, "Spuren suchen");

    // Redescription applies a new placeholder and a role name.
    ui.reconcile_children(
        &root,
        describe(StringRole::FilterName.into(), Some("Type to filter".into())),
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(semantic_name(&ui, "filter-demo").as_deref(), Some("Filter"));
    assert_reported(&ui, "Type to filter");
    install(&mut ui, &[(StringRole::FilterName, "Filtrera")]);
    assert_eq!(
        semantic_name(&ui, "filter-demo").as_deref(),
        Some("Filtrera")
    );
}

#[test]
fn a_library_default_names_the_picker_before_the_first_update()
-> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let strings = UiStrings::english().with(StringRole::PickerName, "Befehle")?;
    ui.ui_settings().set_strings(strings)?;
    let controller = QuickPickController::new(&ui);
    let picker = QuickPick::new(
        controller,
        Element::new("anchor", Label::new("A")),
        |_, _| Ok(PickChoice::Done),
    );
    ui.mount_tree(Element::new("picker", picker))?;
    // Inspected before any flush, the default already uses the installed table.
    let name = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "picker")
        .and_then(|c| c.details.semantics)
        .and_then(|s| s.name);
    assert_eq!(name.as_deref(), Some("Befehle"));
    Ok(())
}
