# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. Take each entry's time from the shell (`date +%H:%M`);
do not estimate it.

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

### 15:57 — docs-wrong — annoyance

- **Category:** docs-wrong
- **Tried:** Read the guide's *Getting started* chapter to learn how to depend on the crate.
- **Expected:** `pert-ui = "0.3.52"` from the registry, as the brief and the README's feature table (`pert-ui = { version = "…", features = [...] }`) suggest.
- **Resolved by:** Ignoring it; the existing `bookmarks/Cargo.toml` already depends on `pert-ui = { version = "0.3.51", features = ["cpu"] }` and resolves to 0.3.52 from the registry. Builds in 21 s.
- **Read:** `pert_ui/guide/getting_started` §"Run your first window": "Until a registry release exists, use a local checkout ... `pert-ui = { path = "../pert-ui/crates/pert-ui" }`". This contradicts the published crate.

### 15:57 — understand — annoyance

- **Category:** understand
- **Tried:** Reading the examples on `Runtime`, `Button`, `TextField`, `Semantics` in the `pert_ui` docs.
- **Expected:** Examples written against `pert_ui::…` (the crate I depend on).
- **Resolved by:** Inferring that `pert_ui_runtime::X` is re-exported as `pert_ui::X`. Most item examples say `use pert_ui_runtime::{...}`, while the guide uses `pert_ui::{...}`; a reader of the facade has to translate each one.
- **Read:** `struct.Runtime.html` (find_path, find_descendant, find), `struct.Button.html`, `struct.TextField.html`, `struct.Semantics.html`.

### 15:57 — discover — annoyance

- **Category:** discover
- **Tried:** Looked for a way to *perform* a `SemanticAction::Activate` on a button from a test (like an accessibility "press").
- **Expected:** An `invoke`/`perform_action` method next to the semantics.
- **Resolved by:** `SemanticAction` docs say explicitly that inspection is read-only and "tools drive components through the same input the runtime accepts from a host". So buttons are clicked with `Runtime::pointer_input` at their `Runtime::bounds`, or focused and activated with Enter. Clear answer, just one more hop.
- **Read:** `enum.SemanticAction.html`, `struct.Runtime.html#method.pointer_input`, `struct.PointerEvent.html`, `enum.PointerKind.html`.

### 15:59 — understand — annoyance

- **Category:** understand
- **Tried:** A throwaway test that mounted `bookmarks::build`, flushed and printed every `ComponentInspection` (key, path, role, semantic name and value), to learn the addresses I would drive.
- **Expected:** From the app README, rows directly under `list` (`list/1/name`) and a label's text in `Semantics::value`.
- **Resolved by:** The dump showed `list/items/1/name`, and the Row's own `.key("row")` replaced by the id; the `Each` page documents both ("Each keys with that key, replacing the element's own"; "in a Column keyed `items`"). Label text is the semantic `name` (`value` is `None`), documented on `Label` ("the full text stays the label's name in inspection") — fine, but `Semantics::value` says "Presented or edited text", so I first looked there. An empty label reports `name: None`, not `Some("")`. The dump itself was the most useful thing I did: one loop over `ui.inspect().components` showed the whole addressable tree.
- **Read:** `struct.Each.html`, `struct.Label.html`, `struct.ComponentInspection.html`, `struct.Semantics.html`, `enum.SemanticValue.html`.

### 15:59 — runtime — slow

- **Category:** runtime
- **Tried:** To replace a field's text, my helper clicked the field, sent Ctrl+A via `command_key_input` and then `ui.text_input(text)`. For `text = ""` the old text stayed (`left: "https:/example.com", right: ""`).
- **Expected:** Committing empty text over a selection to replace it with nothing, as a paste of "" would. In hindsight it is reasonable that an empty commit is a no-op, but `Runtime::text_input` does not say so.
- **Resolved by:** The helper now sends Ctrl+A then Backspace (`CommandKeyInput::new(KeyCode::Backspace, …, CommandKey::Named(KeyCode::Backspace))`) and asserts the field is empty before typing. Also noted: after `Runtime::focus` the caret sits at the start (the `TextField` example's `"New Example"`), so a plain `text_input` *prepends*; a "set this field's text" test helper or a documented "replace all" recipe would save this detour.
- **Read:** `struct.Runtime.html#method.text_input`, `struct.TextField.html`, `enum.CommandKey.html`, `struct.CommandKeyInput.html`, `struct.Modifiers.html`.

### 15:59 — (no friction) — note

- 6 of 7 behavior tests passed on the first run, the 7th after the helper fix above. Clicking by `bounds` centre with `pointer_input` (Move, Press, Release), `Runtime::find` on the inspection path, and `command_key_input` + `native_modifiers` for Ctrl+L all worked exactly as the *diagnostics* guide and `Runtime` docs describe. No source reading so far.

### 15:59 — (no friction) — note

- **Tried:** CPU capture of the main window: `set_capture_support(true)`, `request_capture(CaptureRequest::new(CaptureTarget::window(&ui.primary_window())))`, `flush`, `CpuRenderer::new().serve_captures(&mut ui)`, `pending.wait()?.value?`, `image.encode_png()`.
- **Result:** Worked first time, copied almost verbatim from the *diagnostics* guide §Screenshots. 800×900 PNG written to `target/tmp/bookmarks-main-window.png`, which shows the added row, the error line and the typed name. The only thing I had to choose was where to write it; I used `env!("CARGO_TARGET_TMPDIR")`. A small nit: the guide says to "call it after each flush", and it is easy to miss that `request_capture` needs the *following* flush before `serve_captures` sees the job (the guide example does show this order).
- **Read:** `guide/diagnostics` §Screenshots, `cpu/struct.CpuRenderer.html`, `enum.CaptureTarget.html`.

### 15:59 — (acceptance 3) — note: a broken behavior fails its test

- **Break 1 (behavior 1, Add trims the name):** in `src/lib.rs` line 94 changed `name: entered_name.trim().to_owned(),` to `name: entered_name.clone(),`. `cargo test` → only `add_appends_trimmed_bookmark` failed:
  ```
  thread 'add_appends_trimmed_bookmark' panicked at tests/behaviors.rs:27:5:
  assertion `left == right` failed
    left: [("Rust", ...), ("Crates", ...), ("  Docs  ", "https://docs.rs"), ("Example", "http://example.com")]
   right: [("Rust", ...), ("Crates", ...), ("Docs", "https://docs.rs"), ("Example", "http://example.com")]
  test result: FAILED. 6 passed; 1 failed
  ```
- **Break 2 (behavior 7, Ctrl+L):** changed `KeyChord::primary('l')` to `KeyChord::primary('k')`. Only `clear_filter_shortcut` failed: `panicked at tests/behaviors.rs:200:5 ... left: "rust", right: ""`.
- Both reverted by copying back the original file; `diff` is empty and all 8 tests pass again.

### 15:59 — (no change) — note: application changes

- No change to the application was needed: `bookmarks::build(&mut Runtime) -> WidgetId` already made it testable headlessly, and `Cargo.toml` already enabled the `cpu` feature. `src/` is unchanged. I added only `tests/support/mod.rs`, `tests/behaviors.rs` and `tests/capture.rs`.
- I never read the library's source under `/kit/vendor`.

## Closing

- **Result:** All acceptance items met. (1) Seven tests in `tests/behaviors.rs`, one per README behavior, all passing under `cargo test` with no display; they act only through `pointer_input`, `text_input`, `command_key_input`/`native_modifiers` and read only through `inspect()`/`find`/`children`. (2) `tests/capture.rs` renders the main window with `CpuRenderer` through the capture API and writes `target/tmp/bookmarks-main-window.png` (800×900, checked visually). (3) Two deliberate breaks (name trimming, Ctrl+L binding) each failed exactly the matching test; output recorded above, source restored.
- **Concepts I had to learn:** the `Runtime` as a headless host (viewport, `flush`); keyed elements and addressable inspection paths (`Runtime::find`, `ComponentInspection::path`), including `Each`'s `items` container; inspection `Semantics` (label text = `name`, field text = `value`); input as a host delivers it (`pointer_input` at `bounds`, `focus`/`text_input`, `command_key_input` with `native_modifiers` for commands vs. `key_input` for components only); capture hosting (`set_capture_support`, `request_capture`, flush, `CpuRenderer::serve_captures`, `Pending::wait`).
- **What worked well:** The *diagnostics* guide chapter is close to a recipe for exactly this task: the keyboard section explains why a shortcut test must use `command_key_input`, and the screenshot example ran unmodified. `Runtime::find` with inspection paths, plus a one-loop dump of `inspect().components`, made the tree self-describing. Errors and lookups are typed and documented (`Ok(None)` vs `AmbiguousKey`). 6 of 7 tests passed on the first run; no source reading needed. Build of the whole stack was ~21 s.
- **The three changes I want most:**
  1. A small test-driver facade (e.g. `ui.click(&id)`, `ui.set_text(&id, "…")`, `ui.press_chord(chord)`), or at least a guide section "Testing an application" that shows these three helpers. Everyone writing UI tests will rebuild the same click-at-bounds-centre and select-all-then-Backspace helpers I wrote.
  2. Make label text easier to find in inspection: document on `SemanticValue::Text` / `Semantics::value` that labels report their text as `name`, and/or offer a `text()` accessor on `ComponentInspection`. Also document that `text_input("")` is a no-op and that focus places the caret at the start.
  3. Write the item examples against the `pert_ui` facade (`use pert_ui::…`) rather than `pert_ui_runtime`/`pert_ui_cpu`, and remove the stale "Until a registry release exists, use a local checkout" note from *Getting started*.
