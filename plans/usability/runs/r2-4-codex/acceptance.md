# Acceptance: r2-4-codex (task 4, testing an existing app)

Author check, 2026-10-27. The submitted `work/` (without `target/`, `.claude/`
and `.git/`) was copied to `~/pert-ui-usability/acceptance-r2-4-codex`, built
with `cargo build --offline --all-targets` and tested with `cargo test
--offline` in the kit-aca8df3 sandbox (pert-ui 0.3.75). It builds without
warnings, also after touching `src/lib.rs` and the tests to force a rebuild,
and all 8 tests in `tests/ui.rs` pass: 7 behavior tests and 1 capture test.
The capture test rewrote `captures/main-window.png`, and the result is
byte-identical to the submitted file. No author test was needed: every README
behavior has a tester test that drives it through simulated input and reads
the result from inspection. The author made five deliberate breaks of their
own (below).

Session: brief 19:04:32, ended 19:07:37 (about 3.1 minutes).

## Application diff

The author compared the fixture `plans/usability/task4-app` at `aca8df3`
(extracted with `git archive`) with the submitted `work/bookmarks`, using
`diff -r --exclude target --exclude tests --exclude Cargo.lock --exclude
captures --exclude test-evidence`. `src/lib.rs`, `src/main.rs` and
`Cargo.toml` are byte-identical. `Cargo.toml` already reads `pert-ui = {
version = "0.3.75", features = ["cpu"] }`. The only change is a new
`## Headless tests` section appended to `README.md` (13 lines). It says how to
run the tests, where the capture goes and where the mutation evidence is.
That is documentation and does not change behavior. FRICTION.md says "No
persistent application behavior or testability changes" but does not mention
the README addition. The additions are `tests/ui.rs`,
`captures/main-window.png`, `test-evidence/trim-mutation.txt` and the
generated `Cargo.lock`.

## Items

| Item | Result | Code | Check |
|---|---|---|---|
| 1. One test per README behavior, all passing | met | `tests/ui.rs`, one `#[test]` per behavior (mapping below). The only app function called is `bookmarks::build` (`:16-22`). Input goes through `testing::Driver`: `replace_text`, `click` preceded by `advance(ClickCounter::DEFAULT_INTERVAL + 1 ms)`, and `chord(KeyChord::primary('l'), KeyCode::KeyL)`, plus `Runtime::focus` and `clear_focus`. Reads go through `Driver::semantics`: the role, the `name` for labels and buttons, and `SemanticValue::Text` for fields. Rows are read in display order through `Runtime::children` of the `Each`'s documented internal `items` column (`:76-88`). | All 7 pass against the unchanged app in the acceptance sandbox. |
| 2. CPU render of the main window writes a PNG | met | `tests/ui.rs:236-260`: `set_capture_support(true)`, `request_capture(CaptureRequest::new(CaptureTarget::window(primary_window)))`, `settle`, `CpuRenderer::new().serve_captures`, `pending.wait()`, size assertion (900 x 700), not-blank assertion, `encode_png` with a signature check, then a write to `captures/main-window.png` that is read back | The test passes. The author opened the PNG. It shows the real window: Name and URL side by side (URL with its `https://` placeholder), Add, the Filter field, "2 bookmarks", and the Rust and Crates rows with Remove buttons. |
| 3. A deliberate break makes a test fail, recorded | met | FRICTION.md entry 19:06 "(intentional acceptance check)" and `test-evidence/trim-mutation.txt` record one break: `entered_name.to_owned()` without trimming. With it, `add_appends_and_trims_the_name` fails at `tests/ui.rs:91:9` with left `"  Guide  "` and right `"Guide"`, exit status 101. The source was restored. | The author reproduced that break with the same failure (`ui.rs:91:9`), plus four more (below). The submitted `src/` is identical to the fixture. |
| Any change to the app listed and behavior-neutral | met | Only the README section described above. It is documentation, not a change to the app or its testability. | Diff above. |

Task-specific checks from `acceptance.md`: every README behavior has one
test, and each test drives input and reads inspection. All tests pass
against the unchanged app. A CPU capture PNG is written. The recorded
deliberate break made a test fail. The application's code and manifest are
unchanged, and the only change is a README addition that does not change
behavior. All are met.

## Behavior-to-test mapping (`tests/ui.rs`)

| README behavior | Test | What it drives / checks |
|---|---|---|
| 1. Add | `add_appends_and_trims_the_name` (l. 102) | Adds `"  Guide  "` (https) and `HTTP` (http). The rows in display order are `1, 2, 3, 4`. Each row's name, URL and Remove button (role Button) are read, and `Guide` is trimmed. |
| 2. Refused add | `refused_add_keeps_fields_and_success_clears_the_error` (l. 114) | Four refused cases, each checking the error text, that the rows are unchanged and that both fields keep their text: an empty name with a bad URL (name checked first), a spaces-only name, an `ftp://` URL and an empty URL. A valid add then adds row 3 and the error is empty. |
| 3. Fields cleared | `successful_add_clears_both_fields` (l. 139) | After the add, row 3 exists and both field values are empty. |
| 4. List and Remove | `list_shows_each_bookmark_and_remove_deletes_only_its_row` (l. 149) | Checks each row's name, URL and Remove button. A second "Rust" is added. Remove on row 1 leaves `2, 3`, and Remove on row 3 leaves `2`. Removing one of the two same-named bookmarks removes only that one. |
| 5. Filter | `filter_matches_substrings_ignoring_case_and_surrounding_spaces` (l. 166) | `"  uS  "` shows row 1, so case and surrounding spaces are ignored in a substring match. `rA` shows row 2. A no-match filter shows nothing. A blank or empty filter shows both rows. |
| 6. Status | `status_tracks_visible_and_total_counts` (l. 182) | The status reads `2 bookmarks`, `1 of 2 bookmarks`, `2 of 3 bookmarks` after an add while filtering, `0 of 3 bookmarks`, `3 bookmarks` for a blank filter, `1 bookmarks` and `0 bookmarks`. |
| 7. Clear filter | `clear_filter_keyboard_command_works_wherever_focus_is` (l. 202) | Ctrl+L as a chord with nothing focused (`clear_focus`) and with focus in filter, Name, URL, Add or a row's Remove. Each time the filter empties, both rows return and the status reads `2 bookmarks`. |

## Author's own deliberate breaks

`~/pert-ui-usability/acceptance-r2-4-codex/author/breaks.sh` applied each
break to `src/lib.rs` in the acceptance copy, ran `cargo test
--no-fail-fast`, then restored the file (`cmp` confirmed it was identical).

- A, name not trimmed: `add_appends_and_trims_the_name` fails at `ui.rs:91`.
- B, the `url.set(String::new())?` line deleted: `successful_add_clears_both_fields`
  fails at `:144`.
- C, filter text lowercased but the name not lowercased:
  `status_tracks_visible_and_total_counts` fails at `:186` and
  `clear_filter_keyboard_command_works_wherever_focus_is` fails at `:215`.
  The filter test itself passes, because its mixed-case inputs `uS` and `rA`
  lowercase to substrings that are already lowercase in "Rust" and "Crates".
  The suite still catches the break.
- C2, fully case-sensitive filter: `filter_matches_substrings_ignoring_case_and_surrounding_spaces`
  fails at `:176`, and the status and clear-filter tests fail too.
- D, `KeyChord::primary('k')`: `clear_filter_keyboard_command_works_wherever_focus_is`
  fails at `:229`.

## Triage notes (not acceptance failures)

- **Rows in display order.** `QueryResult::components` is documented to be in
  incarnation order. The tester read the order through `Runtime::children` of
  the `Each`'s internal `items` column instead, which the `Each` docs
  document. The test asserts that this column exists and has the key `items`
  (`tests/ui.rs:76-88`). The tester asks for an ordered-row helper on
  `Driver`.
- **An empty label has no semantic name.** The first run of the refused-add
  test failed because the empty `error` label's name is `None`, not `Some("")`.
  The `Semantics` docs describe this normalization, but the testing guide's
  static-text example does not mention it. The tester asks for an
  empty-label example in the guide.
- **`SemanticRole` is not in `testing::*`.** `pert_ui::testing` re-exports
  `SemanticValue` but not `SemanticRole` (`crates/pert-ui/src/lib.rs:270-274`
  at aca8df3). The compiler suggested `SemanticValue::Button` for the missing
  `SemanticRole::Button` (E0433), which did not help. The tester imported
  `pert_ui::SemanticRole` explicitly.
- **`clear_focus()?`.** `HostIntegration::clear_focus` returns `()`, so
  `clear_focus()?` failed with E0277. The compiler diagnostic made the fix
  clear.
- **The README addition** is described under the application diff. It is
  harmless, but it is a change to the app's directory that FRICTION.md does
  not list.
- **Time plausibility.** All FRICTION.md entries (19:05 to 19:06) lie between
  the brief (19:04:32) and the end of the session (19:07:37). FRICTION.md says
  no library source was read. The transcript was not used for this
  acceptance.
