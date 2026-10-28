# Acceptance: r3-6-head-codex-1 (task 6, notes editor, head kit)

Verifier: fresh Claude acceptance context (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (task 6 paragraph) and the frozen
round 3 task 6 notification interpretation in `round-3-plan.md`.

**Result: met — all five acceptance items are met.**

Evidence comes only from headless runs through `run.sh exec` in the
sandbox, using the library's input, inspection and CPU-capture facilities.
None of this shows native execution or human usability.

## Run and kit metadata

| | Kit `kit-r3-head/kit.env` | Run `conditions.env` |
|---|---|---|
| commit | 32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2 | same |
| dirty | 0 | same |
| version | 0.3.88 | same |
| toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) | same |
| prepared | 2026-10-28T16:41:41+01:00 | same |

Run details: tester codex (codex-cli 0.165.0, gpt-6.1-sol, medium effort),
90-minute budget. The brief was delivered at 2026-10-28T16:48:40+01:00 and
the session **ended at 2026-10-28T16:55:08+01:00**, `exit_status=0`,
`outcome=ended`.

Inside the sandbox, `rustc 1.99.1 (58cf8f9c3 2026-10-13)` and cargo 1.99.1
were used. `Cargo.lock` resolves `pert-ui 0.3.88` from the registry source,
which the run-local cargo config replaces with `/kit/vendor` offline. No
workspace path or installed library was used (`evidence/40-toolchain.log`).

## Setup

- `ACCEPTANCE_DIR` had already been initialized by the controller with
  `run.sh exec KIT ACCEPTANCE_DIR true`, and its `work/` was empty.
- I copied `RUN_DIR/work` to `ACCEPTANCE_DIR/work` with `rsync -a`,
  excluding `/target`, `.git`, `.claude` and `.codex`. The source had no
  symlinks (`find -type l` found 0), and no credentials or CLI homes were
  copied.
- Before any test, the copied files matched the original byte for byte
  (`evidence/original-work-sha256.txt` against
  `evidence/copy-initial-sha256.txt`). The tester's original PNG was saved
  as `evidence/tester-original-notes-editor.png`.
- After all tests (`evidence/41-hash-check.txt`):
  - Every file in `original-source-manifest.json` is unchanged in
    `RUN_DIR/work`.
  - Every such file in the copy still matches the manifest. This includes
    `src/main.rs`, `Cargo.toml` and `Cargo.lock`, and the regenerated
    `notes-editor.png` and `test-notes/*`.

## Build and the tester's own checks

All commands run as `run.sh exec /home/per/pert-ui-usability/kit-r3-head /home/per/pert-ui-usability/acceptance-r3-6-head-codex-1 <cmd>`.

| Log | Command | Result |
|---|---|---|
| `evidence/01-tester-build.log` | `cargo build --offline` | builds, exit 0 |
| `evidence/02-tester-tests.log` | `cargo test --offline -- --nocapture` | `tests::acceptance_through_ui` ok, 1 passed |
| `evidence/03-features.log`, `03b-features-cpu.log` | `cargo tree --offline -e features -i pert-ui` / `-i pert-ui-cpu` | pert-ui features `cpu` and `default` (→ desktop → gpu → native) are enabled by notes-editor; pert-ui-cpu is pulled in |
| `evidence/30-final-full-suite-run1.log`, `…run2.log` | `cargo test --offline -- --nocapture --test-threads=1` | ok both times: bin 1 passed; verifier binary 8 passed (includes the tester's test again), 1 ignored (exploration dump) |

The required dependency features are confirmed: `Cargo.toml` has
`pert-ui = { version = "0.3.88", features = ["cpu"] }` with default features
left on.

The tester's single test (`src/main.rs:463-655`) drives the real `App`
headlessly with `Driver`. Their test alone does not cover several frozen
checks:

- It edits with `Driver::replace_text` instead of simulated typing.
- It reaches menu items through F10 and arrow keys, not clicks in the menu
  bar.
- For the unmodified close it checks only the tab count, not that no dialog
  appears.
- It uses its own folder.

I added independent checks for these.

## Added acceptance tests (in the copy only)

- `work/acceptance_tests/verifier.rs`: all of the verifier's checks.
- `work/tests/acceptance_verifier.rs`: a test entry point. The submission is
  a binary crate, so its private `app()` and `Editor` cannot be reached from
  an integration test. This file compiles the **unchanged** `src/main.rs` as
  a module with `include!` and places the verifier module inside it.
  Application behavior is unchanged.
- Test access: the only value read from application state is the root
  `WidgetId` that the application stored from `mount_tree`
  (`HeadlessApp::state().root`). It is the starting point for `Driver::find`,
  because the library has no other public accessor for the mounted root.
  Every observation goes through `Driver::semantics`/`find`/
  `address_children`, `Runtime::inspect`, `Runtime::bounds` and the files on
  disk.
- How input is driven:
  - `Driver::click` for list rows, the editor, and dialog buttons/fields.
  - `Driver::type_text` (including `"\n"`) and `Driver::key(Enter/ArrowDown)`.
  - `Driver::chord` with Control+S/N/W, and Control+End for the caret.
  - `Runtime::pointer_input` for menu-bar clicks and the divider drag.
- How the menu is clicked: at 0.3.88, menu rows report no item bounds. The
  check clicks the `File` title at its inspected border box. It computes the
  row's rectangle from the open menu panel's inspected bounds divided by its
  three rows, moves the pointer there, and **asserts through inspection that
  exactly the intended row is highlighted**. Only then does it press and
  release, and it asserts that the menu closed.
- Each test uses its own folder under `/work/acceptance-notes/<test>`.
- Captures written by the tests (CPU renderer, `CpuRenderer::serve_captures`):
  - `work/acceptance-captures/item5-two-tabs-one-modified.png`
  - `work/acceptance-captures/item2-failed-save.png`
  - `work/acceptance-captures/item3-refusal.png`
  - `work/acceptance-captures/item4-close-dialog.png`

Development history of the added tests, kept for transparency:

- `10-explore.log` and `11-explore.log` are exploration dumps.
- `20-verifier-first.log` is the first passing run. During that interim
  step, a `#[cfg(test)] mod` line had been appended to the copy's
  `src/main.rs`. I then restored `src/main.rs` byte for byte from the
  original (SHA256 7422659f…, matching the manifest) and replaced the line
  with `tests/acceptance_verifier.rs` (`21-verifier-entry.log`).
- All final results come from the `30-*` logs, where the submitted source is
  unchanged.

## Per-item results

### 1. Folder listing, opening two notes, reopening switches tab — **met**

- Source:
  - `build` reads the folder at startup, keeps regular files ending in
    `.txt` and sorts them (`src/main.rs:210-219`).
  - The sidebar is a `VirtualList` of Button rows with keyboard activation
    (`263-266`).
  - `Editor::open` adds a tab only if the note is not already open, sets it
    active and reveals the tab with `reveal_child` (`75-122`).
- Check: `item1_list_from_own_folder_open_and_reopen`.
  - The verifier's folder was created in the order pear.txt, todo.md,
    apple.txt, data.json, mango.txt, README, plus an `archive/` directory.
  - Inspection lists exactly `apple.txt, mango.txt, pear.txt`, in
    on-screen order.
  - apple.txt opens with a pointer click. pear.txt opens with ArrowDown
    until the list's choice is pear.txt, then Enter.
  - The tab list shows two tabs, with pear selected and only pear's editor
    visible.
  - Reopening apple by pointer keeps 2 tabs and selects apple: the status
    reads `apple.txt: 2 lines, 4 words` and apple's editor is visible.
  - Reopening pear by Enter keeps 2 tabs and selects pear.
- Tester's check: Alpha/Beta are listed while `ignore.md` is not, and
  reopening keeps two tabs with Alpha selected (`main.rs:477-516`).

### 2. Typing, marker, status counts, Ctrl+S and menu Save, failed save — **met**

- Source:
  - The tab title is `name *` when the text differs from the saved text
    (`84-86`).
  - The status is derived from the text signal and counts `'\n' + 1` lines
    and `split_whitespace` words (`88-95`). It is shown for the active note
    (`282-296`).
  - Save is a single registered command bound to Ctrl+S and used by the menu
    row (`241-246`, `313`).
  - `save_note` writes the file and, on failure, keeps `saved` unchanged so
    the marker stays, and sets the notification label (`123-137`, `302`).
- Check: `item2_typing_marker_status_save_paths_and_failure`.
  - The editor is focused by a pointer click, then Control+End.
  - Typing `" more"` changes the title to `beta.txt *` and the status to
    `1 lines, 3 words`.
  - Enter gives `2 lines`. Typing `"second   line\nthird"` gives an editor
    value of `first line more\nsecond   line\nthird` with
    `3 lines, 6 words`. The file is still unchanged.
  - Ctrl+S, pressed with focus in the editor, writes exactly that text to
    disk and removes the marker. The other note is untouched.
  - More typing marks the tab again (`4 lines, 8 words`). **File → Save by
    simulated pointer clicks** writes the exact text and clears the marker.
  - Failure test: after more typing, the notes folder is **removed**
    (`remove_dir_all`) and Ctrl+S is pressed.
    - The `notification` label is visible in inspection with name
      `Save failed for beta.txt: No such file or directory (os error 2)`.
    - Its border box `[0, 592, 389, 20]` lies inside the 960×640 window.
    - The title stays `beta.txt *` and the editor text is intact.
    - File → Save by pointer fails the same way.
  - Capture `item2-failed-save.png` shows the message above the status line.
- Notification criterion (frozen round 3 interpretation): the failure
  message appears in a plain `Label`, not a `ToastStack`. The interpretation
  allows "a visible failure message in another component … if inspection
  exposes it". Here inspection exposes the full message text, and the source
  shows it is the actual I/O error. Discoverability is recorded separately
  under observations.

### 3. New note dialog: refusals and valid creation — **met**

- Source:
  - `new_note` resets and opens a library `DialogController` (`165-170`).
    The form is a library `Dialog` (`322-332`, `363-370`).
  - `create` refuses names outside 1–64 characters, names containing `/` or
    `\`, and names starting with `.`, through a visible `error` label. It
    appends `.txt` when missing, creates the file with `create_new` (an
    existing name gives "A note with that name already exists."), adds and
    sorts the list, accepts the dialog and opens the note (`171-206`).
- Check: `item3_new_note_dialog_refusals_and_creation`.
  - Ctrl+N shows the form. The host component has role `Dialog` and a
    visible dialog panel.
  - These names are each refused (typed into the field) after clicking
    Create: empty, `a/b`, `apple` (existing), `apple.txt`, `back\slash`,
    `.hidden`, 65 characters.
  - For each refusal:
    - The `error` label is visible with non-zero height and the expected
      explanation.
    - The dialog stays open.
    - The directory listing is byte-for-byte unchanged and no `a/`
      directory appears.
    - The sidebar and tabs are unchanged.
  - Capture `item3-refusal.png` shows the explanation in the dialog.
  - The valid name `kiwi` creates an empty `kiwi.txt` and closes the dialog
    (no panel visible). The sidebar reads `apple, kiwi, pear` and the tab
    `kiwi.txt` is open and selected, with status `kiwi.txt: 1 lines, 0 words`.
  - **File → New note by pointer** opens the dialog with the error cleared.
    `fig.txt` creates `fig.txt` (not `.txt.txt`), listed in order and opened.
  - Ctrl+N pressed **while the text editor has focus** opens the dialog and
    types no character.
  - A 64-character name is accepted. Cancel creates nothing.

### 4. Close a modified tab with Cancel, Discard, Save; unmodified closes without dialog — **met**

- Source:
  - `close` opens the close dialog only when text ≠ saved; otherwise it
    removes the tab immediately (`153-164`).
  - The Save button saves and removes the tab only if the save succeeds
    (`339-350`). Discard removes without writing (`351-357`). Cancel only
    closes the dialog (`358`).
- Check: `item4_close_choices_and_unmodified_close`.
  - Cancel (Ctrl+W pressed with focus in the editor): the close form and the
    root `Dialog` appear (capture `item4-close-dialog.png`). After Cancel, no
    dialog is visible, the tab stays `plum.txt *` and selected, the editor
    text is intact, and the file still has its original contents.
  - Discard (**File → Close tab by pointer**): the tab is gone and the file
    is unchanged on disk.
  - Save: the reopened note shows the disk text. After typing and Ctrl+W →
    Save, the file contains `plum on disk\nsaved on close` and the tab is
    gone. The handler writes before removing the tab; a failed write keeps
    the tab, as shown below.
  - Unmodified tab by Ctrl+W, and again by File → Close tab: **no visible
    dialog panel or form anywhere in the inspection snapshot**. The tab
    closes, the status becomes empty and the file is untouched.
  - Save choice with the folder removed: the tab stays open and modified,
    the notification reports the failure and the text is intact.

### 5. CPU capture with two notes open, one modified — **met**

- Source: the tester's test calls `set_capture_support`,
  `request_capture(CaptureTarget::window(primary))`,
  `CpuRenderer::new().serve_captures` and `encode_png` (`main.rs:537-546`).
- The tester's capture is `work/notes-editor.png`, saved as
  `evidence/tester-original-notes-editor.png`. It shows tabs `Alpha.txt *`
  and `Beta.txt` and status `Alpha.txt: 2 lines, 5 words`. Re-running the
  tester's test in the copy regenerated it with an identical SHA256
  (515b16…).
- Independent check: `item5_cpu_capture_two_tabs_one_modified` asserts
  through inspection that the tabs are `ideas.txt` and `shopping.txt *` and
  the status is `shopping.txt: 4 lines, 4 words`. It then writes
  `acceptance-captures/item5-two-tabs-one-modified.png` from the CPU
  renderer, and visual inspection shows both tabs with the marker on
  shopping.txt.

### Supplementary checks (features outside the numbered items)

- `supplementary_active_tab_consistency_after_close`: closing the active
  tab of three leaves `b.txt` selected. The status describes `b.txt`, only
  b's editor is visible, and typing then Ctrl+S saves `b.txt`. Selection,
  status and Save agree.
- `supplementary_divider_drag`: a 60 px pointer drag on the divider widens
  the sidebar from 220 to 280 px. The tester checked the same thing with
  48 px.

## Aggregate

Items 1–5 are all met, so `acceptance=met`. The session end time is
recorded, so `accepted=2026-10-28T16:55:08+01:00`.

## Observations (not graded)

- **Notification discoverability (separate usability observation, as the
  frozen rule requires):**
  - The failure is a persistent plain label above the status line. It is
    not a toast and has no Notifications role.
  - Inspection exposes the full message text.
  - It stays until the next successful save clears it, and it is shared
    with "Could not open" errors.
  - When Save fails from the close dialog, the dialog stays open while the
    message appears behind it.
- Sorting uses `Vec<String>::sort` (byte order), so uppercase names would
  sort before lowercase ones. No mixed-case folder was tested. I did not
  grade this because the task does not define case handling.
- With no tab open, the editor area (the `Dynamic`/`Tabs` region) has zero
  width and the right pane is blank (see `item3-refusal.png`). The tester
  recorded the same thing in FRICTION.md.

## Library observations (head 0.3.88), not application defects

- `MenuBar` menu rows and `Tabs` titles report no `SemanticItem` bounds, so
  `Driver::click_item` cannot click them. The verifier had to derive menu
  row positions from the panel's bounds and confirm them with the hover
  highlight. This matches the round's already-recorded geometry finding.
- `HeadlessApp` and `Runtime` have no public accessor for the mounted root
  `WidgetId`. Tests depend on the application storing it.
- `KeyChord` has no Control constructor, and the split-pane types are not in
  the prelude. The tester recorded both in FRICTION.md and the diagnostics.

## Protocol deviations

None affecting the result. Notes:

- An interim test entry line was appended to the copy's `src/main.rs`
  during exploration. It was restored byte for byte, and the final evidence
  uses the unchanged file (see "Development history").
- The tester's own test writes into the copy's `/work`: it regenerates
  `notes-editor.png` and `test-notes/` (both match the original hashes) and
  briefly uses `test-notes-moved`. This is expected for running their check.
- The `run.sh exec` invocations go through a logging wrapper
  (`evidence/run.sh.helper`) that records each command and its exit status.
- I did not write to `RUN_DIR`. The original source manifest still matches
  `RUN_DIR/work`.

## Evidence index

- Logs: `evidence/01-tester-build.log`, `02-tester-tests.log`,
  `03-features.log`, `03b-features-cpu.log`, `10-explore.log`,
  `11-explore.log`, `20-verifier-first.log`, `21-verifier-entry.log`,
  `30-final-full-suite-run1.log`, `30-final-full-suite-run2.log`,
  `40-toolchain.log`, `41-hash-check.txt`.
- Hashes: `evidence/original-work-sha256.txt`,
  `evidence/copy-initial-sha256.txt`.
- Added test source: `work/acceptance_tests/verifier.rs`,
  `work/tests/acceptance_verifier.rs`.
- Captures: `work/acceptance-captures/*.png`,
  `evidence/tester-original-notes-editor.png`.
