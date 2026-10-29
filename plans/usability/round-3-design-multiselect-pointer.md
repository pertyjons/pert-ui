# Round 3 checkpoint: MultiSelect keyboard highlight

Status: revision 3 approved conditionally for implementation, 2026-10-29; revision-1 review conditionally
approved the architecture and requested the explicit transitions below. This is separate from the Menu/Select shared
OptionList patch. It is a verified new library usability finding within the
round's completion scope, recorded in TODO.md until repaired. Independent
design review precedes implementation.

## Evidence and responsibility

[P13](runs/triage-r3/extra-input-probes/README.md) reproduces on both frozen
versions: opening under a pointer parked over Gamma selects Gamma; Down cannot
leave it, and Enter toggles Gamma. A long list remains on O03 after twelve
Downs instead of reaching O13. Never/Leave/outside controls behave correctly.
Actual values and on_change logs establish the consequence, not passing scope
assertions. The source item is ChoiceRow::pointer_input; the probe calls it Row,
which is actually the separate data description type. Original evidence stays
unchanged; this is an author correction of its source item name.

MultiSelect uses retained individual ChoiceRow components inside VirtualList,
with a SearchField forwarding SearchKey to Shared::key. It does not use
OptionList. Keep these responsibilities and current choice/draft/write rules.
No new public API, signal publication from reconciliation, transport, event
flag or generic runtime/platform behavior is needed.

## Proposed private ownership policy

A Shared Cell<bool> records whether actual row-pointer interaction owns the
highlight. Start false on opening; clear on closing, panel show/hide/input
reset, pointer leaving the panel, dropdown permission revocation and every
handled SearchKey before its action. Query changes also clear ownership when
observed during Panel update, before pending rows are published; track query
revision privately. This includes text editing and clear/filter reset, rather
than treating typing as continued pointer intent. No signal is written by
these private resets.

Each ChoiceRow remembers window-space position from every delivered Move,
Press and Leave, even when unavailable, before early returns. Never clear this
coordinate on input reset, reconcile or visibility. A non-refresh Move at a
different coordinate inside the row gains ownership; when ownership holds,
Move may update the enabled row highlight as before. A refresh alone or a
duplicate host coordinate cannot gain ownership after keyboard navigation.
A primary press gains ownership for a genuine admitted pointer gesture;
release toggles only the same retained row under existing pressed/capture
rules, without changing notification or commit timing. Disabled options remain
untoggleable. Genuine motion over a disabled option counts as pointer motion
but cannot choose it. Specify and test how this interacts with active()'s
current early return rather than silently dropping coordinate observation.

A Row Leave must not clear shared ownership: during layout/scroll a refreshed
hit can move to another mounted row, and runtime synthesizes that Leave without
an is_refresh marker. Panel::hover_changed(false) clears ownership when the
pointer actually leaves the containing panel; ordinary transitions among rows
keep it. Test real row-to-row motion, pointer-driven data/scroll refresh and
keyboard-driven scrolling separately.

Wheel scrolls the VirtualList without itself taking highlight ownership. If
pointer interaction already owns highlighting, refreshed rows follow the
pointer; if the keyboard/search owns it, wheel preserves that highlight until
actual pointer motion or press. This is an explicit policy for this searchable
multi-value control. Avoid a privileged VirtualList offset observer solely to
force spelling/behavior symmetry with OptionList's direct wheel handler.
Do not advertise wheel as choosing an option.

SearchKey Submit acts on fresh Enter press, unlike OptionList's release-based
activation; no held-release row capture is needed. Keep SearchField's existing
IME, modified navigation and repeat behavior. Do not clear ownership on ignored
modified editing keys via a new global input route. A query revision observed
later still ends pointer ownership before results change.

Identical-coordinate window re-entry is stationary until distinct motion.
Native scale/zoom coordinate changes may count as motion, as documented for
the Menu/Select patch; no cross-platform coordinate fix is bundled here.

## Evidence and integration

Use public input and inspect actual highlighted rows, selected bound values,
on_change callbacks and draft commit/discard. Cover the frozen short/long
parked cases and controls; Up/Down/PageUp/PageDown and repeated keys; genuine
row motion after keyboard navigation; duplicate and Leave/re-entry coordinates;
wheel in both ownership modes and at the limit; dynamic options/labels/filter
and query changes; disabled options and dropdown permission revocation then
restore; hide/show and close/reopen; pointer press/release; both EachToggle and
OnClose with accepted writes and cancelled drafts. No callback/reconcile write
may be silently ignored. Include a negative control restoring unconditional
refreshed-Move highlight that fails on chosen values, not just paint.

Update MultiSelect rustdoc and controls guidance with initial/keyboard highlight,
actual pointer motion, stationary refresh and wheel policy, plus CHANGELOG.
Remove the verified TODO entry when fixed. Public signatures/schema stay the
same; generate API listings and inspect any diff. Run the required workspace
gate after integration, focused contract checks and independent fresh review.
Native evidence is limited to an actually run affected interaction scenario;
missing OS/platform/human evidence stays explicit. Frozen blind runs are never
relabelled as repaired-head tests.


## Accepted revision-1 corrections

All [review-1 requirements](runs/review-r3/multiselect-design-review-r1.md)
are accepted. Record Move, Press, Release and Leave coordinates before early
returns. Split option enabled from dropdown allowed: allowed motion (including
an already highlighted or disabled option) can gain ownership; disabled-option
press also gains it but cannot arm/toggle. Denied dropdown motion/press only
records position. Panel::update reads permission directly and clears ownership
when denied, including empty results. It also reads query directly and compares
its revision. Redescription of options, labels, filter, binding or commit timing
keeps ownership; a changed filter_generation alone is not a keyboard event.

Use the actual lifecycle hooks: begin, every dismiss/focus-within close, and
Panel visibility_changed. Panel is not a gesture owner, so its input_reset is
not relied on. Release after press A / SearchKey Down regains pointer ownership
when the retained pressed row still accepts its inside matching release; test
highlight, toggle and callback. Direct keys while the overflow VirtualList has
focus scroll it with the wheel policy, not Shared::key policy; test both modes.

Identical-coordinate window re-entry is stationary only when the same row is
still under it. A newly revealed row may have no coordinate memory and count
that host Move as distinct. Conversely a row that last saw P in its Leave,
then scrolled back under P, may treat a single jump back to P as stationary;
pin this accepted per-row residual. For actual panel-exit clearing, grow a
short popup under the pointer after moving just outside it: the refresh must
not seize keyboard highlight. Removing only that clear must fail. A no-op wheel
bubbling to an enclosing scroll area also cannot gain ownership.

## Verified long-list row-click focus defect

The review's source-only lead is now reproduced through public input in the
[current-source probe](runs/triage-r3/multiselect-focus-probe/README.md), working
0.3.91 at d479516. A short popup keeps SearchField focus after clicking O02;
a long popup focuses VirtualList. After a pointer Leave control, Down/Enter
and typing still fail to reach search: Enter writes no toggle and search field text stays
empty. This is independent of the stale-pointer defect. No frozen-baseline
or native evidence for this additional lead is claimed.

Include its repair in this control-family checkpoint. Explicitly document that
a successful primary row click returns focus to the popup search. The existing
"without moving focus" wording holds only for short lists and is replaced.
If the list already had focus by Tab, the click now intentionally returns to
search; direct Tab into the list still permits its ordinary keyboard scrolling.

Proposed private composition: Panel stores its mounted WidgetId in Shared
from its first UpdateContext::id, with private cleanup on unmount; it is neither
a public address nor signal publication. A successful matching row release,
after releasing capture/clearing pressed and applying an admitted write, requests
focus within that panel before reporting the write to the application through EventContext::request_focus_within. The panel
is not focusable and search is its first focusable descendant. Request focus
only for this admitted click; do not steal focus on hover, wheel, data updates
or an aborted/outside release. Preserve errors and accepted writes if a user
callback fails; do not replay on_change or undo a successful toggle.

Do not request this transfer while the pointer is still captured: leaving the
focused list can cancel its descendant's capture. The intended transfer occurs
on successful release, not midway through a held gesture. Between press/release
an overflowing list may still have focus and its keys follow the direct-list
scroll policy; the short-list SearchKey press/Down/release case stays distinct.
Tests cover short/long click→Down/Enter/type, Tab-list→click→search, outside
release, revoked permission, both commit timings, callback error/no replay and
cleanup/unmount. Use only public component/context facilities, no privileged
VirtualList focus override or new API. A fresh focused design recheck must
approve this related repair before implementation.


## Adopted revision-2 ordering condition and pins

The [revision-2 review](runs/review-r3/multiselect-design-review-r2.md)
approves implementation once its ordering correction is adopted; it is adopted
here before implementation. Split toggle admission/write from reporting. For
an inside matching enabled-option release: clear pressed/release capture,
set highlight and regain pointer ownership, perform the permission-rechecked
write, request focus within Panel only if admitted, then report an EachToggle
write. OnClose still edits its draft and does not report a bound write until
its existing committed-close path. No report is made for refused admission.

The focus request precedes on_change so an application's later explicit focus
request wins. If reporting fails, preserve the accepted value/draft and queued
search focus, propagate the error and never replay; test the next flush's focus.
Keep callback borrows out of application code as today.

Pin long-list press/Down/release: Down scrolls the directly focused list; capture
stays with the original row, and release outside its new clip cancels the click
without toggling or returning focus. Disabled-option presses gain ownership
but do not arm, and denied-dropdown presses only record position; these
incomplete clicks can leave the long list focused. Document the policy as
"a completed primary click on an enabled option returns focus to search".
A Tab-focused list also returns to search on such a completed click. Hover,
wheel, data updates and cancelled/incomplete clicks request no focus.

The existing press-time move to the list cancels SearchField preedit before
a completed release returns; a short list keeps its composition. Document/test
this IME residual, not restoration of uncommitted composition. MultiSelect sets
no debounce. Refresh the stored Panel id on every update; unmounted clears
only if Shared still holds that same panel identity. Removal during capture,
then release, yields no error or focus theft. Update both the private row
comment and public no-focus-change promise. All required review pins are
accepted; no additional architecture/API decision remains before implementation.
