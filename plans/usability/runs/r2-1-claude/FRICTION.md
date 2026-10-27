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


### 19:01 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** in a test with `use pert_ui::prelude::*; use pert_ui::testing::*;` (the "one use block covers it" from the testing guide chapter) I wrote a helper returning `Result<Semantics, _>`, the return type of `Driver::semantics`.
- **Expected:** `Semantics` to be among "the other names a test writes" that `testing` re-exports, since `SemanticValue` is.
- **Resolved by:** adding `use pert_ui::Semantics;`.
- **Read:** guide/testing (intro paragraph), `testing::Driver::semantics`.
- **Diagnostic:** `error[E0425]: cannot find type `Semantics` in this scope` at `tests/headless.rs:37:63`; the help suggested adding a type parameter, which is wrong.

### 19:01 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** getting the root `WidgetId` of content an `App` initializer mounted, from a `HeadlessApp`, to look up controls in tests.
- **Expected:** an accessor such as `HeadlessApp::root()` or `Runtime::roots()`.
- **Resolved by:** the `Rc<RefCell<Option<..>>>` slot pattern shown in the guide's "close policy" example; it works but every app has to thread a slot out of its initializer just to be testable.
- **Read:** guide/testing "The close policy", `testing::HeadlessApp`, `Runtime` method list.


### 19:03 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** checking in a test that a refused value leaves the old value in effect, by reading the control's `Semantics::value` after Enter.
- **Expected:** both editors to report the same thing. `IntegerInput` reports the bound number (70 after typing 150), while `TextField` with a normalizer reports the refused draft text ("bad name!"), so the "old value kept" check for text has to go through other evidence (the dirty marker, Escape restoring the text).
- **Resolved by:** asserting on the dirty marker and on Escape restoring the value; nothing in the docs states which of draft or bound value `Semantics::value` carries for each editor.
- **Read:** `Semantics::value`, `TextField` "Normalization", `IntegerInput`.

### 19:03 — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** verifying headlessly that the maximum width is "shown with its unit" (`IntegerInput::unit("px")`).
- **Expected:** the unit in the control's semantics value or description.
- **Resolved by:** a `Detail::Full` query: `ComponentDetails::properties` holds `unit`, `display_value` and `shows_display`. The property names are not documented on `IntegerInput`; I found them by dumping the map. Also `InspectValue` (the property value type) is, like `Semantics`, not in `prelude`/`testing`.
- **Read:** `Detail`, `InspectionQuery`, guide/testing "Reading what the runtime shows".


### 19:04 (written later) — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `cx.spawn(work, deliver)?;` in the Apply button's callback, as in the `EventContext::spawn` example.
- **Expected:** no warning, since the example binds `let _receipt = cx.spawn(..)?` without saying the receipt matters.
- **Resolved by:** `let _receipt = ...`. The `#[must_use]` note ("the receipt is the only report of a `Cancelled` operation; failures also propagate from flush") explained it well, so this cost under a minute.
- **Diagnostic:** `warning: unused `TaskReceipt` that must be used` at `src/lib.rs:247:13`.

## Closing

- **Result:** All five acceptance items are met, and `cargo test` checks each one headlessly (`tests/headless.rs`, 7 tests, all passing), using `App::headless`, `testing::Driver` input (`replace_text`, `key`, `click`, `wait_until`), `Driver::semantics`/inspection queries and `HeadlessApp::admit_close`:
  1. `shows_loaded_values`: the three titled groups, all five controls, the buttons and the marker, with the values loaded from a JSON file. The width shows its `px` unit (checked through inspection properties).
  2. `refuses_invalid_input_and_keeps_old_value`: an invalid prefix (space, too long) and quality 150 are refused. The explanation is the field's description, `invalid` is set, the marker stays clean and Apply stays disabled, and Escape shows the old value.
  3. `apply_and_revert_follow_dirty_state`: both buttons are disabled when clean and enabled when dirty. Revert restores the applied values, and editing back to the applied value is clean again.
  4. `apply_saves_in_background` writes the file through `EventContext::spawn`, the state becomes clean, and a restart loads it. `failed_save_is_reported_and_stays_dirty` uses a path under a regular file: "Save failed: … Not a directory" is shown and the state stays dirty.
  5. `closing_is_refused_while_dirty`: `admit_close` returns Close when clean and KeepOpen while dirty, including for an unconfirmed focused draft that admission saves. The notice is shown and goes away after Revert.
  `captures_for_review` writes CPU captures (`target/tmp/*.png`), which I looked at. The native window itself was never run (no display).
- **Concepts I had to learn:** `Element` keys and `mount_tree`; `Signal`/`ReadSignal` with `map`/`combine` for derived state (edited settings → dirty → button permission); `Permission` from a `ReadSignal<bool>` via `enabled`; draft mode of editors (normalizer / range → save on Enter or focus loss, refused drafts kept); `InputRejection` + `on_rejection` → `FormFields`/`FieldHandle` → `Field` status; `Group::titled`; `EventContext::spawn` (work off-thread, deliver on the UI thread); `App::on_close` + `NativeCloseAction` and the close-admission order; `App::headless`, `Driver`, `Runtime::find_descendant`, semantics vs. `Detail::Full` properties; CPU captures.
- **What worked well:** The guide answered almost every question in the order I needed it. The testing chapter's close-policy example matched my task almost exactly. The per-item pages state their timing and draft rules precisely. The code compiled on the first build apart from one import, and every test passed on its first run. Semantics (`disabled`, `invalid`, `description` taken from the Field status) made the assertions read like the acceptance criteria. `FrameImage::encode_png` made visual review possible with no extra crates. I never needed to read the library's source.
- **The three changes I want most:**
  1. Put `Semantics`, `InspectValue` and the other types that `Driver`/query results return in `pert_ui::testing` (or the prelude), as the testing chapter's "one use block covers it" promises.
  2. Document which value `Semantics::value` carries while a draft is pending (TextField: the draft; IntegerInput: the bound number), and list the inspection property keys (`unit`, `display_value`, `shows_display`, …) on each control's page.
  3. Give `HeadlessApp` (or `Runtime`) a way to reach the mounted roots, so a test does not need an `Rc<RefCell<Option<_>>>` slot threaded out of the initializer.
