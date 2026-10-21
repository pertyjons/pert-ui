# Bookmarks

A small bookmark list built with `pert-ui`. A form adds bookmarks, a filter
narrows the list by name, a status line counts them and each bookmark has a
Remove button. It starts with two bookmarks:

| Id | Name | URL |
|---|---|---|
| 1 | Rust | `https://www.rust-lang.org` |
| 2 | Crates | `https://crates.io` |

`src/lib.rs` builds the whole interface: `bookmarks::build(&mut Runtime)`
mounts it into a runtime and returns the root component's `WidgetId`.
`src/main.rs` opens the same content in a native window (`cargo run`).

## Widget keys

The root is a scroll area keyed `bookmarks`. Inside it, the column `content`
holds, in order:

- `form`, holding the field `name-field` with the text field `name`, and the
  field `url-field` with the text field `url`;
- the button `add`;
- the label `error`, empty unless an add was refused;
- the field `filter-field` with the text field `filter`;
- the label `status`;
- `list`, one row per shown bookmark. Each row is keyed by the bookmark's id
  (`"1"`, `"2"`, ... ; new bookmarks get the next number) and holds the labels
  `name` and `url` and the button `remove`.

## Behaviors to test

1. **Add.** Pressing Add with a non-empty name and a URL that starts with
   `http://` or `https://` appends a bookmark at the end of the list. Leading
   and trailing spaces are trimmed from the name.
2. **Refused add.** Add refuses a name that is empty or only spaces, and a
   URL that does not start with `http://` or `https://`. Nothing is added,
   the `error` label shows `Enter a name.` or `The URL must start with http://
   or https://.` (the name is checked first), and both fields keep their text.
   The next successful add clears the message.
3. **Fields cleared.** After a successful add, both the Name and the URL
   fields are empty.
4. **List.** Every bookmark is shown as a row with its name, its URL and a
   Remove button; pressing Remove deletes that bookmark only.
5. **Filter.** Text in the filter field hides every bookmark whose name does
   not contain it, ignoring case and surrounding spaces. An empty filter shows
   all bookmarks.
6. **Status.** The `status` label reads `N bookmarks` (for example
   `2 bookmarks`, also for one) while the filter is empty, and `N of M
   bookmarks` while filtering, where N is the number shown and M the total.
7. **Clear filter.** The keyboard command "Clear filter", Ctrl+L (Cmd+L on
   macOS), empties the filter field, wherever focus is.

## Headless tests

Run `cargo test --offline` from this directory. `tests/ui.rs` contains one
integration test for each behavior above and a CPU-rendered main-window capture
test. The capture is written to `captures/bookmarks.png` on each run. The tests
use runtime input and inspection and need no display.

The intentional regression check and its failing output are recorded in
`/work/FRICTION.md` and `mutation-failure.txt`; the application source has been
restored. The latest passing run is recorded in `test-output.txt`.
