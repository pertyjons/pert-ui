# Acceptance: r1-1-codex (task 1, settings editor)

Author check, 2026-10-21. The submitted `work/` (without `target/`, `.claude`,
`.git`, `.codex`, `.agents`) was copied to
`~/pert-ui-usability/acceptance-r1-1-codex`, its `test-output/` removed, built
with `cargo build --all-targets` and tested with `cargo test` in the kit
sandbox (offline): builds without warnings; both tester tests
(`tests::loaded_fields_validation_dirty_revert_save_and_close`,
`tests::failed_save_stays_dirty`) pass. The native window was not run (no
display). The tests mount the editor's real `mount` function in a headless
`Runtime`, drive it with `focus`, `text_command(SelectAll)`, `text_input` and
`key_input` (buttons are activated by focus plus Enter, not pointer input) and
read everything back from `Runtime::inspect` (semantics and properties), plus
the JSON file on disk.

The submission also contains `tools/doc.py` (a tester-written HTML-to-text
extractor for the kit's rustdoc pages under `/kit/doc`, used to read the
documentation), `check.log` and `test.log` (the tester's saved `cargo check`
and `cargo test` output) and `test-output/` (files the tests write).

Author test added in the acceptance copy only (appended to `mod tests` in
`src/main.rs`): `author_refusal_messages_visible`. The tester's explanation
assertions (`src/main.rs:475-480`, `486-491`) match substrings ("1–32",
"1 to 100") that the permanent help texts also contain, so they would pass
without any error being shown. The author test reads the `Field`'s inspected
`status` and `message` properties and its semantic description: an invalid
prefix (`bad prefix!`), a 33-character prefix, quality `101` and `0` and width
`15` each set the field status to `error` with the tester's specific message
(e.g. "Use 1–32 letters, digits, - or _ only.", "JPEG quality must be an
integer from 1 to 100."), the control is marked invalid, the inspected value
stays at the last valid one and Apply/Revert stay disabled; a later valid
edit clears the status and makes the editor dirty. It also checks that a
loaded `WebP` format and an enabled toggle are shown. It passes (3 of 3
tests pass in the acceptance copy).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Fields, groups and buttons exist and show loaded values | met | `src/main.rs:29-52` `load`, called at startup from `mount` (`:78`); fields and titled groups `:231-258` (width has `.unit("px")` `:143`) | `src/main.rs:411-471` writes a file, mounts, checks visibility of titles, fields and buttons and every loaded value from inspection (incl. the `px` unit); `:562-563` reloads the saved file; author test checks `WebP` and toggle `true` |
| 2. Invalid prefix and out-of-range quality refused, explanation shown, old value kept | met | `TextField::normalize` / `IntegerInput::normalize` returning `InputRejection` `:111-153`, `on_rejection` writes `FieldStatus::error` into the wrapping `Field` `:105-109`, `:234-252` | tester `:473-501` (refusal, invalid flag, Apply disabled, value kept after Escape); explanation verified by the author test above, since the tester's substring check also matches the help text |
| 3. Apply/Revert disabled when clean, enabled when dirty; Revert restores | met | dirty = `current != applied` `:81`, marker `:82-88`, enabled `:183`, Revert `:217-230` | helper `dirty` `:381-386` reads `Semantics::disabled` of both buttons and the marker from inspection; `:472`, `:509`, `:524-546` restore all five fields |
| 4. Successful save writes the file; failed save reported and stays dirty | met | Apply spawns a `std::thread` worker that writes a temp file and renames it, result returned through `Runtime::mailbox` `:169-210` | `:547-561` file content and clean state; `:566-584` parent path is a regular file, "Save failed:" shown from inspection, state dirty, Revert still works |
| 5. Closing refused while dirty | partly met | `CloseGuard::allow` `:54-70`, used by the `FloatingWindow::on_close` `:269-275` and by `App::on_close` `:298-313` | `:510-523` closes an in-view `FloatingWindow` the tester added around the editor and shows the refusal; the native window's refusal (`App::on_close`) is implemented but no check drives it, although `Runtime::request_close` with the host's presentation requests can be driven headlessly (the Claude run of this task does). The added window also gives a clean close that leaves an empty native window. |

Task-specific checks from `acceptance.md`:

- Out-of-range and malformed input refused through the field itself, last
  valid value applied: met for the prefix (malformed and too long), quality
  (above and below range) and width (author test). A malformed quality
  (`abc`) is refused by `IntegerInput` itself (control invalid, value kept),
  but the field shows only its permanent help "Enter an integer from 1 to
  100" with status `normal`, because `on_rejection` does not report built-in
  parse failures (the tester's friction entry at 15:36). Acceptable for
  item 2, which names an invalid prefix and an out-of-range quality.
- Apply and Revert enabled state from inspection: met (`Semantics::disabled`).
- Save off the UI thread, unwritable path reported: met (worker thread plus
  library mailbox; write under a regular file reported and dirty).
- Close refusal through the library's close request, not a custom button
  only: met through the library's `FloatingWindow` close button and its
  `on_close` admission. The native `App::on_close` path shares the same guard
  but is not exercised; the library offers no headless driver for it
  (`App::handle_close` is crate-private, `Runtime::request_close` only queues
  a request for a native host).
- Loading at startup: met (`mount` loads the file; tests write a file first
  and read the loaded values back).

Notes for the triage (not acceptance failures):

- To make closing testable headlessly, the tester wrapped the whole editor in
  an in-view `FloatingWindow` inside a `WindowLayer` inside the native window
  (`src/main.rs:260-276`). In the native app that window can be moved inside
  the native window, and closing it while clean removes the editor and leaves
  an empty native window running. (Corrected in triage: activating the
  in-view window's close button moves focus to it, which saves a valid
  draft before the guard runs, so that path does consider an uncommitted
  draft; a probe confirmed it.)
- The save uses a hand-built `std::thread` plus `Runtime::mailbox` rather than
  `Runtime::spawn`; `spawn` needs `&mut Runtime`, which the `Button` callback
  (an `EventContext`) does not have.
- Groups are `Label`s with `TextStyle::Heading` in one `Column`, not the
  library's `Group` (titled) or `Form`.
- Range is declared twice for each `IntegerInput` (`range` plus a
  `normalize` repeating the same bounds) only to get a rejection message.
- Dirty tracking is the tester's own pair of signals plus five per-field
  signals mirrored by hand on Revert (`:217-229`).
- `App::on_close` uses `.expect("initialized")` (`:304`), a production
  `expect`; `use pert_ui::{prelude::*, *}` glob-imports the whole facade.
- Malformed numeric input gets no `FieldStatus` message (see above); the
  tester asked for one callback reporting every numeric validation failure.
- The tester read `Label` text from `Semantics::name`, after first trying
  `value` (friction 15:39), and hit `DrawError` not converting into
  `RuntimeError` (friction 15:39, E0277).
- First friction entry: the getting-started guide in the kit tells the reader
  to use a local checkout while the brief gives `pert-ui = "0.3.52"`.
- No library source was read, according to the friction log.

Friction log times: entries at 15:36, 15:36, 15:39, 15:39, 15:41 and 15:41
fall between `brief_delivered=15:35:04` and `ended=15:41:24`; the transcript
shows `date +%H:%M` being run, the two 15:39 entries match the failed test
builds at about 15:39:05 and 15:39:25 in `log/builds.jsonl`, and the two
15:41 entries are explicitly marked as recorded during closing. Plausible.

## Author decision (2026-10-21)

The subagent that ran this check counted item 5 as met. The author reads
it as partly met: the task's window is the application window, whose
refusal is untested, and the in-view window introduced for testing adds a
close path with a user-visible defect. Result: partly met (items 1–4 met,
item 5 partly met); no accepted completion time.
