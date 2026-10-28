# Follow-up index

Optional and deferred work that the completed plans leave open. Each item
is taken up only when an application needs it. That need must be shown by
a probe or a real consumer. Each item then gets a plan of its own with a
design checkpoint. This index records where an item came from and what
is already decided. It is not a design: decisions that hold stay in
[contracts](../docs/contracts.md), [architecture](../docs/architecture.md)
or the plan that made them. Verified deviations from AGENTS.md or the
design notes belong in [TODO.md](../TODO.md), not here.

Remove an item when a plan takes it up, and link that plan from the item
until then if one is being written.

## Application structure

- **General signal observer outside components.** From the
  [app-author plan](app-author-improvements.md). Declined by the
  [mixer-probe plan](archive/mixer-probe-improvements.md) slice 3 checkpoint and in
  [user-write callbacks](../docs/contracts.md#user-write-callbacks):
  user writes report through the controls' callbacks, application writes
  are known to the code that makes them. It would need ownership,
  event-phase delivery, batching, reentrancy, failure and feedback budgets
  defined first, since writes are forbidden during update, layout, paint,
  inspection and delivery. Reopen only for a need those callbacks cannot
  serve.
- **Writable model projection.** From the
  [diagram-probe plan](archive/diagram-probe-improvements.md): a field of an
  application model as a control binding. It must define validation, write
  revisions, transaction failure and external-write conflicts, and route
  writes through the application rather than treating a document mutation as
  an undo step or an authoritative acceptance. Round 2 of the usability
  study: both task 1 runs bound each control to its own field signal and
  derived the edited record with chained `combine` calls, which the tuple
  form of `ReadSignal::combine` (0.3.85) replaces; neither needed to write
  through a projection. The follow-up stays for a control bound to a field
  of one model signal, with its validation, revision and conflict rules.
- **Default insets.** From the app-author plan slice 4: list rows and a
  bottom status region touch the window edge. It is a visual default
  decision, separate from flex layout.

## Text and editing

- **Initial caret or selection for a `TextField` focused from code.** From
  the diagram-probe plan. Partly answered by `TextField::session` with
  `EditorStart` (table-editing plan, 0.3.39). Still open is whether a
  standalone policy without a session is wanted.
- **Find in an editor buffer** and **a longer editor undo history.** From
  the app-author plan. Large-buffer editing itself is TODO.md TEXT-01.
- **Sessions for more editors.** From the
  [table-editing plan](archive/table-editing-improvements.md) slice 5: `InlineText`,
  the inline number rows, `TextArea` and `SearchField` ignore editor
  sessions today. The inline rows already own a resting and editing policy
  that a later slice can map onto the protocol. Parameter-backed numbers
  ignore sessions too.
  The developer-usability blind tests (F15) supply application evidence for
  `InlineText`: two task-list testers needed to start a row's rename from
  the list and sent synthetic key presses instead. API audit part 6f2 met
  that need with `TextCommand::BeginEdit`, which begins editing a mounted
  label; a session-attached `InlineText` would still need a host that
  describes the label again for each edit.
- **Controlled forms of more controls** (a control built from a value that
  reports a change). From table-editing slice 6. API audit part 6f3 added
  controlled `Checkbox`, `Toggle` and `InlineText` for the developer-usability
  blind tests' per-row state (F16), with the rules in contracts.md's
  "Controlled controls". `RadioGroup`, `Select`, `Slider` and the number
  inputs, and `TextField` have no controlled form; each is considered only
  with its refusal, presentation and callback timing specified, and
  `TextField`'s normalizer contract would then widen as `InlineText`'s did.
  Round 2's evidence: F16 recurred only for display controls, which round 2
  part P6 gave described values (`ProgressBar::new`, `Spinner::new`,
  `Field::status`); no round 2 run needed an editing control without a
  controlled form per row.

- **Refusal for other captioned controls.** From
  [plan-audit-repairs](plan-audit-repairs.md) slice 2: `Label` and the
  components built on it refuse display text they cannot prepare, but
  buttons, chips, links, menu rows and tab and window titles prepare their
  captions themselves and still fail on such text. Extend the refusal to
  them if a consumer binds data to those captions.
- **A sendable font key for styled text.** From the
  [API audit fix plan](usability/api-audit/fix-plan.md) part 5b (decision
  G4-7): `RichText` and `TextSpan` are UI-thread values because a span can
  name a runtime-local `FontId`, so a worker cannot build styled text for
  `Runtime::spawn` to return. A sendable key would change `FontId` and
  `FontManager` identity. No run has shown the need; the restriction is
  documented on both types.

## Collections

- **Table editing extensions.** From table-editing slice 6 and its
  follow-ups:
  - Enter moving down after a confirm.
  - Range selection of cells with multi-cell copy and paste.
  - A fill handle.
  - A row-level edit transaction across several cells.
  - The session protocol in `Tree` rows and `PropertyGrid`.
- **The removed-column rule.** From table-editing slice 6: an open edit
  whose column is removed. Columns cannot be removed from a mounted table
  today, so the rule has no trigger. It gets one, and a test, when columns
  become removable.
- **Large collections.** From the
  [log-explorer plan](archive/log-explorer-improvements.md):
  - An indexed or paged child source for trees with extreme fan-out, with
    incremental flattening.
  - Horizontal virtualization of table cells.
  - An incremental source-update API (append, replace a range) with its
    consistency and invalidation contract.
  - Text selection across collection rows.

## Build and dependencies

- **Duplicate dependency pruning.** From the app-author plan slice 7: two
  `tiny-skia` versions, `thiserror` 1 and 2, winit's Adwaita decorations and
  arboard's X11 fallback, once their cost is measured.
