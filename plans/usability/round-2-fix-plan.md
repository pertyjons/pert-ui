# Round 2 fix plan

Status: **draft revision 4, 2026-10-27.** Revision 1 was reviewed by Codex
(needs revision); revision 2 answered every point and Codex's recheck left
three open, which revision 3 answers; Codex's second recheck left one
(D4's part traversal), which revision 4 answers; Codex's third recheck
found it ready for the maintainer's decisions
([review record](#review-record)). **Approved by the maintainer on
2026-10-28:** D1 to D15 as recommended; task 6 is written by a fresh agent
session as [round 3](#round-3-proposal) describes; the parts land on the
branch `round-2-fixes`; the design checkpoints are approved by the author and
Codex once Codex finds them ready, with the recommended options, under the
maintainer's delegation of 2026-10-28, and recorded as such. Each part's
state is in [Progress](#progress). Written against `developer-usability` at 26e6b5a (0.3.75); the
library code is unchanged at ceb5dc7, which Codex's recheck read.

## Purpose

Round 2 of the blind developer test ([results](../archive/developer-usability.md#round-2-results),
[triage](runs/triage-r2/)) reopened three round 1 findings and found
twenty-eight new ones. The maintainer decided on 2026-10-27 that every
reopened and new finding is fixed, and that breaking API changes are
welcome wherever they make the library better. This plan says how: the
policy choices first, as decisions for the maintainer, then one reviewed
part per version, then the order, a proposal for round 3 and a coverage
table.

AGENTS.md governs every part, in particular "Public API and documentation
are one deliverable", "Component API conventions" (an addition that offers
a new way to do something names what it retires or why none can be) and
"Publishing and compatibility" (each breaking change records its replacement
usage and migrates the repository's consumers in the same change).

Path prefixes: `R/` = `crates/pert-ui-runtime/src/`, `F/` =
`crates/pert-ui/src/`, `G/` = `crates/pert-ui/src/guide/`. Line numbers are
at 26e6b5a. Run names drop their `r2-` prefix, as the results do.

## Scope

- **Reopened round 1 findings:** F3 (glob membership), F9 (`Each` rows in
  display order), F16 (per-row state for display controls).
- **New findings:** R2-1 to R2-28, as the results table words them.
- **TODO.md entries pulled in,** each removed in the part that resolves it:
  - `R/described.rs` `Each::new`'s placeholder key (2026-10-27), with F9
    in [P5](#p5-rows-in-display-order-0380);
  - `F/lib.rs` `pub mod svg` glob collision (2026-10-27), with F3 in
    [P2](#p2-glob-membership-0377), whose collision review it belongs to;
  - the L3 native-probe `UnexpectedOverlap` noise in
    `crates/pert-ui-wgpu/src/native_smoke.rs` and
    `F/platform/smoke.rs` (2026-10-27), in
    [P11](#p11-application-state-from-the-initializer-0386), whose
    native runner change runs the native suite anyway;
  - `crates/pert-ui/examples/diagram_editor/app.rs` `close_requested`
    (draft policy rule 10, 2026-10-27), in
    [P12](#p12-form-drafts-and-the-settings-recipe-0387), which adds the
    form-level draft API that entry needs.
- **Follow-ups pulled in:** "Typed state handoff from `App::new`"
  (plans/follow-ups.md, "Application structure"), which R2-1 is; removed
  from the index in P11. "Controlled forms of more controls" and "Writable
  model projection" are touched by D1 and D10 and updated with round 2's
  evidence, not implemented, unless the maintainer chooses those
  alternatives.
- **Round 2's kit issues and tester errors** are listed in the coverage
  table: the kit issues are round 3 preparation, the tester errors need no
  library change.

### Out of scope

- **N8**, the `Inline*` controls as presentations of the standalone ones
  ([layering plan, "Not in this plan"](layering-plan.md#not-in-this-plan)),
  unless the maintainer pulls it in ([D15](#d15-n8)).
- App-author slice 9's tutorial ([its plan](../app-author-improvements.md)).
  The recipes this plan writes are its inputs, not its tutorial.
- Native evidence on Windows and macOS (TODO.md NATIVE-01 to NATIVE-05).
  Parts that change platform code record that evidence as missing.
- TODO.md entries not listed above, and the other follow-ups.
- Publication, and running round 3 itself, which this plan only proposes.

## Decisions for the maintainer

Each decision names the recommended option, the alternatives, the
trade-off and what each breaks. Parts below assume the recommended options;
a different choice changes only the parts it names.

### D1: F16, display controls that take a plain value

Today `ProgressBar::new` takes only `impl Into<ReadSignal<ProgressStatus>>`
(`R/progress.rs:158`), and so do `Spinner::new` (`R/progress.rs:363`),
`InlineReadout::new` and `InlineReadout::formatted`
(`R/inline_controls.rs:614`, `:622`) and the `Field::status` builder
(`R/field_status.rs:538`). A `ReadSignal` converts only from a signal or
another binding (`R/signal.rs:275-294`), and `TableItems::cell` and
`ListItems::build` get no state owner (`R/table.rs:259-271`), so every
task 5 run kept a `Signal<ProgressStatus>` per job or a `sibling` per cell
build beside the row data it republished. `ColorSwatch` and `Image` already
split the two: `ColorSwatch::new(Color)` and `ColorSwatch::bound(binding)`
(`R/color_swatch.rs:115`, `:129`), `Image::new(RasterImage)` and
`Image::bound(binding)` (`R/image.rs:88`, `:95`), the same `new`/`bound`
words contracts.md's [choice sources](../../docs/contracts.md#choice-sources)
use.

- **Recommended: fixed `new`, reactive `bound`.** Every built-in display
  control that presents one application value takes it at construction:
  `ProgressBar::new(ProgressStatus)` and `ProgressBar::bound(binding)`,
  `Spinner::new(bool)` and `Spinner::bound(binding)`. A text-presenting
  control takes a `TextSource` (`R/text_source.rs:61`) as `Label` does, so
  `InlineReadout::new(impl Into<TextSource>)` covers fixed and bound text
  and `formatted` stays the bound number form (a fixed number is formatted
  by the application). A presentation builder gets the fixed value under its
  name and the binding under the `_signal` name the builders already use
  (`View::visible_signal`, `MenuItem::checked_signal`): `Field::status(FieldStatus)`
  and `Field::status_signal(binding)`, likewise `Property::status`
  (`R/property_grid.rs:67`). The checkpoint lists every binding-only
  presentation input in the API listing and decides each; display controls
  that already split (`ColorSwatch`, `Image`) stay. Permission inputs
  (`enabled`, `editable` and the other `Permission`-typed builders) are not
  presentation inputs and keep their own composition rules. The rule is
  stated under contracts.md's "Ownership paths"
  (`docs/contracts.md:664-673`), not by widening "Choice sources", which
  deliberately covers only choice controls (`docs/contracts.md:75-79`).
  - **Breaks:** every `ProgressBar::new(signal)`, `Spinner::new(signal)` and
    `Field::status(signal)` call. Replacement: `ProgressBar::bound(signal)`,
    `Spinner::bound(signal)`, `Field::status_signal(signal)`;
    `InlineReadout::new(binding)` keeps compiling through `TextSource`'s
    conversions from `ReadSignal<String>` (`R/text_source.rs:191-235`).
  - **Retires:** the per-row role of `Signal::sibling` for display
    controls. Its rustdoc's per-row paragraph (`R/signal.rs:353-369`),
    guide/state.md's "Advanced: a per-row binding" row (`G/state.md:20`) and
    guide/layout.md's paragraph (`G/layout.md:532-540`) narrow to the
    editing controls without a controlled form; `sibling` keeps its first
    purpose, component-owned state such as a popup's open flag.
- **Alternative A: one constructor over a value-or-binding type**, a
  `TextSource` for any `T` (`impl Into<Source<T>>` with conversions from `T`
  and from bindings). One name per control, but a second vocabulary beside
  `new`/`bound`, and a generic `From<T>` beside `From<Signal<T>>` needs its
  coherence shown; per-type conversions, as `TextSource` has, avoid that at
  the cost of one source type per value type.
- **Alternative B: `controlled` constructors.** That vocabulary means a
  value plus a change callback (contracts.md "Controlled controls",
  `docs/contracts.md:675-683`); a display control has no change to report,
  so the word would mislead.
- **Alternative C: `ReadSignal::constant(value)`.** A `ReadSignal` holds the
  runtime's shared state (`R/signal.rs:169-176`, `:229-232`), which
  `combine`'s `ForeignRuntime` check and subscriptions rely on
  (`R/signal.rs:803-806`); a runtime-less constant changes that invariant,
  and a runtime-bound one (`ui.constant(v)`) still allocates a binding per
  row per description. Retires nothing.
- **Extension (not recommended now): controlled forms of the editing
  controls** (`RadioGroup`, `Select`, `Slider`, the number inputs,
  `TextField`; follow-ups "Controlled forms of more controls"). That would
  retire `sibling`'s per-row role entirely, but no round 2 run needed an
  editing control per row beyond the existing controlled `Checkbox`,
  `Toggle` and `InlineText`; each needs its refusal and timing specified.

### D2: R2-2, ending a refused draft from a form's Revert

Draft policy rule 3 says "writing the value the editor already shows changes
nothing" (`docs/contracts.md:1250-1253`), and an equal `Signal::set` notifies
nothing (`R/signal.rs:410-412`), so a Revert that writes the applied values
back leaves a refused draft shown. L4b's decision D3 already rejected
"write the bound value" for this reason and added `TextCommand::DiscardDraft`
(`plans/usability/layering-design-L4b.md:522-526`). Its delivery route,
`EventContext::request_text_command` (`R/runtime.rs:3265-3299`), queues one
request per editor, needs each editor's `WidgetId`, and delivers in the event
phase of the same flush, in request order: a target unmounted or hidden
before delivery is dropped, a modal-blocked target returns `ModalBlocked`
from the flush with later requests still queued, a target's error propagates
from the flush with later requests still queued, and ending a focused
in-place session returns focus to the owning control, or for a row-mode
`InlineText` to its nearest focusable ancestor, as Escape does. A
`FieldHandle` already records its field's control (`R/field_status.rs:312-319`),
which `FormFields::focus_first_error` uses (`:293-303`).

- **Recommended: a form-level discard over the existing route.**
  `FormFields::discard_drafts(&self, cx: &mut EventContext) ->
  Result<usize, RuntimeError>` queues one `DiscardDraft` request for each
  registered field whose `Field` has recorded a control, in registration
  order, and returns the number of **requests queued**; it says nothing about
  which controls had a draft. Its contract, stated in its rustdoc:
  - **Atomic admission:** every recorded control is checked for the
    caller's runtime before any request is queued, so a `ForeignRuntime`
    error queues nothing. No other error is possible at the call.
  - **Delivery** is `request_text_command`'s, unchanged: the event phase of
    this flush, in registration order; dropped for a target hidden or
    unmounted by then; `ModalBlocked` or a target's error returned from the
    flush, with the later requests still queued and delivered by the next
    flush (partial progress is possible and documented).
  - **Notifications:** each discarded draft reports `on_modified(false)`, and
    `on_rejection(None)` where a refusal was shown, after settling, as for a
    single `DiscardDraft`; a session's edit stays open.
  - **Focus** is inherited, not suppressed: a focused in-place session that
    is discarded returns focus as Escape does. The method requests no focus
    of its own.
  - It writes no application data, so the draft policy and L4b's matrix are
    unchanged. The editors' pages and contracts.md's rule 3 say that an
    equal write keeps a draft and point to the form discard; the
    settings-form recipe shows Revert as "write the applied values, then
    discard the drafts".
  - **Breaks:** nothing. **Retires:** the per-editor `request_text_command`
    loop with collected ids (1-codex's workaround) as the taught Revert;
    `DiscardDraft` stays the single-editor route.
- **Alternative A: a draft-policy change.** An application write ends a
  draft even when the value is equal. A signal records no equal write (no
  revision, no notification), so this needs a new write kind, such as a
  `Signal::reassert` that advances the revision without changing the value,
  and rule 3 restated around revisions. It breaks the equality rule every
  derived binding relies on ("equal results do not invalidate",
  `R/signal.rs:739-740`) or adds a second write method, and reopens L4b's
  matrix row "application write".
- **Alternative B: the recipe only,** with `request_text_command` per
  editor. No new API, but the friction stays: the application must collect
  every editor's id.

### D3: F3, what the `testing` glob and the prelude promise

The testing chapter says one `use` block covers a test (`G/testing.md:1-17`),
but `testing` (`F/lib.rs:266-283`) lacks `Semantics`, which
`Driver::semantics` returns (`R/testing.rs:377`), `FlushReport`, which
`settle` and `advance` return (`R/testing.rs:266`, `:294`), `SemanticRole`
and `InspectValue`; because it re-exports `SemanticValue`, rustc suggests
`SemanticValue::Slider` for a missing `SemanticRole` (3-claude, 4-codex).
`InspectValue` does not come from `Semantics`, which has no property map
(`R/diagnostics.rs:455-500`): a test meets it through an inspection query,
`QueryResult::components` (`R/inspection_service.rs:360-372`) to
`ComponentInspection` to `ComponentDetails::properties`
(`R/diagnostics.rs:748-756`). `ThemeMode` left the prelude in L2's decision
D2 (`plans/usability/layering-design-L2.md:952-962`) and two task 3 runs
needed it.

- **Recommended rule for `testing`:** the module's **seeds** are every item
  it exports for a test to call: `Driver`, `TestError`, `HeadlessApp`, the
  inspection-query names (`InspectionQuery`, `Selector`, `Detail`, and the
  `Runtime` methods that answer them), the capture names (`CaptureRequest`,
  `CaptureTarget`, `FrameImage`, `CpuRenderer`), and the input names it
  already holds. `testing` holds every type reachable from a seed's public
  signatures, transitively through the public fields and variants of the
  values they return (the **payload traversal**), except the names the
  prelude holds and an explicit, reviewed exclusion list of names a test
  does not write (for example `ValidationHints` and `Invalidation`, which a
  component inspection carries and which stay in `component` and
  `inspection`). A test then needs
  `prelude` and `testing` for anything a `testing` item hands it.
- **Recommended completeness test:** two checks.
  - The facade's `tests/foundation/modules.rs` `every_name` list gains the
    added names, as for every module today.
  - A listing-driven test in the unpublished `tests/diagnostics` crate reads
    the committed listings of every package a seed's signatures reach: the
    facade (`api/pert-ui/`), the runtime (`api/pert-ui-runtime/`), the core
    (`api/pert-ui-core/`) and the CPU renderer (`api/pert-ui-cpu/`). It
    resolves each type path to its defining item (a facade re-export to the
    item it re-exports), starts from the seeds, follows signatures and the
    payload traversal **across package boundaries**, and fails for any
    reachable pert-ui-owned type (`pert_ui`, `pert_ui_runtime`,
    `pert_ui_core`, `pert_ui_cpu`) that `pert_ui::testing` and
    `pert_ui::prelude` do not export and the exclusion list does not name.
    It checks each facade configuration whose `testing` differs: default
    (`api/pert-ui/default.txt`), `cpu` (`cpu.txt`, `CpuRenderer`),
    `no-default.txt` (no `HeadlessApp`) and `no-default-desktop-cpu.txt`.
    It runs in `cargo test --workspace`; the API listing gate keeps its
    input current. A hand list alone would not notice a new method's return
    type, which is how `Semantics` was missed.
  - **The checker's evidence outside the runtime:**
    `CpuRenderer::render` returns `pert_ui_cpu::CpuRenderError`
    (`crates/pert-ui-cpu/src/lib.rs:154`), which the facade exports only
    from `cpu` (`F/cpu.rs:40`) and which is in neither `testing` nor the
    prelude. A runtime-only checker cannot see it (Codex recheck). P2's
    test must report it before the fix. **Recommended:** export
    `CpuRenderError` from `testing` under `cpu`, beside `CpuRenderer`: a
    test that renders a frame itself, as 5-r1-claude-1 did, propagates or
    matches it. The alternative is the exclusion list, with the reason that
    a test only propagates it with `?` into `Box<dyn Error>`.
- **Recommended for `ThemeMode`: back in the prelude,** as the one
  exception to L2's D2. A theme switch is first-program application code
  (a settings menu, light and dark captures); the other services stay at the
  root.
- **Alternative A:** `ThemeMode` in `testing` only (both task 3 uses were
  captures). Smaller prelude, but an application's own theme switch still
  needs a root import.
- **Alternative B:** narrow the chapter's promise and name the extra imports
  instead of widening the glob. No new names, but the recurrence's cause
  (a promise the module does not keep) stays, and rustc's misleading
  suggestion stays.
- **Alternative C:** hand-added names without the listing test. Fixes
  today's F3 but gives no protection against the next recurrence.
- **Breaks:** nothing; the collision review checks the added glob names
  against consumer-local names, as L2's did.

### D4: F9 and R2-6, rows in display order

`Each`'s default container is structural and adds no address segment
(`R/described.rs:274-282`, `:355`); a container given with `within` is an
ordinary keyed element whose key is a segment of the rows' addresses
(`R/described.rs:300-330`). `Runtime::children` lists structural wrappers
(`R/runtime/tree.rs:136-149`, "ordered immediate children") without saying
so; a `VirtualList`'s children are its mounted rows in display order (group
1 probe); a `Table`'s are its structural `body` and its structural edit-end
reporter (`R/table.rs:2262-2277`), the body's children its mounted rows.
The guide's only ordered example uses `within` (`G/testing.md:203-260`), and
`QueryResult::components` is in incarnation order
(`R/inspection_service.rs:371`). Both task 4 runs had to walk around the
container.

Revision 1 proposed `Driver::rows` as "addressable children" that also
served `within`; those are two different things (Codex review). Revision 2
splits them.

- **Recommended: addressable children, plus a documented route for
  `within`.**
  - **`Driver::address_children(&parent)`** (name fixed at the checkpoint)
    returns, in child order, every descendant of `parent` that introduces
    exactly **one relative address segment** below it: the components
    `Runtime::find(&parent, &[segment])` would resolve, under `find`'s
    rules (`R/runtime/tree.rs:222-295`). It is defined relative to the
    supplied parent, not as "the parent's address plus one", because a
    parent may have no address of its own: a structural parent has none,
    and a part's name addresses only the part itself, its descendants
    addressing as if it were structural (`R/layout.rs:722-730`,
    `R/runtime/workspace.rs:249-256`). So:
    - the walk descends through structural levels (never into a drag
      preview); an element that contributes its key, or a slot its slot
      name, is returned and nothing below it is; a part is returned under
      its part name **and the walk continues through its descendants**,
      because a part's name addresses only the part and `find` resolves
      its addressable descendants without that segment
      (`R/runtime/tree.rs:319`): for `root → Part("panel") → Own("row")`,
      `address_children(&root)` returns both `panel` and `row`, as
      `find(&root, &["panel"])` and `find(&root, &["row"])` both succeed;
    - a structural `parent`, or a part, is a valid parent: the result is
      relative to it, exactly as `find` from it;
    - two returned components may introduce the same relative segment
      (structural levels can make one address reach several elements, as
      `find` documents); both are returned, in child order, and the
      rustdoc says that `find` refuses that address with `AmbiguousKey`;
    - a drag preview is never returned, as it never matches `find`.

    It is a generic tree operation built on public addressing
    (`Element::structural`, `slot`, `part`), so external collections need
    no hook and no target is unsupported: a leaf returns an empty list.
    Errors: `TestError::Runtime` with `ForeignRuntime` or `Unmounted`, as
    `Runtime::children`.
  - **What it gives per collection,** each a test in P5: an `Each` with its
    default container, its rows in list order; a `VirtualList`, **every
    currently mounted addressable row** in index order, which includes
    offscreen rows the list retains: its overscan
    (`R/virtual_list.rs:822-826`), a row pinned by focus or a pointer
    gesture (`:400-408`) and a row kept by `keep_mounted` (`:896-900`),
    which `Table` uses while a cell is edited; a `Table`, its mounted rows
    in source order, with the same retention (the body and the reporter
    are structural; the reporter has no children); an `Each` with `within`,
    the one container, so the documented route is
    `address_children(&driver.find(&each, &["tags"])?)`, which the testing
    chapter shows.
  - **Unmounted rows** are not returned. The documented route to one is the
    collection's controller (`ListController`, `TableController`), which
    scrolls to a key and mounts it; a `WidgetId`-based reveal cannot locate
    a row that has no component.
  - **Limits, documented:** a row the application marks
    `Element::structural` is looked through, so its addressable children
    appear instead of it; child order is description order, which for every
    built-in collection is display order, and for an external container is
    whatever order it describes.
  - `Runtime::children` and `Runtime::key` say that structural wrappers are
    listed and skipped by addresses, and that a virtualized collection lists
    only its mounted rows, offscreen retained ones included.
  - **Retires:** the walk through the `items` container and the
    bounds-sorted descendant walk as taught ways; `children` stays the
    structure walk for hosts and custom components.
- **Alternative A: a collection-row hook.** A `Component` method naming the
  row container (or the `within` container's key), so one `Driver::rows`
  serves every case including `within`. Every external collection must
  implement it to be supported, a second structural concept beside
  `Element::structural`.
- **Alternative B: `Runtime::children` looks through structural wrappers**,
  matching addresses. One method, no testing addition, but every caller
  that walks structure changes behavior, including
  `Runtime::find_descendant`, whose rustdoc ties it to `children`
  (`R/runtime/tree.rs:168-170`), the table's own example
  (`R/table.rs:1386-1389`) and custom hosts; a component would lose the way
  to reach its own structural parts.
- **Alternative C: documentation only.** The recipe without `within`
  (children of the `Each`'s first child) works today but teaches an
  internal key.
- **Sub-decision, the TODO's placeholder key:** `Each::new`'s `build`
  returns an `Element` whose key `Each` replaces (`R/described.rs:344-355`),
  so every caller writes a dead key (`.key("task")` in its own example).
  **Recommended:** `build` returns anything that converts into keyless
  content: any component, or an `Element` for rows of mixed types; `Each`
  keys it. The checkpoint names the conversion (a trait implemented for every
  `Component` and for `Element`, beside `IntoElement` at `R/layout.rs:410-416`).
  **Breaks:** callers that return a component through `.key(..)` keep
  compiling only if `Element` is accepted; the dead keys are removed from
  every example. **Alternative:** document the placeholder; keeps the dead
  key.

### D5: R2-11 and R2-24, focus and idle checks in tests

`clear_focus` is only `HostIntegration::clear_focus` (`R/host.rs:419-426`),
and `needs_paint` only `HostIntegration::needs_paint` (`R/host.rs:272-278`),
since L3's N3 took both off `Runtime`. `testing::*` re-exports
`HostIntegration` (`F/lib.rs:267-268`), so both are reachable, but the
testing chapter names neither, and `Driver` (`R/testing.rs:164-615`) has
neither. `Runtime::paint` clears the selected window's scene damage
(`R/runtime/compose.rs:336`) and returns `DrawError`
(`R/runtime/compose.rs:32`), which `TestError` (`R/testing.rs:710-780`) has
no variant for. `Runtime::next_deadline` reads the **selected window's**
pending work, timers and update deadlines together with work shared by all
windows (`R/runtime.rs:5574-5600`); the cross-window aggregate is
`HostIntegration::application_deadline` (`R/runtime/native_windows.rs:1385`).
`Driver::settle` flushes the selected window (`R/testing.rs:266-268`).

- **Recommended: `Driver::clear_focus()`** as the window's deactivation does,
  and **an idle check that measures a quiet interval**, not an observation
  that paints away its own evidence:
  `Driver::quiet_for(&mut self, interval: Duration) -> Result<Quiet,
  TestError>` (names fixed at the checkpoint). It settles, paints once to
  establish a painted baseline, then advances the clock by `interval` in
  `Driver::FRAME` steps with a flush each, and reports what happened in
  that interval: components updated, the earliest deadline still scheduled
  and whether the selected window needs painting at its end, with
  `Quiet::is_idle()` true when all three are empty. **Scope, aligned with
  `Driver::settle`:** the selected window's presentation and deadlines
  (`next_deadline`) plus work shared by all windows, with the existing
  allowance for notification followups in other windows that a flush
  delivers; another window's own timers do not make the report non-idle,
  and the rustdoc says so. `TestError` gains `Draw(DrawError)` for the
  paint. The handling of a final partial frame and a zero interval is
  settled at P4's checkpoint.
  - **Breaks:** `TestError` gains a variant; it is `#[non_exhaustive]`, so
    no match changes.
  - **Retires:** `HostIntegration` as the taught route in tests; it stays in
    `testing` for `select_window` and `set_clipboard`, which `Driver`'s
    errors name.
- **Alternative scope: every window,** through `application_deadline`.
  The report then covers the whole application, but another window's
  animation or timer keeps it non-idle while the selected window is quiet,
  and the paint check still covers only the selected window.
- **Alternative A: a side-effect-free observation,**
  `Driver::pending_work()` reporting `needs_paint`, `next_deadline` and
  dirty components now, with the test painting first itself. Cheaper, but
  the test must know to settle and paint before asking, the step the
  testers missed.
- **Alternative B: re-expose the methods on `Runtime`.** Undoes L3's N3
  layering for application code.
- **Alternative C: documentation only,** the recipe through
  `HostIntegration`. No API, but a test reaches into the host layer for an
  everyday check.

### D6: R2-20, what toast inspection reports

`ToastStack::inspect` reports the count, `scroll_offset`, `paused` and
`dismiss_label`, and semantics with role `Notifications`, the count as value
and `Dismiss` (`R/notifications.rs:808-835`). Toasts are painted by the
stack, not child components, so no message reaches inspection or content
search; both Claude task 5 runs on each commit kept their own message lists.

- **Recommended: one `SemanticItem` per visible toast,** in display order,
  named by its message's plain text, with `Activate` when it has an action
  and `Dismiss`; the focused toast `selected`; the action labels in a list
  property beside the items; [D8](#d8-r2-22-r2-23-and-r2-27-tables-in-tests)'s
  item bounds filled with the toast's rectangle. Items are how a component
  reports rows it draws itself, as the table's headers are
  (`R/table.rs:3030-3060`). The `ToastHost` page says what inspection
  reports, and states two limitations:
  - **Content search does not find toast messages:** it reads a component's
    semantic name and value, not its items (`R/runtime/content_search.rs:265-285`).
    Extending content search to items is not in this plan (TODO.md entry
    added in P9).
  - **The item's rectangle is the toast, not its controls:** the stack hit
    tests separate action and dismiss rectangles
    (`R/notifications.rs:670-690`), so clicking an item's center does not
    activate the action; a test activates or dismisses through the stack's
    keyboard focus. The checkpoint decides whether the action and dismiss
    controls are reported as items of their own with their rectangles.
- **Alternative: toasts as child components** (a label and buttons per
  toast). `Driver::click` and content search would work on them directly,
  but the stack's painting, focus and hit testing move into retained
  children, a redesign without a demonstrated need beyond reading the text.

### D7: R2-21, a growing child in a narrow `Row`

`ProgressBar` measures `min(available, 3 × ControlWidth)`
(`R/progress.rs:213-224`), 192 px at Standard. A bounded `Column` takes a
shortfall from its growing children by weight down to their minimums, but a
`Row` never shrinks a child (`R/layout.rs:598-611`;
`R/containers.rs:640-645` gives the reason: a narrower child could wrap to
another height, which the row's measurement does not repeat). A bar beside
its percentage in a narrow cell overflows with `EscapesParent`; five task 5
runs met it.

- **Recommended: rows shrink growing children as bounded columns do,**
  by weight down to each one's minimum, measuring a shrunk child again at
  its final width so a wrapping child gets its new height. Only rows whose
  natural widths do not fit change. **Residual overflow stays:** only
  growing children absorb the deficit, and none below its minimum, so a row
  whose non-growing children or minimums exceed its width still overflows,
  reported by validation as today; the rustdoc says so and P7 tests it.
  **Breaks (behavior):** such rows lay out differently, and a layout pass
  measures a shrunk child twice. `with_grow`'s "Rows never shrink a child"
  and guide/layout.md's matching sentence (`G/layout.md:453-460`) change.
- **Alternative A: a smaller natural width for `ProgressBar`** (one
  ControlWidth, filled by `with_grow`). Fixes the bar only, and changes
  every bar that does not grow.
- **Alternative B: documentation only:** the rule on `ProgressBar` and a
  bar-plus-percent cell recipe with a bounded container, as 5-head-claude-2
  wrote.

### D8: R2-22, R2-23 and R2-27, tables in tests

A cell is wrapped in a component keyed by its column (`R/table.rs:531-536`),
below the structural `body` (`R/table.rs:2262-2277`), so its content is at
`<table>/<row>/<column>/<content>`, which no page states; `TableItems` says
only "the element's own key is free" (`R/table.rs:239`). Content keyed like
its column collides in `find_descendant`. `TestError::NotFound` names the
whole path (`R/testing.rs:716-719`). Header titles are `SemanticItem`s
without bounds (`R/table.rs:3030-3060`; `SemanticItem` at
`R/diagnostics.rs:359-384` has no geometry), so pointer sorting in a test
needs positions computed from the application's widths. `Component::inspect`
takes no context (`R/runtime.rs:2121`); the runtime adds window geometry
afterwards (`R/runtime/inspect.rs:182-189`, `border_box` and `clip` from the
node). `Table::new` takes a fixed row height (`R/table.rs:1513-1524`) and
clips taller content with no validation report.

- **Recommended:**
  - **Addresses (R2-22):** document the cell address on `Table`,
    `TableItems::cell` and in the testing chapter, with the
    `find_descendant` collision; `TestError::NotFound` names the longest
    prefix that resolved, so a test sees where a path stopped.
  - **Headers (R2-27), a geometry contract external components can
    implement:** a component reports an item's rectangle in its **own local
    coordinates** (a public `SemanticItem` builder, no new context), and the
    runtime transforms it to window coordinates when it builds the
    inspection, exactly as it does for `border_box`: through the view scale
    and ancestors' transforms, intersected with the component's and its
    ancestors' clip. Built-ins use only that builder, so a custom component
    can do what the table does. Edge cases, each specified and tested in
    P8: geometry unknown before the first layout (no bounds, never a guess);
    a stale rectangle after a change not yet laid out (the inspection of the
    last layout, as `border_box`); view scaling; ancestor clipping; a
    header scrolled horizontally partly out of view (clipped bounds) or
    fully out (no bounds); a hidden component or a window other than the
    selected one. `Driver::click_item(&id, index)` reveals the component,
    then clicks the center of the item's visible bounds, failing with a
    typed `TestError` for an index out of range, an item without bounds
    and a target in another window; it re-reads the items after the reveal,
    since revealing may scroll and change them. Inspection schema 15
    becomes 16 (`R/diagnostics.rs:1004`; `SemanticItem` denies unknown
    fields, `:361-365`).
  - **Row height (R2-23):** `Table::new`'s rustdoc says the row must hold
    its tallest control and shows the metric lookup, and development
    validation reports a cell whose content measured taller than the row
    (a validation code the checkpoint names), since the cell's clip
    suppresses `EscapesParent`.
- **Alternative for headers: addressable header components**
  (`<table>/<column>`), which collide with a row keyed like a column, or
  documentation of the keyboard route only (`set_active_column` plus
  Alt+Up/Down, `R/table.rs:1211-1233`), which leaves pointer sorting
  untestable without geometry arithmetic.
- **Alternative for item geometry: a public inspection context** passed to
  `Component::inspect`. Components could compute window geometry
  themselves, but every implementation changes signature and each must get
  the transform and clip right.
- **Alternative for row height: a theme-derived default,**
  `Table::new(controller)` with a `row_height` builder defaulting to
  `ControlHeight` plus padding. Fewer clipped tables by default, but breaks
  every `Table::new` call and still needs the report for an explicit height.

### D9: R2-1, state from the initializer

`App::new` takes `FnOnce(&mut Runtime) -> Result<(), AppError>`
(`F/app.rs:358-361`); `on_error`, `on_close` and `on_window_error` get only
the runtime and their argument (`F/app.rs:507-510`, `:579-588`, `:594-597`);
and `HeadlessApp` exposes the runtime, the main window, `admit_close` and
`shutdown` (`F/app/headless.rs:151-227`). Its own example and the guide's
close-policy section teach an `Rc<RefCell<Option<_>>>` slot
(`F/app/headless.rs:22-72`, `G/testing.md:287-337`); six runs built one.

- **Recommended: a typed application state.** `App<S = ()>`: the
  initializer returns `Result<S, AppError>`; `on_close`, `on_error` and
  `on_window_error` receive `&mut S` beside the runtime, so every
  application callback can reach what the initializer built (the
  checkpoint confirms `on_window_error`, which has no demonstrated need but
  would otherwise be the one handler still needing a slot); `HeadlessApp<S>`
  gets `state()` and `state_mut()`. An initializer that returns `Ok(())`
  keeps `S = ()`, so applications without state change only their handler
  signatures. The checkpoint decides whether the native runner stays
  non-generic (state type-erased inside `App`). A handler never runs
  without state: initialization failures already bypass `on_error`
  (`F/app.rs:483-485`).
  - **Breaks:** every `on_close`, `on_error` and `on_window_error` closure
    gains the state argument. Replacement: `.on_close(|ui, state, window|
    ...)`, `.on_error(|ui, state, error| ...)`, `.on_window_error(|ui,
    state, error| ...)`.
  - **Retires:** the slot pattern in the guide and rustdoc, and the
    follow-up "Typed state handoff from `App::new`".
- **Alternative A: an accessor for a window's mounted roots**
  (`HeadlessApp::roots`). Smaller, but gives only `WidgetId`s, not the
  signals and handles a close policy reads, so the slot stays.
- **Alternative B: documentation only.**

### D10: R2-3, derivations over several bindings

`ReadSignal` derives through `map` (one input, `R/signal.rs:758`) and
`combine` (two, `:798`); dirty state over five settings fields took four
chained `combine` calls building tuples in both task 1 runs. `map` projects
a struct signal read-only; no writable field projection exists.

- **Recommended: `combine` over a tuple of bindings,** replacing today's
  method rather than adding beside it. `ReadSignal::combine` becomes an
  associated function taking a tuple of two up to a fixed number of
  bindings (the number is a documented limit the checkpoint fixes) and a
  function of their values. It keeps `map`'s and today's `combine`'s
  semantics (`R/signal.rs:734-760`, `docs/contracts.md:605-620`): at most one
  evaluation per batch of input writes, equal results invalidate nothing,
  every input retained for the result's lifetime and released when the
  last result drops, `ForeignRuntime` for inputs of different runtimes.
  `map` stays.
  - **Breaks:** every `a.combine(&b, f)`. Replacement:
    `ReadSignal::combine((&a, &b), |(a, b)| ...)`.
  - **Retires:** chained `combine` building tuples.
- **Writable field projection stays a follow-up** ("Writable model
  projection", plans/follow-ups.md), updated with round 2's evidence: it
  needs validation, revisions and conflict rules, and it does not by itself
  remove the n-ary dirty check.
- **Alternative A: an auto-tracking derivation** (`Runtime::computed(|r|
  ...)` reading any binding). No input list, but dependencies change per
  run, which the evaluation contract and its batching do not cover.
- **Alternative B: a dirty-tracking helper in `FormFields`.** A helper that
  observes application-owned comparisons (each field registering a
  "differs from applied" binding the application derives) could count
  dirty fields as `FormSummary` counts errors, without holding the
  application's values. It is narrower than a general derivation: it
  serves forms only, and the application still derives each comparison.
- **Alternative C:** the tuple form and the projection, the latter with its
  own design checkpoint in this plan.

### D11: R2-4, `TaskReceipt` and `#[must_use]`

`TaskReceipt` is `#[must_use]` (`R/runtime/tasks.rs:30-33`), so every
`cx.defer(...)?` and `cx.spawn(...)?` in a callback warns, and the guide
binds `let _receipt`. Failures surface from the flush; `Cancelled` is the
receipt's alone. Since L4a, `EventContext::defer`, `EventContext::spawn` and
`Runtime::defer` are cancelled before delivery only by shutdown, and by a
panic as it unwinds (`R/runtime/tasks.rs:12-16`, `:123-124`, `:378-384`,
`:394`): events the application already knows. `defer_while_shown` is also
cancelled by hiding or removal (`:294`), which nothing else reports. Each
method returns `Result<TaskReceipt, RuntimeError>`, and callers write `?`.

Revision 1 moved `#[must_use]` to `defer_while_shown`; that attribute would
apply to the `Result`, which `?` consumes, so `cx.defer_while_shown(..)?;`
would discard the receipt without a warning (Codex review).

- **Recommended: a distinct receipt type for the cancellation nothing else
  reports.** `defer_while_shown` returns a `#[must_use]` receipt type of
  its own (name at implementation, for example `ShownTaskReceipt`) with the
  same `status` and `status_binding` over `TaskStatus`, so the warning
  survives `?`; `TaskReceipt`, returned by `defer`, `spawn` and
  `Runtime::defer`, loses the attribute. `let _receipt` leaves the
  examples and guide. G2-8's rule (`plans/usability/api-audit/decisions.md:699-700`)
  holds unchanged: the must-use type is the one whose drop hides an
  otherwise unreported cancellation. Why a second type is not a second way:
  it marks a different cancellation contract, which the type then shows.
  **Breaks:** code that names `TaskReceipt` for a `defer_while_shown`
  result (the method's own example, `R/runtime/tasks.rs:259-272`).
  Replacement: the new type.
- **Alternative A: keep `#[must_use]` on `TaskReceipt`,** and say in the
  examples why the receipt is dropped. No change; the warning stays on
  every callback.
- **Alternative B: no `#[must_use]` anywhere.** Approves losing the warning
  for `defer_while_shown`'s hide or remove cancellation.
- **Alternative C: `defer` returns `()` and a second method returns the
  receipt.** Two ways for one thing; rejected by the anti-regrowth rule.

### D12: R2-25, a toast that stays

Every toast lives five seconds of unpaused time (`R/notifications.rs:129-133`,
`:357`); `Toast` has `new` and `action` only. `Banner` is the persistent
surface (`R/notifications.rs:14-18`), but the `Toast` and `ToastHost` pages
do not point to it.

- **Recommended: the pointer only.** `Toast`, `ToastController::push` and
  `ToastHost` point to `Banner` for a notification that stays. No run
  needed a different finite lifetime.
- **Alternative A: `Toast::lifetime(Duration)`** within a documented range.
  New capability, not a second way; reconsider when a consumer needs it.
- **Alternative B: a persistent toast.** A second way to do what `Banner`
  does; rejected by the anti-regrowth rule.

### D13: R2-26, the fraction a failed or cancelled task reached

`ProgressStatus::Failed` and `Cancelled` are unit variants drawn as a full
track (`R/progress.rs:33-38`, the fill at `:190-192`).

- **Recommended: terminal variants carry the reached fraction.**
  `Failed(Option<NormalizedValue>)` and `Cancelled(Option<NormalizedValue>)`;
  `None` keeps today's full track, `Some` fills to the fraction in the
  Error or Muted role. The enum is `#[non_exhaustive]` and `Copy`
  (`R/progress.rs:20-21`). Lands with F16 in P6, which changes the same file.
  **Breaks:** every construction and match of the two variants.
  Replacement: `ProgressStatus::Failed(None)`, or `Failed(Some(reached))`.
- **Alternative A: a bar builder that keeps the last determinate fill.**
  Hidden state in the widget: a remount or redescription forgets it, and
  the status no longer says what is shown.
- **Alternative B: no change;** the behavior is documented.

### D14: R2-5, the number inputs' semantic value

`IntegerInput` and `DecimalInput` report the saved number as
`SemanticValue::Number` with the range, also while a draft is pending, and
the draft's text only when nothing is saved
(`R/text_fields/number_inputs.rs:2116-2126`); `TextField` reports its
current text, the draft. Their property keys (`unit`, `display_value`,
`shows_display`, `text`; `R/text_fields/inline.rs:794-799`,
`R/text_fields/number_inputs.rs:2082`) are not listed.

- **Recommended: document it.** `Semantics::value` and each editor's page
  say which value is reported while a draft is pending, and list their
  keys, as `Field` does (`R/field_status.rs:455-461`). The saved number is
  what the field means; the draft is in `text`.
- **Alternative: report the shown draft,** as `TextField` does. Uniform
  with text editors, but a test or tool reading a pending draft no longer
  sees the number in effect, and `SemanticValue::Number`'s range then
  describes text.

### D15: N8

- **Recommended: not in this plan.** Round 2 shows no friction from the
  `Inline*` controls' parallel API; P6 changes only `InlineReadout`'s
  constructor, consistently with the standalone display controls.
- **Alternative:** a final part with its own design checkpoint, after P12.

## Parts

One version each, from 0.3.76. A part closes its IDs; the
[coverage table](#coverage) lists each once.

**Two checks recur below:**
- **Fixture check.** `plans/usability/task4-app` declares its own workspace
  and a registry dependency (`plans/usability/task4-app/Cargo.toml:8-11`),
  so the workspace gate never compiles it. A part that changes an API the
  fixture uses prepares a kit from the part's commit (`plans/usability/run.sh
  prepare <kit-dir>`) and runs `cargo test` in a copy of the fixture with
  `run.sh exec`, and the fixture's dependency requirement follows the new
  version.
- **Native evidence.** Where a part changes behavior a user sees in a native
  window, it names the Linux check or hand check that shows it, run on an
  unlocked desktop with a hardware GPU and recorded with the platform; where
  none is run, the Progress entry and `docs/verification.md`'s native
  evidence record it as missing. Windows and macOS stay missing throughout
  (NATIVE-01 to NATIVE-05).

### P1 Documentation (0.3.76)

Rustdoc and guide only; no behavior or API change. Text describes the code
of this commit: nothing here names an item a later part adds.

- **R2-5** (D14): `Semantics::value` (`R/diagnostics.rs:593-597`),
  `IntegerInput` and `DecimalInput` state the reported value while a draft
  is pending, the role `NumberInput`, and list their inspection keys.
- **R2-7:** `InlineText`'s editing child `"editor"`, its focus and the
  draft get a section of their own (`R/text_fields/inline_text.rs:17-29`);
  the testing chapter gets "In-place editors", reading the draft through
  the child.
- **R2-8:** `VirtualList::reorderable` and `on_activate`
  (`R/virtual_list.rs:1087`, `:1121`) say the list consumes Alt with an
  arrow and Enter before application commands and link the routing order;
  each `CommandScope` variant (`R/runtime/commands.rs:343-352`) says where
  it sits in that order.
- **R2-9:** `Checkbox` and `Toggle` point to `Field::hide_label`
  (`R/field_status.rs:524`) for a control named by text shown elsewhere.
- **R2-10:** `KeyChord::new` (`R/runtime/commands.rs:33-34`) gets an
  explicit Control and Alt example with `Modifiers`' field syntax.
- **R2-12:** the testing chapter's capture (`G/testing.md:352-383`) ends
  with `FrameImage::encode_png` written under `CARGO_TARGET_TMPDIR`.
- **R2-13:** `Label` and the chapter's sentence on names
  (`G/testing.md:25-26`) say an empty label has no name.
- **R2-14:** guide/custom-widgets.md's imports (`G/custom-widgets.md:15-20`)
  say a widget module globs `component`, adding `prelude` when it also
  builds elements, and its example needs exactly what it globs.
- **R2-15:** guide/custom-widgets.md gets one complete value-editing
  control: live writes or a commit, Escape in `keyboard_event`, the restore
  in `cancel_input`, what `on_change` reports, and the focus ring through
  `ColorRole::FocusRing`'s recipe. A failed restore is recorded and reported
  through a `Notifier`, as `Component::cancel_input` documents
  (`R/runtime.rs:2517-2547`), **and the recipe states that route's limit:**
  it reports only while the component stays mounted; a restore that fails
  during removal is not reported (as triage group 2 recorded for 3-codex).
  The recipe's doctest exercises a failed restore while mounted. A
  guarantee for removal would be a runtime contract change, not part of
  this plan.
- **R2-16:** every type whose serde impls are feature-gated (the
  `cfg_attr(feature = "serde", ...)` derives) says so in its rustdoc, and
  the crate root's feature table links them. A docs.rs-only `doc(cfg)` badge
  is not used: the kit renders on stable, where testers would not see it,
  and AGENTS.md keeps nightly features out except `api/generate.sh`.
- **R2-18:** guide/appearance.md's root globs (`G/appearance.md:169`,
  `:211`, `:238`, `:256`) become `prelude` and named imports. **Same kind,
  fixed here:** the root globs in `G/surfaces.md:32`, `:88`, `:184`, `:328`,
  `G/custom-widgets.md:123`, `:303` and `G/controls.md:281`, `:301`.
- **R2-19:** `SizeLimits::exact`'s example (`R/layout.rs:207-217`) uses
  `Size::from([260, 260])` and says it is the infallible form.
- **R2-28:** `Pending::wait` (`R/inspection_service.rs:584-590`) says it
  blocks forever on the UI thread unless the reply has already arrived;
  the UI-thread examples (`G/diagnostics.md:124`, `:170`, `:178`,
  `G/testing.md:376`, `F/cpu.rs:35`, `crates/pert-ui-cpu/src/lib.rs:260`)
  use `try_reply` after the flush that serves the request.
  `CaptureError::Unsupported`'s message change, which revision 1 put here,
  is implementation work and moves to P3.

- **Tests that fail when reverted:** the new doctests (R2-10, R2-12, R2-15
  with its failed restore, R2-19, R2-28's `try_reply` examples). The pure
  text changes have none; the review checks them against the code.
- **Docs:** as listed; `docs/manual.md` if it repeats a changed sentence.
- **Checkpoint:** none.
- **Gate:** the rustdoc/public-examples row: targeted doctests, the
  affected examples' compilation and headless sessions, warning-free
  rustdoc (default and `--features serde`), `git diff --check`.
- **`api/` diff:** none expected; `api/generate.sh` is run to show it.
- **Native evidence:** not applicable (no behavior change).
- **Breaking:** no.

### P2 Glob membership (0.3.77)

F3, by D3, and the TODO's `svg` collision.

- `pert_ui::testing` and `pert_ui_runtime::testing` gain the names the
  rule requires. At 26e6b5a the hand-checked list is `Semantics`,
  `SemanticRole`, `SemanticAction`, `SemanticItem`, `InspectValue`,
  `FlushReport`, `RevealRefusal`, the query payload types
  (`QueryResult`, `ComponentInspection`, `ComponentDetails`) and, under
  `cpu`, `CpuRenderError`; the listing-driven test settles the final list and the exclusion list, which
  the checkpoint reviews. `ThemeMode` joins both preludes; the prelude
  page's "where the other names are" row (`F/lib.rs:126`, `R/lib.rs:400`)
  drops it.
- D3's completeness tests, over the facade configurations it names, and
  `every_name` and `every_glob_combines` in
  `crates/pert-ui/tests/foundation/modules.rs` extended.
- **The `svg` collision (TODO.md):** the collision review of L2 is repeated
  for the changed globs, and the `svg` module is renamed or its `::svg`
  spelling documented, as the checkpoint chooses, with a scratch-crate
  check.
- The testing chapter's introduction (`G/testing.md:11-17`) states the rule.
- **Breaking:** only if the `svg` module is renamed (replacement: the new
  module path in every `use`).
- **Tests that fail when reverted:** the listing-driven test (it reaches
  `Semantics` from `Driver::semantics`, `FlushReport` from `settle`,
  `InspectValue` through `QueryResult` and, in the `cpu` configuration,
  `pert_ui_cpu::CpuRenderError` from `CpuRenderer::render`, the case
  outside the runtime), `every_name`'s new imports, a
  prelude-only program writing `ThemeMode`.
- **Checkpoint:** yes: the rule's wording, the seeds, the exclusion list,
  the added names and the collision review; Codex reviews, the maintainer
  approves. It changes L2's guarantee.
- **Gate:** the public-API row, the API listing gate, and the diagnostics
  snapshots (`cargo test --locked -p pert-ui-diagnostics -- --ignored`): a
  new re-export can change a suggested path; any `.stderr` change is
  reviewed like the `api/` diff. The `cpu`, `--no-default-features` and
  `desktop-cpu` configurations' Clippy and tests, since the `testing`
  module differs between them.
- **Native evidence:** not applicable.

### P3 Small ergonomics (0.3.78)

- **R2-4** (D11): `defer_while_shown` returns its own `#[must_use]`
  receipt type; `TaskReceipt` loses the attribute; `let _receipt` leaves
  the examples, the guide (`G/state.md`, `G/layout.md`) and the gallery.
- **R2-17:** `impl From<u16> for Spacing` as `Spacing::Fixed`, matching
  `Length` (`R/containers.rs:13`, `R/runtime/theme.rs:332-341`), so
  `.padding(24)` compiles; the integer literal stays unambiguous because
  `Spacing` has no other integer conversion.
- **Moved from P1:** `CaptureError::Unsupported`'s message
  (`R/capture.rs:169`) names the call that declares capture support and the
  CPU renderer's `serve_captures` route for tests (5-r1-claude-2#7's tester
  error showed the gap). Its `Display` text is pinned by a unit test.
- **Module closure:** the new receipt type joins wherever `TaskReceipt` is
  today, by the existing rules: the `component` module (G5-26's rule, every
  name implementing `Component` writes; `TaskReceipt` is there,
  `crates/pert-ui/tests/foundation/modules.rs`'s `component_names`) and the
  crate root; the prelude only if L2's closure over a prelude member's
  constructors and builders requires it (`EventContext` is a prelude
  member), which the part checks and records. `every_name` lists it.
- **Breaking:** `defer_while_shown`'s return type (replacement: the new
  receipt type, with the same `status` and `status_binding`).
- **Tests that fail when reverted:** a doctest with `.padding(24)`; in the
  facade's `foundation` binary, under `#![deny(unused_must_use)]`, a test
  that `cx.defer(...)?;` and `cx.spawn(...)?;` compile without binding; a
  `compile_fail` doctest that `cx.defer_while_shown(...)?;` with its
  receipt discarded is denied (the `?` form revision 1 missed); the
  `CaptureError` message test.
- **Docs:** `TaskReceipt`, the new receipt type, the three deferral
  methods, decisions.md's G2-8 record (an addendum).
- **Checkpoint:** none; D11 settles the shape.
- **Gate:** the public-API row and the API listing gate (the new type and
  the return type show in the listing; the committed listings record no
  `#[must_use]` attributes at all, so the attribute's placement is checked
  by the tests above, not by the listing); the package-local implementation
  checks for the message change.
- **Native evidence:** not applicable.

### P4 Driver: focus and idle (0.3.79)

R2-11 and R2-24, by D5.

- `Driver::clear_focus()`; the quiet-interval check with its report type;
  `TestError::Draw(DrawError)`. Rustdoc examples for each.
- The testing chapter gets "Nothing focused" and "Nothing scheduled while
  idle"; `HostIntegration::clear_focus` and `needs_paint` point to the
  `Driver` methods for tests.
- **Breaking:** `TestError` gains a variant (`#[non_exhaustive]`, no match
  changes).
- **Tests that fail when reverted:** a focused draft saved by
  `clear_focus`; over the interval, the report shows an animating spinner
  (updates and a deadline), a toast waiting to expire (a deadline), a
  binding written by a timer the test installed (updates and damage), and
  is idle once the last work ended; a paint failure arrives as
  `TestError::Draw`; **two windows:** with a spinner animating in a second
  window and nothing in the selected one, the report is idle, and with the
  spinner in the selected window it is not (the test that distinguishes
  D5's scope from the alternative); P2's listing-driven test covers the new
  signatures.
- **Checkpoint:** yes, short: the method's semantics (the baseline paint,
  the clock it advances, the selected-window scope with shared work, a
  final partial frame and a zero interval), its names and the report
  type.
- **Gate:** the public-API row and the API listing gate.
- **Native evidence:** not applicable (test-only facilities).

### P5 Rows in display order (0.3.80)

F9 and R2-6, by D4, and the TODO's `Each::new` placeholder key.

- `Driver::address_children(&parent)` (name at the checkpoint), defined by
  addresses, with the per-collection results and limits D4 lists.
- `Runtime::children` and `Runtime::key` (`R/runtime/tree.rs:127-149`)
  document structural wrappers and a virtualized collection's mounted rows;
  `Selector::children`'s incarnation order is pointed out where the
  chapter used it.
- `Each::new`'s `build` returns keyless content (D4's sub-decision); `Each`'s
  docs and examples, the guide's layout and testing chapters and the task 4
  fixture drop their dead keys.
- The testing chapter's "List rows" reads an `Each` without `within`, an
  `Each` with `within` through its container, a `VirtualList` (revealing a
  row first) and a `Table`.
- **Breaking:** `Each::new`'s `build` signature, if the checkpoint changes
  it. Replacement: return the component; `Element` still accepted for mixed
  rows. Consumers migrated: `Each` callers in the examples, gallery, guide,
  tests and `plans/usability/task4-app/src/lib.rs:104-117`.
- **Tests that fail when reverted:** `address_children` of
  - an `Each` (default container, in list order after a reorder), an
    `Each` with `within` (the container) and of that container (the rows);
  - a reordered `VirtualList`; a scrolled `VirtualList`, whose result holds
    the visible and overscan rows, a focused row scrolled out of view and
    a `keep_mounted` row, and not an unmounted row, which the test then
    mounts through `ListController` and finds;
  - a `Table` (rows, not the body or the reporter);
  - a **structural parent** (the result relative to it, as `find` from it);
  - a **part** and its descendants: enumerating the part's parent returns
    both the part (under its name) and its addressable descendant (under
    the descendant's own segment, not the part's name), and enumerating the
    part itself returns the descendant;
  - two children introducing the same relative segment through structural
    levels (both returned; `find` refuses with `AmbiguousKey`);
  - a slot (returned under its slot name); a drag preview (never
    returned);
  - a leaf (empty), a parent whose row is marked structural (its
    addressable children appear) and an unmounted parent (`Unmounted`);
  - and an `Each` whose `build` returns a bare component addresses the row
    by the item key.
- **Checkpoint:** yes: the operation's definition, its name, the keyless
  conversion and its coherence, the migration list.
- **Gate:** the public-API row and the API listing gate; the diagnostics
  snapshots if the conversion trait carries an
  `#[diagnostic::on_unimplemented]` message; the fixture check.
- **Native evidence:** not applicable (no presentation change).

### P6 Display controls take values (0.3.81)

F16 by D1, R2-26 by D13.

- `ProgressBar::new(ProgressStatus)` and `ProgressBar::bound(binding)`;
  `Spinner::new(bool)` and `Spinner::bound(binding)`;
  `InlineReadout::new(impl Into<TextSource>)`; `Field::status(FieldStatus)`
  and `Field::status_signal(binding)`, likewise `Property`; the rest of the
  checkpoint's list.
- `ProgressStatus::Failed(Option<NormalizedValue>)` and
  `Cancelled(Option<NormalizedValue>)`, drawn to the fraction; the
  inspection description says the fraction.
- contracts.md's "Ownership paths" (`docs/contracts.md:664-673`) states the
  `new`/`bound` rule for display controls, leaving "Choice sources" as it
  is; `TableItems::cell` (`R/table.rs:259-271`) and `ListItems::build` say
  that display controls take the record's value; `Signal::sibling`'s
  per-row paragraph and the two guide passages narrow (D1).
- **Breaking:** the constructors and the variants, each with its
  replacement in the CHANGELOG. Consumers migrated: `ProgressBar::new`,
  `Spinner::new`, `InlineReadout::new` and `Field::status` callers in the
  packaged examples (`inventory`, `log_explorer`), the gallery
  (`apps/gallery/src/pages/controls.rs`), `docs/manual.md`, the runtime
  README, the guide (`G/controls.md`) and the facade's tests; the
  `ProgressStatus::Failed`/`Cancelled` matches.
- **Tests that fail when reverted:** a table whose rows build
  `ProgressBar::new(record.status)` with no signal, republished, shows each
  row's status; `ProgressBar::bound` follows its binding as before (today's
  tests migrated); a `Failed(Some(half))` bar paints half the track in the
  Error role (a CPU capture or the display list); a `Field` with a fixed
  status names its error; a `Field` whose enablement is a `Permission`
  keeps its composition.
- **Docs:** each changed page; the guide's state table (`G/state.md:16-21`)
  and the editable-list recipe gain a progress column.
- **Checkpoint:** yes: the list of presentation inputs and their split, the
  contract wording, the `ProgressStatus` shape.
- **Gate:** the public-API row and the API listing gate.
- **Native evidence:** the gallery's progress page viewed natively on Linux
  with a failed bar at a partial fraction, recorded; otherwise missing.

### P7 Rows that shrink (0.3.82)

R2-21, by D7.

- `Row` takes a shortfall from its growing children by weight down to their
  minimums, measuring each shrunk child again at its final width; the
  column's shortfall code (`R/containers.rs:640-700`) is shared rather than
  copied.
- `Element::with_grow`, guide/layout.md and `ProgressBar` say so, including
  the residual overflow; guide/layout.md gets the bar-plus-percent cell
  (with P6's fixed bar).
- **Breaking:** behavior only: rows whose natural widths overflow give the
  difference from their growing children. Replacement usage for an
  application that relied on the overflow: a child that must keep its width
  sets a minimum through `SizeLimits`.
- **Tests that fail when reverted:** a 167 px cell with a growing bar and a
  label: no `EscapesParent`, the label inside; a row with a wrapping label
  shrunk below its natural width gets the taller height; weights and
  minimums split a shortfall as the column does; **residual overflow:** a
  row whose non-growing children alone exceed its width, and one whose
  growing children reach their minimums, still overflow and are still
  reported; a row that fits is unchanged.
- **Checkpoint:** yes: the algorithm, the second measurement's cost and
  where it may run, and the gallery pages whose layout changes (captures
  before and after).
- **Gate:** the public-API row (behavior change across packages), the
  packaged examples' headless sessions, the headless gallery, and the
  gallery's captures compared before and after.
- **Native evidence:** a Linux hand check of a narrow and a wrapping row
  (the gallery's layout page and a table with the progress cell, resized
  narrow) in the native window, recorded; otherwise missing.

### P8 Tables in tests (0.3.83)

R2-22, R2-23 and R2-27, by D8.

- Cell addresses documented on `Table`, `TableItems::cell` and in the
  testing chapter; `TestError::NotFound` gains the resolved prefix.
- `SemanticItem` gains its optional local rectangle, which the runtime
  transforms into window bounds in the inspection (D8's contract); the
  table's header items fill it; `Driver::click_item(&id, index)` with its
  typed errors.
- **Inspection schema 16 with explicit version validation.** Today
  `InspectionSnapshot` and `QueryResult` derive `Deserialize` and carry
  their version as a plain `u32` (`R/diagnostics.rs:946-958`,
  `R/inspection_service.rs:359-371`), so neither a constant change nor an
  optional field rejects an older document, least of all one without
  items. P8 makes both decoders validate the version: deserialization goes
  through a checked form (for example `serde(try_from = ...)`) that rejects
  any `schema_version` other than `SCHEMA_VERSION` with a typed error. This
  is decoder enforcement, beside the existing obligation on an external
  inspector to compare the version before decoding
  (`R/diagnostics.rs:1000-1003`), which stays documented.
- `Table::new`'s row-height guidance and the development-validation report
  for a cell taller than its row.
- **Breaking:** `TestError::NotFound`'s fields; the inspection schema
  version and its now-enforced check (a versioned format, so a version
  step, not a migration). Replacement: match `NotFound { path, .. }`;
  re-capture snapshots with the new runtime.
- **Tests that fail when reverted:**
  - a header click through `click_item` sorts the column, also with the
    table horizontally scrolled so the header is partly clipped (clicked
    inside the visible part) and at a view scale other than 1;
  - a header scrolled fully out of view has no bounds and `click_item`
    fails with its typed error, as do an index out of range and a target in
    another window; items read before the first layout have no bounds;
  - a pending layout (a change not yet flushed) and a failed layout keep
    the items' bounds of the last successful one; a hidden target's items
    have none; a reveal that scrolls changes the items, and `click_item`
    uses the re-read ones;
  - a table in a detached window and one in a promoted surface report
    bounds in their own window's coordinates;
  - a header whose visible center falls on a resize grip: `click_item`
    resizes rather than sorts, as documented (the sorting test chooses
    geometry that avoids the grip);
  - content keyed like its column found by its full address and
    `find_descendant`'s ambiguity named; a `NotFound` on `["r1", "missing"]`
    reports `r1` resolved;
  - a 32 px button in a 28 px row produces the new validation issue, and a
    fitting one none;
  - **both envelopes,** `InspectionSnapshot` and `QueryResult`, under
    `--features serde`: a round trip with items with and without bounds; an
    empty collection (no components, no items) at schema 16 accepted; the
    same empty document at schema 15 rejected; an unsupported future version
    rejected; an unknown field rejected; an item without the bounds field
    accepted as no bounds.
- **Checkpoint:** yes: the geometry contract and its edge cases, the
  version validation (its checked intermediate keeps closed-schema
  rejection: unknown fields still fail), which items fill bounds now, the
  validation code, and three qualifications from Codex's recheck:
  - **Presentation geometry:** the transform follows committed
    presentation geometry, not logical ancestors blindly: a promoted
    surface or a detached window resets scale and clipping
    (`R/runtime/tree.rs:1818-1824`), and an item of a component presented
    there gets that window's coordinates.
  - **Correlation with layout:** node geometry is staged until a whole
    layout succeeds (`R/runtime/tree.rs:1893-1896`); the checkpoint says
    how a component's reported local rectangles stay tied to that last
    successful layout (for example, recorded with it rather than read fresh
    at inspection), so a pending or failed layout never pairs new
    rectangles with old bounds.
  - **`click_item` is pointer input:** a press and release at the center
    of the item's visible rectangle, with no guarantee of the item's
    semantic action. A partly clipped header can put that center on a
    resize grip, which wins over sorting (`R/table.rs:2638-2646`); the
    rustdoc says so.
- **Gate:** the public-API row (persistence: the inspection schema), the API
  listing gate, `--features serde` Clippy, tests and rustdoc, and
  `-p pert-ui-core --features serde` if a core type is touched.
- **Native evidence:** a Linux hand check of pointer sorting on the
  gallery's table in the native window, recorded; otherwise missing.

### P9 Notifications in inspection (0.3.84)

R2-20 by D6, R2-25 by D12.

- `ToastStack::inspect` reports one item per visible toast, with its
  rectangle, and the action labels; `ToastHost`'s page says what inspection
  reports and states D6's two limitations (no content search, the item
  rectangle is not the action's).
- `Toast`, `ToastController::push` and `ToastHost` point to `Banner`.
- A TODO.md entry records that content search does not read semantic items.
- **Breaking:** no (the stack's semantics gain items).
- **Tests that fail when reverted:** two toasts, one with an action: the
  items' names, actions, order and bounds; a bound message change shows in
  the next inspection; an expired toast leaves the items; activating the
  action through the stack's focus and keys, as the page documents.
- **Checkpoint:** none beyond P8's, which fixes the item shape, unless D6's
  open question (separate items for the action and dismiss controls) is
  answered yes, in which case P9 gets a short one.
- **Gate:** the public-API row.
- **Native evidence:** not applicable (inspection only).

### P10 Derivations over several bindings (0.3.85)

R2-3, by D10.

- `ReadSignal::combine` over a tuple of bindings, up to the documented
  limit, replacing the two-input method, with D10's semantics.
- contracts.md's evaluation class names it; plans/follow-ups.md's
  "Writable model projection" records round 2's evidence.
- **Breaking:** `combine`'s form. Replacement:
  `ReadSignal::combine((&a, &b), |(a, b)| ...)`. Consumers migrated: every
  caller in the runtime, the examples (`inventory`, `diagram_editor`), the
  gallery, the guide (`G/layout.md`), the facade's tests and the task 4
  fixture (`plans/usability/task4-app/src/lib.rs:66`, `:72`).
- **Tests that fail when reverted:** a five-input derivation evaluates once
  per batch of writes and not on equal results; inputs are released when
  the last result drops; a binding of another runtime fails with
  `ForeignRuntime`; the tuple-arity limit is a compile error (a
  `compile_fail` doctest).
- **Checkpoint:** yes: the signature, the limit, the batching contract.
- **Gate:** the public-API row and the API listing gate; the diagnostics
  snapshots if the tuple trait gets an `on_unimplemented` message; the
  fixture check.
- **Native evidence:** not applicable.

### P11 Application state from the initializer (0.3.86)

R2-1 by D9, the follow-up "Typed state handoff from `App::new`", and the
TODO's L3 native-probe overlap noise.

- `App<S = ()>`, the initializer returning `S`, `on_close`, `on_error` and
  `on_window_error` receiving `&mut S`, `HeadlessApp<S>::state` and
  `state_mut`.
- The slot leaves `HeadlessApp`'s example (`F/app/headless.rs:22-72`), the
  guide's close-policy section (`G/testing.md:287-337`), `G/layout.md` and
  the diagram editor (`crates/pert-ui/examples/diagram_editor/main.rs`).
- **The TODO's overlap noise:** the pixel-readback probe's stacked roots
  and the `persistent-paint-error` probe report `OverlapPolicy::Allow` or
  mount under one stacking container, so this part's native run logs no
  validation errors from probes.
- **Breaking:** the handler signatures (D9); `App` becomes generic.
  Consumers migrated: the examples, `F/platform/multi_window_smoke.rs`,
  `F/platform/smoke.rs` (its `on_error` at `:1134`),
  `F/platform/schedule.rs`'s tests (`:477`, `:526`, `:544`), the facade's
  surface tests (`tests/surfaces/window_close.rs`, `windows.rs`), the
  guide, `docs/manual.md`, the runtime README if it shows `App`, and the
  task 4 fixture's `main.rs`.
- **Tests that fail when reverted:** a headless close policy reads state the
  initializer returned, with no slot; a test reads `HeadlessApp::state`;
  `F/platform/schedule.rs`'s scheduler tests drive `on_error` with state
  through the runner's error path without a window; the native suite's log
  has no probe `UnexpectedOverlap`.
- **`on_error` and `on_window_error` need native evidence:** `HeadlessApp`
  deliberately bypasses both (`F/app/headless.rs:105-111`). The native
  check that installs `on_error` (`F/platform/smoke.rs:1126-1134`,
  `window_resize_and_idle`) is changed to read and change its state in the
  handler and assert it afterwards, and runs in `pert-ui-native-checks`;
  `on_window_error` gets the same in the multi-window smoke if a check can
  provoke a presentation failure there, otherwise its native evidence is
  recorded as missing.
- **Checkpoint:** yes: the type, the handler signatures (including
  `on_window_error`), the runner's erasure, the migration list.
- **Gate:** the dependency/feature/platform row, since the native runner
  changes: the workspace gate, release-profile tests, the MSRV, every
  supported feature configuration (including `--no-default-features
  --features desktop-cpu`), the API listing gate, library packaging and the
  packaged-consumer check, the fixture check, and `cargo run --locked -p
  pert-ui-native-checks` on Linux with the platform recorded, plus a hand
  check of closing a window with unsaved state; Windows and macOS evidence
  recorded as missing.

### P12 Form drafts and the settings recipe (0.3.87)

R2-2 by D2, and the TODO's `diagram_editor` close policy.

- `FormFields::discard_drafts(cx) -> Result<usize, RuntimeError>` with
  D2's contract (atomic admission, requests queued, delivery, partial
  progress, notifications, inherited focus) in its rustdoc;
  contracts.md's rule 3 and the editors' pages say an equal write keeps a
  draft and point to it.
- guide/state.md gets the settings-form recipe: fields registered in a
  `FormFields`, dirty state through P10's `combine`, Apply through
  `EventContext::spawn`, Revert writing the applied values and discarding
  the drafts, and a close policy reading P11's state.
- **The TODO's close policy:** the diagram editor's inspector fields feed a
  `FormFields` through `on_modified`, and `close_requested` keeps the window
  open while `FormSummary::modified` is non-zero, tested through
  `HeadlessApp::admit_close`.
- **Breaking:** no.
- **Tests that fail when reverted:**
  - an `IntegerInput` with a refused draft and a `TextField` with a kept
    draft in one form: Revert's equal write keeps both drafts;
    `discard_drafts` returns the number queued, and after the flush both
    drafts are gone, each reported `on_modified(false)` after settling and
    the refusal cleared with `on_rejection(None)`;
  - a field whose control is not mounted is not counted; a field hidden
    between the call and the flush is dropped at delivery without error;
  - a focused `InlineText` with an open session: the session ends and focus
    returns to the owning control; in row mode, to the list;
  - modality: a target blocked by an open modal dialog makes the flush
    return `ModalBlocked`, the requests behind it stay queued and are
    delivered by the next flush;
  - a control recorded from another runtime makes the call fail with
    `ForeignRuntime` and queue nothing;
  - a target whose `text_command` fails: the flush returns its error, the
    later requests stay queued;
  - the diagram editor's close is refused with a kept inspector draft.
- **Checkpoint:** yes, short: the method's contract and the contract text;
  L4b's matrix is unchanged, which the checkpoint confirms.
- **Gate:** the public-API row and the API listing gate; the
  `diagram_editor` headless session.
- **Native evidence:** a Linux hand check of Revert discarding a refused
  draft in the settings recipe's or the diagram editor's native window,
  recorded; otherwise missing.

## Rules for every part

- **One review unit per part.** A fresh implementing agent makes the change,
  bumps the workspace patch version (the workspace version, the internal
  version requirements and `Cargo.lock`), writes the CHANGELOG entry and
  runs the part's gate from a clean state. The author reviews the diff,
  records the part under [Progress](#progress) and commits it, staging only
  the part's paths. An independent Codex review (`codex exec -s read-only`,
  stdin from a file or `/dev/null`) follows before the part is called done;
  its fixes are separate commits with a recheck.
- **Branch.** The parts land on a branch from `developer-usability`'s head
  that the maintainer names. No merge or push without the maintainer's
  instruction.
- **Design checkpoints.** A part marked with a checkpoint starts with a
  design document beside this plan (`round-2-design-P<n>.md`), reviewed by
  Codex and approved by the maintainer before code, as 6f1 to 6f3 and the
  layering parts had. It includes a migration manifest: the maintained
  consumers (the packaged examples and their sessions, the gallery, the
  native checks, the guide, `docs/manual.md`, the READMEs, the facade's and
  runtime's tests and doctests, the diagnostics fixtures and
  `plans/usability/task4-app`) and the frozen ones that are not migrated
  (the submissions and probes under `plans/usability/runs/` and
  `~/pert-ui-usability/`).
- **CHANGELOG.** Every entry names each item the part's `api/` diff adds,
  removes or changes, and its finding IDs. A breaking change is a
  `Breaking:` entry with its replacement usage. Churn from the toolchain or
  a dependency is described as such.
- **Text describes the code of its own commit.** No rustdoc, guide or
  contract sentence names an item a later part adds.
- **Consumers migrate in the same part,** per the manifest; the task 4
  fixture keeps building against the packages.
- **Tests.** Each behavior change has a test that fails with the change
  reverted; new facade tests join the area binary they cover
  (`crates/pert-ui/tests/<area>/main.rs`). Each new public item has rustdoc
  with a runnable example.
- **Persisted formats.** Only P8 changes a versioned schema (inspection 15
  to 16). `Workspace::SCHEMA_VERSION` stays 2.
- **Same-kind defects in touched code** are fixed in the part and named in
  its Progress entry (P1's root globs are the first).
- **Out-of-scope findings** go to TODO.md in the AGENTS.md form and are
  reported, not fixed. A TODO entry this plan pulls in is removed in the
  part that resolves it.
- **Prelude closure.** A part that adds API to a prelude member re-applies
  L2's closure rule (`plans/usability/layering-design-L2.md`, D1) and P2's
  `testing` rule; the completeness tests enforce the second.

## Order and constraints

| Part | Content | IDs | Version | Breaking | Checkpoint |
|---|---|---|---|---|---|
| P1 | Documentation | R2-5, R2-7 to R2-10, R2-12 to R2-16, R2-18, R2-19, R2-28 | 0.3.76 | no | no |
| P2 | Glob membership | F3; TODO `svg` | 0.3.77 | only a module rename | yes |
| P3 | Small ergonomics | R2-4, R2-17 | 0.3.78 | `defer_while_shown`'s return type | no |
| P4 | Driver: focus and idle | R2-11, R2-24 | 0.3.79 | a `TestError` variant | short |
| P5 | Rows in display order | F9, R2-6; TODO `Each::new` | 0.3.80 | `Each::new` | yes |
| P6 | Display controls take values | F16, R2-26 | 0.3.81 | yes | yes |
| P7 | Rows that shrink | R2-21 | 0.3.82 | behavior | yes |
| P8 | Tables in tests | R2-22, R2-23, R2-27 | 0.3.83 | yes; schema 16 | yes |
| P9 | Notifications in inspection | R2-20, R2-25 | 0.3.84 | no | no |
| P10 | Derivations over several bindings | R2-3 | 0.3.85 | yes | yes |
| P11 | Application state from the initializer | R2-1; follow-up; TODO overlap noise | 0.3.86 | yes | yes |
| P12 | Form drafts and the settings recipe | R2-2; TODO `diagram_editor` | 0.3.87 | no | yes |

Constraints:

- **P1 first:** its pages are those later parts extend; it names no later
  item.
- **P2 before P4, P5 and P8:** they add `Driver` methods and `TestError`
  fields, and P2's listing-driven test then checks their signatures.
- **P5 before P8:** P8's testing-chapter table section reads rows with P5's
  operation.
- **P6 before P7:** P7's bar-plus-percent recipe uses the fixed bar.
- **P7 before P8:** the table chapter shows the progress cell laid out.
- **P8 before P9:** P8 fixes the item shape and the schema step; P9 fills
  it for toasts without a second step.
- **P10 and P11 before P12:** the settings recipe uses both.
- **A decision that changes a part's shape** is recorded in that part's
  checkpoint; a part whose decision is "documentation only" moves its
  IDs into a documentation part with its own version, and the table here
  is updated.
- **Before P1:** task 6 sealed ([round 3](#round-3-proposal)), so no fix
  exists when it is written.
- **Before round 3:** every part closed and reviewed; the kit issues fixed
  (below); the counting definitions in `summarize.py` unchanged.

## Round 3 (proposal)

A proposal, not a decision: round 3 is decided by the maintainer when this
plan closes.

- **A sealed task 6 by an author who does not know the findings.** Revision
  1 had this plan's author write task 6 aimed at round 2's weak areas; such
  a task is shaped by the fixes and is a targeted check, not a holdout like
  task 5, which was written at slice 0 before round 1 (Codex review).
  Instead:
  - **Author:** a fresh agent session, or the maintainer, that receives
    only the library's public description (the facade's README and the
    crate root page), the format of tasks 1 to 5 (their files, without the
    acceptance notes of any run), the tester brief
    (`plans/usability/tester-brief.md`) and the acceptance procedure's
    format. It receives none of round 1's or round 2's findings, the
    triage, the decisions, the API audit, this plan or its design
    checkpoints. The prompt it was given is kept with the task.
  - **Content:** an application of the size of tasks 1 to 5, with a
    headless verification requirement as they have, and its acceptance
    checks, written in the same session.
  - **Seal:** the task and its acceptance checks are committed to
    `plans/usability/reserved/` before P1 lands, and that commit's hash is
    recorded in the runs' README; neither changes afterwards. Tester
    blindness stays as in rounds 1 and 2: the kit's sandbox never contains
    `reserved/` or this plan.
  - A targeted task aimed at round 2's areas may be added as well, labelled
    a targeted transfer task and reported apart from the holdout.
- **Runs, in two separate comparisons:**
  - **Paired, task 6:** two runs per tester on each of the pre-fix commit
    (26e6b5a, 0.3.75) and the head. This is the comparison of the
    libraries.
  - **Head-only reruns, tasks 1 to 5:** one run per tester at the head,
    compared with rounds 1 and 2 as round 2 compared with round 1, and
    reported apart from the paired comparison.
- **Fairness conditions for the paired comparison:**
  - the same harness on both commits: both kits prepared with the head's
    `run.sh`, `bridge.py`, `proxy.py`, `cargo-wrapper.py`,
    `friction-log.md`, `check-probe.py` and `claude-settings.json`, as round
    2 overlaid task 5's baseline kit, with the overlay recorded;
  - the same reading aids in both kits: the text rendering of the
    documentation (the kit fix below) is built for both commits;
  - acceptance frozen before the first run and applied by the same
    procedure to both commits;
  - interleaved order: each tester's queue alternates the two commits, as
    round 2 mixed task 5's (`runs/README.md:133-138`);
  - fresh, isolated sessions per run; matched models and effort settings
    for both commits; the CLI versions recorded.
- **Records as in round 2:** recurrence of F1 to F34 and of R2-1 to R2-28
  per run under round 2's matrix rules; new findings triaged by fresh
  verifying agents and a Codex review; the counting definitions in
  `summarize.py` unchanged.
- **Kit fixes first** ([kit issues found in round 2](runs/README.md#kit-issues-found-in-round-2)):
  a text rendering of the guide and item pages, or a brief line on reading
  the HTML; task 5's notification criterion written into
  `plans/usability/acceptance.md` as the maintainer decided on 2026-10-27;
  no change for the tester tooling.

## Coverage

| ID | Part | Action |
|---|---|---|
| F3 | P2 | `testing` rule, added names, `ThemeMode` in the prelude, listing-driven completeness test (D3) |
| F9 | P5 | `Driver::address_children`; "List rows" without `within`, and the `within` route (D4) |
| F16 | P6 | fixed `new` and reactive `bound` for display controls; `sibling`'s per-row role narrowed (D1) |
| R2-1 | P11 | `App<S>`, state to `on_close`, `on_error` and `HeadlessApp` (D9) |
| R2-2 | P12 | `FormFields::discard_drafts`; rule 3 text; Revert recipe (D2) |
| R2-3 | P10 | `combine` over a tuple of bindings; projection stays a follow-up (D10) |
| R2-4 | P3 | a `#[must_use]` receipt type for `defer_while_shown` only (D11) |
| R2-5 | P1 | number inputs' semantic value and keys documented (D14) |
| R2-6 | P5 | `Runtime::children` and `key` document structural wrappers and mounted rows |
| R2-7 | P1 | `InlineText` editor child section; chapter section |
| R2-8 | P1 | consumed keys on `VirtualList`; `CommandScope` variants |
| R2-9 | P1 | `Checkbox`/`Toggle` point to `Field::hide_label` |
| R2-10 | P1 | `KeyChord::new` example |
| R2-11 | P4 | `Driver::clear_focus` (D5) |
| R2-12 | P1 | capture example ends with `encode_png` |
| R2-13 | P1 | empty label has no name |
| R2-14 | P1 | widget import rule in custom-widgets.md |
| R2-15 | P1 | value-editing control recipe |
| R2-16 | P1 | serde feature stated on gated types |
| R2-17 | P3 | `From<u16> for Spacing` |
| R2-18 | P1 | appearance.md globs, and the same kind in three other chapters |
| R2-19 | P1 | `SizeLimits::exact` example |
| R2-20 | P9 | toast items in inspection; limitations stated; content-search gap to TODO.md (D6) |
| R2-21 | P7 | rows shrink growing children (D7) |
| R2-22 | P8 | cell address documented; `NotFound` resolved prefix (D8) |
| R2-23 | P8 | row-height guidance; validation report for clipped cells (D8) |
| R2-24 | P4 | quiet-interval check; `TestError::Draw`; chapter section (D5) |
| R2-25 | P9 | pointer from `Toast` to `Banner` (D12) |
| R2-26 | P6 | terminal statuses keep the reached fraction (D13) |
| R2-27 | P8 | item geometry contract; `Driver::click_item`; schema 16 with version validation (D8) |
| R2-28 | P1 | `Pending::wait` rule narrowed; UI-thread examples use `try_reply` |
| TODO `Each::new` placeholder key | P5 | keyless `build` (D4) |
| TODO `pub mod svg` collision | P2 | rename or documented spelling |
| TODO L3 probe overlap noise | P11 | probes allow their overlap; clean native log |
| TODO `diagram_editor` `close_requested` | P12 | inspector drafts keep the window open |
| Follow-up: typed state handoff | P11 | implemented; removed from plans/follow-ups.md |
| Follow-up: writable model projection | P10 | evidence recorded; stays a follow-up (D10) |
| Follow-up: controlled forms of more controls | P6 | unchanged unless D1's extension is chosen |
| N8 | — | not in this plan (D15) |
| Kit issue: reading the rendered HTML | round 3 | kit fix before round 3 |
| Kit issue: task 5's notification criterion | round 3 | written into acceptance.md |
| Kit issue: Codex tool refusal | — | no change |
| Tester errors (results, "Listed separately") | — | no library change; 5-r1-claude-2#7's `CaptureError::Unsupported` message improved in P3 |

## Review record

- **Revision 1, Codex review (2026-10-27):** needs revision, with four
  blocking points and six should-fix points plus nits. Its record is
  `~/pert-ui-usability/codex-prompts/codex-r2-fixplan.out`. It found
  coverage complete by ID, the version sequence coherent and D1, D6, D7,
  D9, D10 and D12 to D15 grounded. Each point was checked against the code
  at 26e6b5a before revision 2 applied it; none was rejected.
  - **D11 (blocking):** a method-level `#[must_use]` applies to the
    `Result`, which `?` consumes (`R/runtime/tasks.rs:294`). Revision 2
    gives `defer_while_shown` a must-use receipt type of its own and tests
    the `?` form; the claim that the attribute shows in the listing was
    wrong (the committed listings carry no `#[must_use]`, for example
    `api/pert-ui-runtime/default.txt:38249`) and is replaced by tests.
  - **D4 (blocking):** "addressable children" cannot also yield `within`
    rows, whose container adds a segment (`R/described.rs:300-330`).
    Revision 2 defines one generic operation by addresses, built on
    `Element::structural` so external components need no hook, gives
    `within` a documented route, and lists results, limits and errors per
    collection; a row hook is the alternative.
  - **D2 (blocking):** `request_text_command` queues, drops hidden or
    unmounted targets, returns `ModalBlocked` or a target's error from the
    flush with later requests queued, and returns focus when it ends a
    focused in-place session (`R/runtime.rs:3265-3299`). Revision 2's
    `discard_drafts` is fallible with atomic admission, reports requests
    queued, inherits delivery, notifications and focus, and P12 tests
    focused `InlineText`, modality, a foreign runtime and a failing target.
  - **Task 6 (blocking):** a task written by this plan's author for round
    2's weak areas is not a holdout. Revision 2 has a fresh author without
    the findings write it, seals it before P1, states the fairness
    conditions and keeps the paired comparison apart from the head-only
    reruns.
  - **D5:** painting clears the scene damage the check meant to observe
    (`R/runtime/compose.rs:336`), and `DrawError` has no `TestError`
    variant (`R/testing.rs:710-780`). Revision 2 measures a quiet interval
    after a baseline paint, states its scope, adds `TestError::Draw` and
    gives P4 a short checkpoint.
  - **D3:** `InspectValue` comes from query results, not `Semantics`
    (`R/diagnostics.rs:455-500`, `:748-756`). Revision 2 seeds the test
    from the whole `testing` surface, traverses payloads, keeps a reviewed
    exclusion list and checks each facade configuration whose `testing`
    differs.
  - **D8:** `Component::inspect` has no context and the runtime adds
    geometry (`R/runtime.rs:2121`, `R/runtime/inspect.rs:182-189`).
    Revision 2 has components report local item rectangles that the runtime
    transforms and clips, as for `border_box`, and lists the edge cases P8
    tests.
  - **P8:** neither envelope validates its version on decode
    (`R/diagnostics.rs:946-958`, `R/inspection_service.rs:359-371`).
    Revision 2 adds decoder validation and tests both envelopes, empty ones
    included, beside the inspector's existing obligation.
  - **P1:** the custom-control recipe states that a failed restore is
    reported only while mounted (`R/runtime.rs:2517-2547`) and tests a
    failed restore while mounted; the `CaptureError` message change moves
    to P3, which has the implementation gate.
  - **Verification rows:** the fixture check (the fixture is its own
    workspace, `plans/usability/task4-app/Cargo.toml:8-11`) joins P5, P10
    and P11; P11's `on_error` gets native evidence through the native check
    that installs it (`F/platform/smoke.rs:1126-1134`), since headless
    bypasses it (`F/app/headless.rs:105-111`); P6, P7, P8 and P12 name a
    Linux hand check or record its evidence as missing.
  - **Nits:** D1's rule goes under "Ownership paths" and leaves Permission
    inputs separate; D6 states that content search does not read items
    (`R/runtime/content_search.rs:265-285`) and that the toast rectangle
    is not its action's (`R/notifications.rs:670-690`); D7 and P7 state and
    test residual overflow; D9 decides `on_window_error`
    (`F/app.rs:594-597`); D10's alternative B is reworded as a helper over
    application-owned comparisons.

- **Revision 2, Codex recheck (2026-10-27):** needs revision; record
  `~/pert-ui-usability/codex-prompts/codex-r2-fixplan-r2.out`. It found the
  D11, D2, D5 (idle semantics), D8 (architecture), P8, P1, verification and
  task 6 points resolved, and three points open. Each was checked against
  the code before revision 3 applied it; none was rejected.
  - **D3:** the checker followed only the runtime listing, so it could not
    see `CpuRenderer::render`'s `pert_ui_cpu::CpuRenderError`
    (`crates/pert-ui-cpu/src/lib.rs:154`, exported only from `cpu`,
    `F/cpu.rs:40`). Revision 3 traverses the facade, runtime, core and CPU
    listings by resolved item, checks every reachable pert-ui-owned type,
    names this case as the checker's evidence, and recommends exporting
    `CpuRenderError` from `testing` under `cpu`.
  - **D4:** "the parent's address plus one segment" fails for structural
    parents and parts, whose descendants do not inherit the part's name
    (`R/layout.rs:722-730`, `R/runtime/workspace.rs:249-256`), and "rows
    scrolled out are absent" was false: overscan, focus or pointer pinning
    and `keep_mounted` retain offscreen rows (`R/virtual_list.rs:822-826`,
    `:400-408`, `:896-900`). Revision 3 defines the operation by one
    relative segment under `find`'s rules, covers parts, structural
    parents, duplicate relative addresses, slots and drag previews, returns
    every mounted addressable row, and documents the controller route for
    unmounted rows; P5's tests cover each.
  - **D5:** `next_deadline` is the selected window's work plus shared work
    (`R/runtime.rs:5574-5600`), not cross-window; that is
    `application_deadline` (`R/runtime/native_windows.rs:1385`). Revision 3
    aligns the check with `Driver::settle`, lists the every-window scope as
    the alternative and adds P4's two-window test.
  - **Folded into P8's checkpoint and tests:** presentation geometry for
    promoted surfaces and detached windows (`R/runtime/tree.rs:1818-1824`),
    correlation of reported rectangles with the last successful layout
    (`R/runtime/tree.rs:1893-1896`), and `click_item` as pointer input at
    the visible center without a guaranteed action, with the resize-grip
    case (`R/table.rs:2638-2646`).
  - **P3:** the new receipt type follows the module-closure rules
    (`component`, the root, the prelude if L2's closure requires it).

- **Revision 3, Codex recheck (2026-10-27):** D3, D5, the P8
  qualifications and P3 resolved; one blocking point: D4's walk stopped at
  a `Part`, while `Runtime::find` continues through a part's descendants
  without consuming its name (`R/runtime/tree.rs:319`). Revision 4 returns
  a part and continues below it, and P5's part test requires both the part
  and its addressable descendant when enumerating their ancestor, and the
  descendant when enumerating the part.
- **Revision 4, Codex recheck (2026-10-27):** D4's part traversal and
  P5's part test match `Runtime::find`; nothing else changed. Ready for the
  maintainer's decisions.

## Progress

- **P1 Documentation (0.3.76), 2026-10-28.** Implemented by a fresh agent
  from this section; R2-5, R2-7 to R2-10, R2-12 to R2-16, R2-18, R2-19 and
  R2-28 as listed (CHANGELOG 0.3.76). Deviations: R2-15's failed restore is
  a refusal by the control itself (the application lowers the limit below
  the pre-drag value), since probes showed a restore to a UI-local `Signal`
  never fails in `cancel_input`; R2-12 uses `option_env!` with the system
  temporary directory as fallback, since a doctest has no
  `CARGO_TARGET_TMPDIR`; R2-16's feature-table row links the four
  serialized documents and each type's page carries the note; R2-14 changed
  the facade's `representative_programs_import_only_their_globs`, which
  encoded the old rule. Each new doctest was shown to fail under a
  temporary mutation of its claim. **Same kind, fixed here** (the author's
  review sent back three entries the agent had put in TODO.md): every
  runtime and core rustdoc example that globbed the crate root, the
  packaged `dynamic_documents` example and the gallery's root globs now
  import `prelude`, modules and named items; the `IntegerInput` page's
  local-signal example moved out from under "Parameter binding"; an
  unused-imports pass over the rewritten examples removed unused imports
  and fixed `KeyChord::named`'s self-comparison. **TODO.md:** one entry
  for the untouched examples with unused imports and the proposal to deny
  them in doctests. Gate (fresh): fmt, workspace Clippy, workspace
  doctests, the facade and gallery tests, the five headless sessions and
  the headless gallery, rustdoc `-D warnings` (default and `serde`), the
  diagnostics snapshots, the API listing gate (no `api/` diff), `git diff
  --check`. Native evidence: not applicable. Codex review: pending.

- **P2 Glob membership (0.3.77), 2026-10-28.** Implemented by a fresh
  agent from the approved design (`round-2-design-P2.md`, revision 3):
  the 23 names in both `testing` modules and `CpuRenderError` in the
  facade's with `cpu`; `ThemeMode` in both preludes; `pert_ui::svg`
  renamed `pert_ui::svg_images` (TODO.md entry removed); the closure check
  `tests/diagnostics/tests/testing_closure.rs` over the five
  configurations with its 18 self-tests; `every_name` and
  `every_glob_combines` over the full union including `host` (same-kind
  fixes the design names). **Deviation:** design section 4 says a
  member's owner is the deepest path segment naming a declared type; at
  the base that misassigns 17 listing paths whose variant shares a type's
  name (`SemanticValue::Choice::…`, `ContentTarget::Row::…`,
  `EditorTrigger::Tab::…`), so the check uses the first type segment,
  which is what the section's stated purpose requires (types do not nest);
  a self-test covers it. Accepted by the author. Mutation evidence (kept in
  `target/p2-evidence/`): the check over the pre-change listings reports
  exactly the 23 names (22 plus `CpuRenderError` in the `cpu`
  configurations, 22 without `NativeWindowId` in the default one) with no
  parse or ambiguity failure; not following fields and variants drops the
  eight names the design predicts; an unreached and a member exclusion
  fail; a name removed from the facade's list only fails the facade-
  contains-runtime assertion; the contract removals fail to compile; a
  scratch crate reproduces E0659 at 84bb60f and compiles at P2. No fixture
  check: the task 4 fixture uses none of the changed paths. Gate (fresh):
  fmt, workspace Clippy, workspace tests, the five headless sessions,
  doctests, rustdoc `-D warnings`, the diagnostics snapshots (no `.stderr`
  change), Clippy, tests and rustdoc for `cpu`, no-default, `desktop-cpu`
  and `svg`, Clippy and tests for no-default `svg`, the API listing gate,
  `git diff --check`. Native evidence: not applicable. **Codex review** (2026-10-28): needs fixes, three
  should-fix points in the closure check, all verified: a method's own
  generic bounds were not scanned (`<F: FnOnce(B)>` never reached `B`;
  14 runtime methods have such bounds); same-name aliases with different
  targets compared equal; a `testing` member that resolves to no
  declaration was dropped from the seeds silently. Fixed in the check with
  three self-tests, each failing with its fix reverted; a
  `PERT_UI_API_DIR` override runs it against other listings: on the
  committed ones it passes, on 84bb60f's it reports exactly the original
  23-name evidence. Codex's recheck: one should-fix, a single-letter
  type export (`B`) that resolves to nothing was still skipped by the
  spelling rule; non-type exports are now recognized by their listing kind
  (`pub fn`/`pub const` lines), with `B`, a renamed type, a function and a
  constant in the self-test. Second recheck: pending.
