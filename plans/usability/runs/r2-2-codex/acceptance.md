# Acceptance: r2-2-codex (task 2, task list)

Author check, 2026-10-27, kit `kit-aca8df3` (0.3.75). The submitted `work/`
(without `target/` and the CLI directories) was copied to
`~/pert-ui-usability/acceptance-r2-2-codex`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline` in
the kit sandbox: builds without warnings, 3 of 3 tests in `src/main.rs`
`mod tests` pass. The application starts with an empty list.

The tester's checks drive the application with the library's `Driver`
(`click`, `replace_text`, `type_text`, `key`, `chord`) and read the status
label from inspection, the shown row titles from inspection semantics after
adding, reordering, renaming and filtering (`displayed`), and the vector, the
selection and the filter counts. Not covered: removal is asserted on the
vector only, never on the rendered rows; the vector is never written
directly; the Escape rename check reads the vector only and would also pass if
the draft had never opened; the filter's selection handling was not tested
for a padded query or for a selected task renamed out of the filter. The
author added `src/author_tests.rs` in the acceptance copy only (hooked in by
`#[cfg(test)] mod author_tests;` appended to `src/main.rs`; source below),
three tests, all pass (6 passed in total):

- `author_list_follows_the_vector`: tasks added with Ctrl+N, typing and
  Enter; removing an element from the `Vec` signal removes the rendered row
  and the status label follows; inserting a done task shows the row and
  "1 of 3 done"; Remove on a selected row removes the rendered row.
- `author_rename_draft_shown_escape_discards_enter_commits`: Enter on the
  selected row focuses the title's in-place editor, the typed draft is its
  inspection value, Escape leaves the editor and keeps the shown title, Enter
  commits the new shown title.
- `author_filter_selection_stays_meaningful`: with the query `" milk "` the
  shown, selected task has Remove enabled and Delete on the list removes it;
  a selected task renamed in place so that it no longer matches is hidden,
  Remove is disabled, and Delete and Alt+Up leave the vector unchanged;
  Escape in the search field shows both tasks again.

Rows are read in on-screen order (sorted by `Runtime::bounds`).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Add, remove, check, reorder update list and status | met | `src/main.rs:137` `add` (Enter `:200`, button `:202`, blank refused), `:95` `Actions::remove` (selects the next shown neighbour), `:30` checkbox, `:110` `Actions::shift`, `:196` `reorderable`; rows from the vector signal `:163`; status `:206`, label `:233` | `src/main.rs:398` (Enter and Add add, blanks refused, checkbox, Move up/down with rendered rows, Remove, status from inspection); author `author_list_follows_the_vector` for removal on the rendered rows |
| 2. Rename in place: Enter commits, Escape discards | met | `src/main.rs:34` `InlineText::controlled` with `normalize` refusing blanks `:38`; start from Enter on the list through `on_activate` and `TextCommand::BeginEdit` `:186`-`:195` | `src/main.rs:429` (Enter commits, shown title read; double click starts editing); author `author_rename_draft_shown_escape_discards_enter_commits` for the draft and Escape |
| 3. Filter hides, keeps selection meaningful, vector unchanged | met | `src/main.rs:170` `ListFilter`, `:217` `CollectionFilter` with `HiddenActive::Clear` `:218`; actions gated by `shown` from `filter.items()` `:171`; remove and shift relative to the shown rows `:99`, `:114` | `src/main.rs:460` (mixed case, shown rows read, vector unchanged, hidden selection cleared, disabled Remove changes nothing, arrows over shown rows, Delete under a filter, no match); author `author_filter_selection_stays_meaningful` |
| 4. Ctrl+N, Delete, Alt+Up, Alt+Down | met | application commands `src/main.rs:243`-`:274`; Alt+arrows on the focused list also through `reorderable` | `src/main.rs:460` (Ctrl+N focus asserted twice, Alt+Up/Alt+Down via `Driver::chord`, Delete on the list) |
| 5. Headless check through simulated input | met | `src/main.rs:305` `mod tests` | the tester's tests exercise items 1 to 4 through simulated input only, with gaps in what they read back that the author tests close; no model function is called |

## Task-specific checks

- The list is driven by the tester's `Vec`: met. The `ListSource` maps the
  vector signal (`src/main.rs:163`) and every action writes the vector;
  author `author_list_follows_the_vector` removes and inserts in the vector
  directly and the rendered rows follow.
- In-place rename commits on Enter and is discarded on Escape: met (tester
  for Enter; author check for the draft and Escape).
- The filter does not change the vector: met (`src/main.rs:487`).
- Ctrl+N, Delete, Alt+Up and Alt+Down through simulated key input: met
  (`Driver::key` and `Driver::chord`).
- The status line follows checking and removing: met, read from the label's
  inspection semantics.

## Author check source

`~/pert-ui-usability/acceptance-r2-2-codex/work/src/author_tests.rs`:

```rust
//! Author acceptance checks (acceptance copy only). Every UI result is read
//! from the mounted tree through the library's inspection (row titles in
//! on-screen order by bounds, the in-place editor's value and focus, the
//! status label, disabled state); the vector is written directly only to show
//! that the list follows it.

use super::*;
use pert_ui::testing::*;
use std::time::Duration;

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

const LIST: &[&str] = &["tasks", "list"];
const SEARCH: &[&str] = &["tasks", "filter-field", "filter"];

fn start() -> R<(Runtime, TaskList)> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(960.0, 640.0, 1.0)?)?;
    let app = build(&mut ui)?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let ctrl_n = KeyChord::new(
        CommandKey::Character('n'),
        Modifiers { control: true, ..Modifiers::default() },
    );
    for t in ["Buy milk", "Write the plan", "Ship it"] {
        d.chord(ctrl_n, KeyCode::KeyN)?;
        d.type_text(t)?;
        d.key(KeyCode::Enter)?;
    }
    drop(d);
    Ok((ui, app))
}

fn titles(d: &Driver, app: &TaskList) -> R<Vec<String>> {
    let list = d.find(&app.root, LIST)?;
    let mut out = Vec::new();
    for row in d.runtime().children(&list)? {
        let y = d.runtime().bounds(&row)?.map(|r| r.y()).unwrap_or(f32::MAX);
        let Some(SemanticValue::Text(t)) = d.semantics(&d.find(&row, &["title"])?)?.value else {
            panic!("no title")
        };
        out.push((y, t));
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(out.into_iter().map(|r| r.1).collect())
}

fn status(d: &Driver, app: &TaskList) -> R<String> {
    Ok(d.semantics(&d.find(&app.root, &["status"])?)?.name.unwrap_or_default())
}

fn pause(d: &mut Driver) -> R {
    d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    Ok(())
}

fn title_of(d: &Driver, app: &TaskList, key: &str) -> R<WidgetId> {
    let list = d.find(&app.root, LIST)?;
    Ok(d.find(&list, &[key, "title"])?)
}

fn select(d: &mut Driver, app: &TaskList, key: &str) -> R {
    let t = title_of(d, app, key)?;
    pause(d)?;
    d.click(&t)?;
    pause(d)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some(key));
    Ok(())
}

#[test]
fn author_list_follows_the_vector() -> R {
    let (mut ui, app) = start()?;
    let mut d = Driver::new(&mut ui);
    assert_eq!(titles(&d, &app)?, ["Buy milk", "Write the plan", "Ship it"]);
    assert_eq!(status(&d, &app)?, "0 of 3 done");
    let removed = app.tasks.get()?[1].clone();
    app.tasks.update(|v| {
        v.remove(1);
    })?;
    d.settle()?;
    assert_eq!(titles(&d, &app)?, ["Buy milk", "Ship it"]);
    assert_eq!(status(&d, &app)?, "0 of 2 done");
    app.tasks.update(|v| v.insert(0, Task { done: true, ..removed }))?;
    d.settle()?;
    assert_eq!(titles(&d, &app)?, ["Write the plan", "Buy milk", "Ship it"]);
    assert_eq!(status(&d, &app)?, "1 of 3 done");
    // Remove through the UI: the rendered row goes, status follows.
    select(&mut d, &app, "3")?;
    let remove = d.find(&app.root, &["actions", "remove"])?;
    d.click(&remove)?;
    assert_eq!(titles(&d, &app)?, ["Write the plan", "Buy milk"]);
    assert_eq!(status(&d, &app)?, "1 of 2 done");
    Ok(())
}

#[test]
fn author_rename_draft_shown_escape_discards_enter_commits() -> R {
    let (mut ui, app) = start()?;
    let mut d = Driver::new(&mut ui);
    let list = d.find(&app.root, LIST)?;
    select(&mut d, &app, "2")?;
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Enter)?;
    d.settle()?;
    let title = title_of(&d, &app, "2")?;
    let editor = d.find(&title, &["editor"])?;
    assert_eq!(d.runtime().focused(), Some(editor.clone()), "draft editor not focused");
    d.type_text("Throw the plan away")?;
    assert_eq!(
        d.semantics(&editor)?.value,
        Some(SemanticValue::Text("Throw the plan away".into()))
    );
    d.key(KeyCode::Escape)?;
    assert_eq!(titles(&d, &app)?, ["Buy milk", "Write the plan", "Ship it"]);
    assert_ne!(d.runtime().focused(), Some(editor));
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Enter)?;
    d.settle()?;
    d.type_text("Write the essay")?;
    d.key(KeyCode::Enter)?;
    assert_eq!(titles(&d, &app)?, ["Buy milk", "Write the essay", "Ship it"]);
    Ok(())
}

#[test]
fn author_filter_selection_stays_meaningful() -> R {
    let (mut ui, app) = start()?;
    let mut d = Driver::new(&mut ui);
    let search = d.find(&app.root, SEARCH)?;
    let remove = d.find(&app.root, &["actions", "remove"])?;
    let list = d.find(&app.root, LIST)?;

    d.replace_text(&search, " milk ")?;
    assert_eq!(titles(&d, &app)?, ["Buy milk"]);
    select(&mut d, &app, "1")?;
    assert!(!d.semantics(&remove)?.disabled);
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Delete)?;
    assert_eq!(titles(&d, &app)?, Vec::<String>::new());
    assert_eq!(app.tasks.get()?.len(), 2);
    assert_eq!(status(&d, &app)?, "0 of 2 done");

    d.runtime_mut().focus(&search)?;
    d.key(KeyCode::Escape)?;
    d.replace_text(&search, "plan")?;
    assert_eq!(titles(&d, &app)?, ["Write the plan"]);
    select(&mut d, &app, "2")?;
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
    d.chord(alt_arrow(false), KeyCode::ArrowUp)?;
    let all: Vec<String> = app.tasks.get()?.into_iter().map(|t| t.title).collect();
    assert_eq!(all, ["Write the essay", "Ship it"]);
    d.runtime_mut().focus(&search)?;
    d.key(KeyCode::Escape)?;
    assert_eq!(titles(&d, &app)?, ["Write the essay", "Ship it"]);
    Ok(())
}
```

## Triage notes

Not acceptance failures; from the source only.

- Keyboard rename uses the library's programmatic start: `on_activate`
  defers `TextCommand::BeginEdit` on the row's title (`src/main.rs:186`-`:195`);
  the deferral's `TaskReceipt` is bound to `_receipt` and dropped, so an error
  inside the deferred closure is not observed.
- `build` uses `.expect("mounted new task field")` on a production path
  (`src/main.rs:240`-`:242`), and `TaskList` carries `#[allow(dead_code)]`
  (`:80`) because only the tests read its fields.
- Alt+Up/Alt+Down exist twice: the list's own `reorderable` handling while
  the list has focus, and application commands. Only one acts per press.
- No library defect observed.
