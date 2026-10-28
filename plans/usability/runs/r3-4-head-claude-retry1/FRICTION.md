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

### 17:55 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Building key paths for `Driver::find` from the app README ("the field `name-field` with the text field `name`"). I could not tell from the docs whether `Form` and `Field` count as address segments, or (like an `Each`'s default container) are structure that adds none.
- **Expected:** A sentence in the guide or on `Field`/`Form` saying their keys are path segments.
- **Resolved by:** Guessing `["content", "form", "name-field", "name"]`; the first run confirmed it. Cost under a minute.
- **Read:** guide/testing ("Reading what the runtime shows", "List rows"), testing/struct.Driver.html.

### 17:55 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Reading an empty text field's content with `Driver::semantics(..).value`. The guide says an empty *label's* `name` is `None`, but does not say whether an empty *editor's* `value` is `None` or `Some(SemanticValue::Text(""))`. I also did not know the payload type of `SemanticValue::Text` (the guide only writes `"Ada".into()`).
- **Expected:** The empty-editor case stated next to the empty-label one.
- **Resolved by:** My `text_at` helper accepts both and calls `.to_string()` on the payload. It compiled and worked, so I did not look further.
- **Read:** guide/testing.

### 17:56 — none — (positive note)

- **Category:** none; recorded so the log shows the time spent.
- **Severity:** none
- **Tried:** Wrote all 7 behavior tests in `tests/behaviors.rs` straight from the testing guide (`Driver::find`, `replace_text`, `click`, `chord`, `clear_focus`, `address_children`, `semantics`). They compiled and passed on the first `cargo test`, with no diagnostics. Then wrote `tests/capture.rs` from the guide's "A capture" section; it passed on the first run, and the PNG at `target/tmp/bookmarks.png` shows the whole window correctly.
- **Resolved by:** n/a
- **Read:** guide/testing (whole chapter), testing/struct.Driver.html, cpu/struct.CpuRenderer.html.

### 17:56 — runtime — (acceptance 3: deliberately broken behavior)

- **Category:** runtime (deliberate)
- **Severity:** none
- **Tried:** In `src/lib.rs` I changed `name: entered_name.trim().to_owned(),` to `name: entered_name.clone(),` (breaks "Add trims the name") and ran `cargo test --test behaviors`.
- **Diagnostic:**
  ```
  test add_appends_a_trimmed_bookmark_at_the_end ... FAILED
  thread 'add_appends_a_trimmed_bookmark_at_the_end' panicked at tests/behaviors.rs:108:5:
  assertion `left == right` failed
    left: [("1", "Rust", ...), ("2", "Crates", ...), ("3", "  Docs  ", "https://docs.rs"), ("4", "Example", "http://example.com")]
   right: [("1", "Rust", ...), ("2", "Crates", ...), ("3", "Docs", "https://docs.rs"), ("4", "Example", "http://example.com")]
  test result: FAILED. 6 passed; 1 failed
  ```
  A second break, `KeyChord::primary('l')` changed to `primary('k')`, failed `ctrl_l_clears_the_filter_wherever_focus_is` at tests/behaviors.rs:280 (`left: "rust"`, `right: ""`).
- **Resolved by:** Restoring the original `src/lib.rs` from a copy (`diff` shows no difference). All 8 tests pass again.

## Application changes

None. `bookmarks::build(&mut Runtime)` already let the tests mount the app on a headless `Runtime`, and every component has a key, so I did not need to touch `src/lib.rs` or `Cargo.toml` (the `cpu` feature was already on). I did not read any library source under `/kit/vendor`.

## Closing

- **Result:** All acceptance items are met. (1) `tests/behaviors.rs` has one test per README behavior (7 tests: add, refused add, fields cleared, list/remove, filter, status, clear filter), all passing under `cargo test` with no display. (2) `tests/capture.rs` renders the main window with `CpuRenderer::serve_captures` and writes `target/tmp/bookmarks.png` (640×480; checked visually). (3) Breaking name trimming, and separately the Ctrl+L chord, made the matching test fail; the output is above. Total time was about 3 minutes, and nothing blocked me.
- **Concepts I had to learn:** `Runtime` plus `Viewport` as the headless harness; `testing::Driver` and the rule that every action settles; key-path addressing with `Driver::find`; semantics (`name` for static text, `value` for editor content); `Each` rows as address children keyed by item key (`address_children`); `KeyChord::primary` with an explicit `KeyCode` for `Driver::chord`; `clear_focus` for "nothing focused"; capture support → `request_capture` → settle → `CpuRenderer::serve_captures` → `Pending::try_reply` → `FrameImage::encode_png`.
- **What worked well:** The testing guide chapter covers this task almost one-to-one, with runnable examples of each step (clicks, replace_text, chords, list rows, the capture recipe including the `try_reply` vs `wait` pitfall and `CARGO_TARGET_TMPDIR`). `prelude` + `testing` were the only imports I needed. The crate index's "Headless tests" row pointed me straight to the chapter. Errors from `Driver` convert into `Box<dyn Error>`, so `?` works throughout.
- **The three changes I want most:**
  1. State whether wrapper widgets such as `Field`, `Form` and `ScrollArea` add a segment to a child's address. The guide covers this for `Each` and `Table` but not for these.
  2. Say what an empty editor's semantic `value` is (`None` or empty `Text`), next to the empty-label rule, and show `SemanticValue::Text`'s payload type in the guide.
  3. Add a `Driver` helper for reading text: "the name or text value at this path". Almost every test writes one, and the guide's `semantics_at`/`property` helpers suggest each user re-derives it.
