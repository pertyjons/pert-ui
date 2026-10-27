# Background job monitor

A pert-ui 0.3.75 application with default features plus `cpu`.

- `cargo run --offline` opens the native application (requires a display).
- `cargo run --offline -- --headless` runs the acceptance checks and writes `jobs.png`.
- `cargo test --offline` runs the same check as a test.

Start job creates a worker with a random total of 50–500 units. Every 100 ms it processes 5–20 units; every fifth job fails at a random point. Cancel wakes the worker immediately. Click any Name, Progress, State or Elapsed header to sort; click again to reverse. Failure notifications remain visible, with no expiry timer. All worker-to-UI updates travel through a bounded pert-ui mailbox. Workers hold only data and channel endpoints.

The headless scenario uses fixed job sizes and increments and a shorter worker interval. It starts five jobs through button keyboard input, inspects progress and summary semantics, clicks Cancel, verifies failure in a row and notification, and captures all four states. It then verifies mounted row order in both directions through the table's keyboard sort actions, and checks that an idle runtime has no deadline or repaint request. Cancellation has a one-second test timeout and is also checked for no later progress. No display or GPU is used.

See `FRICTION.md` for the contemporaneous library evaluation log.
