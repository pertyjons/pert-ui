# Author observations for triage

Observations the author side made while preparing the kit, before any
tester run. They are not tester findings and are not counted as such; slice
3's triage checks whether testers hit the same things and classifies them
with the rest. Testers never see this file.

- 2026-10-21, writing task 4's application: `Runtime::register_command`
  returns a `#[must_use]` `CommandId`, so an application-lifetime command
  needs `let _ =` although dropping the id does not unregister the command.
- 2026-10-21, headless test of task 4's application: reading a component's
  text takes `Runtime::inspection_id`, a linear search of a full
  `Runtime::inspect` snapshot and a string-keyed property map; a test helper
  is close to mandatory.
- 2026-10-21, same test: replacing a text field's content takes a select-all
  chord through `Runtime::command_key_input` with `native_modifiers`, while
  plain keys go through `Runtime::key_input`, so a test author meets both
  entry points.
- 2026-10-21, same test: rows of an `Each` sit under an implicit `items`
  key segment, documented on `Each` but easy to miss when composing a
  `Runtime::find` path.
- 2026-10-21, same application: an empty `Label` still takes vertical space;
  `Show` removes the gap but makes the label unfindable while hidden.
