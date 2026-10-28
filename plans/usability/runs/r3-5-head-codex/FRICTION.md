# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### 17:21 — compile — annoyance

- **Tried:** Returned `mount(ui, vec![])` directly from the `App::new` initializer; mount returns `Result<Monitor, RuntimeError>`.
- **Expected:** The initializer to accept the runtime error type used by the mounting API.
- **Resolved by:** Compiler help: `Ok(mount(ui, vec![])? )` converts the error. The diagnostic was useful and immediately actionable.
- **Read:** `/kit/doc/pert_ui/struct.App.html`, guide testing chapter.
- **Diagnostic:** `error[E0308]: mismatched types` at src/lib.rs:143; `expected Result<_, AppError>, found Result<Monitor, RuntimeError>`.

## Closing

- **Result:** All five acceptance items met. `cargo test --offline` passes the `monitor_acceptance_and_cpu_capture` integration test. It starts five jobs through `Driver::click`, observes progress and completion in row semantics and the summary, cancels a running worker within 250 ms and checks its processed count stays fixed, sees a failure in its row and persistent notification, clicks all four sortable headers in both directions and checks row order, and saves `/work/jobs.png` with all four states. `Driver::quiet_for` verifies idle behavior before jobs and after all workers terminate; development validation reports no issues. The application is runnable with `cargo run --offline` on a machine with a display.
- **Concepts I had to learn:** Retained runtime and keyed elements; signals and mapped bindings; UI-thread mailbox ownership and `UiSender` delivery; application-owned table sources and sorting; table row/cell/content addresses; Driver semantics, clicks and background waits; capture support, queued captures and CPU serving; quiet reports.
- **What worked well:** The crate landing page maps tasks to types and guide chapters. The testing chapter includes an unusually close example: job progress cells with a bar and percent, sorting headers through pointer input, background waits, idle checks and PNG capture. The mailbox reference explains capacity, thread safety and waking clearly. No library source reading was necessary. The single compiler issue had an accurate fix suggestion.
- **The three changes I want most:**
  1. Add a complete mailbox recipe to the state guide: retain the mailbox, clone the sender into a worker, handle Full/Closed, and apply progress on the UI thread. The guide currently describes it in prose while the adjacent one-shot spawn has code.
  2. Include a named mounting helper returning `RuntimeError` in the App example to demonstrate error conversion at the initializer boundary.
  3. Add cross-links from the table testing example to the quiet-idle requirement for progress displays and notification lifetimes. A persistent banner worked here, but choosing it required connecting separate guide sections.
