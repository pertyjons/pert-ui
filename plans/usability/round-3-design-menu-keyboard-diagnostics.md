# Round 3 checkpoint: menu keyboard highlight and historical reveal diagnostics

Status: revision 3 approved for implementation 2026-10-29 by the
[final design recheck](runs/review-r3/menu-diagnostics-design-review-r3.md). Revision 1 review requested precise input transitions;
these are adopted below. The frozen-kit follow-up probes establish both leads. This
checkpoint does not change the blind comparison or acceptance criteria.

## Verified menu defect

Group N's P8 reproduces the same bug on head 0.3.88 and baseline 0.3.75:
a pointer parked while a menu is closed takes over the first keyboard selection
after F10/Down. Enter runs Gamma/Beta rather than the documented Save command.
In a long menu, scrolling on Down refreshes hover under the stationary pointer,
so repeated Down stalls: after thirteen Downs Enter runs R04, not R12. No host pointer event followed the
keyboard steps. Runtime::refresh_hover dispatches a refreshed Move; OptionList
replaces its active row on every Move that hits an enabled row.

### Policy and private state

Keep the generic pointer refresh contract and InputState processing. In the
shared OptionList, track which input owns the highlighted row privately:

The pointer owns highlighting only after acting on this list since its last
show, keyboard takeover or Leave. Keep the last observed window-space e.position() from every delivered Move
(host or refreshed), Press and Leave, recorded before editable checks. Never
clear this remembered coordinate on any reset, reconcile, visibility or Leave.
Ownership resets separately. A non-refresh Move gains ownership only when
its position differs from that last position and it is inside; a separator or
disabled row also counts as movement, but changes no active row. A same-position
host Move never reacquires ownership. Refresh still updates InputState and may
retarget an enabled row only while ownership holds.

- Primary host Press that actually arms the pointer gesture gains ownership
  and sets active to the pressed enabled row even without a preceding Move (`click_item` sends Press/Release). Preserve
  pressed identity and release matching.
- Wheel gains ownership only if the offset actually changes. A no-op wheel at
  a limit must not let a later unrelated refresh retarget the keyboard choice.
- Clear ownership before handled Pressed/Repeated Up, Down, Home, End, PageUp
  or PageDown; before handled Pressed on_right/on_left even if its action fails;
  and before an Enter/Space press that actually arms activation. A refresh
  between activation press/release must not change its row. Releases, idle
  Escape, ignored keys and modified navigation do not clear it.
- Clear on visibility_changed in both directions, input_reset and Leave.
  Select reconcile's active=None path and editable becoming false also clear.
  Keep ownership on menu rows_changed, option/label/enablement redescription
  and ordinary updates: pointer-driven lists must still follow changed data.
  Clearing everywhere pressed is cleared would break this distinction.
- No reconciliation/update signal publication. These are private interaction
  fields; generic pointer dispatch and callback ordering remain unchanged.

While keyboard activation is armed, pointer input updates hover but cannot
gain ownership or change active, including actual Move, Press and wheel. Add
a private ActivationState keyboard-armed query if needed; do not change public
input types. A wheel may retain its existing scroll behavior, but its refresh
cannot retarget the armed choice. Test each pointer path between Enter/Space
press and release, then Escape/release recovery.

P12 also verifies MenuTitle switching. Remember coordinates PER TITLE, not
bar-wide: a Leave to the old title carries the new target's coordinate before
Move delivery, so shared tracking would suppress genuine cross-title motion.
Record window-space position for every delivered Move, Press and Leave before
availability/action checks; never clear it on input_reset/focus loss, reconcile,
visibility or Leave. Switch only for a non-refresh Move at a distinct coordinate.
Hover styling still updates. Test title-to-title and title→content→same old title
coordinate round trips, duplicate motion after keyboard Left/Right reopening,
and every P12 sequence. Identical-coordinate window re-entry counts as stationary
until the next distinct motion; this is an explicit trade-off, not an OS dedup claim.

Newly shown ContextMenu/Select popups under a stationary pointer keep their
initial/selected row until actual pointer movement or interaction. Document
this visible correction in Menu/Select rustdoc and CHANGELOG.

This applies to menus/submenus and Select/InlineSelect's shared OptionList.
MultiSelect P13 is recorded separately in TODO.md; this checkpoint does not
change that distinct row family. It does not redesign QuickPick or other row families with different contracts.
No public flag, constructor or event type is needed. It retires synthetic
PointerKind::Leave as a workaround for reliable keyboard tests.

### Required evidence

Drive actual public input and read highlight, chosen value and callback effects:
parked pointer before F10/open; a prior content click beneath the menu; repeated
Down/End/Page navigation in a scrolling menu; a genuine Move after keyboard
navigation; wheel/data/layout refresh while pointer owns highlight; reopening,
submenu navigation and permission/reset lifecycle. Include Select, whose shared
code was not covered by the frozen probe. A negative control reverting ownership
gating must reproduce the wrong command/scroll stall. Native injected-input
evidence is separate from headless; record platform and no OS/human claim.

## Verified reveal diagnostic defect

P9 retains a genuine NotRevealed(AncestorRefused) from a closed Dialog. Its kind
and ancestor incarnation never change, but its Display/Debug changes from
mounted:true to mounted:false when Runtime drops. A failing test normally drops
its local Runtime before printing the error, so live handle Debug misrepresents
the historical refusal. WidgetId's own Debug is intentionally live and useful
elsewhere; do not change it globally.

### Policy

Implement RevealRefusal's existing Debug trait explicitly, printing refusal kind
and the immutable incarnation of each held WidgetId. Omit current mounted state
from this historical outcome. TestError's existing Display/Debug then becomes
stable without changing its fields or losing typed refusal/identity access.
Cover AncestorRefused, ApplicationControlled and Hidden; ModalBlocked stays the
same. No persisted schema or public signature changes.

The missing ancestor key/path is an optional diagnostic enhancement, not the
verified changing-state defect. Decline a new public snapshot/path field for
this fix: keys are not identity and would add capture/failure/ownership policy
to an otherwise stable typed outcome. The target key and stable ancestor id
remain available; the host can inspect it while the runtime lives. Document the
Debug format's identity-only purpose rather than suggesting unmounted was the
original reason.

### Required evidence

A real refused Driver click is formatted while Runtime lives, after the Dialog
opens, after unmount, and after Runtime drop/function return. Display and Debug
must remain byte-identical for the retained error/outcome, preserve the refusal
kind and immutable id, and omit live mounted state. Test every identity-bearing
refusal variant, without fabricating a wrong historical reason. Existing
WidgetId Debug still reports its current mount state. Keep no live UI lookup in
formatting.

## Integration and review

Read contracts, source and actual probe outputs before implementation. Update
Menu/Select and testing/reveal rustdoc plus affected guides and CHANGELOG
together. Give Button-row pointer/focus continuity and selected-tab removal
their verified public composition recipes; do not change Enter-only
VirtualList::on_activate or steal focus from another application region.
Record those recipes' tests separately from the two runtime repairs.

Run the required workspace/API gate, focused input/diagnostic/recipe checks,
relevant native interaction checks and fresh independent implementation review.
No repaired-head blind result, Windows/macOS or human-input claim follows.
Reviewers read source, author no fixes and do not delegate.


## Revision 2 evidence and diagnostic precision

The [revision-1 review](runs/review-r3/menu-diagnostics-design-review-r1.md)
and [P12–P14 probes](runs/triage-r3/extra-input-probes/README.md) are accepted.
Require public tests for every transition above, including click_item on a
branch then Right/Left, Right with a parked parent pointer, activation press /
refresh / release, no-op wheel, pointer-owned menu rows_changed, Select
redescription and availability revocation/restoration, and parked and long-list
Select/InlineSelect. MenuTitle tests include never/parked/Leave/same-coordinate
host Move/genuine Move/duplicate Move. Native injected-input checks include
same-position motion. A negative control must fail with the old gating.

The historical formatter prints `WidgetId { serial: N, .. }`, preserving the
existing enclosing variant/field names and supporting pretty Debug via
`debug_struct`/`finish_non_exhaustive`. Read only immutable serial, never Cell
or Runtime. The number is the value Runtime::inspection_id returned while that component
was mounted, meaningful only within that runtime. Document the consequence for RevealOutcome, RevealStatus,
TestError, RevealReceipt and nested Cleanup/boxed error formatting; WidgetId's own live Debug
stays unchanged. Test normal and pretty Debug plus Display byte equality across
the stated lifecycle points for each genuine refusal kind.

P14 did not establish a legitimate misclassified Hidden fallback. Do not change
its refusal model or add a TODO based on that lead. It did verify that a host-
reported hidden window causes Runtime::reveal/reveal_and_focus and Driver::click
to return RuntimeError::Hidden. Document that operational error and correct the
misleading internal comment; use explicit set_visible for Hidden-refusal tests.

For Button-row continuity, use a complete public recipe: mount an empty source,
obtain the list WidgetId, then publish Button rows whose callback sets active
and requests focus for that list. WidgetId does not retain the mount. Enter
thereafter invokes on_activate. No new API or Enter-only contract change.
For tab-removal refocus, the observable guard is origin Redescription and
cx.focused() on the Tabs recipient. This intentionally includes focus already
on the strip before removal; no historical-descendant claim is made. Defer with
the Tabs id and newly selected key, recheck ui.focused()==tabs, find the selected
content descendant through Runtime's public lookup, then focus that editor.
Test selected-content removal, already-focused strip, and other-region focus,
including focus moving elsewhere before deferred work. Button row's two-stage
source publication avoids a bootstrap slot; document that request_focus_for
silently ignores stale ids. No owner-retaining cycle is introduced.


## Revision 3 adoption of review 2

All corrections in the [revision-2 review](runs/review-r3/menu-diagnostics-design-review-r2.md)
are accepted above. Extend operational Hidden documentation to Driver::click_item,
queued request_reveal/request_reveal_and_focus via Finished(Err(Hidden)), the
RuntimeError::Hidden variant, RevealOutcome and content-search jump forwarding.
A host-hidden window can be retried after known native visibility resumes; it
is not an explicit set_visible refusal. Include RevealReceipt Debug after drop.
Update MenuBar rustdoc and the F10 guide recipe to require actual pointer motion
for title switching. The zoom/scale residual is accepted and must be documented:
native logical coordinates are rescaled while cached runtime coordinates are
not, so first same-physical-position motion after scale change may count as
movement. No global platform-coordinate change is made in this input repair.
Fresh final design recheck precedes implementation; required tests and separate
fresh implementation review remain owed.

The final recheck's nonblocking clarifications are accepted: same-coordinate
window re-entry is stationary for OptionList too and gets its own test; scale
residual wording belongs in MenuBar/Menu/Select rustdoc and CHANGELOG. Deferred
tab refocus rechecks that the queued key is still selected, ignores a missing
editor, and uses an application-unique editor key to avoid ambiguous lookup.
