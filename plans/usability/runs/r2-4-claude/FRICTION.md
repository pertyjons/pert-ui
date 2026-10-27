# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### <time> — <category> — <severity>

- **Category:** one of
  - *discover*: could not find how to do it;
  - *understand*: found it, but it was unclear how it behaves;
  - *compile*: a compiler diagnostic that did not point to the fix;
  - *runtime*: behavior differed from the documentation or your expectation;
  - *docs-wrong*: the documentation is incorrect;
  - *missing*: there seems to be no way to do it;
  - *workaround*: you did it in a way you think the library did not intend;
  - *source*: you read the library's source (say why).
- **Severity:** *blocker* (you could not continue without it), *slow* (it
  cost noticeable time), *annoyance*.
- **Tried:** what you did.
- **Expected:** what you expected to happen or to find.
- **Resolved by:** what resolved it, or "unresolved".
- **Read:** the documentation pages or items you read for it.
- **Diagnostic:** for *compile*, the exact error code and the first lines.

### 19:40 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Reading the rows of `list` in display order. The app's `Each` has no `.within(...)` container, so the guide's recipe for display order (`Runtime::children` of a container given with `within`) does not apply without changing the app.
- **Expected:** a way to list an `Each`'s mounted rows in order with no app change. The guide (guide › testing › "List rows") says the default container "is internal structure and adds no segment", and that `Selector::children` of the Each's path returns the rows, but it does not say whether those come back in display order.
- **Resolved by:** walking `Runtime::children` from the `Each` down, keeping each descendant `d` where `Runtime::find(list, &[key(d)]) == Some(d)` (that is, a row's own address), then sorting by `Runtime::bounds(..).y()`. It works, but it is more code than I expected, and I used bounds as a stand-in for "display order".
- **Read:** guide/testing (List rows), `Runtime::children`, `Runtime::find`, `Runtime::bounds`, `Rect`.

### 19:40 — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** For behavior 7 ("wherever focus is") I wanted a case with nothing focused. I looked for `Runtime::clear_focus`/`blur`. The only match is `HostIntegration::clear_focus` ("Handles window focus loss"), which is a host trait method.
- **Expected:** a test-level way to drop focus.
- **Resolved by:** testing three focus positions instead: in the filter, in the Name field, and on the Add button (through `Runtime::focus`). I did not test "nothing focused".
- **Read:** `Runtime` method list (search for "focus").

### 19:40 — (positive note) — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** wrote all 7 behavior tests from guide/testing and `testing::Driver` alone: `replace_text`, `click`, `chord(KeyChord::primary('l'), KeyCode::KeyL)`, `semantics().name` / `.value`, and `find` with key paths such as `["content","form","name-field","name"]` and `["content","list","1","remove"]`. All of it compiled and passed on the first build. The only trap was the double-click note in "Clicking": the guide warns about it, so I call `advance(ClickCounter::DEFAULT_INTERVAL + 1ms)` after every click.
- **Expected:** n/a. I log it to record what worked.
- **Resolved by:** n/a
- **Read:** guide/testing, `testing::Driver`, `testing::TestError`, `Semantics`, `SemanticValue`, `Runtime::find`.

### 19:41 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** the CPU capture test (`tests/capture.rs`), copied from guide › testing › "A capture" and `cpu::CpuRenderer::serve_captures`. I wrote the file with `FrameImage::encode_png` to `$CARGO_TARGET_TMPDIR/bookmarks.png` (`target/tmp/bookmarks.png`). It passed on the first run, and the image shows the form, the status line and both rows.
- **Expected:** The guide never says how to get a PNG out of a capture. I found `FrameImage::encode_png` by opening the `FrameImage` page. One sentence in the guide's capture section would save that step.
- **Resolved by:** `FrameImage::encode_png`.
- **Read:** guide/testing (A capture), `cpu::CpuRenderer`, `FrameImage`.

### 19:42 — runtime (deliberate) — annoyance

- **Category:** runtime
- **Severity:** annoyance
- **Tried:** acceptance 3. I copied `src/lib.rs` to a backup, made one change at a time, ran `cargo test --test behaviors` after each, and then restored the original (`diff` confirms it is identical). Results:
  1. Name not trimmed (`entered_name.clone()`): `add_appends_bookmark_with_trimmed_name` FAILED at tests/behaviors.rs:151, `left: [... Shown { key: "3", name: "  Docs  ", ... }]`, `right: [... name: "Docs" ...]`.
  2. Error not cleared on a successful add: `refused_add_shows_error_and_keeps_fields` FAILED at :190, `left: "The URL must start with http:// or https://."`, `right: ""`.
  3. URL field not cleared: `successful_add_clears_fields` FAILED at :201, `left: "https://docs.rs"`, `right: ""`.
  4. Remove deletes everything (`list.clear()`): `rows_show_bookmarks_and_remove_deletes_one` FAILED at :214 (`left: []`). `status_counts_bookmarks` also failed, with `Error: NotFound { path: ["content", "list", "2", "remove"] }`.
  5. Case-sensitive filter: `filter_narrows_list_by_name` FAILED at :236 (`left: []`, `right: ["1", "3"]`). The status and clear-filter tests also failed.
  6. Status ignores the filter: `status_counts_bookmarks` FAILED at :260, `left: "3 bookmarks"`, `right: "1 of 3 bookmarks"`.
  7. Clear filter command does nothing: `clear_filter_command_empties_filter` FAILED at :281, `left: "rust"`, `right: ""`.
- **Expected:** each break makes its behavior's test fail. It did, every time.
- **Resolved by:** n/a. I restored the original `lib.rs`, and all 8 tests pass again.
- **Read:** —

## Closing

- **Result:** All three acceptance items are met. `cargo test` in `/work/bookmarks` runs `tests/behaviors.rs`, which has one test per README behavior (7, all passing), and `tests/capture.rs`, which renders the main window with `CpuRenderer` and writes `target/tmp/bookmarks.png` (passing). Breaking each of the 7 behaviors one at a time made its test fail (entry 19:42). I made **no changes to the application**: `src/lib.rs`, `src/main.rs` and `Cargo.toml` are untouched, because `Cargo.toml` already enabled `cpu` and `build(&mut Runtime)` was already testable. I did not read any library source.
- **Concepts I had to learn:** `Runtime` plus `Viewport` as the headless harness. `testing::Driver` and its "every action settles" rule (one flush 16 ms later). Key paths and `Runtime::find`: only application keys count, and structural levels add no segment. Static text is the semantic `name` and editor content is the `value`. `Each` rows are addressed by the item key and replace the row element's own key. Click spacing against `ClickCounter::DEFAULT_INTERVAL`. `Driver::chord` needs an explicit physical `KeyCode`. Capture support with `request_capture` and `CpuRenderer::serve_captures`, then `FrameImage::encode_png`.
- **What worked well:** the guide's testing chapter is excellent. Every snippet I needed was there, it compiled, and it matched the behavior. The README's key list combined with `Runtime::find` key paths made locating widgets trivial. `Driver` errors are typed and readable, for example `NotFound { path: [...] }`. The whole suite runs in about 0.2 s.
- **The three changes I want most:**
  1. A documented way to list an `Each`'s rows in display order when the app does not use `within`, such as a `Driver::rows(&each)`, or an explicit statement that `Selector::children` results are in display order.
  2. A test-level "drop focus" on `Runtime`/`Driver` (today only `HostIntegration::clear_focus` exists), so "wherever focus is" can include "nowhere".
  3. A line in the capture section of the testing guide showing `encode_png` and where to write the file. Optionally, a `Driver::click` variant that also advances past the double-click interval, since separate single clicks are the common case in tests.
