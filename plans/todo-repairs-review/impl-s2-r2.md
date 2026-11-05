ready

No remaining findings in the two test-gap fixes.

- **`crates/pert-ui/tests/editing/draft_editor.rs:625–730`** — Resolves the alignment gap. Selection fill, preedit underline and IME anchor must each move between Start and End, with Center halfway between. Leaving any at the leading origin fails. Pointer presses are checked against independently obtained Home/End caret positions; leaving hit testing at the leading origin fails the centered start press.

  The color assumptions match this fixture: `Runtime::new()` defaults to `Palette::dark()`, selection uses `TextSelection`, and preedit uses a one-logical-pixel `Primary` underline. The embedded editor has no competing field frame, and the other buttons are unfocused. No evident timing or color ambiguity was found.

- **`crates/pert-ui/examples/diagram_editor/session.rs:56–83,307–355`** — Resolves the label-identity gap. Exact node-ID sets reject suppressing all labels or suppressing the wrong node. Switching rename from `finish` to `start` verifies restoration of the first label.

  The coordinate comparison is valid: scene composition transforms `DrawMask.rect` into window coordinates, and `Runtime::bounds` returns window-coordinate bounds, including ancestor view transforms. The dedicated `label_ink()` differs from the default editor ink. The fixture’s nodes are separate, with short centered labels, so rectangle-center attribution is appropriate.

These checks cover the described regressions. They do not establish pixel-perfect text placement or support arbitrary overlapping-node attribution, but neither is needed for these fixes.

I did not examine unrelated behavior, untracked plans, or the remaining repair slices. I ran no Cargo commands, tests, headless sessions or native checks; compilation and execution remain unverified. `git show --check 3fb2161` passed. No files were edited.