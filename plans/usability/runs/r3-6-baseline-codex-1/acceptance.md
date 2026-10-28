# Acceptance: r3-6-baseline-codex-1 (task 6, notes editor)

Independent verifier: fresh Claude session (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (sha256 `2a082c33…1127`, matches
the frozen `acceptance_sha256` in `runs/review-r3/prelaunch.json`), with the
task 6 notification interpretation from `round-3-plan.md`.

**Result: met (all five items met).**

This is headless, library-driven evidence only. It makes no claim about
native execution or usability for humans.

## Run and kit metadata

| | KIT `kit-r3-baseline/kit.env` | RUN `conditions.env` |
|---|---|---|
| commit | 26e6b5a308df24bffe09f796cf683710979df05e | same |
| dirty | 1 (overlaid brief only, per prelaunch record) | same |
| version | 0.3.75 | same |
| toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) | same |
| prepared | 2026-10-28T16:41:12+01:00 | same |

The kit matches the `baseline` entry in prelaunch.json byte for byte.
Tester: codex-cli 0.165.0, model gpt-6.1-sol, effort medium. Session from
brief 16:42:59 to `ended=2026-10-28T16:48:40+01:00` (about 5.7 min of a
90 min budget), `outcome=ended`, exit 0.

## Copy and isolation

- `RUN_DIR/work` was copied into `ACCEPTANCE_DIR/work` with `rsync -a`,
  excluding `target`, `.git`, `.claude` and `.codex`. The source had no
  symlinks (`find -type l` was empty) and no credentials or CLI homes.
- All 7 copied files match `original-source-manifest.json`. After the
  session, the original `RUN_DIR/work` still matches the manifest
  (`evidence/07-hash-check.log`). The verifier did not write to RUN_DIR.
- Every build and test ran through
  `run.sh exec /home/per/pert-ui-usability/kit-r3-baseline ACCEPTANCE_DIR <cmd>`.
  Dependencies came from the kit's vendored registry only, offline. No
  workspace path or installed library was used.

### Changes made in the copy (test-only)

- `work/src/main.rs`: a 5-line `#[cfg(test)] #[path = "../acceptance/acceptance_tests.rs"] mod acceptance_tests;`
  was appended at the end (`evidence/08-main.rs-test-entry.diff`). This is a
  test entry point only. Non-test builds compile to the same application,
  and no application code changed. The submitted file is preserved as
  `evidence/main.rs.submitted`.
- Added `work/acceptance/acceptance_tests.rs` (sha256 `987d1336…977f`),
  containing the verifier's tests.
- The tester's own test rewrites `work/notes-editor.png`. The regenerated
  file is byte-identical to the submitted one (same sha256 `a7868eac…7072`).
  A pristine copy is in `evidence/submitted-notes-editor.png`.

## Build and test commands and results

| # | Command (all via `run.sh exec KIT ACCEPTANCE_DIR …`) | Result | Log |
|---|---|---|---|
| 1 | `cargo build --offline` | ok, no warnings | `evidence/01-build.log` |
| 2 | `cargo test --offline` (unmodified submission) | **1 passed** (`tests::headless_acceptance`) | `evidence/02-tester-tests.log` |
| 3 | `cargo tree --offline -e features -i pert-ui` | pert-ui v0.3.75 with features `cpu` and `default` (→ desktop → gpu → native); pert-ui-cpu v0.3.75 present | `evidence/03-features.log` |
| 4 | `cargo test --offline discovery -- --nocapture` | inspection-tree discovery probe (temporary, later replaced) | `evidence/04-discovery.log` |
| 5 | `cargo test --offline acceptance_tests::item -- --nocapture --test-threads=1` | 1st attempt: verifier compile error (missing `pert_ui::inspection::*` import); 2nd: 5 passed; 3rd (item 1 with a Tab-traversal step added): passed | `evidence/05-acceptance-attempt{1,2,3-item1}.log` |
| 6 | `cargo test --offline -- --nocapture --test-threads=1` (final) | **6 passed, 0 failed** (tester's test plus 5 acceptance tests) | `evidence/06-final-suite.log` |

The required dependency features (`cpu` plus the defaults) are declared in
`Cargo.toml` (`pert-ui = { version = "0.3", features = ["cpu"] }`) and
confirmed by `cargo tree`.

## What the tester's own check establishes

`src/main.rs:507-652` (`tests::headless_acceptance`) mounts the real
`build()` and drives it with `Driver`. It is good, but it falls short of the
frozen checks in places:

- It edits text with `Driver::replace_text`, not simulated typing at the
  caret with line breaks typed as Enter.
- It chooses menu items with F10/arrow/Enter keyboard navigation, not with
  pointer clicks in the menu bar.
- It reads tab titles and the status with `Driver::semantics`, which is
  acceptable, but tab count with `Runtime::children`.
- It uses a folder whose names are partly out of creation order.

The verifier's tests below fill these gaps. They are the evidence for each
item.

## Verifier tests (`work/acceptance/acceptance_tests.rs`)

Every test calls the application's own `list_notes(&folder)` and then
`build(...)`, the same startup path as `main()`, on its own folder under
`/work/acceptance-out/<item>/notes`. Files are written in non-alphabetical
order and include non-`.txt` files. The `App::run` wrapper itself is not
executed, since that needs a display.

Input goes only through these library facilities:
- `Driver::click` (pointer on list rows, the editor, dialog buttons and the
  menu-bar title)
- `Runtime::pointer_input` (Move/Press/Release on menu rows)
- `Driver::type_text`
- `Driver::key` (Enter, End, Tab, arrows)
- `Driver::chord(KeyChord::primary(..))` (the command key path)

Results are read only through `Runtime::inspect()` (paths, roles, semantic
names, values and items, `tab_count`/`selected`/`open`/`visible_count`
properties, box-model bounds), `Driver::semantics` and the file system. No
`Editor` method or field is called or read.

Menu rows are not separate widgets in 0.3.75: the open popup is one `items`
component with role Menu. So `menu_click` (`acceptance_tests.rs:247`) works
like a user would:
1. Clicks the `title/file` menu-bar title.
2. Moves the pointer down the open popup until inspection reports the wanted
   row as `active`.
3. Presses and releases the primary button there.
4. Asserts that the popup closed.

## Per-item results

### 1. Listing, opening, reopening: **met**

- **Code:**
  - `list_notes` `src/main.rs:378-390` lists regular files with the `txt` extension and sorts them.
  - The sidebar is a `VirtualList` over `names` (`:249-262`).
  - `open` `:66-88` creates a tab once, then sets `active` and calls `reveal_child`.
- **Check:** `item1_listing_open_and_reopen` (`acceptance_tests.rs:307`).
  - **Folder:** created in the order zulu.txt, readme.md, mike.txt, archive.txt.bak, alpha.txt.
  - **Listing:** inspection lists `["alpha.txt","mike.txt","zulu.txt"]` (rows by laid-out y; the List `rows` property is 3).
  - **Keyboard only:** two Tab presses focus the list, then Down and Enter open a note.
  - **Pointer:** a click on a row opens zulu.
  - **Keys and Enter:** open mike, giving 2 tabs.
  - **Reopen zulu by pointer:** `tab_count` stays 2 and `selected` = zulu.txt (the TabList item is marked selected). The status follows.
  - **Reopen mike by keyboard:** count stays 2 and `selected` = mike.txt.
  - The editor shows the file text (TextArea semantics value).

### 2. Typing, marker, status, saves, failed save: **met**

- **Code:**
  - Tab title binding with ` *` when `text != saved` (`src/main.rs:50-55`).
  - Status label `"{name}: {lines} lines, {words} words"` (`:266-289`).
  - `save` `:89-104` writes, and on error calls `Toast::new(format!("Save failed for {}: {e}"))` without touching `saved`.
  - Save, New note and Close tab are registered commands with `KeyChord::primary`; the menu rows use `MenuItem::command` on the same commands (`:219-310`).
- **Check:** `item2_typing_marker_status_saves_and_failed_save` (`:390`).
  - **Typing:** a pointer click focuses the TextArea, then End, `type_text(" two")`, the Enter key, `type_text("three   four")`, Enter.
    - Editor value is `"one two\nthree   four\n"`.
    - Inspection shows title `beta.txt *` and status `1 lines, 2 words` → `2 lines, 2 words` → `3 lines, 4 words`.
    - Disk is unchanged before saving.
  - **Ctrl+S:** the disk content equals the editor text and the title becomes `beta.txt`.
  - **File → Save by pointer clicks:** the disk equals the new text and the marker clears.
  - **Two tabs:** Save writes only the active tab.
  - **Failed save:**
    - The notes folder is removed, then Ctrl+S. The ToastStack (`toasts`, role **Notifications**) count goes 0 → 1 and `zeta.txt *` keeps its marker.
    - A failing File → Save (pointer) raises the count to 2, the marker stays and the text is intact.
  - **Displayed message:** the CPU capture `acceptance-out/item2-failed-save-notification.png` shows the toast "Save failed for zeta.txt: No such file or directory (os error 2)" with Dismiss.
  - This meets the sealed interpretation: existence through the role and count increase in inspection, and display of the actual failure through source plus capture.

### 3. New note dialog: **met**

- **Code:**
  - The library `Dialog` with a `DialogController` (`src/main.rs:313-328`, `:364-366`).
  - Validation in `new_note` (`:136-174`): 1–64 characters, no `/` or `\`, no leading `.`, `.txt` appended; `create_new` refuses an existing file. Errors go into the visible `name-error` Label.
  - On success: `names` is updated and sorted, the dialog is accepted and the note opened.
- **Check:** `item3_new_note_dialog_refusals_and_creation` (`:464`).
  - **Opened via File → New note (pointer):** inspection shows `new-dialog` with role Dialog and `open=true`.
  - **Refusals:** the names `""`, `a/b`, `back\slash`, `.hidden`, `alpha` and `alpha.txt` (alpha.txt exists), plus 65×`x`, are each refused.
    - The dialog stays open.
    - `name-error` is visible with "Use a name of 1 to 64 characters." / "Names cannot start with . or contain / or \." / "A note with that name already exists.".
    - The folder listing is byte-for-byte the same set, and no tab opens.
  - **Valid name `beta`, typed:** `beta.txt` (empty) is created, the dialog closes, the list shows `alpha, beta, mike` in inspection, and the `beta.txt` tab is open and selected with status `beta.txt: 1 lines, 0 words`.
  - **Ctrl+N:** opens the same dialog with the error cleared. `beta` is refused as existing. `gamma.txt` creates `gamma.txt`, not `gamma.txt.txt`; it is listed in order and opened.
  - **Ctrl+N then Cancel:** creates nothing.

### 4. Close choices: **met**

- **Code:**
  - `request_close` `src/main.rs:125-135`: an unmodified tab goes straight to `remove_current`; a modified one opens the close Dialog (`require_response`).
  - The Save button runs `save()` and removes the tab only if it succeeded (`:335-346`).
  - Discard removes without writing (`:347-355`) and Cancel only cancels the dialog (`:356`).
- **Check:** `item4_close_choices_and_unmodified_close` (`:552`).
  - **Unmodified tab, Ctrl+W:** the tab closes at once and the snapshot has no open Dialog/AlertDialog.
  - **Modified tab (text typed with an Enter key):**
    - Ctrl+W shows `close-dialog` open with role Dialog.
    - **Cancel:** the dialog closes, the tab stays (`c.txt *`, still selected), the editor text is `"cee changed\nx"` and the disk is still `cee`.
    - **File → Close tab (pointer), Discard:** the tab is gone, the disk is still `cee`, and reopening shows `cee`.
    - **Ctrl+W, Save:** the disk is `"bee\nsaved on close"` and the tab is gone.
  - **Unmodified tab, File → Close tab (pointer):** closes with no dialog in the snapshot.
  - **Failing Save-and-close (folder removed):** the tab stays modified, the notification count goes +1 and the dialog stays open. Discard then closes it, and the status is `""` with no tab open.
- **Ordering:** "Save writes before the tab disappears" is established three ways:
  - after Save the file holds the text and the tab is gone;
  - removal is gated on save success in source (`:338-341`);
  - the failing save leaves the tab open.

### 5. CPU capture: **met**

- **Submitted:** `work/notes-editor.png` (1000×700) comes from `CpuRenderer::serve_captures` in the tester's test (`src/main.rs:625-633`). It shows tabs `alpha.txt` and `zebra.txt *`. The final suite regenerates it byte-identically.
- **Verifier:** `item5_cpu_capture_two_tabs_one_modified` (`:648`) writes `acceptance-out/item5-two-tabs-one-modified.png`. Viewed: it shows tabs `ideas.txt` and `shopping.txt *`, the typed second line and the status `shopping.txt: 2 lines, 3 words`. The test asserts the PNG signature.

## Captures and other test output (in the copy)

- `work/acceptance-out/item2-two-tabs-one-modified.png`
- `work/acceptance-out/item2-failed-save-notification.png` (shows the failure toast)
- `work/acceptance-out/item5-two-tabs-one-modified.png`
- `work/notes-editor.png` (the tester's test output, identical to the submission)
- `evidence/submitted-notes-editor.png` (pristine submitted capture)
- Notes folders left on disk for inspection: `work/acceptance-out/item{1,3,5}/notes/`. The item 2 and item 4 folders were deliberately removed by their failure checks.

## Missing checks and limits

- The sidebar divider drag is a feature but not a numbered acceptance item, and it was not exercised. Inspection only shows the `split` Splitter as `resizable`.
- `main()` itself (argument parsing, `create_dir_all`, `App::run`) was not executed headlessly. The tests use the same `list_notes` + `build` sequence.
- No native or display execution. No claim of usability for humans.

## Observations (not defects affecting grading)

- **Message discoverability (separate usability observation):** in 0.3.75, inspection exposes the ToastStack only as a count with the Notifications role. The toast's message text is not in semantics or properties. The tester recorded this as friction, and only the capture shows the text.
- **Focus after a row click:** a pointer click on a list row (a `Button` row) does not focus the `VirtualList`. Continuing with arrow keys needs Tab traversal; Tab reaches the list after two presses from a fresh start. This could be an application or library behavior; it was not triaged here.
- The 64-character limit is checked before `.txt` is appended. That is a reasonable reading of the task.
- The tester's test uses the absolute path `/work/test-notes-<pid>` and writes `/work/notes-editor.png`, so it works only inside the sandbox layout.

## Protocol deviations

None observed. Kit and run metadata match, the acceptance hash matches, the
original source is unchanged, and all commands ran through `run.sh exec`.
The only verifier-side error was a missing import in the first draft of the
acceptance tests (a compile error). It was fixed in the test file, and no
application code changed.

## Aggregate

All items are met, so `acceptance=met`, with
`accepted=2026-10-28T16:48:40+01:00` (the `ended` value in
RUN_DIR/conditions.env).
