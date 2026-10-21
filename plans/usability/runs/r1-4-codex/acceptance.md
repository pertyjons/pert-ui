# Acceptance: r1-4-codex (task 4, testing an existing app)

Author check, 2026-10-21. The submitted `work/` (without `target/` and CLI
directories) was copied to `~/pert-ui-usability/acceptance-r1-4-codex`; the
tester's `captures/bookmarks.png` was deleted from the copy first so that a
fresh capture would prove the test writes it. In the kit sandbox `cargo build
--all-targets` builds without warnings (also after touching every source
file) and `cargo test` passes 8 of 8 tests in `tests/ui.rs`, writing a new
900x700 PNG. No author test was needed.

## App changes

`diff -r` against `plans/usability/task4-app` (excluding `target`, `tests`,
`Cargo.lock`):

- `Cargo.toml`: `pert-ui` requirement `0.3.51` changed to `0.3.52`, features
  unchanged (`cpu`). Listed in FRICTION.md (15:53, *understand*) with the
  reason "the docs name 0.3.52". Not behavioral, and not needed: the caret
  requirement `0.3.51` already resolves to the kit's 0.3.52.
- `README.md`: a "Headless tests" section appended (how to run the tests,
  where the capture and the mutation evidence are). Documentation only; not
  listed in FRICTION.md, which says only the dependency changed.
- Added artifacts: `captures/bookmarks.png`, `mutation-failure.txt`,
  `test-output.txt`, and `tests/ui.rs` (the deliverable).
- `src/lib.rs` and `src/main.rs` are byte-identical to the original, also
  after the tester's mutation and restore.

`read_docs.py` and `read_methods.py` in the work root are the tester's own
rustdoc readers: small HTMLParser scripts that print a page's main content
(minus trait implementations) or one method's section from
`/kit/doc/pert_ui`. They are reading aids, not part of the app or tests.

## Items

| Item | Result | Check |
|---|---|---|
| 1. One test per README behavior, all passing | met | seven behavior tests in `tests/ui.rs:139-286` (mapping below); 8/8 pass in the acceptance sandbox |
| Each test drives input or reads inspection | met | input: `Runtime::focus`, `command_key_input` with `native_modifiers(PRIMARY)` (`ui.rs:59-77`), `text_input`, `pointer_input` press/release at the widget's `bounds` centre (`ui.rs:79-96`); reads: `inspect()` semantics matched by `inspection_id` (`ui.rs:35-57`), tree queries `find_descendant`/`children`/`key` (`ui.rs:104-123`). The only app call is the documented entry point `bookmarks::build` (`ui.rs:18`) |
| 2. CPU capture PNG written | met | `cpu_renderer_captures_main_window_as_png` (`ui.rs:288-312`): `set_capture_support`, `request_capture(CaptureTarget::window(primary_window))`, `flush`, `CpuRenderer::serve_captures`, asserts 900x700 and non-flat pixels, writes `captures/bookmarks.png`. Opened: dark theme, Name/URL fields, Add, Filter, "2 bookmarks", Rust and Crates rows with Remove buttons |
| 3. Deliberate break made a test fail | met | FRICTION.md 15:55 entries and `mutation-failure.txt`: removing `.trim()` from the added name failed `add_appends_and_trims_name` with exit 101; `log/builds.jsonl` confirms that run (15:55:09-15:55:36, status 101) followed by a passing run. Reproduced by the author (below) |
| Any change listed and not behavioral | met, with a note | the only code-adjacent change (Cargo.toml version) is listed and inert; the README addition is unlisted but documentation only |

## Behavior-to-test mapping

| README behavior | Test (`tests/ui.rs`) | What it does |
|---|---|---|
| 1. Add | `add_appends_and_trims_name` (139) | adds `"  Example  "`/https and `"Plain HTTP"`/http by typing and clicking Add; asserts ids 3 and 4 appended at the end, name trimmed |
| 2. Refused add | `refused_add_preserves_fields_and_recovers` (159) | empty name with bad URL (name checked first), blank name, `ftp://` and empty URL; asserts rows unchanged, `error` text, both field values kept; then a valid add clears `error` |
| 3. Fields cleared | `successful_add_clears_both_fields` (188) | after an add, both field semantic values are empty |
| 4. List | `list_shows_rows_and_remove_deletes_only_its_bookmark` (197) | initial rows' id, name and URL; adds a second "Rust"; clicks Remove on 1, 3, 2, each time asserting the button's role and name "Remove" and that only that row went |
| 5. Filter | `filter_matches_substrings_ignoring_case_and_surrounding_spaces` (222) | `"  uS  "`, `R`, `aTe`, no match, blank, empty |
| 6. Status | `status_counts_visible_and_total_bookmarks` (238) | `2 bookmarks`, `1 of 2`, `1 of 3` after adding under a filter, `0 of 3`, blank filter `3 bookmarks`, `1 bookmarks`, `0 bookmarks` |
| 7. Clear filter | `clear_filter_shortcut_works_wherever_focus_is` (257) | Primary+L through `command_key_input` with no focus and focus on the filter, name and URL fields, Add and a row's Remove; asserts empty filter, both rows and status |

## Author's deliberate breaks

Each in the acceptance copy of `src/lib.rs`, restored afterwards (the file
is again identical to the original and 8/8 pass):

- Name trimming removed (the tester's break): `add_appends_and_trims_name`
  fails with row 3 named `"  Example  "`.
- Clear-filter chord `primary('l')` changed to `primary('k')`:
  `clear_filter_shortcut_works_wherever_focus_is` fails (filter still
  `"rust"`).
- Filter without `trim()`: the filter and status tests fail.
- Remove by name instead of id: the list test fails (removing row 1 also
  removed the added "Rust").

## Triage notes (not acceptance failures)

- Discovery: the tester first searched `/kit/doc` file names for
  `test|inspect|Harness|Capture|Cpu` and guessed `guide/testing/index.html`,
  `struct.TestHarness.html` and `struct.TestDriver.html`, none of which exist;
  it then found `guide/diagnostics` (keyboard input and CPU capture
  examples), listed `Runtime`'s method anchors and read `PointerEvent`,
  `PointerKind`, `ComponentInspection`, `Semantics`, `SemanticValue`,
  `CommandKey`, `CaptureTarget`, `FrameImage`, `Each`, `TextField`, `Button`.
  FRICTION.md 15:53 asks for one end-to-end testing walkthrough.
- Built by hand: a test `App` wrapper with helpers to replace a field's
  text (focus, Primary+A, Backspace, `text_input`), click a widget (focus to
  reveal, then pointer press/release at the bounds centre), read a
  component's semantics by scanning `inspect()` for its `inspection_id`, and
  enumerate rows through `Each`'s default `items` column. These are the
  reusable helpers it asks the library to provide.
- The README's key list says `list` holds one row per bookmark; in the tree
  the rows sit in `Each`'s `items` column. The tester found this in the
  `Each` docs (FRICTION.md 15:54). A fixture README detail for the author.
- No compile diagnostics were experienced: the tests were written in one
  pass at 15:54, three API guesses (`CommandKey::Unidentified`,
  `clear_focus().unwrap()`, `CaptureTarget::window` by value) were corrected
  from the docs while the first build was still compiling dependencies, and
  that first `cargo test` passed. No workarounds logged; no warnings.
- Fixture note: `task4-app/Cargo.toml` still says `0.3.51` while the kit is
  0.3.52, which prompted the tester's (unnecessary) version edit.

## Source reading

`/kit/vendor` appears in the session record only in the brief's rule text
and in `home/.cargo/config.toml` (the vendored registry). No command read a
file under `/kit/vendor/pert-ui*/src` or rustdoc's `/kit/doc/src`; the
"Source" strings in tool output are rustdoc link labels. No *source* entry
was required, consistent with the friction log's "no library source read".

## Friction log times

Entries at 15:53, 15:53, 15:54, 15:55, 15:55, all between
`brief_delivered=15:52:07` and `ended=15:56:19`. They were taken from the
sandbox clock (`date +%H:%M` and `datetime.now()` in the scripts that
appended them) and match the session record's timestamps (13:53:17,
13:54:44, 13:55:08, 13:55:34 UTC).
