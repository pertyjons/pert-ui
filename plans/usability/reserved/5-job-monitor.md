# Task: background job monitor

Build a monitor for simulated background jobs.

## Features

- A **Start job** button starts a simulated job on a background thread. A
  job has a name ("Job 1", "Job 2", ...), a total size in units (random from
  50 to 500) and processes 5 to 20 units per tick at 10 ticks per second.
- A table of jobs with columns name, progress (a bar and a percentage),
  state (running, done, cancelled, failed) and elapsed time. The table can
  be sorted by each column.
- Each running job has a Cancel action. A cancelled job stops promptly and
  shows its state.
- One job in five fails at a random point; the failure is shown in its row
  and in a notification.
- A summary line: running, done, cancelled and failed counts.
- The UI must not repaint while no job is running.

## Constraints

- `pert-ui` with the `cpu` feature in addition to the defaults.
- The background threads must not touch UI objects directly.
- Verification without a display: tests or a headless mode that drive the
  application through the library's own facilities. A test may replace the
  random source and the clock with deterministic ones.

## Acceptance

1. A headless check starts jobs and sees progress, completion and the
   summary line update.
2. A headless check cancels a running job and sees it stop.
3. A headless check sees a failure shown in its row and as a notification.
4. Sorting by each column orders the rows correctly.
5. A CPU capture (PNG) of the table with jobs in several states.
