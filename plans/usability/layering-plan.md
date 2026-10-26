# Layering pass before round 2

Status: **revision 5**, after Codex's evaluation of revision 1 and its
rechecks of revisions 2 to 4 (record at the end), and after the maintainer
widened the scope on 2026-10-26: the usability work exists to improve the
API, so this pass changes code structure and public APIs where that
improves them, as AGENTS.md's compatibility policy allows while the
library is unpublished. Codex found LD1 to LD6 decidable. **The
maintainer approved the plan with every recommended answer to LD1 to LD6
on 2026-10-26** ([decisions](#decisions-for-the-maintainer)).

This plan runs after the API audit fix plan's last part (6f3, 0.3.70) is
closed and before slice 7 of [the developer-usability
plan](../developer-usability.md) (blind test round 2). Its evidence is the
[complexity inventory](complexity-inventory.md) (eb0591b, measured at
6219dda).

Round 2 measures the **combined** result of the fix plan and this pass
against round 1, as exploratory evidence; it cannot attribute a difference
to layering alone ([round 2](#round-2-and-attribution)).

## Goal and principle

The inventory found that the API grew slowly while its rules and its
discovery surface grew fast, and that the accidental complexity is mostly
**flat discovery** and **unnamed rules**, not surplus capability
(inventory, summary and section 7). This pass therefore first **layers**:
every capability stays reachable, but each audience meets only its own
layer until it looks further. It then **removes and merges** where the
inventory found a second way that exists only for history or an
avoidable trap (parts L3 and L4).

| Layer | Audience | Meets |
|---|---|---|
| 0 | An application's first program | controls, containers, `Signal`, `App`, lists and forms, controlled controls |
| 1 | An application with a specific need | parameters and authority, commands and shortcuts, windows and surfaces, live data, inspection and tests |
| 2 | A custom-widget author | `Component` and its contexts, painting, redescription rules |
| 3 | A custom-host author | the host-integration methods of `Runtime`, presentation, wake delivery, capture serving |
| 4 | Maintainers | implementation rationale and internal ordering |

Rules for every part:

- **One taught way per concept.** Where several ways exist, the layer-0
  documentation teaches one and names the others as advanced, with the
  reason to choose them.
- **Breaking changes are allowed where they improve the API**, by
  AGENTS.md's policy for the unpublished library: each is explicit in its
  part's scope, has a CHANGELOG `Breaking:` entry with replacement usage,
  and migrates every maintained consumer in the same change. Persisted
  schemas keep their own versioning rules.
- **Behavior changes only by design.** L1 and L2 change no behavior. L3
  changes API shape and, in N7, the address contract (persisted workspace
  keys, inspection paths and selectors), each under its own checkpoint.
  L4's behavior changes each start with a design checkpoint.
- **Later parts keep earlier guarantees.** Once L2 has tested which globs
  each audience needs (`prelude`; `prelude` and `testing`; `prelude` and
  `component`), no later part may add a required import to those
  programs.
- **Counts stay in the inventory.** Shipped documentation describes
  audiences and guarantees, never the totals this plan cites.
- **Review.** Each part is one commit (plus review fixes) with an
  independent Codex review of the implementation; L2 also has a design
  checkpoint before any code.

## Part L1: documentation layering (0.3.71)

Rustdoc, the packaged guide and `docs/`; no public item changes, and no
Rust code moves. It takes its own version, as the fix plan's parts do.

1. **contracts.md by audience, per obligation** (inventory S2). The
   inventory classified sections by their *primary* audience, and several
   "maintainer" sections contain obligations others rely on: menu binding
   and callback replacement on redescription (`contracts.md:279`), a
   custom host draining shutdown errors (`:1042`), `keep_alive`
   destruction order (`:1049`), workspace restore cancelling active
   gestures (`:1227`). So:
   - The part first classifies each paragraph and obligation as
     application, extension (widget or host) or maintainer, in a table in
     its review record.
   - Only implementation rationale and internal ordering detail move to a
     new `docs/internals.md` (decision LD3). Every application and
     extension guarantee stays in contracts.md, in its layer.
   - Extension-author material moves under one heading, "For component and
     host authors", after the application sections.
   - Links are checked both ways: incoming links (from `docs/`, `plans/`,
     rustdoc, the guide, the crate-root docs, AGENTS.md and TODO.md) and
     outgoing fragment links inside moved text, plus bare references such
     as "the compatibility policy".
2. **Callback timing classes, separate from the other properties**
   (inventory S3). Timing alone is classified; write access, coalescing,
   cancellation and failure and replay rules stay separate properties,
   because they vary independently: `Tabs::on_select` reports each settled
   change while collection selection coalesces between updates
   (`contracts.md:641`), geometry coalesces per flush (`:393`),
   `on_modified` keeps each change in order (`text_fields.rs:549`), and a
   background completion runs in the first flush after its arrival
   (`runtime/application.rs:144`).
   - The part first builds a complete classification table. Its coverage
     is every public API that takes a callback the runtime calls later or
     during an event, whatever its name: the `on_*` builders in the
     runtime and the facade, constructors that take one (`controlled`,
     `Button::new`), `normalize`, command registration, mailboxes,
     `spawn`, the three deferral calls (`EventContext::defer`,
     `defer_detached`, `Runtime::defer`: run before component evaluation,
     may flush, `runtime/tasks.rs:98`) and `Runtime::start_interval`,
     whose ticks can also run outside a flush through `service_intervals`
     (`runtime/application.rs:272`, `:360`). Two groups are
     listed apart from the rows. Pure binding derivations
     (`ReadSignal::map` and `combine`, which run when the derived value
     is evaluated or read, `signal.rs:730`) and description factories
     (`Dynamic`, `Show` and `Each`'s callbacks, which run during update,
     `described.rs:76`) are classified as "evaluation" callbacks in one
     sentence each: they must be pure, and the table says so.
     Host-integration callbacks (such as `Runtime::set_wake_delivery`'s,
     which runs on the notifying thread, `runtime.rs:4344`) are excluded
     from the classes and keep their own threading and side-effect
     contracts. Each other row gives its timing class and its other
     properties.
   - The timing classes are derived from that table, not fixed in advance;
     a callback that fits none keeps its own description.
   - Each callback's rustdoc names its timing class with a link and keeps
     its family-specific guarantees in its own words. Wording such as
     "after the control's own write" is qualified for controlled
     proposals.
3. **"Which way" tables** (inventory S5). One at the top of the state
   chapter and one at the top of the layout chapter: where a control's
   value comes from (`new`, `controlled`, `for_parameter`; the per-row and
   retained forms as advanced), children from data (`Each`, `VirtualList`,
   `Dynamic`, `Show` versus `visible_signal`), background work
   (`EventContext::spawn`, `Runtime::spawn`, `Mailbox`, live data), and
   deferral (`defer`, `defer_detached`, `Runtime::defer`, with the
   closing-surface rule).
4. **The state chapter opens with a signal and a control** (inventory S6).
   Its custom `Component` example moves to the custom-widget chapter.
5. **Controlled first** (inventory S1, the part 6f3 did not cover).
   `Signal::sibling`'s rustdoc and the guide name the per-row binding as
   the advanced form for controls without a controlled form.
6. **Audience indexes on reference pages** (inventory S4), without moving
   code:
   - `Runtime`'s type documentation gains an index of its methods by
     audience ("Applications", "Tests and inspection", "Hosts"), each
     entry an intra-doc link. Splitting impl blocks is not used: the
     methods already span many modules, and the `api/` listing records
     each inherent impl block (`api/pert-ui-runtime/default.txt:18281`,
     `:18290`), so regrouping would churn it.
   - `Component`'s trait documentation gains a table of its hooks by
     concern (lifecycle, input, layout, painting, notifications,
     inspection, workspace), with the few a typical widget implements
     first. The declaration order is unchanged.
   - The rendered pages are reviewed to confirm the index helps
     navigation.
7. **The anti-regrowth rule** (decision LD4), if approved: its sentence in
   AGENTS.md and the decision record.

**Gate:** the rustdoc/public-examples row of the AGENTS.md table
(targeted doctests, warning-free rustdoc), `cargo fmt --all --check` and
targeted Clippy for the edited `.rs` files, the API listing gate to show
`api/` unchanged, and link checks for `docs/`, `plans/` and the guide.

## Part L2: a layered prelude (0.3.72)

Breaking in imports only. It starts with a **design checkpoint**, reviewed
by Codex and approved by the maintainer, as parts 6f1 to 6f3 had.

### The membership rule (decisions LD1, LD6)

- **Representative programs.** The checkpoint names the application
  programs the prelude must serve, for example the getting-started
  program, the guide's settings form, the editable-list recipe's
  application part, a dialog and a menu. Their test harnesses are not
  application code: names only a harness writes (`Driver`, inspection
  queries) do not count toward prelude membership.
- **Membership.** G5-26's operational test is kept: a name is required
  when a program must write it (an argument it constructs, an enum it
  matches, a value it keeps past the call), not when a value is only
  passed on (`decisions.md:1545-1556`). The prelude becomes the names the
  representative programs require. Its size is an estimate (about 120 to
  150 names), not a target.
- **Completeness guarantee** (decision LD6), by audience:
  - application code of each representative program compiles with
    `use pert_ui::prelude::*` alone;
  - its tests compile with `prelude` and `testing`, as the facade teaches
    today (`crates/pert-ui/src/lib.rs:148`);
  - a custom component compiles with `prelude` and `component`.

  Each is a test.
- **The `component` and `testing` guarantees stay.** G5-26's rule for
  `component` (every name implementing `Component` writes) and the
  `testing` module are kept. Today's prelude test asserts that a custom
  component needs only the prelude (`tests/foundation/prelude.rs:1-7`);
  it becomes "the prelude and `component`", and the application programs
  get their own prelude-only tests. Only the application prelude's
  membership rule in 6e is superseded.

### Thematic modules

- `parameter`: the parameter and edit-authority names (decision LD2).
- `commands`: command and key names.
- `windows`: native window, region, split and tool-window names.
- `inspection`: inspection, capture and profiling names for applications.
- `live`: the live-data family (`SeriesSource`, `series_bridge`,
  `SeriesConfig`, `Timestamp` and the rest), which G5-11 recommended
  (`decisions.md:1477`) and slice 9 still holds as an input
  (`app-author-improvements.md:375`).

Each module is complete for its family: it includes the root-only names
the family needs, not only today's prelude members, with stated
exclusions. A name may appear in several modules; the crate root keeps
every name.

**Names.** `windows`, `inspection` and `live` are private implementation
modules of the runtime today (`crates/pert-ui-runtime/src/lib.rs:37`,
`:229`, `:240`). The checkpoint settles, per name, whether the private
module is renamed or the public module gets another name. The public
`live` module gathers both live-data families, the runtime's UI side and
the core's bridge, which enter the root separately today (`lib.rs:304`,
`:318`). Root exports are kept unless the checkpoint removes one
deliberately as part of a stated break.

**Collision review.** As 6e required
([the 6e section](api-audit/fix-plan.md#6e-prelude-and-component-module)), the checkpoint
checks every combination of globs a program might write (`prelude` with
each thematic module, `component` and `testing`) for distinct items with
the same name, and against common consumer-local names. Overlapping globs
of the same item are valid. It also settles the facade's exposure through
its runtime glob, where each item is documented, and `#[doc(no_inline)]`.

### Documentation placement (decision LD2)

The parameter material in the guide's state chapter (`guide/state.md:63`
onward) moves to a chapter of its own, "Authoritative values", so the
namespace and the documentation layer agree.

### Migration

- **A migration manifest** in the checkpoint lists the maintained
  consumers (the packaged examples, the guide, the gallery, the native
  checks, the facade's and runtime's tests and doctests, the diagnostics
  fixtures, `plans/usability/task4-app`) and the frozen ones that are not
  migrated (the round 1 submissions and probes under
  `plans/usability/runs/` and `~/pert-ui-usability/`).
- **Diagnostics fixtures** keep their intent: a fixture that implements a
  sealed trait (`tests/diagnostics/ui/s01_stateowner_impl.rs:2`) must
  still produce the sealed-trait diagnostic, not a missing-name one.
  Snapshot changes are reviewed like the `api/` diff.
- **Rendered pages** of the prelude and each module are reviewed, and the
  doctests pass in the default, `--no-default-features` and the feature
  configurations whose prelude differs (`native`, `svg`, `cpu`).
- CHANGELOG `Breaking:` entries give the replacement imports.

### Records

- `decisions.md`: G5-26's application-prelude rule is superseded; its
  `component` rule stays.
- App-author slice 9: the smaller prelude and the thematic modules,
  including the live-data module, are met and leave its inputs. Its other
  items stay.

**Gate:** the public-API row, including the API listing gate and the
diagnostics snapshots, plus the feature configurations above.

## Part L3: one way where history left two (0.3.73)

Breaking. N3, N6 and the `find_path` removal change API shape only; N7
also changes the address contract; layout and editing behavior are
preserved. A short design checkpoint
lists each removal or move with its replacement and the consumers to
migrate; Codex reviews it and the maintainer approves it.

- **N3, host integration off `Runtime`.** The host-integration methods
  (presentation requests, wake delivery, capture serving, window
  selection, IME state, `paint` for hosts) move to a `host` module: an
  extension trait implemented by `Runtime`, or a host handle borrowed from
  it, which the checkpoint chooses. `App`, the native checks and custom
  hosts import it; application code no longer sees them on `Runtime`. L1's
  audience index then lists only what remains.
- **N6, `Runtime::mount`.** The legacy root that is never laid out is
  removed; `mount_tree` covers laid-out roots, and the gallery
  self-checks, facade tests and probes that use it migrate (or the
  checkpoint moves it to `testing` if a test needs an unlaid root).
- **N7, lookups.** `find_path` leaves the public surface (the inventory
  found two application uses; internal callers keep a crate-private
  form). `Each`'s default container stops adding its `items` segment, so
  a child's address is its own key under the `Each`'s key. This is an
  **address-contract change**: the default container is an ordinary keyed
  element (`described.rs:333`), and workspace keys
  (`runtime/workspace.rs:219`, `:251`), inspection paths
  (`runtime/inspect.rs:194`) and selectors (`:379`) all include it. Its
  checkpoint therefore requires:
  - old workspace documents: a versioned rejection or an unambiguous
    migration of paths through a default `Each` container, under
    AGENTS.md's persistence rules, with text and serde compatibility
    tests;
  - inspection and selector tests for the new paths, and the inspection
    schema consequence stated (a version step if the schema's path
    contract changes);
  - that an explicitly keyed container given with `.within(...)` keeps
    its segment: only the default container changes, never a segment the
    application named, even if it is called `items`.

**N3's checkpoint** enumerates the exact methods that move, every
consumer outside the facade (the CPU renderer serves captures through
them, `pert-ui-cpu/src/lib.rs:266`, and renderer checks call `paint`),
the facade's exposure of the `host` module, and a public custom-host
example that compiles against it.

**Gate:** the public-API row, including the API listing gate, plus, for
N3, AGENTS.md's dependency/feature/platform row (MSRV, the supported
feature configurations, packaging and the packaged-consumer check, and
the native checks), recording any native evidence that cannot be run.

## Part L4: behavior unifications (0.3.74 and 0.3.75)

Each item changes behavior, has task evidence, and gets its own design
checkpoint (Codex review, maintainer approval), implementation and review,
as the fix plan's 6f parts did.

- **L4a, N4: one deferral call.** `EventContext::defer`,
  `defer_detached` and `Runtime::defer` differ in what cancels the task
  and in which window it runs in: widget-bound work uses the widget's
  window at delivery, detached work the window captured at enqueue, and
  runtime-bound work the selected window (`runtime/tasks.rs:196-204`,
  `:325-332`). The widget-bound default is a trap on surfaces that close
  (`contracts.md:494-496`, `guide/custom-widgets.md:284-289`). The design
  specifies cancellation ownership and execution-window routing
  separately, including a widget detached between enqueue and delivery;
  keeps capacity, ordering, nested flushes, receipts and failure
  propagation unless it redesigns them explicitly; tests surface close,
  window close, detachment and shutdown; and, if a default changes,
  names exactly which callers change behavior. It migrates every
  caller.
- **L4b, N5: one draft policy.** Text and number editors differ in what
  revocation and a window close do to a draft (`contracts.md:355-362`,
  `:914-923`; F4, F6). Some differences follow authority and ownership,
  not presentation: a parameter-bound number draft cancels on focus loss,
  `DraftEditor` follows its owner, and sessions have their own terminal
  reporting (`contracts.md:994`, `:1110`). The design starts from a
  matrix of editor and presentation against ownership (local signal,
  controlled, parameter, session), validity, revocation of enablement or
  editability, focus loss and close admission. It states each intended
  change to retained drafts, committed values, proposals, refusals and
  session notifications, keeps authority boundaries unless it designs a
  change to them, and never hides an implicit commit or a lost terminal
  outcome behind "one policy". Close-admission and session tests are
  included.

**Gate:** the public-API row for each.

## Round 2 and attribution

- If L2 runs before round 2, the parent plan's paragraph on slice 9 and
  round 2 (`developer-usability.md:41-46`) is updated in L2's change: the
  prelude and module layout land on this branch, so round 2 measures
  them. Otherwise the paragraph stays, and the attribution record names
  the parts that landed.
- Slice 7 states that round 2 is an exploratory comparison of the combined
  changes since round 1: the fix plan, 6f3 and this pass.
- The inventory's headline numbers are measured twice under a new dated
  heading: at 6f3's close (before L1) and after this pass's last part, so
  the post-6f3 baseline and this pass are recorded apart.
- Before any blind run, slice 7 freezes the counting definitions (lines,
  distinct types and imports per task, if the maintainer adopts them),
  keeps the reserved task 5's comparison against the round 1 commit, and
  records the library, documentation, fixture and kit revisions used.

## Not in this plan

- **N8**: the `Inline*` controls as presentations of the standalone ones
  (decision LD5). It is the largest change (about six types and seventy
  builder methods) and the only one without tester friction; it goes to a
  plan after round 2 unless the maintainer pulls it in.

## Decisions for the maintainer

All six were decided on 2026-10-26 as recommended.

- **LD1: the prelude now, not in app-author slice 9.**
  - **Recommended:** part L2 before round 2, superseding 6e's
    application-prelude rule (G5-26's `component` rule stays), so round 2
    meets it.
  - **Alternative:** defer L2 to slice 9's own breaking branch, after
    round 2; L3 and L4's timing follows LD5.
- **LD2: parameters as a layer-1 module and chapter.**
  - **Recommended:** the parameter and authority names leave the prelude
    for `parameter`, and the guide's parameter material moves to an
    "Authoritative values" chapter. The protocol is unchanged.
  - **Alternative:** they stay in the prelude and the state chapter.
- **LD3: a maintainer document.**
  - **Recommended:** `docs/internals.md` for implementation rationale and
    internal ordering only, classified per obligation, listed in AGENTS.md's
    design-reading list (an instruction-file change).
  - **Alternative:** that material stays in contracts.md under a final
    "Implementation notes" heading, with no AGENTS.md change.
- **LD4: a rule against regrowth** (inventory question 4), made in L1.
  - **Recommended:** AGENTS.md's component conventions gain a sentence:
    an addition that offers a new way to do something already possible
    names, in its change rationale, the way it makes advanced or retires,
    or says why none can be. Shipped documentation applies the "one taught
    way" rule instead of repeating it.
  - **Alternative:** no rule; reviews judge each addition.
- **LD5: how much changes before round 2.** L3 and L4 do not depend on
  L2's prelude, so this decision stands on its own; if LD1 defers L2 to
  slice 9, L1, L3 and L4 can still run before round 2, and the
  attribution record names the parts that landed.
  - **Recommended:** L1 to L4 before round 2 (layering, the prelude, N3,
    N6, N7, N4 and N5), so round 2 meets the improved API; N8 after
    round 2, having no tester friction and the largest migration.
  - **Alternative A:** L1 to L3 before round 2 (L2 only if LD1 is
    recommended), with L4's behavior changes after it, which brings
    round 2 sooner.
  - **Alternative B:** N8 too before round 2, as an L5 with its own
    design checkpoint.
- **LD6: the prelude's completeness guarantee.**
  - **Recommended:** the representative programs compile with the prelude
    alone, as tests; the prelude makes no promise beyond them.
  - **Alternative:** a broader guarantee, such as every name any guide
    chapter for layer 0 writes, at the cost of a larger prelude.

## Progress

- **L1 (0.3.71), implemented 2026-10-26:** 6360b00 and its review fix;
  done after Codex's recheck.
  - **contracts.md by obligation:** the per-obligation classification is
    in [layering-L1-classification.md](layering-L1-classification.md).
    Only implementation rationale and internal ordering moved, to
    `docs/internals.md`, which links each note back; extension material
    sits under "For component and host authors". The compatibility, error
    and naming sections stay as application material: per obligation they
    describe the API's shape, unlike the inventory's section label.
  - **Callback timing:** contracts.md's "Callback timing" defines the
    classes derived from the classification table (in the input, after
    settling, at the next flush, at parameter delivery, evaluation) and,
    separately, the properties beside the class. Callbacks that fit no
    class keep their own description (`App`'s initializer, `on_close`,
    `on_error`, `on_window_error`, command actions, held-input
    cancellation, `Popover::on_dismiss`, the file browser's request
    callback); host-integration callbacks are excluded. Each classified
    callback's rustdoc names its class with a link.
  - **Guide:** "choosing a way" tables in the state and layout chapters;
    the state chapter opens with a signal, a field and a derived label,
    and its custom component example moved to the custom-widgets chapter;
    the controlled form is taught first.
  - **Reference pages:** `Runtime` has a "Methods by audience" index and
    `Component` a "Hooks by concern" table (eleven groups rather than the
    plan's seven names, splitting input by kind); no code moved.
  - **AGENTS.md:** `docs/internals.md` in the design-reading list (LD3) and
    the anti-regrowth sentence (LD4).
  - **Checks run after the last edit:** `cargo fmt --all --check`,
    workspace Clippy, the workspace doctests, warning-free workspace
    rustdoc, the API listing gate (no change), `git diff --check`, and a
    link check over `docs/`, `plans/`, the guide and the sources, whose
    errors are the same as at HEAD before the change (all pre-existing).
    The rendered `Runtime` and `Component` pages were reviewed: every
    index link resolves.
  - **Deviations:** the state chapter's new example no longer asserts the
    runtime's next deadline (a derived binding keeps the runtime due); a
    few application paragraphs gained one-sentence summaries where they
    named moved material.
  - **Review (Codex):** needs fixes, with one blocker and five should-fix
    points and a nit, all verified: observable guarantees (cancellation
    rules custom components rely on, search exclusion after a failed
    reconciliation, cleanup-error delivery) had moved to
    `docs/internals.md`; numeric normalizers and parsers were classed "in
    the input" although they run during update, paint and inspection;
    "in the input" omitted saves during explicit focus changes; the
    `Runtime` index made test operations host-only; `RegionLayout`
    reports once per divider, not per component; the runtime-access table
    omitted interval ticks; and a test's link to contracts.md was broken.
    **The fix:** every moved sentence was tested again by whether an
    application or extension author observes or relies on it, and each
    one that does is back in contracts.md, with a new extension section
    "Failure cleanup in components"; `internals.md` keeps mechanisms only,
    and no classification row remains purely maintainer. Numeric
    normalizers and parsers are evaluation callbacks; "in the input"
    covers focus changes and states where errors go; the index has a
    "Standing in for a host" group under tests; geometry and the access
    table are corrected; the link is fixed. After it: the L1 gate again
    (formatting, Clippy, doctests, rustdoc, the API listing gate with no
    change, the link check with only pre-existing errors, `git diff
    --check`).

## Review record

- **Revision 1, Codex evaluation (2026-10-26):** needs revision, with one
  blocker and seven should-fix points and a nit. It found layering the
  right response and L1 before L2 sensible, and confirmed the inventory's
  key counts. Revision 2 addresses each point:
  1. **Blocker, impl-block split:** replaced by an audience index of
     intra-doc links; no code moves (L1 step 6).
  2. **Timing classes:** timing is classified separately from access,
     coalescing, cancellation and replay, from a complete table of every
     callback first (L1 step 2).
  3. **contracts.md:** classified per obligation; only implementation
     rationale and internal ordering move; links checked both ways (L1
     step 1, LD3).
  4. **Membership rule:** representative programs, G5-26's operational
     test kept, complete thematic families, the size an estimate, and a
     new decision on the completeness guarantee (L2, LD6).
  5. **Module names:** the collision with the private `windows` and
     `inspection` modules and the glob-combination review are settled in
     the checkpoint (L2).
  6. **Omitted inputs:** the live-data module is added; LD2 covers the
     parameter chapter; slice 9 keeps the items not met.
  7. **Migration and verification:** a manifest of maintained and frozen
     consumers, the prelude test's new contract, diagnostics intent,
     rendered pages and feature configurations, and an implementation
     review for both parts.
  8. **Attribution:** round 2 is a comparison of combined changes; the
     parent plan and slice 7 are updated; the numbers are measured at
     6f3's close and after L2; counting definitions are frozen first.
  9. **LD4 and LD5:** LD4 is made in L1 and applies to change rationale;
     LD5 is framed by scope and risk, and N7's existing mitigation is
     noted.
- **Revision 2, Codex recheck (2026-10-26):** needs revision, with three
  should-fix points and a nit; points 1, 3 and 6 to 9 were resolved and no
  blocker remained. LD1 to LD5 were found decision-ready. Revision 3
  addresses each point:
  1. **Timing coverage:** defined by every public API that takes a
     callback, including the three deferral calls and
     `Runtime::start_interval`, with stated exclusions (L1 step 2).
  2. **LD6:** application code compiles with the prelude alone, its tests
     with `prelude` and `testing`, custom components with `prelude` and
     `component`; harness-only names do not count toward membership.
  3. **`live`:** included in the module-name decision; the public module
     gathers the runtime's and the core's live-data families.
  4. **Citation:** links the 6e section.

  The same revision applies the maintainer's scope decision of
  2026-10-26 (breaking changes that improve the API are wanted): parts L3
  (N3, N6, N7) and L4 (N4, N5) are added, LD5 is reframed, and only N8
  remains outside the plan.
- **Revision 3, Codex second recheck (2026-10-26):** needs revision, with
  six should-fix points; revision 2's points 2 to 4 were resolved, no
  blocker remained, L1 to L4's order and versions were found feasible,
  N8's deferral justified, and LD2 to LD4 and LD6 decision-ready.
  Revision 4 addresses each point:
  1. **Exclusions:** binding derivations and description factories are
     described by their real phases and classified as evaluation
     callbacks (L1 step 2).
  2. **N7:** classified as an address-contract change, with old-document
     handling, compatibility, inspection and selector tests, and explicit
     `.within(...)` containers keeping their segment (L3).
  3. **N3's gate and consumers:** the platform row, the renderer
     consumers, the facade exposure and a custom-host example; later
     parts keep L2's import guarantees (L3, rules).
  4. **N4:** cancellation and window routing are specified separately,
     with the guarantees to keep and the tests (L4a).
  5. **N5:** an ownership and authority matrix, with no hidden commit or
     lost terminal outcome (L4b).
  6. **LD1 and LD5:** their independence is stated; LD5 holds whether or
     not L2 runs before round 2.
- **Revision 4, Codex third recheck (2026-10-26):** needs revision, with
  one should-fix point and two nits; revision 3's points 2 to 5 were
  resolved, and LD1 to LD6 were found decidable, with L1 able to start
  once its decisions are recorded. Revision 5 makes the corrections:
  1. **Host callbacks** are excluded with their own contracts, not
     classified as pure evaluation callbacks (L1 step 2).
  2. **L3's opening** separates API-shape changes from N7's address
     contract.
  3. **LD1's alternative** and the attribution paragraph follow LD5's
     independence.

  The corrections are wording that Codex specified; they are not sent for
  another recheck, and the next review (L1's implementation review)
  verifies them.
