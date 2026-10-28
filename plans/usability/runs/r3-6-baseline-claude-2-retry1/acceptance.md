# Acceptance: r3-6-baseline-claude-2-retry1 (task 6, notes editor)

Independent acceptance by a fresh Claude verifier session (claude-opus-5-5),
2026-10-28, applying `plans/usability/acceptance.md` (sha256 2a082c33…1127,
matching `prelaunch.json`) with round 3's frozen task 6 notification
interpretation. The same procedure applies to both library versions. This
report claims no native execution or human usability. All evidence is
headless, from the sandbox.

**Verdict: met.** All five items are met, each with passing checks that
drive the library's own facilities. `accepted=2026-10-28T17:49:10+01:00`
(the `ended` time in `conditions.env`).

## Protocol and metadata

| Check | Result |
|---|---|
| Kit `kit-r3-baseline/kit.env` vs run `conditions.env` | match: commit 26e6b5a308df…, dirty=1, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), prepared 2026-10-28T16:41:12+01:00 |
| Kit vs `prelaunch.json` baseline entry | identical |
| Frozen acceptance hash | `acceptance.md` sha256 = prelaunch `acceptance_sha256` |
| Session outcome | `outcome=ended`, `exit_status=0`, `ended=2026-10-28T17:49:10+01:00` (12.9 min after the brief, budget 90) |
| Copy | `RUN_DIR/work` copied to `ACCEPTANCE_DIR/work` with rsync, excluding `target`, `.git`, `.claude` and `.codex`. The source has no symlinks, so none needed refusing. No credentials or CLI homes were copied. |
| Source integrity | Every file in `original-source-manifest.json` matches, both in the original and in the copy, after all runs (`evidence/09-source-integrity.log`) |
| Sandbox | Every build and test ran as `run.sh exec /home/per/pert-ui-usability/kit-r3-baseline <ACCEPTANCE_DIR> …` against the kit's vendored registry, not a workspace path. |

**Protocol deviations:** none found. One note: `round-3-plan.md` changed on
disk during this session (the controller's execution log). I re-read the
task 6 notification paragraph afterwards and it was unchanged. The hashes of
`acceptance.md` and the task file are recorded above and in
`evidence/`.

## Build, features and tester's checks

- `run.sh exec KIT ACC cargo build --locked --all-targets` built successfully
  (exit 0, `evidence/01-build.log`).
- `cargo tree --locked -e features -i pert-ui` shows pert-ui v0.3.75 with
  `cpu` and `default` (→ desktop → gpu → native) from `notes`, satisfying the
  required `cpu` feature on top of the defaults. `Cargo.toml` has
  `pert-ui = { version = "0.3.75", features = ["cpu"] }` (`evidence/02-tester-tests.log`).
- `run.sh exec KIT ACC cargo test --locked`: all 6 tester tests pass
  (`acceptance.rs` ×4, `capture.rs`, `startup.rs`), exit 0. The capture test
  rewrote `work/capture.png` byte-identically to the submitted PNG (sha256 1fa1bffe…).
- Final suite: `run.sh exec KIT ACC cargo test --locked --all-targets -- --test-threads=1`
  passed 12 tests (6 tester, 5 verifier acceptance, 1 verifier exploration), exit 0
  (`evidence/08-final-full-suite.log`).

Shortfalls in the tester's tests that I covered with my own checks:
- They choose File menu items by keyboard (F10/arrows/Enter), never by
  simulated clicks.
- They never use File → Close tab.
- Their folder is the tester's own.

## Added verifier tests (acceptance copy only)

- `work/tests/verifier_acceptance.rs` (sha256 184a8780…): five tests, one per
  item. They drive `Driver::click/chord/key/type_text/replace_text` and
  `Runtime::pointer_input`, and read `Runtime::inspect()` semantics plus the
  file system. The application is built through its public `notes::build`
  entry point, the same one the tester's tests and `main` use. No application
  model function is called, and no application or test-access change was
  made.
- In this library version, menu rows are not separate components. They are
  semantic items of an internal `items` list with uniform row height (in
  `menu.rs`/`select.rs` of pert-ui-runtime 0.3.75). `menu_click`
  (`verifier_acceptance.rs:201`) therefore works like this:
  1. clicks the File title with `Driver::click`;
  2. reads the rows ["New note", "Save", "Close tab"] from the open menu's
     semantics;
  3. sends Move, Press and Release pointer events at the middle of the chosen
     row (bounds height ÷ 3);
  4. asserts that the menu closed.
- `work/tests/verifier_explore.rs` (sha256 9b684943…): a print-only dump of the
  inspection snapshot used to find keys and roles (`evidence/03-verifier-explore.log`).
- Captures:
  - `work/verifier-captures/verifier-two-tabs.png` (sha256 be8c1824…);
  - `work/verifier-captures/verifier-save-failure.png` (sha256 1f79a983…);
  - copies of both in `evidence/captures/`;
  - the tester's original in `evidence/tester-capture-original.png`.
- Verifier-side errors fixed during development, none of them application
  failures:
  - attempt 1: missing `pert_ui::inspection` imports (compile error);
  - attempt 2: the dialog button lookup used a direct-child path, but the
    buttons sit under an `actions` row;
  - attempt 3: I wrongly assumed snapshot order is display order. It is mount
    order, so the new `kiwi.txt` row came last. The check now orders
    inspected rows by their border box and cross-checks the runtime's
    laid-out order.

  Logs: `evidence/04`–`07`.

## Per item

### 1. Folder listing, opening and reopening: **met**

- **Code:** `src/lib.rs:15-30` (`list_notes`: regular files ending in `.txt`,
  sorted), `278-279` (read at startup), `308-318` (`VirtualList` rows plus
  `on_activate`), `264-266` (row click), `120-153` (`open`: reuse the
  existing doc, `reveal_child` selects its tab).
- **Verifier check:** `item1_lists_own_folder_and_reopen_switches`
  (`verifier_acceptance.rs:242`).
  - **Folder:** my own, created in the order zebra.txt, mango.txt,
    data.json, apple.txt, banana.txt.bak, cherry.txt, README. Inspection
    lists exactly apple, cherry, mango, zebra, in that order.
  - **Opening:** mango by pointer, zebra by list focus plus ArrowDown and
    Enter.
  - **Reopening:** mango by click and zebra by keyboard each leave 2 tabs and
    make that tab selected, according to the `TabList` semantics. The status
    line follows.
- **Tester's check:** `tests/acceptance.rs::lists_notes_and_opens_tabs` (passes).

### 2. Typing, marker, status, saving and failed save: **met**

- **Code:**
  - text editing: `TextArea` per tab (`lib.rs:110`);
  - marker: `title` combines the text with its saved copy (`132-134`);
  - status: `status_line` mapped from the text (`51-55`, `136`), shown by a
    `Dynamic` label (`325-330`);
  - Save: a command with Ctrl+S (`300-301`) and a `MenuItem::command` sharing
    it (`334`);
  - `save_doc`: on error it pushes a toast and leaves `saved` unchanged, so
    the marker stays (`156-168`).
- **Verifier check:** `item2_typing_save_shortcut_menu_and_failure`
  (`:277`).
  - **Typing and status:** typing "one two", Enter, "three", Enter, Enter,
    "  four\tfive " gives marker `cherry.txt *` in the `TabList` semantics.
    The status label reads `1 lines, 2 words`, then `2 lines, 2 words`, then
    `4 lines, 5 words`.
  - **Ctrl+S:** writes exactly `one two\nthree\n\n  four\tfive ` to disk and
    clears the marker.
  - **Marker clearing:** typing then Backspace back to the saved text clears
    the marker.
  - **File → Save by pointer click:** on a second tab, it writes `pear red
    green\n\nyellow` and clears that marker. The other tab and file are
    unaffected.
  - **Failed save:** with the folder removed, Ctrl+S and then File → Save by
    click each raise the `Notifications`-role host's count (0→1→2). The tab
    stays `apple.txt *` in both cases.
- **Notification criterion:** count increase plus Notifications role, per the
  frozen interpretation. The source formats the message as `Could not save
  {name}: {error}`. The CPU capture `verifier-save-failure.png` shows two
  toasts reading "Could not save apple.txt: No such file or directory (os
  error 2)" while the tab still shows `apple.txt *`.
- **Tester's check:** `typing_marks_and_saving_clears` (passes; its menu path
  uses the keyboard).

### 3. New note dialog: **met**

- **Code:** `lib.rs:33-48` (rules: 1–64 chars, no `/` or `\`, no leading
  `.`, append `.txt`), `226-244` (`create`: duplicate check against the list
  and disk, `create_new`, list update, `accept`, open), and `347-364` plus
  `383-393` (library `Dialog` with `DialogController`, error `Label`).
- **Verifier check:** `item3_new_note_refusals_and_creation` (`:341`).
  - **Opening:** Ctrl+N opens one library `Dialog`, per its role and visible
    `dialog` part.
  - **Refusals:** each of `""`, `a/b`, `mango`, `zebra.txt`, `back\slash`,
    `.dot` and a 65-character name is submitted with a click on Create. For
    each one:
    - the dialog stays open;
    - the specific explanation is a visible semantic name;
    - the folder listing on disk is unchanged;
    - no tab opens.
  - **Valid name:** `kiwi` closes the dialog and creates an empty `kiwi.txt`.
    It is listed in order (apple, cherry, kiwi, mango, zebra) and opens as
    the active tab with status `kiwi.txt: 1 lines, 0 words`. Typing goes
    into it.
  - **Through the menu:** File → New note by pointer click opens the dialog.
    The now-existing `kiwi.txt` is refused (Enter submits). A 64-character
    name is accepted and opened.
- **Tester's check:** `new_note_dialog_refuses_and_creates` (passes).

### 4. Close tab choices and unmodified close: **met**

- **Code:** `lib.rs:193-203` (`close`: unmodified → `remove`, otherwise open
  the `require_response` dialog) and `207-216` (`finish_close`: Cancel keeps;
  Save closes only when `save_doc` succeeds; Discard removes without
  writing).
- **Verifier check:** `item4_close_choices_and_unmodified_close` (`:406`).
  - **Unmodified tabs:** Ctrl+W closes one and File → Close tab by pointer
    closes another. In both cases no dialog appears in the snapshot, the tab
    is gone and the status line is empty.
  - **Cancel** (modified tab, text with a line break): the tab stays
    `apple.txt *`, the editor text is kept and the file is unchanged on disk.
  - **Discard** (dialog opened via File → Close tab by click): the tab is
    gone and the file is unchanged on disk.
  - **Save:** while the dialog is shown, the file is still old and the tab
    still listed. After Save, the file holds `very ripe` and the tab is gone,
    with the other tab active.
  - **Failed Save:** with the folder removed, the tab stays open and modified
    with its text, and the notification count rises.
- **Tester's check:** `closing_tabs` (passes; Ctrl+W only).

### 5. CPU capture with two notes open, one modified: **met**

- **Tester's capture:** `tests/capture.rs` uses `CpuRenderer::serve_captures`
  and `FrameImage::encode_png`. It asserts the tab titles
  `["groceries.txt", "ideas.txt *"]` and writes `capture.png`, 960×640. It
  regenerated byte-identically in the sandbox. When viewed, the PNG shows the
  File menu bar, a sidebar listing three notes, tabs `groceries.txt` and
  `ideas.txt *`, the edited text and the status line `ideas.txt: 3 lines, 7
  words`.
- **Verifier capture:** `item5_cpu_capture_two_tabs_one_modified` (`:474`)
  wrote `verifier-two-tabs.png` (900×600), showing `apple.txt` and
  `zebra.txt *`.

## Aggregate

Items 1–5 are all met: `acceptance=met`, with
`accepted=2026-10-28T17:49:10+01:00`. Passing the original tests was not
itself treated as acceptance. Items 2, 3 and 4 rest on the verifier's
pointer-driven menu checks and own folder.

## Observations (not grading)

- **Message discoverability (usability, library 0.3.75):** inspection
  exposes the toast host's count (`Notifications` role, `visible_count`) but
  not the toast message text. Only the capture shows the text. The tester
  recorded this as friction (FRICTION.md 17:47).
- **Menu-row addressing (library):** a test cannot click menu rows as
  components or activate them through `Driver`. The verifier had to compute
  pointer positions from internal list geometry (tester friction 17:43).
- **Close-dialog behaviour:** `finish_close` closes the dialog before
  attempting the save, so after a failed Save the dialog is gone and the tab
  remains modified, and a toast reports it. This meets the specification.
- **Cosmetic:** in `verifier-save-failure.png` the `mango.txt` row is
  highlighted because the test's last pointer position (the menu-row click)
  is over it. This is pointer hover, not selection.
- `main.rs` takes the folder from the first argument (default `./notes`,
  created if missing). It was not executed natively: there is no display,
  and the tester recorded the expected EventLoop error. `tests/startup.rs`
  covers `App::headless` startup and divider resizing by keyboard and
  pointer drag.

## Evidence log (ACCEPTANCE_DIR/evidence)

`01-build.log`, `02-tester-tests.log` (tests plus feature tree),
`03-verifier-explore.log`, `04`–`07` verifier attempts (compile/lookup/order
errors, then pass), `08-final-full-suite.log`, `09-source-integrity.log`,
`copy-sha256.txt`, `captures/`, `tester-capture-original.png`.
