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

### 19:05 — understand — annoyance

- **Tried:** Looked for list display order using `Selector::children` from the testing guide.
- **Expected:** A query over rows to preserve their displayed order for the append assertion.
- **Resolved by:** `QueryResult::components` documents incarnation order, not display order. `Each` documents its default internal `Column` keyed `items`; the tests will use `Runtime::children` on that column for ordered rows, preserving the application's existing paths and behavior.
- **Read:** `/kit/doc/pert_ui/guide/testing/index.html` (“List rows”), `struct.QueryResult.html`, `struct.Runtime.html#method.children`, `struct.Each.html`.
- **Diagnostic:** None; documentation lookup. No library source read and no application change needed.

### 19:06 — compile — annoyance

- **Tried:** Imported `pert_ui::prelude::*` and `pert_ui::testing::*`, then asserted a Remove control has `SemanticRole::Button`; also used `clear_focus()?` while testing the shortcut without focus.
- **Expected:** The testing imports to include the semantic role alongside `SemanticValue`, and focus clearing to return a result like focus setting.
- **Resolved by:** Explicitly import `pert_ui::SemanticRole`; remove `?` from `clear_focus`, which returns `()`. Compiler types resolve the latter; the similarly named enum suggestion does not resolve the missing import.
- **Read:** `/kit/doc/pert_ui/testing/index.html`, `/kit/doc/pert_ui/enum.SemanticRole.html`, `/kit/doc/pert_ui/struct.Runtime.html`.
- **Diagnostic:** `error[E0433]: cannot find type SemanticRole in this scope` at `tests/ui.rs:95:36`; suggestion: `SemanticValue::Button`. `error[E0277]: the ? operator can only be applied to values that implement Try` at `tests/ui.rs:225:13`, `clear_focus()?`, type `()`.

### 19:06 — runtime — annoyance

- **Tried:** After a successful add following refused adds, required the error label's semantic name to equal an empty string.
- **Expected:** `Some("")` for an empty label because the testing guide says static text is its semantic name.
- **Resolved by:** `Semantics` documents normalization: an empty name becomes absent. Check that the widget exists and has Label semantics, then interpret its absent name as empty text.
- **Read:** `/kit/doc/pert_ui/guide/testing/index.html` (“Reading what the runtime shows”), `struct.Label.html`, `struct.Semantics.html` (normalization paragraph).
- **Diagnostic:** `refused_add_keeps_fields_and_success_clears_the_error ... FAILED`; `Error: "missing name: [\"content\", \"error\"]"`. Seven other tests passed, including PNG rendering.

### 19:06 — runtime — annoyance (intentional acceptance check)

- **Tried:** Temporarily broke name trimming in `/work/bookmarks/src/lib.rs`: changed `name: entered_name.trim().to_owned()` to `name: entered_name.to_owned()`. Ran `cargo test --offline --test ui add_appends_and_trims_the_name -- --exact`.
- **Expected:** The UI-driven add test must fail when the visible bookmark name keeps surrounding spaces.
- **Resolved by:** Confirmed exit status 101 and the exact assertion failure below. Restored `src/lib.rs` byte for byte in a `finally` block immediately afterward. This is an intentional mutation demonstration, not a library defect. Full output is `/work/bookmarks/test-evidence/trim-mutation.txt`.
- **Read:** Application README behavior 1 and application `src/lib.rs` (no library source read).
- **Diagnostic:** `add_appends_and_trims_the_name ... FAILED`, `tests/ui.rs:91:9`, `assertion left == right failed`, `left: "  Guide  "`, `right: "Guide"`; `0 passed; 1 failed; 7 filtered out`.
- **Application changes:** None retained. No testability changes were necessary.

## Closing

- **Result:** All acceptance items met. `/work/bookmarks/tests/ui.rs` has seven behavior tests, one per README item, plus one CPU-rendered main-window PNG test. Final `cargo test`: **8 passed, 0 failed**, without a display; `cargo fmt --check` also passed. Capture: `/work/bookmarks/captures/main-window.png` (900 × 700). Temporarily disabling name trimming caused the expected UI assertion failure, recorded in `/work/bookmarks/test-evidence/trim-mutation.txt`; original source restored. The dependency already had `pert-ui = "0.3.75"` with `cpu` plus defaults. No persistent application behavior or testability changes, and no library source reads.
- **Concepts I had to learn:** Runtime and viewport; Driver settling and simulated input; addressable keys versus structural children; semantic names versus values; ordered children versus incarnation-ordered queries; application keyboard chords and focus; capture support, pending capture requests and CPU capture serving; empty semantic-name normalization.
- **What worked well:** The testing guide gives complete examples for the exact task: input, inspection, Each row paths, shortcuts and CPU captures. Existing `build` entry point and stable application keys made integration tests possible without exposing internal state. CPU rendering needed no OS services or extra image dependency. Runtime and Each docs explicitly explain ordered children and structural wrappers.
- **The three changes I want most:**
  1. Show an empty-label assertion in the testing guide and mention name normalization beside the static-text example.
  2. Re-export `SemanticRole` from `testing` beside `SemanticValue`, or show the explicit import in a role assertion example; the compiler's `SemanticValue::Button` suggestion was unhelpful.
  3. Add an ordered-row helper to Driver (or a default-Each ordered-row example) so append tests do not need to name the documented internal `items` container. The guide's named-container example is helpful but would require changing an existing application solely to inspect order.
