# Acceptance: r3-6-baseline-codex-2 (task 6, notes editor, baseline 0.3.75)

Verifier: fresh Claude acceptance session (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (sha256 `2a082c33…1127`, equal to
the frozen `acceptance_sha256` in `runs/review-r3/prelaunch.json`), task 6's
task-specific paragraph, and the round 3 task 6 notification interpretation
in `round-3-plan.md` ("Frozen experiment"). Apply the same grading to both library versions.

**Verdict: met.** All five acceptance items are met. The evidence is the
verifier's own headless checks, which pass in the harness sandbox against
pert-ui 0.3.75 with `cpu` + defaults. The tester's own test also passes,
but it does not establish simulated typing or clicked menu items by itself
(see "Tester's own checks").

## Protocol and metadata

| Check | Result |
|---|---|
| KIT `kit.env` vs RUN_DIR `conditions.env` | identical: commit `26e6b5a308df24bffe09f796cf683710979df05e`, dirty=1, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), prepared 2026-10-28T16:41:12+01:00 |
| KIT vs prelaunch.json `baseline` | identical |
| Acceptance procedure hash | `2a082c33f8a1bed2d243706920a7e9ec17320a13cbab4c4d5539bb389b901127`, equal to prelaunch |
| Session | codex-cli 0.165.0, gpt-6.1-sol (configured), medium; brief 17:02:23, `ended=2026-10-28T17:10:00+01:00`, outcome ended, exit 0 |
| Copy | `rsync -a` of RUN_DIR/work excluding `target`, `.git`, `.claude`, `.codex`; no symlinks present (none to refuse); no credentials or CLI homes copied. All 26 copied files matched `original-source-manifest.json` |
| Original after verification | all manifest files re-hashed: unchanged. Nothing was written to RUN_DIR |
| Sandbox | every build/test ran via `run.sh exec KIT ACCEPTANCE_DIR …`; pert-ui crates resolve from `/kit/vendor/pert-ui-*-0.3.75` (cargo metadata, evidence/08) |

**Protocol deviations: none.** Notes:
- Running the tests rewrites `/work/notes-editor.png` in the copy, because
  the tester's test writes it. The regenerated file is byte-identical to the
  submitted one (sha256 `b921869a…dcfd13`).
- The verifier's first draft overwrote the exploration probe
  `probe_inspection`. It was restored verbatim as an ignored test, and its
  original output is preserved in `evidence/03-probe.log`.

## Build and test commands (all through `run.sh exec KIT ACCEPTANCE_DIR`)

| Command (inside `/work`) | Result | Log |
|---|---|---|
| `cargo tree -e features -i pert-ui --offline` | pert-ui v0.3.75 with features `cpu`, `default` (→ desktop, gpu, native) | evidence/01-toolchain-features.log |
| `cargo build --offline --locked` (unmodified copy) | ok, no warnings | evidence/02-tester-build-test.log |
| `cargo test --offline --locked -- --nocapture` (unmodified copy) | 1 passed (`tests::acceptance_through_ui`) | evidence/02-tester-build-test.log |
| `cargo build --offline --locked` (with verifier tests) | exit 0 | evidence/08-final-run.log |
| `cargo test --offline --locked -- --nocapture` | 7 passed, 1 ignored, exit 0 | evidence/08-final-run.log |
| `cargo test --offline --locked tests::acceptance_through_ui` | tester's test alone: 1 passed, exit 0 | evidence/08-final-run.log |
| `cargo metadata --offline --locked` | pert-ui 0.3.75 from `/kit/vendor`, features cpu, default, desktop, gpu, native | evidence/08-final-run.log |
| `cargo test --offline --locked -- --include-ignored` | 9 passed (incl. 2 probes), exit 0 | evidence/09-final-run-with-probes.log |
| `cargo test --offline --locked` (final state) | 7 passed, 2 ignored, exit 0 | evidence/09-final-run-with-probes.log |

Intermediate verifier iterations are in `evidence/04-acceptance-run1.log`
(errors in the verifier's own tests: `TimeReversed` from the verifier's
`flush(Duration::ZERO)`, an `AmbiguousKey("dialog")` lookup across the
nested dialogs, and a keyboard step that counted a focused row button as the
focused list), `05-probe-focus.log` (Tab order probe), `06-…` and `07-…`
(passing). Each of those errors was in the test code and was fixed there.
None of them was an application failure.

## Added test source and test entry point

- `work/src/acceptance_tests.rs` (verifier-authored, final sha256
  `fda69cc8…ad430`).
- `work/src/main.rs`: the only change is 5 appended lines, a test-only module
  declaration (`#[cfg(test)] #[path = "acceptance_tests.rs"] mod
  acceptance_tests;`). It does not change application behavior. Diff:
  `evidence/main.rs.entry-point.diff`; the pre-change file is
  `evidence/main.rs.before-entry-point`.
- Startup in the verifier harness mirrors `main()` lines 391–394: it calls
  `Editor::list(folder)` and then `build(ui, folder, names)`. `App::run`, the
  native window path, is not exercised. After startup, the checks use only
  `Driver` (click, type_text, key, chord, semantics), `Runtime::pointer_input`
  for menu rows, `Runtime::inspect`, the CPU renderer and the files on disk.
  No application model function is called after startup.
- Captures written by the verifier: `work/acceptance-two-tabs.png`
  (sha256 `e8fa64f1…d5a2`) and `work/acceptance-save-failure.png`
  (sha256 `5c14661e…d606`). Both are 1000×700 CPU-renderer PNGs and are
  deterministic across runs.

How the checks drive the UI:
- **Menu items** (`Acc::menu`, acceptance_tests.rs:236): `Driver::click` on
  the `title/file` menu-bar title. The pointer then moves down the open
  `Menu` popover with `pointer_input(Move)` until the menu's own semantics
  highlight the wanted row, and presses and releases the primary button
  there. The check asserts that the menu closes afterwards.
- **Typing:** `Driver::click` into the TextArea, the End key, then
  `Driver::type_text` with line breaks entered as `Driver::key(Enter)`.
- **Shortcuts:** `Driver::chord(KeyChord::primary(c), KeyCode::KeyC)`, the
  library's command-key path.
- **Inspection reads:** list rows, as the visible child components of the
  `notes` list ordered by border-box y; tab titles and the selected tab, from
  the `TabList` semantics items; the status line, from the `counts` Label's
  semantic name; editor text, from the TextArea's semantic value; dialogs, as
  the visibility of the `dialog` part under each `Dialog`-role component;
  notifications, as the `toasts` component's `Notifications` role and its
  Number value (the count).

## Per-item results

### 1. Listing, opening, reopening: **met**
- Source: `Editor::list` (main.rs:67–79) keeps regular files with extension
  `txt` and sorts them. It is called at startup (main.rs:392). The sidebar is
  a `VirtualList` with `on_activate` (main.rs:285–291), and each row button
  defers `open` (main.rs:54–63). `open` reuses an existing note and reveals
  its tab (main.rs:101–132).
- Evidence: `item1_listing_open_and_reopen` (acceptance_tests.rs:350).
  - The verifier's folder was created in this order: `pear.txt`,
    `zucchini.txt`, `readme.md`, `apple.txt`, `photo.png`, `mango.txt`,
    `old.txt.bak`, and a directory `archive.txt/`. Inspection lists
    `["apple.txt","mango.txt","pear.txt","zucchini.txt"]`, and the List
    semantics report count 4.
  - Opening by pointer (pear) works.
  - Opening by keyboard works: 5 real Tab presses reach the list itself,
    then arrows and Enter open mango and zucchini.
  - Reopening pear by pointer and mango by keyboard leaves 3 tabs, makes the
    reopened tab the selected one, and updates the status line to match.
- The tester's test also covers this, using its own folder and `ui.focus`
  for the keyboard part.

### 2. Typing, marker, counts, saves, failed save: **met**
- Source:
  - The title is `text.combine(saved)` with a ` *` suffix (main.rs:110–112).
  - The status is `<name>: {breaks+1} lines, {split_whitespace} words`
    (main.rs:113–120), shown through a `Dynamic` Label (main.rs:295–306).
  - The Save command uses Ctrl+S (main.rs:270–275). The menu item is
    `MenuItem::command` on the same registered command (main.rs:322–324).
  - `save_note` writes, and on success sets `saved` (clearing the marker).
    On error it pushes `Toast::new("Save failed for {name}: {e}")` and
    leaves `saved` unchanged (main.rs:142–154, 80–83).
- Evidence: `item2_typing_save_and_failed_save` (acceptance_tests.rs:434).
  - Typing goes into the TextArea: " is sweet", Enter, "second line",
    Enter, Enter, "  four\tfive  ". The marker appears in the TabList
    semantics.
  - The status line follows each step: 1 lines/2 words → 1/4 → Enter 2/4 →
    final 4 lines/8 words. The editor's semantic value equals the typed text.
  - Ctrl+S: the file on disk equals the typed text and the marker is gone.
  - More typing (Enter + "from menu") gives 5 lines/10 words. A clicked
    File → Save then writes the exact text to disk and clears the marker.
  - With a second tab modified, the notes folder is removed. Ctrl+S then
    raises the `toasts` Notifications count from 0 to 1 and the stack is
    visible; the tab keeps `apple.txt *` and its text, and no file exists.
    A clicked File → Save fails the same way (count +1, marker kept).
- Notification criterion (round 3 interpretation): the `ToastStack` count
  increase with the Notifications role establishes that the notification
  exists. The source (main.rs:150) and the capture
  `acceptance-save-failure.png` show that the toast displays the actual
  failure: "Save failed for apple.txt: No such file or directory (os error 2)".
- The tester's test uses `replace_text` and F10 keyboard menu navigation
  instead of typing and clicks. The verifier's check is the evidence for
  this item.

### 3. New note dialog: **met**
- Source:
  - The dialog is a library `Dialog` with `DialogController` (main.rs:373–375).
  - The form has a TextField, an error Label, and Create/Cancel buttons
    (main.rs:331–346).
  - `create` (main.rs:203–243) validates 1–64 characters, no `/` or `\`, and
    no leading `.`, and appends `.txt` when missing. It uses
    `create_new(true)`, so an existing note is refused, and it refreshes the
    list, closes the dialog and opens the new note.
  - The New note command uses Ctrl+N (main.rs:263–268).
- Evidence: `item3_new_note_dialog` (acceptance_tests.rs:516). The checks
  alternate between the clicked File → New note item and Ctrl+N, and use a
  fresh dialog for each refusal; each opening shows an empty explanation.
  - Inspection shows the dialog part visible under a `Dialog`-role component.
  - Refused with the explanation visible in inspection:
    - empty → "Use 1–64 characters, no / or \, and do not start with a dot."
    - `sub/dir` → the same text
    - `apple` and `pear.txt` → "A note named … already exists."
    - Extras from the feature list: `back\slash`, `.hidden` and a
      65-character name.
  - After all refusals the folder's entries are unchanged, the list is
    unchanged and no tab is open.
  - Valid `kiwi` creates an empty `kiwi.txt`. The list becomes
    apple/kiwi/pear, the kiwi tab opens selected, the status reads
    "kiwi.txt: 1 lines, 0 words", and the dialog closes.
  - `banana.txt` creates `banana.txt`, not `banana.txt.txt`.
  - A 64-character name is accepted.

### 4. Close choices: **met**
- Source:
  - `close` (main.rs:171–182) removes an unmodified tab at once and opens
    the confirmation `Dialog` (main.rs:350–378) for a modified one.
  - `finish_close` (main.rs:183–196): Save calls `save_note` first and
    returns without closing on failure. Discard removes without writing.
    Cancel calls `DialogController::cancel`.
  - The Close tab command uses Ctrl+W (main.rs:277–282).
- Evidence: `item4_close_choices` (acceptance_tests.rs:618). It opens three
  tabs.
  - **Cancel** (Ctrl+W on modified pear): the dialog opens (also the positive
    control for the no-dialog detector). Cancel closes the dialog; the tab is
    kept and still modified, the editor text is unchanged, and the file on
    disk is unchanged.
  - **Discard** (clicked File → Close tab): the dialog opens, the tab is
    gone, and the file still holds the original text.
  - **Save** (Ctrl+W on modified mango): the file holds
    "mango text\nsaved on close" and the tab is gone.
  - **Unmodified tab:** Ctrl+W closes it with no dialog visible in the
    snapshot. The status line becomes empty and the file is unchanged. The
    clicked File → Close tab on an unmodified tab also closes with no dialog.
  - **Failed Save on close** (folder removed): the notification count rises
    by 1, the tab stays as `apple.txt *` with its text, and the dialog stays
    open. Cancel then keeps the tab.

### 5. CPU capture PNG: **met**
- Source: the tester's `capture` uses `CpuRenderer::new().serve_captures`
  and `encode_png` (main.rs:499–512). The submitted `work/notes-editor.png`
  shows tabs `Alpha.txt *` and `Beta.txt`, and the test regenerated it
  byte-identically.
- Evidence: `item5_cpu_capture_two_tabs_one_modified`
  (acceptance_tests.rs:707). The CPU renderer served the capture, which is
  1000×700. The verifier viewed `acceptance-two-tabs.png`: tabs `ideas.txt`
  and `plan.txt *`, with the typed two-line text and the status
  "plan.txt: 2 lines, 8 words".

## Feature-list observations outside the numbered items
- `feature_divider_drag_resizes_sidebar` (acceptance_tests.rs:723): a pointer
  drag on the split divider widens the sidebar from 220 to 340 px.
- Sorting is Rust byte order (`names.sort()`, main.rs:77), so mixed-case
  names would sort uppercase first. The verifier's folder used lowercase
  names, and "alphabetically" does not specify case handling. This is
  recorded as an observation, not a defect.
- After a failed Save on close, the confirmation dialog stays open; the task
  only requires the tab to stay open.
- File I/O is synchronous on the UI thread, which the task does not
  prohibit.

## Usability observation (separate from grading)
- Message discoverability: on 0.3.75, toast inspection exposes only the
  count (Number value) under the Notifications role, not the message text.
  The tester reported this in `FRICTION.md` and the final message. The text
  is visible only in a render (`acceptance-save-failure.png`).

## Tester's own checks
`tests::acceptance_through_ui` (main.rs:520–668) passes and drives Driver
input and inspection. However:
- It edits with `replace_text` (select-all + text_input), not typed text
  with Enter line breaks.
- It reaches menu items by F10 and arrow-key navigation, not pointer clicks
  in the menu bar.
- It keys the sidebar with `ui.focus` instead of Tab.

The verifier's checks above supply this missing evidence. Passing original
tests alone was not counted as acceptance.

## Library vs application defects
- Application defects: none found.
- Library defects: none found. The baseline limitation that toast messages
  are absent from inspection is recorded as a usability observation, per the
  frozen interpretation.
- Verifier-test errors: listed under "Build and test commands" and all
  resolved. There is no unresolved missing evidence.

## Aggregate
Items 1–5 are all met, so `acceptance=met`. The session end time is
recorded in `conditions.env`, so `accepted=2026-10-28T17:10:00+01:00`.
No native execution or human usability is claimed.
