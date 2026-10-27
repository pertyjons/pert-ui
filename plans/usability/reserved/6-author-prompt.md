# Prompt given to the author of task 6

Kept with the task as the round 3 proposal requires. The author was a fresh
Claude agent session (2026-10-28) whose inputs were copies of the files
listed below, placed in an isolated directory.

---

You are writing one task for a blind usability test of a Rust GUI library,
`pert-ui`. Developers (AI coding agents) who have never seen the library get
one task, the library's documentation and a sandbox, and build the
application. Their friction, time and results are measured. Your task will be
held in reserve and used later as a holdout: it must test ordinary
application development with the library, not any particular weak spot.

Read only these files in `INPUTS` (do not open any other file on the
machine, in particular nothing in the pert-ui repository, its `plans/`
directory or any other test material; your task must not be shaped by
anything but what is listed here):

- `README.md`: the library's registry page.
- `crate-root.md`: the library's crate root documentation page.
- `1-settings-editor.md`, `2-task-list.md`, `3-custom-control.md`,
  `4-testing-an-app.md`, `5-job-monitor.md`: the existing tasks, as the
  format and size to match.
- `tester-brief.md`: what every tester is told.
- `acceptance-format.md`: the author's acceptance procedure and the
  task-specific checks of tasks 1 to 5, as the format of the checks you
  write.

Write task 6:

- **An application of the size of tasks 1 to 5**, in a domain different
  from all five, that an ordinary desktop application developer would
  plausibly build. Use only capabilities the README and crate root page say
  exist; do not require anything they do not describe.
- **The same sections as tasks 1 to 5:** a title, a short description,
  `## Features`, `## Constraints` (including the library features to enable
  and the requirement of verification without a display, through the
  library's own facilities) and `## Acceptance` with numbered items, each of
  which a headless check can decide.
- **Its author acceptance check**, in the form of the entries under
  "Task-specific checks" in `acceptance-format.md`: one paragraph, starting
  `**6. <title> (reserved).**`, saying what decides each acceptance item
  beyond what a tester's own check shows.

Write exactly two files in `OUTPUT`: `6-<short-kebab-name>.md` (the task) and
`6-acceptance-entry.md` (the paragraph). Do not write anywhere else. Reply
with the two file names and two sentences on why the task is a fair,
ordinary-use holdout.
