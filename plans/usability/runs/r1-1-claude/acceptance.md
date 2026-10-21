# Acceptance: r1-1-claude (task 1, settings editor)

Author check, 2026-10-21. The submitted `work/` (without `target/`, `.claude/`
and `.git/`) was copied to `~/pert-ui-usability/acceptance-r1-1-claude`,
built with `cargo build --all-targets` and tested with `cargo test` in the
kit sandbox (offline): builds without errors or warnings, 6 of 6 headless
tests in `tests/headless.rs` pass. The tester's checks drive the editor
through focus, `text_command(SelectAll)`, typed text and Enter/Escape/Space
key input on a plain `Runtime`, read results from `Runtime::inspect`
semantics (values, `invalid`, `disabled`, names, descriptions) and write CPU
captures. The three captures (`loaded.png`, `invalid.png`, `failed-save.png`)
were viewed: groups titled, `px` unit shown, refusal messages under the
fields, dirty marker and save error visible.

Two author checks were added in the acceptance copy only
(`acceptance-r1-1-claude/work/tests/author.rs`, both pass):
`author_save_into_read_only_directory_is_reported_and_stays_dirty` (a save
into a `0555` directory over a `0444` file, guarded against a
permission-bypassing user: reported as "Could not save …: Permission denied",
Apply/Revert stay enabled, marker shown, file unchanged), because the
tester's failure check uses a missing parent directory rather than an
unwritable one; and `author_malformed_quality_is_refused_and_keeps_value`
("abc" and "12.5" in the quality field: marked invalid, value 75 kept,
nothing dirty).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Fields, groups and buttons exist and show values loaded from the file | met | `src/lib.rs:83-96` `load`, `:138-148` signals seeded from the loaded file; groups `:208-241` (`Group::titled`), width unit `:191`; buttons `:244-300` | `tests/headless.rs:106-134` `shows_loaded_values_in_titled_groups` (group names, every field value and label, buttons, from inspection after loading a written file); `:225-228` a fresh runtime loads a saved file |
| 2. Invalid prefix and out-of-range quality refused, explanation shown, old value kept | met | `src/lib.rs:173-175` `TextField::normalize` + `on_rejection` to `FieldStatus::error`; `:178-185` `IntegerInput::range` + a normalizer repeating the range check | `tests/headless.rs:136-174` (`invalid` flag, the message on the `Field`'s description, quality still 75, no dirty marker, Apply disabled, Escape shows the applied prefix); `invalid.png` |
| 3. Apply/Revert disabled when clean, enabled when dirty; Revert restores | met | `src/lib.rs:163-165` dirty derived from edited vs. applied signals, `:273`, `:299` `Button::enabled`; revert `:287-298`; marker `:310` | `tests/headless.rs:176-204` (enabled state read from inspection `Semantics::disabled`; prefix, width and toggle restored) |
| 4. Successful save writes the file; failed save reported, state stays dirty | met | `src/lib.rs:247-270` Apply via `EventContext::defer` → `Runtime::spawn` (worker thread), applied only on `Ok`; `:98-101` `save` | `tests/headless.rs:206-230` (file written, dirty cleared, reloaded); `:232-248` (missing parent directory: reported, Apply enabled, marker shown); author read-only-directory check |
| 5. Closing refused while dirty | partly met | `src/lib.rs:114-131` close policy (`KeepOpen` + notice "apply or revert"); installed by `src/main.rs:19-22` `App::on_close` | `tests/headless.rs:250-283` (`Runtime::request_close` → `take_presentation_requests` → the policy → `complete`; receipt `CloseRejected` while dirty, `Closed` after Revert) |

Task-specific checks from `acceptance.md`:

- Out-of-range and malformed input refused through the field itself, last
  valid value applied: met. The prefix and quality editors refuse their own
  drafts (`normalize` → `InputRejection`); the bound signals keep the last
  valid value (inspection shows 75 and the applied prefix after Escape).
  Unparsable numeric text ("abc", "12.5") is refused with the value kept, but
  explained only by the red outline and the help text "1 to 100", not by an
  error message (author check output; the tester also reports this).
- Apply and Revert enabled state read from inspection: met
  (`Semantics::disabled` in `Harness::enabled`, `tests/headless.rs:89-91`).
- Save off the UI thread: met, through the library's background facility
  `Runtime::spawn` (`src/lib.rs:256-268`), whose work closure runs on a
  spawned thread and whose result is delivered on the UI thread. A write to
  an unwritable path is reported (tester: missing directory; author:
  read-only directory).
- Close refusal through the library's close request: met.
  `Runtime::request_close` creates the request and the receipt is read back;
  the test plays the native host's part by calling the same policy `main`
  installs with `App::on_close`, since the library has no headless runner
  for `App::on_close`. There is no custom close button.
- Loading at startup: met (`build` calls `load`; tested with a written file
  and with a file saved by the app).

Notes for the triage (not acceptance failures):

- The close check is a hand-written headless host: it maps
  `NativeCloseAction::KeepOpen` to `PresentationStatus::CloseRejected` itself
  (`tests/headless.rs:259`), mirroring what the native runner does
  (`crates/pert-ui/src/platform/windows.rs` `apply_close`); the tester had to
  factor the policy into an `Editor` method shared by `App::on_close` and the
  test, and kept the built `Editor` in an `Rc<RefCell<Option<_>>>` in `main`
  to reach it from `on_close` (`src/main.rs:13-23`).
- The quality and width refusal messages need a `normalize` closure that
  repeats `IntegerInput::range`, because the field's own range/parse
  invalidity is not reported through `on_rejection`
  (`src/lib.rs:180-184`, `:192-196`).
- Background work from a button is composed as
  `cx.defer(|ui| ui.spawn(..))` (`src/lib.rs:255-270`); both handles are
  discarded (`let _receipt`, `.map(|_| ())`), the first after an
  `unused_must_use` warning.
- The dirty state is derived by hand from five chained `combine` calls
  (`src/lib.rs:151-163`); `FormStatus` is used only for per-field messages.
- The tests read the refusal message from the `Field` (`prefix-field`)
  rather than from the control, whose description is "Unsaved changes"
  while it holds a refused draft; group keys were renamed after a bare
  `AmbiguousKey` error from `find_descendant`.
- The tests wait for background delivery by polling `flush` with a sleep,
  as the `Runtime::spawn` example does (`tests/headless.rs:94-103`).
- The native binary builds but was not run (no display).

Friction log timing: entries at 15:35, 15:38 (three) and 15:40 (three), all
within the session (brief 15:34:54, ended 15:40:23). The 15:35 entry was
typed by hand at 13:35:40Z (15:35 local), matching the clock. The three
15:38 entries were first written as 15:48, 15:50 and 15:52 (estimates, after
the session's actual end) and corrected to the shell's `date +%H:%M` output
two seconds later; they describe events from roughly 15:36 to 15:38. The
15:40 entries take the time from `date` in the same command. After the
correction the times are plausible.

## Author decision after the Codex review (2026-10-21)

Item 5 is partly met. The close policy checks only committed values: a
valid prefix typed into the field but not yet committed with Enter or by
leaving the field is not counted as unsaved, and the library does not
commit or cancel a focused editor's draft before `App::on_close`. An author
probe in the acceptance copy (`author_close_with_pending_draft`: focus the
prefix, select all, type, then the same host close as the tester's test)
got `Closed` with Apply disabled, so the typed change is lost without a
refusal. The tester's test always pressed Enter before closing. Result:
partly met; no accepted completion time.
