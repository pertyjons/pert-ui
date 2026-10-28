# Round 2 fix plan, part P12 design checkpoint: form drafts and the settings recipe

Part P12 of [the round 2 fix plan](round-2-fix-plan.md#p12-form-drafts-and-the-settings-recipe-0387)
closes R2-2 by decision D2 (approved with the recommended option on
2026-10-28) and resolves the TODO.md entry on the diagram editor's
`close_requested` (draft policy rule 10, `TODO.md:119-127`). The plan gives
P12 a short checkpoint for the method's contract and the contract text, and
asks it to confirm that L4b's matrix is unchanged; this document settles
them, with the settings-form recipe, the diagram editor's change, tests,
CHANGELOG, gate and native evidence, before any code. Codex reviews it; the
author approves it together with Codex's review under the maintainer's
delegation of 2026-10-28.

Every reference to code is to `a98fdb2` (P4 committed, 0.3.79), and every
line number was read there; revision 2 rechecked its citations at the same
commit. Lines differ from the plan's, which are at 26e6b5a: P1 to P4
changed several of the cited files (among them `R/runtime.rs`,
`R/text_fields.rs`, `R/text_fields/inline_text.rs`, `R/property_grid.rs`,
`E/diagram_editor/app.rs`, the guide chapters and `docs/contracts.md`), so
a 26e6b5a line is not assumed to hold. Parts P5 to
P11 land before P12 and may move lines in the cited files, so the
implementer relocates a citation by its content. **P12 builds on two
approved designs that land before it:** P10's tuple `combine`
([round-2-design-P10.md](round-2-design-P10.md), sections 2.1 and 3:
`ReadSignal::combine((&a, &b, …), |(a, b, …)| ..)`, 2 to 12 inputs, the
function receiving `&(A, B, …)`) and P11's application state
([round-2-design-P11.md](round-2-design-P11.md), sections 2.3 to 2.4 and
the diagram editor's row of section 6: `App<S>`, handlers receiving
`&mut S`, `HeadlessApp::state` and `runtime_and_state_mut`, the diagram
editor's state `Rc<DiagramApp>`). P11's section 4 lists what P12 relies on;
nothing below needs more. Path prefixes are the plan's: `R/` =
`crates/pert-ui-runtime/src/`, `F/` = `crates/pert-ui/src/`, `G/` =
`crates/pert-ui/src/guide/`; `T/` = `crates/pert-ui/tests/`, `E/` =
`crates/pert-ui/examples/`, `U` = `~/pert-ui-usability/` (frozen runs).

Status: **revision 2 with the author's correction of Codex's recheck,
approved 2026-10-28** by the author together with Codex's review (recheck:
ready for approval, two nits, corrected as Codex proposed), under the
maintainer's delegation, with P12-D1 to P12-D3 as recommended. Revision 1 was
reviewed by Codex (needs revision: one blocker, two should-fix points and a
nit); revision 2 answers each ([review record](#review-record)). P12-D1 to
P12-D3 are approved by the author as recommended, under the maintainer's
delegation of 2026-10-28.

## What testers built by hand, and what the library says

- **R2-2** (results row at `plans/archive/developer-usability.md:920`,
  triage N2 at `runs/triage-r2/group-1.md:237`, the acceptance row at
  `:56`). Both task 1 runs wrote a Revert that writes the applied values
  back. U `r2-1-claude/work/src/lib.rs:266-284` writes five signals and
  clears the statuses; the author check
  (U `acceptance-r2-1-claude/work/tests/author.rs:68-93`) types `150` into
  a quality field bound to 70, clicks Revert and finds the refused draft
  still shown (`invalid`), because the write equals the saved value.
  U `r2-1-codex/work/src/main.rs:175-192` adds a deferred loop of
  `ui.text_command(field, TextCommand::DiscardDraft)` over editor ids that
  `:250-254` collected with `find_descendant` after mounting: the workaround
  D2 retires. The triage's probe `same_value_write_keeps_refused_draft`
  (`runs/triage-r2/group-1.md:23-25`) reproduces it.
- **The library says** an equal write changes nothing (`docs/contracts.md:1253-1256`),
  `IntegerInput`'s page says "External changed values win over drafts"
  (`R/text_fields/number_inputs.rs:88`), `TextField`'s says "an external
  write that changes the binding wins" (`R/text_fields.rs:178-180`); none
  says what to do instead. `TextCommand::DiscardDraft` is documented
  (`docs/contracts.md:1313-1320`), but a form has no way to reach its
  editors without collecting their ids.
- **The diagram editor's close policy** asks only whether the document is
  dirty (`E/diagram_editor/app.rs:896`), so a draft the inspector's label
  `TextField` or a size `DecimalInput` refuses (`:458-470`, `:472-497`)
  stays unsaved through admission and is lost with the window without a
  prompt (TODO.md entry; the function's own comment says so, `:886-890`).

## 1. Today's behavior, from the code

- **What a form records.** `FormFields` keeps one `Entry` per registered
  field: key, status signal, a `modified` mark and a shared control cell
  (`R/field_status.rs:140-145`). `Field::form(&handle)` binds the status and
  the control cell (`:544-550`); `Field::status` binds the status and no
  cell (`:537-543`). The cell is written in `Field::update` with
  the Field's single child (`:650-652`), cleared when the Field unmounts
  (`:655-659`) or is described with another target (`:613-622`). Hiding does
  not clear it. `focus_first_error` walks the entries in registration order
  and skips a cell that is empty or whose control is not visible (effective
  visibility, `R/runtime.rs:899-902`) (`R/field_status.rs:278-304`).
- **Every recorded control is in the form's runtime.** `Field::update` reads
  the handle's status before it records the control (`:637-639`), and a read
  of another runtime's signal fails with `ForeignRuntime`
  (`R/runtime.rs:2958-2965`). A handle's status is a sibling of the form's
  summary signal (`R/field_status.rs:256`), so a `Field` mounted in another
  runtime with this form's handle fails its update and never records.
- **The modified count** follows only `FieldHandle::set_modified`
  (`:358-373`), which the application calls from an editor's `on_modified`.
  A removed editor reports nothing (rule 10, `docs/contracts.md:1289-1299`),
  so a mark set by an editor that is later removed stays set until the
  application clears it or calls `FieldHandle::remove` (`:386-410`).
- **The route D2 builds on.** `EventContext::request_text_command`
  (`R/runtime.rs:3267-3311`) checks the target's runtime, then pushes
  `(Weak<Node>, command)` onto the runtime's queue; its only error is
  `ForeignRuntime`. The queue is delivered by `apply_requested_text_commands`
  (`:6236-6262`) inside the flush's settle loop (`:5875`), in request order:
  a target dropped, unmounted or hidden by then is skipped; a modal-blocked
  target yields `ModalBlocked`; any error is returned with the later
  requests put back at the front of the queue. Delivery goes to the target,
  and when it answers `NotAnEditor`, to the focused component inside its
  subtree (`:5520-5572`).
- **What `DiscardDraft` does.** `TextField`/`TextArea` (`R/text_fields.rs:2366-2386`),
  the number editors and inline rows (`R/text_fields/number_inputs.rs:1873-1902`)
  and `InlineText` (`R/text_fields/inline_text.rs:569-587`) discard the
  draft as Escape does, also while disabled or read-only, answer
  `Unavailable` without one, and end by calling the refusal report's
  `clear`, which calls `on_rejection(None)` once if a refusal was
  **reported** (a save was refused) and keeps a failed `None` pending
  (`R/text_fields/normalize.rs:126-137`). That call runs inside the command's
  delivery, in the event phase; the editor has already discarded its draft
  and requested its `on_modified(false)`, which is delivered after
  settling. An in-place session whose editor holds focus returns focus as
  Escape does (`R/runtime.rs:3274-3284`). A `DraftEditor` answers
  `Unavailable` (`docs/contracts.md:1317-1318`).
- **The `on_rejection` pages omit the command.** `TextField::on_rejection`
  lists the end of a reported refusal "at a later save ... and at Escape"
  (`R/text_fields.rs:374-379`), the number editors' "at a later save or at
  Escape" (`R/text_fields/number_inputs.rs:2420-2423`), `InlineText`'s "at a
  later save, at focus loss or at Escape" (`R/text_fields/inline_text.rs:456-459`);
  each also ends at `DiscardDraft`, which every caller of P12's method
  triggers.
- **A button click moves focus.** Pointer presses focus a focusable
  component (`R/runtime.rs:1385-1397`), so the focused editor makes its
  focus-loss save attempt before a Revert button's callback runs: a valid
  draft is saved, a refused one is kept (rules 1 and 2).
- **Who has an `EventContext`.** Button and `MenuItem::new` actions do
  (`R/menu.rs:266-272`); command actions receive `&mut Runtime`
  (`R/runtime/commands.rs:607-612`), and so do the native close policy and
  deferred work.

## 2. The rules

### 2.1 `FormFields::discard_drafts`

1. **Signature:** `pub fn discard_drafts(&self, cx: &mut EventContext) ->
   Result<usize, RuntimeError>`, in `R/field_status.rs` beside
   `focus_first_error`. `&mut` because `request_text_command` takes it.
2. **Admission, all or nothing ([P12-D1](#decisions)).** If `cx` belongs to
   another runtime than the form (the runtime of its summary signal), the
   call returns `ForeignRuntime` and queues nothing. That check covers every
   recorded control (section 1, second point), so no other error is
   possible at the call: each request it then queues cannot fail. The call
   does not read or write any signal.
3. **What it queues.** One `TextCommand::DiscardDraft` request, through
   `EventContext::request_text_command`, for each registered field, in
   registration order, whose control cell holds a control that is visible
   (effectively) at the call. Skipped, as `focus_first_error` skips them:
   a field no `Field` or `Property` bound with `form(&handle)` shows (never
   mounted, unmounted, described with `status` or `status_signal`, or
   removed with `FieldHandle::remove`), and a field whose control is
   hidden, which holds no draft (rule 7). The control is the Field's single
   child; when it is not an editor, the command reaches an editor inside it
   only while that editor holds focus (section 1, delivery), so a form wraps
   each editor in its own `Field`.
4. **Return value:** the number of requests queued. It says nothing about
   which controls had a draft; a control without one answers `Unavailable`,
   and a non-editor `NotAnEditor`, neither an error.
5. **Delivery** is `request_text_command`'s, unchanged: the event phase of
   the flush the caller runs in, or of the next flush when the callback ran
   in an input call; dropped for a target unmounted or hidden by then;
   `ModalBlocked` for a target the current modality blocks, whose request is
   spent; a target's error returned from the flush. After either error the
   later requests stay queued and the next flush delivers them, so progress
   can be partial, in registration order.
6. **Notifications** are each editor's for `DiscardDraft`, for an editor
   that **still holds its draft when the request is delivered**:
   `on_rejection(None)` inside the delivery where a refusal was reported,
   `on_modified(false)` after settling; a session's edit stays open and its
   next completion reports `Unchanged`. A failing `on_rejection` callback is
   that target's error (rule 5): its draft is already discarded, its `None`
   stays pending (`normalize.rs:126-137`). **A draft already ended by then
   gets the other route's reports.** Updates run before queued text
   commands in each settle wave (`R/runtime.rs:5865-5875`), so when the
   caller has just written a *changed* value, the editor's update discards
   the draft as an application write (`R/text_fields/number_inputs.rs:1337-1342`,
   `R/text_fields.rs:2452-2458`), reports `on_modified(false)`, and the
   request finds no draft and answers `Unavailable` before reaching the
   refusal report (`R/text_fields/number_inputs.rs:1877-1882`, whose own first step is that
   synchronization,
   `R/text_fields.rs:2374-2376`). A reported refusal then ends, as
   `on_rejection` documents for application writes, at the editor's next
   save or Escape (`R/text_fields.rs:376-379`). An application that shows
   refusals as statuses therefore clears the statuses it owns on Revert
   itself; it cannot rely on `on_rejection(None)` from this call.
7. **Focus** is inherited, not suppressed: a focused in-place session that
   is discarded returns focus as Escape does (row mode: its nearest
   focusable ancestor). The method requests no focus of its own.
8. **The form's counts** change only through the handles: the method writes
   no status and no `modified` mark. An application that feeds
   `set_modified` from `on_modified` sees `FormSummary::modified` fall when
   the reports arrive after settling.
9. **No application data** is written, so the draft policy is unchanged
   (section 2.2).

### 2.2 The draft policy and L4b's matrix

**Confirmed unchanged in substance.** The method is `DiscardDraft` sent to
several targets; `DiscardDraft` is already part of the policy
(`docs/contracts.md:1313-1320`, L4b's D3 at `layering-design-L4b.md:522-572`),
its effect, availability and reports are unchanged, and no event row is
added: the matrix (`docs/contracts.md:1322-1333`) lists events that reach
an editor, and a form discard reaches each as the command. Text changes:

- **Rule 3** (`:1253-1256`) keeps its sentence and continues: "The draft
  stays; an application that must end it, such as a form's Revert after
  writing the applied values, sends `TextCommand::DiscardDraft`, to every
  field of a form with `FormFields::discard_drafts`."
- **The `DiscardDraft` paragraph** (`:1313-1320`) adds `on_rejection(None)`
  where a refusal was reported, beside `on_modified(false)`, and one
  sentence: "`FormFields::discard_drafts` queues it for every field a form
  shows, in registration order." It also says that a draft a changed write
  ended before the command arrived is reported by that write's route: the
  command finds no draft, and a reported refusal ends at the next save or
  Escape (rule 2.1.6).
- **The matrix row label** "Application write" becomes "Application write
  of a changed value"; no cell changes. It is the reading R2-2's testers
  took for the equal write. The policy-table test
  (`T/editing/draft_policy.rs:1448`) passes unchanged.

### 2.3 The settings-form recipe

A new section **"A settings form"** in `G/state.md`, after "Worker results
and live data" (it uses `EventContext::spawn`, introduced there) and before
"Bounded updates and recovery" (`G/state.md:170`). One runnable doctest,
`# #[cfg(feature = "native")]` with an empty `main` otherwise, as the
testing chapter's close policy is (`G/testing.md:456-496`), importing
`pert_ui::prelude::*` and, in its test part, `pert_ui::testing::*` only. It
shows, in this order:

1. **The state** the initializer returns (P11): the field signals, the
   applied record in a signal, the `FormFields`, the derived `changed`
   binding and the root.
2. **Fields registered in a `FormFields`:** a `TextField` whose normalizer
   refuses an empty name and an `IntegerInput` with `range(1, 99)`, each in
   `Field::new(..).form(&handle)`, each reporting `on_modified` to
   `set_modified` and `on_rejection` to the handle's status.
3. **Dirty state through P10's `combine`:** `changed` over the fields and
   the applied record; Revert's availability over `changed` and the form's
   summary (`*changed || summary.modified() > 0`), so Revert is offered
   for a refused draft whose bound value equals the applied one.
4. **Apply through `EventContext::spawn`:** the button reads the values,
   writes them on the worker (a stand-in for the settings file) and sets
   the applied record in the delivery.
5. **Revert:** write the applied values, clear the two statuses the
   application published (`FieldHandle::clear`), then
   `form.discard_drafts(cx)?`, with the comment that an equal write keeps a
   draft. The prose says the click first takes focus, so a valid draft is
   saved by focus loss and then overwritten, and a refused one is kept
   until the write (a changed value) or the discard (an equal one) ends it;
   and that the statuses are the application's, so Revert clears them
   itself (rule 2.1.6).
6. **The close policy reading P11's state:** `on_close(|_, settings, _|
   ..)` keeps the window open while `changed` or `summary.modified() > 0`.
7. **The test,** with `app.headless()`, `runtime_and_state_mut` and a
   `Driver`, covers both bound values Revert can meet:
   - **Equal:** type `150` into the count and press Enter (refused, the
     field invalid, status `Error`, `modified() == 1`, the binding still the
     applied 1); `admit_close` is `KeepOpen`; click Revert: the field is no
     longer invalid, its status is `Normal`, `modified() == 0`, the binding
     1; `admit_close` is `Close`.
   - **Changed:** type `8` and press Enter (saved, `changed` true), then
     `150` and Enter (refused, status `Error`); click Revert: the binding is
     1, the field is no longer invalid, its status is `Normal` (cleared by
     Revert, since this draft ended by the write, rule 2.1.6) and
     `modified() == 0`; `admit_close` is `Close`.
   - Then a valid name, Apply, `wait_until` the applied record follows,
     `changed` false.

Sketch (the implementer compiles and trims it; names may change, the
calls may not):

```rust
#[derive(Clone, Debug, PartialEq)]
struct Applied { name: String, copies: i64 }
struct Settings {
    name: Signal<String>, copies: Signal<i64>, applied: Signal<Applied>,
    form: FormFields, name_field: FieldHandle, copies_field: FieldHandle,
    changed: ReadSignal<bool>, root: WidgetId,
}
fn build(ui: &mut Runtime) -> Result<Settings, RuntimeError> {
    // signals, form.field("name")?, form.field("copies")?
    let changed = ReadSignal::combine((&name, &copies, &applied), |(name, copies, applied)| {
        *name != applied.name || *copies != applied.copies
    })?;
    let unsaved = ReadSignal::combine((&changed, &form.summary()), |(changed, summary)| {
        *changed || summary.modified() > 0
    })?;
    let revert = Button::new("Revert", move |cx| {
        let applied = applied.get()?;
        name.set(applied.name)?;    // a value the field already shows keeps its draft,
        copies.set(applied.copies)?;
        name_field.clear()?;        // the statuses are the application's own
        copies_field.clear()?;
        form.discard_drafts(cx)?;   // so the drafts are discarded too
        Ok(())
    })
    .enabled(&unsaved);
    // Apply: cx.spawn(move || values, move |values| applied.set(values)), enabled(&changed)
    // mount Field::new(..).form(&handle) for both editors, the buttons; return Settings
}
let mut app = App::new("Settings", |ui| Ok(build(ui)?))
    .on_close(|_, settings, _| {
        let unsaved = settings.changed.get()? || settings.form.summary().get()?.modified() > 0;
        Ok(if unsaved { NativeCloseAction::KeepOpen } else { NativeCloseAction::Close })
    })
    .headless()?;
```

The recipe is the plan's "settings-form recipe" input to app-author slice 9
(plan, "Out of scope"), not a tutorial. The testing chapter's close-policy
paragraph that counts unsaved fields (`G/testing.md:502-507`) points to it.

### 2.4 The diagram editor's close policy

1. **The inspector's form.** `Inspector` (`E/diagram_editor/editor.rs:37-44`)
   gains `form: FormFields` and one `FieldHandle` each for `label`, `width`
   and `height`, registered in `Editor::new` (`:79-106`). In
   `node_inspector` (`E/diagram_editor/app.rs:452-532`) the label
   `TextField` and both `DecimalInput`s report `on_modified` to their
   handle's `set_modified`. The rows are not bound with `Property::form`:
   the editor publishes no statuses and discards nothing, so it needs no
   recorded control. The `SegmentedControl` and `ColorPicker` keep no
   draft (rule 10's list).
2. **Forgetting a removed inspector's marks, only when it is removed.**
   A removed editor reports nothing (section 1), so the marks of a node
   grid that leaves would stay set; but clearing them merely because
   `shown` changed is wrong, because the grid is keyed by node
   (`app.rs:446`, `:531`) and `Dynamic` reconciles a child that keeps its
   key, with its drafts (`R/described.rs:121-124`): `select(B)` then
   `select(A)` before a flush leaves A's grid and a refused draft in place,
   and its editors do not report `true` again, since an editor reports only
   a state that differs from the one it last delivered
   (`R/text_fields/modified.rs:37-38`, `:54-56`). So the clear is tied to a
   removal that is certain:
   - `Shown::One` carries an **inspector incarnation**,
     `Shown::One { node: NodeId, incarnation: u64 }`, from a counter in
     `Inspector`. `Editor::sync` (`editor.rs:175-198`) compares the target
     it computes (nothing, several, or one node) with the current one;
     when the target differs, it advances the counter, writes `shown` with
     the new incarnation and sets the three handles' marks to `false`;
     when it is the same, it writes nothing to `shown` and keeps the marks.
   - `node_inspector` keys the grid by node **and** incarnation
     (`format!("{}-{incarnation}", id.key())` in place of `id.key()`,
     `app.rs:531`). Every change of target therefore changes the key the
     `Dynamic` reads at its next update, A → B → A included, so the grid
     whose marks were cleared is replaced and unmounts with its drafts.
   - Hence a clear always coincides with a removal of the editors it
     covers. The converse direction is safe: a removal `sync` did not cause
     leaves a mark set, which makes the policy ask, never close over a
     draft. Hiding the inspector reports `false` itself (rule 10).
   - Every path that changes the target goes through `sync` (`install`,
     `select` and `replace`, `editor.rs:153-171`, `:201-209`, `:260-270`,
     and the outline's selection change, `app.rs:751-754`). The type
     comment (`editor.rs:21-24`) says leaving a node discards its drafts,
     also when it is selected again before the screen updates. The
     session's `Shown::One(start)` assertion (`session.rs:266-269`) matches
     the node instead.
   - The two size editors get keys of their own, `width-input` and
     `height-input` (today both share their `Property`'s key, `app.rs:524-525`,
     `:496`), so the session finds them as it finds `label-input`.
3. **The policy** (`app.rs:886-913`) asks when the document is dirty **or**
   `inspector.form.summary().get()?.modified() > 0`, with the same discard
   dialog ([P12-D2](#decisions)); its message (`:876`) becomes "The diagram
   or the inspector has unsaved changes."; the doc comment says a draft the
   inspector's field refuses now counts. Accepting closes and loses the
   draft, which the user agreed to; cancelling keeps the window and the
   draft.
4. **Tested through `HeadlessApp::admit_close`.** The headless session
   drives a `Runtime` of its own (`E/diagram_editor/session.rs:121-131`), and
   the example's `App` exists only in `main.rs`'s native `window()`. P12
   moves the `App` construction into `#[cfg(feature = "native")] fn
   application(dir: PathBuf) -> Result<App<Rc<app::DiagramApp>>, Box<dyn
   std::error::Error>>` in `main.rs` (P11's initializer, handlers and
   `size`), which `window()` runs and the session's new `close_admission`
   step starts with `headless()` in a directory of its own (section 6,
   test 6). That step replaces the session's stand-in, which released focus
   with `HostIntegration::clear_focus` and called `close_requested` itself
   (`session.rs:513-543`, the passage P4's design left to P12,
   `round-2-design-P4.md:494`). Without `native` the step is compiled out,
   as `window()` is; the module comment says the close checks need the
   `native` feature.

## 3. Public API

In `pert_ui_runtime` (root and prelude, unchanged membership), and so at the
facade's root and prelude through its glob:

```rust
impl FormFields {
    /// Discards the draft of every field this form shows, for a Revert that
    /// writes the applied values back: writing the value an editor already
    /// shows changes nothing and keeps its draft (draft policy rule 3).
    ///
    /// [Rules 2.1.2 to 2.1.9: queues one `TextCommand::DiscardDraft`, through
    /// `EventContext::request_text_command`, for each registered field, in
    /// registration order, whose `Field` or `Property` bound with `form` has
    /// a visible control; what is skipped; the return value; delivery in the
    /// event phase of this flush, or of the next one after an input call,
    /// with dropped, modal-blocked and failing targets and partial progress;
    /// `on_rejection(None)` in the delivery and `on_modified(false)` after
    /// settling for an editor that still holds its draft then, while a
    /// draft a changed write in the same callback already ended reports as
    /// an application write does, so statuses the application published
    /// from `on_rejection` are cleared by the application; inherited focus; the summary changes only through the
    /// handles; a field's control is the Field's single child, so each editor
    /// gets its own `Field`.]
    ///
    /// [Runnable example: an `IntegerInput` bound to 7 in a `Field` with
    /// `form`, a Revert button that writes 7, clears the field's status and
    /// calls this; a `Driver`
    /// types 150 and presses Enter (refused, invalid), clicks Revert, and
    /// the field is valid again with the binding still 7.]
    ///
    /// # Errors
    /// Returns `ForeignRuntime`, queueing nothing, when `cx` belongs to
    /// another runtime than the form. A target's error comes from the flush
    /// that delivers its request, not from this call.
    pub fn discard_drafts(&self, cx: &mut EventContext) -> Result<usize, RuntimeError>;
}
```

- **No `#[must_use]`:** `Result` is already must-use, and `?` consuming it
  is the intended use; the count is informative.
- **Implementation outline:** compare `cx`'s runtime with the summary
  signal's; collect the visible recorded controls into a `Vec` (releasing
  the entries' borrow before queueing); queue each with
  `request_text_command`; return the length. No new runtime internals
  beyond a crate-private runtime comparison, which `read_kind` and
  `request_text_command` already make.
- **Dependencies:** none; `pert-ui-runtime` keeps building without window
  or GPU dependencies.

## 4. Module membership and the prelude closure

- **No new names.** `FormFields` is a prelude member (`R/lib.rs:454`); the
  method's argument and result types, `EventContext` and `RuntimeError`, are
  prelude members (`R/lib.rs:417-420`), and `usize` is a primitive, so
  L2's closure rule (`layering-design-L2.md`, section 1.2) adds nothing
  (plan, "Prelude closure"). No `testing` member reaches `FormFields`, so
  P2's listing-driven check (`tests/diagnostics/tests/testing_closure.rs`) is
  unaffected; it runs in the gate.
- **The recipe** writes only prelude names (`App`, `Button`, `Column`,
  `Element`, `Field`, `FieldStatus`, `FormFields`, `InputRejection`,
  `IntegerInput`, `NativeCloseAction`, `ReadSignal`, `Runtime`,
  `RuntimeError`, `Signal`, `TextField`, `WidgetId`) and, in its test part,
  `testing` names (`Driver`, `HeadlessApp` through `headless()`).
- **Collision review:** a method, so none.
- **Earlier parts.** P6 keeps `Field::form` and adds `status_signal`, which
  clears the target as `status` does (`round-2-design-P6.md`, section 3.5);
  rule 2.1.3 names both. P10 and P11 are used as approved. P2, P4, P5, P7,
  P8 and P9 are untouched; P8's inspection schema does not change.

## 5. Documentation

Text describes P12's own commit.

- **`FormFields`** (`R/field_status.rs:173-186`): the overview adds "and can
  discard every field's draft"; "schedules nothing" becomes "schedules
  nothing of its own: it changes only when a handle is written or removed,
  and queues text commands only when `discard_drafts` is called" (a
  same-kind correction the new method makes necessary).
- **`discard_drafts`**: section 3.
- **`Field::form`** (`:544-545`) and **`Property::form`**
  (`R/property_grid.rs:81-86`): "lets `focus_first_error` reach this field's
  control" adds "and `discard_drafts`". **`FieldHandle::remove`**
  (`R/field_status.rs:386-391`): neither reaches its control afterwards.
- **`FieldHandle::set_modified`** (`:358-366`): one sentence that a removed
  editor reports nothing, so an application that removes a field's editor
  while keeping its handle clears the mark itself, or removes the field
  (same-kind: the diagram editor needs it, section 2.4 rule 2).
- **Editors' pages, an equal write keeps a draft:** `TextField`'s
  "Normalization" (`R/text_fields.rs:178-180`, which `TextArea`'s follows,
  `:247-251`), `IntegerInput`'s page (`R/text_fields/number_inputs.rs:86-88`,
  which `DecimalInput` and the inline rows follow), each after its sentence
  that a changed application write wins, and `InlineText::normalize`
  (`R/text_fields/inline_text.rs:293-298`), after "Text the application
  writes to the binding or describes is never normalized": "Writing the
  value the field already shows changes nothing and keeps the draft;
  `FormFields::discard_drafts` or `TextCommand::DiscardDraft` ends it."
- **`on_rejection`, same-kind:** the three pages of section 1 add
  `TextCommand::DiscardDraft` (and so `FormFields::discard_drafts`) to where
  a reported refusal ends.
- **`docs/contracts.md`:** section 2.2.
- **Guide:** `G/state.md`'s recipe (section 2.3); the testing chapter's
  paragraph at `G/testing.md:502-507` adds that the
  [settings-form recipe](crate::guide::state#a-settings-form) shows such a
  policy with Revert; the controls chapter's number-input paragraph
  (`G/controls.md:431`, "External changed values win over drafts") adds
  that an equal write keeps a draft and `:438-439` names `discard_drafts`;
  `G/getting-started-continued.md:24` ("coordinates errors and can focus the
  first one") adds "and discard every field's draft".
- **`docs/manual.md:436`:** "`TextCommand::DiscardDraft` discards a draft a
  field keeps" adds "and `FormFields::discard_drafts` every draft a form's
  fields keep, as a Revert does".
- **Diagram editor:** `main.rs`'s module comment (close admission counts the
  inspector's kept drafts; the close checks need `native`),
  `close_requested`'s comment (section 2.4).
- **Records:** TODO.md's `close_requested` entry (`TODO.md:119-127`) is
  removed; `plans/follow-ups.md` gains the command-action item
  ([P12-D3](#decisions)) under "Text and editing"; `docs/verification.md`'s
  native evidence gains P12's row (section 8).
- **Not changed:** the READMEs (none names `DiscardDraft` or `FormFields`),
  `docs/architecture.md`, the gallery's form pages (no Revert).

## 6. Migration manifest

P12 breaks nothing, so no consumer has to change; the taught texts and the
diagram editor change as section 5 and 2.4 say.

| Consumer | P12 |
|---|---|
| `R/field_status.rs`, `R/property_grid.rs` | the method; rustdoc (section 5) |
| `R/text_fields.rs`, `R/text_fields/number_inputs.rs`, `R/text_fields/inline_text.rs` | rustdoc only (section 5) |
| `E/diagram_editor/editor.rs`, `app.rs`, `main.rs`, `session.rs` | section 2.4: the inspector's form, `Shown::One { node, incarnation }`, the grid keyed by node and incarnation, the size editors' own keys; the session's close passage moves to `close_admission`, and its `Shown::One(start)` check (`session.rs:266-269`) matches the node |
| Other packaged examples and their sessions | unchanged: no Revert, no `FormFields` |
| Gallery (`apps/gallery/src/pages/controls.rs:1259`, `properties.rs:17`, `text.rs:394`) | unchanged: no Revert; [P12-D3](#decisions)'s alternative is not taken for native evidence (section 8) |
| Native checks, platform code | unchanged |
| `T/editing/form_drafts.rs` (new module of the `editing` area, `T/editing/main.rs`) | section 7's tests |
| `T/editing/draft_policy.rs` | unchanged; its policy-table test confirms 2.2 |
| `T/foundation/modules.rs` | unchanged (no new names) |
| Rustdoc and guide | section 5 |
| `docs/contracts.md`, `docs/manual.md`, `docs/verification.md` | sections 2.2, 5 and 8 |
| READMEs | unchanged |
| Diagnostics fixtures (`tests/diagnostics/ui/`) | none uses `FormFields` (`git grep`); the snapshots run in the gate |
| `plans/usability/task4-app` | unchanged: it uses no `FormFields` (`git grep`), so no fixture check |
| Frozen: U `r2-1-codex/work/src/main.rs:175-192`, `:250-254`; U `r2-1-claude/work/src/lib.rs:266-284`; the acceptance kit's author check (U `acceptance-r2-1-claude/work/tests/author.rs:68-93`) | not migrated |

## 7. Tests

- **New-behavior tests** each name the assertion that detects a wrong
  implementation and a compiling mutation that must make it fail.
- **Contract and compatibility tests** check rules P12 inherits or keeps.
- A button click moves focus (section 1), so the tests that need an
  editor to keep focus call the method from a component's
  `deliver_notifications`, as `T/editing/draft_policy.rs:2051-2075`'s
  `Discarder` sends its command: a `FormDiscarder` holding the form, a
  trigger signal and a cell recording the call's result.

### New-behavior tests

Headless, in `T/editing/form_drafts.rs`, except test 6:

1. **Revert discards what its equal write keeps.** One form: an
   `IntegerInput` bound to 7, `range(1, 99)`, in `Field::form`, with
   `on_rejection` writing its handle's status and `on_modified` its mark,
   holding a refused draft `150`; and a normalizing `TextField` holding a
   draft, then disabled (kept, not refused, rule 5), with `on_modified`.
   Two buttons: "Write" writes both applied values (equal), "Revert" writes
   them and calls `discard_drafts`, recording its result and the summary
   it reads right after the call. Named assertions: after Write, both
   drafts remain (the number field `invalid`, the text field's semantic
   value the draft) and `modified() == 2`: rule 3, the R2-2 reproduction;
   after Revert, the recorded result is `Ok(2)` and the summary read inside
   the callback still has `modified() == 2` (rule 2.1.8); after the click's
   settle, the number field is not `invalid`, the text field shows the
   bound text, both bindings are unchanged, `modified() == 0`, and the
   number field's status is `Normal` (its `on_rejection(None)` arrived).
   **Changed value, same form rebuilt:** the number field saves `8` (the
   binding 8, applied 7), then holds a refused `150` with status `Error`;
   Revert writes 7 and calls `discard_drafts`. Named assertions: the result
   is `Ok(2)`; after the settle the binding is 7, the field is not
   `invalid`, `modified() == 0`, and the status the test's `on_rejection`
   published is still `Error`: the write ended the draft before delivery,
   so the request answered `Unavailable` and no `None` came (rule 2.1.6);
   pressing Escape in the field afterwards delivers the pending `None`
   (status `Normal`). This pins the qualification the recipe relies on.
   Mutations: queue only fields whose status is an error, as
   `focus_first_error` selects (the text field keeps its draft,
   `modified() == 1`); clear each handle's mark in the call (the summary
   read in the callback is 0); return the count without queueing (both
   drafts remain).
2. **Reach and count, whatever the marks say.** The form of test 1 plus
   two visible, mounted fields whose marks are never set: an unchanged
   `IntegerInput` (no draft) and a normalizing `TextField` holding a draft
   with **no `on_modified` callback**; and three that are skipped: a field
   registered but never shown, a field whose `Field` uses `status_signal`
   (P6) on the handle's status, and a field whose control was hidden with
   `Runtime::set_visible` before the call. Revert is activated by
   `key_input` on the focused button (an input call), and before the flush
   the test hides test 1's number field's control. Named assertions: the
   result is `Ok(4)`; the flush succeeds (the unchanged editor answers
   `Unavailable`, not an error); test 1's text field's draft and the
   unreported draft are both discarded (the latter's semantic value is the
   bound text again); the hidden number field lost its draft to hiding
   (rule 7; its `on_modified(false)` arrives) and its request was dropped
   without error. Mutations: **select fields by their `modified` mark**
   (`Ok(2)`, and the unreported draft remains); count registered fields
   (7); queue hidden controls (5); skip only empty cells, checking
   visibility at delivery alone (5).
3. **Registration order and a failing target.** Field A: a `TextField` with
   a reported refusal whose `on_rejection` returns an error on `None`;
   field B, registered after A: the refused `IntegerInput` of test 1.
   Named assertions: the Revert click's settle returns A's error; A's draft
   is discarded and B's is still there; the next `settle` succeeds and B's
   draft is gone; the failed `None` is still pending on A (its next Escape
   reports it). Mutation: queue in reverse registration order (B is gone
   after the failing settle).
4. **Focus is inherited.** A standalone `InlineText` in a `Field` with an
   open, focused session holding a changed draft, registered second after
   an `IntegerInput` holding a refused draft; a `FormDiscarder` triggers the
   call. Named assertions: both drafts end, the session's draft is not
   saved, and focus is on the `InlineText`, as after Escape. The same with
   a row-mode `InlineText` in a `VirtualList` row's `Field`, as
   `discard_draft_in_a_row_leaves_focus_to_the_list`
   (`T/editing/draft_policy.rs:2082-2149`) builds it: focus is on the list.
   Mutation: request focus for the first recorded control, as
   `focus_first_error` does (focus lands on the `IntegerInput`).
5. **Another runtime's caller ([P12-D1](#decisions)).** Runtime A holds two
   forms: one whose mounted field holds a refused draft, one with a
   registered field that is not shown. A `FormDiscarder` mounted in
   runtime B calls `discard_drafts` on each. Named assertions: both results
   are `Err(ForeignRuntime)`; after A's flush the draft is still there.
   Mutation: check only the recorded controls, D2's wording (the unshown
   form returns `Ok(0)`).
6. **The diagram editor's close admission** (`E/diagram_editor/session.rs`,
   native, through `HeadlessApp` in a fresh directory): a clean document
   closes (`Close`); a focused inspector label draft is saved by admission
   before the policy (the label reads `Start!`; `Defer`, cancelled to
   `KeepOpen`; undone); a canvas rename is saved the same way (as the
   stand-in checked, `session.rs:533-543`); then with the document clean, a
   height of `5` typed and Enter (refused) makes `admit_close` `Defer` with
   the discard dialog open, and cancelling gives `KeepOpen` with the height
   field still `invalid`; selecting another node makes `admit_close`
   `Close`. **A → B → A without a flush:** back on node A with a refused
   height, the test calls `editor.select([B], [])` and `editor.select([A],
   [])` with no flush between, then settles. Named assertions: the height
   editor is a new mount (its `WidgetId` differs from the one before), it is
   not `invalid` and shows A's height, `modified() == 0`, and `admit_close`
   is `Close`; that is, no draft survives that the form does not count.
   Mutations: `close_requested` ignores the form (the refused draft
   closes: `Close`); `sync` does not forget the marks (`Defer` after
   selecting another node); the height `DecimalInput` without `on_modified`
   (`Close` for the refused height); **the grid keyed by node alone**, as
   today (after A → B → A the same height editor is still `invalid` while
   `modified() == 0`, the silent close Codex found).

### Contract and compatibility tests

- **Modality, inherited:** field A outside an open modal `Dialog` holding a
  refused draft, field B and the Revert button inside it. The click's settle
  returns `ModalBlocked`; A's draft remains (its request is spent); the
  next `settle` discards B's.
- **The draft policy is unchanged:** `the_policy_table_holds_for_every_built_in_editor`
  and the other `draft_policy.rs` tests pass unchanged after the matrix
  label's rewording.
- **Doctests:** `discard_drafts`'s example, the settings-form recipe (both
  run), and the rewritten paragraphs' existing examples; each imports
  exactly what it uses (R2-14's rule).
- **`focus_first_error` is unchanged** (`T/controls/field_status.rs`), and
  `FormFields`'s existing tests pass.
- **The diagram editor's session** passes with `--features png,svg,cpu`
  (the gate's command, native included) and with `--no-default-features
  --features cpu` (its documented headless command, `E/diagram_editor/main.rs:31-33`),
  where `close_admission` is compiled out.
- **P2's closure test** passes over the regenerated listings.

## 8. Scope, version and evidence

- **`api/` diff:** `pub fn pert_ui_runtime::FormFields::discard_drafts(&self,
  &mut pert_ui_runtime::EventContext) -> core::result::Result<usize,
  pert_ui_runtime::RuntimeError>`, at the root and prelude paths, in
  `api/pert-ui-runtime/default.txt` and `serde.txt` (where
  `focus_first_error` appears twice each, `default.txt:17456`, `:33243`). No
  facade, core, CPU or wgpu listing changes: the facade's listings do not
  repeat runtime methods.
- **Breaking:** no.
- **CHANGELOG** for 0.3.87, citing this checkpoint:
  - `Added (R2-2):` `FormFields::discard_drafts(&self, &mut EventContext)
    -> Result<usize, RuntimeError>`: queues `TextCommand::DiscardDraft` for
    every visible field control a form records, in registration order, and
    returns the number queued; `ForeignRuntime` for a caller of another
    runtime, queueing nothing. Retires collecting editor ids for a per-editor
    `request_text_command` loop as the way to end drafts from a Revert.
  - `Documentation (R2-2):` draft policy rule 3, the `DiscardDraft`
    paragraph and the matrix label in `docs/contracts.md`; an equal write
    keeps a draft on the `TextField`, `IntegerInput` and `InlineText` pages;
    `on_rejection(None)` also at `DiscardDraft` on the three `on_rejection`
    pages; `FormFields`, `Field::form`, `Property::form`,
    `FieldHandle::remove` and `set_modified`; the state chapter's
    "A settings form" (`combine`, `spawn`, Revert, a close policy over
    `App`'s state); the testing, controls and getting-started chapters;
    `docs/manual.md`.
  - `Changed:` the diagram editor counts the inspector's kept drafts in a
    `FormFields`, and its close policy asks with one (TODO.md entry
    resolved); leaving a node remounts its inspector grid, also when the
    node is selected again before the next update, so a cleared count never
    hides a surviving draft; its session checks close admission through
    `HeadlessApp::admit_close` with the `native` feature.
  - Follow-up recorded: a form discard from a command action (P12-D3).
- **Version:** 0.3.87 after P11's 0.3.86: the workspace version, the
  internal version requirements and `Cargo.lock`. If the order changes, the
  heading follows the actual version.
- **Gate,** the plan's public-API row: the workspace gate (formatting,
  Clippy, tests with section 7's tests, the five headless sessions, the
  workspace doctests, warning-free rustdoc, the diagnostics snapshots); the
  API listing gate, its diff reviewed against the CHANGELOG; the
  `diagram_editor` headless session in both configurations of section 7
  (the second because P12 adds `native`-gated code to the example); the
  mutations of section 7 recorded in the Progress entry; `git diff --check`;
  and Codex's implementation review. No fixture check (section 6), no
  dependency, feature or platform change, so no MSRV or packaging run.
- **Native evidence (Linux):**
  - **Hand check of the diagram editor's close with a refused draft:**
    `cargo run --locked -p pert-ui --example diagram_editor -- --window`;
    select a node, type `5` into Height and press Enter (the outline shows
    the refusal), close the window from its title bar: the discard dialog
    opens; Cancel keeps the window and the draft; Escape in the field, then
    close: the window closes without asking. Recorded with the platform in
    the Progress entry and `docs/verification.md`. It also adds to
    NATIVE-05's Linux evidence for the diagram editor's window.
  - **Revert discarding a refused draft natively: missing.** No native
    window has a form Revert: the recipe is a doctest, the diagram editor
    has none, and the gallery's form pages have none
    (`apps/gallery/src/pages/controls.rs:1259-1340`). The behavior is runtime
    code shared with the headless tests; the missing evidence is recorded
    in the Progress entry and `docs/verification.md`.
  - Windows and macOS: missing (NATIVE-01 to NATIVE-05).
- **Persistence, real time, backends:** no persisted schema
  (`Workspace::SCHEMA_VERSION` and the inspection schema unchanged), no
  live-data endpoint, no backend type.

## Corrections to the plan

- **`on_rejection(None)` is not after settling.** D2 and P12's test list
  say both reports come after settling. `on_rejection(None)` runs inside the
  command's delivery, in the event phase (`R/text_fields/normalize.rs:126-137`,
  called at `R/text_fields.rs:2383`), and only where a refusal was
  **reported** by a refused save, not merely shown; `on_modified(false)`
  comes after settling. Rule 2.1.6 and test 1 state it.
- **After a changed write, the command reports nothing.** D2's
  notifications hold for a draft the command ends. Where the Revert's write
  changed the value, the update ends the draft first and the command
  answers `Unavailable`, so no `on_rejection(None)` comes from it (rule
  2.1.6); the recipe clears its statuses itself.
- **A foreign recorded control cannot arise.** `Field::update` reads the
  handle's status before recording (`R/field_status.rs:637-652`), and that
  read fails across runtimes, so every recorded control is in the form's
  runtime. The real case is a caller of another runtime, which P12-D1
  checks at the form; the plan's test "a control recorded from another
  runtime" becomes test 5.
- **Hidden controls are skipped at the call**, narrowing D2's "each
  registered field whose `Field` has recorded a control": a hidden control
  holds no draft (rule 7), and `focus_first_error` skips it the same way
  (`R/field_status.rs:295-298`). A control hidden after the call is still
  dropped at delivery (test 2).
- **The diagram editor's test needs an `App` the session can start.** The
  session drives a `Runtime` (`E/diagram_editor/session.rs:121-131`); P12
  extracts `application(dir)` and replaces the session's admission stand-in
  (section 2.4 rule 4).
- **A removed inspector would leave a stale mark.** Feeding a `FormFields`
  from `on_modified` alone is not enough in the diagram editor: selecting
  another node removes the editors without a report (rule 10), so `sync`
  forgets the marks (section 2.4 rule 2).
- **The Revert hand check has no native window** (section 8): neither the
  recipe nor the diagram editor has a Revert.
- **The `on_rejection` pages omit `DiscardDraft`** (section 1), a same-kind
  gap the method relies on; fixed in P12.
- **Line numbers:** rule 3 is at `docs/contracts.md:1253-1256` and
  `request_text_command` at `R/runtime.rs:3267-3311` (the plan cites
  `:1250-1253` and `:3265-3299` at 26e6b5a).

## Findings outside this part

- None new. TODO.md entries touching P12's files, not resolved here:
  `E/diagram_editor/canvas.rs` `NodeView::paint` (2026-10-26), the renamed
  node's label painted under its editor; NATIVE-05 (2026-10-20), to which
  section 8's hand check adds Linux evidence for the diagram editor's
  window; the unused-imports entry (2026-10-28), whose rule P12's examples
  follow.

## Not in this part

- **A form discard from a command action** ([P12-D3](#decisions)).
- **A form-level status reset.** The recipe clears the statuses it owns
  with `FieldHandle::clear` (section 2.3, rule 2.1.6); statuses are the
  application's, and no run asked the form to reset them.
- **A Revert in the diagram editor's inspector or in the gallery.** Neither
  is requested; the native evidence is recorded as missing instead.
- **Clearing a removed field's mark in the library.** Making `FormFields`
  forget a mark when its `Field` unmounts would change `modified`'s meaning
  for fields fed without `Field::form`; the rustdoc says what to do
  (section 5).

## Decisions

- **P12-D1: where the runtime check sits.**
  - **Recommended: at the form.** The call fails with `ForeignRuntime` when
    `cx` belongs to another runtime than the form's summary signal, whether
    or not any field is shown. Every recorded control shares the form's
    runtime by construction (section 1), so this is D2's atomic admission
    with one comparison, and a misdirected call fails the same way before
    and after its fields mount.
  - **Alternative: each recorded control, D2's wording.** Equivalent while
    a field is shown; a form with nothing shown returns `Ok(0)` to a caller
    of another runtime, so the misuse surfaces only once something mounts.
- **P12-D2: the diagram editor with a kept inspector draft.**
  - **Recommended: ask with the existing discard dialog** (section 2.4
    rule 3), as for a dirty document: one prompt, and accepting is an
    informed discard. The window stays open unless the user accepts, which
    is what the TODO's "keep the window open, or ask" and the plan's
    "keeps the window open" require.
  - **Alternative: `KeepOpen` outright** with a status line. No dialog
    change, but the user cannot close without finding and fixing or
    escaping a draft in a tool window that may be collapsed, and the dirty
    document and the draft get two different answers.
- **P12-D3: command actions, which receive `&mut Runtime`.** A Revert bound
  to a shortcut or the command palette has no `EventContext`
  (`R/runtime/commands.rs:607-612`); a Revert button or `MenuItem::new` has
  one.
  - **Recommended: not in this part;** recorded in `plans/follow-ups.md`
    under "Text and editing", to be taken up when an application needs a
    Revert command. Round 2's runs and the recipe use buttons, and D2
    approved the `EventContext` form.
  - **Alternative: accept either receiver now,** through a sealed trait
    implemented by `EventContext` and `Runtime` that queues on the same
    queue (delivery at the next flush from a `Runtime`). It closes the gap
    without a second method, but adds a trait and a delivery rule no round
    2 run needed.

## Review record

- **Revision 1 (2026-10-28):** written from the code at a98fdb2 (P4
  committed), building on the approved P10 and P11 designs.
- **Revision 1, Codex review (2026-10-28, against a98fdb2):** needs
  revision, with one blocker, two should-fix points and a nit; the runtime
  admission check, the prelude closure, persistence, the P10/P11 ordering,
  the notification-timing correction, the reuse of the discard dialog and
  the recorded missing native Revert evidence were found to fit. Record
  `~/pert-ui-usability/codex-prompts/codex-design-P12.out`. Each point was
  checked against a98fdb2 before revision 2 applied it; none was rejected.
  1. **Blocker, clearing marks before reconciliation:** confirmed. The grid
     is keyed by node (`E/diagram_editor/app.rs:531`), `Dynamic` keeps a
     child under the same key with its drafts (`R/described.rs:121-124`),
     and an editor does not report a state it already delivered
     (`R/text_fields/modified.rs:37-38`), so A → B → A before a flush left
     a refused draft with a cleared mark. Revision 2 ties the clear to a
     certain removal: `Shown::One` carries an inspector incarnation that
     `sync` advances, with the clear, only when the target changes, and the
     grid is keyed by node and incarnation, so every clear coincides with a
     remount (section 2.4 rule 2). Test 6 adds A → B → A without a flush,
     with the node-only key as its mutation.
  2. **Should-fix, statuses on Revert with a changed value:** confirmed.
     Updates run before queued text commands (`R/runtime.rs:5865-5875`); a
     changed write discards the draft in the update
     (`R/text_fields/number_inputs.rs:1337-1342`, `R/text_fields.rs:2452-2458`),
     and the command then answers `Unavailable` before the refusal report
     (`number_inputs.rs:1877-1882`, `text_fields.rs:2374-2376`). Revision 2
     qualifies rule 2.1.6 and the rustdoc and contracts text (the reports
     are the editor's while it still holds the draft; otherwise the write's
     route applies), has the recipe clear its own statuses, and tests both
     the equal and the changed bound value in the recipe and in test 1.
  3. **Should-fix, unconditional targeting:** confirmed: test 2's eligible
     controls all had marks, so selecting by mark passed. Test 2 adds an
     unchanged editor and a draft without `on_modified`, expects `Ok(4)`
     and the unreported draft discarded, and names selection by the
     `modified` mark as its mutation.
  4. **Nit, provenance:** the claim that the touched files were unchanged
     since 26e6b5a was incomplete. Revision 2 keeps the a98fdb2 baseline
     and drops the inventory.
- **Revision 2, Codex recheck (2026-10-28):** every revision 1 point
  resolved; ready for approval with two nits: the recipe's `build` returned
  `Box<dyn Error>` into an initializer returning `AppError` (now
  `Result<Settings, RuntimeError>`, which `?` converts), and three
  citations named the wrong operation (the update path is
  `R/text_fields.rs:2452-2458`, `report.clear` is at `:2383`). Corrected by
  the author as Codex proposed. Approved by the author under the
  maintainer's delegation, with P12-D1 to P12-D3 as recommended.
