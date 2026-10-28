# Acceptance: r3-1-head-codex (task 1, settings editor)

Independent acceptance by a fresh Claude verifier session (claude-opus-5-5),
2026-10-28, following `plans/usability/acceptance.md` (task 1 paragraph) and
the frozen round 3 rules. All builds and tests ran through
`run.sh exec /home/per/pert-ui-usability/kit-r3-head
/home/per/pert-ui-usability/acceptance-r3-1-head-codex <command…>` (sandboxed,
offline, vendored kit). Everything here is headless evidence from the
library's test facilities. It does not show native execution or human usability.

**Verdict: acceptance=met.** All five items are met. The session end time is
recorded (`ended=2026-10-28T16:59:16+01:00`), so `accepted` is written.

## Protocol and metadata

- Kit `kit-r3-head/kit.env` and run `conditions.env` match: commit
  `32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2`, `dirty=0`, `version=0.3.88`,
  `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, prepared `2026-10-28T16:41:41+01:00`.
  Run outcome `ended`, exit 0, tester codex (codex-cli 0.165.0, gpt-6.1-sol,
  medium), brief 16:55:09, ended 16:59:16 (4.1 min. That is short, but the
  file mtimes and the summary's build log agree with it).
- Copy: `RUN_DIR/work` → `ACCEPTANCE_DIR/work` with `rsync -a`, excluding
  `/target`, `.git`, `.claude` and `.codex`. The source had no symlinks and no
  credentials or CLI homes. Every copied file matched
  `original-source-manifest.json` before any change.
- The library resolves to `/kit/vendor/pert-ui-0.3.88` and its runtime, core and
  wgpu crates (`evidence/05-library-source.log`). No workspace path and no
  installed latest library were used.
- Features (`evidence/03-features.log`): `pert-ui` default features only
  (`default` → `desktop` → `gpu` → `native`), and `pert-ui-cpu` is not in the
  graph. The task requires the default features and needs `cpu` only for
  captures. Neither the submission nor the verifier made captures, so there are
  no capture paths.
- After the session, the original `RUN_DIR/work` files still match the
  controller manifest (`evidence/11-integrity.log`). Nothing was written to
  RUN_DIR, the repository or the kit.
- **Protocol deviations: none.**

## Changes in the acceptance copy (verifier-owned)

- `work/src/main.rs`: four lines appended at the end (a comment plus a
  `#[cfg(test)] #[path = "../acceptance/settings_acceptance.rs"] mod
  settings_acceptance;` entry point). These are test builds only, and no
  application line changed (diff in `evidence/11-integrity.log`, original in
  `evidence/main.rs.original`). This entry point was needed because the binary
  crate's `app()` constructor is private.
- Added test source: `work/acceptance/settings_acceptance.rs`.
- Files the tests create: `work/acceptance-output/<test>/settings.json`, plus a
  temporary FIFO in `acceptance-output/item4-fifo/` that the test deletes. The
  tester's own tests rewrite `work/test-output/*/settings.json` with identical
  bytes.
- No other file changed. `Cargo.toml` and `Cargo.lock` are identical, and every
  build used `--locked`.

The verifier tests start the app through its own `app(path).headless()`. They
drive it only with `pert_ui::testing::Driver` (`replace_text`, `key`,
`click`, focus plus keys) and `HeadlessApp::admit_close`. They observe it only
through `Driver::semantics`, `Runtime::inspect()` (the Field `message`
property, Label names) and the file on disk. The one application value they
use is `Editor::root`, the mounted root `WidgetId`, for addressing. No
application signal is read or written.

## Commands and results

| # | Command (via `run.sh exec KIT ACCEPTANCE_DIR`) | Result | Log |
|---|---|---|---|
| 1 | `bash -c 'cargo --version; rustc --version; pwd; ls'` | cargo/rustc 1.99.1 | `evidence/01-toolchain.log` |
| 2 | `cargo build --offline --locked` (unmodified submission) | builds; 1 `dead_code` warning | `evidence/02-build.log` |
| 3 | `cargo tree --offline --locked -e features -i pert-ui`; `-i pert-ui-cpu` | default features; no cpu | `evidence/03-features.log` |
| 4 | `cargo test --offline --locked -- --nocapture` (unmodified submission) | **2 passed, 0 failed** | `evidence/04-tester-tests.log` |
| 5 | `cargo metadata …` (pert-ui manifest paths) | `/kit/vendor/*-0.3.88` | `evidence/05-library-source.log` |
| 6 | `cargo test … settings_acceptance -- --nocapture --test-threads=1` (attempt 1) | verifier compile error (E0618, `app` shadowed by a local) | `evidence/06-acceptance-attempt1.log` |
| 7 | same (attempt 2) | 5 passed, 3 failed, all verifier-test errors (see below) | `evidence/07-acceptance-attempt2.log` |
| 8 | same (attempt 3) | **8 passed, 0 failed** | `evidence/08-acceptance-attempt3.log` |
| 9 | `cargo test --offline --locked -- --test-threads=1` (final full suite) | **10 passed, 0 failed** | `evidence/09-final-full-suite.log` |
| 10 | `cargo build --offline --locked` (non-test build with entry point) | builds | `evidence/10-final-build.log` |
| 11 | host-side hash check of original and copy against the controller manifest | original unchanged | `evidence/11-integrity.log` |

The attempt 2 failures were mistakes in my tests, not application failures. I
changed the tests and left the application alone:
- `item1`: I had asserted that the width `NumberInput`'s semantic name is
  "Maximum width". It is `None`, because the Field wraps a `Row` (control and
  unit), so the label names the Field instead (observation O1). The check now
  reads the Field's name.
- `item3` and `item4_successful_save`: `Driver::click_item` on the format
  `SegmentedControl` returned `ItemWithoutBounds { key: "input", index: N }`
  (library observation L1). The checks now focus the control and use its
  documented keyboard input (`End`, `ArrowRight`).

## Per-item results

### 1. All fields, groups and buttons exist and show the values loaded from the file: **met**

- Code: `src/main.rs:77-104` loads the JSON at startup. `NotFound` gives
  defaults, and an unreadable or invalid file gives defaults with a "Load
  failed" message. `src/main.rs:201-257` builds the titled Output, Quality and
  Behavior groups, the Field-wrapped prefix, format (`SegmentedControl`
  PNG/JPEG/WebP), quality (`IntegerInput` 1–100) and width (`IntegerInput`
  16–16384 with a "px" unit Label), the Toggle, the marker, Apply/Revert and
  the message.
- Tester check: `tests::loaded_fields_validation_revert_and_close` (titles,
  loaded values, unit and format selection through semantics).
- Verifier check: `settings_acceptance::item1_structure_and_loaded_values`.
  It reads the three group titles, the field roles and labels, and the
  format's items `["PNG","JPEG","WebP"]` with PNG selected, quality 33,
  width 16384, "px" and the switch "Open folder after export" on. It also reads
  the Apply/Revert Button roles and names, both disabled, the marker "All
  changes saved" and the message "Settings loaded". A second file
  (`z`/JPEG/1/16/false) rules out matching defaults. Without a file, the app
  shows defaults (photo/JPEG/85/2048/on, "Ready") and creates no file. The
  discovery test prints the full addressable tree (`evidence/08-…log`).

### 2. Invalid prefix and out-of-range quality refused, explanation shown, old value kept: **met**

- Code: a `TextField::normalize` refusal with `InputRejection`
  (`src/main.rs:132-144`) and `IntegerInput::range` (`src/main.rs:146-160`).
  `on_rejection` publishes `FieldStatus::error` to the `FormFields` handle,
  and the Field draws that message below the control.
- Tester check: refuses "bad prefix!" and 101 and reads `invalid` and the
  description. Its "old value kept" assertion reads the application's own
  signal, so on its own it is not enough evidence.
- Verifier check: `settings_acceptance::item2_refusals_keep_last_valid_value`.
  Through the field itself (`replace_text` + Enter):
  - Prefix `"bad prefix!"`, `"dot.name"`, `"ümlaut"`, 33×`a` and `""` are all
    refused. The control is `invalid`, and the Field's inspected `message` is
    "Use 1–32 letters, digits, - or _ only".
  - Quality `0`, `101` and `-5` are refused with "Enter a number from 1 to
    100". Quality `abc`, `50.5` and `""` are refused with "Enter a whole
    number".
  - Width `15` and `16385` are refused with "Enter a number from 16 to 16384".
  - After each refusal, Escape shows the last valid value (`cam-01_X`, 33,
    16384) through semantics.
  - With the refused prefix and quality drafts still in place, clicking Apply
    writes `prefix: "cam-01_X", quality: 33` to disk, which shows the old
    values are still the ones in effect. The refused draft stays shown and
    invalid for correction.

### 3. Apply/Revert disabled when clean, enabled when dirty, Revert restores applied values: **met**

- Code: `dirty` combines value-vs-applied comparison with the `FormFields`
  modified-draft summary (`src/main.rs:110-123`). Both buttons use
  `.enabled(&enabled)`, which also disables them during a save. Revert
  (`src/main.rs:189-200`) restores the applied values, clears the statuses and
  calls `discard_drafts`.
- Tester check: Revert after only refused drafts, and after valid edits plus a
  toggle, with enabled state read from semantics `disabled`.
- Verifier check: `settings_acceptance::item3_dirty_state_and_revert`.
  Enabled state comes only from semantics `disabled` and the marker Label
  name.
  - Round 2's established case: clean → both disabled and "All changes saved".
    A valid quality 60 → both enabled and "Unsaved changes". Then a later
    refused prefix draft, then Revert → the prefix shows `cam-01_X` and is not
    invalid, the Field message is cleared, quality is 33, both buttons are
    disabled and the marker is clean.
  - Format (keyboard End → WebP) and toggle changes are reverted to PNG/on.
  - Editing quality to 34 and back to 33 returns to the clean state.
  - After Apply at 70, a change to 80 plus a refused prefix, then Revert,
    gives 70. Revert goes to the newly applied values, not the loaded ones.
- I did not require Revert to be enabled for a clean form with only a refused
  entry. This app enables it in that case and says so in its README ("Unsaved
  drafts also count as unsaved changes").

### 4. Successful save writes the file; failed save reported and stays dirty: **met**

- Code: Apply snapshots the values and calls `cx.spawn(work, deliver)`
  (`src/main.rs:162-188`). `EventContext::spawn` defers to `Runtime::spawn`,
  the library's background-thread facility. `serde_json` and `fs::write` run
  in the worker. Delivery on the UI thread sets the applied record on success
  or shows "Save failed: …. Apply to retry or Revert." and leaves `applied`
  unchanged, so the state stays dirty.
- Tester check: saves to a file, then saves to a path that is a directory
  (failure), then retries. It reports through the message Label's semantics
  and button semantics.
- Verifier checks:
  - `item4_successful_save_writes_file`: changes all five fields through the
    UI and clicks Apply. The disk JSON is `{"format":"JPEG","max_width":16,
    "open_folder":false,"prefix":"new_name-2","quality":100}`. The buttons are
    disabled and the marker clean. A restart loads the saved values.
  - `item4_save_runs_off_the_ui_thread`: the settings path is a FIFO, whose
    write blocks until a reader opens it. A reader thread waits 1.5 s.
    Clicking Apply returns after about 1.9 ms, while the FIFO has no reader.
    The UI shows "Saving…" with both buttons disabled and still answers
    semantics queries. The reader then receives the JSON
    (`prefix: "threaded"`), and the UI reaches "Settings saved" and the clean
    state. A save on the UI thread would block the click.
  - `item4_failed_save_to_unwritable_path`: saving where the parent directory
    is missing gives "Save failed: No such file or directory (os error 2)…".
    Saving into a read-only directory (0o555, permissions enforced for the
    sandbox user) gives "Save failed: Permission denied (os error 13)…". In
    both cases the marker is "Unsaved changes", both buttons are enabled, no
    file is created and `admit_close` returns `KeepOpen`.

### 5. Closing refused while dirty: **met**

- Code: the `App::on_close` policy returns `KeepOpen` and sets "Unsaved
  changes: apply or revert before closing." while `dirty`, and otherwise
  returns `Close` (`src/main.rs:273-282`).
- Tester check: `admit_close` → `KeepOpen` with refused drafts, then `Close`
  after Revert.
- Verifier check: `settings_acceptance::item5_close_refused_while_dirty`, all
  through `HeadlessApp::admit_close`, the library's headless close-request
  path.
  - Clean at startup → `Close`.
  - Valid quality change → `KeepOpen`, and the message shows "…apply or
    revert…". After Apply → `Close`.
  - Toggle change → `KeepOpen`. After Revert → `Close`.
  - A focused, uncommitted valid draft is committed by admission → `KeepOpen`.
    After Revert → `Close`.
  - The failed-save case also refuses (item 4).

## Missing checks

None for the five items. Captures were not made (`cpu` is optional, and
inspection establishes the visible texts). No native-window close was
exercised, only the headless admission that the library documents as running
the same steps.

## Observations (not grading defects)

Application and usability:
- O1: The width Field wraps a `Row` (input + "px"), so the NumberInput itself
  has no semantic name, and a refusal's text is not its semantic description.
  The Field still has the name "Maximum width", draws the message and reports
  it in inspection (`message: "Enter a number from 16 to 16384"`). This
  affects accessibility, not the task's visible-explanation requirement.
- O2: The tester's own checks read application signals (`e.values`,
  `e.message`, `e.dirty`) for some assertions. The verifier checks replace
  those with semantics, inspection, disk and close-admission evidence.

Library leads (for triage, not verified defects):
- L1: `Driver::click_item` refuses `SegmentedControl` items with
  `ItemWithoutBounds` in 0.3.88, so a test cannot click a segment by index
  (`evidence/07-acceptance-attempt2.log`). This is consistent with the round 3
  item-geometry finding already under repair.
- L2: Field's label naming and message description do not reach a control
  nested inside a structural container such as `Row` (see O1). The Field docs
  say "In semantics the label names the control". Whether a wrapped
  composite should be named needs triage.
