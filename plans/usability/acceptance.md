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
   the author adds a check of their own in the acceptance directory and
   records it in the acceptance result. The tester's partial check alone
   does not meet the item, but the author's check can: the item is met when
   the implementation is correct and the author's check passes, and the
   result names the author's check as the evidence.
4. Record per item: met, partly met (what is missing) or not met, and for
   each a pointer to the code and the check, in the run's `acceptance.md`.
   Write `acceptance=met|partly|not-met` and, when every item is met,
   `accepted=<the session's end time>` to the run's `acceptance.env` for
   `summarize.py`. A run whose submission is not fully met has no accepted
   completion time; its partial result is reported instead.

## Task-specific checks

Each task's acceptance items are written here before its first run and
never shown to testers. The items of the tasks used in rounds 1 to 3 are in
Git history (this file at commit 8254019); those tasks are known now, so a
new round writes new tasks.
