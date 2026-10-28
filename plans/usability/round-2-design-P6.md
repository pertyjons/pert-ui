# Round 2 fix plan, part P6 design checkpoint: display controls take values

Part P6 of [the round 2 fix plan](round-2-fix-plan.md#p6-display-controls-take-values-0381)
closes the reopened F16 by decision D1 and R2-26 by decision D13 (both
approved with the recommended option on 2026-10-28). The plan gives P6 a
checkpoint for the list of binding-only presentation inputs and how each is
split, the contract wording and the shape of `ProgressStatus`; this document
settles them, with the exact API, behavior rules, migration manifest, tests,
CHANGELOG, gate and native evidence, before any code. Codex reviews it; the
author approves it together with Codex's review under the maintainer's
delegation of 2026-10-28.

Every reference to library code is to `07eb0a4` (revision 1 read
`ee55226`; the two commits between add only the P4 and P2 designs), whose
library behavior is unchanged since 26e6b5a (0.3.75), the commit the plan
cites. P1 has since been committed (84bb60f, 0.3.76): it changes
documentation, examples and imports only, none of the behavior below, but it
moves lines in some cited files (for example `R/virtual_list.rs`,
`R/table.rs`, `G/controls.md` and the gallery's imports), so the implementer
relocates a citation by its content. Path
prefixes are the plan's: `R/` = `crates/pert-ui-runtime/src/`, `F/` =
`crates/pert-ui/src/`, `G/` = `crates/pert-ui/src/guide/`; `T/` =
`crates/pert-ui/tests/`, `GA/` = `apps/gallery/src/`, `RL` =
`api/pert-ui-runtime/default.txt`, `U` = `~/pert-ui-usability/` (frozen
runs).

P6 lands after P2 to P5 (0.3.77 to 0.3.80). P2's and P4's designs are
approved at revision 3 and committed (`round-2-design-P4.md`, 5942c54;
`round-2-design-P2.md`, 07eb0a4), and so is P5's (`round-2-design-P5.md`,
416dba7), which names none of P6's items.
Where P6 depends on them, section 5 states the dependency and section 7
defines the migration by a search at P6's base commit instead of assuming
their details.

Status: **revision 3 with the author's correction of Codex's second
recheck, approved 2026-10-28** by the author together with Codex's review,
under the maintainer's delegation of 2026-10-28. Revision 1 was
reviewed by Codex (needs revision, seven should-fix points and a nit group);
revision 2 answered each; Codex's recheck of revision 2 left three
should-fix points and a nit, which revision 3 answers
([review record](#review-record)). **P6-D1**
(`ProgressStatus::fraction`) is approved by the author as recommended under
the maintainer's delegation of 2026-10-28. Codex's second recheck left one
narrow test point (the spinner's construction assertion), which the author
corrected in test 14 as Codex proposed; the checkpoint is approved with it.

## What testers built by hand, and what the library says

- **F16 at the head** (`plans/archive/developer-usability.md:897-902`; round
  1 row `:439`): 6f3's controlled `Checkbox` and `InlineText` passed both
  task 2 runs, but all eight task 5 runs, on both commits, kept a binding per
  job for a display control the row data already described:
  - a `Signal<ProgressStatus>` per job beside the republished rows
    (U `r2-5-head-claude-1/work/src/lib.rs:58`, `:120`; triage
    `runs/triage-r2/group-3.md:91`, `:98`; `group-4.md:38`, `:150`, `:154`);
  - a `sibling` per cell build, following guide/state.md's "Advanced" row:
    `ProgressBar::new(self.store.sibling(ProgressStatus::of(j.processed, j.total)))`
    (U `r2-5-head-codex-1/work/src/lib.rs:70`; `r2-5-r1-codex-2/work/src/main.rs:246`;
    `group-3.md:112`, `group-4.md:156`). The verifiers kept the
    classification (`runs/triage-r2/codex-review.md:104`,
    `claude-check-of-codex-runs.md:183-186`).
- **The library's own consumers do the same.** The gallery wraps every fixed
  value in a signal: `let fixed = |status: FieldStatus| ui.signal(status).read_only();`
  (`GA/pages/text.rs:665`, used at `:714-766`) and
  `ProgressBar::new(ui.signal(value).read_only())` with
  `field.status(ui.signal(FieldStatus::error(..)).read_only())`
  (`GA/pages/controls.rs:996-1012`); the facade's prelude fixture writes
  `ProgressBar::new(ui.signal(ProgressStatus::of(1, 2)))`
  (`T/foundation/vocabulary.rs:491`).
- **R2-26** (`plans/archive/developer-usability.md:944`): 5-head-claude-1
  showed "15%" beside a full red bar, since "there is no status that has both
  a fraction and an outcome" (U `r2-5-head-claude-1/work/FRICTION.md:72-74`,
  `:92`); 5-r1-claude-2 the same (`group-4.md:47`, `:210`). Documented
  behavior, an optional capability (`group-3.md:95`, `:307`).

## 1. Today's behavior, from the code

- **`ProgressBar`** holds `source: ReadSignal<ProgressStatus>` and the
  status last read (`R/progress.rs:148-154`); `new` takes only
  `impl Into<ReadSignal<ProgressStatus>>` (`:157-166`). `reconcile` keeps
  everything for the same source and otherwise adopts the new one with
  `Invalidation::Paint` (`:196-203`); `update` reads with `read_paint` and
  requests `animate` while indeterminate (`:204-214`). The fill is
  `fill()`'s `(start, width, role)` (`:168-193`); terminal statuses fill the
  whole width (`:189-191`), a partial fill leaves the gap of half the bar's
  height (`:252-262`), and a zero-width fill draws the track alone (`:243`).
  Inspection: property `status` (the status name, `:81-90`), and while
  determinate the `fraction` property and `SemanticValue::Number` in 0..=1;
  description the status name, `invalid` for `Failed` (`:273-297`).
- **`ProgressStatus`** is `Clone, Copy, Debug, Default, PartialEq`,
  `#[non_exhaustive]` (`R/progress.rs:20-39`), no serde derive; `Failed` and
  `Cancelled` are unit variants documented as full tracks (`:33-38`).
  `is_finished` matches them (`:78-80`).
- **`Spinner`** holds `source: ReadSignal<bool>` (`R/progress.rs:353-359`);
  `new` takes only a binding (`:361-371`); `reconcile` and `update` mirror the
  bar's (`:374-394`); inspection reports `busy` (`:421-431`).
- **`InlineReadout::new`** takes `impl Into<ReadSignal<String>>` and builds a
  reader closure that `read_paint`s it (`R/inline_controls.rs:613-617`);
  `formatted` does the same for a typed value through a `NumberFormat`
  (`:621-634`). `reconcile` always adopts the reader (`:678-705`); `update`
  shows the read text, requesting layout when it changes without a hint, and
  validates it through the inline field (`:716-735`,
  `R/text_fields/inline.rs:163-181`).
- **`Field::status`** takes `impl Into<ReadSignal<FieldStatus>>` and clears
  the form target (`R/field_status.rs:537-543`); `Field::form` takes the
  handle's binding and target (`:544-550`). `reconcile` compares sources with
  `same_optional` and falls back to `Normal` without one (`:603-630`);
  `update` reads the status with `read`, since the message is measured
  (`:637-639`). `Field` has no `Permission`-typed builder (`:480-550`).
  `Property::status` and `Property::form` delegate to its `Field`
  (`R/property_grid.rs:66-86`).
- **`TextSource`** converts from fixed text (`&str`, `String`, `&String`,
  `RichText`, `StringRole`) and from `Signal`/`ReadSignal` of `String` or
  `RichText`, by value and by reference (`R/text_source.rs:165-236`): every
  argument `InlineReadout::new` accepts today.
- **Precedents of the split:** `ColorSwatch::new(Color)` and
  `ColorSwatch::bound(binding)` (`R/color_swatch.rs:113-135`, reconcile
  `:203-235`), `Image::new(RasterImage)` and `Image::bound(binding)`
  (`R/image.rs:86-101`), `Container::background_role` and
  `background_role_signal`, each replacing the other (`R/containers.rs:105-157`).
- **Collections give `build` and `cell` no state owner**
  (`R/virtual_list.rs:58-94`, `R/table.rs:259-271`) and describe every
  mounted row again on a source replacement and on a scroll that mounts
  another row; the per-row `sibling` route is taught in `Signal::sibling`'s
  rustdoc (`R/signal.rs:354-369`), guide/state.md's table (`G/state.md:20`)
  and the editable-list recipe (`G/layout.md:528-541`).
- **The prelude** holds `ProgressBar`, `ProgressStatus`, `Spinner`,
  `InlineReadout`, `Field`, `FieldStatus`, `Property`, `FormFields` and
  `FieldHandle`, but not `NormalizedValue` (in `parameter`) or `TextSource`
  (at the root); the prelude pages say "a fraction is a `NormalizedValue`
  from `parameter`" (`R/lib.rs:383-384`, `F/lib.rs:109-110`), L2's
  "Not required" exception (`plans/usability/layering-design-L2.md:156-159`).

## 2. Binding-only presentation inputs, and the decision for each

Every public constructor or builder in RL whose argument is
`impl Into<ReadSignal<T>>` or `ReadSignal<T>` (the full search: `pub fn` lines
of RL taking `ReadSignal`, by value, by reference or through `Into`;
writable `Signal` inputs are editing controls, outside D1). The facade
listings add none (`api/pert-ui/*.txt` re-export the runtime by glob).

| Input | Kind | P6 |
|---|---|---|
| `ProgressBar::new` (`R/progress.rs:158`) | display control | **split:** `new(ProgressStatus)`, `bound(binding)` |
| `Spinner::new` (`R/progress.rs:363`) | display control | **split:** `new(bool)`, `bound(binding)` |
| `InlineReadout::new` (`R/inline_controls.rs:614`) | display control, text | **`new(impl Into<TextSource>)`**, covering fixed and bound |
| `InlineReadout::formatted` (`:621`) | display control, number | stays bound; a fixed number is formatted by the application (D1) |
| `Field::status` (`R/field_status.rs:538`) | presentation builder | **split:** `status(FieldStatus)`, `status_signal(binding)` |
| `Property::status` (`R/property_grid.rs:67`) | presentation builder | **split**, as `Field` |
| `ColorSwatch::bound`, `Image::bound` | display control | already split; unchanged |
| `Container::background_role_signal` | presentation builder | already split (`background_role`); unchanged |
| `View::visible_signal`, `ToolGroup::visible_signal`, `Banner::visible_signal` | presence | no fixed form: a fixed presence is the element's presence in the description; the binding exists to hide and show without a description while keeping retained state |
| `LevelMeter::running_signal`, `Trace::running_signal` | work gate | no fixed form: without the builder the widget always reads, and a fixed false would never read |
| `MenuItem::checked_signal`, `selected_signal` | an action row's mark | no fixed form now: a menu row is an action, not a display control; `toggle` and `radio` own a signal mark and these builders reflect state the application or the theme owns. No run described a menu from records, and a `checked(bool)` would be a second way with nothing to retire (AGENTS.md, "Component API conventions") |
| `Dynamic::new`, `Each::new`, `Show::new` | structure | not presentation inputs: a fixed source is the content itself |
| `VirtualList::new`, `estimated`, `ListFilter::new` | collection source | collection contracts, outside D1 (contracts.md, "Choice sources" excludes collections, `docs/contracts.md:75-79`) |
| `VirtualList::keep_mounted` | retention | not presentation: it follows an edit session's lifetime |
| `PickPending::new` | asynchronous result | not presentation: a pending result is a binding by nature |
| `Runtime::command_enabled_signal`, every `Permission`-typed builder | authority | `Permission` rules, unchanged (D1) |
| `LevelMeter::new`, `Trace::new` (`SeriesSource`) | live data | its own ownership path, unchanged |

## 3. The rules

### 3.1 The display-control rule

1. A **display control** presents one application value and writes nothing.
   It chooses the value's source at construction, as a choice control chooses
   its options': `new` takes the value, `bound` follows a `ReadSignal`. The
   two are exclusive constructor paths; no builder adds the other source.
2. **A fixed value applies when the description is adopted** (construction
   for a mount, `reconcile` for a same-key description) and changes only
   with a new description. This is `TextSource`'s rule for fixed text
   (`R/text_source.rs:28-30`), which P6 keeps without exception, including
   for the readout's value text (3.4).
3. **A binding is read from the next update**; a same-key description with
   the same binding (`ReadSignal::same_source`) keeps what was read; another
   binding is read at the next update and the old one no longer affects the
   control. This is today's behavior of every `new`.
4. **Reconcile compares sources, not shown values.** Fixed to fixed with an
   equal value is the only description that changes nothing. Every other
   difference is adopted even when the shown value stays equal: a different
   fixed value, a different binding, a change of kind (fixed to bound or
   bound to fixed) and, for `Field`, a different form target. After bound
   to fixed, writes to the old binding no longer update the control.
5. **Presentation builders** take the fixed value under their own name and
   the binding under the `_signal` name; the two replace each other and any
   other source of the same property (for `Field`, `form`), as ordinary
   property builders do.
6. **Validity:** `ProgressStatus`, `bool` and `FieldStatus` have no invalid
   values. Fixed readout text is validated at adoption with the check bound
   text gets in the update (3.4.3).
7. **No privileged path:** a custom display widget does the same with public
   `ReadSignal` and `UpdateContext::read`/`read_paint`. The implementation
   shares one crate-private `enum` (fixed value or binding, with `same_as`
   and the two reads) between `ProgressBar`, `Spinner` and `Field` rather
   than copying it.

### 3.2 `ProgressBar`

1. `new(status)` shows `status`; `bound(status)` follows the binding, as
   `new` does today. Everything else (geometry, colors, gap, reduced motion,
   scheduling) is unchanged and independent of the source.
2. **Fixed indeterminate animates** exactly as bound indeterminate: each
   update of an indeterminate bar requests `animate`, so a row described as
   `ProgressBar::new(ProgressStatus::Indeterminate)` sweeps, and stops when a
   new description gives another status or the bar is hidden or unmounted.
3. **Reconcile** (rule 3.1.4): fixed `a` to fixed `a` returns `None`; fixed
   `a` to fixed `b` sets the shown status to `b`; bound to fixed sets it to
   the fixed value and drops the binding, also when the value equals what was
   read; fixed to bound and bound to another binding keep the shown status
   until the next update reads the binding. Each adopted change returns
   `Invalidation::Paint`, which runs the bar's update before paint as today
   (`R/runtime.rs:1080-1102`); that update requests `animate` for an
   indeterminate status (3.2.2).

### 3.3 `Spinner`

1. `new(busy)` spins while `busy` is true and rests invisibly while false;
   `bound(busy)` follows the binding, as `new` does today. A described `true`
   steps every 100 ms exactly as a bound one, aligned to the runtime clock,
   so redescribing a row keeps the lit dot's step.
2. Reconcile as 3.2.3, with `busy` for the status; the update that follows
   schedules the 100 ms step for a busy spinner whatever its source.

### 3.4 `InlineReadout::new`

1. `new(text: impl Into<TextSource>)`: fixed plain or rich text, a
   `StringRole`, or a binding of `String` or `RichText`, by every conversion
   `TextSource` has. Every argument the old signature accepted converts
   (section 1).
2. **Plain text:** the readout shows the source's plain text
   (`RichText::plain_text`) in its value style, as it shows a `String` today;
   spans, colors and embedded icons of rich content are not drawn. Text mode
   selects and copies that plain text.
3. **Fixed text applies at adoption,** as `TextSource` promises. Today every
   readout is a reader closure run in the update (`R/inline_controls.rs:600-617`,
   `:716-734`). P6 keeps the reader for `formatted` and gives `new` its
   `TextSource`: the readout holds either. For fixed content, construction
   and `reconcile` take its plain text and apply it through the inline
   field's display check (`InlineField::display_value`,
   `R/text_fields/inline.rs:163-181`), which is pure (`check_text`,
   `R/text_fields.rs:3686-3697`, and field state) and so allowed in an
   infallible `reconcile` (`R/runtime.rs:1084-1093`):
   - **valid:** the shown text, the `text` and `display_value` inspection
     properties and the semantic value change at adoption; `reconcile`
     returns `Invalidation::Layout` when the text changed and no width hint
     is set, otherwise `Invalidation::Paint`. Open text mode on a changed
     text ends in the update that follows, which `reconcile` marks: ending
     it ends the embedded editor's draft slot (`InlineField::stop_text`,
     `R/text_fields/inline.rs:241-250`), an effect `reconcile` does not
     perform;
   - **invalid** (over the display limit or holding a control character):
     nothing shown changes, so the last valid text stays (empty on a first
     mount), and the readout's next update returns the `TextEditError` as
     today's bound refusal does, in every flush until a valid description
     replaces it. This is AGENTS.md's rule for a value a description carries
     from application data: refused at adoption with the last valid
     presentation kept.
   - an equal fixed text returns `None`.
4. **A role or a binding is read in the update** with
   `TextSource::read_paint`, and **validated before anything is committed**,
   with the acceptance order of fixed text. Today's update commits first:
   it stores a new unit and requests layout (`R/inline_controls.rs:719-722`),
   ends text mode and stores the new text (`:723-731`), and only then
   validates through `display_value` (`:732`), so a refused text already
   shows in the `text` property and the semantic value (`:879-889`) while
   the field keeps its old display, against the last-valid-presentation rule
   (`docs/contracts.md:224-228`). P6 validates the candidate text and unit
   with the same check first; on a refusal the update returns the
   `TextEditError` and changes nothing: not the unit, the text, inspection,
   the semantic value, the layout request or text mode. On success it
   commits as today: layout is requested when the text changes without a
   hint, and text mode ends only if the shown text changes
   (`R/inline_controls.rs:584-586`). This also applies to `formatted`, which
   shares the update; it is an in-scope same-kind fix in touched code, named
   in the Progress entry. A same-key description with the same binding or role keeps
   the text read (`TextSource::same_as`); another binding replaces it, and
   the old one no longer drives the readout. A role follows a newly installed
   string table (`R/text_source.rs:17-20`).
5. `formatted` keeps its signature and reader: adopted at every reconcile
   and run in the update, with rule 4's ordering.

### 3.5 `Field` and `Property`

1. `Field::status(status: FieldStatus)` shows `status`; `status_signal(status)`
   follows a binding, as `status` does today; `form(handle)` follows the
   handle's binding and target, unchanged. **Each of the three replaces the
   other two**; `status` and `status_signal` clear the form target, as
   `status` does today (`R/field_status.rs:541`), so
   `FormFields::focus_first_error` no longer reaches that field.
2. Without any of them the status is `FieldStatus::Normal`, as today.
3. Reconcile (rule 3.1.4): only fixed to fixed with an equal status changes
   nothing. A different fixed status, a different binding, a change of kind
   (bound to fixed with an equal value included) and a different form
   target are adopted with `Invalidation::Layout` (the message is measured),
   as source and target identity are compared independently of the shown
   status today (`R/field_status.rs:613-627`); a replaced target's control
   cell is cleared, as today (`:619-621`), so `focus_first_error` no longer
   reaches the control.
4. `Property::status(FieldStatus)` and `Property::status_signal(binding)`
   delegate to the row's `Field`, with the same replacement rule.
5. Presentation, semantics and inspection of a fixed status are those of the
   same status read from a binding.

### 3.6 `ProgressStatus`

1. **Shape:** `Failed(Option<NormalizedValue>)` and
   `Cancelled(Option<NormalizedValue>)`. `None` is today's full track in the
   Error or Muted role, for a task whose reached fraction is unknown;
   `Some(reached)` fills the track to `reached` in that role. The other
   variants, the derives, `Default` and `#[non_exhaustive]` are unchanged;
   the enum stays `Copy`.
2. **`fraction(self) -> Option<NormalizedValue>`** ([P6-D1](#decisions)):
   the fraction the status carries: `Determinate(v)` gives `Some(v)`,
   `Failed(r)` and `Cancelled(r)` give `r`, every other status `None`. A
   running task's last status therefore ends as
   `ProgressStatus::Failed(last.fraction())`: the reached fraction after a
   determinate run, a full track after an indeterminate one.
3. **Paint:** `fill()` for `Failed(Some(r))` is `(0, width × r, Error)`, for
   `Cancelled(Some(r))` `(0, width × r, Muted)`; the gap beside a partial fill
   applies as for `Determinate`. A zero fraction leaves the track empty, as
   `Determinate` at zero does; the failure stays in inspection and in the
   application's message. `Failed(Some(MAX))` draws as `Failed(None)`.
4. **Inspection:** a status that carries a fraction (`Determinate`, and
   `Failed` or `Cancelled` with `Some`) reports it as the `fraction` property
   and as `SemanticValue::Number { value, minimum: Some(0.0), maximum:
   Some(1.0) }`; the description stays the status name (`failed`,
   `cancelled`), and `invalid` is set for every `Failed(_)` and never for
   `Cancelled(_)`. `None` reports no value and no `fraction` property, as
   today, so a status changing from `Some` to `None` clears both. The
   description is a status name like the `status` property, which tests
   match (`T/controls/control_reconcile.rs:288-291`); the number belongs in
   the value, where `Determinate` reports it (`R/progress.rs:283-291`) and
   where a tester read it ("the progress bar reports its fraction and
   `invalid`", U `r2-5-head-claude-1/work/FRICTION.md:88`).
5. `is_finished` and `name` match `Failed(_)` and `Cancelled(_)`; `of` is
   unchanged.

## 4. Public API

All in `R/progress.rs`, `R/inline_controls.rs`, `R/field_status.rs` and
`R/property_grid.rs`, at the runtime root and its prelude, re-exported by the
facade's globs; no module membership changes.

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum ProgressStatus {
    #[default]
    Idle,
    Indeterminate,
    Determinate(NormalizedValue),
    Succeeded,
    /// Ended in failure. [3.6.1: `None` a full track in the Error role, for
    /// a fraction the application does not know; `Some(reached)` the track
    /// filled to `reached`; a zero fraction leaves it empty. What went wrong
    /// is the application's message, through a `Field` or `Banner`.]
    Failed(Option<NormalizedValue>),
    /// Stopped before finishing. [The same in the Muted role.]
    Cancelled(Option<NormalizedValue>),
}
impl ProgressStatus {
    /// The fraction this status carries. [3.6.2.]
    /// Example: `of(3, 8)` ends as `Failed(last.fraction())`, equal to
    /// `Failed(Some(three_eighths))`; `Indeterminate.fraction()` is `None`,
    /// so a failure while indeterminate is `Failed(None)`.
    #[must_use]
    pub const fn fraction(self) -> Option<NormalizedValue>;
    // `of`, `is_running`, `is_finished` unchanged in signature; the enum's
    // page gains an example matching `Failed(reached)`.
}

impl ProgressBar {
    /// Shows `status`, as described: a row of a collection describes its
    /// record's status this way, and a new description shows a new one.
    /// [3.2; for a status the application writes over time, `Self::bound`.]
    /// Example: mounts `ProgressBar::new(ProgressStatus::of(3, 4))`,
    /// describes it again with `ProgressStatus::Failed(status.fraction())`
    /// through `reconcile_children`, and after the flush reads the bar's
    /// `status` property `failed` and `fraction` 0.75.
    pub fn new(status: ProgressStatus) -> Self;
    /// Follows an application-owned status binding. [Today's `new` text.]
    /// Example: today's (indeterminate animates, a known fraction schedules
    /// nothing), with `bound`.
    pub fn bound(status: impl Into<ReadSignal<ProgressStatus>>) -> Self;
}

impl Spinner {
    /// Spins while `busy` is true and rests invisibly while false, as
    /// described. [3.3; `Self::bound` for a binding.]
    /// Example: `Spinner::new(true)` schedules its step
    /// (`next_deadline() == Some(100 ms)`); described again as
    /// `Spinner::new(false)` it schedules nothing.
    pub fn new(busy: bool) -> Self;
    /// Follows an application-owned binding. [Today's `new` text.]
    /// Example: today's, with `bound`.
    pub fn bound(busy: impl Into<ReadSignal<bool>>) -> Self;
}

impl InlineReadout {
    /// Shows text verbatim: fixed text, a string-table role or a binding, as
    /// a `TextSource`. [3.4: plain text only; fixed text applies at
    /// adoption and is refused there when it cannot be shown; a binding or
    /// role is read in the update.]
    /// Example: `InlineReadout::new("Ready")` reports the text right after
    /// `mount_tree`, before any flush; a second readout bound to a
    /// `Signal<String>` shows a write after the flush. The type example
    /// keeps its bound position readout.
    pub fn new(text: impl Into<TextSource>) -> Self;
    // `formatted` unchanged.
}

impl Field {
    /// Shows `status`, as described, replacing an earlier status, status
    /// binding or form. [3.5; the form target is cleared.]
    /// Example: `Field::new(..).label("Name").status(FieldStatus::error("Required"))`
    /// reports the `message` "Required" and marks the control invalid.
    pub fn status(self, status: FieldStatus) -> Self;
    /// Follows an application-owned status binding, replacing an earlier
    /// status, status binding or form. [Today's `status` text.]
    /// Example: a field described with `.status(FieldStatus::warning(..))`
    /// is described again with `.status_signal(checked.read_only())`; it
    /// shows the binding's value after the flush, and a later
    /// `checked.set(FieldStatus::error(..))` shows the error after the next.
    pub fn status_signal(self, status: impl Into<ReadSignal<FieldStatus>>) -> Self;
    /// [Unchanged signature; the text says it replaces an earlier status or
    /// status binding.]
    pub fn form(self, handle: &FieldHandle) -> Self;
}

impl Property {
    /// Shows `status`, as [`Field::status`].
    /// Example: a `PropertyGrid` row with `.status(FieldStatus::warning(..))`
    /// whose editor is described by the warning.
    pub fn status(self, status: FieldStatus) -> Self;
    /// Follows a status binding, as [`Field::status_signal`].
    /// Example: a row described again from `.status(..)` to
    /// `.status_signal(..)`, then a write to the binding shown after the
    /// flush.
    pub fn status_signal(self, status: impl Into<ReadSignal<FieldStatus>>) -> Self;
}
```

- **Examples.** Every new or changed public item has the runnable example
  named above (the plan's rule, `round-2-fix-plan.md:1292-1295`); the
  variants `Failed` and `Cancelled` are shown by `fraction`'s and the enum
  page's examples. All are doctests importing the runtime as the existing
  examples do.
- **Names.** `new`/`bound` are D1's and `ColorSwatch`'s; `status_signal`
  follows `visible_signal`, `checked_signal` and `background_role_signal`;
  `fraction` reads as what it returns and collides with nothing in RL.
- **Anti-regrowth** (AGENTS.md, "Component API conventions"): the fixed
  forms retire the per-row `sibling` and the "signal for a fixed value"
  pattern for display controls (the gallery's `fixed` helpers); `bound` and
  `status_signal` are today's behavior under the binding's name, not a second
  way; `fraction` replaces the `match` over three variants a caller would
  otherwise write.

## 5. Module membership, prelude closure and dependencies

- **No membership change.** Every changed item is already in the runtime and
  facade preludes (section 1).
- **Prelude closure** (L2's D1, re-applied as the plan requires): `bound`
  and `status_signal` take `ReadSignal` (prelude) of prelude types;
  `Field::status` takes `FieldStatus` (prelude); `InlineReadout::new` takes
  `impl Into<TextSource>`, L2's conversion exception
  (`layering-design-L2.md:145-147`); the new payload and `fraction`'s result
  are `NormalizedValue`, which a prelude-only program never writes: it
  builds `Failed(None)`, `Failed(last.fraction())` or matches
  `Failed(reached)` and reads `reached.map(|r| r.get())` without naming the
  type. L2's "Not required" exception (`:156-160`) is extended in those words, and the
  prelude pages' bullet with it (section 6); P6-D1 is the trade-off. The
  prelude-only fixture of section 8 writes each new form.
- **P2's `testing` rule:** none of these items is reachable from a
  `testing` seed (P2 design, sections 2.2-2.3: the driver, queries,
  captures, input; `ProgressStatus` and the controls are prelude members,
  where the traversal stops), so P2's listing-driven test passes unchanged;
  the Progress entry records its run.
- **Dependency on P4:** P4's design adds `Driver::quiet_for` with a rustdoc
  example and tests that mount `Spinner::new(binding)` (P4 design, section 3
  and tests 4 and 7). P6 migrates them by the rule of section 7: to
  `Spinner::bound(binding)` where the test writes the binding (P4's example
  and test 4 set it false), to `Spinner::new(true)` where it only needs a
  busy spinner. P4's design is approved and committed, but its code lands in
  P4's own commit; the final call sites are whatever that commit has.
- **Dependency on P5:** none in substance. P5's design (approved,
  416dba7; it names none of P6's items) changes `Each::new`'s `build`
  and adds `Driver::address_children`; any `ProgressBar`, `Spinner`,
  `InlineReadout` or `Field::status` call it adds is found by section 7's
  search.
- **Later parts:** P7's bar-plus-percent cell and P8's table chapter use the
  fixed bar; no P6 text mentions row shrinking, cell addresses or later
  items.

## 6. Documentation

Text describes P6's commit.

- **docs/contracts.md, "Ownership paths"** (`docs/contracts.md:664-673`):
  after the table, this paragraph (Choice sources stays as it is):

  > **Display controls.** A control that presents one application value and
  > writes nothing chooses the value's source at construction, as a choice
  > control chooses its options' ([choice sources](#choice-sources)): `new`
  > takes the value, which applies when the description is adopted and
  > changes only with a new description, so a collection row describes its
  > record's value directly; `bound` follows a `ReadSignal` from the
  > control's next update, and a same-key description with the same binding
  > keeps what was read. `ProgressBar`, `Spinner`, `ColorSwatch` and `Image`
  > have both. A control that presents text takes a `TextSource`, which
  > covers fixed and bound text (`Label::new`, `InlineReadout::new`);
  > `InlineReadout::formatted` formats a bound number. A presentation builder
  > takes the fixed value under its own name and a binding under its
  > `_signal` name (`Field::status` and `Field::status_signal`,
  > `Property::status` and `Property::status_signal`,
  > `Container::background_role` and `Container::background_role_signal`),
  > each replacing the other. `Permission`-typed builders follow the
  > permission rules instead; bindings that switch presence or work
  > (`View::visible_signal`, `ToolGroup::visible_signal`,
  > `Banner::visible_signal`, `LevelMeter::running_signal`,
  > `Trace::running_signal`) and an action row's mark
  > (`MenuItem::checked_signal`, `selected_signal`) have no fixed form.

- **`ListItems::build`** (`R/virtual_list.rs:66-93`): "in one of three ways"
  becomes four, with a first bullet **Described**: a display control takes
  the record's value itself (`ProgressBar::new(record.status)`,
  `Spinner::new`, `ColorSwatch::new`, `Image::new`, text through
  `Label::new` or `InlineReadout::new`, a status through `Field::status`); it
  binds and writes nothing, so the replaced source's description shows the
  new value after the flush. "Per row" and "Retained" say "an editing control
  without a controlled form".
- **`TableItems::cell`** (`R/table.rs:262-270`): a display control in a cell
  takes the record's value, such as `ProgressBar::new(record.status)`; a
  checkbox, toggle or editable label takes it through its controlled form;
  another editing control binds per row or from a retained cell component.
- **`Signal::sibling`** (`R/signal.rs:354-369`): the per-row paragraph
  narrows: a display control in a row takes the record's value
  (`ProgressBar::new`), a controlled control its value and callback; for an
  **editing** control without a controlled form, a per-row binding made here
  is the advanced form, with its cost. The first paragraph (component-owned
  state, `:346-352`) is unchanged.
- **guide/state.md, "Where a control's value comes from"** (`G/state.md:15-21`):
  a row after `controlled`: "A value at construction: a display control's
  `new`, such as [`ProgressBar::new`], or a builder such as
  [`Field::status`] | the control shows the application's value and writes
  nothing, as a list row's progress does; `bound` ([`ProgressBar::bound`]) or
  the `_signal` builder follows a binding instead". The first row says "an
  editing control's `new`". The `sibling` row says "a list row's editing
  control without a controlled form and without a draft".
- **guide/layout.md, "An editable list of records"** (`G/layout.md:494-541`
  and its example): `Task` gains `progress: ProgressStatus`, each row a
  `ProgressBar::new(task.progress)` keyed `progress` after the title; the
  paragraph at `:528-541` says the bar is a display control that takes the
  record's status with `new`, needing neither a binding nor a callback, and
  "A control without a controlled form" becomes "An editing control without
  a controlled form". The example's test writes one record's progress
  through `edit` and asserts, after the flush, that the row's bar reports the
  new status (its semantics description); the bar keeps its natural width
  and the example's viewport keeps the row inside the list, which the
  implementer checks with `validation_report()` (no `EscapesParent`),
  widening the viewport if needed.
- **guide/controls.md, "Progress and activity"** (`G/controls.md:490-495`):
  `ProgressBar` presents a `ProgressStatus` "it is described with, or a
  binding the application writes"; how a task ended "and how far it got";
  `Spinner` likewise.
- **`ProgressStatus`'s type page** (`R/progress.rs:16-19`): "every
  transition is a write to the application's binding" becomes "every
  transition is the application's: a new description of a control that
  takes the status, or a write to a binding it follows". "The library never
  advances, times or completes a task" stays.
- **Rustdoc of the changed items:** section 4. Also `ProgressBar`'s type page
  (`R/progress.rs:98-129`): "a [`ProgressStatus`], described or bound", the
  terminal fill rule, the fraction in inspection, and "Re-describing the bar
  … with another status or status binding"; `Spinner`'s (`:306-334`)
  likewise; `InlineReadout`'s (`R/inline_controls.rs:544-546`) "fixed or
  bound text, or a formatted typed value", and its redescription paragraph
  (`:584-586`) "a fixed text applies when the description is adopted; a
  binding, role or format is read again in the next update"; `Field`'s
  (`R/field_status.rs:440-461`)
  "The status is described or read from the application's binding", and
  "Show a status with `Field::status`, bind it with `Field::status_signal`,
  or with `Field::form`…"; `Property`'s (`R/property_grid.rs:9-10`) "an
  optional status"; `TextSource`'s list of takers (`R/text_source.rs:9-16`)
  adds `InlineReadout`'s text.
- **Prelude pages** (`R/lib.rs:383-384`, `F/lib.rs:109-110`): "[`ProgressStatus::of`]
  builds progress from counts and [`ProgressStatus::fraction`] carries a
  running task's fraction into `Failed` or `Cancelled`; a fraction written as
  a number is a [`NormalizedValue`] from [`parameter`]."
- **docs/manual.md:** `:181` the `Field` row names `.status(FieldStatus)`,
  `.status_signal(read_signal)` or `.form(&handle)`; `:389`
  `InlineReadout::new(text)` with fixed or bound text, or
  `InlineReadout::formatted(read_signal, format)`; `:486-487`
  `ProgressBar::new(status)` or `ProgressBar::bound(read_signal)`, "and how a
  task ended, with the fraction it reached", `Spinner::new(busy)` or
  `Spinner::bound(read_signal)`, in the form of the `ColorSwatch` row
  (`:335`).
- **plans/follow-ups.md, "Controlled forms of more controls"**
  (`plans/follow-ups.md:60-67`): round 2's evidence, as the plan's scope says:
  F16 recurred only for display controls, which P6 gave described values; no
  round 2 run needed an editing control without a controlled form per row.
- **Same-kind fix in touched text:** `Field`'s type paragraph at
  `R/field_status.rs:437-441` runs three sentences into two overlong lines;
  rewrapped while its status sentence changes.
- **Unchanged:** `docs/manual.md` and the runtime README's other passages;
  the runtime README (`crates/pert-ui-runtime/README.md:1289`, `:1337`) and
  guide/controls.md's toolbar example (`G/controls.md:596`, `:648`) describe
  and call `InlineReadout::new(position.read_only())`, which stays correct.

## 7. Migration manifest

**Rule:** at P6's base commit (P5's), `git grep` for `ProgressBar::new`,
`Spinner::new`, `InlineReadout::new`, `.status(` on a `Field` or `Property`,
`ProgressStatus::Failed` and `ProgressStatus::Cancelled` over every
maintained consumer; each hit migrates as below. A call whose binding is
created only to hold a value that is never written afterwards becomes the
fixed form (the F16 pattern); every other call keeps its binding under
`bound`/`status_signal`. `InlineReadout::new` calls compile unchanged and
are not edited. The hits at HEAD:

| Consumer | Change |
|---|---|
| Runtime rustdoc: `R/progress.rs:131-146`, `:336-351` | examples split into `new` and `bound` (section 4) |
| `R/property_grid.rs:66-71` | delegation (3.5.4) |
| `R/inline_controls.rs:577`, `R/parameter_binding.rs:613`, `R/text_fields/inline.rs:876` | unchanged (`InlineReadout::new` with a `ReadSignal<String>`) |
| `crates/pert-ui/examples/log_explorer/app.rs:443`, `:882`; `session.rs:118` | `Failed(self.progress.get()?.fraction())`; `ProgressBar::bound(explorer.progress.read_only())` (the explorer writes its status over time); `matches!(…, ProgressStatus::Failed(_))` |
| `crates/pert-ui/examples/inventory/app.rs:719` | unchanged (`InlineReadout::new`) |
| Gallery `GA/pages/controls.rs:996-1012` | the `fixed` helper builds `ProgressBar::new(value)` and `field.status(FieldStatus::error("The disk is full"))`, testing `matches!(value, ProgressStatus::Failed(_))` |
| Gallery `GA/pages/controls.rs:1028`, `:1035`, `:1090` | `ProgressBar::bound`, `Spinner::bound` (the page writes them) |
| Gallery `GA/pages/controls.rs:1062-1067` | Fail and Cancel end the task at its reached fraction, `Failed(current.fraction())` (the action reads the current status), so Step then Fail shows a partial red fill and Start then Fail a full one |
| Gallery `GA/pages/controls.rs:1099-1133` | `Failed(None)` and `Cancelled(None)`, plus two states "Failed at three of eight" and "Cancelled at three of eight" (`Failed(ProgressStatus::of(3, 8).fraction())`), the native evidence of section 9 |
| Gallery `GA/pages/text.rs:665`, `:714-766` | the `fixed` helper removed; `.status(FieldStatus::…)` |
| Gallery `GA/pages/text.rs:337`, `:695` | `.status_signal(…)` |
| Gallery `GA/pages/controls.rs:254`, `:291`; `GA/tests/controls.rs:230-281` | unchanged; the progress test keeps passing (section 8, contract tests) |
| Facade tests `T/controls/control_reconcile.rs:276-312`, `T/foundation/redescription.rs:473`, `T/presentation/progress.rs:18`, `:171`, `:214` | `bound` (they write or rebind the signal; `:214` is the shared `spinner` helper, whose callers write it at `:277`, `:348`) |
| `T/presentation/progress.rs:363` | `Spinner::new(false)` (the motion test never writes `busy`) |
| `T/presentation/progress.rs:435` | `ProgressBar::new(status)` (never written) |
| `T/foundation/vocabulary.rs:491` | `ProgressBar::new(ProgressStatus::of(1, 2))` |
| `T/presentation/progress.rs:72-73`, `:196`, `:428-429`, `:452-456`; `T/controls/control_reconcile.rs:276` | `Failed(None)`, `Cancelled(None)`, patterns `Failed(_)`/`Cancelled(_)` |
| `T/controls/field_status.rs:57`, `:144`, `:373`; `T/controls/forms.rs:216`; `T/foundation/semantics.rs:158`, `:234`; `T/collections/property_grid.rs:646` | `.status_signal(…)` (each test writes its status signal: `field_status.rs:115`, `:159`, `:393`, `forms.rs:225`, `semantics.rs:206`, `:250`, `property_grid.rs:659`) |
| `T/controls/field_status.rs:499` | `.status_signal(…)`: the rebinding test (`:488-536`) is the binding compatibility coverage; it writes the replaced signal |
| `T/controls/field_status.rs:172`, `:182` and `:568`, `:575` | `.status(FieldStatus::error(…))`: the hiding test (`:169-211`) and the background test (`:558-617`) never write their signals |
| Facade tests calling `InlineReadout::new` (`T/audit/audit_presentation.rs:261`, `T/audit/sweep_input.rs:638`, `T/controls/activation.rs:263`, `T/editing/begin_edit.rs:1151`, `T/editing/inline_controls.rs:457`, `:574`, `T/editing/inline_draft_editing.rs:97`, `T/foundation/key_repaint.rs:494`, `T/presentation/title_sources_fields.rs:56`, `T/controls/control_reconcile.rs:327`) | unchanged |
| P2 to P5 additions (P4's `quiet_for` example and tests in `crates/pert-ui-runtime/src/testing.rs` and `crates/pert-ui-runtime/tests/testing.rs`; whatever P5 adds) | by the rule above |
| Guide, contracts, manual, prelude pages, follow-ups | section 6 |
| Runtime README, `G/controls.md:648`, `G/authoritative-values.md:201` | unchanged (compiles; text still true) |
| Native checks (`apps/native-checks`), platform code, `crates/pert-ui-wgpu`, `crates/pert-ui-cpu` | none use these items (`git grep`) |
| Diagnostics fixtures (`tests/diagnostics/ui/`) | none use these items; snapshots run in the gate |
| `plans/usability/task4-app` | uses none of them (`git grep` finds no `ProgressBar`, `Spinner`, `InlineReadout`, `FieldStatus` or `.status(`): no fixture check |
| Frozen: submissions and probes under `plans/usability/runs/` and U (for example U `r2-5-head-codex-1/work/src/lib.rs:70`), `plans/usability/api-audit/inputs/` and `plans/archive/` | not migrated |

## 8. Tests

- **New-behavior tests** each name the assertion that detects a wrong
  implementation and a compiling mutation that must make it fail; a test
  that fails only because a new item does not compile is not evidence.
- **Contract and compatibility tests** check rules P6 keeps.

Facade tests join their area binaries (`T/<area>/main.rs`); one new module,
`T/collections/display_rows.rs`, joins `collections`.

### New-behavior tests

1. **Table rows describe their bars (F16)** (`display_rows.rs`): records in
   a `Signal<Vec<Job>>`; a `publish` function builds a `TableSource` from
   them whose `cell` returns `ProgressBar::new(job.status)` and
   `Spinner::new(job.running)`, with no signal per row, and hands it to
   `TableController::set_source`, the only way a table's rows change
   (`R/table.rs:577-585`, `:673-685`), from application code after each
   record write, as `T/collections/controlled_rows.rs:256-271` does. Row `a`
   is `of(1, 4)` and running, row `b` `Failed(of(3, 8).fraction())`. Named
   assertion: after writing row `a` as `Succeeded` and not running,
   publishing and flushing, `a`'s bar reports `status` = `succeeded` and its
   spinner `busy` false, and `b`'s bar still `failed` with `fraction` 0.375.
   Mutation: `ProgressBar::reconcile` returns `None` whenever both
   descriptions are fixed (row `a` stays `determinate`).
2. **The same through a `VirtualList`** (`display_rows.rs`): the list's
   source is mapped from the records (`VirtualList::new` takes a
   `ReadSignal<ListSource>`, `R/virtual_list.rs:738`), and `build` returns a
   `Row` with `ProgressBar::new(task.progress)`; scrolling a row out and back
   remounts it. Named assertion: the remounted row reports its record's
   status. Mutation: `ProgressBar::new` ignores its argument (always
   `Idle`).
3. **Changes of source, bar and spinner** (`T/controls/control_reconcile.rs`),
   each through `check_redescription`: fixed `of(1, 4)` to fixed
   `Succeeded`, fixed to bound, bound to fixed, and **bound reading
   `Failed(None)` to fixed `Failed(None)`** (equal shown value); the same for
   `Spinner` with `true`. Named assertions: `check_redescription` returns
   `Ok`, the property shows the second description's value, and after each
   bound-to-fixed change a write to the old signal leaves the property
   unchanged with `FlushReport::components_updated == 0`. Mutations:
   reconcile keeps the old source when the kind changes (`Differs`);
   reconcile compares shown values and keeps the binding when they are equal
   (the old write updates the bar).
4. **Changes of source, field** (`T/controls/field_status.rs`): bound
   `Error("Required")` to fixed `Error("Required")`, then a write of
   `Normal` to the old signal; and a mounted field described with
   `.form(&handle)`, with the handle in error, described again with
   `.status(FieldStatus::error("Required"))`. Named assertions: the field
   still reports `status` = `error` and no component updates after the old
   write; `FormFields::focus_first_error` returns `false` and focus stays
   where it was. Mutation: `Field::reconcile` adopts only when the shown
   status differs (both assertions fail: the old binding still drives the
   field, the target stays recorded).
5. **Terminal statuses with a fraction** (`T/presentation/progress.rs`,
   the `fill` helper over the display list and inspection), for both
   `Failed` and `Cancelled` with `Some(ZERO)`, `Some(three_eighths)` and
   `Some(MAX)`:
   - fill and remainder: zero draws the Outline track alone; three eighths
     fills `width × 0.375` in Error or Muted with the gap and the Outline
     remainder; `MAX` fills the width;
   - `SemanticValue::Number` with the fraction and the `fraction` property;
   - `invalid` for `Failed` only; `is_finished()` and not `is_running()`;
   - then each status set to `None`: value and `fraction` property absent.

   Named assertions: the fill of `Cancelled(Some(three_eighths))` and its
   semantic value (the variant revision 1 left unguarded). Mutations:
   `fill()` maps terminal statuses to the full width; `inspect` reports a
   value only for `Determinate` and `Failed`; `invalid` set for every
   finished status.
6. **`ProgressStatus::fraction`** (its doctest): `Determinate(v)`,
   `Failed(Some(v))` and `Cancelled(Some(v))` give `Some(v)`; `Idle`,
   `Indeterminate`, `Succeeded`, `Failed(None)` give `None`. Named assertion:
   `ProgressStatus::Failed(ProgressStatus::of(3, 8).fraction())` equals
   `Failed(Some(three_eighths))`. Mutation: `fraction` returns `None` for
   `Determinate`.
7. **A described indeterminate bar animates** (`progress.rs`):
   `ProgressBar::new(Indeterminate)`. Named assertion: `next_deadline() ==
   Some(FRAME)`; redescribed as `new(Succeeded)`, `None`. Mutation: `update`
   requests `animate` only for a bound source.
8. **A described busy spinner steps** (`progress.rs`): `Spinner::new(true)`.
   Named assertion: `next_deadline() == Some(100 ms)`; `Spinner::new(false)`
   schedules nothing. Mutation: `update` calls `wake_after` only for a bound
   source (the shown `busy` is already true from construction, so the
   mutation must target scheduling).
9. **Fixed readout text applies at adoption** (`T/controls/control_reconcile.rs`):
   `InlineReadout::new("001:01:000").label("Pos")`.
   - Named assertions: right after `mount_tree`, before any flush, the
     `text` and `display_value` properties are `"001:01:000"`; after
     `reconcile_children` with `"002:01:000"`, before the flush, both are
     `"002:01:000"`, and after the flush still.
   - Text mode opened with `TextCommand::BeginEdit`: an equal fixed text
     keeps it open, a changed one ends it (`text_editing` false after the
     flush).
   - An invalid fixed text (`"\u{7}"`): `reconcile_children` succeeds, the
     flush returns the `TextEditError` (`InvalidText`), inspection keeps
     `"002:01:000"`, and the next valid description flushes cleanly.
   - A `RichText` of two spans shows their plain text joined.

   Mutations: fixed text applied in the update only (the before-flush
   assertions fail); an invalid fixed text adopted (the kept-text assertion
   fails); text mode ended on an equal text.
10. **Reactive readout sources** (`control_reconcile.rs`): a readout bound
    to a `Signal<RichText>` shows a written rich value's plain text after the
    flush, and described again with another `RichText` binding, it ignores
    writes to the first; a readout of a `StringRole` shows the new text after
    `UiSettings::set_strings` installs a table that changes that role. Named
    assertions: the shown text after each write and after the table change.
    Mutation: the readout reads its `TextSource` once, at the first update,
    and keeps that content (each later assertion fails).

    **A refused bound text keeps the last valid presentation** (rule
    3.4.4): a readout bound to a `Signal<String>` showing `"ok"`, in text
    mode, is written `"bad\u{7}"`. Named assertions: the flush returns the
    `TextEditError` (`InvalidText`); the `text` property, the semantic value
    and `display_value` are still `"ok"` and `text_editing` is still true;
    after a write of `"fine"` the flush succeeds, all three show `"fine"`
    and text mode has ended. The same for `formatted` with a format whose
    display returns a control character. Mutation: today's order, storing
    the text and ending text mode before `display_value` validates it (the
    kept-text and text-mode assertions fail).
11. **A field with a described status** (`T/controls/field_status.rs`):
    `Field::new(text_field).label("Name").status(FieldStatus::error("Required"))`.
    Named assertion: the control is `invalid`, its description is
    "Required" and the field reports `status` = `error`; described again with
    `FieldStatus::Normal`, the message is gone and the help shows. Mutations:
    `status` stores `Normal`; reconcile ignores a fixed status change.
12. **Builders replace** (`field_status.rs`): `.status_signal(s).status(w)`
    shows the warning `w` while `s` changes; `.status(w).status_signal(s)`
    shows `s`'s value and then a later write to `s`; `.form(&h).status(w)`
    shows `w`, and with `h` in error `FormFields::focus_first_error` returns
    `false`. Named assertion: each shown status. Mutation: `status_signal`
    leaves an earlier fixed status in place.
13. **A property row with a described status**
    (`T/collections/property_grid.rs`): `Property::new(..).status(FieldStatus::warning(..))`.
    Named assertion: the editor is described by the warning. Mutation:
    `Property::status` drops its argument.
14. **Fixed values apply at adoption** (`T/controls/control_reconcile.rs`),
    inspected with `Runtime::inspect` **before any flush**, right after
    `mount_tree` and right after a same-key `reconcile_children`:
    `ProgressBar::new(of(1, 4))` then `new(Succeeded)` (property `status`
    `determinate`, then `succeeded`); `Spinner::new(true)` then `new(false)`
    (`busy`, true then false: the constructor's own default is `false`,
    `R/progress.rs:367`, so starting from `true` is what makes the
    construction assertion meaningful);
    `Field::new(..).status(FieldStatus::error("Required"))` then
    `.status(FieldStatus::warning("Check"))` (`status` and `message`). Named
    assertions: each property before the flush, and the same after it.
    Mutation: construction and `reconcile` store the fixed value as the
    source only and the shown value is set in `update` (every before-flush
    assertion fails; the settled results of tests 3 and 11 would still
    pass, which is why this test exists: `check_redescription` flushes
    before it compares, `R/conformance.rs:158-163`). A second compiling
    mutation defers application **only in the constructors** while
    `reconcile` applies at once: the assertions right after `mount_tree`
    fail (the spinner reports `busy` false, the bar and field their
    defaults).
15. **The editable-list recipe** (its doctest, section 6): the test writes
    task 2's progress as `Succeeded` through `edit`, flushes, and reads the
    row's bar (`find(&list, &["2", "progress"])`). Named assertion: the
    bar's semantics description is `succeeded`. Mutation:
    `ProgressBar::reconcile` ignores a changed fixed status (the bar stays
    `idle`).

### Contract and compatibility tests

- **Bound behavior is today's:** the migrated tests of section 7
  (`ProgressBar::bound`, `Spinner::bound`, `Field::status_signal`,
  `Property::status_signal`) pass with their assertions unchanged, including
  the rebinding tests (`control_reconcile.rs:274-317`) and the
  indeterminate scheduling, hiding and reduced-motion tests of
  `progress.rs`.
- **Unknown fractions are today's full tracks:** `each_status_draws_its_own_fill…`
  (`progress.rs:45-88`) with `Failed(None)` and `Cancelled(None)`, and the
  semantics test's "an ended task has no fraction" (`:196-200`) with
  `Failed(None)`, unchanged.
- **The gap holds for a terminal partial fill:** the background-contrast
  test (`progress.rs:416-`) adds `Failed(Some(half))` and
  `Cancelled(Some(half))` to its statuses.
- **Every old `InlineReadout::new` argument still converts:** a test passes
  `ReadSignal<String>`, `&ReadSignal<String>`, `Signal<String>` and
  `&Signal<String>`.
- **Prelude closure** (`T/foundation/vocabulary.rs`, prelude only): the media
  module writes `ProgressBar::new(ProgressStatus::of(1, 2))`,
  `ProgressBar::bound(&status)`, `ProgressStatus::Failed(running.fraction())`,
  a `match` binding `ProgressStatus::Failed(reached)` and reading
  `reached.map(|r| r.get())`, `Spinner::new(true)`, `Spinner::bound(&busy)`,
  `InlineReadout::new("Ready")`, `Field::status(FieldStatus::error(..))`,
  `Field::status_signal(&status)` and `Property::status(..)`, without a
  `parameter` import.
- **Permission inputs unchanged:** the `api/` diff shows no
  `Permission`-typed signature change (review, section 9).
- **The gallery:** its progress test (`GA/tests/controls.rs:230-281`) passes,
  ending with an empty validation report, and the headless gallery's progress
  page has no validation issue with the partial states.
- **The packaged examples' headless sessions**, `log_explorer` (the `Failed`
  match) and `inventory`.
- **P2's listing-driven test** passes with the new listings; `every_name`
  is unchanged.

## 9. Scope, version and evidence

- **`api/` diff** (`api/pert-ui-runtime/default.txt` and `serde.txt`; the
  facade listings re-export by glob and do not change): changed
  `ProgressBar::new`, `Spinner::new`, `InlineReadout::new`, `Field::status`,
  `Property::status`, `ProgressStatus::Failed` and `ProgressStatus::Cancelled`
  (now tuple variants); added `ProgressBar::bound`, `Spinner::bound`,
  `Field::status_signal`, `Property::status_signal`,
  `ProgressStatus::fraction`. Each appears in both the root and the prelude
  path of the listing.
- **Breaking:** yes, each with its replacement in the CHANGELOG.
  `InlineReadout::new` is source-compatible for every conversion the library
  provides, but a caller relying on its own `From<_> for ReadSignal<String>`
  no longer compiles, so it is listed as breaking too.
- **CHANGELOG** for 0.3.81, citing this checkpoint:
  - `Breaking (F16):` `ProgressBar::new(ProgressStatus)` shows a described
    status; `ProgressBar::bound(binding)` follows a binding, as `new` did.
    Replacement: `ProgressBar::new(status.read_only())` becomes
    `ProgressBar::bound(status.read_only())`, or
    `ProgressBar::new(record.status)` in a collection row instead of a
    signal per row.
  - `Breaking (F16):` `Spinner::new(bool)` and `Spinner::bound(binding)`.
    Replacement: `Spinner::bound(busy.read_only())`, or
    `Spinner::new(record.running)`.
  - `Breaking (F16):` `Field::status(FieldStatus)` and
    `Field::status_signal(binding)`; `Property::status(FieldStatus)` and
    `Property::status_signal(binding)`; each replaces the other and `form`.
    Replacement: `.status(signal.read_only())` becomes
    `.status_signal(signal.read_only())`, or `.status(FieldStatus::error(..))`
    for a fixed status.
  - `Breaking (F16):` `InlineReadout::new` takes `impl Into<TextSource>`:
    fixed or bound text, plain or rich (shown as plain text), or a
    string-table role. Every argument the library converted before still
    converts; a caller-defined conversion into `ReadSignal<String>` is
    replaced by passing the `ReadSignal<String>`.
  - `Breaking (R2-26):` `ProgressStatus::Failed(Option<NormalizedValue>)`
    and `Cancelled(Option<NormalizedValue>)`; `Some` fills the track to the
    fraction reached and reports it in inspection. Replacement:
    `ProgressStatus::Failed(None)` for today's full track,
    `ProgressStatus::Failed(last.fraction())` to keep the fraction; patterns
    `Failed(_)`.
  - `Added (R2-26):` `ProgressStatus::fraction`.
  - `Fixed:` an `InlineReadout` whose bound or formatted text is refused
    keeps its last valid text in inspection and its semantic value and stays
    in text mode; it no longer stores the refused text before validating it
    (same kind as P6's fixed-text rule, in touched code).
  - `Documentation (F16):` contracts.md's display-control rule under
    "Ownership paths"; `ListItems::build`, `TableItems::cell` and
    `Signal::sibling` (per-row bindings narrowed to editing controls);
    guide/state.md's table, the editable-list recipe's progress column,
    guide/controls.md, the manual and the prelude pages.
- **Version:** 0.3.81 after P5's 0.3.80: the workspace version, the internal
  version requirements and `Cargo.lock`. If the order changes, the heading
  follows the actual version.
- **Gate:** the public-API row, as the plan states: the workspace checks
  (formatting, Clippy, tests including section 8 and P2's listing-driven
  test, the packaged examples' headless sessions, doctests, warning-free
  rustdoc, the diagnostics snapshots), the API listing gate with its diff
  reviewed against the CHANGELOG, the mutations of section 8 recorded in the
  Progress entry, and Codex's implementation review. No fixture check
  (section 7).
- **Native evidence:** the gallery's progress page (`cargo run --locked -p
  pert-ui-gallery`) viewed natively on Linux on an unlocked desktop with a
  hardware GPU: the "Failed at three of eight" and "Cancelled at three of
  eight" bars fill three eighths in Error and Muted with the gap and Outline
  remainder, and the live bar after Step, Step, Fail keeps its half fill in
  Error; recorded with the platform in the Progress entry and
  `docs/verification.md`'s native evidence. If not run, both record it as
  missing. Windows and macOS stay missing (NATIVE-01 to NATIVE-05).
- **Persistence, real time, backends:** no persisted schema
  (`ProgressStatus` has no serde derive), live-data endpoint or backend type
  is involved.
- **TODO.md:** no entry touches the changed files except NATIVE-05, which
  names the `log_explorer` window's missing native evidence; P6 does not
  resolve it.

## Corrections to the plan

- **No `Field` enablement.** P6's test "a `Field` whose enablement is a
  `Permission` keeps its composition" has no subject: `Field` has no
  `Permission`-typed builder (`R/field_status.rs:480-550`). It is replaced by
  the builder-replacement test (section 8, test 12) and the `api/` review
  that no `Permission`-typed signature changes.
- **Consumers.** Neither packaged example calls `Field::status`: `inventory`
  calls only `InlineReadout::new`, which compiles unchanged, and
  `log_explorer` needs `ProgressBar::bound` and the `Failed` migration. The
  runtime README's and guide/controls.md's examples compile unchanged. The
  gallery's text page (`GA/pages/text.rs`) is a consumer too, with a `fixed`
  helper that P6 retires.
- **"The inspection description says the fraction"** is read as "inspection
  reports the fraction": as the semantic value and the `fraction` property,
  as `Determinate` does. The description stays the status name, the same
  name the `status` property carries and tests match
  (`T/controls/control_reconcile.rs:288-291`); a number in it would
  duplicate the value in prose (3.6.4).
- **`InlineReadout::new` is not strictly non-breaking** (D1's "keeps
  compiling"): true for every library conversion, not for a caller's own
  `From` into `ReadSignal<String>`; the CHANGELOG lists it.
- **D13's "`Some` fills to the fraction"** leaves a zero fraction looking
  like an empty track; 3.6.3 states it, and `fraction()` gives `None` after
  an indeterminate run, so the common "fail while running" case is a full
  track.
- **L2's prelude exception** for `NormalizedValue` covered only
  `Determinate`; the new payloads need its extension (section 5, P6-D1).
- **"The guide's state table … gain[s] a progress column":** the state table
  gains a row; the editable-list recipe gains the column.

## Findings outside this part

None verified. The list of section 2 was read in full; the inputs without a
fixed form are deliberate and documented on their builders.

## Not in this part

- **Rich readout text:** `InlineReadout` draws plain text (3.4.2); drawing
  spans in its value style is not requested.
- **Controlled forms of editing controls** (D1's extension, follow-up
  "Controlled forms of more controls"): updated with round 2's evidence,
  not implemented.
- **A distinct presentation of a failure at zero** (for example an Error
  outline on an empty track): no run showed the case.
- **A fixed `MenuItem` mark** and fixed visibility builders (section 2).

## Decisions

- **P6-D1: how a prelude-only program writes a reached fraction.**
  **Approved by the author as recommended** (2026-10-28, maintainer's
  delegation).
  - **Recommended: `ProgressStatus::fraction()`**, so the terminal status is
    built from the last running one, `Failed(last.fraction())`, and L2's
    exception is extended in words. It needs no new name in the prelude,
    matches how a job ends (its last reported status is at hand, as
    `log_explorer`'s `progress` signal and the task 5 runs' records show), and
    also serves a test or tool reading any status's fraction.
  - **Alternative A: `NormalizedValue` into the prelude,** so
    `Failed(Some(NormalizedValue::try_from(0.4)?))` is written from the
    prelude. One `parameter` vocabulary name joins the prelude for a payload,
    against L2's membership rule and its "Not required" exception for this
    very type (`layering-design-L2.md`, section 1.2, `:156-160`).
  - **Alternative B: constructors from counts,** `ProgressStatus::failed_at(done,
    total)` and `cancelled_at`, one per terminal variant, mirroring `of`.
    Prelude-only too, but two constructors beside the variants themselves,
    a second way to build them, and of no use when the application tracks a
    fraction rather than counts.
  - **Alternative C: no helper.** A prelude-only program can already match
    the last status with inferred payload types:
    `Failed(match last { ProgressStatus::Determinate(v) => Some(v), _ => None })`.
    It compiles without naming `NormalizedValue`, but repeats a three-line
    `match` at every place a task ends (both terminal variants, in every
    worker message handler), and the reader must remember that a terminal
    status may already carry a fraction. `fraction()` is that `match` once,
    covering all three carrying variants.

## Review record

- **Revision 1 (2026-10-28):** written from the code at ee55226 (library
  code as at 26e6b5a).
- **Revision 1, Codex review (2026-10-28):** needs revision, seven
  should-fix points and a nit group; record
  `~/pert-ui-usability/codex-prompts/codex-design-P6.out`. Each point was
  checked against HEAD (07eb0a4, library code unchanged); none was rejected.
  1. **Fixed readout text at adoption:** revision 1 read fixed text in the
     update, against `TextSource`'s promise (`R/text_source.rs:24-30`).
     Revision 2 applies it at adoption through the pure display check
     (3.4.3), refusing invalid text there with the last valid text kept and
     the error returned from the next update, and leaves ending text mode to
     that update because it ends the editor's draft slot; test 9 asserts the
     text before and after the flush. No exception to `TextSource` remains.
  2. **Field reconcile compares sources:** 3.1.4 and 3.5.3 state it, bar
     and spinner included (3.2.3); tests 3 and 4 cover equal-value bound to
     fixed and a mounted form described with a fixed status.
  3. **Table republication:** test 1 publishes through
     `TableController::set_source` from application code after each record
     write (`R/table.rs:577-585`, `:673-685`), as
     `controlled_rows.rs:256-271` does.
  4. **Reactive readout sources:** test 10 covers a `RichText` binding, its
     replacement, and a role following a new string table.
  5. **Terminal statuses:** test 5 covers both variants at zero, three
     eighths and full: fill and remainder, value, `fraction` property,
     `invalid` for failures only, classification, and clearing on `None`;
     unknown fractions stay contract tests.
  6. **Mutations:** test 8's mutation targets scheduling; test 15 (the
     recipe; 14 in revision 2) has a named assertion and a mutation.
  7. **Examples:** section 4 names a runnable example for every new or
     changed public item, `Field::status_signal` and
     `Property::status_signal` with replacement and a later binding write.
  8. **Nits:** P2 and P4 are approved and committed (introduction); the two
     field tests that never write their signals migrate to `status` and the
     rebinding test is named as binding coverage (section 7); `reached` is an
     `Option` (section 5); the description evidence is corrected (3.6.4,
     corrections); P6-D1 cites L2's section 1.2 exception, counts
     alternative B correctly and adds the no-helper alternative C.
- **Revision 2, Codex recheck (2026-10-28):** needs revision, three
  should-fix points and a nit; record
  `~/pert-ui-usability/codex-prompts/codex-design-P6-r2.out`. Each was
  checked against HEAD (416dba7); none was rejected.
  1. **Reactive readout order:** today's update ends text mode and stores
     the text (`R/inline_controls.rs:723-731`) before `display_value`
     validates it (`:732`), so refused text reaches inspection
     (`:879-889`). Revision 3 validates first for bound, role and formatted
     text (3.4.4), adds the rejection and recovery assertions to test 10,
     and records the change as an in-scope same-kind fix with a `Fixed:`
     CHANGELOG entry.
  2. **Adoption timing for the other fixed inputs:** new test 14 inspects
     `ProgressBar`, `Spinner` and a fixed `Field::status` before any flush
     after mount and after a same-key reconcile, with a mutation that defers
     application to the update; `check_redescription` alone cannot see it,
     since it flushes first (`R/conformance.rs:158-163`).
  3. **`ProgressStatus`'s ownership sentence** (`R/progress.rs:16-19`):
     section 6 migrates it to transitions by description or binding write,
     keeping "the library never advances, times or completes a task".
  4. **Nit:** the introduction records P1 as committed (84bb60f, behavior
     unchanged, lines moved in some cited files) and P5's design as
     approved (416dba7).
- **Revision 3, Codex second recheck (2026-10-28):** one should-fix: test
  14's spinner assertion was vacuous at construction (`Spinner::new(false)`
  equals the constructor's default). The author changed test 14 to
  `Spinner::new(true)` then `new(false)` and added the constructor-only
  mutation, exactly as Codex proposed; nothing else changed. Codex found
  no other blocker or should-fix. Approved by the author under the
  maintainer's delegation, with P6-D1 as recommended.
