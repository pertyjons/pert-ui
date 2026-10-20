# Plan: repairs found by auditing the completed plans

Status: proposed 2026-10-20 and revised after a Codex review; slices 0 to 9
are implemented on branch `plan-audit-repairs` (0.3.45 to 0.3.51). Slice 10's
archiving is done and its hand checks wait for the maintainer. Every type, variant and method named below as new is a
proposal until a slice implements and documents it.

## Background

On 2026-10-20 the five plans in this directory were audited against `main`
at 5a6e451, twice and independently: once by Codex (`codex exec -s
read-only`, one run per plan) and once by a Claude agent per plan. The
findings only one auditor reported were then verified by a further Codex
run. All the evidence is static: no auditor ran Cargo. The audited plans and
the commits that merged them are:

| Plan | Versions | Squash merge |
|---|---|---|
| [app-author-improvements](app-author-improvements.md) | 0.3.2–0.3.8 | 116ab0c |
| [mixer-probe-improvements](archive/mixer-probe-improvements.md) | 0.3.9–0.3.17 | 116ab0c |
| [diagram-probe-improvements](archive/diagram-probe-improvements.md) | 0.3.18–0.3.23 | 0ba201b |
| [log-explorer-improvements](archive/log-explorer-improvements.md) | 0.3.24–0.3.31 | 9d23503 |
| [table-editing-improvements](archive/table-editing-improvements.md) | 0.3.32–0.3.44 | 5a6e451 |

Every public API the plans promise exists, and every version has a
CHANGELOG entry. What the audit found falls into five groups:

- **Defects.** Reconciliation errors from requested children (`Dynamic`,
  `Each`, `Show`) leave no `FailureSite`, although app-author slice 5
  promised context for reconciliation errors: `Runtime::flush` propagates
  the error from `reconcile_children` without noting the owner. A `Label`
  whose text exceeds `TextEditError::MAX_TEXT_BYTES` fails the flush, even in
  an ellipsis table cell, so one long record in application data fails the
  window. The `live_mixer` example's headless session has two unbounded waits
  (`held.recv()` and the loop until the engine reports `Closed`), and its
  resume handshake on a zero-capacity channel and its teardown join can
  block as well, although its plan says every step waits boundedly. No CI step or test runs the
  packaged examples' headless sessions; CI only compiles them.
- **Promised tests that do not exist.** Each plan's contract-test lists
  contain cases the implementation never got. Slices 4 to 6 below list
  them.
- **Documentation, manual and gallery.** The plans' working rules have every
  slice update the manual and the gallery. Several slices did neither, and
  `docs/verification.md` describes the 0.3.8 native suite as covering
  "flex layout and selection notifications in the gallery", which the
  gallery never demonstrated. Some rustdoc is stale after later slices.
- **Open work recorded only in a plan.** Optional follow-ups, decisions left
  open and items dropped from a slice are written only in the plan that
  dropped them, partly contradicted by later work.
- **Stale records.** Every plan's status line still says it is "being
  implemented on branch …". Those branches are deleted. The review records
  omit the per-slice implementation reviews. Some notes contradict the code
  (a "not done" that is done, tests listed as missing that exist, an Escape
  outcome that differs from the checkpoint). `docs/verification.md` has no
  record for 0.3.32–0.3.44.

Two reported findings were refuted during verification and are not in this
plan: `Runtime::find` resolving a `Part` and an ambiguous address is tested
(`crates/pert-ui-runtime/tests/input_contracts.rs`), and the
`UnboundedCollection` report is shown to clear on redescription through
`Dynamic` (`crates/pert-ui/tests/foundation/unbounded_collections.rs`).

Slices 8 (operating-system services) and 9 (tutorial, prelude and module
layout) of the app-author plan are not audit findings. They are open feature
work with their own plan, and this plan does not implement them. Slice 0
keeps that plan open for them and carries the inputs later plans left for
slice 9 into it.

## Working rules

The [app-author working rules](app-author-improvements.md#working-rules)
apply, with these specifics:

- One branch, `plan-audit-repairs`, from `main`. One commit per slice, plus
  review-fix commits. No merge or push without the maintainer's instruction.
- A workspace patch version bump per slice that changes a packaged file
  (code, rustdoc, packaged guide or example), with its CHANGELOG entry.
  Slices 0 and 9 change only plans, `docs/` and `TODO.md` and take no
  version.
- An independent Codex review (`codex exec -s read-only`) of every slice
  before it is called done. Slices 1 and 2 begin with a design checkpoint
  that Codex reviews before implementation. Codex cannot run Cargo in its
  sandbox; the author runs the gates and the regression tests.
- A test the audit lists that turns out to expose a defect is fixed in the
  same slice, with the defect named in the CHANGELOG. A listed case that
  turns out to be impossible or meaningless is dropped with a sentence in
  the slice's status line saying why, never silently.
- Gates follow the AGENTS.md table. Test-only slices run the workspace gate
  because they cross packages. Slice 3 changes the CI workflow and therefore
  updates AGENTS.md's gate description in the same commit. Slice 8 changes
  the gallery and records a native run of the gallery interaction checks.
- Refer to sections and items, not plan line numbers: slice 0 rewrites the
  plans and the line numbers move.

## Slices in order

### 0. Records

Status: done (documentation only, no version).

Documentation and plans only, no version.

- **Plan status lines.** Rewrite each audited plan's status line to say
  which slices were implemented, in which versions, and which squash merge
  holds them. Drop "being implemented on branch …" and the "is a proposal
  until" sentence for implemented slices. The app-author plan stays open for
  slices 8 and 9 and says so; the table-editing header names slices 6 and 7
  (0.3.40–0.3.44).
- **Review records.** Add the per-slice design checkpoints and
  implementation reviews, with their fix versions, to each plan's review
  record, taken from the slice status lines, `docs/verification.md` and the
  merge commits. Where a review is claimed only in a commit message, say so.
- **Notes that contradict the code.**
  - Mixer slice 4: remove "Not done: the receipt's captured target is not
    asserted";
    `a_command_button_captures_the_context_at_activation` asserts it. Reword
    contract test 4, since a `CommandReceipt` carries no context. Number its
    contract tests in order.
  - Mixer slice 4: "icon-only command buttons stay out of scope" points to
    `Button::command_icon` (0.3.34).
  - Log-explorer slice 1: the scaled `child_transform` and
    `Justify::SpaceBetween` after shrinking are tested. Slice 2: the headless
    runtime has no clipboard, and the phase and reentrancy test exists.
  - Log-explorer slice 3: `SelectableText::text_key_event` returns
    `TextKeyOutcome::Ignored`, not `NotAnEditor`, for an Escape it does not
    use. Escape still reaches enclosing bindings and surfaces, which a test
    shows. Record that as the implemented decision.
  - Log-explorer slice 1: the `UnboundedCollection` report keeps no mounted
    range and gives the visible share as a percentage, not in pixels.
    Record that as a deliberate simplification: the row count, content
    extent and allocation it keeps are enough to act on.
  - Log-explorer slice 4: the before and after timings were measured on the
    example's 50 000-line default, not the probe's 200 000 lines. Point to
    slice 9.
  - Diagram slice 0: it shipped as 0.3.18, so correct the bullet that says
    documentation takes no version bump. The headless-testing guide it names
    does not exist: point to `EventContext::now` and `custom-widgets.md`
    instead.
  - App-author slice 4: the row alignment and no-shrink decisions are
    superseded by 0.3.10 (allocation-height alignment) and 0.3.25 (bounded
    column shrinking). Mark them as superseded and link to the changes.
- **Open work in one place.** Add `plans/follow-ups.md`, an index of the
  optional and deferred work the closed plans leave open. Give each item
  its origin, what an application need would have to show before work
  starts, and a link to where its decisions live. Durable decisions stay in
  `docs/` (for example the declined signal observer in `docs/contracts.md`).
  TODO.md stays limited to verified deviations, as AGENTS.md requires. The
  index takes:
  - From app-author: find in an editor buffer, a longer editor undo history,
    duplicate-dependency pruning (two `tiny-skia` versions, `thiserror` 1 and
    2, winit's Adwaita decorations, arboard's X11 fallback) and default
    insets.
  - The general signal observer, as declined by the mixer checkpoint and
    `docs/contracts.md`, with what would reopen it.
  - From diagram: a writable model projection and a typed state handoff from
    `App::new`. The caret policy for a `TextField` focused from code is
    partly answered by `TextField::session` with `EditorStart`; record
    whether a standalone policy is still wanted.
  - From log-explorer: indexed or paged tree children with incremental
    flattening, horizontal cell virtualization, incremental source updates,
    and text selection across collection rows.
  - From table-editing: range selection with multi-cell copy and paste, a
    fill handle, a row-level edit transaction, and sessions in `Tree` and
    `PropertyGrid`. Enter moving down after a confirm, which slice 6
    dropped but its follow-up list omits. Sessions for `InlineText`, the
    inline number rows, `TextArea` and `SearchField`. Controlled controls
    (a `Checkbox` from a `bool`), with the conditions slice 6 set. The
    removed-column rule, which gets a trigger and a test when columns
    become removable.
- **App-author slice 9 inputs.** Carry the mixer plan's prelude and
  `border_box` discoverability findings into the app-author plan's slice 9.
  Note there that `getting-started.md`, `getting-started-continued.md` and
  the screenshot section of `diagnostics.md` already cover part of the
  tutorial and capture chapter, so slice 9 starts from them.
- **TEXT-01.** Add the app-author slice 6 requirement to separate the
  editor capacity from the label and toast limits.
- **Verification record.** Add a "Table-editing plan" section to
  `docs/verification.md` stating what is known: each slice was
  Codex-reviewed (from the plan and the merge commit). Move the inventory
  release-session timings there from the plan. Say that per-slice gate runs
  were not recorded, and record instead the workspace gate on `main` at
  5a6e451, which this slice runs. Reword the 0.3.8 native-suite row so it
  claims only what the gallery then contained.
- **Native evidence.** Add a TODO.md entry NATIVE-05 for the
  `diagram_editor`, `log_explorer` and `inventory` windows, `SelectableText`,
  table cell editing and Tab scopes, none of which has native evidence.
  Slice 10 closes its Linux part.

### 1. Failure site for reconciliation errors

Status: done in 0.3.45 as the checkpoint below decided, with its review
corrections. Tests: `crates/pert-ui/tests/foundation/failure_sites.rs`. The
implementation review added these:
- A request queued behind a rejected one is reported by the next flush.
- An earlier site of the same flush, from a nested flush in a deferred
  task, stays authoritative.
- A call refused at entry (`TimeReversed`) leaves the earlier site.
- Exact `Display` text for the other phases.

The `last_failure` rustdoc now states that boundary: a site is reset only
by an admitted outermost call.

Design checkpoint first. The checkpoint decides:

- **The variant.** Add `FailurePhase::Reconcile` (the enum is
  `#[non_exhaustive]`), and name the component that requested the children
  in the `FailureSite` path, not the child whose key or type was rejected.
  Alternatively, record the rejected child's key in a new field: the
  checkpoint chooses one. Its `Display` form is "reconciliation of …".
- **Coverage.** Every error `reconcile_children` returns while the flush
  services requested children: `DuplicateKey`, `ChildTypeChanged` and
  errors from mounting a new child. Also whether the mount-time path
  (`mount_tree`, keyed children described in `update`) already notes a
  site, so the same failure is not reported twice or with two phases. A
  site an inner component already recorded (a failing `update` of a new
  child) stays authoritative; the owner is named only when nothing inside
  recorded one.
- **No new payloads on `RuntimeError`.** Existing error matching keeps
  working, as slice 5 decided.

Tests: a duplicate key from `Each`, a changed child type under one key from
`Dynamic`, and a failing mount of a new child from `Show`. Each asserts
`Runtime::last_failure` and the `App` default report's text. A later
successful flush clears or replaces the site, as the existing phases
document. The manual's failure-reporting text and `last_failure` rustdoc
name the new phase.

#### Design checkpoint (2026-10-20)

Today `Runtime::flush` services the children that components request
through `UpdateContext::request_children` (which `Dynamic`, `Show`, `Each`
and custom components use) by calling `reconcile_children` per owner, and
returns its error without noting a site. New children are mounted there
without running their `update`; that runs later in the same flush and
already notes an `Update` site when it fails. So the failures this slice
covers are the description checks that reject the whole request before
anything changes:

- `EmptyKey`, `DuplicateKey`, `ChildTypeChanged` (same key, other component
  type, at any depth of the description);
- an error from `Component::check_description` (public, so custom
  components can fail it);
- `Exhausted`, `InvalidChild` for an owner that cannot take children, and
  errors from preparing the new children.

Decisions:

- **A new phase.** `FailurePhase::Reconcile`, written "reconciliation" in
  `FailureSite`'s `Display`. `FailureSite::path` names the component whose
  request failed, as for the other phases: it is a mounted component, the
  key path is meaningful to inspection, and the owner is the code that
  described the children.
- **A new field for the rejected description.** `FailureSite` gains
  `description: Vec<String>`, the keys from the owner's requested children
  down to the rejected element, for example `["row-3"]` for a duplicated
  key or `["row-3", "label"]` for a nested type change. It is empty for the
  other phases and for errors that name no element (`Exhausted`, an owner
  that cannot take children). For `EmptyKey` it names the keys down to the
  element whose child has the empty key. `Display` appends it as
  "reconciliation of form/list at row-3/label". Keeping it separate leaves
  `path` meaning "keys of mounted components" for every phase. The struct
  is already `#[non_exhaustive]`, so the field is not a breaking change for
  consumers, who cannot construct it.
- **Where the description comes from.** The crate-private validation
  (`validate`, `validate_types`, `discard_owned_descriptions` and
  `prepare_children`) records the keys it descends through into a
  crate-private trail when it returns an error. The public
  `Runtime::reconcile_children` keeps its signature and errors.
- **Only in the flush.** The site is noted in `apply_requested_children`
  only, through the existing `note_failure` rule: the first site of an
  outermost flush wins, so a site a nested callback already recorded stays
  authoritative. Application code calling `Runtime::reconcile_children` or
  `mount_tree` directly gets the error from that call, and records no site.
  The same holds for a deferred task, which `last_failure` documents as
  failing outside any component.
- **No new payloads on `RuntimeError`**, as app-author slice 5 decided.

Contract tests (facade, `tests/foundation`, beside the existing
`last_failure` tests):

1. `Each` over a source with two items mapped to the same key: the flush
   fails with `DuplicateKey`. `last_failure` is `Reconcile` at the `Each`'s
   path, with the description running through the default `items` column
   to the duplicated key (`["items", "dup"]`), and `to_string()` is
   "reconciliation of root/each at items/dup".
2. `Dynamic` redescribing a child under the same key with another component
   type, and nested one level down: `ChildTypeChanged`, and the description
   names the changed element in both cases.
3. `Show` whose shown content fails `check_description`: that error, with
   the shown element's key.
4. A custom component requesting children that contain an empty key: the
   description stops at that element's parent.
5. A new child whose `update` fails in the same flush: the site is that
   child's `Update`, not `Reconcile`.
6. A rejected request leaves the mounted children unchanged. The next
   successful flush returns `None` from `last_failure`. A failing direct
   `Runtime::reconcile_children` call records no site.
7. The `Display` of the existing phases is unchanged (an empty
   description adds nothing).

`App`'s default report formats the site through `Display`, so test 1's text
is what it prints. It is not asserted through a native window.

Codex reviewed this checkpoint (2026-10-20) and approved it, with these
corrections, which are part of the decision:

- **Not atomic in every case.** Preparation can fail after earlier new
  siblings were mounted; the rollback removes them again. So the promise is
  that the surviving topology is unchanged, not that no lifecycle effect
  ran. `InvalidChild` also rejects descendants described for an existing
  self-managed child; the trail names that child.
- **A request-local trail.** It is created for each `reconcile_children`
  call, captured before the rollback and discarded after the call. More
  tests:
  - Preparation rollback.
  - A moved-child collision from `discard_owned_descriptions`.
  - Retained constructor descriptions discarded before validation.
  - Several owners across successive flushes.
  - An owner unmounted before its request runs.
  - Recovery by an unchanged flush and by a newly valid value. Only later
    requests are queued again; the failed request is consumed.
- **The reporting boundary.** The site is reset by the outermost flush,
  paint, `flush_windows` and interval service, not by `note_failure`. A
  direct `reconcile_children` call records no new site but leaves an
  earlier one in place. The `last_failure` rustdoc says so, and a test
  shows it.
- **Callback wording.** `FailurePhase`, `FailureSite::phase` and `path`,
  `Display`, `last_failure` and the manual's failure text describe a
  component callback today. `Reconcile` is runtime work done for a
  component, so they are reworded, and the contracts and CHANGELOG record
  the owner path versus the description path. Panics are excluded:
  preparation catches one only to roll back and resumes unwinding, so there
  is no error to annotate.
- **Compatibility.** Adding the variant and the field is source-compatible
  because both types are `#[non_exhaustive]`. Derived equality now includes
  the description, and the `Debug` output changes. Neither type implements
  serde.

### 2. Display text beyond the text limits

Status: done in 0.3.46 as the twice-revised checkpoint below decided.
Tests:
- `crates/pert-ui/tests/presentation/display_text_limits.rs`;
- the cut and fit tests in `crates/pert-ui-runtime/src/text.rs`.

The implementation review found one blocker: a line of exactly the limit
could still try a candidate one ellipsis over it. The fit now treats a
candidate beyond the limit as too wide. It also found three should-fix
findings, all fixed:
- An overflow change no longer separates the kept layout from its shown
  text.
- The cut tests place clusters, whitespace and regional-indicator runs of
  both parities at each share boundary.
- Lifecycle tests:
  - disagreeing measurement and arrangement widths;
  - open help closing on a refusal and returning after it;
  - nothing drawn before a success, and the kept text clipped after.

Design checkpoint first, then implementation. Today a `Label` (and any
display text using the same preparation) over `MAX_TEXT_BYTES` returns
`TextEditError::TooLarge` from layout, which fails the flush. A byte bound
alone does not settle it: preparation also rejects layouts beyond the raster
limits (`TextLimit::MAX_RASTER_SIDE` and the coverage budget), and a
wrapping text of a few kilobytes at a narrow width already exceeds them, as
a `SelectableText` test shows. The checkpoint decides between:

- **Bounded display (recommended).** Display text prepares at most the byte
  limit, cut at a grapheme boundary. When that candidate still exceeds the
  raster limits, it is reduced further to what fits, by a defined rule (for
  example whole lines from the start), so preparation of display text cannot
  fail on size. It is shown with the label's overflow treatment: an
  ellipsis for `Ellipsis` and `MiddleEllipsis`, a trailing ellipsis for a
  wrapping label.
- **Typed refusal.** The label keeps its last valid presentation and reports
  the refusal through an event-phase notification, as `on_binding_refusal`
  does for editors, with a defined first presentation when no valid one
  exists yet.

The checkpoint names every existing contract it keeps or migrates, with the
rustdoc, tests and `docs/contracts.md` sections involved:

- `SelectableText` already keeps its last valid presentation, selection and
  copy on oversized input. Decide whether it adopts the new rule, and
  whether copying copies the displayed or the full text.
- `Label` promises the full text as its inspection name and as delayed help
  when truncated. Decide what an oversized text's name and help are, and
  bound the help, since a `Tooltip` prepares the same way.
- Rich content, the first presentation, and recovery when the text becomes
  valid again.
- Editors keep their documented refusal.

If the truncation gets a development diagnostic, the checkpoint decides its
representation: a new `ValidationCode` is part of the inspection snapshot,
and the snapshot's revision rules in `diagnostics.rs` and the serialization
test decide whether `SCHEMA_VERSION` changes. The workspace persistence
schema does not change for this.

Tests:

- An ellipsis table cell over the byte limit.
- A wrapping label over the byte limit, and one under it that exceeds the
  raster limits at a narrow width and at scale 2.
- A multi-byte grapheme at the cut.
- Recovery to a valid text.
- The diagnostic in debug, and its absence in the release profile, run with
  `--release`.

Gates: the workspace gate, the release-profile tests of the touched test
files, and the serde configurations if the diagnostic changes the snapshot.
Update the log-explorer plan's follow-up for long display text to point
here.

#### Design checkpoint (2026-10-20)

How it fails today:

- `Label::prepare` asks `MeasureContext::rich_text` (wrapping and rich
  content) or `MeasureContext::fitted_text` (plain text with an ellipsis
  mode). Both return `TextEditError::TooLarge` above `MAX_TEXT_BYTES` and
  `TextError::TooLarge(TextLimit::RasterSide | Coverage)` when the prepared
  layout exceeds the raster limits. `Label::measure` and `arrange` return
  the error, so the flush fails.
- `fitted_text` runs grapheme segmentation over the whole text and
  measures the whole first line before cutting it. A 1 MB log line is
  segmented and shaped in full, then refused.
- The help of an ellipsis label is a wrapping `Label` over the same source,
  so it fails the same way when it is shown.
- `SelectableText` already refuses such text without failing the flush.

Decisions:

- **Ellipsis fitting is bounded** (`fitted_text`, so custom components
  benefit as well). Plain text longer than `MAX_TEXT_BYTES` is cut before
  fitting, at grapheme boundaries:
  - `Ellipsis` keeps the start, up to the limit.
  - `MiddleEllipsis` keeps the same number of grapheme clusters from the
    start and from the end, together within the limit, and joins them.
    The ellipsis then falls at the join.

  The cut text is then fitted exactly like text with a line break: it
  always ends up truncated and shows the ellipsis. Its `shown` text is what
  `FittedText::shown` returns, as now. `TextOverflow` and `fitted_text`
  document the cut. A long line in an ellipsis table cell therefore shows
  its start (or its start and end) with an ellipsis, and no work scales
  with the full length beyond one bounded scan.
- **`Label` refuses what it still cannot prepare**, with the
  `SelectableText` contract: text over the byte limit in `Wrap` mode or as
  rich content, or a layout beyond the raster limits at the current width,
  style or scale.
  - The flush does not fail.
  - The label keeps its last prepared layout and size. Before its first
    success it has no layout: zero size, nothing painted.
  - It reports the refusal as the `text_error` inspection property and
    marks its semantics invalid.
  - It recovers when a later text, width, style or scale fits.

  Other errors, such as a binding from another runtime or a foreign font,
  still fail the update or layout.
- **Inspection keeps the full text.** The `text` property and the semantic
  name stay the whole caption, as documented. For a cut ellipsis label,
  `truncated` is true and `shown` is what it shows.
- **Help.** An ellipsis label offers its delayed help only for text within
  `MAX_TEXT_BYTES`. A longer text could not be shown whole in a tooltip
  anyway, and the full text stays readable through the name. A help text
  within the byte limit that still exceeds the raster limits at the
  tooltip's width is refused by the help's own `Label`, under the rule
  above. The ellipsis label therefore does not show that help: it asks its
  help label, through the crate-private state it already shares, whether
  the help is prepared, before making the help surface visible.
- **No new diagnostic.** The `text_error` property is the developer-visible
  evidence, as for `SelectableText`, so `ValidationCode` and the inspection
  schema do not change, and no serde configuration is affected.
- **Other captioned controls** (buttons, chips, links, menu rows, titles)
  keep failing on text they cannot prepare. Their captions are commands
  and names, not application data. The follow-up index records extending
  the refusal to them if a consumer needs it.

Contract tests (facade, `tests/editing` beside the `SelectableText` tests
or a new `display_text_limits.rs`):

1. An ellipsis `Label` in a `Table` cell whose text is twice
   `MAX_TEXT_BYTES`: the flush succeeds, `shown` starts with the text's
   start and ends with "…", `truncated` is true, and the name is the full
   text.
2. The same with `MiddleEllipsis`: `shown` keeps the start and the end
   around one "…".
3. A cut that would split a multi-byte grapheme cluster (an emoji sequence
   or a base with combining marks at the limit) keeps whole clusters.
4. A `Wrap` label over the byte limit: no flush failure, zero size before
   any success, `text_error` set and semantics invalid. Then a valid text
   recovers, and an oversized one after it keeps the valid layout and size.
5. A `Wrap` label under the byte limit that exceeds the raster limits at a
   narrow width and at scale 2: refused as in 4, and recovered when the
   width grows.
6. Rich content over the byte limit with an ellipsis mode: refused as
   in 4.
7. An ellipsis label over the byte limit offers no help on hover; one just
   under it whose help exceeds the raster limits shows no help surface.
   An ordinary truncated label still shows its help.
8. `fitted_text` from a custom component on an oversized text returns a
   truncated result and does not shape the whole text, shown with the
   service's shaping counter in a crate test.
9. Release profile: tests 1, 4 and 8 run with `--release`.

#### Revision after review (2026-10-20)

Codex rejected the checkpoint above: the help rule was circular, since
hidden help is not laid out, and the forced cut lacked a byte budget for
the ellipsis and a cache distinction. Its other findings are applied too.
This revision replaces the decisions and tests above where they differ.

- **Today's cost, corrected.** An oversized fitted text is segmented and
  copied into the fit cache's key, then rejected by resolution before it
  is shaped. It is not shaped in full. The bounded cut removes the
  segmentation and the copy.
- **The forced cut.** It applies to the first line, after hard-break
  detection, as truncation always does. When that line is longer than the
  budget `MAX_TEXT_BYTES - "…".len()`, the cut works as follows:
  - Only a bounded window is segmented: the budget's bytes from the start
    and, for `MiddleEllipsis`, from the end. The tail window starts at a
    character boundary, and its first, possibly partial, cluster is
    dropped.
  - Finding the line break remains a byte scan of the text.
  - `MiddleEllipsis` keeps the same number of clusters from each window,
    within the budget together. The cut keeps the clusters' original
    boundaries and the join position. It is never segmented again, and
    whitespace is trimmed only when a candidate is built.
  - The candidate split puts the ellipsis exactly at the join when every
    kept cluster stays. Fewer kept clusters come from both ends.
  - One cluster longer than the budget leaves nothing on that side, so the
    result can be "…" alone.
  - The cut result always shows the ellipsis, and its byte length stays
    within `MAX_TEXT_BYTES`.
  - The fit cache key records that the text was cut, so a cut text never
    aliases an ordinary text with the same bytes.
- **`Label` refusal as a state.**
  - The refusable errors are the size limits: `TextEditError::TooLarge` and
    every `TextError::TooLarge(_)`, raster side, coverage and
    `TextLimit::Segments` alike, as `SelectableText` treats them.
  - `measure` returns the last size it measured successfully, or zero before
    any success.
  - `arrange` keeps the last successful `prepared` layout and `shown` text
    together and records the allocated size. A refused label always paints
    clipped to that size.
  - The refusal recorded by `arrange`, the last layout phase, is
    authoritative for inspection.
  - When the refusal starts or ends, or help eligibility changes, `arrange`
    requests an update. The update then reconsiders the help, as it already
    does when truncation starts or stops. Help that is open when the label
    becomes refused closes.
- **Help, simplified.** An ellipsis label offers help only while it is cut,
  not refused, and its text is within `MAX_TEXT_BYTES`. A help text within
  the byte limit that exceeds the raster limits at the tooltip's width is
  refused by the help's own `Label`, and the panel is then empty. That is
  documented rather than prevented: no state is read from a hidden child.
- **Every embedded `Label` adopts the refusal**, by design:
  - table and tree cells an application builds from `Label`;
  - an ellipsis label's help, and `Tooltip::new`, whose panel can then be
    empty;
  - `AlertDialog`'s title and message;
  - `Group` headings.

  Controls that prepare their captions themselves (buttons, chips, links,
  menu rows, tab and window titles) keep failing, and the follow-up index
  records extending the refusal to them.
- **Inspection differs from `SelectableText` on purpose.** A refused
  `Label` keeps naming its current source text in `text` and in its
  semantic name, since that is the application's value. `SelectableText`
  names the text it still shows, because its selection refers to it. Both
  set `text_error` and mark their semantics invalid.
- **Documentation updates:**
  - `Label` and `Label::overflow`: full help only within the byte limit; the
    refusal state.
  - `TextOverflow`, `FittedText` and `MeasureContext::fitted_text`: the cut,
    its budget and that it always shows the ellipsis.
  - `Tooltip::new`.
  - The text-source section of `docs/contracts.md`, the manual's text row,
    and the CHANGELOG.

Contract tests, replacing the list above:

1. An ellipsis `Label` in a `Table` cell with twice `MAX_TEXT_BYTES`: the
   flush succeeds, `shown` is the start and "…", `truncated` is true, the
   name is the full text, and `shown` is within `MAX_TEXT_BYTES`.
2. `MiddleEllipsis` on the same text: start and end around one "…" at the
   join. Variants:
   - Asymmetric byte sizes, with ASCII at the start and multi-byte text at
     the end.
   - Whitespace and a combining mark next to the join.
   - Regional-indicator flags across the window edges.
3. Exact budgets:
   - A line of exactly `MAX_TEXT_BYTES` bytes is not cut by the bound.
   - A line one byte over it is cut, and the result fits the limit with the
     ellipsis.
   - A single cluster over the budget gives "…".
4. A cut text in a very wide label still shows the ellipsis. A cut result
   and an ordinary text with the same bytes do not share a cache entry,
   warm or cold.
5. A `Wrap` label over the byte limit:
   - Zero size and no paint before any success, `text_error` set, semantics
     invalid, and the flush succeeds.
   - A valid text recovers.
   - Another oversized text keeps the valid layout, clipped to the new
     allocation.
   - Same-key redescription across these states.
6. A `Wrap` label under the byte limit beyond the raster limits at a
   narrow width and at scale 2. It is refused there and recovers when the
   width grows, also when `measure` and `arrange` see different widths.
7. An ellipsis label whose single-line layout exceeds the raster side in a
   very wide allocation is refused. Rich content over the byte limit, and
   over `RichText::MAX_SEGMENTS`, is refused.
8. Help:
   - None over the byte limit.
   - Open help closes when the text becomes refused, and returns while
     hovered once the text is valid and cut.
   - An ordinary truncated label still shows its help.
9. Embedded labels: an oversized `Tooltip::new` text, an `AlertDialog`
   message and a `Group` heading do not fail the flush.
10. A crate test of `fitted_text` on a 1 MB line: the longest text shaped,
    from a new test-only maximum in the text service, is within
    `MAX_TEXT_BYTES`.
11. Release profile: tests 1, 5 and 10 run with `--release`.

Second review (2026-10-20): one blocker, since segmenting windows can
misplace boundaries inside a regional-indicator run or an unfinished
cluster, and three should-fix findings. Resolved as follows:

- **No windows.** The head is taken by iterating the line's grapheme
  clusters forward from its start, and for `MiddleEllipsis` the tail by
  iterating them backward from its end (`unicode-segmentation` iterates
  both ways with correct boundaries). Each side stops once its share of the
  budget is used. So every boundary is a source boundary.
  - The work is bounded by the budget, except where the segmenter must look
    back through a long run of regional indicators or one huge cluster.
  - An unfinished cluster at the head's end is never taken, since only
    whole clusters are.
  - The tail's share is half the budget and the head's the rest; the longer
    side is then reduced to the other's cluster count.
- **Trimming by boundaries.** `candidate` trims whitespace clusters using
  the boundaries it is given, instead of segmenting its slices again, so
  the join and the source boundaries hold.
- **Trigger and budget.** A line is cut when it is longer than
  `MAX_TEXT_BYTES`. The kept clusters then use at most
  `MAX_TEXT_BYTES - "…".len()` bytes, so a line of exactly
  `MAX_TEXT_BYTES` is not cut by the bound.
- **Refused help is cleared.**
  - An ordinary `Label` that becomes refused keeps its last valid layout.
  - The help label of an ellipsis label, and the label of `Tooltip::new`,
    clear it instead, through a crate-private flag set where they are built.
    Their panel is then empty, never stale.
  - A test takes a help from valid to refused.
- **Further tests:**
  - Regional-indicator runs of both parities around the head's end and the
    tail's start.
  - A combining sequence extending past the head's share.

### 3. Example sessions in CI and bounded waits

Status: done in 0.3.47. Each step has its configuration and placement:
- The CI step runs right after `cargo test --locked --workspace`, so it
  reuses that build: the debug profile, before the first `cargo clean`.
  It uses `--features png,svg,cpu`, the features that build unifies through
  the gallery, so `diagram_editor` also runs its CPU export.
- It runs `live_mixer`, `dynamic_documents`, `diagram_editor`,
  `log_explorer -- --lines 10000` and `inventory -- --items 2000`, with a
  ten-minute step timeout.
- The audit of the other examples found more unbounded waits, all now
  bounded or removed:
  - the proposal wait in `dynamic_documents`;
  - after the review, the inspection thread's join in `log_explorer`;
  - `live_mixer`'s teardown, which now also waits until the thread has
    returned.

  `getting_started` opens a window and is not run.
- The Codex review found the join, the features and the AGENTS.md command
  block; all are fixed.
- Local debug timings on Fedora 44 (24 threads): `live_mixer` 0.2 s,
  `dynamic_documents` 0.1 s, `diagram_editor` 0.4 s, `log_explorer` at
  10 000 lines 9.3 s (mostly its fixed follow-mode waits), `inventory` at
  2 000 items 1.7 s.
- The first CI run after the change records the added time.

- **Bounded waits in `live_mixer`.** Every blocking step of the session
  gets a deadline that returns an error naming the step:
  - `held.recv()`;
  - the loop waiting for `SinkErrorKind::Closed`;
  - the resume handshake on the zero-capacity channel (`resume.send`);
  - the teardown that joins the producer thread.

  Keep the deterministic hold that the full-queue step depends on.
- **The other examples.** Audit `diagram_editor`, `log_explorer` and
  `inventory` for the same unbounded waits and bound any they have.
- **CI.** Add steps to `.github/workflows/ci.yml` that run each packaged
  example's headless session in the debug profile, so development
  validation runs, with a moderate size where the example takes one
  (`--lines`, `--items`). Each step has a `timeout-minutes` as containment.
  The slice names, for each example, the exact feature configuration from
  those `docs/verification.md` records, and the place in the workflow
  relative to its `cargo clean` boundaries, so that a step reuses the build
  of its group instead of adding one. Update AGENTS.md's gate text and
  table in the same commit, as it requires.
- **Before adding the steps,** measure each session's time on the
  maintainer's machine and choose sizes for it. Local timings do not show
  the added CI build time; the first CI run after the change does, and the
  slice records it.

### 4. Missing tests: app-author and mixer

Status: done in 0.3.48. The tests are in:
- `foundation/application_services.rs`, `signal_writes.rs`,
  `callback_conveniences.rs`, `serialization.rs`, `row_stretch.rs` and
  `startup_viewport.rs`;
- `editing/binding_refusal.rs`;
- `surfaces/menu_changes.rs`.

Two listed cases needed more than a test:
- **A full command queue as `Button::command`'s error.** It is reached by
  arming the button with Enter, filling the queue from `try_event` on the
  same button (which keeps its focus and the press), and releasing Enter.
  A first attempt that targeted another component cancelled the press.
  The implementation review found that route.
- **"A collapsed in-view window"** cannot be built:
  - an in-view window keeps its minimum size, so its content always has
    area, which a test shows;
  - minimizing collapses it to the title bar and hides the content, which
    is then not validated, and a second test shows it reported again once
    restored.

  The mixer plan's expectation is corrected accordingly.

The implementation review also made two other tests strict:
- the short slider's readout must be drawn at every height and cut exactly
  where the fader ends;
- the interval test counts ticks outside the runtime, so shutdown cannot
  hide one.

The audit lists these tests as promised but missing:

- **Interval timer.** A running application interval survives the window
  losing focus and `Runtime::cancel_input`, and stops at shutdown, as
  app-author slice 5 promised and the CHANGELOG claims.
- **Binding refusal.**
  - Consecutive invalid values with the same reason, with the
    notification policy the rustdoc states.
  - Replacing one callback with another.
  - A failing callback followed by an unchanged flush, proving there is no
    replay.
- **`Signal::update`.** Document what a panicking closure leaves: `update`
  skips its own final write of the changed copy, while writes the closure
  made itself before panicking remain, as the rustdoc permits. Document that
  fallible edits use `get` then `set`. Test an ordinary panic and a
  write-then-panic.
- **Error and capture conveniences** (mixer slice 4):
  - `source()` is kept through the chain for an error with its own cause.
  - The serialized form of both typed capture constructors, including a
    window target.
  - A full command queue reported as `Button::command`'s error.
- **Menu callbacks** (mixer slice 3): pointer choice in a nested menu and a
  context menu, and keyboard activation of a direct menu-bar title.
- **Row stretch** (mixer slice 2): a row with unbounded height, and a short
  vertical slider or meter with its readout, asserting what is clipped.
- **Startup viewport** (mixer slice 1):
  - A secondary native window with zero height.
  - A collapsed in-view window asserting the startup diagnostic, which
    `startup_viewport.rs` covers only with an empty split pane.

### 5. Missing tests: diagram and table-editing slice 1

Status: done in 0.3.49, tests only.
- **Files:**
  - `controls/color_picker.rs` and `controls/knob.rs`;
  - `foundation/click_counter.rs` and the unit test in `click.rs`;
  - `editing/input_adornments.rs`, `text_fields.rs` and
    `open_selected.rs`.
- **The knob's refused admission** uses a faulted parameter, an enabled
  knob whose `begin` fails. An edit held by another context is cancelled
  when that context ends, so it cannot hold the parameter.
- **The click-series resets** are shown by a click, a press on the clear
  button or bar, and a click at the first place within the interval, which
  must be a single click.
- **Mutation checks:** each new reset and refusal test fails with its reset
  removed.
- **The implementation review** found four tests that could pass with the
  behavior broken. Each now observes its premise:
  - the typed draft before it is dropped, and the saved value after;
  - the bar press leaving the caret;
  - the replacement shown and wholly selected before typing;
  - the knob's refused and recovered presses beginning no edit and an edit.

- **ColorPicker** (diagram slice 2):
  - Alpha End.
  - Removal with an open hex draft and with a channel draft.
  - Hiding a channel draft.
  - Replacing the handler while the popover is open, which the existing
    redescription test does not cover.
  - No notification on removal while a callback is installed.
- **Knob:** an enabled knob whose edit admission is refused (forcing the
  `begin` failure), followed by a successful press within the click
  interval.
- **`ClickCounter`:**
  - Builder replacement precedence, and a threshold changed during an
    existing series taking effect from the next press.
  - Saturation, as a unit test in `click.rs` seeding the count near
    `u32::MAX`.
  - The text editors' clear-button reset and scrollbar reset, which the
    slice status recorded as untested.
- **Select-all on open** (table-editing slice 1): an external replacement of
  the bound value while mounting, for each editor in the family, not only
  `IntegerInput`.

### 6. Missing tests: table-editing slices 4 to 6

Status: done in 0.3.50, tests only.
- **Files:** `controls/tab_traversal.rs`, `editing/editor_sessions.rs` and
  `collections/table_editing.rs`.
- **Coverage:** every listed case has a test, with its premise observed,
  and none was dropped. The table's earlier `key_input`-only tests now run
  on both key paths.
- **Mutation checks:** each part has one or more, among them a scope that
  walks the logical parent, a composition no longer protecting the
  session, and a refused completion treated as terminal. Each failed the
  intended tests and was restored.
- **No defect in the tested contracts** was found. These findings remain:
  - Disabling an editor releases focus through cancelled input, so its
    session reports nothing and stays open. `TextField::session` and the
    contracts do not say so; slice 7 documents it.
  - A hidden native window delivers the `FocusLeft` it caused once it is
    shown again, as hidden windows hold notifications. Slice 7 documents
    it.
  - `ToolWindowMode::Window` given at mount with `expanded(true)` opens no
    native window, though `ToolWindow::mode` and the type's rustdoc promise
    it; going through the controller does. That is a contract mismatch
    outside this plan's scope, recorded in TODO.md (TOOL-01).
  - **The implementation review** found five weak tests; all are now
    strengthened:
    - **Entering an open popover by traversal.** A `Popover` cannot be
      entered: focus outside it dismisses it, as the nonmodal surface
      contract requires, which a test now asserts. Entry in the deferred
      place is shown with a notification surface, which focus leaving does
      not dismiss, in both directions.
    - **A hidden editor's dropped draft.** Shown again, the editor holds
      the saved value, and Enter saves nothing.
    - **Parameter numbers with dirty drafts.** Changed and invalid drafts
      behave the same with and without a session, and nothing is reported.
    - **Tab during a selection drag.** Shift+Tab is covered too, and the
      drag survives both.
    - **The unmount test** now runs on both key paths.

- **Traversal scopes** (slice 4):
  - A scope inside a detached view and a tool window.
  - Window and application dialogs opened inside a scope.
  - A scoped nonmodal popup reached in its deferred place and dismissed when
    traversal leaves it.
  - Tooltips staying out of traversal.
  - The menu bar and F10 with a scope active.
  - Tab repeats against a claimed Tab.
- **Editor sessions** (slice 5):
  - A consumer-defined list host whose rows open editors through the public
    API, with Tab navigation between rows and a refused edit kept open.
    This replaces the single-editor `Host` as the proof the plan asked for.
  - A normalization-equivalent draft reporting `Unchanged`.
  - A refused Tab completion.
  - IME composition protecting the session, and a Unicode start during
    composition.
  - Permission revocation during a session.
  - Window focus loss versus hiding, through the runtime's window entry
    points.
  - Pointer selection in an open `Select`.
  - `DecimalInput` sessions.
  - Parameter-backed numbers ignoring sessions.
- **Table cell editing** (slice 6):
  - An editor that cannot take focus.
  - A read-only revocation keeping focus.
  - A disabled editor closed by Escape on the body.
  - Unmount with an edit open, asserting no session survives. Hiding with an
    open edit is already tested.
  - The keyboard scenarios that run only through `key_input`, including
    reopening a refused edit, also through the command path, as slice 6
    requires.

### 7. Documentation, guide and example repairs

Status: done in 0.3.51. Beyond the list:
- The session findings of slice 6 are documented in `TextField::session`,
  the number fields' `session` and the editor-session section of
  `docs/contracts.md`.
- The manual also names `KeyChord::named`, `?` on runtime errors in command
  actions, `CaptureTarget::window` and `Runtime::find`.
- The guide's error-handler example is a compiling `no_run` doctest.
- **Codex review:** three rounds corrected the routing order the rustdoc
  first gave:
  - orphan repeats, owned and suppressed keys and modal rejection;
  - fresh presses versus repeats per stage;
  - where bubbling starts.

  The review also corrected the manual's `?` claim for `Runtime` variants,
  the guide's initialization wording and the stroke extent at miters and
  caps.

- **Rustdoc.**
  - `DraftEditor`: Tab reaches an owner that claims it through
    `Component::tab_navigation`.
  - `Runtime::command_key_input`: `# Errors` names `tab_navigation`, and the
    routing order includes help dismissal before Escape and the Tab offer.
    `guide/diagnostics.md`, which calls that rustdoc the complete order,
    stays consistent with it.
  - `Painter::stroke_path` and `stroke_polyline`: link to `stroke_shape` and
    its inward stroke, as `stroke_ellipse` does.
- **Manual.**
  - `on_binding_refusal` in the text-input inventory.
  - `live_mixer` among the packaged examples.
  - `View::fixed` among the bounding containers.
  - A sentence that a sort can move an edited row out of view, pointing to
    `TableController::reveal`.
  - The APIs from mixer slices 3 to 5 where the manual's inventories cover
    their families.
- **Guide.**
  - `state.md`: power mapping (`ParameterMapping::Power`, `PowerExponent`).
  - `appearance.md`: `background_role_signal`.
  - `controls.md`: `on_binding_refusal` beside the refusal it describes.
  - `layout.md`: replacing a tree source re-flattens its expanded branches.
  - `custom-widgets.md`: `App::on_error` reaching application state,
    including an empty slot during initialization, and a short export
    section on rendering a scene to PNG with the CPU renderer, pointing to
    the example.
- **`diagram_editor` example.** Install an `App::on_error` that reaches the
  application state through the same slot as `on_close`, so the guide's
  text has a running counterpart.

### 8. Gallery demonstrations

Status: done; the gallery is not published, so no version. Each
demonstration has a headless gallery test:
- **Layout › Layout:** flex growth, a spacer, `Justify::SpaceBetween`,
  `Align::Stretch` and column growth, plus a box whose fill follows
  `background_role_signal`.
- **Data & media › Collections:** a selection-change line with its origin
  under the grid, tree and table.
- **Controls › Chips & breadcrumbs:** `Each` and `Show` beside the
  existing `Dynamic`.
- **Controls › Progress:** a clock on `Runtime::start_interval`.
- **Data & media › Images:** decoding moved from a thread and mailbox to
  `Runtime::spawn`, with a generation guard for stale results.
- **Data & media › File browser:** its mailbox is kept by
  `Runtime::keep_alive` instead of being captured by a callback.
- **Text › Text input:** a field showing `on_binding_refusal`.
- **Controls › Parameter families:** a power-mapped parameter.
- **Surfaces › Menu bar:** View rows reporting through
  `MenuItem::on_change`.
- The README rows follow, and the native gallery interaction checks
  passed afterwards; see [verification](../docs/verification.md#plan-audit-repairs).
- **The Codex review found:**
  - **Clock:** two Starts queued inside one deferred operation could start
    two timers. The deferred task now checks again. Input cannot produce
    that case, since every input flushes first, so the test covers two
    Starts in a row and Stop restoring idle.
  - **Decoding:** each request is numbered, and the test sends two and
    asserts that only the second is shown. That proves the guard only when
    the first result arrives last.
  - **Power mapping:** checked at the halfway position (2.5, not 5).
  - **Menu:** every zoom row reports its choice.

Each demonstration on the page of its family, with a gallery README row and
a headless gallery test where the gallery tests that family:

- `with_grow`, `spacer`, `Justify` and `Align::Stretch` in rows and columns.
- `on_selection_change` on a list, table and tree, showing the origin.
- `Show` and `Each` beside the existing `Dynamic`.
- `Runtime::start_interval` and `Runtime::spawn` with `keep_alive`, for
  example a clock and a background computation with a stale-result guard.
- `on_binding_refusal` on a field whose binding the page can refuse.
- `ParameterMapping::Power` on a numeric control.
- `background_role_signal` and `MenuItem::on_change`.

Then run the native gallery interaction checks on the maintainer's desktop,
and record the run and what the new demonstrations add in
`docs/verification.md`.

### 9. Log-explorer measurements

Status: done, evidence only.
- The split and the 200 000-line run are recorded in
  [verification](../docs/verification.md#log-explorer-plan), measured in a
  temporary worktree that was removed afterwards.
- The planned "before" on 0.3.28 could not be rebuilt: the log-explorer
  branch was squash-merged into 9d23503 and deleted, so no commit holds
  0.3.28.

Evidence only, no version. With temporary instrumentation that is not kept,
as before:

- The release-profile wheel step of the `log_explorer` session split into
  cache hits, shaping and rasterization.
- The before and after comparison on the probe's 200 000 lines
  (`--lines 200000`), with the 50 000-line figures kept beside it. "Before"
  is 0.3.28, rebuilt from history in a temporary worktree.

Record both in `docs/verification.md`'s log-explorer section, and correct
the plan's slice 4 status to match.

### 10. Native checks and closing

Status: the archiving is done; the hand checks wait for the maintainer.
- The four closed plans are in `plans/archive/`, with their links and the
  links to them repaired.
- The app-author plan stays for its slices 8 and 9.
- This plan stays until the hand checks below are recorded; NATIVE-05
  tracks them.

- **Hand checks.** With the maintainer on Fedora 44 GNOME/Wayland, run the
  `diagram_editor`, `log_explorer` and `inventory` windows, `SelectableText`
  on the gallery Text page, cell editing on the gallery table, and Tab
  through a scope. Record what was exercised and by whom in
  `docs/verification.md`, and narrow NATIVE-05 to Windows and macOS.
- **Archive.** Move the four closed plans and this one to `plans/archive/`.
  Update the links to them from `docs/verification.md`, the app-author plan
  and `plans/follow-ups.md`. Also update the links inside every moved plan,
  which go one directory deeper (`../docs/`, `../TODO.md`, the app-author
  plan that stays). Then check every link in `plans/` and `docs/`. The
  app-author plan stays in `plans/` for slices 8 and 9.

## Not in this plan

App-author slices 8 and 9, the follow-ups the register records, TEXT-01's
large-buffer editor, the Windows and macOS evidence of NATIVE-01 to
NATIVE-05, and publication.

## Review record

2026-10-20: the audit described in the background, five Codex runs and five
Claude agents, one per plan, followed by a Codex verification of the
findings that only Claude reported. The reports are not kept in the
repository.

2026-10-20, Codex (`codex exec -s read-only`) on this plan: eight should-fix
findings and no blocker, all applied. Slice 2 must handle the raster limits
as well as the byte limit, and name the `SelectableText`, `Label` help and
inspection contracts it keeps or migrates. Slice 2 must also settle the
diagnostic's schema revision and run the serde and release gates. The
`Signal::update` panic guarantee is narrowed to the final write. The
ColorPicker handler replacement and the table hiding case are already
tested and narrowed. `live_mixer`'s resume handshake and teardown are
added. The CI steps get timeouts, exact configurations and placement.
Archiving repairs outgoing links too.
