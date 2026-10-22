//! Placeholders as `TextSource`s through the public facade: fixed text, a
//! binding the control follows, a string-table role following a replaced
//! table, the library default following a replaced table, and redescription
//! with the same source keeping the text already read.
use crate::support;
use pert_ui::*;
use support::{frame, key};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 400.0, 1.0)?)?;
    Ok(ui)
}

/// A text property of the visible component `key`.
fn property(ui: &Runtime, key: &str, name: &str) -> Option<String> {
    ui.inspect()
        .components
        .into_iter()
        .filter(|c| c.key == key && c.visible)
        .find_map(|c| match c.details.properties.get(name) {
            Some(InspectValue::Text(text)) => Some(text.clone()),
            _ => None,
        })
}

fn install(ui: &Runtime, role: StringRole, text: &str) -> TestResult {
    ui.ui_settings()
        .set_strings(UiStrings::english().with_text(role, text)?)?;
    Ok(())
}

/// One editor kind, described with a placeholder source.
#[derive(Clone, Copy, Debug)]
enum Editor {
    Field,
    Area,
    Search,
}

fn editor(kind: Editor, value: Signal<String>, placeholder: TextSource) -> Element {
    match kind {
        Editor::Field => Element::new("editor", TextField::new(value).placeholder(placeholder)),
        Editor::Area => Element::new("editor", TextArea::new(value).placeholder(placeholder)),
        Editor::Search => Element::new("editor", SearchField::new(value).placeholder(placeholder)),
    }
}

#[test]
fn editor_placeholders_take_fixed_bound_and_role_text() -> TestResult {
    for kind in [Editor::Field, Editor::Area, Editor::Search] {
        let mut ui = runtime()?;
        let value = ui.signal(String::new());
        let hint = ui.signal("Name".to_owned());
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([editor(kind, value.clone(), "Fixed".into())]),
        ))?;
        frame(&mut ui);
        assert_eq!(
            property(&ui, "editor", "placeholder").as_deref(),
            Some("Fixed"),
            "{kind:?}"
        );

        // A binding is followed.
        ui.reconcile_children(
            &root,
            vec![editor(kind, value.clone(), hint.read_only().into())],
        )?;
        frame(&mut ui);
        assert_eq!(
            property(&ui, "editor", "placeholder").as_deref(),
            Some("Name"),
            "{kind:?}"
        );
        hint.set("Namn".to_owned())?;
        frame(&mut ui);
        assert_eq!(
            property(&ui, "editor", "placeholder").as_deref(),
            Some("Namn"),
            "{kind:?}"
        );
        // The same binding keeps the read text at once.
        ui.reconcile_children(
            &root,
            vec![editor(kind, value.clone(), hint.read_only().into())],
        )?;
        assert_eq!(
            property(&ui, "editor", "placeholder").as_deref(),
            Some("Namn"),
            "{kind:?}"
        );

        // A role follows a replaced table.
        let role = StringRole::SearchPlaceholder;
        ui.reconcile_children(&root, vec![editor(kind, value.clone(), role.into())])?;
        frame(&mut ui);
        assert_eq!(
            property(&ui, "editor", "placeholder").as_deref(),
            Some("Search"),
            "{kind:?}"
        );
        install(&ui, role, "Sök")?;
        frame(&mut ui);
        assert_eq!(
            property(&ui, "editor", "placeholder").as_deref(),
            Some("Sök"),
            "{kind:?}"
        );
    }
    Ok(())
}

fn options() -> Vec<Choice<&'static str>> {
    vec![Choice::new("a", "Alpha"), Choice::new("b", "Beta")]
}

/// A select over `options` whose value matches none of them.
fn select(
    ui: &Runtime,
    inline: bool,
    placeholder: Option<TextSource>,
) -> Result<Element, ChoiceError> {
    let value = ui.signal("none");
    Ok(if inline {
        let control = InlineSelect::new(ui, value, options())?;
        let control = match placeholder {
            Some(placeholder) => control.placeholder(placeholder),
            None => control,
        };
        Element::new("chooser", control)
    } else {
        let control = Select::new(ui, value, options())?;
        let control = match placeholder {
            Some(placeholder) => control.placeholder(placeholder),
            None => control,
        };
        Element::new("chooser", control)
    })
}

#[test]
fn select_placeholders_take_fixed_bound_and_role_text_and_follow_the_table() -> TestResult {
    for inline in [false, true] {
        let mut ui = runtime()?;
        // The default follows a replaced table.
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([select(&ui, inline, None)?]),
        ))?;
        frame(&mut ui);
        let shown = |ui: &Runtime| property(ui, "select", "text");
        assert_eq!(shown(&ui).as_deref(), Some("Select…"), "inline {inline}");
        install(&ui, StringRole::SelectPlaceholder, "Välj…")?;
        frame(&mut ui);
        assert_eq!(shown(&ui).as_deref(), Some("Välj…"), "inline {inline}");

        // Fixed text.
        ui.reconcile_children(&root, vec![select(&ui, inline, Some("Pick one".into()))?])?;
        frame(&mut ui);
        assert_eq!(shown(&ui).as_deref(), Some("Pick one"), "inline {inline}");

        // A binding, kept across redescription with the same binding.
        let hint = ui.signal("Choose".to_owned());
        ui.reconcile_children(
            &root,
            vec![select(&ui, inline, Some(hint.read_only().into()))?],
        )?;
        frame(&mut ui);
        assert_eq!(shown(&ui).as_deref(), Some("Choose"), "inline {inline}");
        hint.set("Wähle".to_owned())?;
        frame(&mut ui);
        assert_eq!(shown(&ui).as_deref(), Some("Wähle"), "inline {inline}");
        ui.reconcile_children(
            &root,
            vec![select(&ui, inline, Some(hint.read_only().into()))?],
        )?;
        assert_eq!(shown(&ui).as_deref(), Some("Wähle"), "inline {inline}");

        // An explicit role follows the table.
        ui.reconcile_children(
            &root,
            vec![select(
                &ui,
                inline,
                Some(StringRole::SearchPlaceholder.into()),
            )?],
        )?;
        frame(&mut ui);
        assert_eq!(shown(&ui).as_deref(), Some("Search"), "inline {inline}");
        install(&ui, StringRole::SearchPlaceholder, "Sök")?;
        frame(&mut ui);
        assert_eq!(shown(&ui).as_deref(), Some("Sök"), "inline {inline}");
    }
    Ok(())
}

fn chooser(
    ui: &Runtime,
    value: &Signal<Vec<&'static str>>,
    placeholder: Option<TextSource>,
    search: Option<TextSource>,
) -> Result<Element, ChoiceError> {
    let mut control = MultiSelect::new(ui, value.clone(), options())?;
    if let Some(placeholder) = placeholder {
        control = control.placeholder(placeholder);
    }
    if let Some(search) = search {
        control = control.search_placeholder(search);
    }
    Ok(Element::new("tags", control))
}

#[test]
fn multi_select_placeholders_take_text_sources_and_follow_the_table() -> TestResult {
    let mut ui = runtime()?;
    let value = ui.signal(Vec::new());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([chooser(&ui, &value, None, None)?]),
    ))?;
    frame(&mut ui);
    let trigger = |ui: &Runtime| property(ui, "anchor", "placeholder");
    let search = |ui: &Runtime| property(ui, "search", "placeholder");
    assert_eq!(trigger(&ui).as_deref(), Some("Select…"));
    install(&ui, StringRole::SelectPlaceholder, "Välj…")?;
    frame(&mut ui);
    assert_eq!(trigger(&ui).as_deref(), Some("Välj…"));

    // The popup's search field takes the table's default.
    let anchor = support::widget(&ui, &root, "anchor");
    ui.focus(&anchor)?;
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(search(&ui).as_deref(), Some("Search"));
    install(&ui, StringRole::SearchPlaceholder, "Sök")?;
    frame(&mut ui);
    assert_eq!(search(&ui).as_deref(), Some("Sök"));

    // Bound placeholders, kept across redescription with the same binding.
    let hint = ui.signal("Tags".to_owned());
    let find = ui.signal("Find a tag".to_owned());
    let describe = |ui: &Runtime| {
        chooser(
            ui,
            &value,
            Some(hint.read_only().into()),
            Some(find.read_only().into()),
        )
    };
    ui.reconcile_children(&root, vec![describe(&ui)?])?;
    frame(&mut ui);
    assert_eq!(trigger(&ui).as_deref(), Some("Tags"));
    assert_eq!(search(&ui).as_deref(), Some("Find a tag"));
    hint.set("Etiketter".to_owned())?;
    find.set("Hitta".to_owned())?;
    frame(&mut ui);
    assert_eq!(trigger(&ui).as_deref(), Some("Etiketter"));
    assert_eq!(search(&ui).as_deref(), Some("Hitta"));
    ui.reconcile_children(&root, vec![describe(&ui)?])?;
    assert_eq!(trigger(&ui).as_deref(), Some("Etiketter"));
    assert_eq!(search(&ui).as_deref(), Some("Hitta"));

    // Fixed text.
    ui.reconcile_children(
        &root,
        vec![chooser(
            &ui,
            &value,
            Some("Pick".into()),
            Some("Filter".into()),
        )?],
    )?;
    frame(&mut ui);
    assert_eq!(trigger(&ui).as_deref(), Some("Pick"));
    assert_eq!(search(&ui).as_deref(), Some("Filter"));
    Ok(())
}
