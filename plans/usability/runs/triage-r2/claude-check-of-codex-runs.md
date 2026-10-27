# Claude check of Codex's review changes to Codex-produced runs (round 2)

Fresh Claude agent, 2026-10-27, read-only except this file. Checked against
the runs (`~/pert-ui-usability/<run>/`), the kits' vendored sources
(`kit-aca8df3/vendor/pert-ui*-0.3.75`, `kit-359122e-r2/vendor/pert-ui*-0.3.52`),
round 1's definitions (`plans/developer-usability.md` "Round 1 findings",
`plans/usability/runs/triage/group-D.md`, `group-E.md`) and the matrix rule
(recurred needs evidence of the problem or its workaround; reading a page
without visible effect is not recurrence; passed needs evidence the run
exercised the area). Judged from the runs, not from whether the triage files
have already been revised. Paths below: `R/` = `pert-ui-runtime-0.3.75/src`,
`F/` = `pert-ui-0.3.75/src` in `kit-aca8df3/vendor`.

## Verdicts on the listed changes

1. **r2-1-codex F3 passed -> not reached: agree.** The run writes none of
   F3's names (`Modifiers`, `KeyChord`, `CommandKey`, `CommandScope`,
   `ListDrop`, `DropPosition`); its only imports are `prelude::*`
   (`r2-1-codex/work/src/main.rs:1`) and `testing::*` (`:292`), and its
   inspection helper names only `InspectionQuery`, `Selector`, `Detail`
   (`:309-320`). Its diagnostics.md has no import error (the failures are
   `NativeWindowId::root` and `App::size(w, h)` guesses). "Compiled with the
   globs" does not show the area was exercised; G3 applies the narrow test to
   r2-5-head-codex-1 (group-3.md:184-185), so not reached is also the
   consistent value.

2. **r2-3-codex F33 recurred -> passed: agree.** `size_limits` returns
   `SizeLimits::exact(Size::new(260.0, 260.0).unwrap())`
   (`r2-3-codex/work/src/picker.rs:84-86`). The deciding fact is round 1's
   own scope: group-D.md:46 recorded r1-3-codex's `Size::new(..).unwrap()`
   and `SizeLimits::new(..).unwrap()` and made F33 only the missing
   infallible exact `SizeLimits`, pointing to `Size::from([u16; 2])` as the
   non-panicking `Size` route that already existed. The `SizeLimits` unwrap
   is gone (`R/layout.rs:200-217`); the remaining `Size` unwrap is the part
   round 1 already classed as application code. The new doc point stands:
   `exact`'s own example builds the `Size` with `Size::new(260.0, 260.0)?`
   (`R/layout.rs:207-215`), which cannot be copied into `size_limits`, and
   `Size::from([u16; 2])` (`R/layout.rs:129-137`) is not mentioned there.
   Keep it as the author-found documentation row (G2 N11).

3. **F11 not reached -> passed in r2-1-codex, r2-2-codex, r2-5-head-codex-1,
   r2-5-head-codex-2: partly agree, disagree with the resolution.**
   - Codex's facts are right. Each run converts a `RuntimeError` into
     `AppError` with `?` in the `App::new` initializer:
     `r2-1-codex/work/src/main.rs:264` (`build` returns
     `Result<Built, RuntimeError>`, `:58`), `r2-2-codex/work/src/main.rs:299`,
     `r2-5-head-codex-1/work/src/main.rs:4`,
     `r2-5-head-codex-2/work/src/main.rs:326`. The initializer type is
     `Result<(), AppError>` (`F/app.rs:358-360`). So G3's "never convert a
     library error into `AppError`" (group-3.md:196-198) is false, and the
     triage applied two criteria (G2 counted this evidence as a pass).
   - But a successful supported conversion cannot show anything about F11.
     F11 is a diagnostic on a *failed* conversion (rustc naming `DrawError`)
     plus `paint()`'s `DrawError` not converting into `RuntimeError`. The
     `RuntimeError` -> `AppError` path worked identically at 0.3.52 (same
     blanket `impl<E> From<E> for AppError where DrawError: From<E>`,
     `kit-359122e-r2/vendor/pert-ui-0.3.52/src/app.rs:167-170`;
     `DrawError::Runtime(#[from] RuntimeError)`,
     `pert-ui-runtime-0.3.52/src/drawing.rs:47`). Proof that the criterion
     does not discriminate: G4 marks F11 passed in all four task 5 runs on
     the *unfixed* 0.3.52 library (group-4.md:144-151). No round 2 run
     converted an unsupported error into `AppError` or `paint()`'s
     `DrawError` into `RuntimeError` (every `paint()?` goes into
     `Box<dyn Error>`: `r2-5-head-codex-2/work/src/main.rs:491`,
     `r2-5-r1-codex-1/work/src/main.rs:577`,
     `r2-5-r1-codex-2/work/src/main.rs:588`, and H-C2
     `tests/headless.rs:108`); no diagnostics.md mentions `AppError` or
     `DrawError`. Codex itself notes the rejection diagnostic was not tested.
   - **Recommended:** make F11 consistent the other way: `not reached` in
     every run whose only evidence is a supported conversion (T1-C/X,
     T2-C/X, T3-C/X, H-C1/C2/X1/X2, B-C1/C2/X1/X2). Widening to `passed`
     credits a fix no run touched, in Codex's runs among others.

4. **r2-4-codex F19 not reached -> passed: disagree.** The registration
   without `let _ =` is the author's fixture, and the author edited it for
   0.3.75: against round 1's fixture the only changes are dropping
   `use pert_ui::{CommandScope, KeyChord};` and `let _ = ` before
   `ui.register_command(` (`diff r1-4-codex/work/bookmarks/src/lib.rs
   r2-4-codex/work/bookmarks/src/lib.rs`; `fixture-0.3.75/src/lib.rs` is
   identical to the run's). A warning-free build of code the author adapted
   to the fix is circular: it shows the fixture preparation, not a tester
   writing a command registration. Driving Ctrl+L does not touch `CommandId`.
   Not reached. (r2-4-claude, same reason.)

5. **r2-4-codex F28 not reached -> passed: disagree.** F28 is a guide gap: a
   developer writing `Signal::update` on a custom type does not learn about
   the `PartialEq` bound. The tester wrote neither the type nor the `update`:
   `Bookmark` with `#[derive(Clone, Debug, PartialEq)]` and the `update` in
   Remove are unchanged fixture code (`r2-4-codex/work/bookmarks/src/lib.rs:11`,
   `:113`), identical in round 1's fixture, where task 4 could not have
   surfaced F28 either. Executing inherited code is not exercising a
   documentation gap. Not reached. (By contrast, r2-5-head-codex-1's F28 pass
   is sound: its own `Job` derives `PartialEq`, `src/lib.rs:28`, and it calls
   `update` at `:253`, `:292`.)

6. **r2-5-head-codex-1/-2 F29 passed -> not reached: agree.** The empty
   labels sit in fixed-height table rows: `Label::new("")` at
   `r2-5-head-codex-1/work/src/lib.rs:93` in `Table::new(&table, 64.0)`
   (`:309`), and at `r2-5-head-codex-2/work/src/main.rs:82` in
   `Table::new(&table, 42.0)` (`:311`). A fixed row height hides whether the
   label takes height; G4 uses exactly this reason for B-X2
   (group-4.md:194-196).

7. **r2-5-head-codex-2 missed `TaskReceipt` warning: agree.**
   `r2-5-head-codex-2/diagnostics.md:15-33` has `unused TaskReceipt that
   must be used` on `cx.defer(…)` with the receipt's must_use note, and the
   final code binds `let _receipt = cx.defer(…)` (`work/src/main.rs:215`).
   It belongs to G1 N4's run coverage. It is the only task 5 run with that
   warning (`grep -l TaskReceipt r2-*/diagnostics.md`: r2-1-claude,
   r2-1-codex, r2-5-head-codex-2).

8. **r2-5-r1-codex-1/-2 F3 "by use" kept as recurred: agree, labelled as
   inferred.** At 0.3.52 `Modifiers` is root-only
   (`pert-ui-runtime-0.3.52/src/lib.rs:268`). B-X1 globs the crate root next
   to the prelude, `use pert_ui::{prelude::*, *};`
   (`r2-5-r1-codex-1/work/src/main.rs:1`), and uses `Modifiers` at `:499-505`;
   B-X2 imports it by name from the root (`r2-5-r1-codex-2/work/src/main.rs:433`)
   and uses it at `:537-543`. Neither logged an import failure (B-X1's
   `Modifiers::ALT` E0599 is a constant guess, not an import). Round 1
   counted F3 "6 by use"; keeping the same standard is required for the
   comparison, and the head runs' F3 passes are likewise by use
   (group-3.md:181-184). The cells must be marked "inferred from code" so
   they are not read as reported friction.

9. **X2 runs' plain-Label notification: agree with Codex
   (acceptance-policy question, not a tester error).** The frozen task says
   only "the failure is shown in its row and in a notification"
   (`plans/usability/reserved/5-job-monitor.md:15-16`, acceptance 3 at `:33`);
   it names no component. Both acceptance files themselves call it a
   judgement (`r2-5-r1-codex-2/acceptance.md` triage notes;
   `r2-5-head-codex-2/acceptance.md:49-53`). The two runs differ in evidence:
   - Head (H-X2): a deliberate choice after reading `Toast` and `ToastHost`
     (`work/FRICTION.md:14-17`). Its reason is partly right: a toast's fixed
     five-second life schedules a wake at expiry (G3 P5,
     `R/notifications.rs:129-133`, `:879`), which can fall after the last job
     ends, against the task's no-repaint-while-idle rule; only "animated" is
     wrong. `Banner`, the correct answer, is not pointed to from `Toast` or
     `ToastHost` (NF-7). Calling this a tester error blames the tester for the
     documentation gap the same run evidences.
   - Baseline (B-X2): no friction entry and no sign it read the notifications
     module; its README calls the label "a persistent notification banner"
     (`work/README.md:13`). Weaker evidence of a doc gap, but still not a
     misreading of an explicit requirement.
   - The author should fix one rule (any visible failure text outside the row,
     or a notification surface) and apply it to both commits; under either
     rule the two X2 runs get the same item-3 result, so the head/baseline
     comparison is unaffected.

## Other Codex-review changes touching Codex-produced runs

- **T3-X F12 pass qualified (removal-error not reported): agree.** The run
  follows the documented route (`r2-3-codex/work/src/picker.rs:148-158`:
  record, `notifier().request()`, return from `deliver_notifications`), and
  `Component::cancel_input` says a removed component's failure is not
  reported (`R/runtime.rs:2533-2538`). The pass stands with the stated limit.
- **G2-8 / N4 premise (T1-X, T2-X): agree.** `TaskStatus::Cancelled` still
  covers shutdown and a panicking callback (`R/runtime/tasks.rs:12-16`,
  `:141-150`); only owner disappearance stopped cancelling `defer`/`spawn`
  (`:123-124`). N4 stays a design reconsideration, not a verified obsolete
  attribute. No cell changes.
- **"No projection of a struct signal" narrowed (T1-X): agree.** `map` is a
  read-only projection; T1-X's chained `combine` calls
  (`r2-1-codex/work/src/main.rs:68-84`) show the cost of the missing n-ary or
  writable-field form.
- **H-X1 added to NF-5 as acceptance-derived: agree, as weak evidence.** The
  tester's idle check used `next_deadline` and `advance` but not paint
  (`r2-5-head-codex-1/acceptance.md:40-45`); that fits a missing recipe but is
  not a reported friction. Note that acceptance.md's triage note (":51")
  says `needs_paint` is reachable only through `pert_ui::host`; that is wrong,
  `testing::*` re-exports `HostIntegration` (G3 P6), so the evidence is the
  missing pointer only.
- **F20 "recurrence by exposure" for B-X1/B-X2: disagree with Codex, agree
  with G4's rejection.** The matrix rule says reading an affected page without
  visible effect is not recurrence; neither submission imports
  `pert_ui_runtime` or `pert_ui_core` (group-4.md:166-174). (This proposal was
  harsher on Codex's runs, not lenient.)
- **N7 coverage "T2-X acceptance": correct the attribution.** The bounds sort
  in `r2-2-codex/acceptance.md:37`, `:104-110` is the author's check. The
  tester itself read rows in display order with `Runtime::children` of the
  list and no friction (`r2-2-codex/work/src/main.rs:376-391`). T2-X is not an
  occurrence of N7; if anything it is evidence the route was found.
- **F9 vs N1 for T4-X (retained as passed): see leniency below.**
- **F16 recurred in all four Codex task 5 runs (retained): agree.** H-X2 by
  code: per-job `Signal<ProgressStatus>` beside the republished rows
  (`r2-5-head-codex-2/work/src/main.rs:40`, `:61`, `:191`, `:246`); B-X2 a
  `sibling` per cell build (`r2-5-r1-codex-2/work/src/main.rs:246`).

## Leniency toward Codex's own runs

- **F11 passes (point 3)** and **r2-4-codex F19/F28 passes (points 4-5)**
  credit fixes the Codex runs never exercised. These are the clearest cases
  where Codex's review makes its own runs look better than the evidence
  supports (it applies the same to the Claude runs, so it is not one-sided,
  but the direction is lenient).
- **r2-4-codex F9 passed (Codex retained; G2 flagged it as a judgement).**
  F9 is "Each puts its rows under an implicit `items` segment ...;
  documented, but the only example uses `within`". T4-X's friction entry
  (`r2-4-codex/work/FRICTION.md:32-35`) and its closing wish (`:71`) are about
  exactly that: it had to rely on the implicit `items` container, asserting
  its key in the test (`tests/ui.rs:75-86`), because the guide's only ordered
  example uses `within`, which would mean changing the application. The
  address part of F9 is fixed (paths skip the structural container,
  `tests/ui.rs:89-97`), but the run shows the implicit container still costs
  a reader time. Under the matrix rule (workaround visible in friction and
  code) `recurred` is at least as defensible as `passed`; the author should
  decide F9's scope explicitly rather than default to the pass.
- Checked and found sound: H-X1 F2/F3/F28, H-X2 F2/F3/F23, T1-X F4/F5/F6,
  T3-X F3 recurred, T3-X F12/F18/F24 passed, B-X1 F10 recurred
  (`r2-5-r1-codex-1/diagnostics.md` E0308).
- Minor, not a correction: H-X1 pressed Alt for keyboard sorting as a key,
  `key_input(KeyCode::AltLeft, …)` (`r2-5-head-codex-1/work/tests/headless.rs:120-125`),
  neither the chapter's `Driver::chord` nor `native_modifiers`. It worked
  without friction, so F34 `passed` holds, but it shows a third route in use.
