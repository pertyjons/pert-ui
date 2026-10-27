# Background job monitor

A Rust application built with pert-ui 0.3.75 (default features plus `cpu`).

- `cargo run --offline` opens the desktop monitor when a display is available.
- `cargo test --offline` runs the headless acceptance session and writes
  `/work/job-monitor.png` using pert-ui's CPU capture API.

Start job creates a worker with 50–500 units and a randomly chosen throughput
of 5–20 units every 100 ms. Every fifth job fails at a randomly chosen unit.
Cancel wakes the worker immediately through a channel. Workers send owned
snapshots to a bounded pert-ui mailbox; all UI state stays on the UI thread.
Failure notifications persist at the bottom of the window. Click a column
header to sort, or use Alt+Up/Down on an active table cell.

The deterministic headless session uses actual background threads with a test
clock and fixed plans. It clicks Start and Cancel through pert-ui's Driver,
checks rendered semantics, exercises both sorting directions for each data
column, checks row geometry order, captures running/done/cancelled/failed rows,
and verifies no deadlines or component updates remain when all jobs stop.

See FRICTION.md for the documentation and development experience.
