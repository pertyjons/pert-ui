# Round 2 fix plan, part P4 design checkpoint: focus and idle in the driver

Part P4 of [the round 2 fix plan](round-2-fix-plan.md#p4-driver-focus-and-idle-0379)
closes R2-11 and R2-24 by decision D5 (approved with the recommended option
on 2026-10-28). The plan gives P4 a short checkpoint for the quiet-interval
check's semantics, its names and its report type; this document settles
them, and the smaller rules `Driver::clear_focus` and `TestError::Draw` need,
before any code. Codex reviews it; the author approves it together with
Codex's review under the maintainer's delegation of 2026-10-28.

Every reference to library code is to `ee55226`, where the library code is
unchanged since 26e6b5a (0.3.75), the commit the plan cites; P1's uncommitted
working-tree edits are not cited. Path prefixes: **T**
`crates/pert-ui-runtime/src/testing.rs`, **H**
`crates/pert-ui-runtime/src/host.rs`, **RT**
`crates/pert-ui-runtime/src/runtime.rs`, **CO**
`crates/pert-ui-runtime/src/runtime/compose.rs`, **F**
`crates/pert-ui/src/lib.rs`, **G** `crates/pert-ui/src/guide/`, **U**
`~/pert-ui-usability/` (frozen runs).

P4 lands after P2 (glob membership and the listing-driven completeness test
for `testing`) and P3 (small ergonomics), whose designs are written in
parallel. Where P4 depends on them, section 4 states the dependency instead
of assuming their details.

Status: **revision 3, approved 2026-10-28** by the author together with
Codex's review, under the maintainer's delegation of 2026-10-28, with P4-D1
and P4-D2 as recommended. Revision 1 was reviewed by Codex (needs revision,
six should-fix points and two nits); revision 2 answered each point, and
Codex's recheck left one, which revision 3 answers; Codex's second recheck
found no new point: ready for approval ([review record](#review-record)).

## What testers built by hand, and what the library says

- **Clearing focus (R2-11, results row at
  `plans/archive/developer-usability.md:929`).** 4-claude did not test the
  "nothing focused" case; 4-codex found `HostIntegration::clear_focus`
  through the `testing` glob, called `driver.runtime_mut().clear_focus()`
  (U `r2-4-codex/work/bookmarks/tests/ui.rs:226`) and first wrote
  `clear_focus()?`, which fails because the method returns `()`
  (U `r2-4-codex/work/FRICTION.md:40-44`). The inherent method became
  crate-private in L3's N3 (RT:5315-5316); the testing chapter does not name
  it, and of the testing pages only the facade's `testing` page shows it
  (F:243-260); `DecimalInput`'s page has the one other rustdoc example
  (section 5).
- **Idle checks (R2-24, results row at
  `plans/archive/developer-usability.md:942`).** Each run that checked
  idleness built a different check, and none measured what it meant to:
  - 5-head-claude-2 paints, settles, asserts no updates and no deadline,
    **paints again**, then asserts `!needs_paint()`
    (U `r2-5-head-claude-2/work/tests/headless.rs:107-115`). The second paint
    clears the damage the assertion reads (CO:336), so the paint check cannot
    fail.
  - 5-head-codex-2 paints, reads `next_deadline`, then `advance(10 s)` and
    reads `needs_paint` (U `r2-5-head-codex-2/work/src/main.rs:491-498`):
    one flush for the whole stretch, so periodic work shows as at most one
    update (T:270-276).
  - 5-head-codex-1 reads deadlines and update counts over three one-second
    jumps and never checks paint (U `r2-5-head-codex-1/work/tests/headless.rs:195-201`).
  - The check's parts are reachable through `testing::*`, which re-exports
    `HostIntegration` (T:79-80, F:267-268), but no testing page names them
    (triage `runs/triage-r2/group-3.md:63-74`, NF-5 at `:281-294`).

## 1. Today's behavior, from the code

- **`HostIntegration::clear_focus`** (H:419-426, forwarding at H:649-651 to
  RT:5315-5379) is a window's deactivation, with effects of two reaches:
  - **in the selected window:** it closes nonmodal surfaces and keeps modal
    ones (RT:5322-5325), clears the window's pointer buttons, capture,
    hover and pointer position (RT:5327, :5342-5345, :5355-5356), calls the focused component's `focus_lost`, which
    lets a local editor save a valid draft, and queues its error
    (RT:5364-5369), and queues the errors of the focus-within notifications
    (RT:5376-5377);
  - **runtime-wide:** it cancels a drag when the drag is in this window or
    there is none (RT:5316-5319), cancels **every** admitted held-input
    registration with `HeldCancellation::FocusLost`, whichever window owns it
    (RT:5320, `runtime/held.rs:166-171`, documented at `held.rs:15-20`), and
    clears the runtime's pressed keys, command presses and reported
    modifiers (RT:5337-5341).

  It returns `()`. The next flush first delivers queued parameter terminal
  notifications (RT:5767, `dispatch_deferred` at RT:759-768), then returns
  the oldest queued focus error (RT:5768-5770), before it dispatches
  commands, tasks, timers or settles presentation; an outermost flush also
  answers queued inspection requests on that error (RT:5736, :5744-5755).
- **`Runtime::paint`** (CO:32-34) refuses with `DrawError::PendingUpdates`
  while queued commands, focus errors, focus restoration, stack hover,
  requested removals, moves, children, text commands, pending updates or
  layout wait (CO:49-61), and clears the window's scene damage on success
  (CO:336). Component paint callbacks run here, lazily (CO:99-179): a test
  that never paints never runs them.
- **`HostIntegration::needs_paint`** (H:272-278, CO:340-364) is true for scene
  damage, a paint-dirty presented component, most of the pending work
  `paint` refuses and **a capture waiting for this window** (CO:345-350),
  which its rustdoc does not mention. It does **not** read the command queue
  (CO:342-364 against CO:49), so a window can need no paint while `paint`
  refuses with `PendingUpdates`.
- **Paint failure sites:** an admitted `paint` starts without a failure site
  (CO:48; `Runtime::last_failure`, `runtime/application.rs:425-475`) and
  records one with `FailurePhase::Paint` only when a component's `paint` or
  `paint_foreground` fails (CO:146-157). A component may itself return
  `DrawError::PendingUpdates` (its error type is `DrawError`, RT:2194); the
  admission refusal (CO:49-61) and the unsized-entry guard (CO:99-107)
  record no site.
- **`Runtime::next_deadline`** (RT:5572-5620) returns the current time while
  immediate work waits (RT:5575-5595: background and deferred tasks, drag
  service, computations, queued commands and reveals, parameter
  notifications, focus errors, requested structure changes, pending updates
  and layout, inspection requests, notifications), otherwise the earliest of
  the **selected window's** update deadlines and timers (RT:5596-5609) and
  the work all windows share: interval timers, view-scale settling and polled
  live sources (RT:5613-5615). Its rustdoc says only "the earliest elapsed
  deadline" (RT:5572). The cross-window aggregate is
  `HostIntegration::application_deadline` (`runtime/native_windows.rs:1385`).
- **`Runtime::flush`** (RT:5677-5702) runs the selected window, plus the
  notification followups it delivers in other visible windows, which
  `FlushReport::components_updated` counts (RT:3667-3676). A flush dispatches
  queued commands once, at its start (RT:5771), before it runs component
  timers and notifications (RT:5777-5790); a command one of those callbacks
  queues with `EventContext::invoke_command` (`runtime/commands.rs:1756-1787`,
  writable outside update, paint, layout, inspection and delivery,
  RT:709-728) waits for the next flush, so a successful flush can leave work
  that `paint` refuses.
- **Nested runtime operations flush other windows.** Within a flush, some
  operations run flushes of their own, whose reports are not part of the
  returned `FlushReport`:
  - a dispatched queued command calls `Runtime::invoke_command`
    (`runtime/commands.rs:1698`), which first calls
    `HostIntegration::flush_windows(now)` (`runtime/commands.rs:1045`), so
    every eligible window runs its due updates and timers;
  - queued reveals (`EventContext::request_reveal`, `runtime/reveal.rs:266`)
    are dispatched after the update waves (RT:5813); each selects its
    target's window and flushes it (`runtime/reveal.rs:546-549`, `:559`), so
    a reveal into another window runs that window's due timers and updates;
  - deferred tasks and interval ticks receive `&mut Runtime` and may call any
    operation, including those that flush (`docs/contracts.md:1116-1121`).
- **Periodic built-in work:** `UpdateContext::animate` wakes at the window's
  presentation interval (RT:2996-3010); `Spinner` steps every 100 ms through
  `wake_after` (`progress.rs:302`, `:382-392`); a focused, editable text
  field blinks its caret on a 500 ms deadline (`text_fields.rs:110-111`); a
  toast schedules one wake at its expiry (`notifications.rs:879`);
  `Runtime::start_interval` keeps the application's deadline set while it
  runs (`runtime/application.rs:289-291`).
- **`Driver`** keeps no state (T:88-95); each action ends with `settle`, one
  flush at `FRAME` (16 ms) past the clock (T:96-104, :236-268). `TestError`
  is `#[non_exhaustive]`, derives `Clone, Debug, Eq, PartialEq` and keeps a
  runtime failure in one form, `Runtime` (T:702-713). `DrawError` derives
  the same traits and has a `Runtime(RuntimeError)` variant
  (`drawing.rs:23-78`).

## 2. The rules

### 2.1 `Driver::clear_focus`

1. It calls `HostIntegration::clear_focus` on the driven runtime, then
   `Self::settle`. The behavior is the deactivation's, unchanged (section
   1); the driver adds only the settle, as every action does (T:98).
2. **Scope, as the deactivation's (section 1):** focus, surfaces, pointer
   state and the focus callbacks are the selected window's; another
   window's focus is untouched, and a test of a detached window selects it
   first, as for input (T:106-113). Physical-input cleanup is runtime-wide:
   pressed keys, command presses and reported modifiers are cleared, and
   every admitted held-input registration, in any window, is cancelled with
   `HeldCancellation::FocusLost`, because releases may no longer arrive
   (`held.rs:15-20`). The rustdoc states both reaches; P4 changes neither.
3. **No neutral-input requirement.** Unlike `key`, `chord`, `replace_text`
   and `click` (T:128-136, :691-699), it does not refuse held keys or
   reported modifiers: releasing them is what a deactivation does
   (RT:5337-5341). Afterwards input is neutral, so the next driver action is
   admitted.
4. **Errors** are the settle's: `TestError::ClockOverflow`; a focus-loss or
   focus-within callback's error, which the deactivation queued and the
   settle's flush returns as `TestError::Runtime` (a later queued error comes
   from the next flush). Ordering, as the flush has it (section 1): queued
   parameter terminal notifications are delivered first, then the error is
   returned before commands, tasks, timers or presentation run, so the
   runtime has not settled and a further `settle` delivers the rest; the
   flush is not side-effect free, and the rustdoc says only what precedes
   the error. The other errors of `Runtime::flush`.

### 2.2 `Driver::quiet_for` and `QuietReport`

`quiet_for(interval)` measures what happens in the selected window over an
interval of runtime time that follows a painted baseline.

1. **Overflow first.** `end = now + interval` is computed before anything
   runs; overflow is `TestError::ClockOverflow` with nothing flushed or
   painted, as `advance` checks (T:277-280).
2. **Baseline.** One `Runtime::flush` at the **current** clock (as
   `wait_until` flushes, T:305-307), then one `Runtime::paint`, whose display
   list is dropped. Work already due when the call starts runs here and is
   not counted: the report describes what happens after the baseline. The
   baseline does not move the clock, so the measured interval is exactly
   `interval`.
3. **Refused baseline.** When the runtime refuses the paint with
   `DrawError::PendingUpdates` because the baseline flush left work queued
   (section 1, `Runtime::flush`), the call does not fail: that work is
   activity. The refusal is told apart from a component's own
   `PendingUpdates` by provenance (decision [P4-D2](#decisions)): after the
   failed paint, `Runtime::last_failure()` names no `FailurePhase::Paint`
   site. The interval then runs from the unpainted state, and the report
   records it in `baseline_painted: false`, which makes `is_idle()` false
   whatever the interval's end shows. A `PendingUpdates` whose paint
   recorded a `FailurePhase::Paint` site is the component's error and ends
   the call as `TestError::Draw`, as any other paint error does (rule 8).
4. **Steps.** The clock advances from the start to `end` in `Driver::FRAME`
   steps, one `Runtime::flush` each, and nothing is painted between them, so
   damage made anywhere in the interval is still pending at its end. **A
   final partial frame** is one shorter step that lands exactly on `end`:
   `quiet_for(40 ms)` flushes at 16, 32 and 40 ms past the start. **A zero
   interval** makes no step; the report then holds the state right after the
   baseline (no updates, the deadline and paint need left by the baseline),
   a check of "nothing scheduled now" without letting time pass.
5. **The report** (`QuietReport`):
   - `components_updated`: the sum (saturating) of the steps'
     `FlushReport::components_updated`, which includes the notification
     followups a step delivers in other visible windows (RT:3670-3673). A
     component of another window that a binding change only marks pending
     is updated when that window flushes, and is not counted. The count is
     exactly the sum of the reports the steps' `Runtime::flush` calls
     return. **Limit, inherited from nested runtime operations:** a flush
     nested inside a step (a queued command's admission, a queued reveal,
     or a deferred task or interval tick that calls a flushing operation;
     section 1) runs updates, in the selected window or any other, whose
     counts that flush's caller discards, so they are not counted. A test
     that needs them observes the components themselves.
   - `next_deadline`: `Runtime::next_deadline` after the last step (after the
     baseline for a zero interval): absolute, as `FlushReport::next_deadline`
     is, and the current time when immediate work waits.
   - `needs_paint`: `HostIntegration::needs_paint` after the last step
     (after the baseline for a zero interval), the host's own observation
     and nothing else, as D5 states it: whether the selected window needs
     painting at the interval's end. A capture waiting for its host keeps it
     true (CO:345-350).
   - `baseline_painted`: whether the baseline paint succeeded; false only
     under rule 3.
   - `is_idle()`: the baseline was painted, and no update, no deadline and
     no paint need followed.
6. **Scope, aligned with `Driver::settle`:** the selected window's own
   updates, deadlines, timers and presentation, plus the work all windows
   share that `Runtime::next_deadline` includes (section 1). Another window's
   own timer or animation does not make the report non-idle, because the
   driver's flushes do not run that window's own timers and deadlines; a
   test checks that window by selecting it with
   `HostIntegration::select_window` and calling `quiet_for` again. Shared
   work counts whichever window it serves: an interval timer, a background
   task or a polled live source presented only in another window keeps
   `next_deadline` set. **Qualification:** a nested runtime operation inside
   a step (a queued command's admission, a queued reveal into another
   window, or a deferred task or interval tick that flushes; section 1) can
   run other windows' due work, uncounted (rule 5). The driver selects
   nothing itself, and command admission and reveals restore the selected
   window (`runtime/commands.rs:1047-1049`, `runtime/reveal.rs:549`; a
   deferred task or tick that selects another window is the application's
   choice), so the final observations, `next_deadline` and `needs_paint`,
   stay the selected window's.
7. **Cost and choosing the interval.** One flush per 16 ms of runtime time,
   with no wall-clock sleep: a second is 63 flushes. The interval should be
   longer than the longest period the test means to rule out (a spinner
   steps every 100 ms, a caret blinks every 500 ms). A deadline beyond the
   interval, such as a toast's expiry, still shows in `next_deadline`, so it
   is never mistaken for idleness.
8. **Errors:** `TestError::ClockOverflow` (rule 1); a flush's error, of the
   baseline or of a step, as `TestError::Runtime`, ending the call with the
   clock where that flush left it; a paint error other than rule 3's
   refusal, a component's own `PendingUpdates` included, as
   `TestError::Draw`, or as `TestError::Runtime` for `DrawError::Runtime`
   (section 2.3).
9. **Neutral input is not required:** the call sends no input. A
   component-owned held action that schedules work, such as a number field's
   held step button, which repeats on its own event timer
   (`text_fields/number_inputs.rs:1122-1133`), is activity the report shows.
   A held command repeats only on incoming repeated-key events
   (`runtime/commands.rs:1435`), which the call does not send.

Evidence that rule 4's FRAME steps are needed: one jump of the whole
interval runs a periodic component once (T:272-276), as 5-head-codex-2's
check did; FRAME steps run it once per period. Evidence that rule 2's
baseline paint is needed: without it, damage from before the call stays
pending and an idle window reports `needs_paint`; painting at the end
instead removes the evidence, as 5-head-claude-2's check did.

### 2.3 `TestError::Draw`

1. `TestError` gains `Draw(DrawError)`, displayed transparently, as
   `Runtime` is.
2. **One form for runtime failures.** `TestError`'s rustdoc promises that a
   runtime failure keeps the one form `Runtime` (T:704-705). The conversion
   is therefore a hand-written `impl From<DrawError> for TestError`, not a
   derived `#[from]`: `DrawError::Runtime(error)` becomes
   `TestError::Runtime(error)`, every other variant `TestError::Draw`. The
   driver's methods and this conversion always normalize so; a consumer can
   still construct `TestError::Draw(DrawError::Runtime(..))` by hand, which
   the variant's rustdoc does not pretend to forbid. The conversion is
   public, so a test helper returning `TestError` can apply `?` to
   `Runtime::paint`.
3. Only `quiet_for` produces it among the driver's methods.

## 3. Public API

In `pert_ui_runtime::testing`, re-exported by `pert_ui::testing` (section
4):

```rust
impl<'a> Driver<'a> {
    /// Clears focus in the selected window as its deactivation does, then
    /// settles. [2.1: the deactivation's effects, the draft saved by the
    /// editor's focus-loss rules, no neutral-input requirement, neutral
    /// input afterwards, the selected-window and runtime-wide reaches;
    /// # Errors with the ordering of rule 4; an example in
    /// which a normalized `TextField`'s typed draft is saved and nothing is
    /// focused afterwards.]
    pub fn clear_focus(&mut self) -> Result<(), TestError>;

    /// Paints the selected window, lets `interval` of runtime time pass in
    /// `Self::FRAME` steps without painting, and reports what happened.
    /// [2.2: sections "The interval" (baseline, steps, final partial frame,
    /// zero interval, refused baseline), "Scope" (with command admission's
    /// qualification), "Cost"; # Errors; an
    /// example in which a busy `Spinner` is not idle over 250 ms and is idle
    /// once its binding is false.]
    pub fn quiet_for(&mut self, interval: Duration) -> Result<QuietReport, TestError>;
}

/// What happened in the selected window over a `Driver::quiet_for`
/// interval. [The rule-5 fields; that `is_idle` is the usual assertion and
/// the fields say which kind of work remains; that the report is not a
/// `FlushReport` because it covers many flushes and paint need.]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[must_use = "a quiet interval checks nothing unless its report is read"]
pub struct QuietReport {
    /// [2.2 rule 5.]
    pub components_updated: usize,
    /// [2.2 rule 5; the shared work it includes, by kind.]
    pub next_deadline: Option<Duration>,
    /// [2.2 rule 5; the waiting-capture case.]
    pub needs_paint: bool,
    /// [2.2 rules 3 and 5: false when the runtime refused the baseline
    /// paint for queued work, which keeps the report from being idle.]
    pub baseline_painted: bool,
}
impl QuietReport {
    /// The baseline was painted, and then nothing updated, nothing is
    /// scheduled and nothing needs painting.
    #[must_use]
    pub fn is_idle(&self) -> bool;
}

pub enum TestError {
    // ... existing variants unchanged ...
    /// Painting the selected window failed (`Driver::quiet_for`'s baseline
    /// paint). The driver and `From<DrawError>` report a runtime failure as
    /// `Self::Runtime` instead, so neither produces `DrawError::Runtime`
    /// here.
    #[error(transparent)]
    Draw(DrawError),
}
/// Keeps a runtime failure in its one form, `TestError::Runtime`. [2.3.]
impl From<DrawError> for TestError;
```

- **Names.** `quiet_for` keeps the plan's wording and reads as the check it
  is (`driver.quiet_for(d)?.is_idle()`). The type is `QuietReport`, not the
  plan's provisional `Quiet`: it parallels `FlushReport`, and a one-word
  `Quiet` is likelier to collide with an application's own name under the
  `testing` glob (L2's collision concern, `layering-design-L2.md`
  section 4).
- **Public fields with `#[non_exhaustive]`**, as `FlushReport` has public
  fields (RT:3669-3676): a test reads them; only the runtime constructs the
  report, and a later field is not a breaking change.
- **`#[must_use]` on the type,** not only through `Result`: with `?` the
  `Result` is consumed and the report dropped silently, the case P3 handles
  for `defer_while_shown`'s receipt (plan D11). `quiet_for` exists only for
  its report, unlike `settle`, whose `FlushReport` is rightly discardable.
  This is AGENTS.md's rule for results whose discard is a mistake.
- **The implementation** (T): overflow check; `flush(now)`; a `match` on
  `paint()`: `Ok` sets `baseline_painted`; `Err(DrawError::PendingUpdates)`
  with no `FailurePhase::Paint` site in `last_failure()` is rule 3's
  refusal; any other error, including a component's `PendingUpdates`, is
  returned through the `From` of section 2.3; a loop
  `at = at.checked_add(FRAME).map_or(end, |next| next.min(end))` while
  `at < end`, flushing at `at`; then the report. No new runtime internals:
  every call is public or a `HostIntegration` method, so a custom test
  harness can do the same (AGENTS.md: no privileged path).

## 4. Module membership and dependencies

- **`QuietReport`** is defined in `pert_ui_runtime::testing` beside `Driver`
  and `TestError` and joins the facade's `#[doc(inline)]` re-export of them
  (F:275-276). It is not at either crate root, as `Driver` is not.
- **`DrawError`** becomes reachable from `TestError`'s variants. By
  [decision P4-D1](#decisions) it joins both `testing` modules
  (`#[doc(no_inline)]`, T:81-86 and F:269-274); it stays in `component`
  (`lib.rs:651` of the runtime) and at the roots. It is the same item
  everywhere, so the globs still combine.
- **Dependency on P2.** P2 defines the `testing` rule, its seeds, the
  payload traversal, the exclusion list and the listing-driven test over the
  committed listings. P4 relies on it as follows, without assuming its
  details:
  - P2's test must see P4's new signatures: `quiet_for`'s `QuietReport`
    (fields of primitive types only) and `TestError::Draw`'s `DrawError`.
    With `DrawError` exported (P4-D1), the test passes for it; removing that
    export must make the test fail once the listing is regenerated (a
    contract test, section 7).
  - `DrawError`'s own payloads (`VectorId`, `MaskId`, `ImageId`, `PathError`,
    `drawing.rs:50-64`) raise the question `RuntimeError`'s payloads already
    raise for `TestError::Runtime`. P4 applies whatever P2 settles for the
    payloads of an error a seed returns (export or a reviewed exclusion), and
    the P4 Progress entry records the outcome per name.
  - P2's `every_name` and `every_glob_combines` lists
    (`crates/pert-ui/tests/foundation/modules.rs:78-87`, `:151-` at HEAD)
    gain `QuietReport` and `DrawError` in the shape P2 leaves them.
  - P2 rewrites the testing chapter's introduction (G `testing.md:11-17`)
    and the two `testing` module pages to state its rule; P4's sentence
    edits (section 5) apply to that text.
- **Dependency on P3:** none in substance. P4 reuses P3's form of a
  `compile_fail` test under `#![deny(unused_must_use)]` for a value
  discarded through `?` (section 7).
- **Later parts.** P4's examples and tests write `Spinner::new(binding)` as
  of their commit; P6 (`Spinner::bound`) migrates them, so P6's manifest
  gains P4's `quiet_for` rustdoc example and its tests. P5 and P8 add
  further `Driver` methods and `TestError` fields after P4.
- **Prelude closure:** P4 adds no API to a prelude member (`Driver`,
  `TestError` and `QuietReport` are in `testing` only), so L2's closure rule
  is not engaged; P2's `testing` rule is, through its test.

## 5. Documentation

Text describes P4's own commit.

- **The testing chapter** (G `testing.md`):
  - **"Nothing focused"**, after "Keys and chords" (`:131`): a window's
    deactivation through `Driver::clear_focus`, what it ends (held keys,
    menus and popovers; modal dialogs stay), that a focused editor applies
    its focus-loss rules (a valid draft is saved, link to contracts.md's
    draft policy), and that the next action needs no release. Example: a
    normalized `TextField`'s draft saved by `clear_focus`, then a chord that
    reaches an `Application` command with nothing focused.
  - **"Nothing scheduled while idle"**, after "Background work" (`:263`):
    the check, why a test paints first and lets time pass in frames (in one
    sentence each, 2.2's evidence paragraph), the report's fields, the
    selected-window scope with shared work, choosing the interval and a zero
    interval. Example: a window with a focused `TextField` is not idle, its
    caret blinking; after `clear_focus` it is. This uses only `TextField`,
    which no later part changes.
  - The introduction's description of the driver (as P2 leaves it) adds
    that it clears focus and checks a quiet interval.
- **`Driver`'s page** (T:88-162): "Settling" ends with a pointer to
  `quiet_for` for "is anything left"; the methods' rustdoc per section 3.
- **`testing` module pages:** the runtime's `HostIntegration` bullet
  (T:28-30) and the facade's page (F:211-214) drop "clearing focus" from
  what a test uses the trait for and name `Driver::clear_focus` and
  `Driver::quiet_for` beside the driver's other actions; the facade's second
  example (F:243-260) keeps `select_window` and
  `take_presentation_requests` and drops its `focus`/`clear_focus` lines.
  The `host` module page (F:305-307) and the trait page (H:63-67) drop "or
  clears focus as a window's deactivation would" from what a test stands in
  for.
- **`HostIntegration::clear_focus`** (H:419-426) gains "A headless test calls
  `Driver::clear_focus`, which also settles." **`needs_paint`** (H:272-278)
  gains that a capture waiting for its host keeps it true and that a queued
  command does not (CO:342-364 against CO:49; a same-kind fix in touched
  text), and "A test that checks idleness calls `Driver::quiet_for`, which
  paints a baseline first."
- **`Runtime::next_deadline`** (RT:5572-5573) states its scope: the selected
  window's update deadlines and timers, the work windows share, and the
  current time while work waits; `HostIntegration::application_deadline`
  for every window. `QuietReport::next_deadline` is defined by it, so its
  one-line rustdoc is completed here.
- **`DecimalInput`'s example** (`text_fields/number_inputs.rs:212-226`)
  imports `testing::HostIntegration` only to clear focus, the taught route D5
  retires; it becomes `Driver::new(&mut ui).clear_focus()?;` with a
  `Box<dyn Error>` result. It is the only rustdoc example that does so; the
  others that import `testing::HostIntegration` call `prepare_detach`,
  `service_intervals`, `set_clipboard` or `take_presentation_requests`
  (`layout.rs:537`, `runtime.rs:3108`, `:5137`, `selectable_text.rs:86`,
  `tool_windows/mod.rs:129`) and stay.
- No change to `docs/contracts.md`, `docs/manual.md` or the READMEs: none
  describes the driver (`rg Driver` over them finds only a GPU driver
  column), and the runtime README's `needs_paint` paragraph (`:807-809`) is
  for hosts.

## 6. Migration manifest

P4 breaks nothing (section 8), so no consumer has to change; the taught
texts move to the new methods.

| Consumer | P4 |
|---|---|
| Testing chapter, `testing` and `host` module pages, `HostIntegration` and `Runtime::next_deadline` rustdoc, `DecimalInput`'s example | changed (section 5) |
| `crates/pert-ui-runtime/tests/testing.rs` | gains section 7's tests |
| `crates/pert-ui/tests/foundation/modules.rs` | gains the names (section 4) |
| Packaged examples and their headless sessions | unchanged; `diagram_editor/session.rs:525`, `:537` release focus through `HostIntegration::clear_focus` before calling the close policy themselves, standing in for `App`'s close admission (`crates/pert-ui/src/app/close.rs:33-70`), a passage P12 rewrites for its close-policy TODO |
| Gallery (`apps/gallery/src/tests/*.rs`, `interaction_check.rs`, `live.rs:829-835`, `main.rs:280`) | unchanged: host-level checks where the gallery stands in for a host |
| Native checks, platform code (`crates/pert-ui/src/platform/`) | unchanged: host code |
| Facade's other tests and doctests | unchanged |
| Diagnostics fixtures (`tests/diagnostics/ui/`) | none expected; the snapshots run in the gate, since a re-export can change a suggested path |
| `plans/usability/task4-app` | unchanged: it uses none of the changed items (`git grep` finds no `TestError`, `clear_focus` or `needs_paint`); no fixture check |
| `docs/manual.md`, READMEs | unchanged (section 5) |
| Frozen: submissions and probes under `plans/usability/runs/` and U, such as U `r2-5-head-claude-2/work/tests/headless.rs:107-115` and U `r2-4-codex/work/bookmarks/tests/ui.rs:226` | not migrated |

## 7. Tests

All in `crates/pert-ui-runtime/tests/testing.rs`, the existing test binary of
the driver (it already has `FailingFocusLoss`, `is_failure` and a detached
window setup, `:137-149`, `:20-22`, `:334-384`); the plan's "facade area
binary" rule does not apply to a runtime item. Each window has a viewport,
so trees lay out.

- **New-behavior tests** each name the assertion that detects a wrong
  implementation and the compiling mutation that must make it fail. A test
  that fails only because a new item does not compile is not evidence.
- **Contract and compatibility tests** check rules P4 keeps or states.

### New-behavior tests

1. **A focused draft is saved by `clear_focus`:** a `TextField` with a
   trimming normalizer, focused, `type_text(" draft ")`, then
   `driver.clear_focus()?`. Named assertion: the binding is `"draft"` and
   `focused()` is `None`. Mutation: call `HostIntegration::cancel_input`
   instead of `clear_focus` (no save, focus kept).
2. **A focus-loss error is returned by `clear_focus`:** `FailingFocusLoss`
   focused. Named assertion: `is_failure(&error, "focus loss failed")` on
   the returned error. Mutation: drop the settle (the queued error is not
   returned).
3. **`clear_focus` needs no neutral input:** press a key through
   `command_key_input` (so `driver.key` returns `InputHeld`), then
   `clear_focus()?`. Named assertion: `driver.key(KeyCode::Tab)` succeeds
   afterwards and `keyboard().pressed()` is empty. Mutation: call
   `require_neutral` first.
4. **A spinner is busy over the interval:** `Spinner` bound to `true`,
   `quiet_for(250 ms)`. Named assertion: `components_updated >= 2`,
   `next_deadline.is_some()`, `needs_paint`. Then the binding is set false
   and, named, `quiet_for(250 ms)?.is_idle()`. Mutations: one flush at `end`
   instead of FRAME steps (one update: the first assertion fails); no
   baseline paint (the second report keeps the damage of the change: the
   idle assertion fails).
5. **A timer the test installed:** `start_interval(100 ms)` incrementing a
   counter a `Label` shows, `quiet_for(250 ms)`. Named assertion:
   `components_updated >= 2`, `needs_paint`, `next_deadline.is_some()`;
   after `timer.cancel()`, `quiet_for(250 ms)?.is_idle()`. Mutations: paint
   after each step (`needs_paint` false: fails); count only the last step's
   updates (the last step, at 250 ms, runs no tick: fails).
6. **A toast waiting to expire:** a `ToastHost` with one pushed toast,
   settled, then `quiet_for(1 s)`. Named assertion: `components_updated ==
   0`, `!needs_paint`, `next_deadline.is_some()` and `!is_idle()`. After
   `advance` past the expiry, `quiet_for(1 s)?.is_idle()`. Mutation:
   `is_idle` ignores `next_deadline`.
7. **Two windows (D5's scope against its alternative):** the setup of
   `click_refuses_another_window_and_a_refused_reveal` with a busy `Spinner`
   in the detached panel and nothing animating in the primary window.
   Primary selected: named assertion `quiet_for(250 ms)?.is_idle()`.
   Detached window selected: `components_updated >= 2` and
   `next_deadline.is_some()`. Mutation: read the deadline from
   `HostIntegration::application_deadline` (the first assertion fails).
8. **Baseline work is not counted:** write a signal a `Label` reads, then
   at once `quiet_for(100 ms)`. Named assertion: `is_idle()` (the update ran
   in the baseline). Mutation: add the baseline flush's count.
9. **The clock moves by exactly the interval:** from a clock of 0,
   `quiet_for(40 ms)`. Named assertion: `now() == 40 ms`. With
   `quiet_for(Duration::ZERO)`, named: `now()` unchanged and
   `components_updated == 0`. Mutations: round up to whole frames (48 ms);
   take one step for a zero interval (16 ms).
10. **A paint failure is `TestError::Draw`:** a leaf whose `paint` returns
    `DrawError::TooComplex`. Named assertion:
    `quiet_for(ZERO) == Err(TestError::Draw(DrawError::TooComplex))`. A leaf
    whose `paint` returns `DrawError::Runtime(failure(..))`: named,
    `is_failure(&error, ..)` with `TestError::Runtime`. Mutations: ignore
    the paint's error (`Ok` returned: the first fails); a derived
    `#[from]` that wraps every variant (the second fails).
11. **A refused baseline is activity, not an error:** a component keeps the
    `Notifier` it took in its first update in a cell the test shares
    (`Notifier` is `Clone`, `runtime/notify.rs:242-245`), and its
    `deliver_notifications` queues a registered command that counts its
    runs with `EventContext::invoke_command`. After settling, the test calls
    `notifier.request()` directly, outside any input entry point (an input
    entry point would deliver the notification in a flush of its own before
    returning, `runtime/notify.rs:347-354`, RT:4961). The baseline flush
    then delivers the notification after dispatching commands (RT:5771,
    :5786), so the command is still queued when it paints. Named assertion:
    `quiet_for(ZERO)` returns `Ok` with `baseline_painted == false`,
    `next_deadline == Some(now)` and `!is_idle()`, and the command has run
    zero times. Mutation: return the refusal as `TestError::Draw`.
12. **A refused baseline is never idle, even when the end needs no paint:**
    the same setup, the window painted once before the call, then
    `quiet_for(100 ms)`. The first step runs the queued no-op command, which
    damages nothing. Named assertion: the command ran once,
    `components_updated == 0`, `next_deadline == None`, `needs_paint ==
    false` (the host's observation at the end) and `baseline_painted ==
    false`, so `!is_idle()`. Mutations: `is_idle` ignores `baseline_painted`
    (the idle assertion fails); force `needs_paint` true after a refused
    baseline, revision 1's rule (the `needs_paint == false` assertion
    fails).
13. **A component's own `PendingUpdates` is an error:** a leaf whose `paint`
    returns `DrawError::PendingUpdates`. Named assertion: `quiet_for(ZERO) ==
    Err(TestError::Draw(DrawError::PendingUpdates))`, and `last_failure()`
    names the leaf with `FailurePhase::Paint`. Mutation: treat every
    `PendingUpdates` as rule 3's refusal (`Ok` returned).

### Contract and compatibility tests

- **Overflow is checked first:** with the clock past zero (`advance(1 ms)`,
  so that `now + Duration::MAX` overflows) and damage pending (a written
  label, not settled), `quiet_for(Duration::MAX)` returns
  `TestError::ClockOverflow`,
  the clock is unchanged and `HostIntegration::needs_paint` is still true
  (nothing was flushed or painted). Mutation for the rule: check after the
  baseline.
- **`clear_focus`'s two reaches:** a detached window with a focused field
  and an admitted held-input registration (`Runtime::register_held_input`,
  `runtime/held.rs:74`) holding a key; the primary window selected and
  `driver.clear_focus()?`. Named assertions: the detached window's focused
  component is unchanged (select it and read `focused()`), and the held
  registration received `HeldCancellation::FocusLost`. It records today's
  behavior, which P4 keeps and documents; mutation for the rule: filter
  `cancel_held` by window (the second assertion fails).
- **Command admission in a step, with another window's work due:** a
  primary-window component whose event timer (`EventContext::set_timer`,
  due inside the interval) queues a registered no-op command from
  `timer_elapsed`; the detached window holds a component that counts its
  updates and requests one with `wake_after` each time. With the primary
  selected, `quiet_for(250 ms)`. Named assertions: the detached counter
  advanced during the call (the admission's nested `flush_windows` ran it);
  `components_updated` equals the primary window's own updates, without the
  detached ones; `next_deadline` holds no detached deadline. It records the
  inherited qualification of 2.2 rules 5 and 6; it guards no rule P4 could
  break by mistake, so it has no mutation.
- **A queued reveal in a step, with another window's work due:** the same
  detached counting component, and a primary-window component whose event
  timer, due inside the interval, calls `EventContext::request_reveal` on
  a component in the detached window from `timer_elapsed`. With the primary
  selected, `quiet_for(250 ms)`. Named assertions: the detached counter
  advanced during the call (the reveal's nested flush of its target's
  window ran it); `components_updated` equals the primary window's own
  updates; `next_deadline` holds no detached deadline and the primary is
  still selected afterwards (`current_window()`). Like the command case, it
  records inherited behavior and has no mutation.
- **A step's flush error ends the call:** a component that fails its update
  when an interval tick writes its binding; `quiet_for(250 ms)` returns that
  error as `TestError::Runtime`.
- **The report must be read:** a `compile_fail` doctest on `QuietReport`
  under `#![deny(unused_must_use)]` with `driver.quiet_for(d)?;`, beside a
  passing doctest that differs only by binding the report (so the failure is
  the discarded report, not another error), as P3 does for its receipt.
- **Doctests:** the examples of `clear_focus`, `quiet_for`, `TestError::Draw`
  (in the facade's `testing` page or the variant's rustdoc) and the changed
  `DecimalInput` and facade `testing` examples.
- **Module membership** (P2's tests, section 4): `every_name` imports
  `QuietReport` and `DrawError` from `pert_ui::testing`;
  `every_glob_combines` resolves them; the listing-driven test passes with
  the new listings, and fails, once the listing is regenerated, when
  `DrawError` is removed from `testing`, which shows that it sees
  `TestError::Draw`.
- **The existing driver tests** (`tests/testing.rs`) and the testing
  chapter's doctests pass unchanged.

## 8. Scope, version and evidence

- **`api/` diff:** added `Driver::clear_focus`, `Driver::quiet_for`,
  `QuietReport` with its four fields, `is_idle` and derived impls,
  `TestError::Draw`, `impl From<DrawError> for TestError`, and the
  `testing` re-export of `DrawError` (P4-D1), in
  `api/pert-ui-runtime/{default,serde}.txt` and in every
  `api/pert-ui/*.txt`, since the facade's `testing` module carries them in
  every configuration.
- **Breaking:** no. `TestError` is `#[non_exhaustive]` (T:709), so a new
  variant changes no match outside the crate; the new conversion and glob
  names are additions, checked by the collision review below.
- **Collision review:** `QuietReport` occurs nowhere in the repository
  (`git grep`), and `DrawError` is the same item in every module that
  exports it; no maintained consumer defines either name.
- **CHANGELOG** for 0.3.79, citing this checkpoint:
  - `Added (R2-11):` `Driver::clear_focus`, the selected window's
    deactivation followed by a settle, for "nothing focused" in a test.
  - `Added (R2-24):` `Driver::quiet_for` and `QuietReport`
    (`components_updated`, `next_deadline`, `needs_paint`,
    `baseline_painted`, `is_idle`): a painted baseline, then an interval in
    frame steps, over the selected window and the work windows share.
  - `Added (R2-24):` `TestError::Draw` and `From<DrawError> for TestError`,
    which keeps a runtime failure as `TestError::Runtime`; `DrawError` in
    `testing` (P4-D1).
  - `Documentation (R2-11, R2-24):` the testing chapter's "Nothing focused"
    and "Nothing scheduled while idle"; `HostIntegration::clear_focus` and
    `needs_paint` point to the driver, and `needs_paint` names a waiting
    capture; `Runtime::next_deadline` states its scope; the `testing` and
    `host` pages; `DecimalInput`'s example uses the driver.
- **Version:** 0.3.79 after P3's 0.3.78: the workspace version, the internal
  version requirements and `Cargo.lock`. If the order changes, the heading
  follows the actual version.
- **Gate:** the public-API row, as the plan states: the workspace checks
  (formatting, Clippy, tests including P2's listing-driven test and section
  7, the packaged examples' headless sessions, doctests, warning-free
  rustdoc, the diagnostics snapshots), the API listing gate with its diff
  reviewed against the CHANGELOG, the mutations of section 7 recorded in the
  Progress entry, and Codex's implementation review. No fixture check
  (section 6).
- **Native evidence:** not applicable. P4 adds test-only facilities and
  changes no platform code; the native `App` keeps calling
  `HostIntegration::clear_focus` and `needs_paint` unchanged.
- **Persistence, real time, backends:** no persisted schema, live-data
  endpoint or backend type is involved.

## Corrections to the plan

- **Not breaking.** The order table and P4's section list "a `TestError`
  variant" as breaking; adding a variant to a `#[non_exhaustive]` enum
  breaks no consumer, so the CHANGELOG has `Added`, not `Breaking:`.
- **`TestError::Draw` and the one-form rule.** D5 adds `Draw(DrawError)`
  "for the paint"; a derived conversion would put a runtime failure in two
  forms, against `TestError`'s rustdoc (T:704-705). Section 2.3 routes
  `DrawError::Runtime` to `TestError::Runtime`.
- **Why `HostIntegration` stays in `testing`.** D5 says it stays "for
  `select_window` and `set_clipboard`, which `Driver`'s errors name".
  `Driver` names `select_window` (T:111, :720-722) and `set_wake_delivery`
  (T:309-311); no driver item names `set_clipboard`. It stays because a test
  standing in for a host calls `select_window`, `take_presentation_requests`,
  `set_clipboard` and the others; P4 changes only what a test is taught for
  focus and idleness.
- **The spinner test needs an interval over 100 ms.** A `Spinner` steps every
  100 ms (`progress.rs:302`), not every frame, so a shorter interval reports
  a deadline with no update; section 7 uses 250 ms.
- **"It settles, paints once":** `Driver::settle` moves the clock by a frame
  (T:256-268); the baseline flushes at the current clock instead, so the
  report covers exactly the requested interval (2.2 rule 2).
- **Test location:** the driver's tests are runtime tests in
  `crates/pert-ui-runtime/tests/testing.rs`, not a facade area binary.
- **`needs_paint`'s documentation omits captures and the command queue**
  (H:272-278 against CO:342-364 and CO:49); fixed in P4 because P4 edits
  that rustdoc and the report relies on it.
- **D5's scope rationale needs a qualification.** D5 aligns the check with
  `Driver::settle` and says another window's own timers do not make the
  report non-idle. That holds for the final observations, but nested
  runtime operations flush other windows: a queued command's admission
  (`runtime/commands.rs:1045`), a queued reveal (RT:5813,
  `runtime/reveal.rs:546-559`) and deferred tasks or interval ticks that
  call flushing operations (`docs/contracts.md:1116-1121`). A step can so
  run other windows' due work, and the nested updates are not counted.
  Section 2.2 rules 5 and 6 state it; the scope itself is unchanged.

## Findings outside this part

- `crates/pert-ui-runtime/src/runtime/held.rs:15-17`
  `HeldCancellation::FocusLost`: the link text reads
  `` `Runtime::clear_focus` `` but targets
  `HostIntegration::clear_focus`; since L3's N3 there is no public
  `Runtime::clear_focus` (RT:5315-5316). Public API documentation: the
  rendered name points readers to a method that does not exist. Fix: link
  text `HostIntegration::clear_focus`. P4's implementation records it in
  TODO.md (searching for an existing entry first) and does not fix it.

## Not in this part

- **Naming what is active.** A non-idle report gives counts, not the
  components that updated or scheduled; profiling and development validation
  already name pending components. No run needed more.
- **Every-window idleness** (D5's alternative scope): a test selects each
  window in turn.
- **`Driver` methods for other host calls** (`select_window`,
  `set_window_visible`): not requested by round 2.

## Decisions

- **P4-D1: `DrawError` in `testing`.** `TestError::Draw` makes `DrawError`
  reachable from every driver method's error.
  - **Recommended: export it from both `testing` modules.** A test that
    asserts a paint failure writes `TestError::Draw(DrawError::...)`; the
    component that fails may live in the application crate, so the test file
    need not glob `component`. It is the same item as at the roots and in
    `component`, so the globs combine and nothing collides. Its own payloads
    follow P2's rule for an error's payloads (section 4).
  - **Alternative: P2's exclusion list,** with the reason that a test only
    propagates the error with `?` into `Box<dyn Error>`. One name fewer in
    the glob, but a test that matches the variant needs a second import,
    the recurrence of F3 that P2 exists to end.
- **P4-D2: which `PendingUpdates` from the baseline paint is activity.**
  `DrawError::PendingUpdates` comes from the runtime's admission refusal
  (CO:49-61) or the unsized-entry guard (CO:99-107), and also from any
  component whose `paint` returns it (RT:2194, CO:146-157).
  - **Recommended: classify by provenance.** Only a `PendingUpdates` after
    which `Runtime::last_failure()` names no `FailurePhase::Paint` site is
    the runtime's refusal, and so activity (rule 3); a component's returned
    `PendingUpdates` is its paint failure, `TestError::Draw`, like any other
    variant it returns. The distinction uses the public failure-site
    contract, which an admitted paint resets and only a component failure
    sets (`runtime/application.rs:430-436`, CO:48, CO:156), so it needs no
    new runtime internals. A component bug then surfaces as an error the
    test sees, instead of a report that is never idle with no cause named.
  - **Alternative: every `PendingUpdates` is activity.** One branch fewer,
    but a component that misuses the variant makes every check non-idle and
    hides the failing component that `last_failure` would have named.

## Review record

- **Revision 1 (2026-10-28):** written from the code at ee55226 (library
  code as at 26e6b5a).
- **Revision 1, Codex review (2026-10-28, against ee55226):** needs
  revision, with six should-fix points and two nits; the unchanged schemas,
  the nonbreaking variant, the part order and the test location were found
  justified. Its record is
  `~/pert-ui-usability/codex-prompts/codex-design-P4.out`. Each point was
  checked against the code at HEAD before revision 2 applied it; none was
  rejected.
  1. **Refused-baseline test precondition:** `Runtime::key_input` delivers
     the notification it caused in a flush of its own before returning
     (RT:4961, `runtime/notify.rs:347-354`), so the command ran before the
     baseline. Test 11 requests a retained `Notifier` directly after
     settling; the assertions and mutation stay.
  2. **Sticky `needs_paint`:** forcing it true after a refused baseline
     contradicted D5's "at the interval's end", and `needs_paint` does not
     read the command queue (CO:342-364 against CO:49), which also
     corrected section 1. `needs_paint` is now only the host's final
     observation; a refused baseline is the new field `baseline_painted`,
     which `is_idle` requires; test 12 covers a nonzero interval whose
     queued command completes without damage.
  3. **`PendingUpdates` provenance:** a component can return the variant
     (RT:2194, CO:146-157). Decision P4-D2 classifies by
     `Runtime::last_failure`'s `FailurePhase::Paint` site; rule 3, rule 8,
     the implementation outline and test 13 agree.
  4. **`clear_focus` reaches beyond the selected window:** held
     registrations are cancelled runtime-wide (RT:5320, `held.rs:166-171`,
     `:15-20`), and keys, command presses and modifiers are runtime state
     (RT:5337-5341). Section 1 and rule 2.1.2 separate the two reaches; a
     compatibility test shows another window keeping focus while its held
     registration gets `FocusLost`.
  5. **Focus-error ordering:** the flush delivers queued parameter
     notifications before it returns the focus error (RT:5767-5770), and
     answers inspection on the error (RT:5736). Section 1 and rule 2.1.4
     state the actual order without claiming a side-effect-free flush.
  6. **Nested command admission:** a dispatched command's admission calls
     `flush_windows` (`runtime/commands.rs:1698`, `:1045`), whose updates in
     any window are not in the outer report. Rules 2.2.5 and 2.2.6 state the
     limit, the final observations stay selected-window scoped, a
     compatibility test covers it, and a correction qualifies D5's
     rationale.
  7. **Nit, normalization:** a consumer can construct
     `TestError::Draw(DrawError::Runtime(..))`; the text now says the
     driver and `From<DrawError>` always normalize.
  8. **Nit, wording:** the "only example" is narrowed to the testing pages,
     `Update` joins the phases `invoke_command` refuses (RT:713), and rule
     2.2.9's example is a number field's held-step timer
     (`text_fields/number_inputs.rs:1122-1133`), since a held command
     repeats only on incoming repeated-key events
     (`runtime/commands.rs:1435`).
- **Revision 2, Codex recheck (2026-10-28, against ee55226):** needs
  revision; the eight revision 1 points were found resolved, one should-fix
  left. Record `~/pert-ui-usability/codex-prompts/codex-design-P4-r2.out`.
  Checked against the code before revision 3 applied it:
  1. **Other nested flushes:** queued reveals are dispatched after the
     update waves (RT:5813) and each selects its target's window and flushes
     it (`runtime/reveal.rs:546-549`, `:559`), with no command involved;
     deferred tasks and interval ticks may call flushing operations
     (`docs/contracts.md:1116-1121`). Revision 3 generalizes section 1, rules
     2.2.5 and 2.2.6 and the D5 correction to nested runtime operations,
     naming reveals; `components_updated` stays the sum of the reports the
     steps' flushes return; a compatibility test covers a primary-window
     timer whose reveal runs a detached window's due work.
- **Revision 3, Codex second recheck (2026-10-28):** no new findings; the
  remaining point is resolved and the earlier corrections hold. Ready for
  approval. Approved by the author under the maintainer's delegation, with
  P4-D1 and P4-D2 as recommended.
