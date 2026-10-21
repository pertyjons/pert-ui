# Photo export settings

A pert-ui settings editor with validated drafts, dirty tracking, background JSON saves, and close protection.

Run on a desktop with `cargo run --offline -- settings.json` (the path is optional).
Run without a display with `cargo test --offline -- --nocapture`.

The tests mount the same editor in pert-ui's Runtime, send real text/key input,
activate its buttons, inspect visible semantics, and verify saved JSON. Test
artifacts are kept under `test-output/`.

Apply/Revert operate on accepted field values. Text/numeric drafts commit on
Enter or focus loss; refused drafts retain the previous effective value. During
saving, editing and actions are disabled. A failed load is reported and defaults
are used. Saves write a sibling temporary file and rename it over the target.
