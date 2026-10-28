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

### 17:04 — compile — annoyance (written later)

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `Label::new(..).style(TextStyle::Title)` for a heading (a guess).
- **Expected:** Some title style.
- **Resolved by:** E0599 "no variant ... named `Title`"; `enum.TextStyle.html` lists `Heading`. Quick.
- **Read:** `enum.TextStyle.html`.
- **Diagnostic:** `error[E0599]: no variant, associated function, or constant named `Title` found for enum `pert_ui::TextStyle``

### 17:06 — discover — slow

- **Category:** discover
- **Severity:** slow
- **Tried:** In a headless test, `Driver::semantics(&driver.find(&root, &[..., "format-field", "format"])?)` on the
  element that holds my `Select<Format>` (to check that it shows the format loaded from the file).
- **Expected:** The dropdown's semantics (a ComboBox with the chosen label), as every other control I
  mounted (TextField, IntegerInput, Toggle, Button) reports on its own element.
- **Resolved by:** Got `TestError::NoSemantics { key: "format" }`. `Driver::address_children` of the Select
  is empty, and `Driver::find(&select, &["anchor", "select"])` is `NotFound`. A recursive walk with
  `Runtime::children` + `InspectionQuery ... Detail::Full` showed the structure `format/anchor/select`
  (ComboBox, value `Choice { label: Some("JPEG") }`) and `format/popup/options`.
  `Runtime::find_descendant(&select, "select")` returns it. The only hint was the `Select::placeholder`
  example, which searches the snapshot for a component keyed `"select"` without saying why.
  The `Select` page has no "addresses and tests" section like `Table` has.
- **Read:** guide/testing ("Reading what the runtime shows"), `struct.Select.html` (whole page),
  `testing/enum.TestError.html#variant.NoSemantics`.

### 17:08 — runtime — annoyance (written later, 17:10)

- **Category:** runtime
- **Severity:** annoyance
- **Tried:** `Column::align(Align::Stretch)` on the page column inside `ScrollArea::new(..).axes(ScrollAxes::Both)`
  (the guide's getting-started pattern), to give the three titled `Group`s one width.
- **Expected:** Groups as wide as the window.
- **Resolved by:** In the CPU capture the groups ran past the window's right edge (right border cut off).
  With two-axis scrolling the content has no width bound, so Stretch has nothing to fill to. A vertical-only
  `ScrollArea::new` fixed it. Getting-started recommends `.axes(ScrollAxes::Both)` for standalone forms without
  saying that it unbounds the width for `Align::Stretch`.
- **Read:** guide/layout "Growing, shrinking and stretching rows and columns", guide/getting_started.

### 17:08 — compile — annoyance (written later, 17:10)

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `Column::padding(Spacing::Medium)` (a guess, by analogy with `Group::padding`).
- **Expected:** Padding around the page.
- **Resolved by:** E0599 twice. `struct.Column.html` has only `gap`/`justify`/`align`; padding needs a
  `Container`, which also clips and adds an address segment, so I dropped the padding.
- **Read:** `struct.Column.html`, `enum.Spacing.html`.
- **Diagnostic:** `error[E0599]: no method named `padding` found for struct `pert_ui::Column` in the current scope`

### 17:09 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Looked at CPU captures (`CpuRenderer::serve_captures`) of the clean state (Apply/Revert disabled)
  and the dirty state (both enabled) to check that the button state is visible.
- **Expected:** An obviously different look for disabled buttons.
- **Resolved by:** At first glance I thought Apply was drawn disabled while its semantics said enabled.
  A pixel comparison showed the same background in both states and only the label brighter when enabled
  (text about (233,236,242) versus (194,200,211)). It works, but the disabled state is hard to see in the
  default dark theme. I found no statement of the disabled look on `struct.Button.html` or in the appearance guide.
- **Read:** guide/testing "A capture".

## Closing

- **Result:** All five acceptance items are met and shown by headless tests in `tests/acceptance.rs`
  (8 tests: loaded values in every field/group/button; refused prefix and quality with the explanation shown
  and the old value kept; Apply/Revert enabled only when dirty, Revert restoring; a successful save writing
  the file and a reload showing it; a failed save to a path under a regular file reported and still dirty;
  close refused while dirty, including for an unsaved focused draft and a refused draft). All of them go through
  `App::headless`, `Driver` and semantics/inspection. `tests/capture.rs` renders CPU captures that I checked by eye.
  The native binary builds but was not run (no display). I never read library source under `/kit/vendor`.
- **Concepts I had to learn:** `Element` keys and key paths; `Signal`/`ReadSignal` and `ReadSignal::combine`;
  `Field` + `FormFields`/`FieldHandle` (status, `set_modified`, `discard_drafts`); editor drafts
  (TextField with `normalize` and IntegerInput hold a draft until Enter or focus loss; `on_rejection`,
  `on_modified`); `Permission` through `.enabled(ReadSignal<bool>)`; `EventContext::spawn` (worker, then
  delivery on the UI thread); `App<State>` with `on_close` / `NativeCloseAction`; `HeadlessApp`
  (`runtime_and_state_mut`, `admit_close`); `Driver` (`find`, `replace_text`, `key`, `click`, `wait_until`,
  `semantics`); `Semantics` and `InspectionQuery` properties; CPU captures.
- **What worked well:** The guide's "A settings form" section in the State chapter is almost exactly this task
  (dirty state, Revert discarding drafts, Apply through a worker, close policy, and a headless test), so the
  design was quick. The testing chapter (Driver, `wait_until`, `admit_close`) worked as documented, first try.
  The refusal messages from `IntegerInput::range` ("Enter a number from 1 to 100") come for free, and `Field`
  shows them under the control with an outline. Semantics dumps (`Debug` on `Semantics`) made exploring easy.
- **The three changes I want most:**
  1. Make a `Select`'s semantics reachable from its own element (or through `Driver::find` / `address_children`),
     and document its internal addresses (`anchor/select`, `popup/options`) the way `Table` documents its addresses.
  2. Make the disabled button look clearly different in the default theme, or document what it looks like.
  3. In getting-started, say that `ScrollAxes::Both` removes the width bound (so `Align::Stretch` overflows), and
     give `Column`/`Row` a `padding` builder, or point to `Container` from their pages.
