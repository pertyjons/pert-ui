**One blocker left: the verifier session loads your personal global Claude instructions. Everything else you listed checks out.**

**Blocker:** `accept.py:57-62` starts `claude -p` with your normal `HOME`. Testers get a home directory of their own inside the run (`run.sh` `init_run`); the verifier doesn't. So it loads `~/.claude/CLAUDE.md`, whose instruction is "uppdatera version" (update the version), plus your user-level settings, hooks and skills. It also runs with `--dangerously-skip-permissions`. After adding tests to the copy, it may well bump the version in the copy's `Cargo.toml`. That breaks the prompt's rule against changing application code, and for task 4 it would add a difference against the fixture that doesn't come from the submission. It also means the context isn't really fresh, which the plan's "fresh Claude acceptance contexts" asks for.

Fix it in one of two ways:
- Run the verifier with its own config directory that holds only the credentials copy, the way `run.sh` does for testers.
- At minimum, add a line to the prompt saying to ignore user-global instructions and never change version fields.

**Checked and resolved:**
- **Kit identity:** commit, version and toolchain are compared against the run's `conditions.env`. The verifier is also told to confirm the match itself.
- **Frozen procedure:** `acceptance.md` is checked against its hash in `prelaunch.json`.
- **Fresh acceptance directory:** the controller refuses an existing one and initialises it with `run.sh exec … true`, as `acceptance.md` step 1 says.
- **Integrity:** the original source manifest is taken before the session and compared after, with escaping symlinks refused. The exclusions match the copy instructions, and the extra `.git`/`.codex` exclusions are explained in the prompt.
- **Report location:** reports go only to the acceptance directory and you copy them after review. The `acceptance.env` format matches what `summarize.py` reads.
- **Incomplete grading:** grading is item by item, the met/partly/not-met rule is clear, and there is no `accepted` line without an end time.
- **Features:** the task's required features must be used and confirmed.
- **Task 1:** the Revert test needs a valid change first and then a later refused draft. A clean form with only a refused entry isn't required to enable Revert. Close is tested both ways (refused while dirty, allowed once resolved) through the public close-request path. That fits the task text "Closing the window with unsaved changes is refused".
- **Task 4:** the production sources are compared with the fixture at the run's commit. The deliberate break is allowed only in the copy, must be recorded, and must be restored byte for byte before the final passing run.
- **Task 6:** the prompt covers the checks in the `acceptance.md` paragraph and points to the frozen notification interpretation in `round-3-plan.md`.

**Optional hardening, not blocking:**
- `accept.py:20` could also compare `dirty` and `prepared`, or the whole `kit.env` against the snapshots in `prelaunch.json`. That would catch a kit re-prepared at the same commit.
- The after-session manifest covers only `RUN_DIR/work`. Hashing `conditions.env` and the logs too would cover everything `summarize.py` reads.

Once the blocker is fixed, it's ready.
