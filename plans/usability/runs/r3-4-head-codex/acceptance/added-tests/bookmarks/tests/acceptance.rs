//! Independent acceptance checks added by the verifier (not the tester).
//!
//! These complement tests/ui.rs where it does not itself establish a
//! README behavior's conditions: they confirm through inspection where
//! focus is before Ctrl+L (reaching buttons by Tab rather than a click),
//! read the "Clear filter" command's registration through the runtime's
//! command listing, and add a bookmark through character typing instead of
//! whole-text replacement. Only public input and inspection are used; the
//! application is unchanged.
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{CommandScope, CommandSource};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const NAME: &[&str] = &["content", "form", "name-field", "name"];
const URL: &[&str] = &["content", "form", "url-field", "url"];
const ADD: &[&str] = &["content", "add"];
const FILTER: &[&str] = &["content", "filter-field", "filter"];
const ERROR: &[&str] = &["content", "error"];
const STATUS: &[&str] = &["content", "status"];

fn mount() -> Result<(Runtime, WidgetId)> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 700.0, 1.0)?)?;
    let root = bookmarks::build(&mut ui)?;
    Driver::new(&mut ui).settle()?;
    Ok((ui, root))
}

fn find(ui: &mut Runtime, root: &WidgetId, path: &[&str]) -> Result<WidgetId> {
    Ok(Driver::new(ui).find(root, path)?)
}

fn sem(ui: &mut Runtime, root: &WidgetId, path: &[&str]) -> Result<Semantics> {
    let id = find(ui, root, path)?;
    Ok(Driver::new(ui).semantics(&id)?)
}

fn text(ui: &mut Runtime, root: &WidgetId, path: &[&str]) -> Result<String> {
    match sem(ui, root, path)?.value {
        Some(SemanticValue::Text(value)) => Ok(value),
        other => Err(format!("expected text at {path:?}, got {other:?}").into()),
    }
}

fn label(ui: &mut Runtime, root: &WidgetId, path: &[&str]) -> Result<Option<String>> {
    Ok(sem(ui, root, path)?.name)
}

fn rows(ui: &mut Runtime, root: &WidgetId) -> Result<Vec<String>> {
    let list = find(ui, root, &["content", "list"])?;
    Ok(Driver::new(ui)
        .address_children(&list)?
        .into_iter()
        .map(|row| row.segment)
        .collect())
}

/// Focuses `target` by clicking it, then types `typed` character by
/// character through text input.
fn click_and_type(ui: &mut Runtime, root: &WidgetId, path: &[&str], typed: &str) -> Result {
    let id = find(ui, root, path)?;
    let mut driver = Driver::new(ui);
    driver.click(&id)?;
    assert_eq!(
        driver.runtime().focused(),
        Some(id),
        "click focused {path:?}"
    );
    for ch in typed.chars() {
        driver.type_text(&ch.to_string())?;
    }
    Ok(())
}

/// Moves focus to `target` with the Tab key only, asserting it arrives.
fn tab_to(ui: &mut Runtime, target: &WidgetId) -> Result {
    let mut driver = Driver::new(ui);
    for _ in 0..30 {
        if driver.runtime().focused().as_ref() == Some(target) {
            return Ok(());
        }
        driver.key(KeyCode::Tab)?;
    }
    Err(format!("Tab never reached {target:?}").into())
}

#[test]
fn acceptance_clear_filter_command_is_registered_with_ctrl_l() -> Result {
    let (mut ui, _root) = mount()?;
    let context = ui.command_context(CommandSource::Programmatic);
    let entries = ui.commands_for(&context)?;
    let clear: Vec<_> = entries
        .iter()
        .filter(|e| e.name == "Clear filter")
        .collect();
    assert_eq!(clear.len(), 1, "commands: {entries:?}");
    assert_eq!(clear[0].scope, CommandScope::Application);
    assert_eq!(clear[0].shortcuts, [KeyChord::primary('l')]);
    assert_eq!(clear[0].enabled.clone()?, true);
    Ok(())
}

#[test]
fn acceptance_ctrl_l_clears_filter_from_inspected_focus() -> Result {
    let (mut ui, root) = mount()?;
    let name = find(&mut ui, &root, NAME)?;
    let url = find(&mut ui, &root, URL)?;
    let add = find(&mut ui, &root, ADD)?;
    let filter = find(&mut ui, &root, FILTER)?;

    // Text in the form, which Ctrl+L must leave alone.
    click_and_type(&mut ui, &root, NAME, "keep")?;
    click_and_type(&mut ui, &root, URL, "http://x")?;

    for target in ["filter", "name", "url", "add", "remove", "nothing"] {
        // Type the filter character by character.
        click_and_type(&mut ui, &root, FILTER, "rUst ")?;
        assert_eq!(
            rows(&mut ui, &root)?,
            ["1"],
            "filter applied before {target}"
        );
        assert_eq!(
            label(&mut ui, &root, STATUS)?.as_deref(),
            Some("1 of 2 bookmarks")
        );

        let expected = match target {
            "filter" => Some(filter.clone()),
            "name" => Some(name.clone()),
            "url" => Some(url.clone()),
            "add" => Some(add.clone()),
            "remove" => Some(find(&mut ui, &root, &["content", "list", "1", "remove"])?),
            _ => None,
        };
        match &expected {
            Some(id) => tab_to(&mut ui, id)?,
            None => Driver::new(&mut ui).clear_focus()?,
        }
        assert_eq!(ui.focused(), expected, "focus before Ctrl+L ({target})");

        Driver::new(&mut ui).chord(KeyChord::primary('l'), KeyCode::KeyL)?;

        assert_eq!(
            text(&mut ui, &root, FILTER)?,
            "",
            "filter cleared ({target})"
        );
        assert_eq!(rows(&mut ui, &root)?, ["1", "2"], "all shown ({target})");
        assert_eq!(
            label(&mut ui, &root, STATUS)?.as_deref(),
            Some("2 bookmarks")
        );
        assert_eq!(
            text(&mut ui, &root, NAME)?,
            "keep",
            "name untouched ({target})"
        );
        assert_eq!(
            text(&mut ui, &root, URL)?,
            "http://x",
            "url untouched ({target})"
        );
    }
    Ok(())
}

#[test]
fn acceptance_typed_add_appends_trims_and_clears() -> Result {
    let (mut ui, root) = mount()?;
    // A refused add first, so the success must also clear the message.
    click_and_type(&mut ui, &root, NAME, "  ")?;
    click_and_type(&mut ui, &root, URL, "https://docs.rs")?;
    let add = find(&mut ui, &root, ADD)?;
    Driver::new(&mut ui).click(&add)?;
    assert_eq!(
        label(&mut ui, &root, ERROR)?.as_deref(),
        Some("Enter a name.")
    );
    assert_eq!(text(&mut ui, &root, NAME)?, "  ");
    assert_eq!(text(&mut ui, &root, URL)?, "https://docs.rs");
    assert_eq!(rows(&mut ui, &root)?, ["1", "2"]);

    // Type the rest of the name after the two spaces already there.
    click_and_type(&mut ui, &root, NAME, "Docs ")?;
    // The click may have placed the caret anywhere; the field's text is what counts.
    let typed = text(&mut ui, &root, NAME)?;
    assert_eq!(typed.trim(), "Docs", "typed name {typed:?}");
    Driver::new(&mut ui).click(&add)?;

    assert_eq!(rows(&mut ui, &root)?, ["1", "2", "3"]);
    assert_eq!(
        label(&mut ui, &root, &["content", "list", "3", "name"])?.as_deref(),
        Some("Docs")
    );
    assert_eq!(
        label(&mut ui, &root, &["content", "list", "3", "url"])?.as_deref(),
        Some("https://docs.rs")
    );
    let remove = sem(&mut ui, &root, &["content", "list", "3", "remove"])?;
    assert_eq!(remove.role, SemanticRole::Button);
    assert_eq!(text(&mut ui, &root, NAME)?, "");
    assert_eq!(text(&mut ui, &root, URL)?, "");
    assert_eq!(sem(&mut ui, &root, ERROR)?.name, None);
    assert_eq!(
        label(&mut ui, &root, STATUS)?.as_deref(),
        Some("3 bookmarks")
    );
    Ok(())
}
