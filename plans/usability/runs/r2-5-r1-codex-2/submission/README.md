# Background job monitor

Uses pert-ui 0.3.52 with default features plus `cpu`.

```sh
cargo run --offline                 # desktop window; requires a display
cargo run --offline -- --headless    # acceptance check and jobs.png
cargo test --offline                # same acceptance check as a test
```

Workers send owned reports through a retained UI mailbox. Its UI-thread handler
updates the table and signals. Cancellation uses a separate channel. Every fifth
job fails at a random unit threshold; a persistent notification banner reports it.
Column headers toggle sorting; Alt+Up/Down sorts the active column. Name and state
sort lexically; progress and elapsed time sort numerically.

The headless check uses Runtime keyboard input, real background threads, mailbox
delivery, table sorting callbacks, semantic inspection and CPU capture. It uses
deterministic ticks and elapsed times. It checks fixed expected row sequences for
all four columns in both directions, confirms cancelled workers exit within
250 ms, and verifies idle flushes update no components and request neither paint
nor deadlines. `jobs.png` captures all four states before remaining jobs stop.

Native mode uses 100 ms worker ticks, random sizes of 50–500 units, and random
increments of 5–20 units. Determinate progress schedules no animation. Cancel
immediately marks the row and ignores queued reports for it. Shutdown closes the
mailbox, stops workers and joins them. Full-mailbox delivery retries reports,
checking cancellation and shutdown during backpressure.

No native window was exercised because this environment has no display.
See FRICTION.md for first-use documentation observations. The HTML-to-text helper
in tools/read_doc.py reads rendered documentation without consulting source.
