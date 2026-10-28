# Acceptance: r3-6-head-claude-2-retry1 (task 6, notes editor, head)

Verifier: fresh Claude acceptance session (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (sha256 `2a082c33…1127`, matches the
frozen `acceptance_sha256` in `runs/review-r3/prelaunch.json`), task 6 paragraph
plus round 3's sealed task 6 notification interpretation (`round-3-plan.md`,
"Frozen experiment"). Evidence is from headless library-driven tests only.
It does not show native execution or human usability.

**Verdict: met.** All five items are met, with passing library-driven checks of
my own in the acceptance copy. The tester's own six tests also pass.

## Protocol and metadata

| Check | Result |
|---|---|
| Kit `kit-r3-head/kit.env` vs run `conditions.env` | commit `32dd9c4f667b…`, dirty=0, version 0.3.88, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, prepared `2026-10-28T16:41:41+01:00`: identical in both and equal to `prelaunch.json` `head` |
| Frozen acceptance hash | `2a082c33f8a1bed2d243706920a7e9ec17320a13cbab4c4d5539bb389b901127` = prelaunch value |
| Session | brief 17:28:50, `ended=2026-10-28T17:36:13+01:00`, exit 0, outcome ended (7.4 min of the 90 min budget); credential_source=explicit-copy (the recorded auth-recovery replacement slot for `r3-6-head-claude-2`) |
| Copy | `RUN_DIR/work` → `ACCEPTANCE_DIR/work` with rsync, excluding `target`, `.git`, `.claude`, `.codex`. The source has no symlinks. All 8 copied files match `original-source-manifest.json` |
| Original after work | Rehashed against the manifest: no mismatches (`evidence/07-original-unchanged.log`). Nothing was written to RUN_DIR |
| Application code in copy | `src/*.rs`, `Cargo.toml`, `Cargo.lock`, `tests/app.rs` unchanged (same hashes). No test access hooks were added. Only new test files were added |

**Protocol deviations: none found.** One record observation: the run's
`summary.md` lists a `tests/listing.rs` in its diagnostics. The final
submission has no such file; the tester merged that work into
`tests/app.rs`. It is an intermediate state, not a deviation.

## Build and test commands

Every command ran as `run.sh exec /home/per/pert-ui-usability/kit-r3-head
/home/per/pert-ui-usability/acceptance-r3-6-head-claude-2-retry1 <cmd>`. That
is the sandbox, with the kit's vendored crates (`/kit/vendor`, offline). No
workspace or installed library was used.

| # | Command | Result | Log |
|---|---|---|---|
| 1 | `cargo build --offline --locked` | ok, built pert-ui 0.3.88 from the vendored registry | `evidence/01-build.log` |
| 2 | `cargo tree --offline --locked -e features -i pert-ui` (+ `-i pert-ui-cpu`, Cargo.lock, cargo config) | features `cpu` and `default` (→ desktop → gpu → native) enabled by `notes`; pert-ui-cpu 0.3.88 in the tree; source replaced by `/kit/vendor` | `evidence/02-features.log` |
| 3 | `cargo test --offline --locked --test app` (tester's suite) | 6 passed, 0 failed. The regenerated `notes-capture.png` is byte-identical to the submitted one (`cbeb539b…`) | `evidence/03-tester-tests.log` |
| 4 | `cargo test --offline --locked --test acceptance_probe -- --nocapture` | exploration only: dumps menu bar inspection | `evidence/04-probe.log` |
| 5 | `cargo test --offline --locked --test acceptance -- --nocapture --test-threads=1` | 5 passed on the first attempt | `evidence/05-acceptance-attempt1.log` |
| 6 | `cargo test --offline --locked -- --nocapture --test-threads=1` (final, all targets) | lib 0, bin 0, acceptance 5/5, probe 1/1, app 6/6, doctests 0: exit 0 | `evidence/06-final-full-suite.log` |

## Added test sources and captures (acceptance copy only)

- `work/tests/acceptance.rs`: the independent acceptance checks, sha256 `a9e511ce…c7a1`.
- `work/tests/acceptance_probe.rs`: exploration probe, not a check; sha256 `b5e9dfb4…404e1f`.
- `work/acceptance-capture.png` (= `evidence/acceptance-capture.png`, `ea5b6cc5…`): my CPU capture for item 5.
- `work/acceptance-failed-save.png` (= `evidence/acceptance-failed-save.png`, `09ea73a6…`): CPU capture right after the failed Ctrl+S.
- `evidence/original-notes-capture.png`: the submitted capture as copied; `evidence/rerun-notes-capture.png`: the regenerated copy, byte-identical.

How my tests drive the app: they call only `notes::app(dir).headless()` and
use the key-path layout the app publishes (`Notes::content_path`, key names).
Input goes through `Driver::click`, `type_text`, `key`, `chord`,
`replace_text`, `Runtime::focus` and raw `Runtime::pointer_input`. Results are
read from `Runtime::inspect()` component records (path, visibility, semantics,
bounds), from CPU captures and from the files on disk.

Menu clicks (`menu` helper, `tests/acceptance.rs:183`): menu bar titles and
rows are not addressable in 0.3.88. The File title has `path: None`, key
`title/file`, role `Menu`. The open menu's rows report no item bounds
(`evidence/04-probe.log`), so `Driver::click_item` cannot target them. The
helper does this instead:

1. Presses and releases the pointer at the centre of the File title's
   inspected border box.
2. Moves the pointer down the open menu until the menu's inspected value
   (`Choice.label`, the highlighted row) equals the wanted row.
3. Presses and releases there, then asserts that the menu closed.

Each menu action's effect is asserted separately: a file written, a dialog
opened, a tab closed. This is real pointer input, not a keyboard substitute.

## Items

### 1. Listing in order, opening two, reopening switches — **met**

- Source: `src/lib.rs:64-77` keeps only regular files ending in `.txt`, then
  sorts them. `open_note` (`src/lib.rs:162-184`) reuses an existing doc, calls
  `reveal_child` on its tab and sets it active. The rows are a `VirtualList` of
  frameless buttons with `on_activate` (`src/lib.rs:109-133`, `359-366`).
- My check: `item1_listing_and_reopen` (`tests/acceptance.rs:262`).
  - The folder is created in this order: `mango.txt`, `zeta.md`, `kiwi.txt`,
    `README`, `apple.txt`, `notes.txt.bak`, `banana.txt`.
  - Inspection lists `apple, banana, kiwi, mango`: visible row components
    ordered by their y bounds, with the count cross-checked against the List
    role's `Choice.count`.
  - It opens kiwi by pointer and apple by keyboard (focus the list, Down,
    Enter). The TabList reports count 2 with apple selected.
  - Reopening kiwi by pointer gives count 2, kiwi selected and the status
    line `kiwi.txt: …`. Reopening apple by keyboard (Home, Enter) gives
    count 2, apple selected.
- Tester's check: `lists_and_opens` (`tests/app.rs:25`) passes. It uses
  `address_children` and a folder where `b.txt` is created first plus
  `other.md`.

### 2. Typing, marker, status counts, Ctrl+S and menu Save on disk, failed save — **met**

- Source:
  - Each tab holds a `TextArea` over a per-doc signal. The title is
    `ReadSignal::combine(text, saved)`, which appends ` *` (`src/lib.rs:169-176`).
  - The status line is `Dynamic` over the active doc with
    `status_text = '\n' count + 1` and `split_whitespace` (`src/lib.rs:98-102`,
    `373-379`).
  - Save, New note and Close tab are registered commands with `KeyChord::primary`
    bindings, shared by `MenuItem::command` (`src/lib.rs:393-413`).
  - On a write error, `save_doc` pushes the toast `Could not save {name}: {error}`
    and leaves `saved` unchanged, so the marker stays (`src/lib.rs:187-201`).
- My check: `item2_edit_save_and_failed_save` (`tests/acceptance.rs:310`).
  - Typing goes through `type_text` and the Enter key into an empty note.
    Inspection then reads:
    - the tab label `plan.txt *`;
    - the status line moving `1 lines, 0 words` → `1 lines, 2 words` →
      `2 lines, 2 words` → `4 lines, 6 words`;
    - the editor's `SemanticValue::Text` = `"first line\nsecond  line here\n\nend"`.
  - The disk is still empty before saving.
  - Ctrl+S (`Driver::chord`) writes exactly that text and the label drops ` *`.
  - More typing, then **File → Save by pointer clicks in the menu bar**. The
    file equals the inspected text, the marker is gone, and the status line
    equals my own count of the inspected text (5 lines, 7 words).
  - Failed save: the folder is removed and Ctrl+S pressed. The
    Notifications-role stack's value rises from 0 to 1. Its item reads
    `Could not save plan.txt: No such file or directory (os error 2)`, which
    is the real failure. The label stays `plan.txt *`.
  - `evidence/acceptance-failed-save.png` shows the toast on screen with the
    marker kept.
  - A second failed save through File → Save by menu clicks raises the count
    to 2, and the marker stays.
  - This meets the sealed notification criterion: a `ToastStack` count rise
    with the Notifications role, a displayed failure message, and the tab
    still modified.
- Tester's check: `edits_and_saves` (`tests/app.rs:102`) passes, but it
  reaches File → Save with F10 and arrow keys, not menu clicks. Without my
  check it would leave the menu-click requirement unproven.

### 3. New note dialog refusals and valid creation — **met**

- Source:
  - `note_file_name` checks the name: 1–64 chars, no `/` or `\`, no leading
    `.`, and appends `.txt` when missing (`src/lib.rs:80-95`).
  - `create_note` refuses an existing name and otherwise writes the empty
    file, accepts the dialog, adds the name to the sorted list and opens it
    (`src/lib.rs:245-262`).
  - The dialog is a library `Dialog` with `DialogController`. Its refusal text
    is a bound `Label` with `ColorRole::Error` (`src/lib.rs:264-294`).
- My check: `item3_new_note_dialog` (`tests/acceptance.rs:392`).
  - Ctrl+N (chord) opens the dialog. The content `app/new-dialog/dialog/new-note`
    is visible and the host role is `Dialog`.
  - Each of these names is submitted with the Create button and refused:
    `""`, `a/b`, `a\b`, `.hidden`, a 65-char name, `pear` (an existing
    `pear.txt`) and `fig.txt`. For each one:
    - the dialog stays open;
    - the visible `name-error` label carries the explanation, e.g.
      "Enter a name.", "A name must not contain / or \.",
      "A note named pear.txt already exists.";
    - the directory listing on disk is unchanged.
  - A 64-char name submitted with Enter creates `<64 y>.txt`, empty, and opens
    it as the active tab.
  - **File → New note by menu clicks** reopens the dialog with the old
    refusal cleared. `banana` plus the Create click creates an empty
    `banana.txt`.
  - The disk then holds exactly the two new files. Inspection lists
    `banana, fig, pear, <64 y>` (the `.csv` file is hidden), the TabList shows
    count 2 with banana active, and the status reads `banana.txt: 1 lines, 0 words`.
- Tester's check: `new_note_dialog` (`tests/app.rs:163`) passes. It reaches
  the menu with F10.

### 4. Close tab: Cancel, Discard, Save, unmodified without dialog — **met**

- Source:
  - `command_close` closes an unmodified tab directly. Otherwise it opens the
    library `Dialog` (`src/lib.rs:234-243`).
  - Save: `save_doc` runs, then `close_doc`, and only if the save succeeded.
    Discard: `close_doc` with no write. Cancel: cancels the dialog
    (`src/lib.rs:296-334`).
- My check: `item4_close_choices` (`tests/acceptance.rs:455`).
  - Four notes are opened.
  - Unmodified tab + Ctrl+W: no visible component with a `dialog` part
    anywhere in the inspection snapshot, tab count 4→3, `four.txt` gone.
  - Unmodified tab + **File → Close tab by menu clicks**: no dialog content,
    count 3→2, the previously active title gone.
  - Modified `two.txt` (typed text plus Enter) + Ctrl+W: the dialog content
    `…/close-dialog/dialog/close-tab` with Save/Discard/Cancel is visible.
  - **Cancel**: the dialog content is gone, the tab stays (`two.txt *`
    active), the editor text equals the typed text, and the disk still holds
    `dos`.
  - **Discard**: the tab is gone and the disk still holds `dos`. Reopening
    shows `dos` unmodified.
  - **Save** (dialog reached through **File → Close tab by menu clicks**): the
    disk holds exactly the inspected editor text and the tab is gone. The
    write and close run in one deferred callback in that order
    (`src/lib.rs:305-309`), and the close depends on the save succeeding.
    The failed-save case below shows that dependency.
  - Folder removed + Ctrl+W + Save: the tab stays, still marked ` *`, and the
    Notifications count is 1.
- Tester's check: `closes_tabs` (`tests/app.rs:223`) passes and covers the
  same choices. It checks the dialog through component `visible`, not the
  whole snapshot.

### 5. CPU capture PNG, two notes open, one modified — **met**

- Tester's check: `cpu_capture` (`tests/app.rs:308`) uses `CpuRenderer::new().serve_captures` through `Runtime::request_capture`. The submitted `notes-capture.png` (960×640) shows the tabs `groceries.txt` and `ideas.txt *` and the status line `ideas.txt: 2 lines, 6 words`. I viewed it, and the rerun reproduces it byte-for-byte.
- My check: `item5_cpu_capture` (`tests/acceptance.rs:548`) takes a fresh CPU capture. The inspected tab titles are `["alpha.txt *", "beta.txt"]` and `acceptance-capture.png` shows both tabs, the marker on alpha and the status line. I viewed it.

## Aggregate

Items 1–5: met, met, met, met, met → `acceptance=met`. The session end time
is recorded (`ended=2026-10-28T17:36:13+01:00`), so `accepted` is written.

## Missing checks

None for the acceptance items. Two limits on what this shows:
- Divider dragging and the real windowed start are task features, not
  numbered acceptance items. They are not graded. The tester's
  `divider_resizes_sidebar` passes headlessly. A windowed run was not
  attempted (no display in the sandbox).
- No native or human-usability claim is made.

## Usability observations (library, separate from grading)

- **Menu rows can't be targeted by pointer.** Menu bar titles and menu rows
  have no address (`path: None`), and menu rows report no
  `SemanticItem::bounds`. So `Driver::click_item` can't click a menu command,
  even though its documentation lists "a menu command" as an example item. A
  pointer-click test has to scan with hover. The tester used F10 and arrow
  keys instead. Related to the pending Select/menu part-address checkpoint.
- **Undocumented `dialog` segment.** Dialog content sits under an
  undocumented `dialog` address segment (`app/new-dialog/dialog/new-note`).
  This was the tester's main friction entry.
- **Failure messages are discoverable.** On head, the failure message is
  readable as a toast stack item name and visible in the capture.

## Evidence log index

`evidence/01-build.log`, `02-features.log`, `03-tester-tests.log`,
`04-probe.log`, `05-acceptance-attempt1.log`, `06-final-full-suite.log`,
`07-original-unchanged.log`, `08-added-tests.sha256`, and the PNGs listed above.
