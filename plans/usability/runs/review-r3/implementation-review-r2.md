**Verdict: approved** for the first repair checkpoint (0.3.89: exposed parts, item geometry, the component export and the related doc repairs). All three blocking findings from review r1 are fixed in the current source, with evidence I read myself. What remains are small non-blocking points and evidence limits, listed below. I edited nothing, ran nothing and didn't delegate.

## The three blocking findings

**1. Native tab step: fixed.**
- `apps/gallery/src/interaction_check.rs:830-848` now opens `tool-windows`, which sits under `layout-category`. It asserts that item 1 is not selected, clicks it, then checks that item 1 is selected.
- `apps/gallery/src/gallery.rs:244-273` confirms item 1 is `controls-category`.
- `review-recheck/r3-native-pointer-checks-review2.log` passes the step on both wgpu and CPU (34 steps each). It records Fedora 44 on Wayland and lists the checks it skipped. No GPU adapter was reported.
- The negative control (`native-tab-no-input`) replaced the click with `Ok(())` and failed with "the Controls category title was not selected". Its runner restores the original bytes and asserts the restore; the restored hash equals the original. It ran through the headless gallery test, which uses the same step code.

**2. Idle checks: fixed.**
- `crates/pert-ui/tests/foundation/exposed_parts.rs:121-138` asserts, right after each reconcile (rename and removal) and before `quiet_for`:
  - `dirty == None` for the parent and the child (`assert_clean`, `:13-24`),
  - `!needs_paint()`,
  - `next_deadline() == None`.
- The negative control re-added parent layout invalidation in the name-only branch at `tree.rs:650-652`. It failed with `root was invalidated: Some(Layout)`, and the source was restored to its original hash.
- `crates/pert-ui/tests/surfaces/pointer_items.rs:136-150` paints, inspects three times, then checks the Tabs `dirty` value, `needs_paint` and the deadline before `quiet_for`. This one has no negative control, but reading it, the assertions are correctly placed before any flush.

**3. Menu, MenuBar and ContextMenu coverage: fixed.**
- **Menu** (`pointer_items.rs:400-449`): for both the plain and the icon menu, the `trigger` part reports `SemanticRole::Button` with its name. This does tell the inner button apart from its wrapper, because the Tooltip reports `SemanticRole::Tooltip` (`popover.rs:499`). The test clicks the trigger and a row, checks the command ran, and checks the list refuses with `NotRevealed` both before opening and after the command closes it.
- **MenuBar** (`:451-507`): File and Edit titles, with `run` keys at the File level, in a nested Tools submenu and under Edit. The counts 10, 11 and 111 show each scope runs only its own command.
- **ContextMenu** (`:509-553`): a real secondary press and release on the content, then a row click, then `NotRevealed` once it has closed.
- **Doctests:** the `Menu` doctest (`menu.rs:702-724`) and the ContextMenu "Inspection and tests" doctest (`menu.rs:1321-1344`) both click through the public parts. The ContextMenu one opens with the ContextMenu key; its first doctest (`:1281`) opens with a secondary pointer press. Both pass in the in-repo and external runtime doctest logs.

## Should-fix items from r1

All addressed:
- **Error variants:**
  - `exposed_parts.rs:246` now matches `OtherWindow { key }`.
  - `pointer_items.rs:56`, `:86` and the new tests match `NotRevealed`.
- **Editor `clipboard` part:**
  - Documented on TextField and TextArea (`text_fields.rs:212-219`, `:274-281`), SearchField (`search.rs:111-116`) and CommandPalette (`command_palette.rs:511-513`).
  - Tested in `exposed_parts.rs:383-412`, which also looks up `items` below `clipboard`.
- **Workspace part/slot forwarding:** tested in `exposed_parts.rs:343-381`.
- **Custom-widgets guide:** `custom-widgets.md:49-55` now introduces the contract and why the nested Select is exposed. The pointer-precision paragraph has its own heading at `:95`.
- **Narrow strip:** `pointer_items.rs:192-210` asserts the actual strip width is 96 px and sets control height to 64. Each arrow is that wide (`tabs.rs:811`, `:863-869`), so the two arrows exceed the strip and the title space is 0. The earlier 8-px failure is kept in `r3-narrow-strip-extent-check.log`.
- **Nits:**
  - ContextMenu's `items` is in the `testing.md:122` table.
  - The Tabs doctest no longer imports `windowing::*`.
  - Both design status lines are updated.
  - CHANGELOG line 28 now says "Breaking inspection format", and the extra blank line is gone.
- **Tab identity guard:** still in place at `tabs.rs:1893`. The recreation test at `pointer_items.rs:180-191` flushes before re-adding the tab, so it shows the new incarnation is targeted safely after a flush. Nothing claims it detects deletion of the guard, and the CHANGELOG ("recreated keys") doesn't overclaim.
- **Component export:** `ApplicationError` is in `component` (runtime `lib.rs:615`) and is tested with only the component glob. `ExposedPart` is in the facade's testing re-exports (`lib.rs:359`) and appears in the API listings.

## Non-blocking findings

1. **Two error checks are still imprecise.** `pointer_items.rs:247` and `:292` use `click_item(...).is_err()` after a Select commits and closes. This is the same weakness as r1 #5: a failure for the wrong reason would pass. They should match `TestError::NotRevealed { .. }`.
2. **The recheck logs don't record their commands.** Unlike `workspace-and-serde/outcomes.json`, the `review-recheck/` logs don't say which flags were used (for example `--all-targets`, `-D warnings`) or which targets were checked. The outputs show no warnings, but Clippy coverage of the new and changed test files is taken from the README's description, not the logs.

## Scope and evidence limits

- **What this approval covers:** this checkpoint only. It does not cover:
  - the literal-tab presentation proposal,
  - the disabled-button proposal,
  - independent acceptance,
  - the full recurrence matrices,
  - the frozen-head comparison both designs require before closure,
  - overall completion of round 3 or of usability.
- **Gate timing:** the broad gates ran 18:00–18:06, plus workspace tests attempt 3, on the source *before* these repairs. Afterwards only focused checks ran:
  - parts tests (11), pointer tests (11),
  - runtime and facade doctests, rustdoc,
  - facade/gallery Clippy, and the native run.

  The surfaces test count went from 461 to 464 and the foundation count stayed at 494. That fits repairs limited to tests, docs and the gallery, but without git I couldn't confirm that no runtime production code changed after attempt 3.
- **Routing doc link:** the `register_command` → `command_key_input` link was added after the broad gates. The external log shows the shifted doctest lines (580→586, 955→961), and they pass. The link target is public, but I can't order the in-repo rustdoc log relative to that edit.
- **Coverage gaps, all non-blocking:**
  - deliberate corruption of the private header cache, or deletion of the identity guard,
  - reordering tabs under a reused key,
  - the drag-preview skip,
  - OptionList having no bounds before its first layout,
  - fault injection of internal failures,
  - opening an editor's clipboard menu and clicking one of its rows.
- **Native evidence:** Linux/Wayland only, with injected runtime input. No adapter was reported, there was no hand input, and nothing ran on Windows or macOS.
