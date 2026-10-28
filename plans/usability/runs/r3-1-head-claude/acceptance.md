# Acceptance: r3-1-head-claude (task 1, settings editor)

Independent verifier: fresh Claude context (claude-opus-5-5), 2026-10-28.
The session was interrupted once by a service usage limit after the verifier
tests were written. It resumed in the same acceptance copy. No evidence was
recopied or discarded, and no blind task was rerun.

**Verdict: met.** All 5 acceptance items are met, each with passing tests
driven through the library in the harness sandbox.

## Run and kit metadata (protocol)

- RUN_DIR `conditions.env`: commit 32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2,
  dirty=0, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), prepared
  2026-10-28T16:41:41+01:00. This matches KIT `kit.env` and the `head` entry
  of `plans/usability/runs/review-r3/prelaunch.json` byte for byte.
- Session: brief 2026-10-28T17:03:13+01:00, ended 2026-10-28T17:10:51+01:00,
  exit 0, outcome ended (7.6 min of the 90 min budget).
- The original submission's files match the controller's
  `original-source-manifest.json` (SHA-256) before the copy, in the copy and
  after all checks (`evidence/09-original-recheck.log`: "original unchanged").
- Copy: `rsync -a` of RUN_DIR/work into ACCEPTANCE_DIR/work, excluding
  `target`, `.git`, `.claude` and `.codex`. The source had no symlinks
  (checked with `find -type l`). No credentials or CLI homes were copied.
- Dependencies: the lockfile resolves pert-ui, -core, -runtime, -cpu and -wgpu
  at 0.3.88 from the kit's vendored registry, offline (`evidence/01-toolchain.log`
  shows the sandbox cargo config). No workspace paths and no installed library.
- **Protocol deviations:** none in the verification itself. Process note: the
  verifier session was interrupted once by a service usage limit and resumed
  in place. The one verifier-test correction is described below; it is not a
  deviation.

## Required features

Task: pert-ui default features, plus `cpu` for captures. `Cargo.toml` has
`pert-ui = "0.3.88"` (default features) and a dev-dependency adding `cpu`.
`cargo tree -e features -i pert-ui` (`evidence/03-tree.log`) confirms default →
desktop → gpu → native, plus cpu from dev-dependencies. The headless app
(`App::headless`, `HeadlessApp`) comes from `native`; captures use `CpuRenderer`.

## Commands and results

Every command ran as `run.sh exec /home/per/pert-ui-usability/kit-r3-head
/home/per/pert-ui-usability/acceptance-r3-1-head-claude <command>`.

| Log (evidence/) | Command | Result |
|---|---|---|
| 01-toolchain.log | `bash -c 'rustc --version; cargo --version; cat ~/.cargo/config.toml'` | rustc/cargo 1.99.1; vendored offline source |
| 02-build.log | `cargo build --locked --all-targets` | ok |
| 03-tree.log | `cargo tree --locked -e features -i pert-ui` | default+cpu confirmed |
| 04-tester-tests.log | `cargo test --locked -- --nocapture` | tester tests: acceptance 8/8, capture 1/1 pass |
| 05-verifier-tests-attempt1.log | `cargo test --locked --test verifier_acceptance -- --nocapture --test-threads=1` | 5/6; v2 failed on a wrong **verifier** assumption (see below) |
| 06-verifier-tests.log | same, after the verifier-only correction | 6/6 pass |
| 07-final-full-suite.log | `cargo test --locked --all-targets -- --nocapture --test-threads=1` | 15/15 pass (8 + 1 + 6) |
| 08-release-build.log | `cargo build --locked --release --bin settings_editor` | ok (native binary built, not run) |
| 09-original-recheck.log | host-side SHA-256 check of RUN_DIR/work against the manifest | unchanged |

**Verifier-test error (corrected, not an application defect):** attempt 1
asserted that every refused quality explanation names the range "1"/"100".
For malformed text (`1e2`, `seventy`) the field correctly refuses with "Enter a
whole number". The refusal assertion (`invalid`) passed before the failing
line. The check now requires the range text only for out-of-range integers and
a non-empty explanation for malformed text (`tests/verifier_acceptance.rs:248-260`).
No application code was changed.

## Added verifier tests and captures

- Test source (verifier-added, the only change to the copy):
  `work/tests/verifier_acceptance.rs`, SHA-256
  0ff97fc7bdd4f978119688bea193929e8d7ba5ab92cef7a9024be2798943c71d.
  The tests drive input only through `App::headless`, `Driver` (`replace_text`,
  `type_text`, `key`, `click`, `settle`) and `HeadlessApp::admit_close`. They
  read results only through `Driver::semantics`, inspection queries
  (`InspectionQuery` with `Detail::Full` properties), CPU captures and files on
  disk. They never read or write the application's signals. Besides `app()`,
  the only application item used is `Editor::root`, the mounted root's
  `WidgetId`, as the key-path anchor. No test entry point or test access was
  added, and no application file was changed.
- Captures (CPU renderer): `work/verifier-captures/{v1-loaded,v2-refused,
  v3-dirty-with-refused-draft,v4-save-failed,v5-close-refused}.png`. These and
  the tester's `clean/refused/dirty.png` are copied to `evidence/captures/` with
  `SHA256SUMS`. The verifier viewed `refused`, `clean`, `v3`, `v4-save-failed`
  and `v5-close-refused`.

## Per-item results

The application source is `work/src/lib.rs`. Tester tests are in
`work/tests/acceptance.rs`; verifier tests are in `work/tests/verifier_acceptance.rs`.

### 1. Fields, groups and buttons exist and show values loaded from the file — **met**

- Source: `load` reads the JSON at startup (`lib.rs:73-83`, `build` at
  136-140). It falls back to defaults with a message when the file is missing
  or invalid. Groups are `Group::titled` "Output", "Quality" and "Behavior"
  (197-241). The prefix `TextField`, the format `Select` with PNG/JPEG/WebP,
  the quality `IntegerInput`, the width `IntegerInput` with `.unit("px")` and
  the toggle are each in a labelled `Field`. Apply and Revert are in 299-304.
- Tester: `shows_the_values_loaded_from_the_file` (acceptance.rs:95) and
  `starts_from_defaults_without_a_file` (142) both pass.
- Verifier: `v1_every_field_shows_the_loaded_file` (verifier_acceptance.rs:158)
  loads values that differ from the tester's sample and the defaults
  (`cam-A_7`, PNG, 12, 16384, off). It checks Group roles and titles, every
  field's semantic value, the unit property "px", Switch/Button roles and names,
  and clean buttons. It also checks the three format choices, in order
  PNG→JPEG→WebP, through key input on the dropdown. Startup loading is also
  shown in `v4_failed_save_is_reported_and_stays_dirty`: a restart shows the
  saved prefix. Capture `v1-loaded.png`. Passes.

### 2. Invalid prefix and out-of-range quality refused, explanation shown, old value kept — **met**

- Source: the prefix `TextField::normalize` with `check_prefix` returns an
  `InputRejection` (lib.rs:44-57, 181-187). `IntegerInput::range` handles
  quality and width (211-218). `on_rejection` puts the reason on the
  `FieldHandle`, so `Field` shows it under the control.
- Tester: `refuses_invalid_input_and_keeps_the_old_value` (acceptance.rs:161)
  passes. It reads the kept value from the application's signal.
- Verifier: `v2_refused_input_keeps_the_last_valid_value_in_effect` (:214)
  refuses input through the field itself. The prefix gets text typed at the
  caret and committed by focus loss (Tab), then empty, 33-character, `a/b` and
  non-ASCII `ä` values. Quality gets 101, 0, -5, 1e2 and "seventy"; width gets
  15. Each shows `invalid` with an explanation in the control's semantics and
  on the `Field`. The 32-character prefix boundary and quality 1/100 and width
  16 are accepted. **Last valid value in effect is shown on disk:** with
  refused drafts left in the prefix and quality fields, Apply writes the last
  valid prefix and quality 100. Capture `v2-refused.png` plus the tester's
  `refused.png`, which the verifier viewed: red explanations sit directly
  under the fields. Passes.

### 3. Apply/Revert disabled when clean, enabled when dirty; Revert restores applied values — **met**

- Source: `changed` compares current with applied values (lib.rs:155-165).
  Apply is enabled when `changed && !saving` and Revert when
  `unsaved && !saving` (169-170). The marker label reads "● Unsaved changes" or
  "All changes applied" (171-178, 302). Revert writes the applied values,
  clears statuses and calls `form.discard_drafts` (272-292).
- Tester: `apply_and_revert_follow_the_dirty_state` (acceptance.rs:216) and
  `choosing_a_format_makes_the_editor_dirty` (397) pass.
- Verifier: `v3_revert_after_valid_change_and_later_refused_draft` (:296) reads
  the enabled state from both semantics (`disabled`) and the inspection
  property `enabled`, and requires them to agree. Clean: both disabled, and
  clicking the disabled Apply writes nothing. A valid quality change makes both
  enabled with a different marker. A toggle and a key-driven format change
  follow. **Round 2's case:** a later refused prefix draft keeps both enabled.
  Revert then restores every field, clears the refusal, disables both buttons,
  restores the clean marker and leaves the file unchanged. After an Apply,
  Revert restores the newly applied values, not the loaded ones. Capture
  `v3-dirty-with-refused-draft.png`. Passes.
- Observation (not graded): with only a refused draft on a clean form, this
  application enables Revert but not Apply. The tester's test covers this
  case. The frozen check does not require either behavior.

### 4. Successful save writes the file; failed save reported and stays dirty — **met**

- Source: Apply calls `cx.spawn(work, deliver)` (lib.rs:244-269). That is
  `EventContext::spawn`, which defers to `Runtime::spawn` and runs the work on
  a spawned thread, per the kit's `pert-ui-runtime` docs. `save` uses
  `std::fs::write` on the worker. Delivery sets `applied` on success
  (clearing dirty) or the message "Save failed: …" on failure, keeping dirty.
- Tester: `apply_saves_the_file` (acceptance.rs:275) and
  `a_failed_save_is_reported_and_stays_dirty` (313) pass. The tester's failure
  case is a path under a regular file.
- Verifier, off the UI thread: `v4_save_runs_off_the_ui_thread` (:371) makes
  the settings path a FIFO with no reader. The click on Apply returns, which a
  watchdog confirms; a save on the UI thread would block until the watchdog
  unblocked it. The UI then settles 20 frames showing "Saving…" with Apply
  disabled, and the test reads the JSON (quality 42) from the FIFO. The UI then
  shows "Settings saved" and is clean. Passes.
- Verifier, results: `v4_failed_save_is_reported_and_stays_dirty` (:428)
  removes the folder; Apply then shows "Save failed: … No such file or
  directory". Buttons stay enabled, the marker shows unsaved, the edit is kept,
  no file is created and a close request is refused. A read-only (0555) folder
  also fails ("Permission denied (os error 13)", which ran rather than being
  skipped) and stays dirty. After the folder is restored, Apply succeeds and
  the disk holds all five values. The state is clean, close is admitted, and a
  restart loads the saved prefix. Capture `v4-save-failed.png` shows the
  message. Passes.

### 5. Closing refused while dirty — **met**

- Source: `App::on_close` returns `KeepOpen` when `has_unsaved()` and sets the
  message "Unsaved changes: apply or revert them before closing" (lib.rs:332-342).
- Tester: `closing_is_refused_while_dirty` (acceptance.rs:346) passes. It uses
  `HeadlessApp::admit_close`, including a focused draft that admission commits.
- Verifier: `v5_close_refused_while_dirty_and_admitted_when_resolved` (:501)
  goes through `HeadlessApp::admit_close`. The library documents it as running
  a close request's admission and the `on_close` policy (testing guide, "The
  close policy"). Clean: Close. A valid change: KeepOpen, with the visible
  "apply or revert" message; the edit stays. Resolved by Apply: Close. A toggle
  change: KeepOpen; resolved by Revert: Close. A valid change plus a refused
  draft: KeepOpen; after Revert: Close. Toggled back to the applied value:
  Close. Capture `v5-close-refused.png` shows the request. Passes.
- Not shown: a native window-system close (no display; no native execution
  is claimed).

## Missing checks and limits

- No native window was run (no display). The release binary was only built.
  This is headless AI-driven evidence; no claim about human usability.
- The off-thread proof is behavioral (FIFO blocking), supported by the
  library's documented `spawn` semantics. No thread identity was observed.

## Usability observations (separate from grading)

- In the default dark theme, enabled and disabled Apply/Revert look much the
  same (captures `clean.png` vs `dirty.png`). The tester reported this too.
- When the scroll area shows a vertical scrollbar (e.g. after the refusal
  messages add height), the groups' right borders are clipped (`refused.png`,
  `v3-…png`). This is a layout or library observation, not graded.
- The tester's friction log reports that a `Select`'s semantics are not
  reachable from its own key path; they need `find_descendant(.., "select")`.
  The verifier needed the same workaround.

## Aggregate

Items 1–5 are all met, so **acceptance=met**. A session end time is recorded,
so `accepted=2026-10-28T17:10:51+01:00`.
