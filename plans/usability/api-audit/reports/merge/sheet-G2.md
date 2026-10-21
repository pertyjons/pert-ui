# Merge sheet: G2

Inputs: `plans/usability/api-audit/reports/{codex,claude}-G2.md` and
`reports/verification/{codex,claude}-G2.md`. Both audits read commit
`d4ba108`; `crates/` is unchanged at HEAD `245053b`, where the source checks
below were made. Source paths without a crate prefix are under
`crates/pert-ui-runtime/src/`. Neither verification answered the other's
verification (each answered the other's original report), so a correction
made in one verification and not contested in the other is applied here.

## Findings

| # | Rubric item, rule kind | Finding (one or two sentences, naming the items) | Sig/beh | Configs | Severity | Confidence | Fix (breaking?) | Destination | Sources and status | Slice 3 |
|---|---|---|---|---|---|---|---|---|---|---|
| G2-1 | B1, project rule; policy question on scope | No-argument builders set one ordinary boolean with no `bool` setter, although contracts.md:1156-1158 reserve no-argument shortcuts for `disabled()`/`read_only()`: `Button::frameless` (controls.rs:604), `Group::frameless` (views.rs:166), `Field::hide_label` (field_status.rs:493), `InspectionQuery::{parameters, windows, sources, validation, application}` (inspection.rs:246-276), `Semantics::names_children` (diagnostics.rs:631), `PointerEvent::outside` (pointer.rs:189; claude: a weak instance, it marks a synthetic event value). Added by claude in verification, not checked by codex: `RegionLayout::fixed` (region_layout.rs:151), `Divider::vertical` (views.rs:928), `Toolbar::attached_below` (toolbar.rs:479), `Dialog::require_response` (dialog.rs:226), `QuickPick::nonmodal` (quick_pick.rs:920), `CommandPalette::nonmodal` (command_palette.rs:540), `Element::structural` (layout.rs:655), `View::fixed` (views.rs:35), `pert_ui::App::software_rendering` (pert-ui/src/app.rs:412). Policy question: should the rule permit these one-way shortcuts? | sig | all; `App::software_rendering`: cpu, no-default-desktop-cpu | low (both) | codex medium on applicability, high on facts; claude confirms | Document the shortcut convention (nonbreaking), or add `bool` setters (additive) / change signatures (breaking) | TODO.md | codex-B1-1; confirmed by claude (instances extended, siblings `ContentQuery::hidden(bool)`, `CaptureRequest::include_overlay(bool)` take `bool`); claude's added instances `delta-unverified`; status: confirmed, policy | - |
| G2-2 | B1, project rule; policy question (scope of the controller exception) | A same-key redescription keeps the first description's permission and binding for `Table::resize_permission`, `Table::reorder_permission`, `VirtualList::reorder_permission`, `Tree::reorder_permission` and `FileBrowser::folder_filter` (table.rs:1218-1222, 1389-1391, 1429-1431; virtual_list.rs:428-437, 982-985; tree.rs:885-891, 1033-1035; file_browser/view.rs:106; test pert-ui/tests/audit/audit_collection_permissions.rs:685-737), against contracts.md:308-309. Policy question: does contracts.md:169-171 "controller and options" cover permissions? | beh | all | medium (both) | high on behavior (both); exception scope open | Adopt the new permission on compatible redescription (behavior break), or name the exception in contracts "Permissions and availability" | TODO.md | claude-B1-1; confirmed by codex, which corrected its own roles-table disposition ("initial controller/options/callback exceptions apply") to an unresolved exception-scope question; status: confirmed, policy | - |
| G2-3 | B1, project rule; policy question on scope | `FloatingWindow::closable(bool)` and `minimizable(bool)` (windows.rs:327, 333) take a `bool` and replace, while `movable`/`resizable` on the same type take `impl Into<Permission>` and compose (windows.rs:346, 412); `pert_ui::App::resizable(bool)` (pert-ui/src/app.rs:375) likewise. Policy question: are title-button presence and a startup option "capability restrictions"? | sig | all; `App::resizable`: cpu, default, image-formats, images, no-default-desktop-cpu, serde, svg | low (both) | claude medium; codex confirms facts | Take `impl Into<Permission>` (calls compile, `false` then `true` changes behavior), or document them as fixed options; codex: Permission conversion is not automatically the right fix | TODO.md | claude-B1-2; confirmed by codex (title buttons built conditionally at windows.rs:538, 562); status: confirmed, policy | - |
| G2-4 | B1, project rule; policy question on scope | `PickItem::disabled(bool)` (quick_pick.rs:78), `SemanticItem::disabled(bool)` (diagnostics.rs:392), `Semantics::disabled(bool)` (diagnostics.rs:579) and `Semantics::read_only(bool)` (diagnostics.rs:598) take a `bool` under the names contracts.md:1158 reserves for no-argument shortcuts; the `disabled` setters also clear offered actions (diagnostics.rs:394, 581). Policy question: does the rule apply to descriptive data builders? | sig | all | low (both) | high on facts (both); applicability open | Rename (breaking), or record that data descriptions take the state as `bool` (nonbreaking) | TODO.md | claude-B1-3; confirmed by codex (its roles table already called these descriptive booleans); status: confirmed, policy | - |
| G2-5 | B1; claude: project rule, codex: proposal; policy question | Builders that add rather than replace, with no family exception in contracts.md: `FloatingWindow::title_action` (windows.rs:421-426), `ToolWindowLayout::window`/`windows` (tool_windows/mod.rs:788-809), `Semantics::action`/`actions`, `SemanticItem::action` (diagnostics.rs:420, 556, 566); related builders replace (`ToolWindow::actions`, tool_windows/mod.rs:457; `Semantics::items`, diagnostics.rs:571). Policy question: are list-element composition builders ordinary properties, and should contracts name them? | beh | all | low (both) | claude medium; codex confirms accumulation | Name accumulating composition in contracts "Public naming rules" (e.g. `add_*`), or make `title_action` take all elements at once; codex: a type or behavior change can break callers even without a rename | TODO.md | claude-B1-4; codex partly confirmed: accumulation confirmed for every instance, classification as an ordinary-property violation not established (correction applied: rule kind is the policy question); status: confirmed, policy | - |
| G2-6 | B1, consistency proposal | Two-way boolean state bindings have no convention: `Collapsible::bound(Signal<bool>)` with `expanded(bool)` (collapsible.rs:533, 525) against `ToolWindowLayout::names(Signal<bool>)` with `show_names(bool)` (tool_windows/mod.rs:924, 916); `bound` is also the choice-source constructor name. | sig | all | low (both) | high on facts | One convention, e.g. `<property>_binding(Signal<bool>)` (breaking) | TODO.md | claude-B1-5; confirmed by codex (choice-source rules do not prohibit these builders); status: confirmed, policy | - |
| G2-7 | B2, project rule | `CommandId` (runtime/commands.rs:377-380) and `HeldInputId` (runtime/held.rs:5-8) are `#[must_use]`, but discarding them keeps the registration (commands.rs:565; held.rs:115), so application-lifetime registrations need `let _ =` (guide/diagnostics.md:36). | sig | all | low (both) | high (both) | Remove the type-level attribute; keep `Result` handling and the unregister APIs (nonbreaking) | TODO.md | codex-B2-1, claude-B2-3 (identity part), matched in claude's verification; status: both | F19 (extends to `HeldInputId`) |
| G2-8 | B2, project rule; policy question (receipt discard) | `TaskReceipt` (runtime/tasks.rs:27) and `ToastReceipt` (notifications.rs:195) are `#[must_use]` although their failures also propagate (tasks.rs:104-105; notifications.rs:646), and the guide and examples discard them with `let _receipt` (guide/surfaces.md:314; examples/diagram_editor/app.rs:600, 821, 852; examples/dynamic_documents.rs:26). Policy question: keep receipts `must_use` by design, or only where dropping loses the only failure report? | sig | all | low (both) | high on facts; policy open | Remove the attribute where the outcome is also reported, keep it where the receipt is the only report (codex: sound; unconditional removal is not), or document receipts as must_use by design with the `let _ =` idiom (nonbreaking) | TODO.md | claude-B2-3 (receipt part); codex partly confirmed: Task and Toast confirmed; Command, Reveal, ListReveal and Presentation receipts refuted, see D1; status: partly, policy | F19 (adjacent) |
| G2-9 | B2, project rule | Owners whose drop tears down state lack `#[must_use]`: `ParameterHost<T>` (parameter_binding.rs:495; Drop removes the parameter, :626-635), `AnyParameterHost` (parameter_binding/erased.rs:255), `SeriesSource<T>` (live.rs:627) and `Runtime::series_source` (runtime.rs:4075; the source holds the consumer in an `Rc`, live.rs:645-650); siblings `RemoteHost` (remote.rs:420) and `SeriesConsumer` (core live/series.rs:292) carry a reason. | sig | all | medium (both) | high (both) | Add `#[must_use = …]` with a reason; for `SeriesSource` the reason must say dropping the last source (all clones) closes the bridge (correction from codex's verification). Nonbreaking | TODO.md | codex-B2-2 and claude-B2-1 (`ParameterHost`, `AnyParameterHost`): both; `SeriesSource`/`Runtime::series_source` (claude only): confirmed by codex; status: both | - |
| G2-10 | B2, project rule | Signal-creating functions lack `#[must_use]`, so a created and discarded signal goes unnoticed: `Runtime::signal` (runtime.rs:4048), `StateOwner::signal`, `EventContext::signal`, `UpdateContext::signal` (runtime.rs:2604, 2909), `Signal::sibling` (signal.rs:66, 327); comparable controller constructors carry it. | sig | all | low (both) | high (both) | Function-level `#[must_use]`, not type-level (nonbreaking) | TODO.md | claude-B2-2; confirmed by codex; status: confirmed | - |
| G2-11 | B2, project rule | `PresentationRequest` (runtime/native_windows.rs:1011-1015; Drop cancels, :1050-1057) and `PreparedDetach` (native_windows.rs:241-245) carry a bare `#[must_use]`, while comparable tokens give a reason (`DirectoryRequest`, file_browser.rs:196; `RequestTicket`, async_source.rs:28; `Pending`, inspection.rs:565). Correction applied: `PreparedDetach` has no Drop impl; dropping it leaves the movement uncommitted. | sig | all | low (both) | high (both) | Add reasons (nonbreaking) | TODO.md | claude-B2-4; confirmed by codex with the correction; status: confirmed | - |
| G2-12 | B3, project rule (geometry notifications) | `RegionLayout::on_resize` promises that removing the callback drops undelivered changes (region_layout.rs:253-255), but it always installs a forwarding `SplitPane::on_resize` closure (region_layout.rs:363-372), and SplitPane discards a pending report only when the incoming or previous callback is `None` (split.rs:496-500); a report recorded under one callback reaches a callback reinstalled before delivery (contracts.md:355-356). Claude adds, unchecked by codex: a Restore or Redescription recorded while the layout had no callback reaches a callback installed in the same flush. | beh | all | codex medium, claude low; propose low: it needs a callback to appear between the record and its delivery in one flush (a delivery while the slot is empty consumes the report, region_layout.rs:367-369), and it delivers an extra report of the current sizing, not wrong data | high (codex, by source tracing, not executed; claude confirms) | Keep callback absence when building the forwarding splits, or propagate removal to their reports; add a remove-then-reinstall-before-flush test (nonbreaking) | TODO.md | codex-B3-1; confirmed by claude; status: confirmed | - |
| G2-13 | B3, project rule; policy question (lifecycle exception) | Reconciliation publishes signals: `ContextMenu::reconcile` publishes its private `enabled_signal` (menu.rs:1472); numeric reconciliation abandons an active parameter session on rebind or fixed denial (numeric_controls.rs:568, 585; text_fields/number_inputs.rs:1274, 1309), whose `EditSession::drop` cancels and publishes the public `Parameter::snapshot` at once (parameter_binding.rs:902, 678, 178; signal.rs:431), only the terminal event being deferred. Policy question: is immediate lifecycle cleanup an exception to AGENTS.md:132? Codex reads contracts.md:167 ("cancelled at the next safe point") as the cleanup requirement; claude reads it as departed from, since the state is published at once. | beh | all | parameter path medium (both); ContextMenu instance codex medium, claude low; propose low for that instance: its only reader is `ContextMenuOpener::open`, an event-time `get()` (menu.rs:1360-1361), and no component or computation can subscribe to the private field | high on the publication paths (both) | ContextMenu: non-signal shared state; parameters: defer the cancellation's publication, or record a narrow lifecycle exception in contracts. Private storage need not break API; changing snapshot timing needs contract review | TODO.md | codex-B3-2; claude reviewed the ContextMenu instance without a finding, then withdrew that and accepted it as a low instance; parameter path confirmed by claude; status: both (ContextMenu), confirmed (parameters), policy | - |
| G2-14 | B3, project rule; policy question (scope of the controller exception) | Collection callbacks belong to the first description; a same-key redescription ignores new ones: `VirtualList::{on_selection_change, on_activate, on_text_input, reorderable, placement}`, `Tree::{on_selection_change, on_activate, reorderable}`, `Table::{on_selection_change, on_activate, on_sort, cell_editor}` (virtual_list.rs:428-437, 807, 839-841; tree.rs:885-891, 1052-1053; table.rs:1218-1222, 1554-1555, 1579-1581); `Tabs::on_select`, the list's cited model, replaces and removes (tabs.rs:950-953). Policy question: does contracts.md:169-171 "options" cover callbacks? ColorPicker's handler follows redescription (contracts.md:478-480). | beh | all | medium (both) | high on behavior (both); exception scope open | Replace and allow removal on compatible redescription as Tabs does (behavior break), or name the exception in contracts "Component redescription". Codex: contracts.md:457 (user-write) does not govern all these families | TODO.md | claude-B3-1; codex partly confirmed: retention confirmed for all instances, the unambiguous project-rule breach disputed; codex corrected its own disposition to the same scope question; status: confirmed, policy | - |
| G2-15 | B3, project rule (signature shape) | `Button::command`, `Button::command_icon` and `MenuItem::command` take `on_receipt: FnMut(CommandReceipt, &mut EventContext)` (controls.rs:552, 592; menu.rs:404), while every other UI-thread component callback takes `&mut EventContext` first (e.g. breadcrumb.rs:182, quick_pick.rs:891); no reason is documented. | sig | all | low (both) | high (both) | `FnMut(&mut EventContext, CommandReceipt)` (breaking); owned receipt delivery stays | TODO.md | claude-B3-3; confirmed by codex; status: confirmed | - |
| G2-16 | B3, project rule (signature shape) | `Accordion::on_change(FnMut(&mut EventContext, &[String], ChangeOrigin))` (collapsible.rs:1049, delivered at :1249) passes the origin positionally, while `Tabs::on_select` (`&SelectionChange`, tabs.rs:607-609), the collections' `on_selection_change` (`&ListSelectionChange`, list_selection.rs:86) and `ToolWindowLayout::on_change` (`&ToolWindowChange`, tool_windows/mod.rs:218) pass one change struct; no reason is documented. Correction applied: the missing previous state is not a family departure (`ToolWindowChange` has none); adding `previous` is a proposal. | sig | all | low (both) | high (both) | An `ExpansionChange` payload passed by reference (breaking); `previous` as an optional proposal | TODO.md | claude-B3-4; codex partly confirmed (shape confirmed, previous-state part refuted); status: partly | - |
| G2-17 | B3, consistency proposal | The same settled selection notification has two names: `Tabs::on_select` (tabs.rs:607) and `VirtualList`/`Tree`/`Table::on_selection_change` (virtual_list.rs:872, tree.rs:1054, table.rs:1556), whose rustdoc defines its timing by `Tabs::on_select` (virtual_list.rs:834-836). | sig | all | low (both) | high on facts | One name, e.g. `on_selection_change` on Tabs (breaking; no change of timing) | TODO.md | claude-B3-5; confirmed by codex; status: confirmed, policy | - |
| G2-18 | B3, project rule (signature shape); policy question whether match rules are a callback family | `MultiSelect::filter(impl Fn(&str, &Choice<T>) -> bool)` (multi_select.rs:1382) takes a bare closure, while `QuickPick::matcher(PickMatcher)` (quick_pick.rs:926), `CommandPalette::matcher(CommandMatcher)` (command_palette.rs:546) and `CollectionFilter::matcher(FilterMatcher)` (collection_filter.rs:587) take a matcher value with presets; `filter` is also the controllers' query operation (table.rs:707). | sig | all | low (claude) | claude high on facts, medium on applicability | `MultiSelect::matcher(FilterMatcher)` (breaking) | TODO.md | claude-B3-d1 (claude's inventory delta); status: delta-unverified | - |
| G2-19 | B3, project rule (signature shape) | `Runtime::set_wake_delivery(&self, deliver: Box<dyn Fn() -> bool + Send + Sync>)` (runtime.rs:4183) is the only closure-taking public function that requires a `Box`; its sibling `pert_ui_wgpu::Renderer::new` takes `impl Fn(Notice) + Send + Sync + 'static`, and no reason is documented. | sig | all | low (claude) | high (claude) | `impl Fn() -> bool + Send + Sync + 'static`, boxed inside (callers passing a `Box` still compile) | TODO.md | claude-B3-d2 (claude's inventory delta); status: delta-unverified | - |

## Disputed facts

### D1. Do the command, reveal, list-reveal and presentation receipts carry outcomes reported nowhere else? (claude-B2-3, receipt part)

- **Claim (claude):** discarding a receipt is normal, so `CommandReceipt`
  (runtime/commands.rs:1606), `RevealReceipt` (runtime/reveal.rs:132),
  `ListRevealReceipt` (virtual_list.rs:229) and `PresentationReceipt`
  (runtime/native_windows.rs:949) should not be `#[must_use]`, like
  `TaskReceipt` and `ToastReceipt`. Evidence: `ListRevealReceipt`'s rustdoc
  tells callers to discard it explicitly (virtual_list.rs:223-226), and the
  reasons on reveal.rs:132 and tasks.rs:27 are conditional ("retain the
  receipt if the application needs …").
- **Counter (codex):** these four carry outcomes that are not otherwise
  available: command errors are retained, not returned
  (runtime/commands.rs:1655), reveal results are stored (runtime/reveal.rs:680),
  virtual_list.rs:228 calls the list receipt the only report, and
  presentation completion stores its result (runtime/native_windows.rs:1044).
- **What the source shows:**
  - `CommandReceipt`: supports codex. `flush_commands`' rustdoc says "An
    action's own error is retained in its receipt, not returned"
    (runtime/commands.rs:1655-1657); `CommandStatus::Finished` holds the
    `Result` (commands.rs:1589). I found no other report in the code read.
  - `RevealReceipt`: supports codex. `RevealStatus::Finished(Result<…>)`
    (runtime/reveal.rs:115) is stored by `dispatch_reveal_requests`
    (reveal.rs:665-687) and not returned; the type's reason is conditional
    (reveal.rs:132), which is claude's point.
  - `ListRevealReceipt`: the rustdoc states both sides. Discarding "leaves
    the request in place; it only gives up its outcome, so a caller that
    does not need it discards it explicitly" (virtual_list.rs:223-226), and
    the attribute's reason says "the receipt is the only report of whether
    the row was found and mounted" (virtual_list.rs:227).
  - `PresentationReceipt`: supports claude for `App` consumers. `App`
    reports failed presentation through `on_window_error`, "The receipt
    retains the typed failure independently" (crates/pert-ui/src/app.rs:476-477;
    the doctest at app.rs:277-279 discards it with `let _receipt`). For a
    host that completes requests itself (`PresentationRequest::complete`,
    native_windows.rs:1040-1047), the receipt is the only report.
- **What settles it:** the facts above are not in conflict once separated:
  the remaining question is whether "discard is a mistake" (AGENTS.md:181)
  means "loses the only report of an outcome", which is the policy question
  in G2-8. Codex judges claude's conditional fix sound.

### D2. Is the `on_rejection` coverage difference explained by a contract? (claude-B3-2)

- **Claim (claude):** `TextField`/`TextArea::on_rejection` reports length
  limits (text_fields.rs:306-308), the number inputs' `on_rejection` reports
  only the normalizer (number_inputs.rs:2089-2091) while opening "with the
  rules of [`TextField::on_rejection`]" (number_inputs.rs:2086-2087), and no
  contract section explains the split; one name, two coverages in one editor
  family. Severity medium; slice 3 F6.
- **Counter (codex, refuted as a B3 departure):** the difference is
  documented at number_inputs.rs:2086-2095; the shared reporter reports only
  `EntryError::Rejected` (text_fields/normalize.rs:114), number validation
  produces a different error (number_inputs.rs:1039), and length checks
  deliberately produce an `InputRejection` (text_fields.rs:1410). Rubric
  B3's editor clause (rubric.md:221-222) exempts documented differences;
  both callbacks keep the same save-attempt timing.
- **What the source shows:** both sides' citations are accurate.
  - The item rustdoc documents the exclusion (number_inputs.rs:2089-2091),
    and TextField's rustdoc states the matching rule from its side: "Only
    [`InputRejection`]s are reported, not the field's own text checks"
    (text_fields.rs:317-318). `check_length` returns an `InputRejection`
    (text_fields.rs:1410), while number validation returns a
    `NumberInputError` (number_inputs.rs:1039-1045). So both editors report
    only `InputRejection`s; what differs is which built-in check is an
    `InputRejection`. The timing is the same.
  - docs/contracts.md "Text and draft editing" (contracts.md:911-926) and
    "Editor sessions" contain no sentence about `on_rejection` coverage.
- **What settles it:** whether item rustdoc counts as the editor's "own
  documented contract" (rubric.md:221-222) or an exception must cite a
  contract sentence (rubric.md:116-118), and whether the B3 names rule
  ("one name means one timing", rubric.md:225) covers coverage as well as
  timing. The code supports codex on timing and on the difference being
  documented at the item, and claude on there being no contract sentence.
  The behavior itself remains slice 3's F6 either way.

## Withdrawn and refuted parts

- claude's "Reviewed without a finding" for the `ContextMenu` signal
  publication (menu.rs:1472): withdrawn by claude in verification; accepted
  as a low instance of codex-B3-2 (G2-13).
- codex's B1 disposition "No additional verified replacement,
  Permission-composition or revocation departures found": withdrawn by
  codex; it accepts G2-3, G2-4 and G2-6 and narrows G2-5.
- codex's roles-table statement that "initial controller/options/callback
  exceptions apply" to the collections: withdrawn by codex as an
  unqualified disposition; now the exception-scope policy question of G2-2
  and G2-14.
- codex's receipt disposition (no finding): partly withdrawn by codex for
  `TaskReceipt` and `ToastReceipt` (G2-8).
- claude-B2-3, `CommandReceipt`, `RevealReceipt`, `ListRevealReceipt` and
  `PresentationReceipt`: refuted by codex (they carry otherwise unavailable
  outcomes); see D1.
- claude-B3-2 (`on_rejection` coverage): refuted by codex as a B3
  departure (documented editor difference); see D2.
- claude-B3-4, missing previous state as a family departure: refuted by
  codex, since `ToolWindowChange` has no previous state either
  (tool_windows/mod.rs:218-235); not contested.
- claude-B1-4, classification as an ordinary-property project-rule
  violation: not established per codex (collection composition; AGENTS.md
  "accumulating spans" does not settle it); kept as a policy question.
- claude-B3-1, reliance on the user-write rule (contracts.md:457): codex
  notes it does not govern all the listed callback families; the finding
  rests on contracts.md:164 and the exception scope.
- claude-B2-4, `PreparedDetach` as a drop-cancelling token: corrected by
  codex; it has no Drop impl and dropping leaves the movement uncommitted.
- claude-B1-2's Permission fix: codex notes conversion is not automatically
  correct for button-presence and startup options; either fix stays open.
- codex-B1-1, excluded by claude as not instances: `SplitPane::locked` and
  `ToolWindowLayout::locked` (delegate to Permission builders, split.rs:363,
  tool_windows/mod.rs:902), `ToolGroup::essential`/`collapse_first` and
  `Dialog::view_modal`/`application_modal` (mode selectors, not booleans).
- claude's inventory gap 1 (fallible builders missing from builders.txt):
  disputed by codex as out of the list's stated scope; moot in inventory
  version 2, which lists them as `[fallible]`.
