# Plan: improvements found by a diagram-editor probe

Status: proposed 2026-10-19 after an independent Codex review of the
findings; implemented in 0.3.18 to 0.3.23 and squash-merged into `main` as
0ba201b (slice 0 in 0.3.18, slice 1 in 0.3.19, slice 2 in 0.3.20, slice 3 in
0.3.21, slice 4 in 0.3.22 and its review fixes in 0.3.23). The API sketches
below are the design as proposed; each slice's status line says where the
implementation differs.

## Background

On 2026-10-19, after the notes application of
[app-author-improvements](../app-author-improvements.md) and the mixer panel of
[mixer-probe-improvements](mixer-probe-improvements.md), the maintainer's
session built a third application against the public facade only, chosen to
exercise what neither did: a custom widget built from the public extension
points. A diagram editor's canvas is a custom container with one child
component per node, pans and zooms through `child_transform`, stacks its
children, draws edges under them and the selection and gesture feedback over
them, and handles selection, drag, rubber-band selection, connecting nodes,
keyboard nudging and an in-place label editor. Around it: an
application-owned document and undo history, commands with shortcuts in a
`MenuBar`, a `ContextMenu` and a `CommandPalette`, an outline `VirtualList`
sharing the selection, a `PropertyGrid` inspector built with `Dynamic`, a
`ColorPicker` popover, an open dialog built from `FileBrowser` and `Dialog`,
saving and opening through `Runtime::spawn`, PNG export through a second
runtime and the CPU renderer, a `ToolWindowLayout` whose arrangement is
saved as a workspace document, and close admission with an `AlertDialog`.
A headless session drives it with pointer and key input (including
shortcuts through `command_key_input`) and asserts the document, selection,
files and an idle flush; the maintainer ran it natively on Fedora 44
GNOME/Wayland and found it worked well once the drag loop below was fixed in
the application. The application lived in a session scratchpad; the
observations are reported experiment results, not a reproducible evidence
record.

What worked well: the custom container contract (`take_children`/
`request_children`, `arrange`, `child_transform`, `stacks_children` with
`child_stacking_priority`), retained drawing, `Painter` paths, shapes and
text, pointer capture, `cancel_input` and `focus_lost`, the view
transform's `apply`/`invert`/`zoomed_at`, commands with enabled signals in
menus, buttons and the palette, `FileBrowser` over host listings, background
save and open, the CPU renderer for export, workspace restore and deferred
close admission.

What the author ran into, verified in code (file references are to the
checkout on 2026-10-19) or by execution:

- **A layout-generated pointer move.** After every layout or stacking change
  the runtime replays the last pointer position as an ordinary
  `PointerKind::Move` (`crates/pert-ui-runtime/src/runtime/pointer.rs`,
  `refresh_hover`, called from `refresh_stack_hover` in `runtime.rs`), and
  capture routes it to the capture owner. It cannot be told apart from real
  motion. The probe's drag handler invalidated layout on every `Move`; each
  layout produced the next `Move`, and the flush failed with `UpdateLimit`,
  so the first native drag did nothing. Neither `PointerKind::Move` nor
  `Component::pointer_input` mentions the replay; the stacking contract and
  the layout guide mention refreshing stationary hover only. The
  `UpdateLimit` report names the pending component and suggests
  "scheduling/focus feedback", without the invalidation's origin. Built-in
  components that move geometry from input guard against repeats (`Split`,
  `ZoomView`, floating-window placement), so a guard is the existing
  convention, but it is not stated for custom widgets.
- **Observing a user's write.**
  - `ColorPicker` writes its binding once per completed edit and has no
    `on_change`; an application that commits a picked color to its own
    document and undo history must observe the binding. The probe forwarded
    it through a mailbox from a component's `update`, which can lose an
    edit when the mailbox is full; the documented way is a `Notifier`
    request from `update` and the write in `deliver_notifications`, which
    the probe did not find.
  - `TextField`/`TextArea` call `on_save` only in draft mode, which only a
    normalizer or a length limit enables. The probe added a length limit
    merely to get a save on Enter or focus loss; an identity normalizer,
    `.normalize(Ok)`, does the same without a limit, but nothing near
    `on_save` says so.
  - Signals are not writable projections of an application model: the
    inspector creates fresh field signals per `Dynamic` description and
    commits through `on_save`/`on_change`, and an unrelated change to the
    selected node redescribes the panel and drops another field's draft
    (`TextField` drops a draft when its binding changes). The probe also
    tried to synchronize the picker's binding from the `Dynamic` callback,
    where writes are refused, and discarded the error, so the picker kept
    showing the previous node's color. There is no documented stable
    view-model pattern for an inspector over an application document.
- **Widget-author mechanics without a public helper or example.**
  - `PointerEvent` has no click count; the application times double clicks
    with `EventContext::now`, while `Knob` recognizes them privately.
  - A pointer handler sees only `EventContext::modifiers`; a Space-drag pan
    tracked Space through `keyboard_event`, which reaches only a focused
    component. `Runtime::register_held_input` serves this, but no guide
    connects it to custom widgets.
  - An in-place editor was a fresh `TextField` child: focusing it needed the
    canvas to turn non-focusable and call `request_focus_within` on itself,
    and its caret starts at the beginning with no way to select the text.
    The retained `DraftEditor` with `DraftSelection::All` serves this, but
    nothing points a container author to it from `TextField` or the layout
    guide.
  - Overlapping children fail development validation with
    `UnexpectedOverlap`, whose message says "explicitly allow an
    intentional overlay" without naming `OverlapPolicy::Allow` or
    `stacks_children`.
- **Application structure.** `App::on_close` and `App::on_error` receive the
  runtime but not the state the initializer created, so the probe shared it
  through an optional slot; it also registered commands before the state
  that held their ids existed, which is a structure the probe chose, not a
  requirement (Codex refuted that as a library problem). Keeping the
  canvas's selection and the outline's `ListSelection` consistent was manual;
  one `ListSelection` can be the canonical node selection.
- **Export.** Rendering a document at its own size took a second `Runtime`
  with its own signals; capturing the current frame needs none, and
  `Runtime::theme().set_mode` selects a light theme for it. Neither is
  explained where export would be looked for.
- **Documentation errors and gaps.** `docs/manual.md` says `on_save`
  reports "whether Enter, focus loss or a step button saved it"; the
  callbacks receive the context and the value only. `stroke_shape` takes a
  `Length` and strokes inward, while `stroke_path`, `stroke_polyline` and
  `stroke_ellipse` take a `StrokeStyle` and stroke centered, without a
  cross-reference. `EventContext::now` is the time of the last flush, which
  a headless click test must advance.
- **Probe defects found by the review**, recorded so the example in slice 4
  does not repeat them: a middle-button pan ended on either button's
  release; dialog buttons queued `EventContext::defer` work owned by a
  widget the same action hides (it ran, because the task dispatched before
  the dialog hid, but the receipt was ignored and `defer_detached` states
  the intent); inspection reported ad-hoc properties instead of
  `Semantics`, and the in-place editor had no accessible name.

## Working rules

The rules of [app-author-improvements](../app-author-improvements.md#working-rules)
apply unchanged: one slice per branch with its rustdoc, examples, gallery,
manual and CHANGELOG; the maintainer's patch version bump per slice; an
independent Codex review before a slice is called merge-ready; the contracts
read before each slice; and the gate table there. Breaking changes are
permitted while the library is unpublished, with replacement usage recorded
and repository consumers migrated in the same slice.

Additionally:

- **Design checkpoint.** Slices 1, 2 and 3 begin with a chosen alternative,
  a compiling API sketch and an enumerated list of contract tests, reviewed
  before implementation starts.
- Slice 1 changes input routing and takes the public-API/scheduling row of
  the gate table; any change to what the native runner delivers needs native
  evidence recorded in [verification](../../docs/verification.md).
- `TODO.md` entry TEXT-01 (large-buffer `TextArea`) touches the text editors
  that slices 2 and 4 use; it stays out of scope.

## Slices in order

### 0. Immediate documentation repairs

Status: implemented in 0.3.18. Test:
`crates/pert-ui/tests/editing/input_normalization.rs`
(`the_identity_normalizer_saves_finished_edits_without_changing_them`) and
the `EventContext::now` doctest.

- `docs/manual.md`: correct the `on_save` sentence to describe what the
  callback receives; state that saves happen on Enter, focus loss or a step
  button without promising a reason value.
- `TextField::on_save`/`TextArea::on_save`: point to `.normalize(Ok)` for a
  field that should save on confirmation without other validation.
- `Painter::stroke_shape` and the centered path strokes: cross-link the two
  conventions and state where each stroke falls relative to its bounds.
- `EventContext::now` and `guide/custom-widgets.md`: state that the clock
  advances through `flush`, with a double-click test as the example.
- The `UnexpectedOverlap` message and the layout guide name
  `OverlapPolicy::Allow` (for document-order overlap without raising) and
  `stacks_children` (for raised overlap).
- Shipped as 0.3.18, since the corrected rustdoc is packaged.

### 1. Pointer refresh after layout

Status: implemented in 0.3.19 as the checkpoint below decided. The evidence
names the component that handled the refresh (an ancestor of the target when
the event bubbled). Tests: `crates/pert-ui/tests/foundation/pointer_refresh.rs`,
the `Component::pointer_input` doctest and the motion coalescer's unit test.

- Decide how a custom widget can tell the layout-generated refresh from
  physical motion. Alternatives: a distinct `PointerKind` variant (the enum
  is `non_exhaustive`, but existing handlers would then miss stationary
  subregion changes they rely on today); an origin flag on `PointerEvent`
  (`PointerEvent::is_refresh` or a `PointerOrigin`); or not replaying to a
  capture owner whose capture began before the layout, with stationary
  subregions recomputed only for hover. Whichever is chosen, built-in
  components that react to stationary subregions (split tracks, tab titles,
  stacked children) keep working, and a native coalesced move is still a
  physical move.
- Document on `PointerKind::Move`, `Component::pointer_input` and
  `EventContext::invalidate` that a handler must be idempotent for an
  unchanged position, with a complete drag example.
- The `UpdateLimit` diagnostic records bounded evidence of what kept the
  flush busy, such as the component and callback that invalidated in each
  repeated wave, so the report points at a pointer-move feedback loop when
  that is the cause. It stays bounded and debug-only, like the validator.
- Contract tests: an unguarded custom drag that invalidates layout on every
  move settles (or, under the documentation-only alternative, the
  diagnostic names it); a stationary pointer over a stacked child still
  updates hover after a raise; capture keeps its owner; the native runner's
  coalesced moves are unaffected.

#### Design checkpoint (2026-10-19)

Investigation: `refresh_hover` (`runtime/pointer.rs`) runs from
`settle_notification_presentation` after any layout or stacking change and
calls the ordinary `pointer_input` with `PointerKind::Move` at the last
position. That move goes to `captured.or(hit)`: the capture owner while one
exists, otherwise the hit target, while the hover path follows the hit
target. Built-ins depend on it: split and tab subregion hover, tree
expanders and scrollbar hover need the stationary refresh, and captured
split resizing, zoom-view panning, scrollbar thumb dragging and text
selection recompute from current local geometry, which can change under a
stationary pointer (a `ScrollArea` whose content grows while its thumb is
held). Split and zoom view already skip invalidation when the resulting
state is unchanged. List, tree and tab reordering run as drag sessions, for
which `refresh_hover` returns early; their stationary auto-scroll retargets
through `service_drag`, not through a refresh move. Suppressing the refresh
for capture owners, or a separate `PointerKind` variant, would therefore
break components that rely on it.

Chosen alternative: no change to routing. The refresh becomes identifiable,
the convergence requirement is documented where authors look, and the
development diagnostic reports evidence of a refresh feedback loop.

- `PointerEvent::is_refresh(self) -> bool`, true only for the move the
  runtime delivers after layout or a stacking change; `PointerEvent::new`
  and every host-supplied event are false. A private field, so equality
  distinguishes a refresh from a host move at the same position; no public
  constructor makes one. Delivery, bubbling, capture and hover are unchanged.
- `PointerKind::Move`, `Component::pointer_input`, `EventContext::invalidate`,
  the input contract in `docs/contracts.md` and the layout guide state that a
  move can repeat the last window position after layout, and that a handler
  must converge: it must not invalidate again when the state it derives from
  the event (a local position, a resolved drag delta, a hit subregion) is
  unchanged. Comparing window positions alone is not enough, since layout
  can change what a position means. Ignoring refreshes is a policy only for
  a component whose result depends on the window position alone. A doctest
  shows a drag that compares the derived state and settles.
- Debug builds record, per outermost flush, how often a refresh move
  delivered to a target left new work scheduled (a pending update or
  layout) when its dispatch returned; refreshes that schedule nothing are
  not counted, so an inert hover target is never named. When the flush ends
  in `UpdateLimit` and some target's count is at least two, the `UpdateLimit`
  validation message adds that evidence, naming the target and the count and
  pointing to `PointerEvent::is_refresh`, as a clue rather than a verdict.
  The record lives as long as the outermost flush, is captured before its
  guard is dropped, and is bounded by the wave limit; release builds keep
  none.
- No version-compatibility concern: an added accessor on a public struct
  whose fields are private.

Contract tests (facade, `foundation` area, unless noted):

1. A custom component that captures on press and invalidates layout on every
   move fails with `UpdateLimit`; in debug builds the validation message
   names it as a refresh recipient that scheduled work.
2. The same component guarded by its derived drag state settles.
3. `is_refresh` is false for a host `Move`, including one at the same
   position, and true for the move received after a layout with a stationary
   pointer, under capture and for hover.
4. A capture owner whose geometry changes under a stationary pointer receives
   a refresh with its new local position.
5. An unrelated loop (arrangement that requests an update while update
   requests layout) with the pointer resting on an inert button keeps the
   generic message.
6. After a refresh loop, a corrected component settles and a later unrelated
   `UpdateLimit` carries no stale evidence.
7. A drag session (list reorder) receives no `is_refresh` move while it is
   active; existing auto-scroll coverage stays green.
8. Release builds: test 1 still returns `UpdateLimit` and test 2 settles
   (the workspace runs release-profile tests).
9. The native motion coalescer's events are host events (`is_refresh`
   false), in the facade's platform unit tests.

Codex reviewed the first version of this checkpoint (2026-10-19): applied
the corrected rationale (drag sessions do not receive the refresh;
`captured.or(hit)` routing), the convergence rule in place of a
window-position rule, evidence counted only for refreshes that scheduled
work instead of a share of waves, and a working fixture for the unrelated
loop, plus the lifetime, drag-session, release and native-motion tests.

### 2. User-write notification for ColorPicker and an explicit save mode

Status: implemented in 0.3.20 as the checkpoint below decided. The tests
found that a hue step on a grey rewrote the binding with a value rounded
through HSV; a commit whose state still describes the bound color now keeps
it exactly. Tests: `crates/pert-ui/tests/controls/color_picker.rs`
(`on_change_*`, `a_failing_handler_*`, `a_handler_that_writes_*`,
`the_handler_follows_redescription_in_both_forms`) and the `on_change`
doctest.

- `ColorPicker::on_change` reports each committed user write (drag release,
  key step, saved channel or hex field) once, after the binding holds the
  value, in the event phase; not application writes, cancelled drags or
  restored workspace state. Callback errors propagate without rollback or
  replay, as for the other user-write callbacks. The popover form forwards
  it. Decide whether `Permission` revocation mid-drag reports anything (it
  cancels, so it should not).
- Decide whether text editors get an explicit save policy independent of
  normalization and limits (for example a builder that enables draft mode by
  itself), or whether `.normalize(Ok)` documented in slice 0 is the answer.
  Choose at the checkpoint; do not add both.
- Contract tests: one callback per completed picker edit across pointer,
  keyboard and text entry; none for an application write, a cancelled drag
  or an unchanged value; a failing callback leaves the binding written.

#### Design checkpoint (2026-10-19)

Investigation: every picker write goes through `Model::commit` (pointer
release and key steps of the area and strips) or `Model::commit_color` (the
hex field's and channel fields' `on_save`), each writing the committed HSV
state, the bound color and the entry fields. Both run in event phases with
an `EventContext` available at the call site. A commit can leave the bound
color unchanged while changing the HSV state (a hue drag at zero
saturation), so "changed" must be judged on the binding. `into_popover`
builds a `Popover::owned`, whose `retain_initial_children` discards incoming
descendants on redescription: a handler on a redescribed popover form would
never reach the mounted picker unless the popover carries it. `Popover::owned`
has no other user.

Chosen alternative:

- `ColorPicker::on_change(self, handler: impl FnMut(&mut EventContext, Color)
  -> Result<(), RuntimeError> + 'static) -> Self` reports each committed user
  write that changed the bound value, with the value written, after the
  binding and the entry fields hold it, in the event phase of the input that
  committed it: a pointer release, a key step, a hex save or a channel save.
  Application writes, cancelled or revoked gestures, edits that leave the
  bound value unchanged and refused text never call it. An error propagates
  from the input call that committed, with the binding written, and is not
  replayed; the next user write calls the handler again.
- The handler follows redescription like every callback: a same-key
  description replaces it, and one without a handler removes it, while the
  binding and options stay with the mounted picker as today. The popover
  form carries the handler of its new description to the retained picker
  (a crate-private slot shared by the picker's model and its `Popover`); the
  open state, anchor and drafts are kept as documented.
- Text editors: no new save-policy builder. The identity normalizer
  documented in slice 0, `.normalize(Ok)`, is the explicit way to get saves
  without other validation; a second spelling of draft mode would give two
  ways to say one thing.
- `docs/contracts.md` (user-write callbacks), the manual's `ColorPicker`
  row, the picker's rustdoc ("Writing the binding" now names the callback as
  the way to record undo) and the gallery's color picker page show it.

Contract tests (facade, `controls` area, `color_picker.rs`):

1. One call per completed edit, with the written color, for an area drag
   release, a hue key step, a hex save and a channel save; the binding holds
   the value when the handler runs.
2. No call for an application write, an Escape-cancelled drag, a drag ended
   by revoking the permission, a key step at the end of a strip that changes
   nothing, a hue change at zero saturation and a refused hex text.
3. A failing handler's error is returned from the input call, the binding
   keeps the new value, and the next edit calls the handler again.
4. Redescription with another handler calls the new one; redescription
   without one calls nothing; both for the plain picker and for the popover
   form, where the open popover, focus and a typed hex draft survive.
5. A handler that writes another color to the binding: it receives the
   user's color once, the application's color survives, the entry fields
   follow it and no second call happens; also when it then fails, for a
   part commit and a text save.
6. A hue Home/End on a saturated color (the retained hue changes, the color
   does not) and a repeated endpoint step call nothing; alpha-strip pointer
   release and alpha Home/End call it; key presses and repeats call it,
   releases do not.
7. Hiding the picker during a drag, showing it and releasing calls nothing;
   a hex or channel draft saved by ordinary focus transfer calls it once,
   while hiding or removing the picker saves and calls nothing; a failing
   handler on a text save propagates and keeps the write.
8. Revoking a bound permission during a drag calls nothing; after granting
   it again a stale release calls nothing.

Implementation notes from the review: the change is judged on the bound
color before and after the commit, with the HSV bookkeeping kept even when
no call follows; all commit bookkeeping (including the entry's sync state)
finishes before the handler runs, and no internal borrow is held across it;
the plain picker takes the next description's handler before its
permission-equality early return, and the popover form moves the incoming
handler (or `None`) into the mounted picker's shared slot before its
retained-state early return.

Codex reviewed this checkpoint (2026-10-19): the investigation, the
contract's consistency with the other `on_change` callbacks and the
decision against a new save-mode builder were confirmed; tests 5 to 8 and
the implementation notes above were added from its findings.

### 3. Click recognition for custom widgets

Status: implemented in 0.3.21 as the checkpoint below decided. Saturation
is documented but not tested (it needs four billion presses). The reveal
button's reset and `InlineText`'s reset on focus loss have tests; the clear
button's and the scrollbar's resets kept their call sites but have no test
of their own (clearing empties the text, and the scrollbar press is
covered only through its own scrolling tests). Tests: `crates/pert-ui/tests/foundation/click_counter.rs`,
`controls/knob.rs` (`a_double_click_needs_*`, `a_refused_press_*`),
`collections/file_browser.rs` (`four_quick_presses_*`),
`editing/inline_controls.rs` (`inline_text_edits_on_*`),
`editing/text_fields.rs` (`a_drifting_series_*`) and the `ClickCounter`
doctest.

- A public, deterministic click recognizer that custom widgets and the
  built-in controls share: successive presses of the same button within an
  interval and distance on the same component count up; the checkpoint
  below assigns resets to the component (a move or capture loss does not
  reset the helper by itself).
  Alternatives: a `ClickCounter` helper a component owns and feeds, or a
  click count on `PointerEvent` filled by the runtime. Timing is the
  runtime's deterministic clock; a platform's configured interval is used
  only if the native adapter actually supplies it.
- Move `Knob`'s private double-click recognition onto it.
- Contract tests: double and triple clicks, a slow second click, a moved
  second click, a different button, cancellation between clicks, and
  headless determinism.

#### Design checkpoint (2026-10-19)

Investigation: four private recognizers exist. `Knob`
(`numeric_controls.rs`, `DOUBLE_CLICK`, 500 ms, no distance), the file
browser's entry rows (`file_browser/view.rs`, the same constant), `InlineText`
(500 ms) and the text editors (`text_fields.rs`, 500 ms and four logical
pixels, counting one to three for word and line selection). Each keeps its
own last press and resets it on its own cancellation paths. The native
adapter supplies no platform double-click interval (winit reports none), so a
platform-configured interval is out of reach without new platform code.

Chosen alternative: a component-owned helper rather than a count on
`PointerEvent`. The runtime sees focus, capture and routing, but not what a
component accepted or consumed: the knob arms a series only after a press
began an edit, and treats a press that moved the value as a drag.

API sketch (module `click`, re-exported at the crate root; the prelude
takes only types that `Component` signatures name, so it stays out):

```rust,ignore
#[derive(Clone, Debug, Default)]
pub struct ClickCounter { /* interval, distance, last press, count */ }
impl ClickCounter {
    pub const DEFAULT_INTERVAL: Duration; // 500 ms
    pub const DEFAULT_DISTANCE: Length;   // 4 logical pixels
    pub fn new() -> Self;
    pub fn with_interval(self, interval: Duration) -> Self;
    pub fn with_distance(self, distance: Length) -> Self;
    pub fn press(&mut self, at: Duration, position: Point, button: PointerButton) -> u32;
    pub fn press_event(&mut self, cx: &EventContext, event: PointerEvent) -> Option<u32>;
    pub fn count(&self) -> u32;
    pub fn reset(&mut self);
}
```

- A press continues the series when the previous press was of the same
  button, at most the interval earlier (not later: an earlier time starts a
  new series), and within the distance of the previous press on each axis,
  in window coordinates; this is the text editors' rule today. Otherwise it
  starts a new series at one. The count saturates at `u32::MAX`; a component
  that cycles keeps its own bounded state and uses `count > 1` as "this press
  continues a series", and does not reset between the steps of its cycle.
- Zero is a valid interval (only presses at one instant count) and a zero
  distance requires the same position, so the builders need no error;
  `Length` already validates the distance. A builder replaces its value and
  applies from the next press.
- The helper never resets by itself on moves or other events; `press_event`
  feeds `PointerKind::Press` only and returns `None` for the rest. Callers
  reset where they do today: on cancellation, focus loss, a gesture that
  consumed the press, a failed admission, a replaced source, and after
  acting on a pair. Releasing capture is not a reset.
- Migration, keeping each component's state machine: the knob feeds only
  presses that are eligible, acts on a count of two or more and then resets,
  resets when a begin is refused and on every path that clears it today
  (displacement, parameter replacement, `input_reset`); the file browser
  rows and `InlineText` act on a pair and reset, keeping their other resets;
  the text editors replace their timestamp and position with the helper,
  keep their `1 → 2 → 3 → 1` selection cycle (a triple click selects all
  text) and reset where they reset today (focus loss, `input_reset`,
  cancellation, the clear and reveal buttons, a scrollbar press). The knob,
  the rows and `InlineText` gain the four-pixel distance; their
  documentation says so.

Contract tests (facade, `foundation` area for the helper, the component
areas for the migration):

1. Counts for presses at the interval boundary and just past it, at the
   distance on one axis, on both axes diagonally and just past it, a series
   drifting by less than the distance per press, another button, an earlier
   time, a zero interval and a zero distance; saturation.
2. `reset`, `count` and replaced builders; `press_event` ignores non-press
   kinds and uses the runtime's clock.
3. Knob: a double click resets to the default within the distance and not
   beyond it; a refused first press does not arm; a drag between presses
   does not count; four quick clicks reset twice.
4. File browser row: four quick presses activate twice; a press five pixels
   away does not activate.
5. `InlineText`: a double click begins editing, a distant second press does
   not; focus loss between presses resets.
6. Text editors: double and triple clicks keep selecting a word and all text,
   a fourth click starts over, the concealed field selects all on a double
   click, and the clear button, reveal button and scrollbar presses reset.

Codex reviewed the first version of this checkpoint (2026-10-19) and found
that measuring from the first press, Euclidean distance and helper-side
resets would change text selection and the paired activations of the knob
and the rows; the rule, the reset ownership, the pair handling and the tests
above follow its findings, and the threshold API takes `Length` and an
infallible interval instead of new error variants.

### 4. A custom canvas example and the widget-author guide

Status: implemented in 0.3.22: `crates/pert-ui/examples/diagram_editor/`
(headless session by default, `--window` with `native`) and the guide's
`custom_widgets` chapter with its two doctests. Decisions taken while
implementing: held input is offered keys before the focused control, so the
canvas's Space registration takes the key only while the canvas has focus
(otherwise Space could not press a focused button); the canvas records its
size in shared state during arrangement for the zoom commands, instead of
looking itself up by path; the open dialog's listing mailbox and the
`FileBrowserController` it delivers to reference each other, so the example
fills one slot there, and close admission shares the application with
`App::on_close` through a slot as the chapter describes.

- Ship the diagram editor, reduced to what it demonstrates, as a package
  example beside `live_mixer.rs` (headless session by default, `--window`
  with the `native` feature): a custom container with stacked children,
  pan and zoom, idempotent drag invalidation, a button-owned middle pan,
  Space-drag through `register_held_input`, a retained `DraftEditor` for
  in-place renaming with `DraftSelection::All`, one `ListSelection` shared
  by the canvas and an outline, a `Notifier`-based commit of the picker
  binding (replaced by `on_change` once slice 2 lands), `defer_detached`
  for work started by a closing dialog, typed `Semantics` for nodes and
  CPU export of a light-themed document view.
- A widget-author chapter in the guide: the event, update and notification
  phases and which may write; observing a binding from a component;
  focusing a child the component has just described; held keys in pointer
  gestures; overlap and stacking; inspection semantics. Each section links
  the example.
- A stable view-model pattern for an inspector over an application
  document: field state retained per selected identity, synchronized from
  the document in an event phase, with a stated conflict policy, so a
  redescription does not drop another field's draft. A general writable
  projection of a model field stays out of this plan (see below).
- An application-construction section: shared state first, commands
  registered against it, command ids kept apart, and how `on_close` and
  `on_error` reach the state (the shared-slot pattern, with initialization
  failure covered).

## Optional follow-ups

Taken up only when an application path needs them, each with its own
checkpoint: a writable projection of a field of an application model into a
control binding (it must define validation, write revisions, transaction
failure and external-write conflicts, and route writes through the
application rather than treating a document mutation as an undo step or an
authoritative acceptance); a typed state handoff from `App::new` to its
lifecycle callbacks; an initial caret or selection policy for `TextField`
focused from code. They are in the [follow-up index](../follow-ups.md); the
third is partly answered by `TextField::session` with `EditorStart`
(table-editing plan, 0.3.39).

## Not in this plan

Accessibility integration, an undo framework owned by the library, a
diagram or node-graph widget (the canvas stays an application component),
and large-buffer text editing (TODO.md TEXT-01).

## Review record

- 2026-10-19, Codex (`codex exec -s read-only`), review of the findings log
  and the probe sources: confirmed the layout-generated move as the main
  library issue; found public solutions the author had missed
  (`Notifier`, `register_held_input`, `.normalize(Ok)`, `DraftEditor` with
  `DraftSelection`, `UpdateContext::children`, `Runtime::theme`,
  `OverlapPolicy::Allow`); refuted the claim that command registration
  forces a slot; and found probe defects (the refused picker write, a
  draft dropped by redescription, button-agnostic pan release, unreported
  defer results, ad-hoc inspection). The author reproduced the picker
  defect headlessly and found that the dialog's deferred save did run.

Implementation, 2026-10-19: slices 1, 2 and 3 had the Codex design
checkpoints recorded in their sections, and every slice had an independent
Codex implementation review whose findings were fixed with regression tests
(the example's in 0.3.23), as [verification](../../docs/verification.md#diagram-probe-plan)
records.

2026-10-20 audit ([plan-audit-repairs](../plan-audit-repairs.md)): several
contract tests of slices 2 and 3 are missing. The guide chapter describes
error handling without showing `App::on_error`. The custom-widget guide does not explain
export. That plan repairs them.
