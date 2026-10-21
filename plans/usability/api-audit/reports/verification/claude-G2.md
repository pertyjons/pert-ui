# claude verification: G2

Source read at HEAD (crates/ unchanged since d4ba108). Runtime paths are under
crates/pert-ui-runtime/src/ unless stated otherwise. Nothing was compiled.

## The other auditor's findings

| Other ID | Matched by | Verdict | Evidence | Remarks |
|---|---|---|---|---|
| codex-B1-1 (no-argument one-way boolean shortcuts) | none (claude-B1-3 is a different departure: `disabled(bool)`/`read_only(bool)` taking a `bool` under the names reserved for no-argument shortcuts) | confirmed | Each cited method sets one boolean field with no argument: controls.rs:604-607 `Button::frameless` (`self.frameless = true`, rustdoc "Default false"); views.rs:168-171 `Group::frameless` (`framed = false`); field_status.rs:498-500 `Field::hide_label`; inspection.rs:247, 259, 264, 269, 276 `InspectionQuery::{parameters, windows, sources, validation, application}`; diagnostics.rs:633 `Semantics::names_children`; pointer.rs:192 `PointerEvent::outside`. Rule docs/contracts.md:1155-1158 ("A builder for an ordinary boolean property takes a `bool`"; only `disabled()`/`read_only()` are named as no-argument shortcuts). | Severity low and kind (project rule with a policy question on scope) are right. The evidence is stronger than the report says: sibling query builders take a `bool` (`ContentQuery::hidden(bool)`/`rows(bool)`, content_search.rs:100/109; `CaptureRequest::include_overlay(bool)`, capture.rs:137), while `InspectionQuery` takes none. Also, `Semantics` mixes both forms on one type (`disabled(bool)`, `read_only(bool)` beside `names_children()`). `PointerEvent::outside` is a weak instance: it marks a synthetic event value, not a description property. The list is incomplete. builders.txt has further no-argument boolean setters: `RegionLayout::fixed` (region_layout.rs:151), `Divider::vertical` (views.rs:928), `Toolbar::attached_below` (toolbar.rs:479), `Dialog::require_response` (dialog.rs:226), `QuickPick::nonmodal` (quick_pick.rs:920), `CommandPalette::nonmodal` (command_palette.rs:540), `Element::structural` (layout.rs:655), `View::fixed` (views.rs:35), `pert_ui::App::software_rendering` (crates/pert-ui/src/app.rs:412, configurations cpu and no-default-desktop-cpu). `ToolGroup::essential`/`collapse_first` (toolbar.rs:107/116) set a three-state priority, and `Dialog::view_modal`/`application_modal` set an enum, so they are mode selectors, not booleans. `SplitPane::locked` and `ToolWindowLayout::locked` delegate to Permission builders (split.rs:363, tool_windows/mod.rs:902), which makes them legitimate `disabled()`-style shortcuts. The fix is right (document the shortcut convention, or add `bool` setters). Adding the setters need not break anything; changing the existing signatures would. |
| codex-B2-1 (registration identities warn on legitimate discard) | claude-B2-3 (its `CommandId` and `HeldInputId` instances) | matched | commands.rs:377-380 and held.rs:5-8 (`#[must_use]`); discarding the identity keeps the registration. | Same severity (low), kind and fix. claude-B2-3 also covers the receipts. Codex does not take a position on those: its roles table classifies them as "retained request/status receipts" and records no finding. |
| codex-B2-2 (local parameter owners lack `must_use`) | claude-B2-1 (its `ParameterHost<T>` and `AnyParameterHost` instances) | matched | parameter_binding.rs:495, Drop at 626-635; erased.rs:255. | Same severity (medium) and fix. claude-B2-1 also covers `SeriesSource<T>` (live.rs:627) and `Runtime::series_source`, which codex does not mention. |
| codex-B3-1 (RegionLayout callback removal keeps pending reports) | none | confirmed | `RegionLayout::on_resize` rustdoc promises that removing the callback "also drops changes not yet delivered" (region_layout.rs:255). `take_children` always installs a forwarding closure on every generated split (region_layout.rs:363-372), and that closure returns `Ok(())` when the slot is empty. SplitPane discards a pending report only when the incoming or the previous callback is `None` (split.rs:498-500), and `changed` records whenever a callback is present (split.rs:339-343). Neither condition can hold for a generated split. `GeometryReport::take` consumes the pending origin only at delivery (geometry_report.rs:69-75). So a report recorded under callback A survives a description without a callback and reaches a callback installed before the delivery. Rule: docs/contracts.md:355-356. No test covers removal followed by reinstallation (crates/pert-ui/tests/surfaces/geometry_callbacks.rs:453 has only the region test). | Kind and fix are right, and the fix does not break anything. On severity I would say low rather than medium: the departure is observable only when two redescriptions (removal, then reinstallation) happen before the event-phase delivery, because a delivery while the callback is absent consumes the report into a no-op. The same mechanism has a second effect the report does not name. A RegionLayout that starts a callback does not "take the committed preference as known", as SplitPane does (split.rs:496-499), so a Restore or Redescription recorded while the layout had no callback reaches a newly installed callback in the same flush. |
| codex-B3-2 (reconciliation publishes library-owned signals) | partly: claude reviewed the `ContextMenu` instance (menu.rs:1472) and recorded it as "Reviewed without a finding"; the parameter-cancellation instances are unmatched | partly confirmed (I confirm the facts for every instance. I dispute the severity of the `ContextMenu` instance; see "My findings disputed") | ContextMenu: `reconcile` calls `self.enabled_signal.publish_trusted(enabled)` (menu.rs:1472). Parameter path: `NumericControl::reconcile` (numeric_controls.rs:532) calls `abandon()` on a rebind and on a fixed denial (numeric_controls.rs:568, 585). `abandon` drops `self.edit` (numeric_controls.rs:407-412). The number inputs' `adopt`, called from `reconcile` (number_inputs.rs:2138-2140), calls `self.source.abandon()` (number_inputs.rs:1274, 1309), and that drops the session (number_inputs.rs:616-619, 761-764). `Drop for EditSession` (parameter_binding.rs:902) runs the cancellation, which cancels the state and calls `self.parameter.publish()` at once (parameter_binding.rs:678); only the terminal event is deferred (679-686). `publish` calls `Signal::publish_trusted` (parameter_binding.rs:178-182), and that invalidates computations and subscribers synchronously (signal.rs:431-445). The published signal is public: `Parameter::snapshot() -> ReadSignal<ParameterSnapshot<T>>` (parameter_binding.rs:316). Rule: AGENTS.md:132. | The parameter path is the substantive instance. The application can observe the publication: computations and controls bound to the snapshot are invalidated during reconciliation. The code also departs from docs/contracts.md:166-167 ("a parameter's gesture is cancelled at the next safe point"), because the state is cancelled and published at once and only the notification waits. Medium severity fits the parameter path. Kind: project rule plus a policy question, which is correct. The fix is right: defer the cancellation's publication, or record a narrow lifecycle exception in contracts. The ContextMenu instance is a private signal whose only reader is `ContextMenuOpener::open`, which reads it with `get()` in an event (menu.rs:1360-1361). No component or computation can subscribe to it, so it should be low severity, not medium. I found no further parameter-session drops in reconciliation: the choice controls use one-shot sessions (control_binding.rs:138-142). |

## My findings disputed

Codex records "No additional verified replacement, Permission-composition or
revocation departures found" for B1 and only the findings above for B2 and B3.
That disposition contradicts the findings below. I re-checked each against the
source.

- **claude-B1-1 and claude-B3-1 (collection permissions and callbacks belong
  to the first description). Maintained.** Codex's roles table says
  "initial controller/options/callback exceptions apply" for VirtualList, Tree,
  Table and FileBrowser, and cites docs/contracts.md:169. That sentence
  (contracts.md:169-171) names only "their mounted controller and options". It
  does not name callbacks or permissions. ColorPicker is in the same list, and
  its permission and handler follow redescription (contracts.md:333-334, "its
  binding and options stay retained"; contracts.md:478-480, "The handler
  follows same-key redescription … otherwise keeps its binding, options and
  drafts"). So in the contract, "options" excludes callbacks and permissions.
  The retention is stated only in the item rustdoc (virtual_list.rs:428-437;
  table.rs:1389-1391). Under the rubric ("an exception cites the exact
  contract sentence and the behavior it covers"), that is not a contract
  exception. Whether "options" should include them is the policy question my
  findings already record.
- **claude-B1-2 (`FloatingWindow::closable`/`minimizable` and
  `App::resizable` take `bool`). Maintained.** windows.rs:327-336 take a
  `bool` and replace, while `movable` beside them takes `impl Into<Permission>`
  and composes (windows.rs:337-349). Codex does not address these items.
- **claude-B1-3 (`disabled(bool)`/`read_only(bool)` on PickItem, Semantics,
  SemanticItem). Maintained as a policy question.** Codex's roles table
  ("`disabled` fields are descriptive booleans, not Permission authorities")
  agrees with the applicability doubt my finding already states. It does not
  contradict the fact.
- **claude-B1-4 (accumulating builders). Maintained.** `title_action` pushes
  (windows.rs:421-426, "Adds"). Codex's roles table exempts only RichText and
  PathBuilder, the two items my finding also excludes.
- **claude-B2-2, claude-B2-4, and the receipt and SeriesSource parts of
  claude-B2-1/B2-3. Maintained.** Codex's roles table states no fact that
  conflicts with them. It classifies Signal/ReadSignal as shared bindings,
  which says nothing about discarding a newly created signal. It also says
  PreparedDetach's attribute "is present", which is true; my finding is about
  the missing reason.
- **claude-B3-2 (`on_rejection` coverage). Maintained.**
  number_inputs.rs:2086-2091 ("with the rules of [`TextField::on_rejection`]"
  … "invalid for the field's own reasons … is not a refusal") against
  text_fields.rs:306-308, where a length limit is reported.
- **claude-B3-3 and claude-B3-4 (shapes). Maintained.** controls.rs:551
  `FnMut(CommandReceipt, &mut EventContext)`; collapsible.rs:1049
  `FnMut(&mut EventContext, &[String], ChangeOrigin)`. The v2 callbacks.txt
  confirms that every other UI-thread component callback takes
  `&mut EventContext` first.
- **My "Reviewed without a finding" for the ContextMenu signal
  (codex-B3-2). Corrected in part.** The fact is the same in both reports:
  menu.rs:1472 publishes from `reconcile`. That is a literal departure from
  AGENTS.md:132, and the field comment (menu.rs:1258-1259) is not a contract
  exception. I recorded no finding because nothing can observe it, and I
  still judge it low. I withdraw "no finding" and accept it as a low-severity
  instance of codex-B3-2, with the policy question attached. I missed the
  parameter-cancellation path, which codex found, and I confirm it above.
- **Inventory gap 1 (fallible builders).** Codex says they are not a gap
  because builders.txt is limited to `self -> Self`. Version 2 lists them as
  `[fallible]`, so the question is moot.

## Inventory delta

Coverage. Versus version 1:
- builders.txt: 49 `[fallible]` entries added. I read the bodies of all the
  builders, and the keyed setters `ViewState::with` (workspace.rs:248-258),
  `UiStrings::with` and `ThemeMetrics::with_metric` replace per key. Every
  entry returns `Result`, which carries `#[must_use]`, so B2 is satisfied.
- callbacks.txt: 64 `[closure]` entries added. The 60 `on_*` entries are
  unchanged apart from the `[on_]` mark. I checked all 64 for signature shape
  per family.
- bool-params.txt: header only.
- attributes.txt: 2 entries changed (paths qualified). No change in
  substance.
- types.txt: 535 rows gained the prelude column. No B1–B3 effect.
- Semantic lists: I read the new field and variant payloads of the public
  notification and event payload types: SelectionChange,
  ListSelectionChange, ToolWindowChange, ToolWindowChangeKind,
  EditorCompletion, ChangeOrigin, TerminalEvent, PreviewEvent,
  AnyTerminalEvent and AnyPreviewEvent. The newly listed
  `App::software_rendering` adds an instance to codex-B1-1, noted above. No
  other B1–B3 entries.

### claude-B3-d1 MultiSelect's match rule is a bare closure, unlike the other match policies

- Rubric item and kind: B3, project rule ("Signature shapes, per family …
  A shape that differs from its family without a documented reason is a
  finding"); policy question on whether match rules count as a callback
  family.
- Items: `MultiSelect::filter(self, impl Fn(&str, &Choice<T>) -> bool)`
  (multi_select.rs:1382), compared with `QuickPick::matcher(PickMatcher)`
  (quick_pick.rs:926), `CommandPalette::matcher(CommandMatcher)`
  (command_palette.rs:546) and `CollectionFilter::matcher(FilterMatcher)`
  (collection_filter.rs:587). All configurations.
- Observation (signature): the other three query-matching policies are a
  `#[must_use]` `*Matcher` value with presets (`FilterMatcher::substring`,
  `fuzzy`, collection_filter.rs:27-39), passed to a `matcher` builder.
  MultiSelect takes a raw closure under the name `filter`, and has no fuzzy
  preset to reuse. The same name `filter` is also the query-applying
  operation on the controllers (`TableController::filter(&self, query,
  &FilterMatcher)`, table.rs:707; collection_filter.rs:318; tree.rs:468).
  The rustdoc (multi_select.rs:1376-1381) gives no reason for the
  difference.
- Severity: low (a consumer cannot reuse a `FilterMatcher` in a MultiSelect).
  Confidence: high on the facts; medium on whether the rule applies.
- Suggested fix: `MultiSelect::matcher(FilterMatcher)`. Its closure would
  match the label, or a `FilterMatcher` variant would take the `Choice`.
  Breaking.
- Destination: TODO.md.

### claude-B3-d2 `Runtime::set_wake_delivery` takes a boxed closure

- Rubric item and kind: B3, project rule (signature shapes); low.
- Items: `Runtime::set_wake_delivery(&self, deliver: Box<dyn Fn() -> bool
  + Send + Sync>)` (runtime.rs:4183). All configurations.
- Observation (signature): of the 64 closure-taking public functions in
  callbacks.txt, it is the only one that requires a `Box`. Its cross-thread
  sibling `pert_ui_wgpu::Renderer::new` takes `notify: impl Fn(Notice) + Send
  + Sync + 'static`, and every other callback takes `impl Fn*`. The rustdoc
  (runtime.rs:4171-4182) gives no reason. Callers write `Box::new(…)`, as in
  crates/pert-ui-runtime/tests/live_sources.rs:206.
- Severity: low. Confidence: high.
- Suggested fix: `deliver: impl Fn() -> bool + Send + Sync + 'static`, boxed
  inside. Existing calls that pass a `Box<dyn Fn() -> bool + Send + Sync>`
  still compile, because that type implements `Fn`. Not breaking for those
  calls.
- Destination: TODO.md.

Not recorded as a finding: `SelectionChange` names its fields
`previous`/`selected`, while `ListSelectionChange` uses
`previous_selected`/`selected`/`previous_active`/`active` (semantic list,
runtime/notify.rs:181, list_selection.rs:86). This is field naming (group N),
not a callback shape. The groups that own naming should look at it.
