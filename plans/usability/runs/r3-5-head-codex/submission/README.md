# Background job monitor

A pert-ui application with simulated worker jobs, sortable progress/state/elapsed columns, prompt cancellation, failure notifications and live summary counts. Uses default pert-ui features plus `cpu`.

- `cargo run --offline` opens the native application (requires a display).
- `cargo test --offline` runs the headless acceptance check and writes `jobs.png` in `/work`.

Workers send owned updates through a pert-ui mailbox. UI state stays on the UI thread. Cancellation interrupts the worker's 100 ms wait. Normal jobs choose 50–500 units and process 5–20 units per tick; each fifth job fails at a random unit. Failure notifications persist without expiry timers, and completed rows retain their final elapsed time.

The test replaces random plans with fixed ones but uses real background threads and cancellation channels. It checks UI semantics, header sorting in both directions, cancellation latency, development validation, and idle scheduling, then captures the UI with pert-ui's CPU renderer.

See [FRICTION.md](FRICTION.md) for the documentation experience and [jobs.png](jobs.png) for the capture.
