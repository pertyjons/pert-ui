# Acceptance: r3-6-head-claude-1 (task 6, notes editor, head kit)

Independent verifier: fresh Claude session (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (sha256 2a082c33…1127, matches
the frozen hash in `runs/review-r3/prelaunch.json`), task 6 paragraph and
the round 3 sealed notification interpretation.

**Verdict: met.** All five numbered acceptance items are met, each with
passing library-driven evidence. The evidence is headless and AI-run. It is
no claim of native execution or human usability.

## Protocol checks

| Check | Result |
|---|---|
| Kit `kit-r3-head/kit.env` vs run `conditions.env` | identical: commit 32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2, dirty=0, version 0.3.88, toolchain rustc 1.99.1 (58cf8f9c3 2026-10-13), prepared 2026-10-28T16:41:41+01:00. Also matches `prelaunch.json` `head`. |
| Frozen acceptance hash | `sha256(acceptance.md)` = 2a082c33f8a1bed2d243706920a7e9ec17320a13cbab4c4d5539bb389b901127 = prelaunch `acceptance_sha256` |
| Session | brief 16:42:57, ended 2026-10-28T16:51:38+01:00, exit 0, outcome=ended (8.7 min of the 90 min budget) |
| Copy | `rsync -a --exclude=/target --exclude=.git --exclude=.claude --exclude=.codex RUN_DIR/work/ ACCEPTANCE_DIR/work/`. No symlinks in the source (`find -type l` = 0). No credentials or CLI homes copied. All 8 copied files match `original-source-manifest.json`. |
| Original | Rechecked read-only after testing: all 8 files still match the manifest. Nothing was written to RUN_DIR. |
| Copy application source | `src/`, `Cargo.toml`, `tests/editor.rs` identical to the original (`diff`). No production or testability change was made. |
| Sandbox | Every build and test ran as `run.sh exec kit-r3-head ACCEPTANCE_DIR …`, offline against `/kit/vendor` (crates-io replaced by the vendored directory; see `evidence/02-features.log`). No workspace path or installed library was used. |

No protocol deviations.

## Build and test commands (all through the harness)

`H=/home/per/github/pert-ui/plans/usability/run.sh`,
`K=/home/per/pert-ui-usability/kit-r3-head`, `A=ACCEPTANCE_DIR`.

| # | Command | Result | Log |
|---|---|---|---|
| 1 | `$H exec $K $A bash -c 'cargo --version; rustc --version; cargo build --locked --all-targets'` | builds, exit 0 | `evidence/01-build.log` |
| 2 | `$H exec $K $A bash -c 'cargo tree --locked -e features -i pert-ui --depth 1; …'` | pert-ui 0.3.88 with features `cpu`, `default` (→ `desktop`, `gpu`, `native`). pert-ui-cpu is linked. The source is the vendored registry. | `evidence/02-features.log` |
| 3 | `$H exec $K $A cargo test --locked -- --test-threads=1` (tester's suite, before any verifier test) | 5 passed, 0 failed | `evidence/03-tester-tests.log` |
| 4 | `$H exec $K $A cargo test --locked --test acceptance -- --test-threads=1 --nocapture`, first attempt | 2 passed, 4 failed: **verifier-test errors**, see below | `evidence/04-acceptance-attempt1.log` |
| 5 | `$H exec $K $A cargo test --locked --test acc_probe -- --nocapture` (diagnostic probe of snapshot shape) | 1 passed | `evidence/05-probe.log` |
| 6 | same as 4, after correcting the verifier helpers | 6 passed | `evidence/06-acceptance-attempt2.log` |
| 7 | same as 4, with added evidence captures | 6 passed | `evidence/07-acceptance-attempt3.log` |
| 8 | `$H exec $K $A cargo test --locked` (final, default parallelism, all targets) | tester 5/5, acceptance 6/6, probe 1/1, all ok, exit 0 | `evidence/08-final-full-suite.log` |

The first attempt failed because of verifier-test errors. None of them was an
application defect, and all were corrected in the test only:

1. The snapshot check of list rows filtered on the inner Button key
   `"open"`. The list re-keys row content by item key, so the probe showed
   the rows as `…/split/notes/<name>`. The filter now selects by path.
2. acc2 assumed the caret sat at the end after `focus()`. It sits at the
   start, so the typed text was `" plus\nmorefirst note"` (3 words). The test
   now asserts that exact text and its counts.
3. Open dialogs were taken to be visible Dialog-role components. The probe
   showed that the library's Dialog host is always visible and reports
   `expanded` and the `open` property for its shown surface
   (`pert-ui-runtime/src/dialog.rs:410-425`). The helper `open_dialogs` now
   uses `expanded == Some(true)` and asserts that `open` agrees.

## Added verifier test source and captures (in the copy only)

- `work/tests/acceptance.rs` (sha256 a74576de…ffced): the acceptance checks
  acc1–acc5 below.
- `work/tests/acc_probe.rs` (sha256 784a6fa9…2000): diagnostic-only dump of
  Dialog, Button and Notifications snapshot records.
- Captures, all from `CpuRenderer::serve_captures`, 960×640:
  - `work/acceptance-captures/acc5-two-tabs-one-modified.png`
  - `work/acceptance-captures/acc2-failed-save-notification.png`
  - `work/acceptance-captures/acc3-existing-name-refused.png`
- `evidence/tester-original-capture.png`: the tester's submitted capture,
  preserved before tests ran. The tester's capture test rewrote
  `work/capture.png` byte-identically (sha256 d0abe715…ab39 before and after).

The tests use the public facilities only: `HeadlessApp`, `testing::Driver`
(`click`, `key`, `chord`, `type_text`, `replace_text`, `advance`,
`clear_focus`), `Runtime::pointer_input` for menu-row clicks and the divider
drag, `Driver::semantics`/`address_children`/`find`, `Runtime::inspect`
snapshots, and the CPU capture service. The app's `Notes` handle is used only
for its root `WidgetId` as a lookup origin. The app's model functions
(`validate_name`, `status_text`, `list_notes`, `open_note`, …) are never
called. The verifier's own folder (`own_folder`, `tests/acceptance.rs:23`) is
created in the order zulu, todo.md, mike, data.json, alpha, kilo,
notes.txt.bak, plus a directory `folder.txt`.

## Per-item results

### 1. Startup listing, opening two notes, reopening switches: **met**

- Source: `src/lib.rs:61-74` (`list_notes`: regular files ending `.txt`,
  sorted), read at build in `src/lib.rs:145`. The list is a `VirtualList`
  of frameless Buttons with `on_activate` for Enter (`src/lib.rs:106-136`,
  `176-192`). `open_note` reuses an open tab and selects it with
  `reveal_child`, `src/lib.rs:342-375`. The folder comes from argv[1] or
  defaults to `./notes` (`src/main.rs:4`); the binary was not executed (no
  display), and the tests start the same `notes_editor::app(dir)`.
- Tester: `tests/editor.rs:111-166` (`lists_notes_and_opens_tabs`) uses its
  own folder, already out of creation order (beta, alpha, gamma, readme.md).
- Verifier: `acc1_own_folder_listing_open_reopen`
  (`tests/acceptance.rs:218`). The listing is exactly `[alpha, kilo, mike,
  zulu]`, both through the list's addressed rows and in the snapshot's
  visible row records. `todo.md`, `data.json`, `notes.txt.bak` and the
  `folder.txt` directory are excluded. Zulu opens by pointer click, mike by
  Home/ArrowDown/ArrowDown/Enter (arrows alone open nothing). Each editor's
  value is read from `MultilineTextInput` semantics. Reopening zulu by
  pointer and mike by keyboard keeps the TabList item count at 2 and moves
  the `selected` item to the reopened tab. Only one editor is visible.
- Divider (feature): `acc1_divider_drag_resizes` (`tests/acceptance.rs:279`)
  presses the `Splitter` ("Resize panes") with the pointer and drags +60 px.
  The sidebar goes from 220 to 280 px. The tester covered keyboard resizing
  only.

### 2. Typing, marker, status, Ctrl+S and menu Save, failed save: **met**

- Source: a `TextArea` per tab, `src/lib.rs:332`. The title combines the text
  and saved signals into `"<name> •"`, `src/lib.rs:356-358`. The status is
  `status_text` mapped from the text, `src/lib.rs:76-81,199-206,360`.
  Commands: `register_command` with `KeyChord::primary('n'|'s'|'w')`, and the
  menu rows are `MenuItem::command` on the same commands, so each shortcut
  and its menu item share one action (`src/lib.rs:168-173,221-225`). A
  failed save calls `ToastController::push`, with `"Could not save {name}:
  {io error}"` and `saved` left unchanged (`src/lib.rs:379-396`).
- Tester: `tests/editor.rs:168-213` covers typing with `\n`, the marker, the
  status, Ctrl+S and disk contents, menu Save, and a removed folder
  producing a toast while the marker stays. Its menu path clicks the File
  title but chooses the row with ArrowDown/Enter (`tests/editor.rs:88-109`),
  which is not a pointer click on the item.
- Verifier: `acc2_typing_marker_status_saves_and_failure`
  (`tests/acceptance.rs:314`).
  - Typing goes into the library editor: `type_text`, an Enter key press,
    and typed `\n`/`\t`. The editor value `"hello  world\nsecond\tline\n\nfourth"`
    gives a status of `4 lines, 5 words` from the status Label's semantics,
    and the TabList item shows `kilo.txt •`.
  - Ctrl+S goes through `Driver::chord`. The file on disk equals the editor
    text, the marker is gone, and the other note is untouched.
  - **File → Save by pointer**: `menu_click` (`tests/acceptance.rs:194`)
    clicks the `title/file` title, checks `expanded`, then presses and
    releases at the centre of the "Save" row. That point is derived from the
    open row list's bounds, because rows give no per-item bounds. Only the
    active tab is saved, the disk matches, and the marker clears.
  - Failed save: the folder is removed and Ctrl+S pressed. The snapshot's
    `Notifications` component goes from 0 to 1 items, named "Could not save
    kilo.txt: No such file or directory (os error 2)". The marker stays and
    the text is kept. A menu Save in the same state adds a second
    notification. The capture `acc2-failed-save-notification.png` shows the
    toast with the actual OS error. This satisfies the sealed criterion
    (count increase under the Notifications role, plus source and capture
    showing the real failure).

### 3. New note dialog refusals and valid creation: **met**

- Source: `validate_name`, `src/lib.rs:85-104` (1–64 characters, no `/` or
  `\`, no leading `.`, `.txt` appended, duplicate refused). `create_note`
  uses `create_new` and shows errors in the dialog's error Label
  (`src/lib.rs:455-472`). The dialog is a library `Dialog` with
  `DialogController` (`src/lib.rs:258-278`).
- Tester: `tests/editor.rs:219-285` covers empty, `a/b`, `alpha` and
  `beta.txt` refused with the Label text, no file created, `delta` created,
  listed and opened, and the menu reopening the dialog cleared.
- Verifier: `acc3_new_note_dialog_refusals_and_creation`
  (`tests/acceptance.rs:390`).
  - Ctrl+N opens one expanded Dialog in the snapshot.
  - Each of `""`, `a/b`, `a\b`, `.hidden`, 65 characters, `alpha` and
    `zulu.txt` is entered with `replace_text` and Create is clicked. For
    each, the refusal text appears as a visible Label in the snapshot, the
    dialog stays open, and the directory listing is unchanged.
  - A 64-character name creates `<64>.txt` (empty), which is listed in
    order, opened, and its tab selected.
  - File → New note **by pointer** reopens the dialog with the explanation
    cleared. `charlie.txt` stays `charlie.txt`, not `.txt.txt`, and is listed
    and opened.
  - Exactly the two valid files were added on disk.
  - The capture `acc3-existing-name-refused.png` shows the refusal in red
    inside the dialog.

### 4. Close with Cancel, Discard and Save; unmodified close without dialog: **met**

- Source: `close_active` opens the close dialog only when the text differs
  from the saved text, otherwise it removes the tab (`src/lib.rs:425-434`).
  `close_pending` returns early on a failed save, keeping the tab
  (`src/lib.rs:437-445`). Cancel only finishes the dialog
  (`src/lib.rs:252`).
- Tester: `tests/editor.rs:291-383` covers Cancel, Escape, Discard via menu
  (keyboard row choice), Save, a failed Save keeping the tab, and an
  unmodified Ctrl+W leaving `DialogStatus::Idle` (controller state, not the
  snapshot).
- Verifier: `acc4_close_choices_and_unmodified_close`
  (`tests/acceptance.rs:478`).
  - An unmodified tab closes on Ctrl+W, and on File → Close tab **by
    pointer**, with no expanded Dialog or AlertDialog in the snapshot. The
    tab is removed and the file unchanged.
  - Modified mike → Ctrl+W → Cancel keeps the tab (selected, `•`) and the
    editor text, and leaves the file unchanged.
  - Pointer menu Close tab → Discard removes the tab and leaves the file
    byte-identical. Reopening shows the file's text.
  - Ctrl+W → before choosing, the file is unchanged → Save writes the editor
    text to disk, the dialog closes and the tab disappears.
  - With the folder renamed away, dialog Save adds one notification, keeps
    the tab as `alpha.txt •` with its text, and the original file is intact.
  - Discarding the last tab empties the status line.

### 5. CPU capture PNG, two notes open, one modified: **met**

- Tester: `tests/editor.rs:385-414` uses `CpuRenderer::new().serve_captures`
  and writes `capture.png` (960×640). The submitted image (preserved in
  `evidence/tester-original-capture.png`) shows tabs `alpha.txt` and
  `beta.txt •` and the status "beta.txt: 3 lines, 7 words". Regenerating it
  in the acceptance sandbox reproduced it byte-identically.
- Verifier: `acc5_cpu_capture_two_tabs_one_modified`
  (`tests/acceptance.rs:577`), `acceptance-captures/acc5-two-tabs-one-modified.png`,
  shows `mike.txt` and `zulu.txt •` (asserted through TabList semantics
  before capture) and was viewed. More than a screenshot backs the item:
  the marker state is asserted through inspection in the same test.

## Aggregate

Items 1–5 are all met, so `acceptance=met`. The session end time is
recorded (`ended=2026-10-28T16:51:38+01:00`), so `accepted` is written.

## Missing evidence / limits

- The native binary (`src/main.rs`, command-line folder argument) was not
  executed without a display. The headless tests start the same `app(dir)`
  entry the binary calls.
- Menu-row clicks use a pointer position derived from the open menu list's
  bounds and the equal row height, because `MenuBar` rows report no
  `SemanticItem::bounds` (`Driver::click_item` refuses them). This is
  real pointer input through `Runtime::pointer_input`, and the actions
  observed (file written, dialog opened, tab closed) confirm that the
  intended rows were hit. It is still not a library-supported item address.

## Library / usability observations (not application defects)

- Menu rows lack per-item bounds, so a test cannot `click_item` a menu
  command. The tester fell back to keyboard row selection, and the verifier
  derived row geometry. This is the same area as the "stable Select/menu
  part-address" checkpoint in round 3 triage.
- Notification discoverability: on head, the toast message text is present
  as the Notifications item name in inspection. The tester read it there
  directly.
- A typed TAB character renders as a missing-glyph box in the `TextArea` in
  CPU captures (`acc2-failed-save-notification.png`, "second□line"). This is
  a rendering observation, not required by the task.
- After a programmatic `focus()`, a `TextArea` places the caret at the start
  of the text. This is expected but worth knowing for test authors.
- The tester's FRICTION.md records the missing "Inspection and tests"
  sections on the Tabs, MenuBar and Dialog pages, and the discovery cost of
  their part addresses.
