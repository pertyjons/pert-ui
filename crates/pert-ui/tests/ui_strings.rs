//! The library's own strings come from a replaceable table, and an opt-in
//! locale helper formats and parses numbers.
use pert_ui::*;
use std::time::Duration;

mod support;
use support::settle;

/// Every text a component reports: its `text` property, semantic name,
/// description and dismiss label.
fn reported(ui: &Runtime) -> Vec<String> {
    let mut out = Vec::new();
    for component in ui.inspect().components {
        let details = component.details;
        for key in ["text", "dismiss_label"] {
            if let Some(InspectValue::Text(text)) = details.properties.get(key) {
                out.push(text.clone());
            }
        }
        if let Some(semantics) = details.semantics {
            out.extend(semantics.name);
            out.extend(semantics.description);
        }
    }
    out
}

fn swedish() -> UiStrings {
    UiStrings::english()
        .with(StringRole::SelectPlaceholder, "Välj…")
        .and_then(|s| s.with(StringRole::BreadcrumbName, "Sökväg"))
        .and_then(|s| s.with(StringRole::RemoveItem, "Ta bort {item}"))
        .and_then(|s| s.with(StringRole::ToastDismiss, "Stäng"))
        .and_then(|s| s.with(StringRole::HistoryName, "Historik"))
        .and_then(|s| s.with(StringRole::HistoryBack, "Bakåt"))
        .and_then(|s| s.with(StringRole::BackTo, "Tillbaka till {place}"))
        .and_then(|s| s.with(StringRole::WindowClose, "Stäng fönstret"))
        .and_then(|s| s.with(StringRole::AtLeastCharacters, "Minst {count} tecken"))
        .unwrap()
}

#[test]
fn installing_a_table_relabels_built_in_components_without_polling() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 700.0, 1.0).unwrap())
        .unwrap();
    let unit = ui.signal("none");
    let queue = ToastQueue::new(&ui);
    let history = NavigationHistory::new(&ui).unwrap();
    history.visit("a", "Alfa").unwrap();
    history.visit("b", "Beta").unwrap();
    let window = FloatingWindow::new(
        &ui,
        "mixer",
        "Mixer",
        Element::new("body", Label::new("Kanaler")),
    );
    let content = Column::new([
        Element::new(
            "unit",
            Select::new(&ui, unit, [Choice::new("ms", "Millisekunder")]).unwrap(),
        ),
        Element::new(
            "path",
            Breadcrumb::new(
                vec![
                    BreadcrumbSegment::new("root", "Rot"),
                    BreadcrumbSegment::new("here", "Här"),
                ],
                |_, _| Ok(()),
            )
            .unwrap(),
        ),
        Element::new("chip", Chip::removable("Bas", |_| Ok(()))),
        Element::new(
            "history",
            HistoryNavigator::new(&ui, history, |_, _| Ok(())).unwrap(),
        ),
        Element::new("windows", WindowLayer::new([window])),
    ]);
    ui.mount_tree(Element::new(
        "host",
        ToastHost::new(queue.clone(), Element::new("content", content)),
    ))
    .unwrap();
    let _toast = queue.push(Toast::new("Sparat")).unwrap();
    settle(&mut ui);
    let before = reported(&ui);
    for english in [
        "Select…",
        "Breadcrumb",
        "Remove Bas",
        "Dismiss",
        "Navigation history",
        "Back",
        "Close window",
    ] {
        assert!(
            before.iter().any(|t| t == english),
            "{english} in {before:?}"
        );
    }

    assert!(ui.ui_settings().set_strings(swedish()).unwrap());
    settle(&mut ui);
    let after = reported(&ui);
    for swedish in [
        "Välj…",
        "Sökväg",
        "Ta bort Bas",
        "Stäng",
        "Historik",
        "Bakåt",
        "Stäng fönstret",
    ] {
        assert!(after.iter().any(|t| t == swedish), "{swedish} in {after:?}");
    }
    for english in ["Select…", "Breadcrumb", "Remove Bas", "Navigation history"] {
        assert!(
            !after.iter().any(|t| t == english),
            "{english} left in {after:?}"
        );
    }
    // Installing the same table again changes nothing, and nothing polls.
    assert!(!ui.ui_settings().set_strings(swedish()).unwrap());
    assert_eq!(ui.ui_settings().strings().unwrap(), swedish());

    // An application's own text wins over the table.
    let mut ui = Runtime::new();
    let unit = ui.signal("none");
    ui.mount_tree(Element::new(
        "unit",
        Select::new(&ui, unit, [Choice::new("ms", "Millisekunder")])
            .unwrap()
            .placeholder("Pick a unit"),
    ))
    .unwrap();
    ui.ui_settings().set_strings(swedish()).unwrap();
    settle(&mut ui);
    let texts = reported(&ui);
    assert!(texts.iter().any(|t| t == "Pick a unit"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Välj…"), "{texts:?}");
    assert!(ui.next_deadline().is_none());
}

#[test]
fn length_refusals_and_menu_rows_follow_the_table() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let name = ui.signal(String::new());
    let refusal = ui.signal(None::<String>);
    let report = refusal.clone();
    let field = ui
        .mount_tree(Element::new(
            "field",
            TextField::new(name.clone())
                .min_length(Some(3))
                .on_rejection(move |_, rejection| {
                    report
                        .set(rejection.map(|r| r.message().to_owned()))
                        .map(|_| ())
                }),
        ))
        .unwrap();
    ui.ui_settings().set_strings(swedish()).unwrap();
    ui.focus(&field).unwrap();
    settle(&mut ui);
    ui.text_input("ab").unwrap();
    support::press(&mut ui, KeyCode::Enter);
    assert_eq!(refusal.get().unwrap().as_deref(), Some("Minst 3 tecken"));
    assert_eq!(name.get().unwrap(), "");

    // A menu row bound to a label follows it while the menu is retained.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let label = ui.signal("Copy".to_owned());
    let open = ui.signal(false);
    let menu = Menu::with_state(
        open.clone(),
        "Edit",
        vec![MenuItem::new("copy", label.read_only(), |_| Ok(()))],
    )
    .unwrap();
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("menu", menu)]),
    ))
    .unwrap();
    open.set(true).unwrap();
    settle(&mut ui);
    let rows = |ui: &Runtime| -> Vec<String> {
        ui.inspect()
            .components
            .into_iter()
            .filter_map(|c| c.details.semantics)
            .flat_map(|s| s.items)
            .filter_map(|item| item.name)
            .collect()
    };
    assert!(rows(&ui).contains(&"Copy".to_owned()), "{:?}", rows(&ui));
    label.set("Kopiera".to_owned()).unwrap();
    settle(&mut ui);
    assert!(rows(&ui).contains(&"Kopiera".to_owned()), "{:?}", rows(&ui));
    assert!(!rows(&ui).contains(&"Copy".to_owned()));
    assert!(ui.next_deadline().is_none() || ui.next_deadline() > Some(Duration::ZERO));
}

#[test]
fn replacements_are_validated_against_their_role() {
    let english = UiStrings::english();
    assert_eq!(english.get(StringRole::TextCopy), "Copy");
    assert_eq!(
        english.format(UiMessage::AtMostCharacters { count: 8 }),
        "Character count must be at most 8"
    );
    assert_eq!(
        UiStrings::english().with(StringRole::TextCopy, "  "),
        Err(UiStringError::Blank(StringRole::TextCopy))
    );
    assert_eq!(
        UiStrings::english().with(StringRole::TextCopy, "Ko\npiera"),
        Err(UiStringError::ControlCharacter(StringRole::TextCopy))
    );
    assert_eq!(
        UiStrings::english().with(StringRole::BackTo, "Tillbaka"),
        Err(UiStringError::Placeholders(StringRole::BackTo))
    );
    assert_eq!(
        UiStrings::english().with(StringRole::BackTo, "Till {where}"),
        Err(UiStringError::Placeholders(StringRole::BackTo))
    );
    assert_eq!(
        UiStrings::english().with(StringRole::TextCopy, "x".repeat(UiStrings::MAX_LENGTH + 1)),
        Err(UiStringError::TooLong(StringRole::TextCopy))
    );
    let braces = UiStrings::english()
        .with(StringRole::BackTo, "{{{place}}}")
        .unwrap();
    assert_eq!(braces.format(UiMessage::BackTo { place: "x" }), "{x}");
    // Every role has a default and lists its placeholders.
    for role in StringRole::ALL {
        assert!(!english.get(*role).trim().is_empty(), "{role:?}");
    }
}

#[test]
fn a_locale_format_edits_numbers_with_its_separators() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(1234.5_f64);
    let swedish = NumberLocale::new(',', Some(' ')).unwrap();
    let field = ui
        .mount_tree(Element::new(
            "field",
            InlineDecimal::new(value.clone())
                .range(-1.0e6, 1.0e6)
                .unwrap()
                .format(NumberFormat::localized_decimal(
                    swedish,
                    Some(2),
                    "-999 999,99",
                )),
        ))
        .unwrap();
    settle(&mut ui);
    let shown = reported(&ui);
    // Editing text is the shortest exact value in the locale.
    assert!(shown.iter().any(|t| t == "1 234,5"), "{shown:?}");
    ui.focus(&field).unwrap();
    settle(&mut ui);
    support::press(&mut ui, KeyCode::Enter);
    ui.text_input("").unwrap();
    // Replace the draft with locale text and save it.
    let modifier = KeyCode::primary();
    ui.key_input(modifier, KeyAction::Press).unwrap();
    support::key(&mut ui, KeyCode::KeyA);
    ui.key_input(modifier, KeyAction::Release).unwrap();
    ui.text_input("-2 000,25").unwrap();
    support::press(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), -2000.25);
    assert_eq!(
        NumberLocale::default().format_decimal(1234.5, None),
        "1234.5"
    );
}

#[test]
fn an_open_picker_follows_a_new_table() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 500.0, 1.0).unwrap())
        .unwrap();
    let controller = QuickPickController::new(&ui);
    let picker = QuickPick::new(
        controller.clone(),
        Element::new("body", Label::new("Body")),
        |_, _| Ok(PickChoice::Done),
    );
    ui.mount_tree(Element::new("picker", picker)).unwrap();
    settle(&mut ui);
    controller.open(vec![PickItem::new("a", "Alpha")]).unwrap();
    settle(&mut ui);
    let placeholders = |ui: &Runtime| -> Vec<String> {
        ui.inspect()
            .components
            .into_iter()
            .filter(|component| component.visible)
            .filter_map(
                |component| match component.details.properties.get("placeholder") {
                    Some(InspectValue::Text(text)) => Some(text.clone()),
                    _ => None,
                },
            )
            .collect()
    };
    let before = placeholders(&ui);
    assert!(before.iter().any(|text| text == "Search"), "{before:?}");
    let table = UiStrings::english()
        .with(StringRole::SearchPlaceholder, "Sök")
        .unwrap();
    ui.ui_settings().set_strings(table).unwrap();
    settle(&mut ui);
    let after = placeholders(&ui);
    assert!(after.iter().any(|text| text == "Sök"), "{after:?}");
}

#[test]
fn tool_windows_and_filters_follow_a_new_table() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let layout = ToolWindowLayout::new(Element::new("main", Label::new("Main")))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(
            ToolWindow::new(
                "files",
                "Files",
                Element::new("files-body", Label::new("Body")),
            )
            .expanded(true),
        );
    let filter = CollectionFilter::new(
        &ui,
        ListFilter::new(
            &ui,
            ui.signal(ListSource::new(Words(vec!["a"]))).read_only(),
        )
        .unwrap(),
        Element::new("list", Label::new("List")),
    );
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("layout", layout).with_label("Layout"),
            Element::new("filter-demo", filter),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let names = reported(&ui);
    for text in ["Hide Files", "Files options", "Left tool windows", "Filter"] {
        assert!(names.iter().any(|name| name == text), "{text} in {names:?}");
    }
    let table = UiStrings::english()
        .with(StringRole::ToolWindowHide, "Dölj {title}")
        .and_then(|t| t.with(StringRole::ToolWindowOptions, "Alternativ för {title}"))
        .and_then(|t| t.with(StringRole::ToolWindowStripeLeft, "Vänster"))
        .and_then(|t| t.with(StringRole::FilterName, "Filtrera"))
        .unwrap();
    ui.ui_settings().set_strings(table).unwrap();
    settle(&mut ui);
    let names = reported(&ui);
    for text in ["Dölj Files", "Alternativ för Files", "Vänster", "Filtrera"] {
        assert!(names.iter().any(|name| name == text), "{text} in {names:?}");
    }
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
fn a_color_picker_swatch_without_a_name_follows_a_new_table() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 500.0, 1.0).unwrap())
        .unwrap();
    let color = ui.signal(Color::from_hex("#2090a0").unwrap());
    let named = ui.signal(Color::from_hex("#a02090").unwrap());
    ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new("plain", ColorPicker::new(color).into_popover()),
            Element::new(
                "named",
                ColorPicker::new(named).name("Track color").into_popover(),
            ),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let swatch_names = |ui: &Runtime| -> Vec<String> {
        ui.inspect()
            .components
            .into_iter()
            .filter(|component| component.key == "swatch")
            .filter_map(|component| component.details.semantics?.name)
            .collect()
    };
    assert_eq!(swatch_names(&ui), ["Color", "Track color"]);
    let table = UiStrings::english()
        .with(StringRole::ColorName, "Färg")
        .unwrap();
    ui.ui_settings().set_strings(table).unwrap();
    settle(&mut ui);
    // The application's name wins; the default follows the table.
    assert_eq!(swatch_names(&ui), ["Färg", "Track color"]);
}

#[test]
fn default_length_refusals_are_count_neutral_and_replacements_keep_the_count() {
    let english = UiStrings::english();
    for count in [1, 2] {
        assert_eq!(
            english.format(UiMessage::AtLeastCharacters { count }),
            format!("Character count must be at least {count}")
        );
        assert_eq!(
            english.format(UiMessage::AtMostCharacters { count }),
            format!("Character count must be at most {count}")
        );
    }
    let translated = english
        .with(StringRole::AtLeastCharacters, "Minst {count} tecken")
        .unwrap()
        .with(StringRole::AtMostCharacters, "Högst {count} tecken")
        .unwrap();
    for count in [1, 2] {
        assert_eq!(
            translated.format(UiMessage::AtLeastCharacters { count }),
            format!("Minst {count} tecken")
        );
        assert_eq!(
            translated.format(UiMessage::AtMostCharacters { count }),
            format!("Högst {count} tecken")
        );
    }
}
