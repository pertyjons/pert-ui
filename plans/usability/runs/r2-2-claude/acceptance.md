# Acceptance: r2-2-claude (task 2, task list)

Author check, 2026-10-27, kit `kit-aca8df3` (0.3.75). The submitted `work/`
(without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-2-claude`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline` in
the kit sandbox: builds without warnings, 7 of 7 tests in `tests/headless.rs`
pass (including `capture`, a CPU PNG of the window, and
`the_desktop_app_runs_headless`, which drives the real `App` through
`App::headless`).

The tester's checks drive the application with the library's `Driver`
(`click`, `replace_text`, `type_text`, `key`, `chord`, `advance`) and read the
shown rows (title and checkbox values), the status label, the in-place
editor's value and focus, and the Remove button's disabled state from
inspection semantics; the vector, the selection and the filter counts are read
in addition. They never write the vector directly, so the task-specific check
"removing an item from the vector removes the row" was not covered, and the
filter's selection handling was not tested for a padded query or for a
selected task renamed out of the filter. The author added `tests/author.rs` in
the acceptance copy only (source below), two tests, both pass:

- `author_list_follows_the_vector`: removing an element from the `Vec`
  signal removes the rendered row and the status label follows; inserting a
  done task shows the row (checked) and "1 of 3 done".
- `author_filter_selection_stays_meaningful`: with the query `" milk "` the
  shown, selected task has Remove enabled and Delete on the list removes it;
  a selected task renamed in place (Enter, type, Enter) so that it no longer
  matches is hidden, Remove is disabled, and Delete and Alt+Up leave the
  vector unchanged; Escape in the search field shows both tasks again.

Rows are read in on-screen order (sorted by `Runtime::bounds`).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Add, remove, check, reorder update list and status | met | `src/lib.rs:125` `add` (Enter `:193`, button `:200`, blank refused), `:136` `remove_active` (button `:232`), `:40` checkbox, `:108` `move_active` (buttons `:224`), `:222` `reorderable`; rows from the vector signal `:174`; status `:183`, label `:249` | `tests/headless.rs:83` (Enter and Add add, blanks refused, checkbox, Remove, Move up/down; rows, checkbox and status from inspection); author `author_list_follows_the_vector` |
| 2. Rename in place: Enter commits, Escape discards | met | `src/lib.rs:44` `InlineText::controlled` (blank refused), start from Enter on the list through `on_activate` and `TextCommand::BeginEdit` `:211`-`:220` | `tests/headless.rs:146` (double click and Enter on the row start editing; editor focused and its value read; Enter commits, Escape keeps the shown title; empty rename refused) |
| 3. Filter hides, keeps selection meaningful, vector unchanged | met | `src/lib.rs:178` `ListFilter`, `:238` `CollectionFilter` with `HiddenActive::Clear` `:240`; actions gated by `selected_shown` from `filter.items()` `:180`; moves relative to the shown rows `:108` | `tests/headless.rs:189` (case-insensitive, vector unchanged, hidden selection cleared and Remove disabled, moves and arrows over shown rows, remove under a filter); author `author_filter_selection_stays_meaningful` |
| 4. Ctrl+N, Delete, Alt+Up, Alt+Down | met | application commands `src/lib.rs:255`-`:271`; Alt+arrows on the focused list also through `reorderable` | `tests/headless.rs:247` (arrows select one task, Alt+Down/Alt+Up via `Driver::chord`, Delete, Ctrl+N from the list and the search field, Delete inside a field edits text), `:300` (Alt+Down while the new-task field has focus) |
| 5. Headless check through simulated input | met | `tests/headless.rs` | all of the above; no model function is called |

## Task-specific checks

- The list is driven by the tester's `Vec`: met. The `ListSource` maps the
  vector signal (`src/lib.rs:174`) and every action writes the vector; author
  `author_list_follows_the_vector` removes and inserts in the vector directly
  and the rendered rows follow.
- In-place rename commits on Enter and is discarded on Escape: met (tester).
- The filter does not change the vector: met (`tests/headless.rs:202`, `:208`).
- Ctrl+N, Delete, Alt+Up and Alt+Down through simulated key input: met
  (`Driver::key` and `Driver::chord`).
- The status line follows checking and removing: met, read from the label's
  inspection semantics (tester and author).

## Author check source

`~/pert-ui-usability/acceptance-r2-2-claude/work/tests/author.rs`:

```rust
//! Author acceptance checks (acceptance copy only). Every UI result is read
//! from the mounted tree through the library's inspection (row titles in
//! on-screen order by bounds, checkbox values, status label, disabled state);
//! the vector is written directly only to show that the list follows it.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{CommandKey, Modifiers};
use std::time::Duration;
use todo::{Task, TodoApp, build, paths, sample_tasks};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn start() -> R<(Runtime, TodoApp)> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 560.0, 1.0)?)?;
    let app = build(&mut ui, sample_tasks())?;
    Driver::new(&mut ui).settle()?;
    Ok((ui, app))
}

/// Rendered rows, top to bottom: (title, done).
fn rows(d: &Driver, app: &TodoApp) -> R<Vec<(String, bool)>> {
    let list = d.find(&app.root, paths::LIST)?;
    let mut out = Vec::new();
    for row in d.runtime().children(&list)? {
        let y = d.runtime().bounds(&row)?.map(|r| r.y()).unwrap_or(f32::MAX);
        let title = d.semantics(&d.find(&row, &["title"])?)?.value;
        let done = d.semantics(&d.find(&row, &["done"])?)?.value;
        let Some(SemanticValue::Text(title)) = title else { panic!("{title:?}") };
        let Some(SemanticValue::Bool(done)) = done else { panic!("{done:?}") };
        out.push((y, title, done));
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(out.into_iter().map(|(_, t, d)| (t, d)).collect())
}

fn titles(d: &Driver, app: &TodoApp) -> R<Vec<String>> {
    Ok(rows(d, app)?.into_iter().map(|r| r.0).collect())
}

fn status(d: &Driver, app: &TodoApp) -> R<String> {
    Ok(d.semantics(&d.find(&app.root, paths::STATUS)?)?.name.unwrap_or_default())
}

fn pause(d: &mut Driver) -> R {
    d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    Ok(())
}

fn title_of(d: &Driver, app: &TodoApp, key: &str) -> R<WidgetId> {
    let list = d.find(&app.root, paths::LIST)?;
    Ok(d.find(&list, &[key, "title"])?)
}

#[test]
fn author_list_follows_the_vector() -> R {
    let (mut ui, app) = start()?;
    let mut d = Driver::new(&mut ui);
    assert_eq!(titles(&d, &app)?, ["Buy milk", "Write the plan", "Ship it"]);
    // Removing an element from the vector removes the row.
    let removed = app.tasks.get()?[1].clone();
    app.tasks.update(|v| {
        v.remove(1);
    })?;
    d.settle()?;
    assert_eq!(titles(&d, &app)?, ["Buy milk", "Ship it"]);
    assert_eq!(status(&d, &app)?, "0 of 2 done");
    // Inserting one (done) adds the row and the status follows.
    app.tasks.update(|v| v.insert(0, Task { done: true, ..removed }))?;
    d.settle()?;
    assert_eq!(rows(&d, &app)?[0], ("Write the plan".to_owned(), true));
    assert_eq!(titles(&d, &app)?, ["Write the plan", "Buy milk", "Ship it"]);
    assert_eq!(status(&d, &app)?, "1 of 3 done");
    Ok(())
}

#[test]
fn author_filter_selection_stays_meaningful() -> R {
    let (mut ui, app) = start()?;
    let mut d = Driver::new(&mut ui);
    let search = d.find(&app.root, paths::SEARCH)?;
    let remove = d.find(&app.root, paths::REMOVE)?;
    let list = d.find(&app.root, paths::LIST)?;

    // A query with surrounding spaces: the shown, selected task is acted on.
    d.replace_text(&search, " milk ")?;
    assert_eq!(titles(&d, &app)?, ["Buy milk"]);
    let t = title_of(&d, &app, "1")?;
    pause(&mut d)?;
    d.click(&t)?;
    pause(&mut d)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("1"));
    assert!(!d.semantics(&remove)?.disabled);
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Delete)?;
    assert_eq!(titles(&d, &app)?, Vec::<String>::new());
    assert_eq!(app.tasks.get()?.len(), 2);
    assert_eq!(status(&d, &app)?, "0 of 2 done");

    // Renaming the selected, shown task so it no longer matches hides it;
    // nothing can then act on the hidden task.
    d.runtime_mut().focus(&search)?;
    d.key(KeyCode::Escape)?;
    d.replace_text(&search, "plan")?;
    assert_eq!(titles(&d, &app)?, ["Write the plan"]);
    let t = title_of(&d, &app, "2")?;
    pause(&mut d)?;
    d.click(&t)?;
    pause(&mut d)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("2"));
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Enter)?;
    d.settle()?;
    d.type_text("Write the essay")?;
    d.key(KeyCode::Enter)?;
    d.settle()?;
    assert_eq!(titles(&d, &app)?, Vec::<String>::new());
    assert!(d.semantics(&remove)?.disabled, "Remove enabled for a hidden task");
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Delete)?;
    let alt = Modifiers { alt: true, ..Modifiers::default() };
    d.chord(KeyChord::new(CommandKey::Named(KeyCode::ArrowUp), alt), KeyCode::ArrowUp)?;
    let all: Vec<String> = app.tasks.get()?.into_iter().map(|t| t.title).collect();
    assert_eq!(all, ["Write the essay", "Ship it"]);
    // Clearing the query shows both again.
    d.runtime_mut().focus(&search)?;
    d.key(KeyCode::Escape)?;
    assert_eq!(titles(&d, &app)?, ["Write the essay", "Ship it"]);
    Ok(())
}
```

## Triage notes

Not acceptance failures; from the source only.

- Keyboard rename uses the library's programmatic start: `on_activate`
  defers a closure that finds the row's title by key and sends
  `TextCommand::BeginEdit` (`src/lib.rs:211`-`:220`). The deferral's
  `TaskReceipt` is bound to `_receipt` and dropped, so an error returned
  inside the deferred closure is not observed.
- Alt+Up/Alt+Down exist twice: the list's own `reorderable` handling while
  the list has focus, and application commands that also work from other
  focus (the tester's `:300` test). Only one acts per press.
- `next_id` is an `Rc<Cell<u64>>` beside the vector signal; per-row controls
  are `controlled` and write the vector back by id.
- No library defect observed.
