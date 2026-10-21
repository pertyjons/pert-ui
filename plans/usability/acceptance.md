# Author acceptance checks

Written before any run and applied by the author, never shown to testers.
A tester's own headless check exercises what the tester implemented; these
checks decide whether the task's acceptance items are met. Unfinished
submissions are evaluated the same way, item by item.

For every run:

1. Initialize a fresh acceptance directory with `run.sh exec <kit>
   <acceptance-dir> true`, copy the submitted `work/` (without `target/` and the
   CLI's `.claude/`)
   into its `work/`, and build and test it there with `run.sh exec` and the
   task's features. Record whether it builds and whether the tester's own
   checks pass.
2. Read the source against each acceptance item. An item is met only when
   the behavior is implemented and a headless check drives it through the
   library's facilities (input, inspection, captures), not by calling the
   application's own model functions.
3. Where the tester's check does not cover an item's required behavior,
   the author adds a check of their own in the acceptance directory,
   records it in the acceptance result, and does not count the tester's
   partial check as meeting the item.
4. Record per item: met, partly met (what is missing) or not met, and for
   each a pointer to the code and the check, in the run's `acceptance.md`.
   Write `acceptance=met|partly|not-met` and, when every item is met,
   `accepted=<the session's end time>` to the run's `acceptance.env` for
   `summarize.py`. A run whose submission is not fully met has no accepted
   completion time; its partial result is reported instead.

## Task-specific checks

**1. Settings editor.** Out-of-range and malformed input refused through the
field itself, with the last valid value still applied; Apply and Revert
enabled state read from inspection, not from the tester's own flags; the save
runs off the UI thread (worker thread or the library's background facility)
and a write to an unwritable path is reported; close refusal goes through the
library's close request, not a custom button only; loading at startup.

**2. Task list.** The list is driven by the tester's `Vec` (removing an item
from the vector removes the row); in-place rename commits on Enter and is
discarded on Escape; the filter does not change the vector; Ctrl+N, Delete,
Alt+Up and Alt+Down act as specified through simulated key input; the status
line follows checking and removing.

**3. Custom control.** The control is a reusable type implementing the
library's component contract; a drag beyond the area clamps; cancellation
restores the pre-press value; Shift changes the arrow step; focus indication
and colors come from the theme (the two captures differ in colors); the value
appears in an inspection snapshot; the numeric fields stay in sync both ways.

**4. Testing an existing app.** One test per README behavior, each driving
input or reading inspection; all pass against the unchanged app; a CPU
capture PNG is written; the recorded deliberate break made a test fail; any
change to the app is listed and does not change behavior.

**5. Background job monitor (reserved).** Progress reaches the UI through the
library's cross-thread facility, not shared UI objects; cancellation stops
the worker; a failure shows in the row and as a notification; sorting by
every column; no repaint while idle, as far as the library's profiling or
inspection shows it.
