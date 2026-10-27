# Part 6f1 design checkpoint: testing and window close

Part 6f1 of [the fix plan](fix-plan.md#6f1-testing-and-window-close) adds
public API, so its shape is settled here before any code: reviewed by Codex,
then approved by the maintainer. It closes round 1's F2, F4, F5 and F34
([the round 1 table](../../archive/developer-usability.md)); the evidence is triage
[group E](../runs/triage/group-E.md). Decision G5-32 allowed a testing module
once it owns F2's helpers; this part gives it those helpers.

Status: **revision 4, approved by the maintainer on 2026-10-23**, after
Codex's third recheck found it ready (review record at the end). The
maintainer also chose that a retired window's pending error is kept for
`run` to return at exit.

## What the testers built by hand

Group E's table lists the helpers that four runs and the three packaged
sessions each rebuilt: click at a component, replace a field's text, a key
chord, a component's semantics, settle after input, wait for background
work. Each is a few calls on `Runtime` today, but the right calls are not
discoverable: `inspection_id`'s example teaches a full snapshot search,
`text_input("")` silently does nothing, a click needs `reveal`, `bounds` and
two `PointerEvent`s, and two chord recipes disagree (U3).

## 1. The `testing` module

`pert_ui_runtime::testing` owns the driver and its error. It needs no
window, GPU or operating-system service, so it lives in the runtime, which
already builds without them. The facade's `pert_ui::testing` re-exports it
and adds the facade's own test entries (below).

### `Driver`

```rust
/// Drives a runtime as a user would, one settled step at a time.
#[must_use = "a driver does nothing until an action is called on it"]
pub struct Driver<'a> { runtime: &'a mut Runtime }
// Debug: hand-written, finish_non_exhaustive, shows no runtime state.

impl<'a> Driver<'a> {
    pub fn new(runtime: &'a mut Runtime) -> Self;
    pub fn runtime(&self) -> &Runtime;
    pub fn runtime_mut(&mut self) -> &mut Runtime;

    /// The clock step of one settle: 16 ms.
    pub const FRAME: Duration;

    pub fn settle(&mut self) -> Result<FlushReport, TestError>;
    pub fn advance(&mut self, by: Duration) -> Result<FlushReport, TestError>;
    pub fn wait_until(
        &mut self,
        timeout: Duration,
        condition: impl FnMut(&Runtime) -> Result<bool, RuntimeError>,
    ) -> Result<(), TestError>;

    pub fn find(&self, from: &WidgetId, path: &[&str]) -> Result<WidgetId, TestError>;
    pub fn semantics(&self, id: &WidgetId) -> Result<Semantics, TestError>;

    pub fn click(&mut self, id: &WidgetId) -> Result<(), TestError>;
    pub fn replace_text(&mut self, id: &WidgetId, text: &str) -> Result<(), TestError>;
    pub fn type_text(&mut self, text: &str) -> Result<(), TestError>;
    pub fn key(&mut self, code: KeyCode) -> Result<(), TestError>;
    pub fn chord(&mut self, chord: KeyChord, code: KeyCode) -> Result<(), TestError>;
}
```

**Borrowing.** The driver borrows `&mut Runtime` and keeps no state: its
clock is `Runtime::now`. A test builds one where it needs it,
`Driver::new(&mut ui)`, or keeps one for a block and reaches the runtime
through `runtime()`/`runtime_mut()`. This works the same with a plain
`Runtime` and with section 2's `HeadlessApp`, and application state the
test reads (an `Rc` model, signals) stays outside it, as in the packaged
sessions.

**Settling, and what it does not promise.**
- `settle` is one flush at `now + FRAME`. `advance(by)` is one flush at
  `now + by`. Both add with `checked_add` and report
  `TestError::ClockOverflow` rather than wrap or panic.
- One flush is what `Runtime::flush` already settles: updates, layout,
  focus and hover feedback, and the notifications the input requested,
  within `MAX_FLUSH_WAVES`. It is not quiescence. Work a callback defers
  to a later flush, a background result not yet delivered, and timers due
  after the new time stay pending; `wait_until` and `advance` reach them.
- `advance` is one deadline jump. Missed deadlines produce one update, as
  `flush` documents, so `advance(320 ms)` is not twenty 16 ms ticks; a test
  of repeated ticks calls `settle` in a loop.
- Lead U2 (section 4) showed that one flush per step is enough for the
  packaged sessions. That is evidence for those sessions, and the rustdoc
  promises only the bounded flush above.

**Input targeting: the selected window.**
- The driver delivers input where `Runtime`'s input methods do: to the
  selected window. It never selects another window itself, so a sequence
  such as a click followed by typing stays in one place. A test of a
  detached window selects it with `runtime_mut().select_window(..)` first.
- `click` and `replace_text` check the target's window with
  `Runtime::window_of`. A target in another window reports
  `TestError::OtherWindow { key }` before focus or any input changes;
  otherwise `replace_text`'s `Runtime::focus` would select the target's
  window and leave it selected.
- `click` reveals the target with `Runtime::reveal` and inspects the
  outcome. A `Refused` outcome reports `TestError::NotRevealed { key,
  refusal }`, and `Unmounted` reports the runtime's `Unmounted`. After a
  successful reveal, which settles presentation itself, it takes the middle
  of `Runtime::bounds`; no bounds reports `TestError::NotLaidOut { key }`.
  It then sends one primary press and one release there through
  `pointer_input`, and settles.
- Whether the press reaches the target is not checked: an overlapping
  surface is a legitimate result the test asserts on.
- **Click timing.** `click` does not move the clock beyond its settle, so
  two `click` calls at one point are 16 ms apart. That is within
  `ClickCounter::DEFAULT_INTERVAL`, so a built-in control counts them as a
  double click unless a press series was already running, in which case
  they continue it. A component with its own interval counts by that
  interval. A test that wants two separate single clicks puts `advance`
  beyond the component's interval between them; the rustdoc states these
  assumptions. There is no `double_click`.

**Text.**
- `replace_text` focuses the editor, sends `TextCommand::SelectAll` through
  `text_command`, then `text_input(text)`, or the Backspace key when `text`
  is empty, because empty committed text inserts nothing and deletes no
  selection (M4). It reports `TestError::NotAnEditor { key }` when the
  command answers `NotAnEditor`.
- `type_text` is `text_input` plus `settle`, at the caret. Both write what
  an editor's own rules write: a plain field writes its binding, a field
  with a normalizer or length limit holds a draft until Enter or focus
  loss, and a parameter field holds its draft under the edit protocol.

**Keys are synthetic, with stated limits.** `key` and `chord` go through
`command_key_input`, the native host's entry point, so application commands
see them. They build `CommandKeyInput::new` events: the base and logical
identities are the same key, the physical code is the one given, and no
event carries text or a composition. So they are for named keys and
command chords, not for typing, layouts, AltGr, dead keys or an IME:
- `key(code)` presses and releases `code` as `CommandKey::Named(code)`.
  Text is typed with `type_text`. A test of a printable key's native press,
  which carries text, of a layout or of a composition builds
  `CommandKeyInput` itself, and a test of one component's own key handling
  keeps `Runtime::key_input`.
- `chord(chord, code)` presses `chord`'s modifiers in a fixed synthetic
  order (Control, Alt, Shift, Super): each modifier's physical key, then
  the modifiers held so far through `native_modifiers`. It presses and
  releases `code` carrying `chord`'s `CommandKey`, then releases the
  modifiers in reverse order, each followed by the smaller snapshot, and
  settles. The order is deterministic, not a platform's: winit reports
  modifiers before the key on Windows and after it on macOS and Wayland,
  and macOS can also report a modifier change with no key event; a
  component that reads modifiers during a modifier's own press can
  observe the difference. `chord` takes the physical `code` because a
  character chord (`KeyChord::primary('s')`) names no physical key; the
  driver does not guess a layout.

**Neutral input first.** `key`, `chord` and `replace_text` require that
`runtime.keyboard().pressed()` is empty and
`runtime.keyboard().modifiers()` holds none, and report `TestError::InputHeld` otherwise. A helper therefore never
releases a press a test made through the low-level methods, and `key`
really sends its key without modifiers. `click` requires the same of the
keyboard. The runtime exposes no pointer-button state, so the rustdoc
states that precondition for buttons.

**Cleanup on failure.** A compound action releases everything it pressed,
even when a step fails:
- `click`'s release, also after the press returned an error, since
  `pointer_input` may report an earlier recipient's focus-loss error after
  delivering the press;
- the principal key of `key` and `chord`, and `replace_text`'s Backspace,
  also after a press that returned an error, since keyboard delivery can
  report an unrelated notification error after admitting the press;
- every modifier `chord` pressed, and the empty modifier snapshot.

Cleanup goes on after an individual release fails. The original error is
returned; when cleanup also failed, `TestError::Cleanup { error, cleanup }`
carries both, the first cleanup failure as `cleanup`; later cleanup
failures in the same action are not reported, a choice the rustdoc states.
When only cleanup failed, its first error is the result.

**Lookup.**
- `find` is `Runtime::find` with `TestError::NotFound { path }`, which names
  the path, instead of `Ok(None)`.
- `semantics` validates the handle through `Runtime::inspection_id`, then
  runs `Runtime::query(&InspectionQuery::components(Selector::id(..))
  .detail(Detail::Semantics))` (AN-1). A component that reports no
  semantics gives `TestError::NoSemantics { key }`.

**Waiting.** `wait_until` flushes, returns when `condition` holds, and
otherwise sleeps 1 ms of wall time and repeats, up to `timeout`; then it
reports `TestError::TimedOut { waited }`.
- It flushes at the same `now`, so the runtime clock does not move while it
  waits. Work already due and background results run; future deadlines do
  not.
- It polls rather than installing `set_wake_delivery`, which would replace
  a delivery the test or host set and could not be restored.
- Polling is an explicit, bounded test operation. The library's hosts and
  inspection still never poll.

### `TestError`

```rust
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TestError {
    Runtime(#[from] RuntimeError),
    NotFound { path: Vec<String> },
    OtherWindow { key: String },
    NotRevealed { key: String, refusal: RevealRefusal },
    NotLaidOut { key: String },
    NotAnEditor { key: String },
    NoSemantics { key: String },
    TimedOut { waited: Duration },
    ClockOverflow,
    InputHeld,
    Cleanup { error: Box<TestError>, cleanup: RuntimeError },
}
```

- It names what was rejected, as 6c1's rule asks.
- `Runtime` keeps a runtime failure in one form.
- It does **not** convert into `AppError`: that blanket `From` requires
  `DrawError: From<E>`, and a test failure is no drawing failure. Tests
  return `Result<(), Box<dyn std::error::Error>>`, as the guide's chapter
  shows.

### Re-exports (G5-32)

- **Runtime `testing`:** `Driver` and `TestError`, plus what a headless test
  writes besides the driver: `CaptureRequest`, `CaptureTarget`,
  `FrameImage`, `InspectionQuery`, `Selector` and `Detail`. These names
  stay at the root too.
- **Facade `testing`:** a module that re-exports the runtime's and adds
  `cpu::CpuRenderer` with `cpu`, and `HeadlessApp` with `native`, the
  feature that gates `App`.
- The semantics types stay in `component` and the prelude, as 6e placed
  them.
- G5-6's prelude-page list points to `testing` for the capture names
  instead of listing them.

## 2. The close path (F4) and the headless entry (F5)

### What changes in the native close order

Today the native close path runs `App::on_close` with the focused editor's
draft unsaved, so a policy that checks for unsaved changes sees none. An
accepted close then loses the typed text (F4).

The fix is one crate-private admission step, `prepare_close`, that both the
native runner and `HeadlessApp::admit_close` call before `on_close`. The
native runner keeps everything that is a state machine: coalescing repeated
requests while a decision is deferred, the pending decision, the receipts,
and applying the decision.

1. **Which roots are affected.** It depends on what an accepted close
   discards:
   - closing a detached window discards only that window's content, so
     that root is affected;
   - closing the main window ends the application, which discards every
     window, so every open root is affected.
2. **Releasing focus.** Each affected root is selected in turn and its
   focus released through `clear_focus`, as a window deactivation does; the
   previously selected root is selected again afterwards. The rustdoc
   states two consequences:
   - each recipient's `focus_lost` runs, and its error is queued in that
     root, as on deactivation;
   - `clear_focus` also clears the runtime-wide held keys and held-input
     registrations, as deactivation does. For a close the user started,
     the closing window has native focus, so nothing else is held. A
     programmatic `Runtime::request_close` of an inactive window also ends
     held input in the active one.
3. **Each editor applies its existing focus-loss policy**; F4 changes
   none. The rustdoc of `App::on_close` points to them rather than
   restating them, and gives the cases a close policy most often meets:
   - a plain `TextField` or `TextArea` already wrote its text, and a
     composition is dropped;
   - a field that drafts (a normalizer, a length limit, or an
     `EditorSession`) saves a changed draft its rules accept, and keeps it
     unsaved when they refuse it, when the field is read-only or when it
     is disabled;
   - a local number input saves a valid changed draft, keeps an invalid or
     read-only one unsaved, and cancels a pressed or held step gesture;
   - a parameter-bound number input cancels its draft, valid or not:
     parameter authority commits only on an explicit commit;
   - a `DraftEditor` follows its owner's save or cancel policy;
   - an `EditorSession` reports `FocusLeft` with the save's outcome unless
     the edit already ended, and reports nothing when the field is
     disabled.

   So a close policy sees a saved value only where that editor saves on
   focus loss.
4. **Settling, with backoff precedence.**
   - When no root at all is in component-error backoff, admission
     flushes the affected roots: the detached root alone, or every root
     through `flush_windows` for the main window. Each root's flush
     propagates that root's queued focus errors, so a failed `focus_lost`
     in any affected root aborts the close; `on_error` handles it as it
     handles an `on_close` error, and the request is not replayed. This is
     the flush a frame runs, so it may also settle notifications in other
     visible roots, as any frame's flush does. A hidden root is not
     flushed: its errors and notifications wait until it is shown, as they
     do today.
   - When any root is in backoff, affected or not, admission flushes
     nothing: settlement can visit every visible root and would retry a
     backing-off root's update before its deadline. A
     native `CloseRequested` already bypasses the preflush there, and
     `App::run` promises that closing during backoff returns the pending
     error. Bindings the drafts wrote are current; derived values and the
     queued focus errors wait for the next flush; the retained error keeps
     its precedence. Closing never requires a failing update to succeed
     first.
   - **A retired window's pending error.** Today an accepted detached
     close retires the window's native state, and with it a pending
     recovery error, so `run` cannot return it at exit (it searches the
     surviving windows only). That is a defect of the touched close path,
     so this part fixes it: retirement keeps the window's pending error,
     and `run` returns it at exit as it returns a surviving window's. This
     was the author's proposal, which the maintainer chose on 2026-10-23
     over reporting it through `on_error` at retirement, which would offer
     a retry with nothing left to retry.
   - `HeadlessApp` has no backoff, so it always flushes.
5. **`on_close` runs**, as now.
6. **Focus after a kept window.**
   - On a synchronous `KeepOpen`, in each affected root where admission
     released a component's focus, that component is focused again only
     when nothing in the root holds focus after the callback, so a field
     the policy focused keeps focus, and the same incarnation is still
     mounted, visible and focusable. A `Hidden`, `Unmounted` or
     `ModalBlocked` refusal leaves focus cleared; any other error is
     returned, and the native runner hands it to `on_error`.
   - On `Defer`, nothing is restored, natively or headlessly: when the
     decision later keeps the window open, the policy or its confirmation
     decides where focus goes. On `Close`, nothing is restored.
   - **Native reactivation records.** A window that was deactivated keeps
     a record of the component to focus on reactivation, and application
     code can focus another component in the meantime, so admission cannot
     infer from runtime focus that no record exists. Admission therefore
     drops the pending record of every affected root, whatever the
     decision: the close supersedes it. After a kept window, focus is what
     step 6 leaves, and a later reactivation restores only what a later
     deactivation records.

`App::on_close` and contracts.md state this order and the editor cases.
It is the plan's behavior break: valid drafts are saved before the policy
runs.

### `App::headless` and `HeadlessApp`

```rust
impl App {
    pub fn headless(self) -> Result<HeadlessApp, AppError>;
}

#[must_use = "dropping it shuts the runtime down without reporting cleanup errors"]
pub struct HeadlessApp { app: App }
// Debug: hand-written, as App's.
impl HeadlessApp {
    pub fn runtime(&self) -> &Runtime;
    pub fn runtime_mut(&mut self) -> &mut Runtime;
    pub fn main_window(&self) -> NativeWindowId;
    pub fn admit_close(&mut self, window: &NativeWindowId) -> Result<NativeCloseAction, AppError>;
    pub fn shutdown(self) -> Result<(), AppError>;
}
```

- **Gating.** Both are defined with `App`, under the `native` feature
  (`desktop`, `desktop-cpu` and their combinations), and `HeadlessApp` is
  reachable as `pert_ui::testing::HeadlessApp`. `App`'s `#[must_use]`
  message becomes "call run to open the application, or headless in a
  test".
- **`headless`, in this order:**
  1. validate the title as `run` does (`AppError::InvalidTitle`);
  2. set the viewport to the main window's configured size at scale 1;
  3. run the initialization closure on the runtime at clock zero;
  4. select the primary window again, since the closure may have selected
     a detached one, and flush the visible roots through `flush_windows`
     at `Duration::ZERO`, so their content is laid out; the primary window
     stays selected. A hidden root lays out when it is shown, and the
     first root error ends the flush and is returned as below.

  The size is simulated geometry: min and max sizes are not applied,
  because no window system clamps. No event loop, window, renderer or GPU
  is created, so `software_rendering` has no effect. Text layout needs
  none: fonts and text are the runtime's.
- **Failure.** If the closure or the first flush fails, `headless` shuts
  the runtime down, as the native runner does for an initialization
  failure. It returns the same aggregated form, `AppError::Shutdown`, when
  cleanup also failed, and the plain error otherwise.
- **`main_window`** is `Runtime::primary_window`.
- **`admit_close`** runs `prepare_close` and `on_close` for `window` and
  returns the decision. It does not apply it: a test asserts `Close` or
  `KeepOpen`, or for `Defer` drives the confirmation and reads the
  decision with `Runtime::read_signal`.
  - The `Defer` signal is validated at once, as the native runner does, so
    a signal of another runtime reports `ForeignRuntime` there.
  - Errors come back directly; `on_error` belongs to the native runner's
    recovery.
  - On `Defer`, no focus is restored when the decision later keeps the
    window, as in the native runner.
  - Each call runs the policy again. Coalescing a repeated request while a
    decision is pending is the native runner's state, and the rustdoc says
    so.
- **`shutdown`** shuts the runtime down and returns its cleanup failures
  as `AppError::Shutdown`. Dropping a `HeadlessApp` shuts down through
  `Runtime`'s drop, which reports nothing; that is why the type is
  `#[must_use]`.

### A host other than `App`

`Runtime::request_close` and `take_presentation_requests` say how such a
host completes a queued close request. The host owns the request until it
completes it with a `PresentationStatus`; dropping an unfinished request
completes it as `Failed(WindowError::Cancelled)`.
1. take the `Close` request;
2. run its own admission, the steps above;
3. on rejection, complete the request with `CloseRejected`; on an error,
   with `Failed` and the error as a `WindowError`;
4. on a deferred decision, keep the request and read the decision signal
   after each flush; when its window is gone before a decision arrives,
   drop the request, so it reports `Cancelled`, as `App` does;
5. on acceptance of the main window, end the application;
6. on acceptance of a detached window, select it and call
   `close_detached_window`, which may still answer `CloseRejected` under
   `DetachedClosePolicy::KeepOpen`;
7. complete the request with the resulting status.

## 3. Documentation

### The guide chapter

"Testing an application" (`guide/testing.md`, in the guide index) opens
with the `use` block G5-32 asks for. It starts from `Runtime::query` with
`Selector::id`, then covers:
- a click, and click timing;
- typing and replacing text;
- keys and chords, with the synthetic events' limits;
- settling, and when input is visible;
- list rows: `Each`'s `items` segment, `Runtime::children` or
  `Selector::children`;
- background work with `wait_until`;
- `HeadlessApp` and the close policy;
- a CPU capture.

Its examples are doctests, the capture one under `cpu` and the
`HeadlessApp` one under `native`.

### F34's one recipe

The diagnostics guide's "Keyboard input" example becomes `Driver::chord`.
Its text says what the driver sends, and that `command_key_input` and
`native_modifiers` remain the low-level entry points for layouts, text
and composition. The example no longer shows modifier state without the
physical keys.

### F2's doc sub-items

- `Runtime::inspection_id`'s example switches to `query`.
- `Runtime::text_input` says empty text inserts nothing and leaves a
  selection.
- `Runtime::focus` (and `TextField`) say where the caret lands.
- When input is visible: `Runtime`'s type documentation, `text_input`,
  `key_input`, `command_key_input` and `pointer_input` say that input
  delivers to its component at once and writes what that component's
  rules write. A plain editor's binding is written immediately; a draft,
  a parameter proposal and a notification for a hidden window are not.
  The tree, derived bindings and layout show the result after the next
  flush.
- `pointer_input` gets a click example.

## 4. F34 and lead U2

The packaged `inventory` and `log_explorer` sessions settle with three
flushes 16 ms apart, and press modifiers as physical keys only.

**U2 checked (2026-10-23):** in a throwaway worktree of 2032b93, each
session's settle was cut to one flush and both sessions were run with the
gate's arguments (`inventory -- --items 2000`, `log_explorer -- --lines
10000`). Every check of both passed, so for these sessions the extra two
flushes did nothing. The log explorer's other loops are not settles:
- its 20-flush loops let 320 ms pass in 16 ms ticks for the filter's
  debounce, and stay as `settle` loops, because one `advance` would be one
  deadline jump;
- its wheel and tail loops measure per-step cost.

The sessions then use `Driver` for widget clicks, keys, chords, text and
settling. Domain helpers stay where the driver's target is a widget and
theirs is a point:
- the inventory's table cells keep their coordinates;
- the diagram's canvas clicks and drags keep theirs, with their `pause`
  written as `Driver::advance` beyond the click interval.

## Review record

**First review (Codex, 2026-10-23):** not ready. Fourteen points, each
checked against the code and taken into this revision:
1. Closing the main window ends the application, but admission released
   only the selected window's focus. It now releases every window's focus.
2. An unconditional flush made closing during backoff depend on a failing
   update. There is no flush during backoff.
3. `TestError` cannot convert into `AppError`, whose blanket `From` needs
   `DrawError: From<E>`. The promise is removed.
4. Unconditional focus restoration could override the policy's own focus
   or restore focus in an unfocused window. Restoration is now
   conditional.
5. The editor families differ on focus loss, and a parameter draft
   cancels. The cases are listed, and F4 changes none.
6. `click` ignored reveal refusals and the target's window. Both are now
   handled.
7. Compound helpers had no failure cleanup. Cleanup is defined, and
   `TestError::Cleanup` keeps both errors.
8. The keys were claimed to be the native events, which they are not.
   They are synthetic, with their limits stated and the native modifier
   order.
9. `headless` had no initialization order, first flush or failure
   cleanup. All three are defined.
10. Admission and the native state machine were mixed. The method is now
    `admit_close`; the signal is validated at once; coalescing and
    receipts stay native; the non-`App` host steps are listed.
11. Settling promised too much. It now promises one bounded flush, and the
    "at once" sentence is qualified.
12. An implicit clock jump in `click` had side effects. It is gone, along
    with `double_click`; the clock add is checked.
13. The guide's chord recipe would have stayed a second recipe. It is
    migrated, and the point-based diagram helpers stay.
14. `Debug`, `#[must_use]`, the `native` gate and the facade module were
    unsettled. All four are now specified.

The first review's answers to the open questions are adopted: a borrowing
driver; no implicit click-interval advance; conditional focus restoration;
admission only, for both window kinds.

**Recheck (Codex, 2026-10-23):** points 3, 11, 13 and 14 resolved; nine
blockers, taken into revision 3:
1. Clearing every root left other roots' focus errors unflushed. Affected
   roots are flushed with `flush_windows`, and the backoff rule covers any
   affected root. The runtime-wide effect of `clear_focus` is stated.
2. Restoring the selected window broke a click followed by typing across
   windows. The driver now works in the selected window only, and
   `click` refuses a target in another.
3. A pending native reactivation record could override the policy's
   focus. The runner drops it when the policy leaves a component focused.
4. A principal key could stay pressed after a failure. Every press is
   released, cleanup continues past failures, and the helpers require
   neutral keyboard input (`TestError::InputHeld`).
5. Deferred restoration and the custom host's deferred receipt were
   unspecified. `Defer` restores nothing, and the host steps include the
   deferred branch and cancellation on drop.
6. The modifier order is not native on Windows. It is a stated synthetic
   order, and the identity sentence is corrected.
7. The first headless flush could miss the main root. The primary window
   is selected again and every root flushed.
8. The editor cases overstated saving. They are qualified for sessions,
   read-only, held gestures and `FocusLeft`.
9. "Two calls are a double click" needed its assumptions, which it now
   states.

**Second recheck (Codex, 2026-10-23):** blockers 4, 6, 7, 8 and 9 of the
recheck resolved; five blockers and four wording nits, taken into revision
4:
1. The backoff check covered only affected roots, but settlement reaches
   every visible root. Any root in backoff now stops admission's flush.
2. A deactivated window's reactivation record can coexist with runtime
   focus set later. Admission drops affected roots' records whatever the
   decision.
3. `replace_text`'s focus selected another window. It checks the window
   first, as `click` does.
4. A detached close discarded the window's pending recovery error.
   Retirement keeps it for `run` to return at exit, an author decision for
   the maintainer.
5. The custom-host deferred branch contradicted `App`, which drops the
   request (`Cancelled`). The steps now match, in `PresentationStatus`
   terms.

Nits: `keyboard().modifiers()` replaces a nonexistent
`Runtime::modifiers`; the headless flush covers visible roots; only the
first cleanup error is reported; macOS can report a modifier change
without a key.

**Third recheck (Codex, 2026-10-23):** all five blockers and four nits
resolved, no new finding: ready for approval.
