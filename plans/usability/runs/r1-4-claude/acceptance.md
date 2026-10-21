# Acceptance: r1-4-claude (task 4, testing an existing app)

Author check, 2026-10-21. The submitted `work/` (without `target/`, `.claude/`
and `.git/`) was copied to `~/pert-ui-usability/acceptance-r1-4-claude`. It was
built with `cargo build --all-targets` and tested with `cargo test` in the kit
sandbox. It builds without warnings, and all 8 tests pass: 7 in
`tests/behaviors.rs` and 1 in `tests/capture.rs`. No author test was needed,
because every README behavior has a tester test that drives it through
simulated input and reads the result from inspection. The author reproduced two
further deliberate breaks of their own (see below).

Session: brief 15:56:04, ended 16:00:33 (about 4.5 minutes).

## Application diff

`diff -r --exclude target --exclude tests --exclude Cargo.lock` between
`plans/usability/task4-app` and the submitted `work/bookmarks` shows only
`bookmarks/.claude/`. That is an empty `.cc-writes` directory left by the CLI,
not part of the app. `src/`, `Cargo.toml` and `README.md` are byte-identical to
the original. The only other additions are the generated `Cargo.lock` and
`tests/`. The friction log says so too ("No change to the application was
needed"). Behavior is unchanged.

## Items

| Item | Result | Code / check |
|---|---|---|
| 1. One test per README behavior, all passing | met | `tests/behaviors.rs`, one `#[test]` per behavior (mapping below). Each one acts only through `Runtime::pointer_input` (Move/Press/Release at the centre of `bounds`), `text_input`, `command_key_input` with `native_modifiers`, `focus`/`clear_focus`. Each one reads only through `inspect()` semantics (label `name`, field `SemanticValue::Text`), `find`, `children` and `key` (`tests/support/mod.rs`). The only app function called is the documented entry point `bookmarks::build`. 7/7 pass against the unchanged app in the acceptance sandbox. |
| 2. CPU render of the main window writes a PNG | met | `tests/capture.rs:11` `main_window_png_capture`: `set_capture_support`, `request_capture(CaptureTarget::window(primary_window))`, `flush`, `CpuRenderer::serve_captures`, `encode_png`. Writes `target/tmp/bookmarks-main-window.png` (800×900). The author opened it and it shows the real window state: fields, the URL error line, the filter, "3 bookmarks" and three rows with Remove buttons. |
| 3. A deliberate break makes a test fail, recorded | met | FRICTION.md "(acceptance 3)". Break 1 changed `src/lib.rs:94` to stop trimming, and only `add_appends_trimmed_bookmark` failed at `behaviors.rs:27`. Break 2 changed the chord to `primary('k')`, and only `clear_filter_shortcut` failed at `behaviors.rs:200`. Both failures and the restoration (`diff` empty, 8 pass) appear verbatim in the transcript tool output (13:59:51 and 13:59:58 UTC). The author reproduced two breaks independently (below). |
| Any change to the app listed and behavior-neutral | met | No change (see the application diff above). |

Task-specific checks from `acceptance.md`: there is one test per README
behavior, and each one drives input or reads inspection. All pass against the
unchanged app, a CPU capture PNG is written, and the recorded deliberate break
made a test fail. The app is unchanged. All met.

## Behavior-to-test mapping (`tests/behaviors.rs`)

| README behavior | Test | What it drives / checks |
|---|---|---|
| 1. Add (append at end, trim name) | `add_appends_trimmed_bookmark` (l. 17) | Types `"  Docs  "` + https and `Example` + http, clicks Add. The rows read `Docs` (trimmed) and `Example` appended after the two initial rows, row keys `1..4`, error empty. |
| 2. Refused add | `refused_add_shows_message_and_keeps_fields` (l. 35) | Empty name with a bad URL shows the name message first. A spaces-only name is refused. `ftp://`, no scheme, `https:/` and an empty URL each show the URL message. Nothing is added and both fields keep their text. The next valid add clears the message. |
| 3. Fields cleared | `successful_add_clears_fields` (l. 79) | Both fields hold text before Add and are empty after it. |
| 4. List and Remove | `list_rows_and_remove` (l. 94) | Every row has name/url labels and a `Remove` button (role and name from inspection). Clicking Remove on row 1, then row 3, removes only that bookmark, including when two bookmarks share the name "Rust". |
| 5. Filter | `filter_by_name` (l. 128) | Checks matching that ignores case, ignores surrounding spaces and finds substrings inside the name. A no-match filter shows no rows. Blank and empty filters show all rows. An add while filtering shows the new row only if it matches. |
| 6. Status | `status_counts` (l. 164) | `2 bookmarks`, `1 bookmarks` after a remove, `3 bookmarks`, then `1 of 3`, `3 of 3`, `0 of 3` while filtering, and `3 bookmarks` for a blank filter. |
| 7. Clear filter (Ctrl+L) | `clear_filter_shortcut` (l. 190) | `Modifiers::PRIMARY` + `KeyL` through `command_key_input`, with focus in the filter, in the Name field (Name keeps its text), on the Add button, and with no focus. The filter is emptied each time and rows and status are restored. |

## Author's own deliberate breaks (acceptance copy, restored afterwards)

- `src/lib.rs:99` with `url.set(String::new())?` commented out: only
  `successful_add_clears_fields` failed (`behaviors.rs:88`, left
  `"https://docs.rs"`, right `""`).
- `src/lib.rs:50` with `bookmark.name.contains(&filter)` (filtering no longer
  ignores case): `filter_by_name` (l. 137), `clear_filter_shortcut` (l. 198)
  and `status_counts` (l. 181) failed. The status line and the shortcut test
  both depend on filtering, so their failures are legitimate.
- After both breaks, `src/` was copied back from the original, and `diff`
  confirmed it is identical.

## Triage notes (not acceptance failures)

- **Finding the facilities.** The tester converted the rustdoc HTML to text
  with pandoc and read the `guide/getting_started` and `guide/diagnostics`
  chapters first, then `Runtime`, `ComponentInspection`, `Semantics`,
  `SemanticValue`, `SemanticAction`, `PointerEvent`, `Button`, `TextField`,
  `Each`, `Label`, `CommandKey`, `CpuRenderer` and `CaptureTarget`. The tester
  credits the diagnostics chapter as being close to a recipe. Its keyboard
  section explained why to use `command_key_input`, and its screenshot example
  ran unmodified.
- **No "perform action" on semantics.** The tester looked for an
  `Activate`/invoke on inspection. The `SemanticAction` docs say inspection is
  read-only, so the tester switched to pointer input at `bounds`. The tester
  calls the answer clear, but it cost one more hop.
- **Hand-built test driver (`tests/support/mod.rs`).** The tester wrote these
  helpers:
  - click at the centre of `bounds` (reveal, then Move/Press/Release);
  - replace a field's text (click, Ctrl+A, Backspace, then `text_input`);
  - a `chord` helper (`native_modifiers` plus a press and release through
    `command_key_input`);
  - read a label's text from the semantic `name`;
  - read a field's text from `SemanticValue::Text`;
  - list row keys through `children`/`key`.

  The tester's top request is a test-driver facade (`click`, `set_text`,
  `press_chord`) or a "Testing an application" guide section.
- **Runtime surprise.** `text_input("")` over a selection is a no-op, so the
  first version of the field-replace helper left text behind. The fix sends
  Backspace after Ctrl+A. The tester also notes that `focus` places the caret
  at the start, so a plain `text_input` prepends. Neither behavior is stated
  on `Runtime::text_input`.
- **Inspection addressing.** The README's `list/1/name` is actually
  `list/items/1/name`, because `Each` adds an `items` container and replaces
  the row's own key. This is documented on `Each`, and the tester found it by
  dumping `inspect().components` in a throwaway test, which was later deleted.
  A label's text is its semantic `name`, not its `value`, and an empty label
  reports `name: None`. `Semantics::value` says "Presented or edited text",
  which pointed the tester to the wrong field first.
- **Docs remarks.** The tester flagged two documentation problems:
  - `guide/getting-started.md:9` still says "Until a registry release exists,
    use a local checkout". In the kit the crate resolves from a registry, so
    the tester saw this as stale. It is accurate for the unpublished library,
    but it conflicts with the kit's registry setup.
  - The item examples use `use pert_ui_runtime::…` instead of the facade
    `pert_ui::…`.
- **Workarounds and warnings.** No *workaround* entries. No compiler
  diagnostics during the session apart from the test failures above. The
  acceptance build had no warnings.
- **Source reading.** None. The transcript mentions `/kit/vendor` only in an
  initial `ls /kit /kit/vendor` (a directory listing) and in the tester's
  statement that it never read the source. There are no
  `/kit/vendor/pert-ui*/src` or registry `src` paths. Every `.rs:line`
  location in the transcript is in the tester's own `tests/behaviors.rs`. No
  *source* entry is needed.

## FRICTION.md times against conditions.env

`brief_delivered=15:56:04` and `ended=16:00:33`. The final entry times (15:57
×3, 15:59 ×6) all fall inside this window. The tester first wrote estimated
times: 15:52 to 15:56 for the first batch, written at 15:57:36, and 16:03 to
16:07 for the second batch, written at 15:59:21. 15:52 was before the brief,
and 16:03 to 16:07 were after the actual shell time. When `date` printed
15:59, the tester noticed the mismatch and rewrote the times with `sed` to the
shell readings (transcript 13:59:27 UTC). The entries were written in batches,
not as each event happened. As a result, the "understand" entry about the
inspection dump is stamped 15:59, but its event happened at about 15:57:45.
The last notes were written at 16:00:20 and stamped 15:59, which matches when
their events happened (15:59:40 to 15:59:58). The template's instruction header
and placeholder entry were left in the file. These are minor protocol
deviations. They are worth noting for the time analysis but do not affect
acceptance.
