# Part 6f3 design checkpoint: controlled controls

Part 6f3 of [the fix plan](fix-plan.md#6f3-controlled-controls) adds public
API, so its shape is settled here before any code. Codex reviews it, then
the maintainer approves it. It closes round 1's F16
([the round 1 table](../../developer-usability.md)). The evidence is
triage [group C](../runs/triage/group-C.md) (r1-2-claude#4,
pilot-2-claude#3, merged finding 2).

Status: **revision 4**, ready for approval after Codex's first review and
three rechecks (record at the end). The maintainer chose the recommended
answers to D1-D4 on 2026-10-26, before that review. The review reopened D4, which
needs an exception to an AGENTS.md rule; the maintainer then confirmed
the exception and approved the AGENTS.md callback wording the same day
([decisions](#decisions-for-the-maintainer)). Both AGENTS.md changes are
made in the part's implementation. **Approved as a whole by the
maintainer on 2026-10-26.**

## What the testers built by hand

- **F16:** a task list keeps its records in the application's own
  `Vec`, and each row shows a checkbox and an editable title.
  `Checkbox::new` and `InlineText::new` bind only a `Signal`, and
  `ListItems::build` gets no state owner (`virtual_list.rs:58-85`). Both
  task-2 testers made a throwaway `Signal::sibling` per row and wrote the
  change back from the callback. That works for the checkbox, but the
  list describes every mounted row again on each source replacement and
  on each scroll or resize that mounts another row. Every description
  then hands the title a new signal, and `InlineText::reconcile` ends an
  open draft when its binding changes (`inline_text.rs:466-472`), so a
  change to any record, or a scroll, discards what the user is typing.
- **The library's own instances:**
  - the guide's editable-list recipe (`guide/layout.md:513-620`) teaches
    the per-row form for the checkbox and a hand-written retained `Title`
    component for the title, about forty lines with a `Notifier`, a
    `reconcile` and a `deliver_notifications`;
  - the inventory example needs the same retained shape, `ActiveCell`
    (`examples/inventory/app.rs:107-170`), for one checkbox per table cell,
    with the comment "A checkbox binds a signal, and `TableItems::cell`
    has no state owner".

The need in all three is the same: the value lives in the application's
model and reaches the row through its description; the control should
show that value and report the user's change, without a signal of its
own.

## 1. The ownership path

contracts.md's [ownership paths](../../../docs/contracts.md#ownership-paths)
(`contracts.md:529-537`) gain a fourth row:

| Path | Owner and behavior |
|---|---|
| Described value with a change callback (controlled controls) | The application's model. The value arrives in the control's description and changes only by a new description; the control reports each user change as a proposal through its callback and never changes what it shows on its own. |

- **Distinct from a parameter.** `Checkbox::for_parameter` is the
  authoritative path: a host, revisions, begin/commit gestures and remote
  authority. A controlled control has none of these: no host, no
  revision, no pending state. It suits data the application changes
  synchronously in its own model, which is the testers' case.
- **Distinct from a signal.** With `new`, the control writes its binding
  and then reports the write. With `controlled`, nothing is written by the
  control; the callback is the only effect of a user change.

## 2. Constructors

```rust
impl Checkbox {
    /// Creates a labeled control showing `checked`, which the application
    /// owns; each user toggle proposes the opposite value to `on_change`.
    pub fn controlled(
        checked: bool,
        label: impl Into<TextSource>,
        on_change: impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static,
    ) -> Self;
}

impl InlineText {
    /// Creates an editable label showing `text`, which the application
    /// owns; each save proposes the saved text to `on_save`.
    pub fn controlled(
        text: impl Into<String>,
        on_save: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static,
    ) -> Self;
}
```

- **The name (decision D1):**
  - **Recommended: `controlled`**, as the fix plan proposes. It names the
    ownership, is the established term for this pattern, and stays apart
    from the `new`/`bound`/`for_parameter` vocabulary, which
    [contracts.md](../../../docs/contracts.md#choice-sources) reserves for
    where choice controls take their options.
  - **Alternative: `with_value`**, by decision G1-13's first meaning of
    `with_` (a constructor taking a supplied part, as `Menu::with_state`).
    It reads as "a value instead of a binding" but does not say that the
    application decides the outcome.
- **Argument order:** the owned state leads, by the component
  conventions; the label follows as in `new`; the callback comes last, as
  in `Button::new`.
- **The callback is the existing one.** The constructor installs it as the
  control's `on_change` (`Checkbox`) or `on_save` (`InlineText`), with the
  same type. A later `.on_change(...)` or `.on_save(...)` replaces it, as
  ordinary property builders replace. There is no second callback for the
  same event.
- **The callbacks' contracts change by path.** Today they describe a
  successful write unconditionally: `on_change` "reports each successful
  user write or remote proposal submission" (`choice_controls.rs:586-589`),
  `on_save` reports "each save that wrote the binding"
  (`inline_text.rs:317-322`), and contracts.md's "User-write callbacks"
  (`contracts.md:491-495`) and AGENTS.md (`AGENTS.md:148-149`) make the
  same general claim. Each is qualified: on a controlled control the
  callback is the user's proposal, called where the other paths write,
  and the control has written nothing. AGENTS.md's sentence becomes
  "User-write callbacks report successful local writes, proposal
  submission or a controlled control's proposal, never remote
  acceptance", which needs the maintainer's consent as a change to the
  instruction file. The manual's callback wording, if it repeats the
  claim, follows. The CHANGELOG's "Changed" entry names the two methods'
  qualified contracts.
- **`Toggle`** is generated by the same macro as `Checkbox`
  (`choice_controls.rs:499-690`) and gets `Toggle::controlled` with the
  same rules, by the same-kind rule.
- **Every other builder** (`label`, icon slots, `enabled`, `disabled`;
  `placeholder`, `style`, `normalize`, `in_row`, `on_rejection`) works as
  with `new`.
- **The value type** is a fixed value per description, not a binding (see
  [not in this part](#not-in-this-part) for a reactive form).

## 3. Presentation, refusal and timing

These rules go into a new contracts.md subsection, "Controlled controls",
under "Data and edit ownership", before any code, as the fix plan
requires. The `User-write callbacks` section (`contracts.md:489-525`)
points to it.

### Presentation (decision D2)

- **Recommended: the control shows the described value only.** A user
  toggle or save does not change what the control shows; a new description
  with a different value does.
  - **Synchronous model, next flush.** When the callback writes the
    application's model and that write describes the control again, as a
    list's source replacement or a `Dynamic` does, the next successful
    flush reconciles the new description in its settle loop, before
    layout and paint (`runtime.rs:5767-5805`: `apply_requested_children`
    runs before `settle_layout`); no second flush is needed. A raw
    `Runtime` input call may flush pending work before dispatching
    (`runtime/pointer.rs:449-451`; `text_input` always does,
    `runtime.rs:5124`) and settles pending notifications afterwards
    (`runtime/notify.rs:350-355`), but it does not promise to settle the
    redescription its own callback caused: that is guaranteed by the next
    successful flush (`runtime/pointer.rs:308-311`).
    `testing::Driver::click` settles. `Runtime::paint` refuses while
    presentation work is pending (`runtime/compose.rs:32-61`), and the
    desktop host flushes before every paint
    (`platform/mod.rs:458-460`), so no presented frame shows the old
    value after the model accepted the new one. A test establishes this for a
    `VirtualList` row, a `Table` cell and a `Dynamic` child with a raw
    input call followed by exactly one flush (see [tests](#6-tests)).
  - **Asynchronous model.** An application that confirms later shows the
    old value until it describes the new one. It shows a pending state
    from its own model if it wants one, for example by describing the
    title it is saving.
  - **Inspection** reports the described value: `checked` and the
    semantic value for `Checkbox`, the resting text for `InlineText`.
- **Alternative: optimistic until the next description.** The control
  shows the proposed value until its next description, whose value then
  wins. A refusal that describes nothing new would leave the proposed
  value on screen indefinitely, since a list describes its rows only when
  its source or mounted range changes, so the control would disagree with
  the model with no event to correct it. The recommendation avoids that
  state entirely.

### Refusal

- **The application refuses by not changing the value it describes.**
  There is nothing to roll back and nothing is replayed: the control
  already shows the model's value.
- **`InlineText`:** a save ends the session before the callback runs, as
  with `new`, so a refused save closes the editor and shows the described
  text; the typed text is gone. Refusal that keeps the draft open for
  correction is decision D3:
  - **Recommended: the normalizer.** `InlineText::normalize` already
    refuses a draft with an `InputRejection`, keeps the session open with
    the error outline and reports through `on_rejection`
    (`inline_text.rs:173-199`). It runs before the callback, with the
    controlled form exactly as with `new`. Its contract today requires it
    to be deterministic, bounded, idempotent and free of side effects,
    binding writes and I/O (`text_fields.rs:346-349`, which
    `InlineText::normalize` repeats). A check against the model, such as a
    unique name among the other rows, reads state that can change between
    saves, so "deterministic" would forbid it. This recommendation
    therefore widens `InlineText`'s normalizer contract:
    - it may read application state, never write it;
    - it is deterministic and idempotent for a given draft and unchanged
      observed application state, so a saved value stays as it is on the
      next save while the state is unchanged, and a refusal may turn into
      acceptance (or back) when the state changes;
    - reading the model creates no dependency: nothing re-runs the
      normalizer when the model changes. A refused draft is checked again
      at the user's next save (Enter or focus loss), not automatically.

    The rustdoc gets a model-dependent example (refusing a name another
    row already has, then accepting it after that row is renamed), and the
    tests cover it. The widening is limited to `InlineText` by this part's
    scope, not by a difference in when the normalizers run: `TextField`
    also normalizes only when it saves (`text_fields.rs:167-170`,
    `:1817-1818`). `TextField` keeps its wording until it has a controlled
    form; the narrowed follow-up records that.
  - **Alternative: a refusing callback.** `controlled` takes an
    `FnMut(&mut EventContext, &str) -> Result<Option<InputRejection>,
    RuntimeError>`, and `Some` keeps the draft open as a normalizer
    refusal does. It gives model checks one place, but the controlled
    callback would then differ in type from `on_save`, and a later
    `.on_save(...)` would have to replace it with a callback that cannot
    refuse.
- **A described value the control cannot show** (decision D4):
  - A `bool` is always valid.
  - Text the editor does not accept (`TextField::check` fails: over the
    16 KiB limit or not single-line) is, **recommended**, refused when it
    is adopted, at mount or at reconciliation, exactly as a refused bound
    value is today (`inline_text.rs:546-552`): the label keeps its previous
    text (empty if it never had a valid one), draws the error outline,
    reports `binding_error` in inspection, marks its semantics invalid,
    ends an open session unsaved and cannot begin editing. The next
    acceptable description ends the refusal. The runtime does not fail.
  - **This recommendation is an exception to a governing rule.**
    AGENTS.md says "Validate fixed descriptions before adoption where
    their validity is locally knowable" (`AGENTS.md:151-152`), and the
    text's validity is locally knowable; receiving another fixed
    description later does not make the source reactive. The public hook
    for that rule is `Component::check_description`
    (`runtime.rs:1079`). Adopting the recommendation therefore needs the
    maintainer's consent to an explicit exception in AGENTS.md and
    contracts.md ("Component redescription"), worded for values that
    arrive from application data in a description, such as a controlled
    control's value: they are refused at adoption and the last valid
    presentation is kept, as for reactive data. AGENTS.md's sentence
    becomes: "Validate fixed descriptions before adoption where their
    validity is locally knowable, except a value a description carries
    from application data, such as a controlled control's value, which is
    refused at adoption with the last valid presentation kept; validate
    reactive data when observed and preserve the last valid presentation
    on failure." The reason is the
    collection case: one record with a line break in its title would
    otherwise fail every flush in which the list describes its rows, and
    the whole list would stop updating.
  - **Alternative (the rule as it stands):** `check_description` rejects
    such a description up front, so the mounting or reconciling call
    fails, and the application must keep its titles single-line and within
    the limit before describing them, for example by checking with
    `TextField::check` when it stores them.

### Callback timing

- **The same moments as with `new`.** `Checkbox::controlled` calls
  `on_change` where `new` writes and then calls it today
  (`choice_controls.rs:218-226`): a matched primary release inside, a
  matched unmodified Enter/Space release. (`SemanticAction::Toggle` only
  describes the control; nothing executes it.) `InlineText::controlled` calls `on_save` at Enter and when focus leaves
  the label with a changed draft, after the normalizer
  (`inline_text.rs:410-460`).
- **What is proposed:**
  - `Checkbox`: the negation of the value shown. Two activations before
    the next description therefore propose the same value twice; each
    calls the callback.
  - `InlineText`: the normalized draft, only when it differs from the
    described text. An unchanged draft calls nothing, as with `new`.
- **What never calls it:** a new description, Escape, hiding, removal,
  revoked permission, a refused draft, an inspection read. Reconciliation
  publishes nothing, by the contract in
  [component redescription](../../../docs/contracts.md#component-redescription).
- **Errors:** a callback error propagates from the input call, as today;
  there is nothing to roll back. `InlineText` keeps its existing rule that
  a failing rejection report cannot swallow a save report.

## 4. Redescription

- **Same value:** nothing changes. A pending activation stays armed, and an
  open `InlineText` draft stays open. This is F16's fix: a list may
  describe its rows as often as it likes without disturbing an edit.
- **Changed value:**
  - `Checkbox`: the new value is shown at once (reconciliation sets it,
    with a paint invalidation). A **pending activation is cancelled**, as
    a parameter's revision change cancels one (`choice_controls.rs:305-312`):
    a press made while the box showed one value must not release into a
    proposal against another. This is new for this path; with `new`, an
    external signal write during a press keeps the activation.
  - `InlineText`: the new text wins over an open draft and ends the
    session unsaved, as an application write to the binding does with
    `new` (`inline_text.rs:553-558`).
- **Changing the path** between `new`, `controlled` and `for_parameter`
  under the same key is a change of binding: a pending activation is
  cancelled, and an open draft ends unsaved, as a different signal does
  today.
- **The callback** follows the new description, including the
  constructor's callback: every description of a controlled control
  carries one, so it is replaced each time.
- **Focus, hover and the row mode** follow the existing rules.

## 5. Documentation and consumers

- **contracts.md:** the ownership row (section 1), the "Controlled
  controls" subsection (section 3) and a sentence in "User-write
  callbacks" and "Component redescription" pointing to it.
- **Rustdoc:**
  - `Checkbox`/`Toggle` type docs: "Binds either a signal or a parameter"
    becomes three paths; `controlled` gets a doctest with refusal (the
    callback ignores the change and the box stays unchecked) and the
    applied case through `Dynamic`.
  - `InlineText` type docs: "bound to a UI-local `Signal<String>`" becomes
    both paths; `controlled` gets a doctest in which a redescription with
    the same text keeps the draft open.
  - `ListItems::build` (`virtual_list.rs:58-85`) and `TableItems::cell`
    (`table.rs:259-267`): the controlled form becomes the first way, for
    `Checkbox`, `Toggle` and `InlineText`; the per-row and retained forms
    stay for other controls, with their costs.
- **Guide recipe** (`guide/layout.md:476-750`): migrated through the facade
  only.
  - The checkbox becomes `Checkbox::controlled(task.done, "Done", ...)`.
  - The retained `Title` component is removed; the title becomes
    `InlineText::controlled(task.title.clone(), ...).in_row(true)`, keyed
    `"title"`. `on_activate` then finds it at `[key, "title"]` instead of
    `[key, "title", "editor"]`.
  - The "two ways" list becomes the controlled form, with one sentence on
    the retained form for controls without one.
  - The existing assertions stay: the draft survives a change to another
    record, the checkbox writes the record, and the press, double-click
    and filter steps behave as before.
- **Inventory example:** `ActiveCell` is replaced by
  `Checkbox::controlled` in `TableItems::cell`. Its headless session
  (`--items 2000`) is part of the gate.
- **Follow-ups** (`plans/follow-ups.md:60-65`): "Controlled controls" is
  narrowed to the controls F16 does not cover (`RadioGroup`, `Select`,
  `Slider` and the number inputs, `TextField`), keeping its condition that
  each is considered with its refusal, presentation and timing.
- **App-author slice 9** (`plans/app-author-improvements.md:388`): F16
  leaves the input list, as the fix plan's records section says.

## 6. Tests

The tests live in the facade's area binaries (controls and
collections).

The tests below are of two kinds, and each is marked as one in the code:

- **New-behavior tests** cover the transitions only the controlled path
  has: proposing instead of writing, showing the described value after a
  refused change, keeping a draft across a same-value description, and
  cancelling an armed press on a changed value. Each names the assertion
  that detects the removed implementation, and is checked with a
  compiling behavioral mutation that replaces that step with the `new`
  behavior (writing a signal, keeping the shown value after a toggle,
  ending the draft on every description, keeping the press); the named
  assertion must fail. A test that fails only because `controlled` no
  longer compiles is not evidence.
- **Contract and integration tests** check rules the controlled path
  shares with `new` or gets from existing machinery: refusing a described
  text at adoption (which reuses the binding refusal,
  `inline_text.rs:546-552`), the next-flush redescription (which reuses
  the settle loop, `runtime.rs:5767-5805`), callback replacement,
  normalizer refusal (including the model-dependent example of the
  widened contract), unchanged saves and Escape calling nothing, callback
  errors not replayed, `BeginEdit` and `in_row`. They are not compared
  with the `new` behavior. Where one guards a contract the controlled
  path could break on its own, such as refusing at adoption, it is checked
  with a compiling mutation that violates that contract (adopting the
  invalid text), not one that reproduces `new`.

- **`Checkbox::controlled` and `Toggle::controlled`:**
  - a click and a Space release call the callback with the negated value;
  - refusal: with a callback that changes nothing, the box shows the
    described value after the flush, in inspection and semantics
    (detects an optimistic shown value);
  - repeated refused activations each propose the same negated value;
  - applied: inside a `Dynamic` over the model, a raw click followed by
    exactly one flush shows the new value;
  - a description with the same value keeps an armed press, whose release
    then proposes;
  - a description with a changed value cancels an armed press, and the
    release proposes nothing;
  - a redescription with a different callback: the next activation calls
    only the new one;
  - a callback error propagates from the input call; the shown value is
    unchanged and the callback is not called again by a later flush;
  - switching the same key between `new` and `controlled` cancels an
    armed press.
- **`InlineText::controlled`:**
  - Enter and focus loss with a changed draft call the callback with the
    normalized text; an unchanged draft and Escape call nothing;
  - refusal by not applying: the label shows the described text after the
    save;
  - normalizer refusal: the draft stays open with the error outline, and
    the callback is not called;
  - model-dependent normalizer refusal: refused while another row has the
    name; after that row is renamed, the next Enter saves;
  - a described text the editor does not accept is refused (inspection's
    `binding_error`, invalid semantics, `BeginEdit` answers
    `Unavailable`); described while a session is open, it ends the
    session unsaved; the next valid text ends the refusal and restores
    valid semantics;
  - a description with the same text keeps an open draft (detects ending
    the draft on every description); a changed text ends it unsaved;
  - switching the same key between `new` and `controlled` while editing
    ends the draft unsaved;
  - a redescription with a different callback: the next save calls only
    the new one; a callback error propagates from the input call and is
    not replayed;
  - `BeginEdit` and `in_row` behave as with `new` (one test each, as
    compatibility checks).
- **F16 in a collection:** a `VirtualList` whose rows use both controlled
  controls over a `Signal<Vec<_>>`. While a title is being edited, a change
  to another record and a scroll that mounts another row leave the draft
  open; Enter applies it to the record; the row shows the new title after
  exactly one flush. Under a `ListFilter`, a query change that keeps the
  edited row keeps its draft, and one that removes the row ends it
  unsaved with no callback. The same for a `Table` cell checkbox, the
  inventory case.
- **Compatibility checks** (pass with or without the change): `new` and
  `for_parameter` behave as before, including a signal write during a
  press keeping the activation.

## 7. Scope, versions and evidence

- **`api/` diff:** `Checkbox::controlled`, `Toggle::controlled`,
  `InlineText::controlled`; no other public item changes.
- **Breaking:** no.
- **CHANGELOG:** "Added" for 0.3.70 names the three constructors and the
  ownership path; "Changed" names the qualified contracts of `on_change`
  (`Checkbox`, `Toggle`), `InlineText::on_save` and
  `InlineText::normalize`; "Documentation" covers `ListItems::build`,
  `TableItems::cell`, the recipe and the inventory example.
- **Gate:** the public-API row: the workspace checks, including the
  inventory headless session, the API listing, the diagnostics snapshots
  and focused contract tests. The guide doctest covers the recipe.
- **TODO.md:** no entry touches `Checkbox`, `Toggle`, `InlineText`,
  `ListItems` or the inventory example's `ActiveCell` today; none is
  planned.

## Not in this part

- **A reactive controlled value** (`controlled(ReadSignal<bool>, ...)`):
  a derived value, such as a "select all" box over every row, would
  follow a read-only signal without being described again. `Dynamic`
  already covers that by describing the control on each change, and the
  form can be added later without breaking the fixed one. No tester
  needed it.
- **Controlled forms of other controls:** the narrowed follow-up.

## Decisions for the maintainer

- **D1:** the constructor's name.
  - **Recommended:** `controlled`.
  - **Alternative:** `with_value`.
- **D2:** what the control shows between the user's change and the next
  description.
  - **Recommended:** the described value only.
  - **Alternative:** the proposed value until the next description.
- **D3:** how `InlineText::controlled` refuses a save while keeping the
  draft open.
  - **Recommended:** the existing normalizer, with `InlineText`'s
    normalizer contract clarified to allow reading application state;
    the callback cannot refuse.
  - **Alternative:** a callback returning `Option<InputRejection>`.
- **D4:** a described text the editor does not accept. Reopened by the
  Codex review: the recommendation needs an exception to AGENTS.md's
  validation rule.
  - **Recommended:** refused at adoption like a refused binding value;
    the runtime continues. Requires an explicit AGENTS.md and contracts.md
    exception for application data carried in a description.
  - **Alternative:** `check_description` fails the mounting or reconciling
    call, as the rule stands.
  - **Chosen (2026-10-26, after the review):** the recommendation, with
    the exception.
- **Instruction-file wording (callbacks):** AGENTS.md's user-write
  callback sentence gains "or a controlled control's proposal"
  (section 2). **Approved** on 2026-10-26.

## Review record

- **Revision 1, Codex review (2026-10-26):** needs revision, with five
  should-fix points and a nit. The ownership path, the simplification of
  `Title` and `ActiveCell`, `Toggle`'s inclusion and deferring a reactive
  value were found sound. Revision 2 addresses each point:
  1. **Timing:** the promise is the next successful flush, not the flush
     that delivered the input, since a raw input call does not promise to
     settle the redescription its callback caused; the test is a raw input
     followed by exactly one flush (section 3).
  2. **D4:** framed as an explicit exception to AGENTS.md's validation
     rule, needing the maintainer's consent and an AGENTS.md and
     contracts.md change; D4 is reopened.
  3. **Callback contracts:** `on_change`, `on_save`, contracts.md and
     AGENTS.md are qualified by ownership path, with a CHANGELOG "Changed"
     entry (section 2).
  4. **D3:** the incorrect distinction from `TextField` is removed; the
     widening is justified by scope, determinism and idempotence are
     defined for unchanged observed state, and model reads create no
     dependency, with an example and a test (section 3).
  5. **Tests:** same-value preservation, repeated refusals, callback
     replacement and errors, path switches while editing, filter
     retention and removal, refusal during a session and semantic
     recovery are added, with the rule for which assertion detects a
     revert (section 6).
  6. **Link:** fixed.
- **Revision 2, Codex recheck (2026-10-26):** needs revision, with two
  should-fix points. Points 2, 3, 4 and 6 of the first review were
  resolved, and no further product or API decision was found to be
  needed. Revision 3 addresses each point:
  1. **Timing wording:** a raw input call may flush before dispatch and
     settles pending notifications after it, but only the next successful
     flush is promised to settle its callback's redescription; painting is
     refused while work is pending and the desktop host flushes before
     painting (section 3).
  2. **The revert rule:** tests are split into new-behavior tests, each
     with a named assertion and a compiling behavioral mutation, and
     inherited-behavior and contract tests, which need not fail against
     the `new` behavior (section 6).
- **Revision 3, Codex second recheck (2026-10-26):** needs revision, with
  one should-fix point. The timing point was resolved, the earlier points
  stay closed, and the D4 exception, callback wording and normalizer
  contract were found sufficiently specified.
  1. **The revert rule:** refusal at adoption and the next-flush
     redescription reuse existing behavior, so substituting the `new`
     behavior need not fail them. Revision 4 moves them to the contract
     and integration tests, keeps the `new`-behavior mutation for the
     controlled-only transitions, and checks a contract the controlled
     path could break on its own with a mutation that violates it
     (section 6).
- **Revision 4, Codex third recheck (2026-10-26):** ready for approval, no
  findings. The revert rule is resolved, the earlier points stay closed,
  and no further decision is needed for the implementation.
