# Part L4b design checkpoint: one draft policy

Part L4b of [the layering plan](layering-plan.md#part-l4-behavior-unifications-0374-and-0375)
(N5 in the [complexity inventory](complexity-inventory.md#needs-a-decision))
changes behavior, so its rules are settled here before any code. Codex
reviews it, then the maintainer approves it. Every reference to code and
documentation is to eea2daa (after L1 and its review fixes; revision 1
cited 6360b00). L4a, implemented as 0.3.72 at 4c47c9a, changes none of the
code this part cites, so every code reference holds at 2a406a4 too; both
parts edit contracts.md and the guide, in different sections.

Status: **revision 4, approved 2026-10-27** with the recommended answers to
D1 to D4, by the author together with Codex's review of revision 1 and its
rechecks of revisions 2 to 4 (the last: ready for approval), under the
maintainer's delegation of 2026-10-26. The last recheck's nit (the
reporter's presentation mechanics) is applied in section 2 as Codex
specified it.

## What users built by hand, and what the library says

- **Testers.** F4 (round 1): a focused editor's draft was neither saved nor
  cancelled before `App::on_close`, so the close policy saw no change and an
  accepted close lost the typed text; a tester called `Runtime::clear_focus`
  in `on_close` (`developer-usability.md:415`). Part 6f1 fixed the focused
  editor. F6 (round 1, two runs): the number inputs' own refusals did not
  reach `on_rejection`, "and the fields have no `on_modified`"
  (`developer-usability.md:417`). Part 6d2 fixed the first half; the second
  is still true, so a close policy cannot see a number draft the field kept.
- **The rules are restated per editor in four places**, each a list rather
  than a rule: contracts.md's permissions paragraph (`contracts.md:350-357`),
  its close list (`:1043-1051`), `App::on_close`'s step 3
  (`crates/pert-ui/src/app.rs:531-551`) and the controls chapter
  (`guide/controls.md:654-658`). The close lists omit `InlineText`, the
  inline number rows, `InlineSlider` and `SearchField`.
- **The documentation contradicts the code** for one transition, a disabled
  number field:
  - `IntegerInput::session` and `DecimalInput::session` promise that
    disabling "releases focus as cancelled input, so the draft stays unsaved
    and the edit open" (`number_inputs.rs:2488-2495`), and so do contracts.md's
    editor sessions (`contracts.md:1689-1693`);
  - `ColorPicker::enabled` says revoking it "disables the text fields, which
    keep a draft without saving it" (`color_picker.rs:480-486`), and the
    picker's channel fields are `IntegerInput`s (`color_picker.rs:682-704`);
  - but the code discards the typed draft (`number_inputs.rs:1211-1225`,
    `:1372-1376`), as `IntegerInput::enabled` says (`:2093-2101`). Inside one
    `ColorPicker`, revoking the permission keeps the hex field's draft and
    drops a channel field's.

## 1. Today's behavior, from the code

Paths below are under `crates/pert-ui-runtime/src/` unless stated:
**TF** `text_fields.rs`, **SF** `text_fields/search.rs`, **IT**
`text_fields/inline_text.rs`, **DE** `text_fields/draft.rs`, **NI**
`text_fields/number_inputs.rs`, **IF** `text_fields/inline.rs`, **NC**
`numeric_controls.rs`, **TE** `table/editing.rs`, **TB** `table.rs`, **RT**
`runtime.rs`, **TR** `runtime/tree.rs`, **PT** `runtime/pointer.rs`; **CL**
is `crates/pert-ui/src/app/close.rs` and **C** is `docs/contracts.md`.

How the runtime delivers each event, which every row below inherits:

- **Enablement revoked:** the component's update reads the permission. A
  component that stops being focusable gets `cancel_input`, not
  `focus_lost`, unless its cancellation marker is already set, whether or
  not it holds focus, and loses focus if it held it (RT:6246-6265 for a
  bound permission, RT:6376-6403 for a fixed denial applied by
  redescription). Any delivered pointer event other than Leave clears the
  marker (PT:594-596), so an unfocused field the pointer moved over receives
  `cancel_input` too. A fixed denial takes effect in `reconcile`, so focus
  stays until the next flush, and a focus loss in between still reaches
  `focus_lost`.
- **Editability revoked:** the update reads it and calls
  `UpdateContext::cancel_input`; focus stays (TF:2108-2128).
- **Focus loss:** `focus_lost` on the focused component, then
  `focus_within_changed(false)` on its ancestors, which is where in-place
  owners decide (RT:5461-5473).
- **Hiding:** focus is cleared, then `cancel_input`, then
  `visibility_changed(false)` (TR:1527-1559).
- **Removal:** focus is cleared, then `cancel_input` and `unmounted`; the
  component is dropped (TR:1643-1663). A removed component's pending
  notifications are not delivered (C:784-785).
- **Text commands without focus:** `Runtime::text_command` delivers to a
  visible, unblocked target whether or not it holds focus (RT:5594-5610),
  and so does `EventContext::request_text_command`, in the event phase of a
  flush (RT:3245-3263, :6321). Both clear the target's cancellation marker
  (RT:5623). An unfocused editor can therefore acquire a draft.
- **Close admission** (part 6f1): `clear_focus` in each affected window, so
  the focused editor's focus-loss rule runs, then one flush when no window is
  in error backoff (CL:43-90). Admission touches no unfocused editor; an
  accepted close then removes every editor in the closing windows.

### The matrix, first half

| Editor and ownership | Validity failure on a save attempt | Enablement revoked | Editability revoked | Focus loss |
|---|---|---|---|---|
| 1. `TextField`/`TextArea`, signal, plain | No save step: each edit writes (TF:1624-1636) | Nothing unsaved; composition dropped (TF:815-820) | Focus kept, edits refused (TF:2108-2128) | Composition dropped; text already written (TF:782-797) |
| 2. Same, draft mode (normalizer or length limit) | Draft kept with the outline; `on_rejection` (TF:1838-1849) | Draft kept unsaved (TF:2064-2085, :815-820); no save while disabled (TF:1795-1797) | Draft kept; save refused (TF:1816-1828) | Saves a changed draft its rules accept, else keeps it (TF:788-794, :1794-1884) |
| 3. `TextField`, session | As 2; reports `Refused` (TF:1838-1846) | As 2; nothing reported, edit open (C:1689-1693), except that a fixed denial followed by a focus loss before the next flush reports `Refused` (TF:1795-1797, :1680-1685) | `Refused` on Enter or Tab (TF:1816-1828) | `FocusLeft` with the outcome (TF:790-791) |
| 4. `SearchField`, signal | No save step | Pending debounced query discarded, even when due (SF:517-523, :464-477, :287-293) | Pending query kept unwritten (SF:294-298) | Pending query written; when read-only it stays pending after focus left (SF:687-694, :294-298). An unfocused field takes text commands and arms the debounce (SF:640-647, :317-335) |
| 5. `InlineText`, signal | Normalizer refusal keeps the session open with the outline, also on focus loss (IT:567-582) | Session ends unsaved (IT:700-711, :653-658) | No such permission (IT:240-262, C:357) | Saves (IT:876-889; the child keeps the session, DE:889-895) |
| 6. `InlineText`, controlled | As 5 | As 5 | As 5 | Proposes through `on_save`; writes nothing (IT:583-610) |
| 7. `DraftEditor`, custom owner | The owner's (the diagram editor keeps an invalid rename open on Enter and discards it on focus loss, `examples/diagram_editor/canvas.rs:150-153`) | No permission of its own (DE:617-643) | The owner's (`begin_read_only`, DE:298-313) | Session stays; the owner decides in `focus_within_changed` (DE:889-895, :468-476) |
| 8. `IntegerInput`/`DecimalInput`, signal | Draft kept with the outline; `on_rejection` (NI:1277-1311) | **Draft discarded** (NI:1211-1225, :1372-1376); an unfocused kept draft is also discarded by the `cancel_input` that follows (NI:2427-2440) | Draft kept, save refused (NI:1251-1253); a held stepper press is cancelled (NI:1218-1224) | Saves a valid draft, keeps an invalid one (NI:2458-2459); a held stepper press is cancelled (NI:2453-2455), its steps already saved (NI:1023-1039) |
| 9. Same, session | As 8; `Refused` (NI:1288-1295) | **Draft discarded**, nothing reported, edit open (NI:1211-1225; contradicts NI:2488-2495) | `Refused`; a held stepper press **discards the stepped draft** (NI:1218-1224) | `FocusLeft` with the outcome (NI:2456-2457); during a held stepper press the stepped draft is **discarded and nothing reported** (NI:2453-2455) |
| 10. Same, parameter | See [parameter outcomes](#parameter-outcomes) | Cancelled with any parameter gesture (NI:1211-1225) | Kept, commit refused (NI:1251-1253) | Cancelled (NI:2453-2455) |
| 11. `InlineInteger`/`InlineDecimal` text mode, signal | Text mode stays with the outline, also after focus left (NI:1251-1281) | Text mode ends unsaved (NI:1211-1225, :1416-1425) | **The open text mode stays editable**: read-only is fixed when it begins (IF:213-214) and revocation locks only the resting editor (NI:1008-1012); the save is refused | Saves (NI:2406-2414) |
| 12. Same, parameter | See [parameter outcomes](#parameter-outcomes) | Cancelled | As 11 | Cancelled (NI:2453-2455) |
| 13. `InlineSlider` text mode, parameter | See [parameter outcomes](#parameter-outcomes) | Edit cancelled (NC:649-654, :1107-1127) | No such permission; a parameter change that invalidates the open edit ends text mode (NC:683-692) | Cancelled (NC:1526-1537) |
| 14. `Table` cell edit (rows 3 and 9 through a session) | `Refused` keeps the edit open (TE:333-335); after Enter focus stays in the editor | Edit stays open until Escape on the body (TB:1730-1733, TE:281-290) | `Refused`, edit open | A terminal outcome (`Saved`, `Unchanged`) closes the edit; `Refused` keeps it open and unfocused, and activating the cell focuses the editor again with its draft (TE:223-233, `table_editing.rs:545`); neither takes focus back (TE:336-346) |

### The matrix, second half

| Editor and ownership | Hiding | Removal | Close admission | Application write during a draft |
|---|---|---|---|---|
| 1 | Composition dropped | Dropped | As focus loss | Wins; selection and undo reset (TF:2415-2437, :2357-2369) |
| 2 | Draft discarded (TF:808-814) | Dropped unsaved | Focused: as focus loss. A draft kept by an unfocused field is untouched and observable through `on_modified` | Wins, discarding the draft, also when unobserved at the save (TF:1801-1815) |
| 3 | Discarded; nothing reported (C:1689) | Nothing reported | `FocusLeft`, delivered to the host in admission's flush (CL:75-90) | The next save reports `Unchanged` (TF:1801-1815) |
| 4 | Pending query discarded (SF:707-711) | Dropped | Focused: pending query written. An unfocused field's pending query (from a text command) is untouched | Wins over the pending query (SF:524-531, :299-305) |
| 5 | Discarded (IT:896-902) | Dropped | Saves (its focus-within change, RT:5473) | Wins and ends the session (IT:727-731, :556-565) |
| 6 | As 5 | As 5 | Proposes; a callback that writes the model is settled by admission's flush | A changed described text ends the session (IT:635-645) |
| 7 | Composition and gesture end; the session stays (DE:495; `editing/draft_editor.rs:985`) | Unmounting ends the session (DE:668-676, :755-757) | The owner's | The owner's (`replace`, DE:315-351) |
| 8 | Cancelled (NI:2427-2440) | Dropped | As focus loss; a kept invalid or read-only draft is **not observable**: no `on_modified` (F6) | Wins (NI:1157-1186) |
| 9 | As 8; nothing reported | Nothing reported | `FocusLeft`, but none during a held stepper press | The next commit reports `Unchanged` (NI:1249) |
| 10 | Cancelled | Cancelled; edit ownership is released (RT:2573-2575) | Cancelled (`window_close.rs:96`) | A revision change discards it (NI:1226-1232) |
| 11 | Cancelled (NI:2416-2420) | Dropped | Saves; a refused text mode stays, not observable | Wins and ends text mode (NI:1178-1183) |
| 12 | Cancelled | Cancelled | Cancelled | Discarded (NI:1226-1232) |
| 13 | Cancelled (NC:1107-1127) | Cancelled | Cancelled | A conflicting change ends text mode (NC:683-692) |
| 14 | The table hidden: the edit closes quietly (TB:2790-2795, TE:160-172) | A row that leaves closes the edit (TE:300-312) | As focus loss, by outcome: a terminal one closes the edit in admission's flush, `Refused` keeps it open | The editor is built once per edit (TE:470-505): a model write reaches it only if the factory bound a signal the application writes |

### Parameter outcomes

A parameter number field's save attempt (Enter, and for inline rows the end
of a gesture) has five distinct outcomes; revision 1 merged them into
"committed if accepted". The field begins its edit at the commit when no
gesture holds one (NI:628-632).

| Outcome | Path | Draft afterwards | Callbacks and host |
|---|---|---|---|
| Editor validation refusal: does not parse, out of range, off the step lattice | `draft()` fails, nothing is begun; a gesture's session is cancelled (NI:1277-1280) | Kept with the outline | `on_rejection` with the localized message; no `on_save`; the host sees nothing, or a gesture's `Cancelled` |
| Refused admission: `begin_edit` refused (busy, stale, read-only, unavailable, delivery fault) | `refused` turns the binding error into "not accepted" (NI:536-543, :628-632); no session exists, and a refused `begin_edit` changes nothing (`parameter_binding.rs:358-372`); the field accepts the authoritative value (NI:1267-1272) | Replaced by the authoritative value; an inline row leaves text mode (NI:1273-1275) | `on_rejection(None)` clears a shown refusal; no `on_save`; **no host outcome**, since nothing was admitted |
| Terminal refusal of an admitted edit: the commit's outcome is neither `Committed` nor `Submitted` (conflict, rejection) | `commit` returns false (NI:634-637; DecimalInput's at :756-783); the field accepts the authoritative value | Replaced by the authoritative value; an inline row leaves text mode | No `on_save`; the host's outcome event reports the refusal |
| Unchanged: the draft equals the base | No edit is begun, or the gesture's session is cancelled (NI:620-627) | Replaced by the (equal) authoritative value | No `on_save`; no host outcome, or a gesture's `Cancelled` |
| Accepted: `Committed`, or `Submitted` to a remote authority | `commit` returns true (NI:634-637) | Replaced by the committed or submitted value; a remote rejection later resynchronizes through the revision (NI:1226-1232) | `on_save` with the value; the host's outcome event |

`InlineSlider` has a different contract, which this part keeps:

| Moment | Behavior |
|---|---|
| Text mode opens (Enter, F2, a click on the value, `BeginEdit`) | The parameter edit begins then (NC:129-140, :301-324). A refused admission opens no text mode and no session; the refusal is recorded for inspection (NC:282-299) |
| Validation refusal: does not parse, not finite, out of range | Text kept with the outline; the admitted edit stays open (NC:162-189) |
| Confirmation (Enter) | Updates and commits the open edit (NC:191-203, :350-358); an unchanged value still commits, since a commit equal to the base is one authoritative commit (`pert-ui-core/src/edit.rs:174-180`) |
| Terminal refusal of the admitted edit (update or commit refused) | The outcome is recorded, the edit ends and text mode ends, showing the parameter's value (NC:191-203, :282-299) |
| Escape, focus loss, revocation, hiding, removal | The open edit is cancelled; the host sees `Cancelled` (NC:1107-1127, :1526-1537) |
| Callbacks | None: no `on_rejection` and no `on_save`; the host's outcome events are the report |

**Reading.** Most of the matrix is already one policy: validity failure,
focus loss, hiding, removal and application writes agree across every
built-in local editor, and every parameter row cancels where a local row
saves. The differences fall in three groups:

- **Authority and ownership** (kept): a parameter draft commits only on an
  explicit commit (C:731-744), so focus loss cancels it; a custom
  `DraftEditor` owner chooses its own policy; hiding and removal of a
  session's editor report nothing because its host caused them or ends with
  them.
- **Presentation** (kept): a built-in in-place editor (`InlineText`, the
  inline rows' text mode, `InlineSlider`) shows the value at rest and its
  draft only in a session; `SearchField`'s pending query is a delayed write
  of committed text whose results its owner shows.
- **Accidents** (changed by this part): the disabled number draft, focused
  or not (rows 8, 9), the held stepper press (rows 8, 9), the in-place rows'
  editability (rows 11, 12), the search query that outlives focus or arises
  without it (row 4), the `Refused` report from a disabled session field
  (row 3), and the drafts a close policy cannot see (rows 5, 8, 11).

## 2. The policy

These rules go into contracts.md as a new subsection, "Draft policy", under
"Text and draft editing" (C:1141), with the tables at the end of this
section. The permissions paragraph (C:350-357), the close list
(C:1043-1051), `App::on_close`'s step 3 and the controls chapter's paragraph
(`guide/controls.md:654-658`) are replaced by a sentence each and a link.

**Scope.** The rules bind the **built-in editors**: `TextField`, `TextArea`,
`SearchField`, `InlineText`, `IntegerInput`, `DecimalInput`,
`InlineInteger`, `InlineDecimal`, `InlineSlider`, and the `Table` as a
session host. A custom component that owns a `DraftEditor` keeps its own
policy: `DraftEditor`'s guarantees are unchanged, including that hiding
keeps the session (DE:495, `editing/draft_editor.rs:985`), and an owner may
discard on focus loss what it refuses on Enter, as the diagram editor does
(`canvas.rs:150-153`). `DraftEditorController::set_read_only` (D2) is an
additional tool, not an obligation.

**Terms.** A **draft** is text or a value the user changed in an editor that
the editor has not written, saved or proposed; it may arise without focus,
through a text command. A **local** editor is bound to a signal or shows a
controlled value; a **parameter** editor is bound to a `Parameter`
(C:731-744). A **built-in in-place** editor shows its value at rest and a
draft only while an editing session is open: `InlineText` and the text mode
of `InlineInteger`, `InlineDecimal` and `InlineSlider`.

A **save attempt** is the editor checking a draft and then writing, saving,
proposing or committing it, or refusing it. These inputs initiate one, and
nothing else does:

- **Enter**, a fresh press outside IME composition (TF:169-170, :3344-3349),
  in `TextField`, the number fields, `InlineText` and an inline row's text
  mode. In `TextArea` Enter inserts a line break and initiates nothing
  (TF:3345-3346); a `TextArea` saves only on focus loss.
- **Tab and Shift+Tab** with a session (`TextField`, `IntegerInput`,
  `DecimalInput`).
- **Focus loss**, including window deactivation and close admission.
- **A step** of a local field without a session (a step button, held or not;
  NI:1023-1039) and an inline row's resting step or drag release, each a
  save of the stepped value.

**Busy editors.** With a session, Enter and Tab while a composition or a
selection drag is active are kept without a save attempt and without a
report, and Escape ends only the composition or drag (TF:1694-1730,
NI:905-962). Without a session, Enter during a composition commits the
composition, not the draft. These stay as they are.

1. **Saving.** A save attempt saves a changed local draft its rules accept;
   focus loss cancels a parameter draft instead, which commits only on an
   explicit commit. Nothing else saves, commits or proposes: no revocation,
   re-enabling, hiding, removal, description or text command.
2. **Refusal.** A save attempt the editor's checks or the normalizer refuse
   writes nothing and keeps the draft for correction, with the error
   outline, whichever input initiated it; `on_rejection` reports it where the
   editor has one, and a session receives `Refused`. A built-in in-place
   editor keeps its session open after a refused focus-loss attempt. A
   parameter's protocol refusal is not this rule: it replaces the draft by
   the authoritative value ([parameter outcomes](#parameter-outcomes)).
3. **Application writes win.** A changed bound value, a changed controlled
   value or a parameter base or metadata change discards a draft unsaved; a
   session's next completion reports `Unchanged`. Writing the value the
   editor already shows changes nothing.
4. **Revoked editability** keeps focus, selection and copying, cancels a
   composition and editing gestures (a selection drag, a held step, a
   pressed stepper), keeps the draft unsaved and refuses every save attempt,
   with `Refused` to a session. **Changed:** an open built-in in-place
   session becomes read-only too (D2), and ending a held step no longer
   discards a typed or stepped draft.
5. **Revoked enablement** releases focus, if held, as cancelled input, not as
   focus loss, and cancels a composition and gestures. A local editor that
   shows its draft at rest (`TextField`, `TextArea`, `IntegerInput`,
   `DecimalInput`) keeps the draft unsaved, **whether or not it held focus**;
   re-enabling makes it editable and replays nothing; a session receives
   nothing and its edit stays open. **Changed:** the number fields keep
   their draft (D1). The documented exceptions:
   - a parameter draft is cancelled (authority);
   - a built-in in-place session ends unsaved, as Escape ends it
     (presentation: at rest the editor shows the value, and it has no
     disabled editing state);
   - `SearchField` discards a pending query (presentation: the owner shows
     results for the committed query).
6. **A revoked editor reports nothing to a session.** **Changed:** a focus
   loss that reaches a disabled session field, between a fixed denial and the
   next flush, reports nothing instead of `Refused`.
7. **Hiding and removal** discard a draft unsaved and cancel a parameter
   draft, focused or not; a session receives nothing, since its host hid or
   removed the editor or is hidden or removed with it. Cancellation of input
   for any other reason (a pointer gesture's loss, an input failure, a
   geometry change) never discards a local draft.
8. **A pending search query exists only while the field holds focus.**
   **Changed:** an edit the field receives while it does not hold focus (a
   text command, direct or queued) is written at once, as without a
   debounce, when editing is allowed (an edit is refused otherwise); focus
   loss writes a pending query when editing is allowed and otherwise
   discards it. So no `SearchField` ever holds an unwritten query without
   focus, and none needs `on_modified`.
9. **Close admission adds no rule.** It is a focus loss for each affected
   window's focused editor (rule 1); drafts kept elsewhere (rules 2, 4, 5)
   are untouched, so the close policy must be able to see them, which rule
   10 provides.
10. **Every built-in editor that can keep a draft without focus reports it.**
    `on_modified` reports `true` when a draft starts to differ from the value
    the editor shows at rest and `false` when it stops, after settling, as
    `TextField::on_modified` does (TF:555-566). **Changed:** `InlineText`,
    `IntegerInput`, `DecimalInput`, `InlineInteger` and `InlineDecimal` gain
    it. `SearchField` (rule 8) and `InlineSlider` (rule 1) cannot keep one.
    **Ordering:** every completion an editor reports to a session (Enter,
    Tab, focus departure and Escape; Escape reports directly today,
    TF:1705-1714, NI:936-944) goes through one reporting path that requests
    the modified report first, so a host that closes the edit and removes the
    editor in its delivery does so after the editor's report; the terminal
    `false` of a save, an unchanged confirmation or Escape always reaches the
    application. A draft that **removal** ends reports nothing, as every
    removed component's notifications (C:784-785). An edit the `Table` ends
    itself is reported by the table (rule 11, D4).
11. **The `Table` reports every edit's end** (D4). Besides the editor's own
    completions, it ends edits itself: Escape on its body while the editor
    does not hold focus (TB:1730-1733, TE:283-290), another edit beginning
    (TE:239-241), the edited row leaving the rows by deletion or a filter
    (TE:300-312), a failed editor factory (TE:500-510) and the table hiding
    (TB:2790-2795, TE:160-172). Each end, of either kind, reaches
    `Table::on_edit_end` once, after settling, through a notification request
    of its own, so ends keep their place in the global request order with
    every other component's notifications (C:791). An end recorded while no
    callback is installed is not reported later, and removing the callback
    discards ends not yet delivered. Removing the table itself reports
    nothing, as for any component: it is the application's own act.

The contracts.md tables, for the built-in editors:

| Event | Local draft | Parameter draft | In-place session | Session report |
|---|---|---|---|---|
| Save attempt by Enter, Tab or a step | saved if accepted, else kept | [five outcomes](#parameter-outcomes); `InlineSlider` its own | saved or proposed, else kept open | `Saved`, `Unchanged` or `Refused` |
| Save attempt by focus loss | saved if accepted, else kept | cancelled | as a local or parameter draft | `FocusLeft` with the outcome |
| Enter in `TextArea`; Enter or Tab while busy | no save attempt | no save attempt | no save attempt | nothing |
| Application write | discarded | discarded | ended | `Unchanged` at the next completion |
| Editability revoked | kept, save attempts refused | kept, save attempts refused | read-only, save attempts refused | `Refused` at a confirmation |
| Enablement revoked | kept unsaved, focused or not | cancelled | ended unsaved | nothing; the edit stays open |
| Hiding, removal | discarded | cancelled | ended | nothing |
| Close admission | as focus loss when focused, else untouched | as focus loss | as focus loss when focused, else untouched | as focus loss |

| Session completion | `Table` |
|---|---|
| `Saved`, `Unchanged`, `Cancelled` | closes the edit; after Enter, Tab or Escape focus goes to the body or the next cell; after a departure focus stays where it went |
| `Refused` | keeps the edit open; after Enter focus stays in the editor; after a departure the editor stays unfocused until its cell is activated again |
| Ended by the table (body Escape, replacement, row left, failed factory, hiding) | the editor is removed without a report of its own; `on_edit_end` reports the reason |

## 3. Changes, editor by editor

Each change states its effect on retained drafts, written values, proposals,
refusals and session notifications. Nothing listed here commits implicitly
or drops a terminal outcome.

- **`TextField`, `TextArea`:** rule 6 for a session field: the save attempt
  that a focus loss runs while disabled reports nothing. Rule 10's ordering
  for a session field's modified report. Retained drafts, writes and
  refusals unchanged.
- **`SearchField`:** rule 8. An unfocused edit (a direct or queued text
  command, including one from its own clipboard menu) writes at once instead
  of arming the debounce; focus loss while editing is denied discards the
  pending query and its deadline and shows the bound query. Was: armed and
  kept pending without focus (SF:640-647, :317-335) and, while read-only,
  kept pending until the next edit, Enter or focus loss once allowed
  (SF:176-180). No session.
- **`InlineText` (signal and controlled):** `on_modified` (rule 10). No
  behavior change: proposals, refusals and the enablement exception stay.
- **`DraftEditor`:** `DraftEditorController::set_read_only` (D2). Its owner
  contract states that the built-in owners follow the draft policy and that
  a custom owner chooses its own; nothing else changes.
- **`IntegerInput`, `DecimalInput` (signal, with or without a session):**
  - Revoked enablement keeps the typed or stepped draft unsaved, by a bound
    permission or a fixed denial, focused or not, and ends a held stepper
    press without discarding it (D1). The field shows the draft, keeps its
    outline if invalid, and saves it only at a later save attempt after
    re-enabling. A session receives nothing and its edit stays open, which
    makes `session`'s documentation (NI:2488-2495), contracts.md's
    (C:1689-1693) and `ColorPicker::enabled`'s (`color_picker.rs:480-486`)
    true; `enabled`'s documentation (NI:2093-2101) is rewritten.
  - `cancel_input` no longer discards a draft (rule 7): today it discards
    whenever the field is unfocused and no gesture is held (NI:2427-2440),
    which covers the unfocused kept draft that a pointer move and a bound
    revocation reach (PT:594-596, RT:6246-6265). It ends gestures only;
    hiding discards in `visibility_changed`, as `TextField` does
    (TF:808-814), and removal drops the field.
  - Revoked editability during a held stepper press ends the press and keeps
    the stepped draft (rule 4; was: discarded, NI:1218-1224, :1372).
  - Focus loss during a held stepper press ends the press and then makes the
    focus-loss save attempt (rule 1): a field without a session has nothing
    left to save, since each step saved (NI:1023-1039); a session field
    saves the stepped draft if valid and reports `FocusLeft` with the
    outcome. Was: the stepped draft discarded and nothing reported
    (NI:2453-2455), a lost completion.
  - Rule 6 for a session field, and `on_modified` with rule 10's ordering.
- **`IntegerInput`, `DecimalInput` (parameter):** no behavior change; the
  five [parameter outcomes](#parameter-outcomes) stay. `on_modified` reports
  a typed draft until it is committed, refused by the protocol (the draft is
  replaced) or cancelled; a validation refusal keeps it `true`.
- **`InlineInteger`, `InlineDecimal` text mode (signal and parameter):** an
  open text mode follows revoked and restored editability through
  `set_read_only` (D2): typing is refused, the composition and a selection
  drag end, the draft stays, Enter refuses; when editing is allowed again
  the session becomes editable with its draft, selection and undo.
  `on_modified` reports text mode drafts; a value drag or held step at rest
  is a gesture, not a draft, and reports nothing. Enablement keeps ending
  text mode (the in-place exception).
- **`InlineSlider`:** no change. Its rows of the matrix already follow the
  policy as a parameter and in-place editor; its lack of `on_rejection` is
  part of the inline rows' builder set, which N8 revisits.
- **`Table` cell editors:** a number cell editor follows its field: a
  disabled one keeps its draft and the edit stays open until Escape on the
  body (TB:1333-1334), as C:1174-1175 already says of every cell editor.
  The table gains `on_edit_end` (rule 11, D4), which reports every edit's
  end, including those it ends itself; acting on completions, focus and
  Tab are unchanged.

## 4. Public API

All additions; no item is removed or changes signature.

```rust
// Generated by number_inputs.rs's shared builder macro for IntegerInput,
// DecimalInput, InlineInteger and InlineDecimal, and written once for
// InlineText.
impl IntegerInput {
    /// Calls `modified` with `true` when a draft starts to differ from the
    /// value the field shows at rest and `false` when it stops: after a save,
    /// Escape, an application write that discards it, hiding, an edit back
    /// to the saved value or, for a parameter, a cancellation or a protocol
    /// refusal.
    pub fn on_modified(
        self,
        modified: impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static,
    ) -> Self;
}

impl<P: Copy + PartialEq + 'static> DraftEditorController<P> {
    /// Makes the active session read-only, or editable again, without ending
    /// it: the draft, payload, selection and undo history stay.
    pub fn set_read_only(&self, read_only: bool);
}

// D4, recommended:
impl Table {
    /// Calls `ended` once for each cell edit that ends, however it ended,
    /// after the editor's own reports.
    pub fn on_edit_end(
        self,
        ended: impl FnMut(&mut EventContext, &CellEditEnd) -> Result<(), RuntimeError> + 'static,
    ) -> Self;
}

// Both types are defined beside `CellEdit` in `table/editing.rs`,
// re-exported through `table.rs`, and exposed from the runtime root and the
// runtime prelude, and so from the facade's root and prelude.

/// One ended cell edit. Accessors `row()`, `column()` and `reason()`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CellEditEnd { /* private */ }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CellEditEndReason {
    /// The editor reported a terminal completion.
    Completed(EditorCompletion),
    /// Escape on the body ended an edit whose editor did not hold focus.
    Cancelled,
    /// Another edit began.
    Replaced,
    /// The edited row left the rows, by deletion or a filter.
    RowLeft,
    /// The cell editor callback failed.
    Failed,
    /// The table was hidden.
    Hidden,
}

// D3, recommended:
pub enum TextCommand {
    // ... existing variants ...
    /// Discards the editor's draft, as Escape does, without writing.
    DiscardDraft,
}
```

### `on_modified`

The rustdoc follows `TextField::on_modified` (TF:555-591): the timing class
"after settling" with its link; each change delivered once and in order; its
error returned from the flush or input call that delivered it and not
replayed; a removed field reports nothing (C:784-785); replaced by a later
call and by redescription. Each type states its draft: a standalone field's
typed or stepped draft, including one a text command made without focus; an
inline row's text mode only; `InlineText`'s open session. It states rule
10's ordering for session fields. The doctest of `IntegerInput::on_modified`
counts unsaved fields with `FormFields` and `FieldHandle::set_modified`,
keeps the count after revoking the field's permission, and clears it on
Escape; `InlineText::on_modified`'s shows a refused focus-loss save keeping
it `true`.

### `set_read_only`

- **No session:** no effect; a later `begin` or `begin_read_only` chooses
  again.
- **Read-only:** cancels a composition and a selection drag, and releases
  the pointer capture the editor holds for that drag, the capture release
  that context loss performs (RT:1016-1018), through a crate-private
  operation on the editor's node; its timers and other state stay. The IME
  then receives no text input state and `on_edit` does not run, as in a
  session begun with `begin_read_only` (DE:298-302); `replace` still works.
- **Editable again:** typing, IME and `on_edit` resume with the kept draft,
  selection and undo.
- **Publication:** the getters (`is_read_only`) report the new state at once
  (the published state, DE:103-111), and the editor and its owner are
  invalidated for paint, as `replace` invalidates them (DE:349).
- **Idempotent:** setting the current state does nothing.
- **Context:** it takes none, like `end` (DE:353-362), so an owner can call
  it from `update`. A built-in in-place row calls it once per update, after
  `read_permissions` has read the current permissions (NI:1212), with the
  effective state (the `editable` permission, the source's editability and
  a refused bound value); today `lock` runs before that read (NI:1209-1212),
  and the order is reversed so the slot never applies a stale permission.
- **Deferred calls.** While the editor runs an operation (only the
  application's clipboard service can call back meanwhile), a call is
  recorded and applied when the operation completes, as `end` is
  (DE:116-127):
  - repeated calls keep the last requested state;
  - a recorded change belongs to the session it was requested in (a session
    counter beside `end_requested`); it is dropped if that session has
    ended, so it never affects a session begun later;
  - a deferred `end` wins: the session ends and a recorded lock is dropped;
  - otherwise the recorded state is applied as soon as the operation
    completes and is published, **before** `on_edit` runs. An operation
    that began while the session was editable completes as an edit: its
    text stays, and `on_edit` still reports it once, the one case in which
    `on_edit` runs for a session that is read-only by then. (A deferred
    `end`, by contrast, suppresses `on_edit`, DE:709-712, because the session
    no longer exists.)
  - Calls `on_edit` makes are not deferred (no borrow is held while it runs,
    DE:713-724) and apply at once, after the recorded state, so a later
    request supersedes an earlier one: `set_read_only(false)` from `on_edit`
    leaves the session editable, and `end` followed by `begin` from
    `on_edit` starts an editable session that the earlier lock never
    touches.
  - An error from `on_edit` propagates from the input call, as today; the
    edit and the already-applied state stay.

### `TextCommand::DiscardDraft` (D3)

The working replacement for revision 1's "write the bound value", which does
nothing when the value equals the saved one (NI:1178) and otherwise changes
application data.

- **Effect:** the editor discards its draft exactly as Escape does without a
  composition: the shown text returns to the value at rest; nothing is
  written, saved or proposed; undo is cleared as after Escape; an in-place
  session ends; a parameter draft and its gesture are cancelled; a
  `SearchField`'s pending query is discarded without being written.
- **Availability:** answered also while the editor is disabled or read-only,
  since it changes no application data. It answers `Handled` when there was
  a draft and `Unavailable` when there was none. `TextField` and `TextArea`
  outside draft mode, `SelectableText` and non-editors answer as for other
  commands they do not support.
- **Delivery:** by `Runtime::text_command` or
  `EventContext::request_text_command`, which need no focus; it moves no
  focus.
- **Targeting embedded editors.** The command is the owning control's: it
  decides what discarding means (rule 2's in-place session, its parameter
  edit). The runtime offers a command to its target first and forwards it
  only downward, to a focused descendant, when the target answers
  `NotAnEditor` (RT:5612-5650); there is no route from a child back to its
  owner. So:
  - `InlineText` and the inline rows answer `DiscardDraft` at the owner,
    before they pass other commands to their editor (IT:860-875,
    NI:1658-1679), whether or not the session holds focus, so an unfocused
    retained session is discarded through its owner;
  - a `DraftEditor` answers it with `Unavailable` and changes nothing
    (DE:852-861 handles the other commands through its generic editor,
    which does not own the session's policy). A command aimed at the
    focused child therefore does nothing; the rustdoc says to target the
    owning control;
  - a custom owner keeps its policy: if its `text_command` does not answer
    `DiscardDraft`, the runtime forwards it to its focused `DraftEditor`,
    which answers `Unavailable`, and the session stays. An owner that wants
    the command answers it in its own `text_command` with its own
    cancellation, for example `DraftEditorController::end`.
- **Reports:** `on_modified(false)`; `on_rejection(None)` if a refusal was
  shown; a session receives nothing and its edit stays open, as for an
  application write (its next completion reports `Unchanged`).
- **Rustdoc example:** disable a field holding a draft, then discard the
  draft with the command; the binding keeps its value and the field shows
  it.

The advanced routes stay documented with their costs: describing the field
with another binding discards a draft (TF:1916-1935, NI:1349-1361) but needs
a second signal holding the same value and remeasures the field; describing
it under another key remounts it, losing focus, undo and retained layout.

### `Table::on_edit_end` (D4)

- **Recording.** Each path that ends an edit records it, with its reason, at
  the moment it ends: `complete` for terminal completions (TE:328-365),
  `cancel_unfocused` (TE:283-290), the replacement in `begin_edit`
  (TE:239-241), the row check in `deliver_edits` (TE:300-312), a failed
  factory (TE:500-510) and `close_quietly` (TE:160-172), which has no
  context. Recording and the table's own cleanup (closing the edit,
  republishing the rows, focus) are independent of whether a callback is
  installed.
- **Delivery slots.** Each recorded end is one payload with one notification
  request of its own, and each delivery consumes exactly the payload its
  request was made for. The table's existing requests (session completions,
  a pending begin, the `stale` republish; TE:154-159, :292-326, and the
  sessions' reports, `editor_session.rs:248-262`) keep servicing completions
  and housekeeping and never deliver an end. So an end takes its place in
  the global request order (C:791): when `clear_focus` queues a refused
  completion for the table, another component then requests a delivery,
  and the table is hidden, the order is the table's completion handling,
  the other component's callback, then `Hidden`. The notifier contract
  makes one change per request the way to keep that order
  (`runtime/notify.rs:263-271`).

  The table's notifier also carries the session reports, which the table
  cannot tell apart from its own requests, so the end slots use a notifier
  of their own: a crate-private, invisible child that the table mounts
  beside its body, with no size, focus, semantics or workspace state, whose
  `deliver_notifications` delivers one payload per call. The callback's
  `EventContext` is that child's, as `on_selection_change`'s is the body's
  (TB:1625-1633); its id names the child. Inspection lists the child as it
  lists the cell host (`HOST_KEY`, TE:16-17).

  Its presentation mechanics: the reporter is a stable structural leaf that
  stays logically visible while the table is visible (hiding it would keep
  its update from registering its notifier, `runtime.rs:6122`), paints
  nothing, is not focusable and rejects hits (its `hit_test` returns false,
  overriding the default, `runtime.rs:2235`). `Table::arrange`, which today
  requires exactly one visible child (`table.rs:2376`), places the reporter
  at zero extent and keeps the body's allocation and child index. The
  reporter keeps its notifier and identity across the callback's
  redescription. The callback's rustdoc says that its `EventContext` is the
  reporter's, so `cx.request_focus()` does not focus the table body.
- **Timing:** after settling, in an event phase, where the callback may
  write signals, including `FieldHandle::set_modified`, which the factory
  cannot: it runs in the host's update (TE:461-519), where the summary write
  is refused (`field_status.rs:358-372`). An end can therefore arrive after
  a later edit's factory ran, as for a replacement begun from F2 during
  input (TB:1701-1712), whose rows the notification preparation settles
  first (`runtime/notify.rs:350-371`). A hidden table in a visible window is
  still delivered to; a hidden native window holds the requests until it is
  shown, then delivers them in order.
- **Failure:** the error is returned from the flush that delivered it; that
  payload is consumed and not replayed, and the other slots are still
  delivered, as for other notifications.
- **Replacement and removal of the callback.** Describing the table again
  with another `on_edit_end` replaces it: payloads not yet delivered go to
  the callback current at delivery. Describing it without one removes it:
  its undelivered payloads are discarded, while their slots keep their
  positions and deliver nothing (C:785-787), and ends recorded while no
  callback is installed get no payload. Installing a callback again reports
  only ends recorded afterwards; nothing is replayed.
- **Table removal:** reports nothing, as for any removed component; the
  child goes with the table. Unmounting also clears payloads not yet
  delivered, as it clears `stale` (TB:2796-2801), so a table mounted later
  on the same controller reports none of them.
- **A prelude-only callback** (the rustdoc's example):

  ```rust
  use pert_ui::prelude::*;

  # fn example(table: Table, drafts: FieldHandle) -> Table {
  table.on_edit_end(move |_, end| {
      match end.reason() {
          CellEditEndReason::Completed(EditorCompletion {
              trigger: EditorTrigger::Tab { direction: FocusDirection::Backward },
              outcome: EditorOutcome::Saved,
              ..
          }) => { /* saved, moving back */ }
          CellEditEndReason::Hidden | CellEditEndReason::RowLeft => {}
          _ => {}
      }
      drafts.set_modified(false).map(|_| ())
  })
  # }
  ```

### Imports and the anti-regrowth rule

- **Prelude names.** `on_modified`, `set_read_only` and `DiscardDraft` use
  `EventContext`, `RuntimeError` and `TextCommand`, which the prelude holds;
  a program writing `TextCommand::DiscardDraft` needs what one writing
  `TextCommand::BeginEdit` needs. `Table::on_edit_end` brings vocabulary an
  ordinary callback matches, which L2's vocabulary clause puts in the
  prelude (`layering-design-L2.md`, section 2.1, "Collections" row beside
  `CellEdit`): `CellEditEnd`, `CellEditEndReason`, and the payload of
  `Completed`: `EditorCompletion`, `EditorOutcome`, `EditorTrigger` and,
  for `EditorTrigger::Tab`'s field, `FocusDirection`. All six join the
  runtime prelude (and so the facade's); none requires `component::*`.
  `EditorCompletion`, `EditorOutcome` and `EditorTrigger` are root names
  today and `EditorOutcome` and `EditorTrigger` are also in `component`
  (`lib.rs:72`, `:741`); `FocusDirection` is in the prelude at HEAD
  (`lib.rs:528`) and L2's draft moves it to `component` and `testing`,
  so L4b adds it back to the prelude as the same item. Overlapping globs of
  one item are valid, so L2's collision review is unchanged; the prelude
  test of L2 gains the example above as a prelude-only program, and L2's
  import guarantees stand.
- **Anti-regrowth** (AGENTS.md, from L1): `on_modified` on these types is not
  a second way: no public way reports their drafts (F6). `set_read_only`
  makes nothing advanced; restarting with `begin_read_only`, which loses the
  selection and undo, stays for an owner that wants a fresh session.
  `DiscardDraft` makes the rebinding route advanced, for the cost above.

## 5. Implementation outline

- **One modified-report helper.** The modified reporting of TF:1561-1622
  (`note_modified`, `deliver_modified`, the report queue and the notifier)
  becomes a crate-private type shared by `Editor`, `InlineText` and
  `NumberEditor`. Every session report goes through one crate-private path
  that calls `note_modified` first, after the draft's state has been
  updated: `report_session` (TF:1659-1667, NI:882-889) and the Escape paths,
  which call `session.report` directly today (TF:1705-1714, NI:936-944) and
  are changed to use it. That gives rule 10's ordering.
- **Number fields.** A crate-private predicate, "local and standalone", is
  `!source.is_parameter() && !field.inline`. For such a field:
  - where revocation calls `abandon` and `reset_draft` (NI:1220-1225,
    :1372-1376) and where focus loss cancels a held press (NI:2453-2455), it
    ends the press (pointer release, timer, `stop_hold`) and keeps the draft;
  - `cancel_input` (NI:2427-2440) ends gestures and composition and never
    resets the draft; `visibility_changed(false)` (NI:2416-2420) discards it.

  The other sources keep today's paths.
- **Rule 6.** `complete` (TF:1669-1689) and NI's (NI:890-903) report nothing
  when the field is disabled.
- **In-place editability.** `NumberEditor::update` applies `lock` after
  `read_permissions` (NI:1209-1212), and `lock` (NI:1008-1012) passes the
  effective state to an open slot through `set_read_only`. The controller
  applies it with `Editor::set_owner_lock` (TF:2049-2057) plus the capture
  release, and records it in `Shared` beside `end_requested` (DE:96-127),
  with the session counter, when `core` is borrowed. In `DraftEditor::edit`
  (DE:700-724) the recorded state is applied after publication; the check
  that skips `on_edit` for a read-only session (DE:715) uses the state the
  operation ran under, so the edit is still reported.
- **SearchField.** `edited` (SF:317-335) writes at once when the field does
  not hold focus (`cx.focused()`), through `flush` (SF:280-315);
  `focus_lost` (SF:687-694) discards the pending query when `flush` would
  keep it for lack of editability (SF:294-298).
- **`DiscardDraft`.** Each built-in editor's `text_command` answers it
  before its availability checks, with its Escape discard path (TF:1885-1891
  and the session-free Escape, SF:336-342, IT:471-474, NI:1313-1316);
  `InlineText` and the inline rows answer it before returning `NotAnEditor`
  for other commands; `DraftEditor::text_command` (DE:852-861) answers it
  `Unavailable`.
- **`Table::on_edit_end`.** A queue of payloads in the table's edit state
  (TE:97-116), each tagged with the callback generation current when it was
  recorded; removing the callback advances the generation, and a delivery
  whose payload's generation is stale delivers nothing. The reporter child
  is mounted with the table's other children and stores its notifier in the
  edit state, as the table stores its own (TE:104); each recorded end pushes
  a payload and requests one delivery through it, from any phase
  (`close_quietly` included, since requesting is allowed there).

## 6. Documentation and records

- **contracts.md:** the "Draft policy" subsection (section 2), with the
  scope and the save-attempt definition; the permissions paragraph
  (C:350-357) keeps its non-editor sentences and replaces the editor list
  with a link; the close list (C:1043-1051) becomes "each focused editor
  makes the draft policy's focus-loss save attempt"; editor sessions
  (C:1689-1693) adds rule 6 and rule 10's ordering; table cell edits
  (C:1164-1175) gains the completion table and `on_edit_end`.
- **Rustdoc:** `App::on_close` step 3 (`app.rs:531-551`) links the policy
  and keeps its summary sentence; `IntegerInput`/`DecimalInput` `enabled`
  and `editable` (NI:2066-2101) and their type documentation's revocation
  sentences; `SearchField`'s type documentation (unfocused edits) and
  `editable` (SF:176-180); `DraftEditor`'s owner contract (DE:468-476)
  states the scope, `set_read_only` and that `DiscardDraft` targets the owner;
  `InlineInteger`'s type documentation (read-only text mode);
  `TextField::session` (TF:866-873) adds rule 6; `TextCommand::DiscardDraft`;
  `Table`'s cell-editing documentation (TB:1305-1335) and
  `Table::on_edit_end`, `CellEditEnd`, `CellEditEndReason`.
- **Guide:** the controls chapter's disabling paragraph
  (`guide/controls.md:654-658`) and number paragraph (`:424-434`) link the
  policy and name `DiscardDraft`; the testing chapter's close policy section
  (`guide/testing.md:262-273`) adds that a field's kept draft is not saved by
  admission and is counted with `on_modified`, pointing to
  `IntegerInput::on_modified`'s doctest; the custom-widgets chapter's
  `DraftEditor` paragraph (`guide/custom-widgets.md:233-240`) mentions
  `set_read_only` and that a custom owner chooses its policy.
- **Manual:** the `on_modified` sentence (`docs/manual.md:426-428`) names the
  added types.
- **Inventory and plans:** question 7 of the inventory
  (`complexity-inventory.md:516`) is answered in the plan's progress record:
  the authority and presentation differences were deliberate and stay as
  named exceptions; the others were accidents.
- **TODO.md:** no entry touches these files' behavior (TEXT-01 is
  `TextArea`'s capacity, the `DraftAlign` entry is alignment). The
  implementation records one follow-up: the `diagram_editor` example's close
  policy (`examples/diagram_editor/app.rs:891-913`) asks only about the
  document, so a refused inspector draft (`app.rs:452-498`) is lost on close;
  `on_modified` now lets it count them.

## 7. Migration manifest

- **Changed sources:** TF, SF, IT, DE, NI, IF, TE, TB, `text_input.rs`
  (`DiscardDraft`), `lib.rs` (the new table names); `crates/pert-ui/src/app.rs` (rustdoc);
  `color_picker.rs` needs no change.
- **Packaged examples:** none uses a revoked number field, a read-only
  search field or an unfocused search command. The `inventory` example's
  cell editors (`examples/inventory/app.rs:222-250`) and the
  `diagram_editor` inspector and canvas rename run unchanged (the canvas
  keeps its own policy); their headless sessions are in the gate.
- **Guide and manual:** as section 6; the guide's doctests compile with the
  same imports.
- **Gallery:** its number fields are fixed-disabled or always enabled
  (`apps/gallery/src/pages/text.rs:189-191`); no source change. Its
  `TextCommand` matches, if any are exhaustive inside the workspace, gain the
  variant (`TextCommand` is `#[non_exhaustive]`, so external code is
  unaffected).
- **Native checks:** none uses these transitions; no change.
- **Tests and doctests:** the new tests of section 8. A search of
  `crates/pert-ui/tests` found no assertion of a discarded number draft on
  revocation: the revocation tests of sessions and tables use text fields
  (`editing/editor_sessions.rs:1428`, `collections/table_editing.rs:990`),
  and the number tests mount fixed-disabled fields
  (`controls/number_inputs.rs:343-356`) or revoke an inline row's drag
  (`controls/inline_numbers.rs:138`). Any found at implementation is
  rewritten to the new rule and listed in the review record.
- **Diagnostics fixtures:** none involves these types.
- **`plans/usability/task4-app`:** uses `TextField` only; no change.
- **Frozen, not migrated:** the round 1 submissions, probes and triage under
  `plans/usability/runs/` and `~/pert-ui-usability/`.

## 8. Tests

The tests live in the facade's area binaries: a new module
`editing/draft_policy.rs` for the policy, `surfaces/window_close.rs` for
close admission (it needs `native`), `collections/table_editing.rs` and
`editing/editor_sessions.rs` for sessions, `editing/draft_editor.rs`,
`editing/search_field.rs` and `controls/color_picker.rs`.

- **New-behavior tests** cover what only the new rules do. Each names the
  assertion that detects the old implementation and is checked with a
  compiling mutation that restores the old step; the named assertion must
  fail. A test that fails only because a new item no longer compiles is not
  evidence.
- **Contract and compatibility tests** check rules this part keeps. Where
  one guards a rule the change could break, it is checked with a mutation
  that violates that rule.

### New-behavior tests

1. **A disabled number field keeps its draft** (`IntegerInput` and
   `DecimalInput`, by a bound permission and by a fixed redescription): type
   `42` over `7`, revoke, flush. Named assertion: inspection's text is `42`
   and `dirty` is true while the binding holds `7` and focus is gone. Then
   re-enable, focus and `clear_focus`: the binding holds `42`. Mutation:
   restore `abandon` and `reset_draft` in the revocation branch (NI:1220-1225
   for the bound case, :1372-1376 for the fixed one).
2. **An unfocused kept number draft survives revocation:** type `4x` (does
   not parse), move focus to another field (the attempt is refused, the
   draft kept), deliver a pointer move over the field, then revoke the bound
   permission and flush. Named assertion: the field still shows `4x` with the
   outline. Then hide the field and show it again: the draft is gone and the
   bound value shows (rule 7's discard). Mutation: restore `cancel()` in
   `cancel_input`'s unfocused branch (NI:2438-2440).
3. **A disabled session number field keeps the edit and the draft**
   (`editor_sessions.rs`, mirroring `disabling_the_editor_keeps_its_draft_unsaved`
   at `:1428` for `IntegerInput`): after revocation the log is empty; after
   re-enabling, focusing and Enter, named assertion: the log is
   `[(Enter, Saved)]` and the value is the typed one. Mutation: as test 1.
4. **A table's disabled number cell keeps its draft** (`table_editing.rs`):
   the edit stays open; activating the cell again focuses the editor, whose
   text is the draft (named assertion); Enter saves it. Mutation: as test 1.
5. **ColorPicker's fields agree:** revoking the picker's permission while a
   channel field holds a typed draft keeps it, as the hex field keeps its own.
   Named assertion: the channel field's text is the draft. Mutation: as
   test 1.
6. **Focus loss during a held stepper press saves a session field's draft**
   (`DecimalInput` with a session and `stepper(true)`): press and hold a
   step button until two steps repeat, then `clear_focus`. Named assertion:
   the log is `[(FocusLeft, Saved)]` with the stepped value. Mutation:
   restore the press condition in the cancel branch (NI:2453). A companion
   without a session checks that the saved steps stay and no extra step is
   taken (contract).
7. **Revoked editability during a held press keeps the stepped draft**
   (session field): named assertion: after revocation the text is the
   stepped draft and Enter reports `Refused`; after allowing again, Enter
   saves it. Mutation: restore the `lost` reset (NI:1218-1224).
8. **An open inline text mode follows editability** (`InlineInteger`,
   signal and parameter): enter text mode, type `42`, revoke editability,
   flush, type `9`. Named assertion: the draft is still `42`; a composition
   started before the revocation is gone; Enter writes nothing and text mode
   stays. Allow editing again: typing appends and Enter saves (signal) or
   commits (parameter). Mutations: drop the `set_read_only` call from
   `lock`; and, separately, move `lock` back before `read_permissions`, for
   which a bound permission revoked in the same flush leaves the slot
   editable (named assertion as above).
9. **`set_read_only` keeps the session** (`editing/draft_editor.rs`, a
   custom owner):
   - type two edits, set read-only, check that typing is refused and
     `on_edit` does not run, set editable, undo once. Named assertion: undo
     restores the text after the first edit. Mutation: implement
     `set_read_only` as `end` followed by `begin_read_only` or `begin`;
   - capture release: start a selection drag in the editor (primary press
     and a move), set read-only, then, **with the primary button still
     held**, move the pointer over a sibling probe component that records
     pointer events. Named assertion: the probe receives that move and the
     editor does not, before any release is sent. (The runtime clears
     capture on the last physical release, PT:438-441, and the lock already
     stops the drag's selection, TF:2059-2064, so assertions made after the
     release could not tell.) Mutation: omit the capture release; the move
     then goes to the captured editor;
   - reentrant clipboard: an application clipboard service that calls
     `set_read_only(true)` while serving a Paste. Named assertion: the
     pasted text stays, `on_edit` ran once with it, `is_read_only()` was
     already true inside `on_edit`, and a following `text_input` is
     refused. Then the same with `set_read_only(true)` followed by `end()`
     in the service: the session ends and `on_edit` does not run. Mutations:
     apply the lock before the edit's publication (the pasted text is lost
     or `on_edit` does not run); let the recorded lock survive a deferred
     `end` (a later `begin` starts read-only);
   - precedence: the service requests `set_read_only(true)` and `on_edit`
     calls `set_read_only(false)`. Named assertion: the session is editable
     afterwards and a following `text_input` edits. Then the service
     requests the lock and `on_edit` calls `end()` and `begin(...)`: named
     assertion: the new session is editable. Mutation: apply the recorded
     state after `on_edit` (both assertions fail);
   - callback error: as the first sequence with an `on_edit` that returns an
     error. Named assertion: the input call returns that error once, the
     pasted text stays and the session is read-only. Mutation: skip the
     recorded state when `on_edit` fails.
10. **A search field's pending query needs focus** (`search_field.rs`, debounce
    300 ms):
    - focused, type `abc`, revoke editability, `clear_focus`. Named
      assertion: inspection's `pending` is false and the field shows the
      bound query; advancing past the delay writes nothing. Mutation:
      restore the early return that keeps the query pending;
    - unfocused, `Runtime::text_command(&field, Paste)` with `abc` on the
      clipboard. Named assertion: the query is `abc` when the call returns
      and `pending` is false; revoking editability and advancing past the
      delay change nothing. The same through `EventContext::request_text_command`
      from another component's callback: written in that flush. Mutation:
      arm the debounce for unfocused edits as today.
11. **A disabled session field's focus loss reports nothing**
    (`editor_sessions.rs`, `TextField` and `IntegerInput`): type, describe
    the field `disabled()`, then `clear_focus` before any flush. Named
    assertion: the log is empty. Mutation: restore `Refused` in `complete`.
12. **`DiscardDraft` (D3)** (`draft_policy.rs`): for each built-in editor
    with a draft (a disabled number field, a read-only `TextField` in draft
    mode, an open `InlineText` session, a session field in a table, a
    focused `SearchField` with a pending query): the command answers
    `Handled`, the binding is unchanged, the field shows the value at rest,
    `on_modified` reports `false` (named assertion), and a session's log is
    empty with the edit open; on an editor without a draft it answers
    `Unavailable`. Mutation: refuse the command while disabled (the disabled
    case fails). Targeting:
    - aimed at an `InlineText` or an `InlineInteger` row (the owner) with a
      focused open session: `Handled`, the session ends unsaved;
    - aimed at the same owner whose refused session stayed open after focus
      left (unfocused): `Handled`, the session ends. Mutation: forward the
      command to the editor instead of answering at the owner (the
      unfocused case answers `NotAnEditor`);
    - aimed at the focused `DraftEditor` child: `Unavailable`, the session
      and its draft stay;
    - a custom owner that does not answer the command: forwarded to its
      focused `DraftEditor`, `Unavailable`, its session stays (contract).
13. **`on_modified` on the added types** (`draft_policy.rs`):
    - `IntegerInput`: typing reports `true`; Enter `false`; typing then
      Escape `false`; typing then an application write `false`; typing then
      revoking enablement keeps `true`; hiding `false`; an unfocused Paste
      `true`.
    - Parameter `IntegerInput`: an off-step draft keeps `true` after Enter;
      a protocol refusal (another edit holding the parameter) reports
      `false`; a commit `false`.
    - `InlineText`, signal and controlled: beginning an edit reports
      nothing; typing `true`; Enter `false`; a normalizer refusal on focus
      loss keeps `true`; a changed controlled text `false`; disabling
      `false`.
    - `InlineDecimal`: text mode typing `true`, Escape `false`; a value drag
      reports nothing.
    - A callback error: a callback that fails on its `false` report makes the
      delivering call return the error once; a later flush calls nothing
      (no replay), and the session completion still reached its host.

    Named assertion for each type: the recorded sequence. Mutation: remove
    the type's request for delivery (the sequence is then empty), and for
    the inline row, report the drag (the drag assertion fails).
14. **Escape reaches the application before the table removes the editor**
    (`table_editing.rs`): a cell's `IntegerInput` and `TextField` editors
    feed a `FieldHandle` from `on_modified`; type, then Escape. Named
    assertion: after the flush the callback received `false`, the summary's
    count is zero, the log is `[(Escape, Cancelled)]` and the edit is
    closed. Mutation: report Escape's completion before the modified report
    (TF:1705-1714 and NI:936-944 as today); the count stays one.
15. **The table reports the edits it ends** (`table_editing.rs`, D4): one
    `FieldHandle` per cell, registered up front, set from the editor's
    `on_modified` and cleared from `Table::on_edit_end` for the ended cell.
    Leave a refused draft open (count one), then each of these, without
    beginning another edit: Escape on the body; a filter that hides the
    edited row; deleting the row; hiding the table. Named assertion for
    each: `on_edit_end` ran once with the reason (`Cancelled`, `RowLeft`,
    `RowLeft`, `Hidden`) and the count is zero after the flush. A terminal
    completion reports `Completed(..)` after the editor's `false`.
    Mutations: do not record `cancel_unfocused`'s end (the body-Escape count
    stays one); drop the hidden end in `close_quietly` (the hiding count
    stays one).
16. **Replacement is reported once, after settling:** with a refused draft
    open in cell a, (i) select cell c and press F2, which begins the new
    edit during input (TB:1701-1712); (ii) double-click cell c, which begins
    it at the table's delivery (TE:154-159, :298-300). Named assertions for
    each: exactly one `Replaced` for a, whenever it arrives relative to c's
    factory; after the flush, a's count is zero and c's is zero until c is
    typed into, then one. Mutation: record the replacement twice (once in
    `begin_edit`, once in the pending path); the "exactly one" assertion
    fails.
17. **Edit ends keep the global order** (`table_editing.rs`): with a refused
    draft in a cell whose editor holds focus, a probe component beside the
    table that records its notifications, then in one flush: `clear_focus`
    (the editor reports `(FocusLeft, Refused)` to the table), the probe
    requests a delivery, the table is hidden. Named assertion: the recorded
    order is the probe's callback, then `on_edit_end(Hidden)`; nothing about
    the edit reaches the application before the probe. Variants:
    - the edit-end callback fails: the flush returns that error once, the
      probe's callback still ran, and a later flush delivers nothing again;
    - the same inside a detached native window that is hidden before the
      flush: nothing is delivered while it is hidden; showing it delivers
      the probe's callback and then `Hidden`, in that order.

    Mutation: deliver payloads from the table's own `deliver_notifications`
    (draining them on its older completion request); `Hidden` then precedes
    the probe.
18. **Removing the callback discards pending ends** (`table_editing.rs`):
    record an end (body Escape), then, before the flush delivers it,
    describe the table without `on_edit_end` and again with a new callback.
    Named assertion: the new callback receives nothing for that end, the
    edit is closed and the rows republished all the same, and a later end
    reaches the new callback. With replacement instead of removal (a new
    callback, never none), the new callback receives the pending end.
    Mutation: keep undelivered payloads across removal (the reinstalled
    callback receives the old end).
19. **The close policy sees kept drafts and only those** (`window_close.rs`,
    through `HeadlessApp::admit_close`), each with a policy that keeps the
    window open while `FormFields::summary().modified()` is non-zero, fed by
    `on_modified`:
    - a local `IntegerInput` disabled with a typed draft: named assertion
      `KeepOpen`, and afterwards the field still shows the draft. Mutation:
      test 1's;
    - test 2's sequence (unfocused invalid draft, pointer move, bound
      revocation) before the close: named assertion `KeepOpen`. Mutation:
      test 2's;
    - a focused `IntegerInput` with an invalid draft and a focused
      `InlineText` whose normalizer refuses: named assertion `KeepOpen`; a
      synchronous `KeepOpen` focuses the released editor again (for
      `InlineText`, its child editor). Mutation: remove the delivery request,
      as in test 13;
    - a table's `DecimalInput` cell (with `on_modified` feeding the count)
      with a valid typed draft, and the same with a held step button at
      close: named assertions: the model holds the value when the policy
      runs, the table has closed the edit, the summary's count is zero and
      the decision is `Close`. Mutations: test 6's for the held press; for
      both, request the modified report after the session report (the count
      stays one and the decision is `KeepOpen`).

### Contract and compatibility tests

- **The policy table** (`draft_policy.rs`): one table-driven test per editor
  row of section 1 against each event of section 2's tables, asserting the
  written value, the shown draft, `on_modified`'s last report and the session
  log. Rows and cells that changed are covered by the tests above; the rest
  pass before and after the change. It includes the save-attempt inputs:
  `TextArea`'s Enter inserts a line break and saves nothing; with a session,
  Enter and Tab during a composition and during a selection drag save
  nothing and report nothing, and Escape there ends only the composition or
  drag. Mutation for the table itself: swap the focus-loss rule of the
  parameter row to save; the parameter cells fail.
- **Parameter number fields** (`draft_policy.rs` and the existing
  `controls/parameter_controls.rs:632`), for `IntegerInput::for_parameter`,
  `DecimalInput::for_parameter` and `InlineInteger::for_parameter`: an
  off-step or unparsable draft is kept with the outline and `on_rejection`
  reports it, with no host outcome; a refused admission (an edit already
  open on the parameter, `Busy`) replaces the draft by the base, calls no
  `on_save`, and the host's outcome monitor records nothing; a terminal
  refusal of an admitted edit (a conflicting host write between the
  field's begin and commit, through a held step gesture) replaces the draft
  and the monitor records the refusal; an unchanged Enter begins no edit; a
  remote authority's `Submitted` calls `on_save` and shows the submitted
  value. Mutation: keep the draft on a refused admission (the replacement
  assertion fails).
- **`InlineSlider`** (`draft_policy.rs`, existing coverage in
  `controls/parameter_controls.rs` and `editing/begin_edit.rs`): opening
  text mode begins the parameter edit (the host sees it begun); a refused
  admission (`Busy`) opens no text mode and creates no session; an
  unparsable value keeps the text with the outline and the edit open, with
  no callback; Enter with an unchanged value commits once; a terminal
  refusal of the admitted edit (a conflicting host write while typing) ends
  text mode and shows the parameter's value; Escape and focus loss cancel,
  and the host sees `Cancelled`. Mutation: begin the edit at confirmation
  instead of at text mode (the "begun" assertion fails).
- **Table completions** (`table_editing.rs`): a valid Enter closes the edit
  and focuses the body; a refused Enter keeps the edit open with focus in
  the editor; a departure with a valid draft closes the edit and focus stays
  where it went; a departure with a refused draft keeps the edit open and
  unfocused, and activating the cell focuses the editor with the draft (the
  existing `table_editing.rs:545`).
- **Custom owners keep their policy:** the existing
  `editing/draft_editor.rs:985` (hiding keeps the session) and the
  `diagram_editor` session's rename tests stay unchanged.
- **Close admission, focused editors** (`window_close.rs`): a bound
  `InlineText` saves before the policy runs; a controlled one proposes, and
  a callback that writes the model is settled by admission's flush, so the
  policy reads the new value; an `InlineInteger` text mode saves; an
  `InlineSlider` text mode is cancelled, with the host's outcome
  `Cancelled` and no commit; a focused `SearchField` with a pending
  debounced query writes it, and a read-only one discards it; the existing
  parameter case (`window_close.rs:96`) stays.
- **Close admission and sessions** (`window_close.rs`): a table's
  `TextField` cell with a valid draft reports `(FocusLeft, Saved)` and the
  table closes the edit within admission; with a refused draft it reports
  `Refused`, the edit stays open and `KeepOpen` focuses the editor again.
  `HeadlessApp` has no error backoff, so admission's path that flushes
  nothing is the native host's alone; this part does not change it.
- **Unchanged exceptions:** a disabled parameter number field cancels
  (`editor_permissions.rs:182`); disabling `InlineText` ends its draft
  (`foundation/redescription.rs:227`); disabling `SearchField` discards its
  query (`search_field.rs:592`); revoking an inline row mid-drag discards the
  drag (`inline_numbers.rs:138`); a disabled `TextField` keeps its draft and
  saves only after re-enabling (`input_normalization.rs:1079`,
  `editor_permissions.rs:50`); hiding and removing a session editor report
  nothing (`editor_sessions.rs:1461`); hiding a number field discards its
  draft (`controls/number_inputs.rs:79`).
- **Doctests:** `IntegerInput::on_modified`, `InlineText::on_modified`,
  `DraftEditorController::set_read_only`, `TextCommand::DiscardDraft`, and
  the guide examples that compile unchanged.

## 9. Scope, versions and evidence

- **`api/` diff:** added `DraftEditorController::set_read_only`,
  `TextCommand::DiscardDraft` (D3), `Table::on_edit_end`, `CellEditEnd` with
  its accessors and `CellEditEndReason` (D4), and `on_modified` on
  `IntegerInput`, `DecimalInput`, `InlineInteger`, `InlineDecimal` and
  `InlineText`, at each place `api/pert-ui-runtime/default.txt` and
  `serde.txt` record those types. The runtime prelude gains `CellEditEnd`,
  `CellEditEndReason`, `EditorCompletion`, `EditorOutcome`, `EditorTrigger`
  and `FocusDirection` (if L2 has moved it out), recorded in both runtime
  listings; the facade's listings (`api/pert-ui/*.txt`) gain the two new
  types at its root and the six names in its prelude, in every feature
  configuration, and change where they record `TextCommand`'s variants.
- **Breaking:** yes, in behavior.
- **CHANGELOG** for the part's version, citing this checkpoint:
  - `Breaking (N5):` revoking `enabled` on `IntegerInput` or `DecimalInput`
    bound to a signal keeps a typed or stepped draft unsaved, focused or
    not, as `TextField` does, instead of discarding it; a session's edit
    stays open with it; input cancellation other than hiding and removal no
    longer discards it. Replacement usage: `TextCommand::DiscardDraft`
    through `Runtime::text_command` or `EventContext::request_text_command`
    discards it without touching the binding.
  - `Breaking (N5):` a held step button on such a field no longer discards
    the draft when editing is revoked or focus leaves; focus loss then saves
    as without a press, and a session field reports `FocusLeft`. Replacement
    usage: Escape, or `DiscardDraft`, restores the saved value.
  - `Breaking (N5):` an open text mode of `InlineInteger` and
    `InlineDecimal` becomes read-only when editing is revoked, keeping its
    draft; it stayed editable with saves refused. Replacement usage:
    `DiscardDraft` ends it.
  - `Breaking (N5):` `SearchField` writes an edit it receives without focus
    at once, and discards a pending debounced query when focus leaves while
    editing is denied; it armed the debounce without focus and kept a
    refused query pending. Replacement usage: none needed.
  - `Breaking (N5):` a session field reached by a focus loss while disabled
    reports nothing; it reported `Refused`. Replacement usage: none needed;
    the edit stays open, as for a bound revocation.
  - `Changed:` a session field's `on_modified` report is delivered before
    its session completion.
  - `Added (F6):` `on_modified` on `IntegerInput`, `DecimalInput`,
    `InlineInteger`, `InlineDecimal` and `InlineText`, so a close policy and
    `FormFields` see the drafts these fields keep.
  - `Added:` `Table::on_edit_end`, with `CellEditEnd` and
    `CellEditEndReason`, reporting every cell edit's end, including those the
    table ends itself, one notification per end.
  - `Added:` to the prelude, `CellEditEnd`, `CellEditEndReason`,
    `EditorCompletion`, `EditorOutcome`, `EditorTrigger` and
    `FocusDirection`, the names an edit-end callback matches.
  - `Added:` `DraftEditorController::set_read_only` and
    `TextCommand::DiscardDraft`.
  - `Documentation:` contracts.md's "Draft policy", replacing the per-editor
    lists in the permissions and close sections, `App::on_close`, the
    controls chapter and the rustdoc named in section 6.
- **Version:** L4a was implemented as 0.3.72; L4b takes the next workspace
  version when it is implemented, planned as 0.3.75 after L2 (0.3.73) and
  L3 (0.3.74), with the internal requirements and `Cargo.lock`. If the
  order changes, the CHANGELOG heading follows the actual version.
- **Gate:** the public-API row, as the plan states for L4: the workspace
  checks (formatting, Clippy, tests, the packaged examples' headless
  sessions, which include `inventory` and `diagram_editor`, doctests,
  warning-free rustdoc, the diagnostics snapshots), the API listing gate,
  the focused tests above with their mutations recorded, and Codex's
  implementation review. No platform code changes: the native close path
  calls the same admission (CL:43-70), so the native checks are not
  required; if a desktop session runs them, the record says so.
- **Persistence, real time, backends:** no persisted schema, workspace hook,
  live-data endpoint or backend type is involved.

## Corrections to the plan and the inventory

- The citations have drifted: the revocation paragraph is now
  `contracts.md:350-357` (cited as `:355-362`), the close list `:1043-1051`
  (cited as `:914-923`) and editor sessions `:1666-1695` (cited as `:994`,
  `:1110`).
- The inventory's revocation line ("number inputs and `InlineText` end the
  edit", `complexity-inventory.md:268-270`) describes the code, but the
  number fields' own `session` documentation, contracts.md and
  `ColorPicker::enabled` describe the opposite; the inconsistency is inside
  one editor's documentation, not only between editors.
- The inventory counts five closing outcomes "by editor kind"
  (`complexity-inventory.md:271-272`). Closing has no rule of its own: the
  outcomes are the focus-loss column, and the close lists omit four editors.
- "Table/tree cell editors": `Tree` has no cell editor or session host;
  `Table` is the only built-in host (TE), and a tree row edits in place with
  `InlineText::in_row`.
- The plan frames `DraftEditor` as following its owner; that is a public
  guarantee for custom owners, so the unified policy binds the built-in
  owners only.
- Defects the plan did not name came out of the matrix: the stepped draft
  lost with no `FocusLeft` when focus leaves during a held press (rows 8,
  9), an unfocused kept number draft discarded by any later input
  cancellation (row 8), the in-place rows' editability (rows 11, 12), a
  search query that is armed without focus or outlives it (row 4) and the
  `Refused` report from a disabled session field (row 3).

## Not in this part

- **`MultiSelect`'s `OnClose` draft** (`guide/controls.md:135-137`) is a
  choice draft with its own commit modes, not a text or number editor, and
  had no friction; it keeps its rule (closing while denied discards).
- **`Select` as a session editor** has no draft text; its list choice
  completes the edit (C:1686-1687).
- **Custom `DraftEditor` owners' policies**, which stay theirs.
- **The inline rows' builder sets** (N8): `InlineSlider`'s missing
  `on_rejection` and the duplication the added `on_modified` extends, which
  the shared macro keeps to one implementation.

## Decisions for the maintainer

- **D1: what revoked enablement does to a local draft.**
  - **Recommended:** standalone editors keep it unsaved, focused or not
    (`TextField` and `TextArea` as today, the number fields changed);
    in-place editors end their session, parameter editors cancel and
    `SearchField` discards its pending query, as documented exceptions. A
    revocation never drops a standalone field's typed text: the kept draft
    is visible on screen and through `on_modified`, so a close policy can
    keep the window open, which is the principle F4's fix established; an
    in-place draft ends as Escape ends it, and `on_modified` reports that.
    It also makes three existing documents true.
  - **Alternative A:** every editor ends the draft unsaved, as Escape does.
    One rule without exceptions, but `TextField`, `TextArea` and
    `ColorPicker`'s hex field start dropping text on a transient revocation
    (a form disabled while it saves), a session's host would need a terminal
    report so its open edit does not lose its draft silently (a new
    `EditorTrigger` variant), and a close after a revocation can no longer
    see what was typed.
  - **Alternative B:** built-in in-place editors keep their session too,
    shown but unfocusable while disabled. Fully uniform, but `DraftEditor`
    would need an enabled state of its own besides `set_read_only`, and
    `InlineText` an editing state without focus or input, for a case with no
    friction.
- **D2: an open in-place session when editability is revoked.**
  - **Recommended:** it becomes read-only through the new public
    `DraftEditorController::set_read_only`, with the semantics in section 4,
    so rule 4 holds without an exception and a custom owner can do what the
    built-in rows do (AGENTS.md: no privileged path).
  - **Alternative:** the inline rows end text mode unsaved, as for
    enablement. No new API, but a second in-place exception, and the typed
    text is lost while the row keeps focus.
- **D3: how an application discards a kept draft** (new in revision 2).
  - **Recommended:** `TextCommand::DiscardDraft`, answered by every built-in
    editor also while disabled or read-only, with Escape's effect and no
    write. It reaches an unfocused or disabled field through the existing
    command paths, and keeps application data unchanged. It is aimed at the
    owning control; a `DraftEditor` answers `Unavailable`, so a custom
    owner's policy stays its own unless it answers the command.
  - **Alternative:** no new API; the documented route is describing the
    field with another binding (a second signal with the same value, a
    remeasure) or under another key (a remount that loses focus, undo and
    layout). Writing the bound value does not work (section 4).
- **D4: a draft that a host or removal ends** (revised in revision 3).
  Rule 10's ordering delivers `false` for every end the editor performs,
  including Escape and every completion a host acts on. A `Table` also ends
  edits itself (body Escape on an unfocused edit, TB:1730-1733 to
  TE:283-290; another edit beginning, TE:239-241; the row leaving by
  deletion or a filter, TE:300-312; a failed factory; hiding), and those
  remove the editor, which then reports nothing (C:784-785), so a count fed
  only by the editor's `on_modified` stays at one with no draft left. The
  application cannot clear it from the next factory call either: the factory
  runs in the host's update (TE:461-519), where `FieldHandle::set_modified`'s
  summary write is refused (`field_status.rs:358-372`), and body Escape runs
  no factory at all.
  - **Recommended:** keep component removal silent, as for every component
    today (`TextField` included), and give the host the cleanup contract:
    `Table::on_edit_end` (rule 11, section 4) reports each edit's end with
    its reason, one notification request per end, in an event phase
    where the application may write. Removing the table itself is the one
    unreported end, since the application performs it. It stays local to
    the table: a crate-private reporter child for the delivery slots, two
    new public types and six prelude names (section 4).
  - **Alternative:** a runtime-wide final report at removal: the runtime
    delivers a removed editor's outstanding `true` as `false` to its
    `on_modified` in the next event phase, with the context of its nearest
    mounted ancestor. It also covers removing the table and custom hosts,
    but needs a new delivery path for removed components, either general
    (changing C:784-785 for every notification) or special to this one
    callback, and changes `TextField`'s contract.

## Review record

- **Revision 1 (2026-10-27):** written from the code at 6360b00.
- **Revision 1, Codex review (2026-10-27, against eea2daa):** needs
  revision, with nine should-fix points and a nit; D1 and D2 were found
  defensible, and the additions need no persistence or import change.
  Revision 2 re-pins every reference to eea2daa and addresses each point:
  1. **Save attempts:** the tables are defined by save attempts and the
     inputs that initiate them; `TextArea`'s Enter and the busy-editor
     exceptions are kept and asserted in the compatibility table (section 2,
     section 8).
  2. **Custom owners:** the rules bind the built-in editors; a custom
     `DraftEditor` owner keeps its policy, and the hidden-owner retention and
     the diagram editor's invalid-rename discard stay valid and tested
     (section 2's scope).
  3. **Table cells:** qualified by outcome, with a completion table and
     tests for both the terminal and the `Refused` case (row 14, section 2,
     section 8).
  4. **Unfocused number drafts:** retention no longer depends on focus;
     `cancel_input` never discards, hiding does; test 2 and the close test
     cover the pointer-move sequence (sections 3, 5, 8).
  5. **SearchField without focus:** unfocused edits write at once, so no
     pending query exists without focus; direct and queued commands,
     editability revocation and close admission are tested (rule 8, test 10).
  6. **`on_modified` ordering:** the modified report precedes the session
     completion; test 14 asserts a zero count and an allowed close after a
     table save, and test 13 a callback error without replay. Host-ended
     edits are decision D4.
  7. **`set_read_only`:** capture release, publication and invalidation,
     application after the permission read, and deferred calls (repeated
     changes, `end`, a clipboard edit before a deferred lock), with capture
     and reentrant clipboard tests (section 4, test 9).
  8. **Discarding a kept draft:** "write the bound value" was ineffective; an
     explicit reset, `TextCommand::DiscardDraft`, is decision D3, with the
     rebinding and remount routes and their costs as the alternative.
  9. **Parameter outcomes:** validation refusal, protocol refusal, unchanged
     completion and accepted commit or submission are distinguished, with
     their drafts, callbacks and compatibility tests (section 1).
  10. **Versions:** L4a is 0.3.72; L4b takes the actual later version,
      planned 0.3.75 (section 9).
- **Revision 2, Codex recheck (2026-10-27, against 2a406a4):** needs
  revision; points 1 to 5 and the nit were resolved, D3 was found a sound
  replacement for rebinding, and keeping removal silent defensible. The code
  this design cites is unchanged at 2a406a4. Revision 3 addresses the six
  remaining points:
  1. **Host cleanup (D4):** `Table::on_edit_end` reports body Escape,
     replacement, a row leaving by deletion or a filter, a failed factory,
     hiding and terminal completions, in the table's event-phase delivery
     where `FieldHandle::set_modified` may write; table removal is the
     documented unreported end. It is presented beside the runtime-wide
     alternative, and test 15 shows the count returning to zero without
     another edit (rule 11, section 4, D4).
  2. **`InlineSlider` and admission:** `InlineSlider` has its own table and
     compatibility test (the edit begins when text mode opens, no
     callbacks, an unchanged Enter commits); refused admission (no session,
     no host outcome) is separated from terminal refusal of an admitted
     edit, for the number fields too. Behavior is unchanged (section 1).
  3. **Escape ordering:** every session report, Escape included, goes
     through the ordered path; test 14 asserts `false` and a zero count when
     a table cell's editor is escaped (rule 10, section 5).
  4. **`DiscardDraft` targeting:** the command is the owning control's;
     built-in in-place owners answer it, focused or not; a `DraftEditor`
     answers `Unavailable`; a custom owner keeps its policy unless it answers
     the command itself. Owner, focused-child and unfocused-session targets
     are tested (section 4, test 12).
  5. **Deferred locks:** a recorded change belongs to its session and is
     applied before `on_edit`, so calls from `on_edit` supersede it and a
     session begun there is never affected; both sequences and a callback
     error are tested (section 4, test 9).
  6. **Capture-release test:** it asserts routing to another component while
     the primary button is still held, before any release (test 9).
- **Revision 3, Codex recheck (2026-10-27, against 2a406a4):** needs
  revision; points 2 to 6 were resolved, and point 1's host-level solution
  was found right but its delivery contract in need of correction. Revision
  4 addresses the four points:
  1. **Prelude vocabulary:** `CellEditEnd` and `CellEditEndReason` live
     beside `CellEdit` in `table/editing.rs`, re-exported through
     `table.rs`, at the runtime root and in the runtime prelude, and so the
     facade's; with them `EditorCompletion`, `EditorOutcome`,
     `EditorTrigger` and `FocusDirection` join the prelude under L2's
     vocabulary clause, with a prelude-only callback example; the import
     rationale and the API listing manifest are updated (section 4,
     section 9).
  2. **Delivery slots:** one payload per notification request, on a
     notifier of their own (a crate-private reporter child), so ends never
     ride on the table's completion or housekeeping requests and keep the
     global order; test 17 covers the `clear_focus`, other component and
     hiding interleaving, a callback failure and a hidden window's
     resumption.
  3. **Callback replacement and removal:** replacement hands pending ends to
     the current callback; removal discards undelivered payloads while their
     positions and the table's cleanup stay, and ends recorded without a
     callback are never replayed; test 18 removes and reinstalls before
     delivery.
  4. **Replacement timing:** the before-factory assertion is dropped and the
     after-settling timing kept; test 16 covers direct F2 and deferred
     double-click replacement, each with one `Replaced` and correct per-cell
     accounting.
- **Revision 4, Codex third recheck (2026-10-27):** ready for approval. All
  four points resolved; one nit, applied by the author: the reporter's
  presentation mechanics (logically visible structural leaf, paints
  nothing, not focusable, rejects hits, zero extent in `Table::arrange`,
  stable notifier and identity, the callback context's qualification).
