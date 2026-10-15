//! The gallery's navigation history: visits from page changes and Go to
//! commands, and back, forward, the history list, the mouse's side buttons
//! and the shortcuts showing a place again without recording it.
//!
//! The tests move with the Go to commands, each exactly one visit; opening a
//! page through the keyboard would also visit every tab the arrows pass.
use super::support::*;
use pert_ui::*;

/// The category and page the gallery shows, from the strips' selections.
fn shown(ui: &Runtime) -> (String, String) {
    let components = ui.inspect().components;
    let selected =
        |component: &ComponentInspection| match component.details.properties.get("selected") {
            Some(InspectValue::Text(key)) => Some(key.clone()),
            _ => None,
        };
    let category = components
        .iter()
        .find(|component| component.key == "gallery-tabs")
        .and_then(selected)
        .expect("the top strip selects a category");
    let page = components
        .iter()
        .find(|component| {
            component.key == "category-pages"
                && component
                    .path
                    .as_ref()
                    .is_some_and(|path| path.contains(&category))
        })
        .and_then(selected)
        .expect("the category's strip selects a page");
    (category, page)
}

fn place(category: &str, page: &str) -> (String, String) {
    (category.to_owned(), page.to_owned())
}

/// A part of the gallery's own history control in the menu bar.
fn part(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    let control = find(ui, root, "gallery-history");
    find(ui, &control, key)
}

/// Whether a part of the menu bar's history control is disabled.
fn disabled(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| {
            component.key == key
                && component
                    .path
                    .as_ref()
                    .is_some_and(|path| path.iter().any(|segment| segment == "gallery-history"))
        })
        .and_then(|component| component.details.semantics)
        .expect("the part reports semantics")
        .disabled
}

/// Clicks a widget with the primary pointer button at its center.
fn pointer_click_on(ui: &mut Runtime, id: &WidgetId) {
    let (x, y) = center_of(ui, id);
    pointer_click(ui, Point::new(x, y).unwrap());
    settle(ui);
}

fn invoke(ui: &mut Runtime, name: &str) {
    let context = ui.command_context(CommandSource::Programmatic);
    let entry = ui
        .commands_for(&context)
        .unwrap()
        .into_iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("no gallery command {name}"));
    assert_eq!(
        ui.invoke_command(&entry.id, context).unwrap(),
        CommandOutcome::Invoked
    );
    settle(ui);
}

/// The strip of pages holding `page`.
fn strip_of(ui: &Runtime, root: &WidgetId, page: &str) -> WidgetId {
    pages(ui, root)
        .into_iter()
        .find(|entry| entry.3 == page)
        .unwrap_or_else(|| panic!("no gallery page {page}"))
        .1
}

#[test]
fn page_changes_are_visits_and_back_and_forward_cross_categories() {
    let (mut ui, root) = mounted();
    let start = shown(&ui);
    assert!(disabled(&ui, "back"), "nothing to go back to yet");
    invoke(&mut ui, "Go to: Controls");
    invoke(&mut ui, "Go to: Text input");
    // The user's own tab selection is a visit too.
    let strip = strip_of(&ui, &root, "editing");
    ui.focus(&strip).unwrap();
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(shown(&ui), place("text-category", "numbers"));
    let back = part(&ui, &root, "back");
    pointer_click_on(&mut ui, &back);
    assert_eq!(shown(&ui), place("text-category", "editing"));
    assert_eq!(text(&ui, "status"), ["Returned to Text › Text input."]);
    pointer_click_on(&mut ui, &back);
    assert_eq!(shown(&ui), place("controls-category", "controls"));
    // The restorations are reveals, not visits: forward still leads on.
    assert!(!disabled(&ui, "forward"));
    let forward = part(&ui, &root, "forward");
    pointer_click_on(&mut ui, &forward);
    pointer_click_on(&mut ui, &forward);
    assert_eq!(shown(&ui), place("text-category", "numbers"));
    assert!(disabled(&ui, "forward"));
    // Going all the way back reaches the page the gallery opened on.
    while !disabled(&ui, "back") {
        pointer_click_on(&mut ui, &back);
    }
    assert_eq!(shown(&ui), start);
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_history_list_searches_and_returns_to_a_place() {
    let (mut ui, root) = mounted();
    invoke(&mut ui, "Go to: Chips & breadcrumbs");
    invoke(&mut ui, "Go to: Text input");
    invoke(&mut ui, "Go to: Themes");
    let list = part(&ui, &root, "history");
    pointer_click_on(&mut ui, &list);
    ui.text_input("chips").unwrap();
    settle(&mut ui);
    // Enter arrives from inside a flush, as a native check's driver delivers
    // it, so the list closes before the restoration's task is delivered.
    let _enter = ui
        .defer(|ui| {
            ui.key_input(KeyCode::Enter, KeyAction::Press)?;
            ui.key_input(KeyCode::Enter, KeyAction::Release)?;
            Ok(())
        })
        .unwrap();
    settle(&mut ui);
    // The choice queued the restoration during that flush; the next one
    // delivers it.
    settle(&mut ui);
    assert_eq!(shown(&ui), place("controls-category", "chips"));
    // Going to an entry moves the history there without a new branch, so the
    // later places are still ahead.
    assert!(!disabled(&ui, "forward"));
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_mouse_side_buttons_move_through_the_history() {
    let (mut ui, root) = mounted();
    invoke(&mut ui, "Go to: Chips & breadcrumbs");
    invoke(&mut ui, "Go to: Text input");
    let status = find(&ui, &root, "status");
    let (x, y) = center_of(&ui, &status);
    for button in [
        PointerButton::Back,
        PointerButton::Forward,
        PointerButton::Back,
    ] {
        pointer_at(&mut ui, x, y, PointerKind::Press(button));
        pointer_at(&mut ui, x, y, PointerKind::Release(button));
    }
    assert_eq!(shown(&ui), place("controls-category", "chips"));
}

#[test]
fn the_shortcut_commands_move_through_the_history() {
    let (mut ui, _) = mounted();
    let context = ui.command_context(CommandSource::Programmatic);
    let commands = ui.commands_for(&context).unwrap();
    let shortcuts = |name: &str| {
        commands
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("no gallery command {name}"))
            .shortcuts
            .clone()
    };
    assert_eq!(shortcuts("Go back"), [KeyChord::navigate_back()]);
    assert_eq!(shortcuts("Go forward"), [KeyChord::navigate_forward()]);
    invoke(&mut ui, "Go to: Chips & breadcrumbs");
    invoke(&mut ui, "Go to: Themes");
    // The chords travel the native key path, as a window's keys do.
    navigation_chord(&mut ui, true);
    assert_eq!(shown(&ui), place("controls-category", "chips"));
    navigation_chord(&mut ui, false);
    assert_eq!(shown(&ui), place("tools-category", "theme"));
}

/// Presses the platform's back or forward chord through
/// `Runtime::command_key_input`.
fn navigation_chord(ui: &mut Runtime, back: bool) {
    let (modifier, code, base) = if cfg!(target_os = "macos") {
        let (code, bracket) = if back {
            (KeyCode::BracketLeft, '[')
        } else {
            (KeyCode::BracketRight, ']')
        };
        (KeyCode::primary(), code, CommandKey::Character(bracket))
    } else {
        let code = if back {
            KeyCode::ArrowLeft
        } else {
            KeyCode::ArrowRight
        };
        (KeyCode::AltLeft, code, CommandKey::Named(code))
    };
    let mut key = |code, action, base| {
        ui.command_key_input(CommandKeyInput::new(code, action, base))
            .unwrap();
    };
    key(modifier, KeyAction::Press, CommandKey::Named(modifier));
    key(code, KeyAction::Press, base);
    key(code, KeyAction::Release, base);
    key(modifier, KeyAction::Release, CommandKey::Named(modifier));
    settle(ui);
}

#[test]
fn restoring_through_a_category_showing_another_page_records_nothing() {
    let (mut ui, _) = mounted();
    invoke(&mut ui, "Go to: Controls");
    invoke(&mut ui, "Go to: Text input");
    // The Controls category now shows Chips, not Controls.
    invoke(&mut ui, "Go to: Chips & breadcrumbs");
    invoke(&mut ui, "Go back");
    assert_eq!(shown(&ui), place("text-category", "editing"));
    // Revealing the Controls category passes through Chips on the way back
    // to Controls; neither step is a visit.
    invoke(&mut ui, "Go back");
    assert_eq!(shown(&ui), place("controls-category", "controls"));
    invoke(&mut ui, "Go forward");
    assert_eq!(shown(&ui), place("text-category", "editing"));
    invoke(&mut ui, "Go forward");
    assert_eq!(shown(&ui), place("controls-category", "chips"));
}
