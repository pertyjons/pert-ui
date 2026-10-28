# Photo export settings

Run on a desktop with `cargo run --offline -- settings.json`. The optional first
argument chooses the settings file; the default is `settings.json` in the current
directory. Missing files use defaults. Invalid or unreadable files show a load
error and defaults.

Run `cargo test --offline` without a display. The tests start the real `App` with
`headless()`, use pert-ui's `Driver` for text, keys and clicks, inspect widget
semantics, and submit close requests through `HeadlessApp::admit_close`.
They cover loaded values, validation, dirty state, Revert, background save,
save failure and retry, and close refusal. Test files live in `test-output/`.

Press Enter or leave a text/number field to commit valid input. A refused draft
stays visible beside an explanation, while the previous valid setting remains
in effect. Unsaved drafts also count as unsaved changes. Apply writes a snapshot
on a worker; edits made during a save remain dirty against that snapshot. Apply
and Revert are temporarily disabled while a save is running.

See `FRICTION.md` for the first-use experience and documentation feedback.
