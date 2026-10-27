# Acceptance: r2-1-codex (task 1, settings editor)

Author check, 2026-10-27. The submitted `work/` (without `target/`;
there is no `.git/` or `.claude/`) was copied to
`~/pert-ui-usability/acceptance-r2-1-codex`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline`
in the kit-aca8df3 sandbox (pert-ui 0.3.75, default features): builds without
warnings, both tester tests in `src/main.rs` (`mod tests`) pass. The tests
build the real app (`application`, the same `App` with `App::on_close` that
`main` runs) with `App::headless`, drive it with `Driver` (`replace_text`,
`key`, `click`, `wait_until`), read results from `Driver::semantics` and
`Runtime::query` inspection (semantics and the `Field`'s `message`
property), close through `HeadlessApp::admit_close`, and read the JSON file
on disk. The submission also contains `README.md` and `test-output/` (files
the tests write; `success/settings.json` is the last successful save). No
captures (default features only). The native window was not run (no
display).

Author checks added in the acceptance copy only (a `mod author` appended to
`src/main.rs`, source below), 4 of 4 pass:

- `author_loaded_webp_and_toggle`: a loaded `WebP` is the selected choice
  (`Choice { selected: Some(2), label: "WebP" }`) and the toggle shows the
  loaded `false`; the tester's format assertion only matched `JPEG` in the
  debug text.
- `author_malformed_quality_and_out_of_range_width_refused`: `abc` and `12.5`
  in quality and `20000` in width are refused (control invalid, the `Field`
  status `error` with "JPEG quality must be an integer from 1 to 100" /
  "Width must be an integer from 16 to 16384 px", Apply disabled, 73 / 4096
  kept after Escape).
- `author_close_with_uncommitted_draft_is_refused`: `admit_close` gives
  `Close` when clean, then `KeepOpen` with a valid prefix typed but not
  confirmed with Enter, and the status asks to Apply or Revert.
- `author_save_into_read_only_directory_is_reported_and_stays_dirty`: a save
  into a `0555` directory over a `0444` file shows "Save failed: Permission
  denied (os error 13)", Apply and Revert stay enabled, "Unsaved changes"
  stays, the file is unchanged.

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Fields, groups and buttons exist and show the loaded values | met | `src/main.rs:29-49` `load`, called at startup from `build` `:59`; signals seeded `:60-65`; groups are `Column`s with a title `Label` `:200-240`; width `.unit("px")` `:120`; buttons `:133-195`, `:242` | `src/main.rs:367-394` (writes a file; group titles, prefix, quality, width, toggle, format, `px` in the width's properties, both buttons' semantics); author check `author_loaded_webp_and_toggle` |
| 2. Invalid prefix and out-of-range quality refused, explanation shown, old value kept | met | `TextField::normalize` with `valid_prefix` `:98-109`, `IntegerInput::range` `:110-126`, `on_rejection` → `FieldStatus::error` on the `Field`'s `status` `:204`, `:226`, `:230` | `src/main.rs:395-410` (`bad prefix!`: the `Field`'s message and `invalid`; quality `101`: message; Escape shows `loaded_42` / 73; Apply and Revert disabled, so the bound values are unchanged); the messages are not permanent text (`:429` asserts the message is gone after Revert); author check for malformed quality and width |
| 3. Apply/Revert disabled when clean, enabled when dirty; Revert restores | met | dirty = current vs. applied `:83-91`, `can_act` `:84`, `Button::enabled` `:159`, `:195`; Revert sets the five signals, clears statuses and discards retained drafts with `TextCommand::DiscardDraft` through `cx.defer` `:175-195` | `src/main.rs:331-339` reads `Semantics::disabled`; `:394`, `:410` disabled when clean, `:411-413` enabled when dirty with the marker; `:414-429` Revert with a refused quality draft restores prefix and quality, clears `invalid` and the message |
| 4. Successful save writes the file; failed save reported, stays dirty | met | Apply `:133-159` uses `EventContext::spawn` (worker thread, result delivered on the UI thread), applied only on `Ok`; controls locked while saving (`editable` `:68`) | `src/main.rs:441-450` (status "Settings saved", buttons disabled, marker clean, file holds quality 81); `:458-486` (target path is a directory: "Save failed:", buttons enabled, marker, value kept, close refused); author read-only-directory check |
| 5. Closing refused while dirty | met | `App::on_close` `:270-279` returns `KeepOpen` while dirty or saving and sets the status "Unsaved changes: Apply or Revert before closing" | `src/main.rs:432-440` (`admit_close` → `KeepOpen`, status shown), `:451-454` (`Close` after the save), `:479-483`; author check with an unconfirmed draft |

## Task-specific checks

- **Out-of-range and malformed input refused through the field itself, last
  valid value applied:** met. `TextField::normalize` and
  `IntegerInput::range` refuse drafts and the bound signals keep the last
  valid value; the library's own parse refusal reaches `on_rejection`, which
  replaces it with the tester's message (author check).
- **Apply and Revert enabled state read from inspection:** met
  (`Semantics::disabled`, `src/main.rs:331-339`).
- **Save off the UI thread, unwritable path reported:** met, through
  `EventContext::spawn` (one thread per task); a directory as the target
  (tester) and a read-only directory (author) are both reported and leave the
  state dirty.
- **Close refusal through the library's close request:** met.
  `HeadlessApp::admit_close` runs close admission and the `App::on_close`
  policy as a native close request does; there is no custom close button.
- **Loading at startup:** met (`build` calls `load`; the tests write a file
  first and read the loaded values back).

## Triage notes

Not acceptance failures.

- Revert needs `cx.defer` plus `Runtime::text_command(…,
  TextCommand::DiscardDraft)` for each editor (`src/main.rs:186-192`), with
  the editor ids collected after mounting (`:250-254`), because writing an
  unchanged value back to a binding does not end a refused draft (documented:
  "External changed values win over drafts",
  `crates/pert-ui-runtime/src/text_fields/number_inputs.rs:88`). The test at
  `:414` names it ("Revert must discard a refused draft even if its valid
  binding is unchanged"). A lead for the author's triage: a form Revert
  written the obvious way leaves the refused draft shown.
- Groups are `Label`s in `Column`s, not the library's `Group::titled`; three
  group labels share the key `title`, found per group with `Driver::find`.
- The dirty state is derived by hand from four chained `combine` calls over
  five field signals (`:69-83`); the per-field statuses are plain signals.
- `on_rejection` ignores the rejection's message and substitutes a fixed one
  (`:113-117`, `:122-126`), so the library's specific reason ("Enter a whole
  number") is not shown.
- Production `expect("valid size")` twice (`:268-269`); the built state
  reaches `App::on_close` through a `Rc<RefCell<Option<Built>>>` slot; the
  `root` field carries `#[cfg_attr(not(test), allow(dead_code))]`.
- The format is a `String` signal with string choices; the loader validates
  it against the three names.
- Tests write fixtures under `CARGO_MANIFEST_DIR/test-output/` and use
  `unwrap` in test helpers.
- Friction log times (18:47 to 18:48) lie within the session
  (brief 18:45:45, ended 18:49:12).

## Author check source

Appended to `acceptance-r2-1-codex/work/src/main.rs`:

```rust

// Author acceptance checks (added in the acceptance copy only).
#[cfg(test)]
mod author {
    use super::*;
    use pert_ui::testing::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;
    type R = std::result::Result<(), Box<dyn std::error::Error>>;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("author-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn loaded(dir: &PathBuf) -> PathBuf {
        let path = dir.join("settings.json");
        let s = Settings { prefix: "loaded_42".into(), format: "WebP".into(), quality: 73, width: 4096, open_folder: false };
        std::fs::write(&path, serde_json::to_vec(&s).unwrap()).unwrap();
        path
    }
    fn id(d: &Driver<'_>, b: &Built, key: &str) -> WidgetId {
        d.runtime().find_descendant(&b.root, key).unwrap().unwrap()
    }
    fn sem(d: &Driver<'_>, b: &Built, key: &str) -> pert_ui::Semantics {
        d.semantics(&id(d, b, key)).unwrap()
    }
    fn props(d: &Driver<'_>, b: &Built, key: &str) -> String {
        let iid = d.runtime().inspection_id(&id(d, b, key)).unwrap();
        format!("{:?}", d.runtime().query(&InspectionQuery::components(Selector::id(iid)).detail(Detail::Full)).components[0].details.properties)
    }
    fn enter(d: &mut Driver<'_>, b: &Built, key: &str, v: &str) -> R {
        let f = id(d, b, key);
        d.replace_text(&f, v)?;
        d.key(KeyCode::Enter)?;
        Ok(())
    }
    fn status_name(ui: &Runtime, b: &Built) -> String {
        let w = ui.find_descendant(&b.root, "status").unwrap().unwrap();
        let iid = ui.inspection_id(&w).unwrap();
        ui.query(&InspectionQuery::components(Selector::id(iid)).detail(Detail::Semantics)).components[0]
            .details.semantics.as_ref().unwrap().name.clone().unwrap_or_default()
    }

    #[test]
    fn author_loaded_webp_and_toggle() -> R {
        let dir = scratch("webp");
        let slot = Rc::default();
        let mut app = application(loaded(&dir), Rc::clone(&slot)).headless()?;
        let b = slot.borrow().as_ref().unwrap().clone();
        let d = Driver::new(app.runtime_mut());
        let f = sem(&d, &b, "format");
        eprintln!("format: {:?}", f.value);
        assert!(matches!(f.value, Some(SemanticValue::Choice { label: Some(ref l), .. }) if l == "WebP"));
        assert_eq!(sem(&d, &b, "open-folder").value, Some(SemanticValue::Bool(false)));
        drop(d);
        app.shutdown()?;
        Ok(())
    }

    #[test]
    fn author_malformed_quality_and_out_of_range_width_refused() -> R {
        let dir = scratch("malformed");
        let slot = Rc::default();
        let mut app = application(loaded(&dir), Rc::clone(&slot)).headless()?;
        let b = slot.borrow().as_ref().unwrap().clone();
        let mut d = Driver::new(app.runtime_mut());
        for (key, field, draft, kept) in [("quality", "quality-field", "abc", 73.0), ("quality", "quality-field", "12.5", 73.0), ("width", "width-field", "20000", 4096.0)] {
            enter(&mut d, &b, key, draft)?;
            let s = sem(&d, &b, key);
            let p = props(&d, &b, field);
            eprintln!("{key} {draft:?}: invalid={} field={p}", s.invalid);
            assert!(s.invalid);
            assert!(p.contains("must be an integer"), "{p}");
            assert!(sem(&d, &b, "apply").disabled);
            d.key(KeyCode::Escape)?;
            assert!(matches!(sem(&d, &b, key).value, Some(SemanticValue::Number { value, .. }) if value == kept));
        }
        drop(d);
        app.shutdown()?;
        Ok(())
    }

    #[test]
    fn author_close_with_uncommitted_draft_is_refused() -> R {
        let dir = scratch("draft");
        let slot = Rc::default();
        let mut app = application(loaded(&dir), Rc::clone(&slot)).headless()?;
        let b = slot.borrow().as_ref().unwrap().clone();
        let window = app.main_window();
        assert!(matches!(app.admit_close(&window)?, NativeCloseAction::Close));
        {
            let mut d = Driver::new(app.runtime_mut());
            let f = id(&d, &b, "prefix");
            d.replace_text(&f, "typed_not_entered")?;
        }
        assert!(matches!(app.admit_close(&window)?, NativeCloseAction::KeepOpen));
        let mut d = Driver::new(app.runtime_mut());
        d.settle()?;
        assert!(status_name(d.runtime(), &b).contains("Apply or Revert"));
        drop(d);
        app.shutdown()?;
        Ok(())
    }

    #[test]
    fn author_save_into_read_only_directory_is_reported_and_stays_dirty() -> R {
        let dir = scratch("readonly");
        let path = loaded(&dir);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444))?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555))?;
        if std::fs::write(dir.join("probe"), "").is_ok() {
            eprintln!("permissions are bypassed here; skipping");
            return Ok(());
        }
        let slot = Rc::default();
        let mut app = application(path.clone(), Rc::clone(&slot)).headless()?;
        let b = slot.borrow().as_ref().unwrap().clone();
        {
            let mut d = Driver::new(app.runtime_mut());
            enter(&mut d, &b, "quality", "55")?;
            d.click(&id(&d, &b, "apply"))?;
            let bb = b.clone();
            d.wait_until(Duration::from_secs(5), move |ui| Ok(status_name(ui, &bb).starts_with("Save failed")))?;
            eprintln!("status: {}", status_name(d.runtime(), &b));
            assert!(!sem(&d, &b, "apply").disabled);
            assert!(!sem(&d, &b, "revert").disabled);
            assert_eq!(sem(&d, &b, "dirty").name.as_deref(), Some("Unsaved changes"));
        }
        assert!(std::fs::read_to_string(&path)?.contains("\"quality\":73"));
        app.shutdown()?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755))?;
        Ok(())
    }
}
```
