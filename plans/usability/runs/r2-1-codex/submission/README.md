# Photo export settings

A settings editor built with pert-ui 0.3.75 and its default features.

Run on a desktop with `cargo run --offline -- settings.json`. The optional first
argument selects the JSON file; the default is `settings.json` in the working
directory. Missing files use defaults. Load/decode errors are shown in the UI.

Text and number fields commit on Enter or focus loss. Rejected drafts remain
editable with an explanation below the field; Escape restores the valid value.
Apply writes JSON on a background thread. Controls are locked during that save.
Revert restores the applied settings and discards retained editor drafts.

Run `cargo test --offline` without a display. The checks use App::headless,
Driver input, widget inspection, and the native close policy. Test fixtures and
saved output are kept under `test-output/`.

See FRICTION.md for the documentation/testing experience.
