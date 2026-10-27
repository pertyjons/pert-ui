# Acceptance: r2-1-claude (task 1, settings editor)

Author check, 2026-10-27. The submitted `work/` (without `target/`, `.claude/`
and `.git/`) was copied to `~/pert-ui-usability/acceptance-r2-1-claude`,
built with `cargo build --offline --all-targets` and tested with
`cargo test --offline` in the kit-aca8df3 sandbox (pert-ui 0.3.75, features
`cpu`): builds without warnings (also after touching every source file), 7 of
7 headless tests in `tests/headless.rs` pass. The tests build the real app
(`settings_app`, the same `App` with `App::on_close` that `main` runs) with
`App::headless`, drive it with `Driver` (`replace_text`, `key`, `click`,
`wait_until`), read results from `Driver::semantics` and
`Runtime::query` inspection, close through `HeadlessApp::admit_close`, and
write three CPU captures (`clean.png`, `dirty-with-errors.png`,
`close-refused.png`, kept in `acceptance-r2-1-claude/captures/`). The captures
were viewed: titled groups, loaded values, `4096 px`, the refusal messages
under the prefix and width fields, the `● Unsaved changes` marker and the
close-refusal notice are visible. The native window was not run (no display).

Author checks added in the acceptance copy only
(`acceptance-r2-1-claude/work/tests/author.rs`, source below), 2 of 3 pass:

- `author_malformed_quality_and_out_of_range_width_refused` (pass): `abc` and
  `12.5` in the quality field are refused ("Enter a whole number", control
  invalid, Apply disabled, 70 kept); `20000` in the width field is refused
  ("Enter a number from 16 to 16384"), 4096 kept.
- `author_save_into_read_only_directory_is_reported_and_stays_dirty` (pass):
  a save into a `0555` directory over a `0444` file is reported as "Save
  failed: Could not save …: Permission denied (os error 13)", Apply and Revert
  stay enabled, the marker stays, the file is unchanged.
- `author_revert_discards_refused_draft` (**fails**): with the prefix changed
  (dirty) and a refused `150` left in the quality field, Revert restores the
  bound values (quality semantic value 70, prefix `holiday`, Apply disabled)
  but the quality field keeps showing the refused draft: inspection
  `text: "150"`, `valid: false`, semantics `invalid: true`. Revert also clears
  the field's error status, so the field shows `150` with an error outline and
  only the help text "1 to 100".

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Fields, groups and buttons exist and show the loaded values | met | `src/lib.rs:86-98` `load` called at startup from `build` `:150-154`; signals seeded `:157-162`; `Group::titled` groups `:214-238`, width `.unit("px")` `:206`; buttons `:240-284` | `tests/headless.rs:46-83` `shows_loaded_values` (writes a file; group names, every field value incl. `WebP` and the toggle, the `px` unit and display text from inspection, button names, marker); `:239-246` a second start loads the saved file; `clean.png` |
| 2. Invalid prefix and out-of-range quality refused, explanation shown, old value kept | met | `TextField::normalize` with `check_prefix` `:194-198`, `IntegerInput::range` `:200-207`, `on_rejection` → `FieldStatus::error` on the `Field` via `FormFields` `:145-147`, `:188-191` | `tests/headless.rs:111-155` (`bad name!` and a 33-character prefix, quality `150`: `invalid`, specific message in the description, marker clean and Apply disabled while the draft is shown, Escape shows `holiday` / 70); author check for malformed quality and width |
| 3. Apply/Revert disabled when clean, enabled when dirty; Revert restores | partly met | dirty = edited vs. applied `:164-176`, `can_act` `:178`, `Button::enabled` `:264`, `:283`; Revert sets the five signals from `applied` `:266-284`; marker `:182` | `tests/headless.rs:157-196` (enabled state from `Semantics::disabled`; three fields restored; edit back to the applied value is clean). Author check `author_revert_discards_refused_draft` fails: a field holding a refused draft still shows that draft after Revert, so the visible field does not show the applied value |
| 4. Successful save writes the file; failed save reported, stays dirty | met | Apply `:240-265` uses `EventContext::spawn` (worker thread, result delivered on the UI thread), applied only on `Ok`; `save` `:101-109` writes a temp file and renames | `tests/headless.rs:215-247` (file content, clean state, reload); `:249-271` (path under a regular file: "Save failed", Apply/Revert enabled, marker, value 55); author read-only-directory check |
| 5. Closing refused while dirty | met | `App::on_close` policy `:131-141` returns `KeepOpen` and sets `close_refused`; notice "Apply or revert them before closing" `:183-185`, `:297` | `tests/headless.rs:273-303` (`admit_close`: `Close` when clean; `KeepOpen` with a typed but unconfirmed prefix, notice shown; `Close` after Revert); `close-refused.png` |

## Task-specific checks

- **Out-of-range and malformed input refused through the field itself, last
  valid value applied:** met. `TextField::normalize` and
  `IntegerInput::range` refuse drafts; the bound signals keep the last valid
  value (the dirty marker stays clean while a refused draft is shown). The
  library's own parse refusal ("Enter a whole number") reaches the field
  through `on_rejection` (author check).
- **Apply and Revert enabled state read from inspection:** met
  (`Semantics::disabled`, `tests/headless.rs:99-101`).
- **Save off the UI thread, unwritable path reported:** met, through
  `EventContext::spawn` (the runtime starts one thread per task); a path
  under a regular file (tester) and a read-only directory (author) are both
  reported and leave the state dirty.
- **Close refusal through the library's close request:** met.
  `HeadlessApp::admit_close` runs close admission and the `App::on_close`
  policy as a native close request does; there is no custom close button.
  An uncommitted valid draft counts (admission releases focus first).
- **Loading at startup:** met (`build` calls `load`; a written file and a
  file saved by the app are read back).

## Triage notes

Not acceptance failures, except where item 3 says so.

- Item 3's defect comes from the tester's Revert writing the applied values
  back to signals that already hold them: the `IntegerInput` keeps a refused
  draft when its binding does not change. The library documents this
  ("External changed values win over drafts",
  `crates/pert-ui-runtime/src/text_fields/number_inputs.rs:88`), and
  `TextCommand::DiscardDraft` exists for it, so the library does not prevent
  the item; but a form Revert written the obvious way leaves the refused
  draft, which is a lead for the author's triage (a same-value write, or a
  form-level discard, could end refused drafts).
- The dirty state is derived by hand from four chained `combine` calls over
  five field signals (`src/lib.rs:164-176`); `FormFields` is used only for
  per-field statuses, which Revert clears one by one.
- The main window is reached from `App::on_close` through a
  `Rc<RefCell<Option<Built>>>` slot filled by the initializer
  (`src/lib.rs:111-143`).
- The `TaskReceipt` from `cx.spawn` is bound to `_receipt` after an
  `unused_must_use` warning (the tester's friction entry at 19:04).
- Tests use `std::env::temp_dir()` and `.unwrap()` in a test helper; nothing
  in the library path uses `unwrap`/`expect`.
- Friction log times (19:01 to 19:04) lie within the session
  (brief 18:57:38, ended 19:04:52); the last entry is marked "written later".

## Author check source

`acceptance-r2-1-claude/work/tests/author.rs`:

```rust
//! Author acceptance checks (added in the acceptance copy only).

use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::Semantics;
use settings_editor::settings_app;
use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SAVED: &str = r#"{"prefix":"holiday","format":"WebP","jpeg_quality":70,"max_width":4096,"open_folder":true}"#;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("author-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn start(path: PathBuf) -> R<(HeadlessApp, WidgetId)> {
    let (app, built) = settings_app(path)?;
    let app = app.headless()?;
    let root = built.borrow().as_ref().ok_or("not built")?.root.clone();
    Ok((app, root))
}

fn id(d: &Driver, root: &WidgetId, key: &str) -> R<WidgetId> {
    Ok(d.runtime().find_descendant(root, key)?.ok_or(format!("no {key}"))?)
}
fn sem(d: &Driver, root: &WidgetId, key: &str) -> R<Semantics> {
    Ok(d.semantics(&id(d, root, key)?)?)
}
fn disabled(d: &Driver, root: &WidgetId, key: &str) -> R<bool> {
    Ok(sem(d, root, key)?.disabled)
}
fn enter(d: &mut Driver, root: &WidgetId, key: &str, value: &str) -> R {
    let f = id(d, root, key)?;
    d.replace_text(&f, value)?;
    d.key(KeyCode::Enter)?;
    Ok(())
}

#[test]
fn author_malformed_quality_and_out_of_range_width_refused() -> R {
    let dir = scratch("malformed");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let (mut app, root) = start(path)?;
    let mut d = Driver::new(app.runtime_mut());
    for (key, draft, kept) in [("jpeg-quality", "abc", 70.0), ("jpeg-quality", "12.5", 70.0), ("max-width", "20000", 4096.0)] {
        enter(&mut d, &root, key, draft)?;
        let s = sem(&d, &root, key)?;
        eprintln!("{key} {draft:?}: invalid={} description={:?} value={:?}", s.invalid, s.description, s.value);
        assert!(s.invalid, "{key} {draft}");
        assert!(disabled(&d, &root, "apply")?, "apply enabled after refused {draft}");
        d.key(KeyCode::Escape)?;
        match sem(&d, &root, key)?.value {
            Some(SemanticValue::Number { value, .. }) => assert_eq!(value, kept),
            other => panic!("{key}: {other:?}"),
        }
    }
    drop(d);
    app.shutdown()?;
    Ok(())
}

#[test]
fn author_revert_discards_refused_draft() -> R {
    let dir = scratch("revert");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let (mut app, root) = start(path)?;
    let mut d = Driver::new(app.runtime_mut());
    enter(&mut d, &root, "prefix", "changed")?;
    enter(&mut d, &root, "jpeg-quality", "150")?;
    assert!(sem(&d, &root, "jpeg-quality")?.invalid);
    let revert = id(&d, &root, "revert")?;
    d.click(&revert)?;
    d.settle()?;
    let q = sem(&d, &root, "jpeg-quality")?;
    eprintln!("quality after revert: invalid={} value={:?} description={:?}", q.invalid, q.value, q.description);
    let wid = id(&d, &root, "jpeg-quality")?;
    let iid = d.runtime().inspection_id(&wid)?;
    let found = d.runtime().query(&InspectionQuery::components(Selector::id(iid)).detail(Detail::Full));
    eprintln!("quality properties after revert: {:?}", found.components[0].details.properties);
    assert_eq!(sem(&d, &root, "prefix")?.value, Some(SemanticValue::Text("holiday".into())));
    assert!(disabled(&d, &root, "apply")?);
    assert!(matches!(q.value, Some(SemanticValue::Number { value, .. }) if value == 70.0), "{q:?}");
    assert!(!q.invalid, "refused draft still shown after Revert: {q:?}");
    drop(d);
    app.shutdown()?;
    Ok(())
}

#[test]
fn author_save_into_read_only_directory_is_reported_and_stays_dirty() -> R {
    let dir = scratch("readonly");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444))?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555))?;
    if std::fs::write(dir.join("probe"), "").is_ok() {
        eprintln!("permissions are bypassed here; skipping");
        return Ok(());
    }
    let (mut app, root) = start(path.clone())?;
    let mut d = Driver::new(app.runtime_mut());
    enter(&mut d, &root, "jpeg-quality", "55")?;
    let apply = id(&d, &root, "apply")?;
    d.click(&apply)?;
    let status = id(&d, &root, "status")?;
    d.wait_until(Duration::from_secs(5), move |ui| {
        let iid = ui.inspection_id(&status)?;
        let found = ui.query(&InspectionQuery::components(Selector::id(iid)).detail(Detail::Semantics));
        Ok(found.components.first()
            .and_then(|c| c.details.semantics.as_ref())
            .and_then(|s| s.name.as_deref())
            .is_some_and(|n| n.starts_with("Save failed")))
    })?;
    eprintln!("status: {:?}", sem(&d, &root, "status")?.name);
    assert!(!disabled(&d, &root, "apply")?);
    assert!(!disabled(&d, &root, "revert")?);
    assert_eq!(sem(&d, &root, "dirty-marker")?.name.as_deref(), Some("● Unsaved changes"));
    assert!(std::fs::read_to_string(&path)?.contains("\"jpeg_quality\":70"));
    drop(d);
    app.shutdown()?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}
```
