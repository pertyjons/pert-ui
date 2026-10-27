# Background job monitor

Uses pert-ui 0.3.52 with default features and `cpu`.

- `cargo run --offline` opens the desktop monitor (requires a display).
- `cargo run --offline -- --headless` exercises the real widgets and background threads without a display, checks all acceptance criteria, and writes `jobs.png`.
- `cargo test --offline` runs the same acceptance session.

Start jobs with **Start job**, cancel from their rows, and click any of the four data headers to sort ascending/descending. State order is alphabetical; progress sorts by fraction; elapsed sorts by duration; name sorts lexically. Every fifth job fails at a random processing point. Failure notifications stay in a banner; a later failure replaces the message. Finished rows retain their elapsed time and processed units.

Workers receive commands over standard channels and deliver owned snapshots through a pert-ui mailbox. Real jobs tick every 100 ms; the headless session substitutes deterministic totals, processing amounts and clock ticks while keeping the same worker/message/UI paths. No UI handles enter a worker. The headless capture includes running, done, cancelled and failed jobs. The final check stops all workers and verifies both `next_deadline() == None` and no paint damage as runtime time advances.

See `FRICTION.md` for the first-use record.
