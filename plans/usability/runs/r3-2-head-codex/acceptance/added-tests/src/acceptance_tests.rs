// Independent acceptance tests added by the verifier (acceptance copy only).
// They build the submitted `app()` headlessly and drive it only through
// pert_ui::testing::Driver input (click, key, chord, type/replace text) and
// read results through inspection (address children, semantics, focus) plus
// the app-owned `Vec` signal and the library `ListSelection` it exposes.
use super::*;
use pert_ui::testing::*;

type R = std::result::Result<(), Box<dyn std::error::Error>>;

fn ctrl_n() -> KeyChord {
    KeyChord::new(
        CommandKey::Character('n'),
        Modifiers { control: true, ..Modifiers::default() },
    )
}
fn alt(key: KeyCode) -> KeyChord {
    KeyChord::new(
        CommandKey::Named(key),
        Modifiers { alt: true, ..Modifiers::default() },
    )
}
fn rows(d: &Driver<'_>, list: &WidgetId) -> std::result::Result<Vec<String>, TestError> {
    Ok(d.address_children(list)?.into_iter().map(|c| c.segment).collect())
}
/// Row titles in display order: each mounted row's title semantics value.
fn item_names(d: &Driver<'_>, list: &WidgetId) -> std::result::Result<Vec<String>, TestError> {
    let mut out = Vec::new();
    for row in rows(d, list)? {
        let title = d.find(list, &[&row, "title"])?;
        out.push(match d.semantics(&title)?.value {
            Some(SemanticValue::Text(t)) => t,
            other => format!("{other:?}"),
        });
    }
    Ok(out)
}
/// The title of the row the list's own semantics report as selected.
fn selected_items(d: &Driver<'_>, list: &WidgetId) -> std::result::Result<Vec<String>, TestError> {
    let names = item_names(d, list)?;
    Ok(match d.semantics(list)?.value {
        Some(SemanticValue::Choice { selected: Some(i), .. }) => vec![names[i].clone()],
        _ => Vec::new(),
    })
}
/// Lets the double-click interval pass so the next click is a single click.
fn pause(d: &mut Driver<'_>) -> R {
    d.advance(std::time::Duration::from_secs(2))?;
    Ok(())
}
fn status(d: &Driver<'_>, root: &WidgetId) -> std::result::Result<String, TestError> {
    let id = d.find(root, &["status"])?;
    Ok(d.semantics(&id)?.name.unwrap_or_default())
}
fn titles(tasks: &Signal<Vec<Task>>) -> Vec<String> {
    tasks.get().unwrap().into_iter().map(|t| t.title).collect()
}
fn add_three(d: &mut Driver<'_>, root: &WidgetId) -> R {
    let field = d.find(root, &["entry", "new"])?;
    for t in ["Alpha", "Bravo", "Charlie"] {
        d.replace_text(&field, t)?;
        d.key(KeyCode::Enter)?;
    }
    Ok(())
}

/// Task check: the list is driven by the tester's Vec — changing the vector
/// (not through the UI) adds/removes/reorders rows and updates the status.
#[test]
fn acc_vector_drives_rows_and_status() -> R {
    let mut app = app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let root = state.root.clone();
    let list = d.find(&root, &["tasks", "list"])?;
    add_three(&mut d, &root)?;
    assert_eq!(item_names(&d, &list)?, ["Alpha", "Bravo", "Charlie"]);
    assert_eq!(status(&d, &root)?, "0 of 3 done");

    // Remove an item from the vector directly: its row disappears.
    state.tasks.update(|v| v.retain(|t| t.title != "Bravo"))?;
    d.settle()?;
    assert_eq!(rows(&d, &list)?, ["1", "3"]);
    assert_eq!(item_names(&d, &list)?, ["Alpha", "Charlie"]);
    assert!(d.find(&list, &["2", "title"]).is_err());
    assert_eq!(status(&d, &root)?, "0 of 2 done");

    // Push a done task and reverse the vector directly: rows follow.
    state.tasks.update(|v| {
        v.push(Task { id: 99, title: "Delta".into(), done: true });
        v.reverse();
    })?;
    d.settle()?;
    assert_eq!(rows(&d, &list)?, ["99", "3", "1"]);
    assert_eq!(item_names(&d, &list)?, ["Delta", "Charlie", "Alpha"]);
    assert_eq!(status(&d, &root)?, "1 of 3 done");
    let check = d.find(&list, &["99", "done"])?;
    assert_eq!(d.semantics(&check)?.value, Some(SemanticValue::Bool(true)));
    Ok(())
}

/// Item 1: checking, unchecking and removing a done task update the status
/// line; remove/reorder buttons and keys act on the selected task.
#[test]
fn acc_check_uncheck_remove_reorder_status() -> R {
    let mut app = app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let root = state.root.clone();
    let list = d.find(&root, &["tasks", "list"])?;
    add_three(&mut d, &root)?;

    let c1 = d.find(&list, &["1", "done"])?;
    let c3 = d.find(&list, &["3", "done"])?;
    d.click(&c1)?;
    d.click(&c3)?;
    assert_eq!(status(&d, &root)?, "2 of 3 done");
    pause(&mut d)?;
    d.click(&c1)?; // uncheck
    assert!(!state.tasks.get()?[0].done);
    assert_eq!(d.semantics(&c1)?.value, Some(SemanticValue::Bool(false)));
    assert_eq!(status(&d, &root)?, "1 of 3 done");

    // Select Charlie (done) by pointer, remove with the Remove button.
    let t3 = d.find(&list, &["3", "title"])?;
    d.click(&t3)?;
    assert_eq!(selected_items(&d, &list)?, ["Charlie"]);
    let remove = d.find(&root, &["actions", "remove"])?;
    d.click(&remove)?;
    assert_eq!(titles(&state.tasks), ["Alpha", "Bravo"]);
    assert_eq!(item_names(&d, &list)?, ["Alpha", "Bravo"]);
    assert_eq!(status(&d, &root)?, "0 of 2 done");

    // Move up/down buttons at the edges are no-ops, otherwise swap.
    let t1 = d.find(&list, &["1", "title"])?;
    d.click(&t1)?;
    let up = d.find(&root, &["actions", "up"])?;
    let down = d.find(&root, &["actions", "down"])?;
    d.click(&up)?;
    assert_eq!(titles(&state.tasks), ["Alpha", "Bravo"]);
    d.click(&down)?;
    assert_eq!(titles(&state.tasks), ["Bravo", "Alpha"]);
    assert_eq!(item_names(&d, &list)?, ["Bravo", "Alpha"]);
    d.click(&down)?;
    assert_eq!(titles(&state.tasks), ["Bravo", "Alpha"]);
    assert_eq!(state.selection.active().get()?.as_deref(), Some("1"));
    assert_eq!(status(&d, &root)?, "0 of 2 done");
    Ok(())
}

/// Single selection by pointer and arrows; selecting another clears the first.
#[test]
fn acc_single_selection_pointer_and_arrows() -> R {
    let mut app = app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let root = state.root.clone();
    let list = d.find(&root, &["tasks", "list"])?;
    add_three(&mut d, &root)?;
    let t1 = d.find(&list, &["1", "title"])?;
    let t3 = d.find(&list, &["3", "title"])?;
    d.click(&t1)?;
    assert_eq!(selected_items(&d, &list)?, ["Alpha"]);
    d.click(&t3)?;
    assert_eq!(selected_items(&d, &list)?, ["Charlie"]);
    d.key(KeyCode::ArrowUp)?;
    assert_eq!(selected_items(&d, &list)?, ["Bravo"]);
    d.key(KeyCode::ArrowUp)?;
    d.key(KeyCode::ArrowUp)?; // clamps at first
    assert_eq!(selected_items(&d, &list)?, ["Alpha"]);
    assert_eq!(state.selection.selected().get()?.len(), 1);
    Ok(())
}

/// Item 2: Enter commits into the Vec, Escape discards (both start routes),
/// and the edited task is the one in the row (also while filtered).
#[test]
fn acc_inline_rename_enter_escape() -> R {
    let mut app = app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let root = state.root.clone();
    let list = d.find(&root, &["tasks", "list"])?;
    add_three(&mut d, &root)?;

    // Double-click starts editing in place; Enter commits.
    let t2 = d.find(&list, &["2", "title"])?;
    d.click(&t2)?;
    d.click(&t2)?;
    d.runtime_mut().text_command(&t2, TextCommand::SelectAll)?;
    d.type_text("Bravo renamed")?;
    assert_eq!(titles(&state.tasks), ["Alpha", "Bravo", "Charlie"]);
    d.key(KeyCode::Enter)?;
    assert_eq!(titles(&state.tasks), ["Alpha", "Bravo renamed", "Charlie"]);
    assert_eq!(item_names(&d, &list)?, ["Alpha", "Bravo renamed", "Charlie"]);
    assert_eq!(state.tasks.get()?.len(), 3);

    // Enter on selection starts editing; Escape discards the draft fully.
    d.key(KeyCode::Enter)?;
    d.type_text(" XXX")?;
    d.key(KeyCode::Escape)?;
    assert_eq!(titles(&state.tasks), ["Alpha", "Bravo renamed", "Charlie"]);
    assert_eq!(d.semantics(&t2)?.value, Some(SemanticValue::Text("Bravo renamed".into())));
    // A later edit starts from the committed title, not the discarded draft
    // (an edit begins with the whole title selected, so move to the end).
    d.key(KeyCode::Enter)?;
    assert_eq!(d.semantics(&t2)?.value, Some(SemanticValue::Text("Bravo renamed".into())));
    d.key(KeyCode::End)?;
    d.type_text("!")?;
    d.key(KeyCode::Enter)?;
    assert_eq!(state.tasks.get()?[1].title, "Bravo renamed!");

    // Rename while filtered edits the right task.
    let search = d.find(&root, &["tasks", "filter-field", "filter"])?;
    d.replace_text(&search, "charlie")?;
    assert_eq!(rows(&d, &list)?, ["3"]);
    let t3 = d.find(&list, &["3", "title"])?;
    d.click(&t3)?;
    d.click(&t3)?;
    d.runtime_mut().text_command(&t3, TextCommand::SelectAll)?;
    d.type_text("Charlie 2")?;
    d.key(KeyCode::Enter)?;
    assert_eq!(titles(&state.tasks), ["Alpha", "Bravo renamed!", "Charlie 2"]);
    Ok(())
}

/// Item 3: filtering is case-insensitive, hides non-matching rows, keeps a
/// visible selection, clears a hidden one so Delete/Alt+arrows cannot act on
/// a hidden task, arrows move among visible rows only, the Vec is unchanged.
#[test]
fn acc_filter_selection_and_vector() -> R {
    let mut app = app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let root = state.root.clone();
    let list = d.find(&root, &["tasks", "list"])?;
    let field = d.find(&root, &["entry", "new"])?;
    for t in ["Buy milk", "Call Bob", "MILK the cow", "Read"] {
        d.replace_text(&field, t)?;
        d.key(KeyCode::Enter)?;
    }
    let before = state.tasks.get()?;
    let search = d.find(&root, &["tasks", "filter-field", "filter"])?;

    // Select "MILK the cow" then filter "mIlK": stays visible and selected.
    let t3 = d.find(&list, &["3", "title"])?;
    d.click(&t3)?;
    d.replace_text(&search, "mIlK")?;
    assert_eq!(item_names(&d, &list)?, ["Buy milk", "MILK the cow"]);
    assert_eq!(state.selection.active().get()?.as_deref(), Some("3"));
    assert_eq!(selected_items(&d, &list)?, ["MILK the cow"]);
    assert_eq!(state.tasks.get()?, before);

    // Arrow keys move among visible rows only (3 -> 1, skipping 2).
    pause(&mut d)?;
    d.click(&t3)?;
    d.key(KeyCode::ArrowUp)?;
    assert_eq!(state.selection.active().get()?.as_deref(), Some("1"));
    d.key(KeyCode::ArrowDown)?;
    assert_eq!(state.selection.active().get()?.as_deref(), Some("3"));

    // Select "Read", then filter it out: selection cleared, and neither
    // Delete nor Alt+Down nor the buttons touch the hidden task.
    d.replace_text(&search, "")?;
    let t4 = d.find(&list, &["4", "title"])?;
    d.click(&t4)?;
    d.replace_text(&search, "milk")?;
    assert_eq!(state.selection.active().get()?, None);
    assert!(selected_items(&d, &list)?.is_empty());
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Delete)?;
    d.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    let remove = d.find(&root, &["actions", "remove"])?;
    assert!(d.semantics(&remove)?.disabled);
    assert_eq!(state.tasks.get()?, before);

    // No match: empty list, vector intact; clearing restores everything.
    d.replace_text(&search, "zzz")?;
    assert!(rows(&d, &list)?.is_empty());
    assert_eq!(status(&d, &root)?, "0 of 4 done");
    d.replace_text(&search, "")?;
    assert_eq!(rows(&d, &list)?, ["1", "2", "3", "4"]);
    assert_eq!(state.tasks.get()?, before);
    Ok(())
}

/// Item 4: Ctrl+N from several focus locations, Delete removes the selected
/// task, Alt+Up/Alt+Down move it; all through simulated key input.
#[test]
fn acc_keyboard_shortcuts() -> R {
    let mut app = app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let root = state.root.clone();
    let list = d.find(&root, &["tasks", "list"])?;
    let field = d.find(&root, &["entry", "new"])?;
    let search = d.find(&root, &["tasks", "filter-field", "filter"])?;
    add_three(&mut d, &root)?;

    // Ctrl+N from the search field and from the list.
    d.runtime_mut().focus(&search)?;
    d.chord(ctrl_n(), KeyCode::KeyN)?;
    assert_eq!(d.runtime().focused(), Some(field.clone()));
    let t1 = d.find(&list, &["1", "title"])?;
    d.click(&t1)?;
    assert_ne!(d.runtime().focused(), Some(field.clone()));
    d.chord(ctrl_n(), KeyCode::KeyN)?;
    assert_eq!(d.runtime().focused(), Some(field.clone()));
    // Ctrl+N did not insert an "n" into the field.
    assert_eq!(d.semantics(&field)?.value, Some(SemanticValue::Text(String::new())));

    // Alt+Down twice, Alt+Up once on Alpha (list focused after click).
    pause(&mut d)?;
    d.click(&t1)?;
    d.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?;
    assert_eq!(titles(&state.tasks), ["Bravo", "Alpha", "Charlie"]);
    d.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?;
    assert_eq!(titles(&state.tasks), ["Bravo", "Charlie", "Alpha"]);
    d.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?; // at bottom
    assert_eq!(titles(&state.tasks), ["Bravo", "Charlie", "Alpha"]);
    d.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    assert_eq!(titles(&state.tasks), ["Bravo", "Alpha", "Charlie"]);
    assert_eq!(item_names(&d, &list)?, ["Bravo", "Alpha", "Charlie"]);
    assert_eq!(selected_items(&d, &list)?, ["Alpha"]);

    // Delete removes the selected task (and only it).
    d.key(KeyCode::Delete)?;
    assert_eq!(titles(&state.tasks), ["Bravo", "Charlie"]);
    assert_eq!(item_names(&d, &list)?, ["Bravo", "Charlie"]);
    assert_eq!(status(&d, &root)?, "0 of 2 done");
    Ok(())
}

/// Observation (not graded): what Delete does while a text field has focus
/// and a task is selected. Prints the outcome for the evidence log.
#[test]
fn acc_observe_delete_in_text_field() -> R {
    let mut app = app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let root = state.root.clone();
    let list = d.find(&root, &["tasks", "list"])?;
    let search = d.find(&root, &["tasks", "filter-field", "filter"])?;
    add_three(&mut d, &root)?;
    let t1 = d.find(&list, &["1", "title"])?;
    d.click(&t1)?;
    d.replace_text(&search, "a")?;
    d.key(KeyCode::Home)?;
    d.key(KeyCode::Delete)?;
    println!(
        "OBSERVE delete-in-search: search={:?} tasks={:?}",
        d.semantics(&search)?.value,
        titles(&state.tasks)
    );
    // Alt+Up from a button focus (not the list).
    let t2 = d.find(&list, &["2", "title"])?;
    d.click(&t2)?;
    let down = d.find(&root, &["actions", "down"])?;
    d.click(&down)?;
    d.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    println!(
        "OBSERVE alt-up-after-button: focused_is_list={} tasks={:?}",
        d.runtime().focused() == Some(list.clone()),
        titles(&state.tasks)
    );
    Ok(())
}
