# Round 2 fix plan, part P11 design checkpoint: application state from the initializer

Part P11 of [the round 2 fix plan](round-2-fix-plan.md#p11-application-state-from-the-initializer-0386)
closes R2-1 by decision D9 (approved with the recommended option on
2026-10-28), takes up the follow-up "Typed state handoff from `App::new`"
(`plans/follow-ups.md:33-35`) and resolves the TODO.md entry on the L3
native-probe `UnexpectedOverlap` noise (`TODO.md:138-152`). The plan gives
P11 a checkpoint for the type, the handler signatures (including
`on_window_error`), the native runner's erasure and the migration list; this
document settles them, with the exact API, behavior rules, tests, CHANGELOG,
gate and native evidence, before any code. Codex reviews it; the author
approves it together with Codex's review under the maintainer's delegation
of 2026-10-28.

Every reference to code is to `d4e1386` (P3 committed, 0.3.78);
revision 2 rechecked its citations at `7bff14d`, whose code under
`crates/`, `apps/` and `docs/` is the same (`git diff --stat d4e1386
7bff14d` over them is empty). The behavior of the files P11 changes is
unchanged since 26e6b5a (0.3.75), the commit the plan cites: `git diff
--stat 26e6b5a d4e1386` over `F/app.rs`, `F/app/`, `F/platform/`,
`crates/pert-ui-wgpu/src/`, `R/validation*` and `R/runtime/application.rs`
shows only 16 added rustdoc lines in `R/validation.rs`. P4's uncommitted
working-tree edits are not cited. Parts P4 to P10 land before P11 and may move lines in the
cited guide chapters and tests, so the implementer relocates a citation by
its content. Path prefixes are the plan's: `R/` =
`crates/pert-ui-runtime/src/`, `F/` = `crates/pert-ui/src/`, `G/` =
`crates/pert-ui/src/guide/`; `P/` = `crates/pert-ui/src/platform/`, `W/` =
`crates/pert-ui-wgpu/src/`, `T/` = `crates/pert-ui/tests/`, `E/` =
`crates/pert-ui/examples/`, `GA/` = `apps/gallery/src/`, `U` =
`~/pert-ui-usability/` (frozen runs).

Status: **revision 2 with the author's correction of Codex's recheck,
approved 2026-10-28** by the author together with Codex's review (recheck:
ready for approval, one nit, corrected as Codex proposed), under the
maintainer's delegation. Revision 1 was reviewed by Codex (needs
revision: one blocker, three should-fix points and a nit); revision 2
answers each ([review record](#review-record)). P11-D1 (as revised), P11-D2
and P11-D3 are approved by the author as recommended, under the
maintainer's delegation of 2026-10-28.

## What testers built by hand, and what the library says

- **The slot (R2-1, results row at `plans/archive/developer-usability.md:919`,
  triage N1 at `runs/triage-r2/group-1.md:236`).** Every run that wrote a
  close policy or a headless test of an `App` built a shared cell that the
  initializer fills and the policy and the test read:
  `Rc<RefCell<Option<Built>>>` with a `#[derive(Clone)] struct Built`
  (U `r2-1-claude/work/src/lib.rs:111-116`), a policy that does nothing
  while the cell is empty (U `r2-1-codex/work/src/main.rs:270-272`), an
  `fn app(deps) -> (App, Rc<OnceCell<Built>>)` returning the slot beside the
  application (U `r2-5-head-claude-2/work/src/lib.rs:421-425`), and test
  setups that take the value out of the slot after `headless()`
  (U `r2-5-head-claude-1/work/tests/app.rs:19-23`,
  U `r2-3-claude/work/tests/headless.rs:9-13`,
  U `r2-2-claude/work/tests/headless.rs:299-306`).
- **The library teaches it.** `HeadlessApp`'s example
  (`F/app/headless.rs:22-73`), the testing chapter's "The close policy"
  (`G/testing.md:362-417`) and the custom-widgets chapter's "Building the
  application" (`G/custom-widgets.md:585-627`, "share the application with
  them through a slot the initializer fills, and decide what the callbacks
  do while it is still empty") all build one, and so does the packaged
  diagram editor (`E/diagram_editor/main.rs:60-97`), whose two handlers each
  carry a branch for the empty slot.

## 1. Today's behavior, from the code

- **The type.** `App` is a non-generic struct (`F/app.rs:322-333`) holding
  the title, main-window geometry, the boxed initializer
  (`Initialize = Box<dyn FnOnce(&mut Runtime) -> Result<(), AppError>>`,
  `F/app.rs:252`), the `Runtime`, the software flag and three optional
  boxed handlers (`F/app.rs:246-251`). `App::new` takes the initializer
  (`F/app.rs:358-372`); its rustdoc asks the application to "retain
  parameter host owners in components or captured application-owned state"
  (`F/app.rs:354`).
- **The handlers** receive only the runtime and their argument:
  `on_error(FnMut(&mut Runtime, &AppError) -> ErrorAction)`
  (`F/app.rs:507-513`), `on_close(FnMut(&mut Runtime, &NativeWindowId) ->
  Result<NativeCloseAction, RuntimeError>)` (`F/app.rs:579-589`) and
  `on_window_error(FnMut(&mut Runtime, &WindowError))` (`F/app.rs:594-600`,
  no example). Defaults: stderr report plus `Retry` (`F/app.rs:627-655`),
  `Close` (`F/app.rs:617-625`), stderr report (`F/app.rs:601-616`).
  Initialization failures bypass `on_error` (`F/app.rs:483-485`).
- **The native runner** owns the whole `App`: `platform::run(app: App)`
  validates the title, creates the event loop, installs the clipboard, runs
  the initializer and, on its failure, shuts the runtime down through
  `finish_runtime` (`P/mod.rs:63-78`, `:125-140`), then moves the `App` into
  `Runner { app: App, .. }` (`P/mod.rs:142-158`, `:85`). Handlers are called
  as `self.app.handle_error` from component recovery (`P/schedule.rs:15-30`)
  and the surface-limit path (`P/mod.rs:429`), `handle_window_error` from a
  failed presentation request and a stale queued close
  (`P/windows.rs:159`, `:244-249`), and `handle_close` from close admission
  (`P/windows.rs:265-272`). After the loop, `run` calls `finish_runtime` and
  drops the runner (`P/mod.rs:101`), so everything the handlers captured is
  dropped after the runtime has shut down.
- **Six native smokes bypass `run`:** they build `App::new(.., |_| Ok(()))`,
  never run the initializer (five discard it with the crate-private
  `app.initialize.take()`), set content up on `app.runtime` directly and
  call `Runner::new(app, ..)`
  (`P/capture_smoke.rs:227-229`, `:299`; `P/drag_smoke.rs:658-659`, `:704`;
  `P/multi_window_smoke.rs:274-276`, `:327`, `:443-444`, `:458`;
  `P/pacing_smoke.rs:134-135`, `:145`; `P/smoke.rs:1131-1148`). The
  scheduler's unit tests call `ComponentRecovery::handle(&mut app, ..)` on an
  `App` that never ran its initializer (`P/schedule.rs:474-553`).
- **`HeadlessApp`** wraps the whole `App` (`F/app/headless.rs:74-77`).
  `App::headless` validates the title, sets the viewport, runs the
  initializer, selects the primary window and flushes every root; any
  failure shuts the runtime down and is returned (`:122-146`). It exposes
  `runtime`, `runtime_mut`, `main_window`, `admit_close` and `shutdown`
  (`:149-229`); `admit_close` calls `self.app.handle_close` (`:204`).
  `on_error` and `on_window_error` are not called headless (`:109-111`).
  Its `Debug` shows only the title and the geometry (`:79-88`), which a test
  asserts verbatim (`T/surfaces/window_close.rs:450`).
- **Keeping values alive.** `Runtime::keep_alive` is documented as "the
  owner for values an application creates once ... made in the App
  initializer" and drops kept values last during shutdown, after unmounting
  (`R/runtime/application.rs:107-142`); the manual says the same
  (`docs/manual.md:76-77`). Four packaged examples keep their application
  object that way (`E/diagram_editor/main.rs:70`, `E/inventory/main.rs:53`,
  `E/log_explorer/main.rs:55`, `E/live_mixer.rs:792`).
- **Overlap validation** (debug builds only) compares every pair of visible
  components with `OverlapPolicy::Disallow` that share a window and a parent
  and writes one `[pert-ui validation ERROR UnexpectedOverlap]` line per new
  pair (`R/validation/development.rs:677-694`, `:340-342`). Roots share the
  parent "none", and a root's allocation is the whole window
  (`F/app.rs:289`), so two visible roots of one window overlap. A component
  opts out by reporting `OverlapPolicy::Allow` from `Component::inspect`
  (`R/validation.rs:8-20`); a stacking parent's children are allowed
  (`R/runtime/inspect.rs:182-184`). The overlapping probes, by code:
  - `verify_mixed_scene`, run inside `native_gpu_pixel_readback`
    (`W/native_smoke.rs:1440`), mounts 1,000 `static-N` roots and a
    `changing` root of one `Probe` component (`W/native_smoke.rs:193-246`):
    1,001 roots and 500,500 pairs, the TODO's 499,500 pairs of static roots
    plus 1,000 with `changing`, which leaves three of its 500,503 lines
    (`TODO.md:145`) to the other sources;
  - the damage-area probe (`native_damage_area_probe`) mounts a `grid` root
    and a `trace` root over it (`W/native_smoke.rs:375`, `:408`);
  - the native smoke mounts `persistent-paint-error` beside the content
    still shown at that stage (`P/smoke.rs:940-951`); its other stages
    unmount or hide the previous stage's root first (`P/smoke.rs:323`,
    `:494`, `:616`, `:794`).
  The realtime workload probe the TODO also names mounts one leaf root
  (`W/realtime_probe.rs:211`) and overlaps nothing.

## 2. The rules

### 2.1 The state type

1. `App<S = ()>`. The initializer returns `Result<S, AppError>`; `S` is the
   **application state**: whatever the application wants its lifecycle
   callbacks and tests to reach, typically the handles its initializer
   built (signals, `WidgetId`s, controllers, an `Rc` of its application
   object) or the model itself.
2. `S: 'static`. It need not be `Send`, `Sync`, `Clone`, `Debug` or
   `Default`: it lives on the UI thread with the `App`, which is `!Send`
   already.
3. **Opaque.** The library stores `S`, lends it to the application's own
   callbacks and drops it; it never reads, clones, compares, serializes or
   persists it. The application's data model stays the application's
   (`docs/architecture.md:15-16`): `App` keeps an application value, as
   `keep_alive` does, without giving it meaning.
4. **Inference.** `S` is inferred from the initializer's `Ok` value: an
   initializer ending in `Ok(())` keeps `S = ()` and compiles unchanged. The
   default `()` applies where a type is written (`App`, `HeadlessApp`,
   `fn app() -> App`), not to a call, because Rust does not use a defaulted
   type parameter to infer a call. An initializer that never returns `Ok`
   (one that always fails or panics) therefore names the type:
   `App::<()>::new("x", |_| Err(..))`. Checked with rustc 1.99.1 on a
   reduced model in the session scratchpad: `App::new(|_| { fail()?;
   Ok(()) })`, `App::new(|_| Ok(String::new())).on_close(|_, s| ..)` and
   `let c: App = App::new(|_| panic!())` compile; `let d =
   App::new(|_| panic!())` fails with E0282 ("type annotations needed for
   `App<_>`"). The rustdoc of `App::new` states it.

### 2.2 Lifetime and drop order

1. The state exists from the initializer's successful return until the
   application ends. It is owned by the native runner (after `App::run`
   has run the initializer) or by the `HeadlessApp`.
2. **It is dropped after the runtime has shut down, on every path:** the
   end of `App::run` (normal or failed after initialization), a failed
   `App::headless` after the initializer returned (its first flush failed),
   `HeadlessApp::shutdown`, and dropping a `HeadlessApp`, whose runtime
   shuts down in `Runtime`'s drop. So a state's own `Drop` sees a closed
   runtime (a `Signal` read returns `RuntimeError::Closed`), components
   have unmounted and parameters have ended their edits before a parameter
   host in the state goes away, the order `keep_alive` gives its values
   (`R/runtime/application.rs:117-120`) and the order today's captured
   handler values already have (section 1). An `App` that never ran holds
   no state.
3. Values a handler captures (handlers stay `'static` closures and may
   capture) are dropped at the same point as the state.

### 2.3 The handlers

1. Signatures (section 3): each handler receives `&mut Runtime`, then
   `&mut S`, then its argument: `on_error(|ui, state, error| ..)`,
   `on_close(|ui, state, window| ..)`, `on_window_error(|ui, state, error|
   ..)`. `on_window_error` receives the state too (D9 left it to the
   checkpoint): otherwise it would be the one handler still needing a
   slot, and its native evidence is available (section 8).
2. **Never without state.** A handler runs only after the initializer has
   returned `Ok`: initialization failures bypass all three
   (`F/app.rs:483-485` for `on_error`; `on_close` and `on_window_error` are
   invoked only by the runner and by `HeadlessApp::admit_close`, which exist
   only after initialization). This holds by construction (2.5), not by a
   check.
3. **Exclusive.** The runner and `HeadlessApp` call one handler at a time on
   the UI thread and never from inside another handler: a handler receives
   the runtime, not the runner, and what it asks the runtime to do
   (`request_close`, a detach) is queued and handled after it returns. The
   `&mut S` is therefore unaliased for the call, and a change persists to
   the next call and to `HeadlessApp::state`.
4. Everything else is unchanged: timing (`docs/contracts.md:622-626`,
   `:1124-1128`), close admission's order (`F/app.rs:521-577`), defaults,
   retries, error routing, `Defer` semantics.

### 2.4 `HeadlessApp<S>`

1. `HeadlessApp<S = ()>`, returned by `App::<S>::headless`.
2. `state(&self) -> &S` and `state_mut(&mut self) -> &mut S` (D9), and
   `runtime_and_state_mut(&mut self) -> (&mut Runtime, &mut S)`
   ([P11-D2](#decisions)): the runtime and the state at once, as a handler
   receives them, so a test drives the runtime while it reads the state's
   handles (`Driver::new(ui).replace_text(&notes.field, ..)`) without
   cloning them out first.
3. `admit_close` passes the same state to `on_close` that `state()` reads;
   its contract and errors are otherwise unchanged (`F/app/headless.rs:165-193`).
4. `Debug` is unchanged: title and geometry, not the runtime, the callbacks
   or the state, so it needs no `S: Debug`.

### 2.5 The native runner stays non-generic ([P11-D1](#decisions))

1. The generic parameter stays in `F/app.rs`, `F/app/headless.rs` and one
   thin startup function, `platform::run<S: 'static>(app: App<S>)`. Its body
   is today's startup sequence in today's order (`P/mod.rs:63-84`): validate
   the title, create the event loop, install the clipboard on the
   application's runtime, then call `App::<S>::start` (rule 4) and erase the
   result into `Running` at once, then install wake delivery. Everything
   after that, from `Runner::new` on (`P/mod.rs:85-101`), takes the erased
   `Running`: `Runner`, `P/windows.rs`, `P/schedule.rs` and the smokes stay
   non-generic. The event loop therefore still exists before the
   initializer runs, so `App::run`'s guarantee that a call which cannot run
   never builds the host's content (`F/app.rs:660-662`, `:686-689`, tested
   by `T/foundation/public_api.rs:33-56`) is unchanged.
2. The state is erased behind a private trait object, not `dyn Any`: no
   downcast, so no `expect` in a production path (AGENTS.md "Rust and data
   invariants").
3. Private shape (names are the implementer's; the rules above bind):
   - `Host`: the non-generic part every form shares (title, `MainWindow`,
     `Runtime`, software flag).
   - `App<S> { host: Host, initialize: Box<dyn FnOnce(&mut Runtime) ->
     Result<S, AppError>>, handlers: Handlers<S> }`, `Handlers<S>` holding
     the three optional boxed handlers typed in `S`.
   - `Bound<S> { state: S, handlers: Handlers<S> }`, built only from the
     initializer's `Ok` value, implementing a private `trait Lifecycle` with
     one method per handler that returns `None` when none is installed.
   - `Running { host: Host, lifecycle: Box<dyn Lifecycle> }`: what `Runner`
     holds instead of `App`. `Running::handle_error`, `handle_close` and
     `handle_window_error` apply today's defaults when the lifecycle
     returns `None` (`F/app.rs:601-655` move unchanged). The runner keeps
     reading `self.app.runtime`, `title`, `window` and `software` through
     `Host`.
   - `HeadlessApp<S> { host: Host, bound: Bound<S> }`, typed, so
     `state()` needs no erasure.
   - **Field order:** `host` before `lifecycle` in `Running` and before
     `bound` in `HeadlessApp`, so a drop without explicit shutdown drops the
     runtime (which shuts down in its drop) before the state (2.2 rule 2).
4. A crate-private `App::<S>::start(self) -> Result<(Host, Bound<S>),
   AppError>` runs the initializer and binds the handlers; on failure it
   shuts the runtime down through `finish_runtime` and returns that error
   (today's `P/mod.rs:74-78`). `platform::run` calls it where it runs the
   initializer today (rule 1) and erases the result into `Running`;
   `App::headless`
   uses it (after setting the viewport, as today) and keeps `Bound<S>`. The
   six smokes replace their direct `Runner::new(app, ..)` (and
   `app.initialize.take()`) with `Runner::new(app.start()?.into(), ..)`, so
   their trivial initializer runs; the scheduler tests build a `Running` the
   same way, without a window.

### 2.6 Keeping values alive from the initializer ([P11-D3](#decisions))

Returning a value as the state also keeps it alive until the application
ends, which overlaps `keep_alive`'s documented use in the initializer.
AGENTS.md requires naming what the new way retires: **the initializer's use
of `keep_alive` is retired as the taught route.** An `App` initializer
returns what it must keep in its state; `keep_alive` stays the owner for
code that has only a `Runtime` and cannot return to the initializer (a
custom host, a helper that builds part of an application, a deferred task).
Its rustdoc and the manual say so (section 5), and the four packaged
examples that keep their application object return it as the state
instead (section 6). The drop timing differs only in that the state goes
after shutdown completes and a kept value during its last step; both come
after components have unmounted.

### 2.7 Probes that overlap on purpose

1. Each probe component whose roots or siblings overlap by design reports
   `OverlapPolicy::Allow` from its own `Component::inspect`, the TODO's first
   option: `Probe` in `verify_mixed_scene` (every root is one, and they
   overlap one another), `Trace` in the damage-area probe (drawn over the
   `Grid`, which stays `Disallow`: the pair loop compares only disallowing
   components, `R/validation/development.rs:678`, so one exemption removes
   the pair), and `FailingPaint` in the smoke's `persistent-paint-error`
   stage (mounted over the content still shown). Mounting the 1,001 `static-N` roots under
   one stacking container instead would change what `verify_mixed_scene`
   measures (a thousand retained roots, `W/native_smoke.rs:224-246`), so it
   is not used. Allowing removes them from the pair loop, which removes
   the startup overlap diagnostics and the cost of finding them and
   building their report (the initial flush emits them before measurement
   begins, `W/native_smoke.rs:247`, `:253`, and repeated issues are not
   written again, `R/validation/development.rs:339`); any effect on the
   measured loop is left to measurement.
2. Each of the three probes asserts, once every overlapping root is
   mounted and flushed, that `Runtime::validation_report()`
   (`R/runtime.rs:4037-4075`) holds no `ValidationCode::UnexpectedOverlap`
   issue, and fails with its message otherwise:
   - `verify_mixed_scene`: right after the flush that follows mounting
     `changing` (`W/native_smoke.rs:247`);
   - the damage-area probe: right after the flush that follows mounting
     `trace` (`W/native_smoke.rs:417`), not after the first flush
     (`:376`), which sees only `grid`;
   - the smoke: in stage 46, after mounting `persistent-paint-error`
     (`P/smoke.rs:950`), the stage flushes the runtime itself
     (`self.app.runtime.flush(now)`; its update succeeds, only its paint
     fails) and asserts before returning, since the runner's later flushes
     end in the paint failures the stage provokes and the report after the
     loop is read after shutdown has removed every component.
   In a release build the report is disabled and empty, so the assertion
   holds vacuously; the native checks run the dev profile.
3. The native run's log (section 8) contains no
   `[pert-ui validation ERROR UnexpectedOverlap]` line from a probe. Any
   other validation line is listed in the Progress entry; one from a probe
   is fixed in P11 as a same-kind defect, one from the gallery or the
   library is recorded in TODO.md.

## 3. Public API

In `pert_ui` (root and prelude, unchanged membership) and
`pert_ui::testing` (`HeadlessApp`, unchanged membership), with the `native`
feature:

```rust
/// [Type-level rustdoc as today, plus "# Application state": what `S` is
/// (2.1), that the initializer returns it and the three handlers receive it
/// by `&mut` (2.3), that it lives until the application ends and drops
/// after the runtime has shut down (2.2), that it need not be `Send`, and
/// that the library never reads it. Example (no_run): an initializer that
/// returns `Notes { text, saved }`, an `on_close` that keeps the window
/// open while they differ.]
#[must_use = "call run to open the application, or headless in a test"]
pub struct App<S = ()> { /* private */ }

impl<S: 'static> App<S> {
    /// [As today, plus: the initializer's `Ok` value becomes the
    /// application state; `Ok(())` keeps `S = ()`; an initializer that never
    /// returns `Ok` names the type, `App::<()>::new` (2.1 rule 4); values
    /// the application must keep, parameter hosts and mailboxes included,
    /// go in the state (2.6).]
    pub fn new(
        title: impl Into<String>,
        initialize: impl FnOnce(&mut Runtime) -> Result<S, AppError> + 'static,
    ) -> Self;

    // size, min_size, max_size, resizable, software_rendering: unchanged.

    /// [As today, plus: receives the application state; example updated to
    /// show a component failure on a status line the state holds.]
    pub fn on_error(
        self,
        handler: impl FnMut(&mut Runtime, &mut S, &AppError) -> ErrorAction + 'static,
    ) -> Self;

    /// [As today; step 5 of "Close order" reads "This callback runs, with
    /// the closing window selected and the application state"; the example
    /// the type-level rustdoc gives is referenced.]
    pub fn on_close(
        self,
        handler: impl FnMut(&mut Runtime, &mut S, &NativeWindowId)
            -> Result<NativeCloseAction, RuntimeError> + 'static,
    ) -> Self;

    /// [As today, plus: receives the application state; states the default
    /// report (`[pert-ui window error] ...` on standard error) and that a
    /// failed report ends the application with
    /// `AppError::WindowErrorReporting` (a same-kind gap in touched rustdoc,
    /// `F/app.rs:601-616`); new no_run example counting failures in the
    /// state.]
    pub fn on_window_error(
        self,
        handler: impl FnMut(&mut Runtime, &mut S, &WindowError) + 'static,
    ) -> Self;

    pub fn run(self) -> Result<(), AppError>; // unchanged
    /// [As today; returns the state's `HeadlessApp`; step 3 says the
    /// initializer's state is kept for `HeadlessApp::state`.]
    pub fn headless(self) -> Result<HeadlessApp<S>, AppError>;
}
impl<S> std::fmt::Debug for App<S>; // output unchanged

/// [As today, plus the state accessors; example rewritten without a slot
/// (section 5).]
#[must_use = "dropping it shuts the runtime down without reporting cleanup errors"]
pub struct HeadlessApp<S = ()> { /* private */ }

impl<S: 'static> HeadlessApp<S> {
    // runtime, runtime_mut, main_window, admit_close, shutdown: signatures
    // unchanged; admit_close's rustdoc says the policy receives the state.

    /// The state the initializer returned, as the handlers see it. [Example:
    /// a policy counts refused closes in a plain field; the test reads it.]
    pub fn state(&self) -> &S;
    /// The state, for a change a test makes as the application would.
    /// [Example: the test resets that counter.]
    pub fn state_mut(&mut self) -> &mut S;
    /// The runtime and the state at once, as a handler receives them, so a
    /// test can drive the runtime with handles the state holds. [Example:
    /// `let (ui, notes) = app.runtime_and_state_mut();
    /// Driver::new(ui).replace_text(&notes.field, "..")?;`]
    pub fn runtime_and_state_mut(&mut self) -> (&mut Runtime, &mut S);
}
impl<S> std::fmt::Debug for HeadlessApp<S>; // output unchanged
```

- **Argument order** follows D9's replacement usage: runtime first, as in
  every runtime callback, then the state, then the event.
- **`&mut S`, not `&S`:** a handler may keep plain fields (a count, the
  last failure) without a `Cell`; the runner owns the state exclusively
  (2.3 rule 3), so nothing is lost.
- **No accessor returns the state from `run`:** see "Not in this part".
- No `#[must_use]` on the accessors, matching `runtime()` and
  `runtime_mut()`.

## 4. Module membership, prelude closure and dependencies

- **No new names.** `App` stays at the root and in the prelude, `HeadlessApp`
  in `testing` only (`F/lib.rs:356`). The handler argument types
  (`Runtime`, `AppError`, `ErrorAction`, `NativeCloseAction`,
  `NativeWindowId`, `WindowError`, `RuntimeError`) are already prelude
  names (`F/lib.rs:43`, `:51-54`, `:143-144`), so L2's closure rule adds
  nothing; `S` is a type parameter, not a type to export.
- **P2's `testing` rule** is re-applied by its listing-driven test
  (`tests/diagnostics/tests/testing_closure.rs`), which strips generics from
  paths (`:161-204`) and treats `S` as no pert-ui type. `every_name` and
  `every_glob_combines` are unchanged.
- **Earlier parts.** P11's examples use `TextField`, `Signal`, `Driver` and
  the close types only, none of which P4 to P10 change in a way P11's text
  depends on; they are written against the code at P11's base.
- **P12 depends on P11.** P12's settings-form recipe closes with a policy
  that reads P11's state (`on_close(|_, settings, _| ..)`), and its
  diagram-editor change reads the `FormFields` it adds through the state
  P11 introduces there (`Rc<DiagramApp>`, section 6), tested with
  `HeadlessApp::admit_close` and `state()`. P12 relies on 2.2 (lifetime),
  2.3 (exclusive `&mut S`) and 2.4 (accessors). P11's text names no P12
  item (no form discard, no recipe).
- **No dependency change**, no feature change, no new `unsafe`.

## 5. Documentation

Text describes P11's own commit.

- **`App`, `App::new`, the three handlers, `App::headless`**: section 3.
  `App::new`'s "Retain parameter host owners in components or captured
  application-owned state" (`F/app.rs:354`) becomes "in components or in
  the application state".
- **`HeadlessApp`'s example** (`F/app/headless.rs:22-73`): `build` returns
  `Notes`, the initializer is `|ui| Ok(build(ui)?)`, the policy is
  `|_, notes, _| ..`, the test uses `runtime_and_state_mut` to type the
  draft and `state()` for the assertions; no `Rc`, `RefCell` or "not built"
  branch.
- **Testing chapter, "The close policy"** (`G/testing.md:362-417`): the same
  rewrite, with a sentence that the initializer's value is the state that
  `on_close` and the test share, and that an application's builder
  function returns `App<State>` so a test calls `app().headless()?`. The
  paragraph on drafts admission does not save (`:420-425`) is unchanged.
- **Custom-widgets chapter, "Building the application"**
  (`G/custom-widgets.md:585-627`): "receive the Runtime but not what the
  initializer created; share the application with them through a slot"
  becomes: the initializer returns the application as its state, and
  `on_close`, `on_error` and `on_window_error` receive it beside the
  runtime; a failing initializer never reaches them, its error returns from
  `run`. The example's handler shows the failure on a status line the state
  holds; the "slot still empty" branch is gone.
- **`Runtime::keep_alive`** (`R/runtime/application.rs:107-115`): the owner
  for values code with only a `Runtime` creates once and must not lose
  (custom hosts, helpers that build part of an application, deferred work);
  "an `App` initializer returns such values in its application state
  instead, which `App` keeps until the application ends". The runtime crate
  links the facade through its docs.rs URL as it does today.
- **`docs/manual.md:76-77`**: values the application creates once and must
  keep, such as a `Mailbox` made in the initializer, are returned from the
  initializer as the application state, which `on_close`, `on_error` and
  `on_window_error` receive; a custom host keeps them with
  `ui.keep_alive(value)`.
- **`docs/contracts.md`**: "Callbacks outside the classes" (`:622-626`) and
  "Calling the runtime from callbacks" (`:1124-1128`) say the policies also
  receive the application state the initializer returned, after the
  runtime; one sentence states 2.2 rule 2 and 2.1 rule 3 (dropped after
  shutdown; opaque to the library).
- **Module pages:** the facade's `testing` page says `HeadlessApp` hands a
  test the state its initializer returned (`F/lib.rs:239-240`), and so does
  the crate root's table (`F/root.md:80`).
- **Not changed:** `G/layout.md:320-345` and the runtime README
  (`crates/pert-ui-runtime/README.md:1576-1591`) describe
  `FloatingWindow::on_close`, not `App`'s; `README.md`, `F/root.md:19`,
  `G/appearance.md:486`, `G/controls.md:351` and `docs/manual.md:27` use
  `Ok(())` initializers without handlers and compile unchanged;
  `docs/architecture.md` (2.1 rule 3 keeps its statement true).
- **Records:** `plans/follow-ups.md`'s "Typed state handoff from `App::new`"
  (`:33-35`) is removed. TODO.md's overlap entry (`:138-152`) is removed;
  its last sentence, a proposal to cap or summarize overlap reports, is a
  separate improvement and is re-recorded as a narrowed entry ("Findings
  outside this part"). `docs/verification.md` gains P11's native row
  (section 8).

## 6. Migration manifest

| Consumer | P11 |
|---|---|
| `F/app.rs`, `F/app/headless.rs`, `F/app/close.rs` | the type, `Host`/`Bound`/`Running` (2.5); `close.rs` is runtime-only and unchanged |
| `P/mod.rs` (`run`, `Runner`), `P/schedule.rs` (`ComponentRecovery::handle` takes `&mut Running`), `P/windows.rs` | `Running` in place of `App`; call sites unchanged otherwise |
| `P/smoke.rs`, `P/multi_window_smoke.rs` (two checks), `P/capture_smoke.rs`, `P/drag_smoke.rs`, `P/pacing_smoke.rs` | `app.start()` in place of `initialize.take()`; `smoke.rs`'s `on_error` and both multi-window checks' handlers use state (section 7) |
| `P/schedule.rs` tests (`:477`, `:526`, `:544`) | built through `start()` and erased into `Running`; the backoff test uses state (section 7, test 3); the repair test is restructured (section 7, test 4), since it replaced its handler after construction (`:544`) and a `Running` has no handler setter |
| `W/native_smoke.rs` (`Probe`, `Grid`, `Trace`) | `OverlapPolicy::Allow` and the report assertion (2.7) |
| `E/diagram_editor/main.rs` | the initializer returns `app::build(..)?` (`Rc<DiagramApp>`) as the state; no slot, no `keep_alive`; `on_error(|_, diagram, error| ..)` and `on_close(|ui, diagram, _| ..)` without the empty-slot branches; `app::close_requested` unchanged (P12's TODO) |
| `E/inventory/main.rs`, `E/log_explorer/main.rs`, `E/live_mixer.rs` (`window()` only) | return `(inventory, commands)`, `explorer`, `mixer` as the state instead of `keep_alive` (P11-D3) |
| Packaged examples' headless sessions | unchanged: they drive a `Runtime`, not `App` |
| `E/getting_started.rs` | unchanged (`Ok(())` initializer, no handlers) |
| `E/dynamic_documents.rs` | unchanged: it drives a `Runtime` directly and has no `App` (`E/dynamic_documents.rs:9`) |
| Gallery: `GA/interaction_check.rs:1127`, `GA/workspace_check.rs:211` | `.on_error(\|_, _, _\| ErrorAction::Exit)`; their slots are filled by components and read after `run`, not by handlers, and stay |
| Gallery: `GA/main.rs:320`, `:345`, `GA/multi_window.rs:58` | unchanged |
| `T/surfaces/window_close.rs` | the `start` helper (`:13-37`) and the slot at `:419-431` removed: the initializer returns the built value, tests read `state()` and `runtime_and_state_mut()`; the initializers that never return `Ok` (`:462-464`, `:486-489`) write `App::<()>::new`; section 7's tests |
| `T/foundation/public_api.rs:35` | `App::<()>::new` (a panicking initializer, 2.1 rule 4) |
| `T/foundation/modules.rs` | unchanged (no new names) |
| `T/surfaces/windows.rs` | unchanged: its `on_close` (`:510`) is `FloatingWindow`'s |
| Rustdoc and guide | section 5 |
| `docs/manual.md`, `docs/contracts.md`, `docs/verification.md` | section 5, section 8 |
| READMEs | unchanged (section 5) |
| Diagnostics fixtures (`tests/diagnostics/ui/`) | none uses `App` (`git grep` finds no `App::new`); the snapshots run in the gate |
| `plans/usability/task4-app` | `src/main.rs` (`Ok(())`, no handler) compiles unchanged; the fixture check runs because `App` changes, with the requirement raised to the version under test as the plan's fixture-check rule says |
| Frozen: submissions and probes under `plans/usability/runs/` and U, such as U `r2-1-claude/work/src/lib.rs:111-143` and U `r2-5-head-claude-2/work/src/lib.rs:421-425` | not migrated |

## 7. Tests

- **New-behavior tests** each name the assertion that detects a wrong
  implementation and the compiling mutation that must make it fail. That
  the handlers receive the initializer's own value is guaranteed by the
  types (2.5: `Bound<S>` is built only from the `Ok` value, and no `Clone`
  or `Default` bound exists to make another), so the mutations target
  dispatch, persistence and drop order, the parts an implementation can get
  wrong while compiling.
- **Contract and compatibility tests** check rules P11 keeps or states.

### New-behavior tests

Headless, in `T/surfaces/window_close.rs` (the area binary for close
admission), and crate-internal without a window in `P/schedule.rs` and
`F/app.rs`'s tests:

1. **A close policy reads and changes the state the initializer returned:**
   the initializer returns `Notes { field, text, saved, refused: u32 }`; while
   `text` differs from `saved`, the policy increments the plain field
   `refused` and returns `KeepOpen`, otherwise `Close`. The test types a draft through `runtime_and_state_mut`.
   Named assertions: `admit_close` is `KeepOpen` and `state().refused ==
   1`; after `state().saved.set(..)`, `admit_close` is `Close` and
   `state().refused == 1`; after `state_mut().refused = 0` and a new draft,
   `KeepOpen` and `refused == 1`. Mutation: `HeadlessApp::admit_close`
   returns the default decision without calling the bound handler (the
   first assertion fails).
2. **The state is dropped after the runtime has shut down:** a state whose
   `Drop` records `signal.get()` into a shared `Rc<Cell<Option<bool>>>`
   (true for `Err(RuntimeError::Closed)`). Three cases: `app.shutdown()?`;
   `drop(app)`; and an initializer that returns the state and queues a
   focus-loss failure, so `headless()` fails at its first flush (the setup
   of `a_failed_start_and_the_shutdown_report_cleanup_failures`). Named
   assertion: each records `Some(true)`, and an `App` dropped without
   `headless` or `run` records `None`. A second observer captured by the
   `on_close` handler (2.2 rule 3) records the same in each case.
   Mutations: declare the state before the host in `HeadlessApp` (the `drop`
   case fails); drop the bound state before `finish_runtime` in `shutdown`
   (the first case fails) and in the failed `headless` (the third fails).
3. **`on_error` keeps its state across the runner's recovery path:** in
   `P/schedule.rs`'s `component_failures_back_off_..` test, the state is
   `Failures { calls: u32, seen: Rc<Cell<u32>> }` and the handler does
   `calls += 1; seen.set(calls)` before asserting that input was cancelled,
   as today. `ComponentRecovery::handle` runs four times on a `Running`
   built with `start()`. Named assertion: `seen == 4` and the fourth call
   returns the original error. Mutation: `Running::handle_error` applies
   the default report even when a handler is installed (`seen == 0`).
4. **A handler repairs runtime state, then exits with the original error**
   (`P/schedule.rs`'s `custom_error_policy_..`, restructured): before
   `start`, the test creates `repair = app.host.runtime.signal(0)` (crate
   access, as today's `app.runtime.signal(0)`, `:524`) and an
   `action: Rc<Cell<ErrorAction>>` set to `Retry`, and installs one handler
   that asserts the error is the expected one, sets `repair` to 1 and
   returns `action.get()`. It erases `app.start()?` into a `Running`. First
   `handle` with `BorrowConflict`: named assertions `repair == 1` and
   `recovery.error.is_some()` (retained for a close during backoff), as
   today. Then the test sets `action` to `Exit`, and a `handle` with
   `Exhausted` returns `Err(AppError::Runtime(RuntimeError::Exhausted))`, as
   today's second handler did (`:544-552`). No handler is installed after
   `start`. Mutation: `ComponentRecovery::handle` returns the retained
   error instead of the new one on `Exit` (the last assertion fails).
5. **The erased native owner drops the state after the runtime:** an
   internal, window-free test in `F/app.rs`'s tests builds
   `App::new("drop", |ui| Ok(Observed::new(ui.signal(0), seen.clone())))`
   with an `on_error` handler capturing a second observer, erases
   `app.start()?` into a `Running` whose runtime is still open, and drops
   it. Named assertion: both observers record `Some(true)` (their `Drop`
   read `RuntimeError::Closed`). Mutation: declare `lifecycle` before
   `host` in `Running` (both record `Some(false)`).

Native, run by `pert-ui-native-checks` (section 8):

6. **`on_error` with state in a window:** `P/smoke.rs`'s
   `window_resize_and_idle` (both its GPU and software tests) installs
   `on_error(|_, failures, _| { failures.calls += 1;
   failures.seen.set(failures.calls); ErrorAction::Retry })` with the state
   returned by its initializer. Named assertion: `seen == 4` where today's
   test asserts `failures.get() == 4` (`P/smoke.rs:1185-1189`). Mutation:
   as test 3.
7. **`on_window_error` with state in a window:** the multi-window smoke's
   `three_surfaces` (GPU and software) installs
   `on_window_error(|_, reported, error| reported.borrow_mut().push(error.clone()))`
   with an `Rc<RefCell<Vec<WindowError>>>` state its initializer returns.
   Its stale queued close already reaches `handle_window_error`
   (`P/multi_window_smoke.rs:190-209`, `P/windows.rs:244-249`). Named
   assertion, beside the existing receipt checks in phase 5: the reported
   list is exactly `[WindowError::Runtime(RuntimeError::Unmounted)]`.
   Mutation: `Running::handle_window_error` writes the default report
   instead of calling the handler (the list is empty).
8. **`on_close` with state in a window:** the close-request smoke
   (`P/multi_window_smoke.rs:439-475`) records the text the policy saw in
   its state (`Rc<RefCell<Option<String>>>` returned by its initializer)
   instead of a captured cell. Named assertion: unchanged, the policy saw
   the draft admission saved, now read through the state's clone.
   Mutation: `Running::handle_close` returns `Close` without the handler
   (the window closes and the recorded text is `None`).
9. **Probes allow their overlap:** each of the three probes fails at the
   assertion point 2.7 rule 2 names if an `UnexpectedOverlap` issue is in
   `validation_report()`. Mutations, one per probe: remove `Probe`'s
   `inspect` override; remove `Trace`'s (the only exemption in the
   damage-area probe); remove `FailingPaint`'s. Each fails natively with
   the issue named. If the smoke's mutation does not fail, because nothing
   disallowing is shown beside `persistent-paint-error` at stage 46, its
   override is unnecessary: it is dropped and the Progress entry says so.
10. **A clean native log:** the native run's saved output has no
   `[pert-ui validation ERROR UnexpectedOverlap]` line (500,503 in the
   2026-10-27 run, `TODO.md:145`). Recorded as a count in the Progress
   entry and `docs/verification.md`.

### Contract and compatibility tests

- **Initializers without state compile unchanged:** every packaged example,
  guide and rustdoc example with an `Ok(())` initializer, and the task 4
  fixture, through the workspace gate and the fixture check; a doctest on
  `App` writes `let app: App = App::new(..)` to show the default.
- **No handler runs without state:** `headless_refuses_a_blank_title_..`
  installs `on_close` and `on_error` handlers that panic on an
  `App::<()>::new(.., |_| Err(..))` and still gets the initialization
  error; `T/foundation/public_api.rs`'s off-main-thread test keeps
  `initialized == false`. Records 2.3 rule 2, which holds by construction;
  no mutation.
- **`HeadlessApp`'s `Debug` output is unchanged**
  (`T/surfaces/window_close.rs:450`).
- **Admission is unchanged:** the existing `window_close.rs` tests (focus
  release, restoration, `Defer`, hidden windows, errors) pass after their
  migration to state, and so does `F/app/close.rs`'s unit tests.
- **The multi-window and native smokes' existing assertions** pass with the
  migrated signatures, including the stale close's `Failed(Unmounted)`
  receipt and "the stale close did not enter error recovery".
- **Doctests:** `App`'s state example, `on_error`'s and `on_window_error`'s
  examples (no_run, compiled), `HeadlessApp`'s rewritten example and the
  three accessors' examples (run), the testing and custom-widgets chapters'
  rewritten blocks, `keep_alive`'s unchanged example; each imports exactly
  what it uses (R2-14).
- **P2's closure test** passes over the regenerated listings.

## 8. Scope, version and evidence

- **`api/` diff**, in every `api/pert-ui/*.txt` with `native` (`default`,
  `cpu`, `images`, `image-formats`, `serde`, `svg`,
  `no-default-desktop-cpu`): `pub struct pert_ui::App<S>` and
  `pert_ui::testing::HeadlessApp<S>`; their `impl<S: 'static>` blocks;
  `App::new`'s initializer returning `Result<S, AppError>`; the three
  handler signatures with `&mut S`; `App::headless` returning
  `HeadlessApp<S>`; added `HeadlessApp::state`, `state_mut`,
  `runtime_and_state_mut`; `Debug` and the auto-trait lines in their generic
  form (for example `impl<S> Unpin for HeadlessApp<S> where S: Unpin`),
  which the CHANGELOG describes as following from the type parameter. No
  runtime, core, CPU or wgpu listing changes.
- **Breaking:** yes (section 3, CHANGELOG).
- **Collision review:** no new names; `state`, `state_mut` and
  `runtime_and_state_mut` are methods.
- **CHANGELOG** for 0.3.86, citing this checkpoint:
  - `Breaking (R2-1):` `App` takes the application state as a type
    parameter, `App<S = ()>`: `App::new`'s initializer returns
    `Result<S, AppError>`, and `on_close`, `on_error` and `on_window_error`
    receive `&mut S` after the runtime. Replacement usage:
    `.on_close(|ui, state, window| ..)`, `.on_error(|ui, state, error| ..)`,
    `.on_window_error(|ui, state, error| ..)`; without state,
    `|ui, _, window|`. An application that shared what its initializer
    built through a slot returns it instead:
    `App::new("Notes", |ui| Ok(build(ui)?)).on_close(|_, notes, _| ..)`.
    An initializer that never returns `Ok` names the type:
    `App::<()>::new(..)`.
  - `Breaking (R2-1):` `HeadlessApp<S = ()>`, returned by `App::headless`;
    `HeadlessApp` written alone still names `HeadlessApp<()>`.
  - `Added (R2-1):` `HeadlessApp::state`, `state_mut` and
    `runtime_and_state_mut`.
  - `Changed (R2-1):` the state drops after the runtime has shut down, the
    order captured handler values already had; the packaged examples
    (`diagram_editor`, `inventory`, `log_explorer`, `live_mixer`) return
    their application object as the state instead of `Runtime::keep_alive`,
    and the diagram editor's slot is gone.
  - `Documentation (R2-1):` the testing chapter's close policy, the
    custom-widgets chapter's "Building the application", `HeadlessApp`'s
    example, `Runtime::keep_alive` (the initializer returns kept values as
    its state), `on_window_error`'s default report, `docs/manual.md` and
    `docs/contracts.md`; follow-up "Typed state handoff" removed.
  - `Tests:` the native probes that overlap on purpose report
    `OverlapPolicy::Allow` and assert a report without overlap; the native
    run logs no `UnexpectedOverlap` (TODO.md entry resolved; the
    summarizing proposal re-recorded); the native smokes check `on_error`,
    `on_close` and `on_window_error` with state.
- **Version:** 0.3.86 after P10's 0.3.85: the workspace version, the
  internal version requirements and `Cargo.lock`. If the order changes, the
  heading follows the actual version.
- **Gate,** the plan's dependency/feature/platform row, since the native
  runner changes: the workspace gate (formatting, Clippy, tests with
  section 7's headless tests, the five headless sessions, doctests,
  warning-free rustdoc, the diagnostics snapshots); release-profile tests;
  the MSRV (1.99.1, pinned); every supported feature configuration of
  AGENTS.md's row, `-p pert-ui --no-default-features --features
  desktop-cpu` included; the headless gallery; the API listing gate, its
  diff reviewed against the CHANGELOG; library packaging and the
  packaged-consumer check (`U/consumer-check.sh 0.3.86`); the fixture check;
  the mutations of section 7 recorded in the Progress entry (the native
  ones by a native run of the mutated check, or recorded as not run); and
  Codex's implementation review.
- **Native evidence (Linux):**
  - `cargo run --locked -p pert-ui-native-checks 2>&1 | tee
    <scratchpad>/native-p11.log` on an unlocked desktop session with a
    hardware GPU, the platform line it prints recorded with the result:
    every check passes, including tests 6 to 9; `grep -c "UnexpectedOverlap"`
    on the log is 0 (test 10), and other validation lines are listed (2.7
    rule 3).
  - **Hand check of closing a window with unsaved state:** `cargo run
    --locked -p pert-ui --example diagram_editor -- --window`; move a node,
    close the window from its title bar: the discard dialog opens (the
    policy read the state and deferred); declining keeps the window; close
    again and accept the discard: the application exits. Recorded with the
    platform.
  - Windows and macOS: recorded as missing (NATIVE-01 to NATIVE-05); the
    runner change is platform-independent code, but no execution evidence
    exists there.
- **Persistence, real time, backends:** no persisted schema, live-data
  endpoint or backend type is involved; no backend type enters the facade.

## Corrections to the plan

- **The slot is not in `G/layout.md`.** Its `on_close` paragraph
  (`G/layout.md:320-345`) is `FloatingWindow`'s, as is the runtime README's
  (`:1576-1591`); the guide's other slot is the custom-widgets chapter's
  "Building the application" (`G/custom-widgets.md:585-627`), which the
  follow-up itself names.
- **`T/surfaces/windows.rs` has no `App`;** its `on_close` (`:510`) is
  `FloatingWindow`'s. Not migrated.
- **The task 4 fixture's `main.rs` needs no change** (`Ok(())`, no
  handler). It stays a fixture-check consumer.
- **Inference is a second break.** Besides the handler signatures, an
  initializer that never returns `Ok` no longer compiles without naming the
  type (2.1 rule 4): `T/surfaces/window_close.rs:462`, `:486` and
  `T/foundation/public_api.rs:35`.
- **More platform consumers.** Six smokes never run the initializer and
  build the runner directly, and the scheduler tests use an `App` that never
  initialized (section 1); the plan names only `smoke.rs` and
  `multi_window_smoke.rs`. All are migrated through `start()` (2.5 rule 4).
- **`on_window_error` has native evidence available.** The multi-window
  smoke's stale queued close reaches the handler (section 7, test 7), so
  its evidence is planned, not recorded as missing.
- **The TODO's probe names.** The 500,500 pairs come from
  `verify_mixed_scene`, which runs inside `native_gpu_pixel_readback`; the
  realtime workload probe mounts one leaf root and overlaps nothing; the
  remaining sources by code are the damage-area probe and the smoke's
  `persistent-paint-error` (section 1). The native log confirms the set.
- **`HeadlessApp` bypasses the native handlers at `F/app/headless.rs:109-111`**
  (the plan cites `:105-111`), and the testing chapter's close policy is now
  at `G/testing.md:362-417` (P1 grew the chapter).

## Findings outside this part

- `crates/pert-ui-runtime/src/validation/development.rs:677-694` — the
  overlap check compares every pair of disallowing peers and writes one line
  per new pair, so N overlapping roots cost N(N-1)/2 comparisons and lines
  (500,500 for the probe's 1,001 roots). The removed TODO entry's last
  sentence proposed capping or summarizing; P11 re-records it as its own
  TODO.md entry (searching first): summarize per parent above a threshold,
  for example "N components overlap under <parent>; first pairs: ...", with
  a test. Not fixed here.
- TODO.md entries touching P11's files, not resolved here: `F/app.rs`
  `AppError`'s unchecked error list (2026-10-23); the diagram editor's
  `close_requested` and refused drafts (2026-10-27, P12); unused imports in
  examples (2026-10-28), whose rule P11's new examples follow.

## Not in this part

- **Returning the state from `run`** (`Result<S, AppError>`). No round 2
  run needed it, and after `run` the state's handles point to a closed
  runtime; the gallery's native checks read their results through slots
  their components fill, which state from the initializer would not replace.
- **A trait the state implements** (`trait Application { fn on_close(..) }`)
  instead of closures: a second style beside the builder handlers, which D9
  did not choose.
- **State reachable from components or commands.** Components keep reading
  bindings, and commands capture what they act on; the state is for the
  lifecycle callbacks and tests.
- **Summarized overlap reports** (findings above).

## Decisions

- **P11-D1: where the generic parameter stops.**
  - **Recommended: a thin generic startup that erases at once, and a
    non-generic runner** (2.5): `platform::run<S: 'static>` keeps today's
    startup order (event loop before the initializer), calls `start` and
    erases the result into a private `Box<dyn Lifecycle>` built from the
    initializer's value and the typed handlers; `Runner`, the window code,
    the scheduling and the smokes take only `Running`. Cost: the short
    startup function is compiled once per state type, and one dynamic call
    per handler invocation, which happens on failures and close requests
    only. No downcast.
  - **Alternative A: an erased, uninitialized startup object.** `App::run`
    converts the `App<S>` into a non-generic `Pending { host, start:
    Box<dyn FnOnce(&mut Runtime) -> Result<Box<dyn Lifecycle>, AppError>> }`
    that `platform::run` calls after creating the event loop. The whole
    platform module stays non-generic, at the cost of a second private
    type and a second boxed closure that only reorders the same steps.
  - **Rejected: initialize in `App::run` before calling a non-generic
    platform function.** It changes the documented order: a second call or
    a call off the main thread would build the host's content before
    failing (`F/app.rs:660-662`, `:686-689`).
  - **Alternative B: `Runner<S>`.** No box, but every runner method,
    `ComponentRecovery::handle`, the window code and each smoke helper
    carries `<S>`, and the platform code is compiled again for each state
    type an application or test uses.
  - **Alternative C: `Box<dyn Any>` state with a downcast in each handler
    call.** Smaller trait, but a downcast that cannot fail by construction
    still needs an `expect` or an error path in production code (AGENTS.md).
- **P11-D2: a joint accessor on `HeadlessApp`.**
  - **Recommended: add `runtime_and_state_mut`** beside D9's `state` and
    `state_mut`. A test's first step is usually to drive the runtime with a
    handle the state holds; with `state()` alone,
    `Driver::new(app.runtime_mut()).replace_text(&app.state().field, ..)` is
    a borrow error (E0502), and the test takes its handles out first, as
    the round 2 tests did from their slots before driving
    (U `r2-2-claude/work/tests/headless.rs:305`,
    U `r2-3-claude/work/tests/headless.rs:12`). The
    accessor mirrors what a handler receives, so a test can also call the
    application's own `fn(&mut Runtime, &mut State)` helpers. It retires
    cloning handles out of the state as the taught first step.
  - **Alternative: `state` and `state_mut` only;** tests clone the handles
    they drive with (`let field = app.state().field.clone();`). One method
    fewer; every taught test starts with clones, and state that is not
    `Clone` (a receipt, a controller) needs a field-by-field clone or a
    second borrow scope.
- **P11-D3: `keep_alive` in the initializer.**
  - **Recommended: the state replaces it there** (2.6): `keep_alive`'s
    rustdoc and the manual teach returning kept values in the state, and
    the four packaged examples that keep their application object migrate
    (four one-line changes). One taught way in an `App`; `keep_alive` stays
    for code that has only a runtime.
  - **Alternative: keep both,** documenting state for what callbacks read
    and `keep_alive` for what must only live. No example changes, but two
    taught ways for an initializer's long-lived values, which AGENTS.md's
    "names the way it retires" rule asks to avoid without a reason.

## Review record

- **Revision 1 (2026-10-28):** written from the code at d4e1386 (library
  code of the changed files as at 26e6b5a), with the inference rule checked
  on a reduced model with rustc 1.99.1.
- **Revision 1, Codex review (2026-10-28, against 7bff14d):** needs
  revision, with one blocker, three should-fix points and a nit; the public
  signatures, opaque ownership, prelude treatment, unchanged schemas and the
  native evidence scope were found to fit. Record
  `~/pert-ui-usability/codex-prompts/codex-design-P11.out`. Each point was
  checked against HEAD before revision 2 applied it; none was rejected.
  1. **Blocker, startup erasure:** a non-generic `platform::run(app: App)`
     accepts only `App<()>`, and initializing in `App::run` before it would
     run the initializer before the event loop exists (`P/mod.rs:63-78`).
     Revision 2 makes `platform::run<S: 'static>` the one generic platform
     function, in today's order, erasing at once (2.5 rule 1); P11-D1 is
     revised with the erased startup object as the alternative and early
     initialization rejected.
  2. **Damage-probe coverage:** with both `Grid` and `Trace` allowed,
     removing one override leaves the pair excluded
     (`R/validation/development.rs:678`), and the first flush (`:376`) sees
     only `grid`. Revision 2 exempts only `Trace`, places each probe's
     assertion after every overlapping root is flushed (`:247`, `:417`, the
     smoke's stage 46 with its own flush), and names one mutation per probe.
  3. **The erased owner's drop order:** headless tests could not catch a
     reversed `Running`. New test 5 drops an erased `Running` with an open
     runtime, with a state observer and a handler-captured observer;
     mutation: reverse `Running`'s fields. Test 2 also observes a captured
     value (2.2 rule 3).
  4. **The scheduler repair test** replaced its handler after construction
     (`P/schedule.rs:544`), which a `Running` cannot. New test 4 installs
     one handler before `start` with an `Rc<Cell<ErrorAction>>` selector and
     keeps the repair, retained-error and original-error-on-exit assertions.
  5. **Nits:** the diff from 26e6b5a is not empty (16 rustdoc lines in
     `R/validation.rs`); the evidence paragraph says so.
     `E/dynamic_documents.rs` has no `App` (`:9` builds a `Runtime`); the
     manifest gives that reason.
- **Revision 2, Codex recheck (2026-10-28):** every revision 1 point
  resolved; ready for approval with one nit (the overlap exemption's
  timing rationale attributed log writes to the measured loop). The author
  corrected the sentence as Codex proposed. Approved by the author under
  the maintainer's delegation, with P11-D1 (as revised), P11-D2 and P11-D3
  as recommended.
