# Initial triage, round 3 group H: head-only tasks 1–5, both testers

This is a read-only check of sources and documentation against the frozen head kit (`kit-r3-head`, commit 32dd9c4, 0.3.88). I ran no commands, edited nothing and delegated nothing. All execution evidence below comes from recorded runs, completed acceptances and the author's existing probes (`runs/triage-r3/probes`). It is not a usability verdict.

## Path prefixes and run IDs

**Paths:**
- `H/` = `kit-r3-head/vendor/pert-ui-runtime-0.3.88/src`
- `HF/` = `kit-r3-head/vendor/pert-ui-0.3.88/src`
- `RUN/` = `plans/usability/runs/<run>`
- `ACC/` = `~/pert-ui-usability/acceptance-<run>`

**Runs:**

| ID | Run |
|---|---|
| C1 | r3-1-head-claude |
| X1 | r3-1-head-codex |
| C2 | r3-2-head-claude-retry1 |
| X2 | r3-2-head-codex |
| C3 | r3-3-head-claude-retry1 |
| X3 | r3-3-head-codex |
| C4 | r3-4-head-claude-retry1 |
| X4 | r3-4-head-codex |
| C5 | r3-5-head-claude-retry1 |
| X5 | r3-5-head-codex |

Every run's `summary.md` records kit 32dd9c4f667b and 0.3.88.

**Evidence tags:**
- **[F]** the tester's friction log
- **[D]** the run's `diagnostics.md`
- **[T]** a test the tester wrote, whose pass the harness recorded or the acceptance re-ran
- **[A]** a check the acceptance verifier added and ran
- **[S]** source or documentation inference only
- **[Pr]** an author probe (`probes/output.txt`)

## Acceptance status

| Run | Verifier | Grade |
|---|---|---|
| C1 | first session exit 1 (usage limit), resumed session exit 0, original unchanged | met (not yet author-adopted) |
| C2 | exit 0 | met, author-adopted (`RUN/acceptance/author-review.md`) |
| C3 | exit 0 | met (not adopted) |
| C4 | first session exit 1, resumed session exit 0 | met (not adopted) |
| C5 | exit 0 | met (not adopted) |
| X1–X5 | **no acceptance directory exists** | not graded |

- No final grade is inferred for the Codex runs. Their "all items met" lines are self-reports.
- **Acceptance-record precision:** the C2 and C3 reports cite source lines that do not match the submitted files.
  - C3 cites `src/headless.rs:94-119` for a 42-line file, and `picker.rs:52-323` for a 292-line file.
  - C2 cites `build` at `:173-275`; it is actually at `:170-272`.
  - The grades rest on tests that ran, but these citations should be corrected before adoption.
  - The C2 report also repeats the tester's `ListDropPosition` claim, which the source contradicts (see C2#3).

---

## 1. Per-run entries

### C1 (5 entries; #1, #3 and #4 written later)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | `TextStyle::Title` guess (E0599) | rustc listed the variants (`RUN/diagnostics.md:3-9`) | tester error | none |
| 2 | `Select` semantics can't be reached from its key; `address_children` is empty; `find(..,["anchor","select"])` fails | **Verified.** `Select::inspect` sets `semantics = None` (`H/select.rs:1711-1716`). The anchor and list are structural children (`:1662-1666`). The only hint is the `placeholder` example, which searches the snapshot for a component keyed `"select"` (`:1622`). [Pr] P6 asserts NoSemantics and an empty `address_children` (`probes/lib.rs:237-261`). The acceptance verifier needed the same `find_descendant` workaround. | **new N3** (missing capability and documentation gap), slow | approved parts checkpoint (rev 3, `control` part) plus an inspection section on `Select` |
| 3 | `ScrollAxes::Both` "removes the width bound", so `Align::Stretch` overflows | **Contradicted.** Both offers the viewport width (`H/views.rs:516-531`) and places the child at that width (`:702-709`). The doc says "offering viewport width" (`:303-304`). The likely real cause: the vertical bar's lane sits above child drawing (`:312-315`). Group right borders are visible in `ACC/evidence/captures/clean.png` (no overflow) and hidden in `refused.png` (overflow). | documented behavior; tester misattribution | pending probe PR-1; optional note or gutter |
| 4a | `Spacing::Medium` guess | E0599 | tester error | none |
| 4b | `Column` has no `padding` | Verified: Row and Column have gap, justify and align only (`H/containers.rs:1062-1103`), with no pointer to `Container::padding` (`:174`) | optional suggestion | a "padding: wrap in `Container`" sentence |
| 5 | Disabled button hard to tell apart | **Verified.** The fill is `ColorRole::Button` whether enabled or not (`H/controls.rs:810-821`); only the text changes, `Muted` against `OnButton` (`:832-840`). `Button`'s doc says nothing about the disabled look (`:432-441`). `Muted` contrast is promised only against Background, Surface and SurfaceHover (`H/runtime/theme.rs:426-429`). The acceptance verifier concurs. | **new N4** | design or theme decision plus a doc line |
| W1 | Make Select reachable; document `anchor/select` and `popup/options` | Gap verified. Those keys are structural, so they cannot be documented as addresses. | N3 | part route |
| W2 | Disabled look | — | N4 | — |
| W3 | `ScrollAxes::Both` note; padding | Cause contradicted; padding pointer is optional | optional | — |
| acc | Right borders clipped under the vertical scrollbar | Documented overlay (`H/views.rs:312-315`) | optional suggestion | PR-1 |
| acc | A refused draft alone enables Revert, not Apply | Application's choice | app behavior | none |

**Disclosure:** the probe test `tests/explore.rs` (first successful test, `summary.md:13`) was removed from the submission. Its printed claims can't be checked from the record; P6 corroborates them.

### X1 (3 entries)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | Settings recipe covers refused drafts and Revert | Matches `HF/guide/state.md` (settings form) | positive note | — |
| 2a | `normalize` closure gets an owned `String` | The signature is `Fn(String) -> Result<String, InputRejection>` (`H/text_fields.rs:358-361`); E0308 suggested `&s` | tester error | optional typed example |
| 2b | `SegmentedControl` needs `Eq` | E0277 named the fix. The cascaded E0599 note "is not a pert-ui component… owns its data" says nothing about the unmet `T: Eq` (`RUN/diagnostics.md:48-74`). | tester error; optional diagnostic suggestion | optional |
| 3 | `.into()` ambiguity in `map` (E0283) | The guide lists both `ReadSignal<String>` and `ReadSignal<RichText>` (`HF/guide/controls.md:43`) | tester error | optional example |
| W1–W3 | Typed normalizer; `Clone + Eq` note; annotated derived label | Accurate | optional suggestions | — |

**Acceptance leads (pending):**
- F4: `admit_close` is called only after Enter, never with an unconfirmed typed draft.
- F6: the tests assert the control's description, not the `FieldHandle` status, so they don't show `on_rejection` firing.
- The "kept old value" checks read model signals (`main.rs:382`, `:389`).

### C2 (6 entries plus a positive note; #2 and #3 written later)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | The guide still mentions a checkout path and checkout-only examples | `HF/guide/getting-started.md:9-28` leads with the registry form; the checkout text is accurate for an unpublished library | F1's release-time residue; not obstructing | release checklist (F1 destination) |
| 2a | `Runtime::find` returns `Option` | Documented; `Driver::find` covers tests | optional suggestion | decline a new API |
| 2b | Boxed `ApplicationError` plumbing | `From<&str>`, `From<String>` and `From<Box<dyn Error+Send+Sync>>` exist, with a type-level example `ApplicationError::from("…")` (`H/error.rs:45`, `:74-93`) | documented route missed; feeds N6 | N6 |
| 3 | "`ListDropPosition<'_>` borrows its key", so it can't be built from an owned key | **Contradicted.** `ListDropPosition` is an owned, `Clone` enum with no lifetime (`H/list_drag.rs:46-61`). Only `ListDrop<'a>` borrows, and only its `items`. `position()` returns `&ListDropPosition` (`:65-84`). The real residue: `ListDrop` has no public constructor (`pub(crate)` fields), so a `reorder(&ListDrop)` helper can't be shared with buttons. | partly tester error; optional recipe suggestion | optional: recipe factors `reorder(store, keys, &ListDropPosition)`; no new list API |
| 4 | `register_command` doesn't say focused components take a chord first | Verified: its rustdoc has no routing pointer (`H/runtime/commands.rs:572-606`). `reorderable` and `on_activate` do state it (`H/virtual_list.rs:1107-1116`, `:1154-1163`). | **new N10** (minor doc cross-link) | link to the routing order |
| 5 | Guessed `Driver::list_rows` | The intro says "lists a list's rows in order" without a name (`HF/guide/testing.md:11-15`); "List rows" names `address_children` (`:319-327`) | optional suggestion | name the method |
| 6 | The `try_reply`-not-`wait` trap | Documented (R2-28 repair) | documented behavior; optional | decline |
| note | The editable-list recipe matched the task | `HF/guide/layout.md:502-684` | positive (F14) | — |
| W1 | Public "move active row" trigger, or an owned position | Owned position already exists; a trigger is an optional capability | contradicted / optional | — |
| W2 | `find` returning an error | Existing route: `ok_or(ApplicationError::from("…"))?` | optional; decline | N6 pointer |
| W3 | Routing pointer; intro names the method | — | N10 / optional | — |

**Acceptance observations:**
- **"No per-row selected flag":** true per row, but the list reports `Choice{selected}` (`H/virtual_list.rs:2714-2721`).
- **"Done" label beside each checkbox:** this follows the recipe's own layout (`HF/guide/layout.md:572`), so it is adjacent to R2-9 but not a recurrence.
- **Existing TODO corroborated by code:** `C2 lib.rs:46` and `X2 main.rs:35` both write the dead `.key("row")` copied from the recipe (`layout.md:583-588`). This is TODO.md:148-160.

### X2 (1 entry)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | Passing a named `build -> Result<_, RuntimeError>` to `App::new` (E0271) | The diagnostic names the right types (`RUN/diagnostics.md:3-22`). The documented route `|ui| Ok(build(ui)?)` is only in `HF/guide/state.md:298` and `HF/app/headless.rs:45`; `App::new`'s doc doesn't show it (`HF/app.rs:561-597`). | **new N5** (doc gap); event 1 of 3 | one sentence on `App::new` |
| W1 | `App` wrapper around the runtime recipe | — | N5 | — |
| W2 | Recipe with entry field and count | — | optional | — |
| W3 | Link to `CommandScope` routing | — | optional, adjacent to N10 | — |

**Acceptance leads (pending):**
- Alt+Up and Alt+Down work only through the list's own reorder, so they need list focus; no application command is registered.
- The move buttons swap in the full vector even under a filter (`main.rs:94-105`).

### C3 (4 entries; #2 written later)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | `component::*` lacks `ApplicationError` (E0433, no hint) | **Verified contract breach.** The guide says `component` holds every name an implementation writes (`HF/guide/custom-widgets.md:15-24`) and its examples write `ApplicationError` (`:267`, `:282`) under both globs (`:240`). The runtime `component` list omits it (`H/lib.rs:613-663`). | **F3 recurrence** (glob membership, no rustc route) | add it to `component` plus the membership test (author decision, consistent) |
| 2 | f32 geometry vs f64 values | Documented; the tester rounds | documented behavior | optional note |
| 3a | `Viewport` not in the prelude | By design in `testing` and `component` (`HF/lib.rs:218`, `H/lib.rs:647`); the code was an app-side headless mode; E0433 had no hint | F3 discovery symptom (same cell) | pointer; decline prelude widening |
| 3b | `ok_or("…")?` into `RuntimeError` | E0277 named `RuntimeError` and listed `From<ApplicationError>` first; documented route `ApplicationError::from(&str)` | **F11 exercised and passed** | decline `From<&str> for RuntimeError` |
| 4 | "docs-wrong": testing module "closed", yet `DrawCommand` is needed | **Partly.** The module page lists paint output (`DisplayList`) as an exception (`HF/lib.rs:287-291`). The guide intro omits it (`HF/guide/testing.md:21-27`). The guide's own widget test matches `DrawCommand` (`custom-widgets.md:398-399`), so "passed on" doesn't hold for that use. | **new N2** (doc precision), F3 discovery symptom | list the exception in the guide, or reword it |
| W1 | `ApplicationError` in `component`; `Viewport` in the prelude; `DrawCommand` in `testing` | First verified; second by design; third → N2 | split | — |
| W2 | f32 note | — | optional | — |
| W3 | `From<&str>` for `RuntimeError` | A documented route exists | decline | N6 |

The acceptance verifier's own error (TimeReversed) was in its test, not the app.

### X3 (2 entries)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | Guessed `guide/custom_components` path | Navigation guess; pandoc error | tester error | none |
| 2a | `ApplicationError` missing from `component` | As C3#1 (`RUN/diagnostics.md:3-9`) | F3 recurrence | as C3#1 |
| 2b | E0271 with a named `mount` | As X2#1 | N5, event 2 | N5 |
| W1–W3 | Two-axis example; imports summary incl. `ApplicationError`; named mount example | — | optional / covered by the F3 fix / N5 | — |

**Acceptance lead (pending):** `cancel_input` (`picker.rs:144-152`) is never executed. No focus-loss case exists; Escape goes through `keyboard_event`.

### C4 (4 entries)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | Are `Form`/`Field` keys path segments? | **Partly contradicted.** `Runtime::find` states the general rule, "an element contributes its key… built-in containers add a segment only where they document one" (`H/runtime/tree.rs:260-268`), and `Driver::find` links to it (`H/testing.rs:600-601`). The guide only covers Each, VirtualList and Table. | documented behavior | optional guide sentence |
| 2 | Empty editor's `value`: `None` or `Text("")`? | **Verified gap.** `TextField` always reports `Some(Text(self.text))` (`H/text_fields.rs:3677-3683`). Executed: X2 asserts `Some(Text(""))` (`main.rs:281-284`) and X4 does too (`ui.rs:131-132`). The guide covers only the empty label (`HF/guide/testing.md:36-37`, `H/testing.rs:747-749`). `Semantics` says "absent… never an empty placeholder" (`H/diagnostics.rs:632-633`). | **new N7** (doc gap), adjacent to R2-13 | state it next to the label rule |
| 3 | Positive note (tests first try) | — | positive | — |
| 4 | Deliberate breaks of trim and Ctrl+L | Acceptance reproduced both, plus its own third break (`ACC/evidence/02-04`) | acceptance item 3 evidence | — |
| W1 | Wrapper segment statement | — | partly contradicted | optional |
| W2 | Empty-editor value | — | N7 | — |
| W3 | `Driver` text helper | One-line `semantics_at`-style helper already taught (`testing.md:43-50`); X4 asks too | optional (F2 residue wish, not a recurrence under round 2's rule) | — |

### X4 (1 entry)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | Deliberate trim break | `RUN/evidence/mutation-test.log` | acceptance item 3 evidence | — |
| W1–W3 | Fixture example; text helpers; capture-to-PNG helper | The capture route is documented | optional | — |

### C5 (6 entries; #3 and #4 written later)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | `UiSender`, `SendError`, `Mailbox`, `Capacity` not in the prelude | Membership is by design (the `live` row at `HF/lib.rs:132`), but: E0425 and E0433 gave no hint, and rustc **suggested `std::sync::mpsc::SendError`** (`RUN/diagnostics.md:3-46`). The guide paragraph has no `use` line (`HF/guide/state.md:160-164`). | **F3 recurrence** (discovery symptom); N8 | pointer and example, not prelude widening |
| 2 | "`ApplicationError` only has new/get/downcast_ref" | **Contradicted** (`H/error.rs:74-93`, example at `:45`). The E0277 on `new(String)` gives no route. `new`'s doc mentions only the boxed conversion (`:52-58`). Round 2 recorded the same misstep (5-head-claude-2). | **new N6** (discoverability) | a line in `new`; no `msg` |
| 3 | Mailbox handler/model cycle needed an `Rc<RefCell<Option<…>>>` slot | `Runtime::mailbox` has no example (`H/runtime.rs:4503-4530`); neither has the guide. **But** `Mailbox`'s own page has one (`H/mailbox.rs:228-248`), so "the only example is under `keep_alive`" is contradicted. The handler is `FnMut(T)` over captured state and the sender is clonable. X5 avoids the slot by capturing the sender in its Start closure (`X5 lib.rs:282-292`). | application design choice; N8 recipe. Not R2-1 (initializer state isn't involved). | N8 |
| 4 | Is whole-source replacement the live-row path? | Documented: `cell` runs again for mounted rows on each replacement (`H/table.rs:264-280`) | documented; optional cost sentence | N9 |
| 5 | `set_source` from a cell's own button callback | Works [T, A]. The rustdoc is one line (`H/table.rs:687-694`); `BorrowConflict`'s meaning is on `Signal::set` (`H/signal.rs:559-569`). | **new N9** (doc precision, minor) | — |
| 6 | Positive note | — | positive | — |
| W1 | Prelude or `use` line, plus a mailbox recipe | — | N8 (decline prelude) | — |
| W2 | `set_source` from cells; live cost | — | N9 | — |
| W3 | `msg` / `From<String>` | `From<String>` exists | contradicted; decline `msg` | N6 |

**Acceptance observation:** toasts overlay the table. This is documented: a bottom-right stack over the content (`H/notifications.rs:368-372`), so it is by design.

### X5 (1 entry)

| # | Entry | Verification | Class | Disposition |
|---|---|---|---|---|
| 1 | Initializer `|ui| mount(..)` (E0308) | Accurate `?` suggestion (`RUN/diagnostics.md:3-16`) | N5, event 3 | N5 |
| W1 | Mailbox recipe | — | N8 | — |
| W2 | Named mount helper in the `App` example | — | N5 | — |
| W3 | Cross-link the table example, idle checks and notification lifetimes | A toast's deadline is documented (`H/testing.rs:524-528`), as is the `Banner` pointer (`H/notifications.rs:409-410`) | optional | — |

**Disclosure:** the closing note and W3 say "a persistent banner worked". The code uses a plain `Label` (`lib.rs:298`).

**Acceptance leads (pending):**
- Name sort is lexicographic, so it fails at 10 or more jobs (`lib.rs:127`).
- Failure is deterministic every 5th job (`lib.rs:151`) rather than random one in five.
- A failed bar shows a normal running status (`lib.rs:87`; `captures/jobs.png`).

### Event counts and timing

- **Event counts:** 33 friction entries, 39 events after the splits, 30 wishes and 16 acceptance observations. They map to **1 recurring finding (F3, 3 runs)** and **9 distinct new findings (N2–N10)**. All other entries are tester errors, documented behavior, positive notes or optional suggestions.
- **Timing:** the "(written later)" entries give approximate times only.
- **C2 builds:** one failed build with no diagnostics. It matches the deliberate command break described in #4; the friction log gives no time for it.

---

## 2. Recurrence matrices (R = recurred, P = passed by actual exercise, — = not reached)

### F1–F34

| F | C1 | X1 | C2 | X2 | C3 | X3 | C4 | X4 | C5 | X5 |
|---|---|---|---|---|---|---|---|---|---|---|
| F1 | P | P | P | P | P | P | — | — | P | P |
| F2 | P | P | P | P | P | P | P | P | P | P |
| F3 | P | P | P | P | **R** | **R** | P | P | **R** | P |
| F4 | P | — | — | — | — | — | — | — | — | — |
| F5 | P | P | — | — | — | — | — | — | — | — |
| F6 | P | — | — | — | — | — | — | — | — | — |
| F7a | — | — | P | P | — | — | — | — | — | — |
| F7b | — | — | P | P | — | — | — | — | — | — |
| F7c | — | — | — | — | — | — | — | — | — | — |
| F8 | P | P | P | P | — | — | P | P | P | P |
| F9 | — | — | — | — | — | — | P | P | — | — |
| F10 | P | — | P | — | P | P | P | P | P | P |
| F11 | — | — | — | — | P | — | — | — | — | — |
| F12 | — | — | — | — | P | — | — | — | — | — |
| F13 | P | P | — | — | — | — | — | — | P | P |
| F14 | — | — | P | P | — | — | — | — | — | — |
| F15 | — | — | P | P | — | — | — | — | — | — |
| F16 | — | — | P | P | — | — | — | — | P | P |
| F17 | — | — | — | — | — | — | — | — | — | — |
| F18 | — | — | — | — | P | P | — | — | — | — |
| F19 | — | — | P | P | — | — | — | — | — | — |
| F20 | P | — | — | — | P | — | — | — | P | — |
| F21 | P | P | — | — | — | — | — | — | — | — |
| F22 | — | — | — | — | — | — | — | — | — | — |
| F23 | — | — | — | — | — | — | — | — | — | — |
| F24 | — | — | — | — | P | P | — | — | — | — |
| F25 | — | — | — | — | — | — | — | — | — | — |
| F26 | — | — | P | P | — | — | — | — | — | — |
| F27 | — | — | P | P | — | — | — | — | — | — |
| F28 | P | P | P | P | — | — | — | — | — | — |
| F29 | — | — | — | — | — | — | — | — | — | — |
| F30 | — | — | P | — | — | — | — | — | — | — |
| F31 | — | — | — | — | — | — | — | — | — | — |
| F32 | — | — | — | — | — | — | — | — | — | — |
| F33 | — | — | — | — | — | P | — | — | — | — |
| F34 | — | — | P | P | P | P | P | P | — | — |

#### Reasons for each R and P

- **F1 P** [S, code]: each run wrote its own registry requirement (`Cargo.toml:7`: C1, X1, C2, C3, C5 and X5 use 0.3.88; X2 and X3 use "0.3").
  - C2#1 notes the checkout paragraph as a release-time residue, not an obstruction.
  - Task 4 is — because the manifest is the fixture's.
- **F2 P (all)** [T]: every run drives the app through `Driver`. Test files:
  - `C1 tests/acceptance.rs`, `X1 main.rs:335-490`, `C2 tests/headless.rs`, `X2 main.rs:254-389`
  - `C3 tests/picker.rs`, `X3 tests/headless.rs`, `C4 tests/behaviors.rs`, `X4 tests/ui.rs`
  - `C5 tests/monitor.rs`, `X5 tests/headless.rs`

  Their helpers are one-line find-plus-semantics wrappers, which round 2's rule treats as P. The text-helper wishes from C4 and X4 are recorded as optional.
- **F3 R:**
  - **C3** [F#1, #3a, #4; D]: `ApplicationError` is outside the documented `component` glob; `Viewport` and `DrawCommand` got E0433 with no hint (`diagnostics.md:3-17`, `:44-50`).
  - **X3** [F#2a; D]: `ApplicationError` (`diagnostics.md:3-9`).
  - **C5** [F#1; D]: mailbox names gave no hint, and rustc suggested the wrong `std::sync::mpsc::SendError` (`diagnostics.md:11-22`).
- **F3 P:** these runs wrote F3's own names or round 2's testing names through the documented globs, with no import error:
  - C1: `Semantics`, `InspectValue`, `SemanticRole` (`acceptance.rs:56-83`)
  - X1: `Semantics` (`main.rs:315`)
  - C2 and X2: `KeyChord`, `CommandKey`, `CommandScope`, `Modifiers`, `ListDrop`, `ListDropPosition` (C2 `lib.rs:213-269`, X2 `main.rs:107-214`)
  - C4 and X4: semantics names
  - X5: `live` names by explicit path; its only error was E0308
- **F4 P C1** [T, re-run by acceptance]: an unconfirmed "33" draft makes `admit_close` return KeepOpen and commits 33 (`acceptance.rs:365-373`).
- **F5 P** [T, A]: `HeadlessApp::admit_close` (C1 `acceptance.rs:339`, `:350-390`, plus acceptance v5; X1 `main.rs:398-401`, `:430`).
- **F6 P C1** [T]: an `IntegerInput` refusal of "150", "0", "abc" or "7.5" sets the `FieldHandle` status to error through `on_rejection` (`acceptance.rs:194-202`, `lib.rs:123-133`).
- **F7a and F7b P** (C2 [T, A], X2 [T]):
  - `selected_shown` is derived from `filter.items()` (C2 `lib.rs:181-184`, X2 `main.rs:145-149`).
  - `HiddenActive::Clear` clears a hidden selection, and Remove/Delete then do nothing (C2 `headless.rs:160-168` plus the acceptance filter test; X2 `main.rs:345-349`).
- **F8 P** [T]: label text is read as `Semantics::name`:
  - C1 `acceptance.rs:131-134`; X1 `main.rs:348`, `:395`; C2 `headless.rs:14-17`; X2 `main.rs:234-237`
  - C4 `behaviors.rs:26-28`; X4 `ui.rs:30-32`; C5 `monitor.rs:71-76`; X5 `headless.rs:6-8`
- **F9 P** (task 4) [T]: `address_children` of an `Each` without `within`, with no workaround (C4 `behaviors.rs:49-53`, X4 `ui.rs:63-70`).
- **F10 P** [T]: `CaptureTarget::window(&…primary_window())`:
  - C1 `capture.rs:9-11`; C2 `headless.rs:252`; C3 `headless.rs:34-36`; X3 `headless.rs:131-135`
  - C4 `capture.rs:22-24`; X4 `ui.rs:225-227`; C5 `monitor.rs:281-283`; X5 `headless.rs:128-132`
- **F11 P C3** [D]: a failed `?` into `RuntimeError` named the right target and listed `From<ApplicationError>` (`diagnostics.md:19-42`).
  - The E0271/E0308 errors in X2, X3 and X5 are signature mismatches, not failed conversions: —.
- **F12 P C3** [T, A]: a focus-loss restore runs `cancel_input`, and a failed restore would be reported through `Notifier` (`picker.rs:205-216`; `picker.rs` test `:128-141`).
  - X3 implements the same route (`picker.rs:144-152`) but never executes it: — (stricter than round 2; flagged for the author).
- **F13 P:**
  - C1 and X1 [T, A]: `cx.spawn` from Apply (C1 `lib.rs:253-265` plus the acceptance FIFO off-thread proof; X1 `main.rs:168-185` plus the artifact `test-output/save/settings.json` holding "retry_me").
  - C5 and X5 [T], qualified: a worker thread is started from a Start callback through the documented mailbox route (`state.md:30`); not `EventContext::spawn`.
- **F14 P** (C2, X2) [F]: both followed the editable-list recipe (C2 positive note; X2 closing).
- **F15 P** [T]: `on_activate` defers `TextCommand::BeginEdit` (C2 `lib.rs:203-212`, `headless.rs:101-113`; X2 `main.rs:160-169`, `:311-316`).
- **F16 P** [T]:
  - Task 2: controlled `Checkbox` and `InlineText` (C2 `lib.rs:33-44`, X2 `main.rs:27-34`).
  - Task 5: `ProgressBar::new(status)` per cell (C5 `lib.rs:459`, X5 `lib.rs:87`).
- **F18 P** [T, A]: Escape in `keyboard_event` restores (C3 `picker.rs:184-194`, test `:110-111`; X3 `picker.rs:127-134`, test `:84-85`).
- **F19 P** [D]: `register_command(...)?;` drops the `CommandId` and no warning was recorded (C2 diagnostics "None"; X2 only `dead_code`).
- **F20 P** [F]: the run used or cited an item-page example with facade imports:
  - C1: the `Select::placeholder` example.
  - C3: the `ColorRole::FocusRing` and `ColorPicker` notes.
  - C5: the `keep_alive` example's `pert_ui::{Capacity, Runtime}` line.
  - Runs that read only the guide: —.
- **F21 P** [T]: the refused control's description carries the field error, not "Unsaved changes" (C1 `acceptance.rs:176-185`; X1 `main.rs:383-385`).
- **F24 P** [T, A]: `Slider` semantics with a text value plus `x`/`y` number properties (C3 `picker.rs:268-290`; X3 `picker.rs:184-200`).
- **F26 P** [T]: the search field at `["tasks","filter-field","filter"]` (C2 `headless.rs:150`; X2 `main.rs:340`).
- **F27 P** [T, A]: a single click on a title selects the row (C2 `headless.rs:69-73`; X2 `main.rs:293-295`).
- **F28 P** [code, T]: custom `PartialEq` signal types are written (C1 `Settings` `lib.rs:19`; X1 `main.rs:12`; C2 `Task` `lib.rs:10`; X2 `main.rs:3`).
- **F30 P C2**, type-level only: `ApplicationError::from(Box<dyn Error+Send+Sync>)` compiles (`lib.rs:276-279`); the path is never executed.
- **F33 P X3** [code, T]: `size_limits` returns `SizeLimits::exact(Size::from([260,260]))` (`picker.rs:69-71`), and the test asserts square bounds (`headless.rs:47`).
- **F34 P** [T]: modifiers only through `Driver::chord`:
  - C2 `headless.rs:167`, `:187`, `:205`; X2 `main.rs:300`, `:368-377`
  - C3 `picker.rs` test `:161-168`; X3 `:98-105`
  - C4 `behaviors.rs:274`; X4 `ui.rs:211`
- **Not reached, and why:**
  - F7c: no padded query was exercised by tester or verifier.
  - F22: task 4's Ctrl+L with an editor focused isn't one of the chords F22 names.
  - F6 X1: no `FieldHandle` check (probe PR-5).
  - F4 X1: no unconfirmed draft at close.

### R2-1 to R2-28

| R2 | C1 | X1 | C2 | X2 | C3 | X3 | C4 | X4 | C5 | X5 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | P | P | P | P | — | P | — | — | P | — |
| 2 | P | P | — | — | — | — | — | — | — | — |
| 3 | P | P | P | P | — | — | — | — | — | — |
| 4 | P | P | P | P | — | — | — | — | — | — |
| 5 | P | P | — | — | P | P | — | — | — | — |
| 6 | — | — | P | P | — | — | P | P | P | P |
| 7 | — | — | P | — | — | — | — | — | — | — |
| 8 | — | — | P | P | — | — | — | — | — | — |
| 9 | — | — | — | — | — | — | — | — | — | — |
| 10 | — | — | P | P | — | — | — | — | — | — |
| 11 | P | — | — | — | P | — | P | P | — | P |
| 12 | P | — | P | — | P | P | P | P | P | P |
| 13 | — | — | — | — | — | — | P | P | — | — |
| 14 | — | — | — | — | P | P | — | — | — | — |
| 15 | — | — | — | — | P | P | — | — | — | — |
| 16 | — | — | — | — | — | P | — | — | — | — |
| 17 | — | P | — | — | — | — | — | — | — | — |
| 18 | — | — | — | — | — | — | — | — | — | — |
| 19 | — | — | — | — | — | P | — | — | — | — |
| 20 | — | — | — | — | — | — | — | — | P | — |
| 21 | — | — | — | — | — | — | — | — | P | P |
| 22 | — | — | — | — | — | — | — | — | P | P |
| 23 | — | — | — | — | — | — | — | — | P | P |
| 24 | — | — | — | — | — | — | — | — | P | P |
| 25 | — | — | — | — | — | — | — | — | — | — |
| 26 | — | — | — | — | — | — | — | — | P | — |
| 27 | — | — | — | — | — | — | — | — | P | P |
| 28 | P | — | P | — | P | P | P | P | P | P |

#### Reasons

- **R2-1 P:** `App<S>` with `HeadlessApp::runtime_and_state_mut` and `on_close` reading state, with no slot:
  - C1 `lib.rs:332-342`, `acceptance.rs:98`; X1 `main.rs:267-283`, `:340`; C2 `lib.rs:291-293`, `headless.rs:35-36`
  - X2 `main.rs:221-223`, `:256-257`; X3 `lib.rs:60-62`, test `:33-34`; C5 `lib.rs:609-611`, `monitor.rs:306-307`
  - C3, task 4 and X5 test `build`/`mount` directly: —.
  - C5's slot (#3) is a mailbox/model cycle, not initializer state.
- **R2-2 P:** Revert with `FormFields::discard_drafts` clears refused drafts.
  - C1 `lib.rs:287` [T `acceptance.rs:259-267`; A v3]
  - X1 `main.rs:197` [T `:380-415`]
- **R2-3 P:** n-ary or tuple `combine` (C1 `lib.rs:155-178`; X1 `main.rs:110-123`; C2 `lib.rs:181-184`; X2 `main.rs:145`).
- **R2-4 P** [D]: `cx.spawn(...)?;` or `cx.defer(...)?;` with no receipt warning (C1 `lib.rs:253`, X1 `:168`, C2 `:205`, X2 `:162`).
- **R2-5 P** [T]: number-input `Number` semantics read correctly (C1 `acceptance.rs:71-76`, `:124`; X1 `main.rs:328-333`; C3 test `:197-218`; X3 test `:67-74`).
- **R2-6 P** [T]: `address_children` of a `VirtualList` (C2, X2), an `Each` (C4, X4) and a `Table` (C5 `monitor.rs:78-81`, X5 `headless.rs:112-116`).
- **R2-7 P C2** [T, re-run by acceptance]: the `["2","title","editor"]` child holds focus (`headless.rs:108-109`).
- **R2-8 P** [F, T]: the list consumes Alt+arrow once and Enter, as `reorderable` documents (C2 #4, `headless.rs:203-211`; X2 `main.rs:300-316`). C2's remaining wish is N10.
- **R2-10 P:** `KeyChord::new` with a `Modifiers` field literal (C2 `lib.rs:261-269`; X2 `main.rs:199-205`).
- **R2-11 P** [T]: `Driver::clear_focus` (C1 `capture.rs:48`; C3 test `:138`; C4 `behaviors.rs:295`; X4 `ui.rs:208`; X5 `headless.rs:148`).
- **R2-12 P** [T]: the capture ends with `encode_png` (C1, C2, C3, X3, C4, X4, C5 and X5 capture code as listed under F10).
- **R2-13 P** [T]: an empty error label has name `None` (C4 `behaviors.rs:125`, `:157`; X4 `ui.rs:122`).
- **R2-14 P** [code, D]: a component-only glob in the widget module gave no unused-import warning (C3 `picker.rs:10-11`; X3 `:2-3`). The missing `ApplicationError` is counted under F3.
- **R2-15 P** [T, A]: the recipe's live writes, Escape, `cancel_input` and `FocusRing` (C3 acceptance acc2/acc5; X3 code plus test).
- **R2-16 P X3** [T]: enables `serde` and serializes the snapshot (`headless.rs:143-147`).
- **R2-17 P X1**, type-level: `.gap(8)` and `.gap(16)` compile through `From<u16>` for `Spacing` (`main.rs:224`, `:255`).
- **R2-19 P X3:** `Size::from([260,260])` inside `exact` (`picker.rs:70`).
- **R2-20 P C5** [T, A]: toast messages read from the stack's semantic items (`monitor.rs:104-109`; acceptance item 3).
- **R2-21 P** [T, A]: bar plus percent in a narrow row, with an empty validation report (C5 `lib.rs:458-462`, `monitor.rs:293-294`; X5 `lib.rs:86-92`, `headless.rs:150-155`).
- **R2-22 P:** `<table>/<row>/<column>/<content>` paths (C5 `monitor.rs:122-139`; X5 `headless.rs:59`, `:77-85`).
- **R2-23 P:** row height from `ControlHeight` with no validation issue (C5 `lib.rs:557-560`; X5 `lib.rs:285-288`).
- **R2-24 P** [T, A]: `quiet_for` (C5 `monitor.rs:117`, `:146-147`, `:199-201`; X5 `headless.rs:45`, `:149`).
- **R2-26 P C5** [A]: `Cancelled` or `Failed` keeps the reached fraction (`lib.rs:311-312`; acceptance item 3 "fraction kept").
- **R2-27 P** [T, A]: `click_item` on headers sorts (C5 `monitor.rs:88-94`; X5 `headless.rs:98`).
- **R2-28 P:** `try_reply` after the serving flush (all capture runs).
- **R2-9, R2-18 and R2-25 are — everywhere.** X5 used a `Label`, not a `Toast` or `Banner`.

**Totals for group H:** F3 recurred 3 times (C3, X3, C5). No other F finding and no R2 finding recurred. Codex P cells rest on tester tests only, with acceptance still pending.

---

## 3. Deduplicated findings

| ID | Finding | Class / severity | Runs (events) | Evidence | Repair direction | Related |
|---|---|---|---|---|---|---|
| F3 (reopened) | (a) `component` omits `ApplicationError` although the guide promises it. (b) By-design names (`Viewport`, `DrawCommand`, the `live` mailbox names) give no or wrong rustc route. | recurrence; annoyance | C3, X3, C5 (5 events) | `H/lib.rs:613-663`; `HF/guide/custom-widgets.md:15-24`, `:267`; diagnostics as cited | (a) add to the runtime `component` module plus `modules.rs` membership; (b) pointers in state.md and the testing guide; no prelude widening | author-decisions.md:59-65; round-2 P2/D3 |
| N2 | Testing guide intro omits the module page's paint-output exception; matching `DrawCommand` contradicts "passed on" | doc precision; annoyance | C3 | `HF/guide/testing.md:21-27` vs `HF/lib.rs:287-291`; `custom-widgets.md:398-399` | name the exception, or route paint inspection through `component` | author-decisions.md:66-69 |
| N3 | `Select` owner has no semantics and its ComboBox anchor has no address; no inspection section on the page | missing capability plus doc gap; slow | C1 (plus the acceptance workaround) | `H/select.rs:1662-1666`, `:1711-1716`; [Pr] P6 | approved `control` part plus a `Select` "Inspection and tests" section, with a test reading the ComboBox through it | round-3-design-parts.md rev 3; TODO.md:170-180; task-6 N3 |
| N4 | Disabled `Button` differs from enabled only by `Muted` text on the same fill; `Muted` contrast isn't specified against `Button` | presentation and docs; annoyance | C1 (plus acceptance) | `H/controls.rs:810-840`, `:432-441`; `H/runtime/theme.rs:426-429`, `:662-668` | a theme-level disabled treatment (design checkpoint), documented on `Button`; check both palettes (PR-2) | none |
| N5 | `App::new` doesn't show a named `RuntimeError` builder (`|ui| Ok(build(ui)?)`) | doc gap; annoyance | X2, X3, X5 (3 events) | `HF/app.rs:561-597`; route only at `state.md:298` and `app/headless.rs:45` | one sentence plus an example on `App::new` and the layout recipe; no new API | adjacent to F11 |
| N6 | `ApplicationError::new`'s doc points only to the boxed conversion; `From<String>` and `From<&str>` are missed | doc gap; annoyance | C5, C2 (2 events); round 2 had the same | `H/error.rs:52-58` vs `:45`, `:74-93` | mention them in `new`; decline `msg` and `From<&str> for RuntimeError` | author-decisions.md:70-73 |
| N7 | An empty editor's value is `Some(Text(""))`; undocumented, and `Semantics`' "never an empty placeholder" can mislead | doc gap; annoyance | C4 (plus X2 and X4 executions) | `H/text_fields.rs:3677-3683`; `H/diagnostics.rs:632-633`; `testing.md:36-37` | state it with the empty-label rule, on `Semantics::value` and `TextField` | adjacent to R2-13 |
| N8 | Mailbox guide paragraph has neither example nor `use` line; `Runtime::mailbox` has no example | doc gap; annoyance | C5, X5 (wishes) | `HF/guide/state.md:160-164`; `H/runtime.rs:4503-4530` (cf. `mailbox.rs:228-248`) | a state-guide recipe: keep the `Mailbox` in app state, handler over signals or a model, sender passed to the starter, Full/Closed handling | — |
| N9 | `TableController::set_source` doc is minimal: callable from a cell's own control, cost per mounted row, `BorrowConflict` meaning | doc precision; annoyance | C5 | `H/table.rs:687-694`; `H/signal.rs:559-569` | expand the rustdoc | round-2 P8 |
| N10 | `register_command` has no pointer to the key-routing order | doc cross-link; annoyance | C2 (X2 W3) | `H/runtime/commands.rs:572-606` | link the routing section | R2-8, F22 |

**Optional suggestions** (verified, not findings):
- Column/Row padding pointer.
- Name `address_children` in the testing intro.
- `SegmentedControl` unmet-bound E0599 note.
- An f32/f64 note.
- Recipe: shared `reorder(keys, &ListDropPosition)` and move buttons.
- Recipe checkbox naming.
- `Driver` text helper (C4 and X4).
- Capture helper.
- Scrollbar lane gutter.
- Cross-link between toast lifetime and idle checks.

**Declined as new APIs** (a documented route exists):
- `From<&str> for RuntimeError`
- `ApplicationError::msg`
- prelude membership for mailbox names and `Viewport`
- an owned `ListDropPosition` (already exists)

**Kit issues:** none new. C2#1 belongs to F1's release-time destination.

## Pending probes (proposed, not run)

- **PR-1 (C1#3):** 960×640 viewport. `ScrollArea` with `Both` and with `Vertical`, each holding a stretched `Column` of `Group::titled`, with content shorter and taller than the viewport. Assert each group's border-box right edge is 960. CPU-capture x = 952–959 to check whether the border pixels are hidden only when the vertical bar shows.
- **PR-2 (N4):** in Dark and Light, read `OnButton`, `Muted` and `Button`. Capture enabled and disabled buttons and compute each text's contrast on the button fill.
- **PR-3 (N7, optional):** record the value and `text` property of an empty `TextField`, and of an `IntegerInput` with nothing saved.
- **PR-4 (C2#3, optional):** compile probe constructing `ListDropPosition::Before(..)` and cloning `drop.position()` from an outside crate.

**Codex acceptance checks owed:**
- X1: F4 with an unconfirmed typed draft; F6 via `FieldHandle` status.
- X2: Alt+Up/Down with focus outside the list.
- X3: F12 via `clear_focus` during a drag.
- X5: name sort with 10 or more jobs, random one-in-five failure, failed bar status.

Until those run, all five Codex runs remain ungraded.
