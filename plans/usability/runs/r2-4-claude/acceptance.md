# Acceptance: r2-4-claude (task 4, testing an existing app)

Author check, 2026-10-27. The submitted `work/` (without `target/`, `.claude/`
and `.git/`) was copied to `~/pert-ui-usability/acceptance-r2-4-claude`, built
with `cargo build --offline --all-targets` and tested with `cargo test
--offline` in the kit-aca8df3 sandbox (pert-ui 0.3.75). It builds without
warnings, also after touching `src/lib.rs` and the tests to force a rebuild.
All 8 tests pass: 7 in `tests/behaviors.rs` and 1 in `tests/capture.rs`. The
capture test wrote `target/tmp/bookmarks.png` again (480 x 640), and the
author opened it. The author added one check, for behavior 7 with nothing
focused, and made five deliberate breaks of their own (both described below).

Session: brief 19:38:44, ended 19:42:34 (about 3.8 minutes).

## Application diff

The author compared the fixture `plans/usability/task4-app` at `aca8df3`
(extracted with `git archive`) with the submitted `work/bookmarks`, using
`diff -r --exclude target --exclude tests --exclude Cargo.lock`. There is no
difference. `src/lib.rs`, `src/main.rs`, `Cargo.toml` (already
`pert-ui = { version = "0.3.75", features = ["cpu"] }`) and `README.md` are
byte-identical. The only additions are `tests/` and the generated
`Cargo.lock`. FRICTION.md says the same ("no changes to the application").

## Items

| Item | Result | Code | Check |
|---|---|---|---|
| 1. One test per README behavior, all passing | met | `tests/behaviors.rs`. Each test has one `#[test]` (mapping below) and calls only `bookmarks::build`, the documented entry point. Input goes through `testing::Driver`: `replace_text`, `click` followed by `advance(ClickCounter::DEFAULT_INTERVAL + 1 ms)`, and `chord(KeyChord::primary('l'), KeyCode::KeyL)`, plus `Runtime::focus`. Reads go through `Driver::semantics`: the semantic `name` for labels and buttons, and `SemanticValue::Text` for fields. Rows are found with `Runtime::children`/`key`/`find`/`bounds` (`:101-134`). | All 7 pass against the unchanged app in the acceptance sandbox. Behavior 7 is tested with focus in the filter, in the Name field and on the Add button, but not with nothing focused. The author's check `author_no_focus.rs` (below) covers that case and passes. |
| 2. CPU render of the main window writes a PNG | met | `tests/capture.rs:5-33`: `set_capture_support(true)`, `request_capture(CaptureRequest::new(CaptureTarget::window(primary_window)))`, `settle`, `CpuRenderer::new().serve_captures` (asserts 1 served), `pending.wait()`, size assertion, a not-blank assertion, `encode_png`, then a write to `$CARGO_TARGET_TMPDIR/bookmarks.png` | The test passes. The author opened the regenerated PNG. It shows the real window: the Name and URL fields (URL showing its `https://` placeholder), Add, the Filter field, "2 bookmarks", and the Rust and Crates rows with Remove buttons. |
| 3. A deliberate break makes a test fail, recorded | met | FRICTION.md entry 19:42 records seven breaks, one per behavior. Each made its behavior's test fail, with file:line and left/right values. The original `lib.rs` was restored, and diff confirmed it. | The author reproduced the recorded trim break, which fails at `behaviors.rs:151` as recorded, and four more breaks (below). The submitted `src/` is identical to the fixture. |
| Any change to the app listed and behavior-neutral | met | No change (application diff above). | — |

Task-specific checks from `acceptance.md`: there is one test per README
behavior, and each one drives input and reads inspection. All pass against
the unchanged app, a CPU capture PNG is written, the recorded deliberate break
made a test fail, and the app is unchanged. All are met. Behavior 7 is met
together with the author's no-focus check.

## Behavior-to-test mapping (`tests/behaviors.rs`)

| README behavior | Test | What it drives / checks |
|---|---|---|
| 1. Add | `add_appends_bookmark_with_trimmed_name` (l. 146) | Adds `"  Docs  "` with https, then `Example` with http. The rows, in layout order, read keys 1..4, and `Docs` is trimmed and appended last. |
| 2. Refused add | `refused_add_shows_error_and_keeps_fields` (l. 162) | An empty name gets the name message. A spaces-only name with a bad URL gets the name message, so the name is checked first. An `ftp://` URL gets the URL message. Each time, the fields keep their text and the rows are unchanged. The next valid add clears the message and adds a row. |
| 3. Fields cleared | `successful_add_clears_fields` (l. 197) | Both field values are empty after Add. |
| 4. List and Remove | `rows_show_bookmarks_and_remove_deletes_one` (l. 208) | Each row shows its name, URL and a `Remove` button name. Remove on row 1 and then on row 3 deletes only that row. |
| 5. Filter | `filter_narrows_list_by_name` (l. 230) | `rust` shows rows 1 and 3, so it is a case-insensitive substring match. `"  CRATES "` shows Crates, so case and surrounding spaces are ignored. A no-match filter shows nothing. An empty filter shows all rows. |
| 6. Status | `status_counts_bookmarks` (l. 252) | Reads `2 bookmarks`, `3 bookmarks`, `1 of 3 bookmarks`, `0 of 3 bookmarks`, `3 bookmarks`, and `1 bookmarks` after two removals. |
| 7. Clear filter | `clear_filter_command_empties_filter` (l. 274) | Sends Ctrl+L as a chord with focus in the filter, in Name (Name keeps its text) and on Add (focus stays on Add). Each time, the filter empties and the rows and status are restored. |

## Author check: `author_no_focus.rs`

The source is kept at
`~/pert-ui-usability/acceptance-r2-4-claude/author/author_no_focus.rs`. It
was run as `tests/author_no_focus.rs` in the acceptance copy and passes.
Breaking the command (Ctrl+K in place of Ctrl+L) makes it fail at line 38.
It uses only the public facade. `clear_focus` is the `HostIntegration` method
that the `testing` module's own example (`crates/pert-ui/src/lib.rs:244-260`
at aca8df3) shows for "as the window's deactivation would".

```rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn text(driver: &Driver<'_>, root: &WidgetId, path: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let id = driver.find(root, path)?;
    Ok(match driver.semantics(&id)?.value {
        Some(SemanticValue::Text(text)) => text,
        _ => String::new(),
    })
}

fn name(driver: &Driver<'_>, root: &WidgetId, path: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let id = driver.find(root, path)?;
    Ok(driver.semantics(&id)?.name.unwrap_or_default())
}

#[test]
fn clear_filter_with_nothing_focused() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 640.0, 1.0)?)?;
    let root = bookmarks::build(&mut ui)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let filter = driver.find(&root, &["content", "filter-field", "filter"])?;
    driver.replace_text(&filter, "rust")?;
    assert_eq!(name(&driver, &root, &["content", "status"])?, "1 of 2 bookmarks");
    driver.runtime_mut().clear_focus();
    driver.settle()?;
    assert_eq!(driver.runtime().focused(), None);
    assert_eq!(text(&driver, &root, &["content", "filter-field", "filter"])?, "rust");
    driver.chord(KeyChord::primary('l'), KeyCode::KeyL)?;
    assert_eq!(text(&driver, &root, &["content", "filter-field", "filter"])?, "");
    assert_eq!(name(&driver, &root, &["content", "status"])?, "2 bookmarks");
    Ok(())
}
```

## Author's own deliberate breaks

`~/pert-ui-usability/acceptance-r2-4-claude/author/breaks.sh` applied each
break below to `src/lib.rs` in the acceptance copy and ran `cargo test
--no-fail-fast` with the author check present. It then restored the file,
and `cmp` confirmed it was identical to the original.

- A, name not trimmed (`name: entered_name`): `add_appends_bookmark_with_trimmed_name`
  fails at `behaviors.rs:151`.
- B, the `url.set(String::new())?` line deleted: `successful_add_clears_fields`
  fails at `:201`.
- C, filter text lowercased but the name not lowercased: `filter_narrows_list_by_name`
  fails at `:236`. `status_counts_bookmarks` (`:260`),
  `clear_filter_command_empties_filter` (`:279`) and the author check also
  fail, because they depend on the filter.
- C2, fully case-sensitive filter: `filter_narrows_list_by_name` fails at
  `:236`, and the clear-filter test and the author check also fail.
- D, `KeyChord::primary('k')`: `clear_filter_command_empties_filter` fails at
  `:281`, and the author check fails at line 38.

## Triage notes (not acceptance failures)

- **Rows in display order without `within`.** The app's `Each` has no
  `.within(...)` container. The tester found every row by walking
  `Runtime::children` below the `Each`. It kept each descendant whose own
  address `find(list, [key])` resolves back to itself, then sorted the rows by
  `bounds().y()` (`tests/behaviors.rs:101-134`). This works but needs about 30
  lines. The tester asks for a `Driver` row helper, or a statement that
  `Selector::children` returns rows in display order.
- **"Nothing focused" was missed.** The tester looked for a way to clear focus
  on `Runtime` and found only `HostIntegration::clear_focus`, whose doc reads
  "Handles window focus loss". The tester took that to be a host-only method
  and left out the no-focus case. The `testing` module's own example uses
  `ui.clear_focus()` for exactly this case (`crates/pert-ui/src/lib.rs:244-260`
  at aca8df3), and `testing::*` re-exports `HostIntegration` for that purpose.
  So the method was documented, but the tester did not find it from the
  `Runtime` page. This is a discoverability lead, not a missing facility.
- **Getting a PNG from a capture.** The testing guide's capture section does
  not show `FrameImage::encode_png`. The tester found it on the `FrameImage`
  page. The kit's `guide/testing/index.html` has no `encode_png`.
- **Click spacing.** The tester added `advance(ClickCounter::DEFAULT_INTERVAL
  + 1 ms)` after every click, as the guide warns. The tester suggests a click
  variant that does this itself.
- **Time plausibility.** All FRICTION.md entries (19:40 to 19:42) lie between
  the brief (19:38:44) and the end of the session (19:42:34). The session was
  short. The summary records the first passing `cargo test` 2.0 minutes after
  the brief. FRICTION.md says no library source was read. The transcript was
  not used for this acceptance.
