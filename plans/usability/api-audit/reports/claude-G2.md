# claude report: G2 (builders and callbacks: B1, B2, B3)

Commit audited: d4ba108 (crates/ unchanged since; read at HEAD 8b9c1d3 on branch developer-usability).

## Dispositions

| Item | Disposition | Coverage |
|---|---|---|
| B1 builders and permissions | findings (5: claude-B1-1 to claude-B1-5) | builders.txt: all 530 entries; plus the 49 `self -> Result<Self, _>` builders that only errors.txt lists (inventory gap 1). A script extracted every source body of a public `self`-receiver method returning `Self`/`Result<Self, _>` in crates/pert-ui-runtime, pert-ui-core, pert-ui (415 definitions; macro bodies once, which cover the macro-generated entries at number_inputs.rs:2537-2546, text_fields.rs:768-769, choice_controls.rs:647/652, numeric_controls.rs:1586-1605). 217 bodies are plain field assignments (replace); the other 198 were read individually: 44 compose through `Permission::and`, the accumulating ones are listed in claude-B1-4, the rest replace. bool-params.txt: all 78 entries (for the bool/`_signal`/`disabled()` rules). Every `impl Into<Permission>` builder's rustdoc was checked for combination, revocation and redescription statements; the reconcile code of Button, Link, Chip, ColorSwatch, InlineToggle, Collapsible, Accordion, SplitPane, RegionLayout, PropertyGrid, Tabs, CollectionFilter, HistoryNavigator, HistoryButtons, FileBrowser, ToolWindowLayout was read for permission replacement; composites' redescription documentation was checked by rustdoc search (all composites in the contract lists, sampled for QuickPick and MenuBar). Tests read: tests/audit/audit_collection_permissions.rs:685. |
| B2 `#[must_use]` | findings (4: claude-B2-1 to claude-B2-4) | attributes.txt: all 697 `must_use` lines read for types; types.txt / semantic lists: all 534 types classified into builder / handle / receipt / other (roles below); per builder type, type-level or per-method `must_use` checked for all 579 builder methods (530 + 49). Drop impls of all public types located with `rg 'impl Drop for'` and each handle's owner semantics read. F19 re-examined. Guide and examples searched for `let _ =`/`let _receipt` workarounds. |
| B3 callbacks | findings (5: claude-B3-1 to claude-B3-5) | callbacks.txt: all 60 entries, by family: user-write (Checkbox, Toggle, InlineToggle, RadioGroup, SegmentedControl, InlineChoice, Select, InlineSelect, MultiSelect, MenuItem, ColorPicker), component notifications (Accordion, Tabs, VirtualList/Tree/Table `on_selection_change`, ToolWindowLayout), geometry (ScrollArea, SplitPane, RegionLayout, PropertyGrid, FloatingWindow::on_frame), editor (TextField/TextArea/InlineText/number inputs, DraftEditorController::on_edit, SearchField::on_key), activation/dismissal (ColorSwatch, VirtualList/Tree/Table `on_activate`, Table::on_sort, VirtualList::on_text_input, Popover::on_dismiss, FloatingWindow::on_close, FileBrowserController::on_open), parameter sinks (ParameterHost, AnyParameterHost, RemoteHost) and App (`on_close`, `on_error`, `on_window_error`). Plus 63 callback-taking functions not named `on_*` (inventory gap 2) for signature shape. Invocation sites read: choice_controls.rs:207-214, 816-829; control_binding.rs:24-41, 77-79, 129-145; select.rs:1431-1437; multi_select.rs:304-312; menu.rs:170-185; reconcile of every family's components for handler replacement; reconcile bodies of all components scanned for signal writes (7 hits, all read). Tests used: tests/surfaces/tab_notifications.rs:391, menu_changes.rs:146, collections/multi_select.rs:1123, 1297, editing/binding_refusal.rs:183. |

## Findings

### claude-B1-1 Collection permissions survive compatible redescription

- Rubric item and kind: B1, project rule (contracts "Permissions and availability"; AGENTS.md "Component API conventions"); includes a policy question.
- Items: `Table::resize_permission`, `Table::reorder_permission`, `VirtualList::reorder_permission`, `Tree::reorder_permission`, and `FileBrowser::folder_filter`; all configurations.
- Observation (behavior): the builders compose with `Permission::and` correctly, but a same-key redescription keeps the first description's permission and its binding. Rustdoc says so at crates/pert-ui-runtime/src/table.rs:1389-1391 and 1429-1431, virtual_list.rs:982-985, tree.rs:1033-1035, and the type docs table.rs:1218-1222, virtual_list.rs:428-437, tree.rs:885-891. The test crates/pert-ui/tests/audit/audit_collection_permissions.rs:685-737 asserts it: after redescribing with a `true` binding, `resize_allowed`/`reorderable` stays `false` (the original binding), and flips when the original binding is written. `FileBrowser::folder_filter` is "Fixed when the browser mounts" (file_browser/view.rs:106); its CollectionFilter part is a retained generated child.
- Rule: contracts.md:308-309 "Redescription replaces the description's permission, so a new enabled description can unlock a previously disabled component"; contracts.md:164 "texts, bindings, ranges, permissions and callbacks follow the new description". Family exception considered: contracts.md:169-171 "Components whose data belongs to a controller (`VirtualList`, `Table`, `Tree`, `FileBrowser`, `ColorPicker`, `HistoryNavigator`) keep their mounted controller and options and say so". It names "controller and options", not permissions, and the same list's ColorPicker follows redescription for its permission (contracts.md:333-334 "ColorPicker's permission follows same-key redescription although its binding and options stay retained"), as does HistoryNavigator (history_navigator.rs:339-350 adopts `buttons`/`list`). contracts.md:296-297 says the collection restrictions "use this policy", without naming an exception. So the exception sentence does not clearly cover permissions; the item rustdoc does.
- Severity: medium (a redescription that intends to deny reordering leaves the old binding in charge; the inverse case in the test allows what the new description denies). Confidence: high on the behavior; whether "options" includes permissions is a policy question.
- Suggested fix: either adopt the new description's permission on compatible redescription (behavior change, not a signature break), or state the exception in contracts.md "Permissions and availability" with the exact components and behavior. Breaking: behavior only.
- Destination: TODO.md.

### claude-B1-2 Capability toggles that take `bool` beside `Permission` siblings

- Rubric item and kind: B1, project rule (contracts.md:1154-1156 "Capability restrictions take `impl Into<Permission>` and compose with `and`"); policy question on scope.
- Items: `FloatingWindow::closable(bool)` (windows.rs:327), `FloatingWindow::minimizable(bool)` (windows.rs:333), `pert_ui::App::resizable(bool)` (crates/pert-ui/src/app.rs:375; configurations with `App`: cpu, default, image-formats, images, no-default-desktop-cpu, serde, svg).
- Observation (signature): on the same `FloatingWindow`, `movable` (windows.rs:346) and `resizable` (windows.rs:412) take `impl Into<Permission>` and combine with `and`, while `closable` and `minimizable` take a `bool` and replace (windows.rs:327-336). A consumer cannot bind closability to a signal, and `closable(false).closable(true)` lifts the denial, unlike the Permission builders. Part-offering elsewhere uses Permission (`CollectionFilter::offered`, collection_filter.rs:622; `HistoryNavigator::buttons`/`list`, history_navigator.rs:169-178). `App::resizable` is the main window's user-resize capability and takes `bool`, while every in-view resize capability takes a Permission. Not covered by a family exception: the table-column flag exception (contracts.md:296-299) concerns `TableColumn` flags only.
- Severity: low. Confidence: medium (whether a title-button presence is a "capability restriction" is a policy question; `App::resizable` is a startup option).
- Suggested fix: take `impl Into<Permission>` and combine with `and` (`bool` converts, so existing calls compile; `false` then `true` stops lifting the denial: behavior change); or document why these are fixed options. Breaking: behavior only.
- Destination: TODO.md.

### claude-B1-3 `disabled(bool)` and `read_only(bool)` builders

- Rubric item and kind: B1, project rule (contracts.md:1158 "`disabled()` and `read_only()` are no-argument shortcuts away from defaults"); policy question on scope.
- Items: `PickItem::disabled(self, bool)` (quick_pick.rs:78), `Semantics::disabled(self, bool)` (diagnostics.rs:579), `Semantics::read_only(self, bool)` (diagnostics.rs:598), `SemanticItem::disabled(self, bool)` (diagnostics.rs:392); all configurations.
- Observation (signature): these take a `bool` under the names the rule reserves for no-argument shortcuts. Every control's `disabled()` (for example controls.rs:642) takes none, so `.disabled()` on a `PickItem` is a compile error and `.disabled(true)` on a `Button` is one too. These are descriptive data builders rather than capability restrictions, so the rule may not be meant for them. Semantics' `disabled(true)` also clears actions irreversibly (diagnostics.rs:579-585), as documented.
- Severity: low. Confidence: high on the fact; applicability is a policy question.
- Suggested fix: rename to a non-reserved name, such as `PickItem::unavailable(bool)` or `Semantics::with_disabled(bool)`, or record in the naming rules that data descriptions take the state as a `bool`. Breaking.
- Destination: TODO.md.

### claude-B1-4 Accumulating builders outside a documented family exception

- Rubric item and kind: B1, project rule (AGENTS.md:184 "Ordinary property builders replace earlier values"; AGENTS.md "reserve accumulating spans for explicit RichText composition"); policy question.
- Items: `FloatingWindow::title_action` (windows.rs:421-426, `self.actions.push`), `ToolWindowLayout::window`/`windows` (tool_windows/mod.rs:788-809), `Semantics::action`/`actions` and `SemanticItem::action` (diagnostics.rs:420, 556, 566). `RichText` (text/rich.rs:103-146) and `PathBuilder` (geometry.rs:422-528) are explicit composition and are not findings.
- Observation (behavior): each call adds rather than replaces, which rustdoc states ("Adds ordinary interactive title-bar content", "Adds a tool window", "Offers one action"). contracts.md lists no such exception. Next to them, related builders replace: `ToolWindow::actions(Element)` (tool_windows/mod.rs:457) for the same concept as `title_action`, `Semantics::items` (diagnostics.rs:571, "replacing earlier ones") beside the accumulating `Semantics::actions`, and `Element::children` (layout.rs:638).
- Severity: low (documented on each item). Confidence: medium.
- Suggested fix: name the accumulating builders as collection composition in contracts.md "Public naming rules" (for example `add_*`, or a sentence that list-element builders add), or make `title_action` take its elements at once, like `ToolWindow::actions`. Breaking only if renamed.
- Destination: TODO.md.

### claude-B1-5 Proposal: names of writable boolean state bindings

- Rubric item and kind: B1, consistency proposal.
- Items: `Collapsible::bound(impl Into<Signal<bool>>)` (collapsible.rs:533) with `Collapsible::expanded(bool)` (collapsible.rs:525); `ToolWindowLayout::names(impl Into<Signal<bool>>)` (tool_windows/mod.rs:924) with `ToolWindowLayout::show_names(bool)` (tool_windows/mod.rs:916).
- Observation (signature): both pairs are an initial `bool` plus a two-way `Signal<bool>` binding, which the `_signal` rule (contracts.md:1157, read-only `ReadSignal<bool>`) does not cover. The binding builder is `bound` in one and the bare noun `names` in the other, and the initial-value builder is `expanded` in one and `show_names` in the other. `bound` is also the choice-source constructor name (AGENTS.md "`bound` follows a dynamic list"), here used as a builder.
- Severity: low. Confidence: high on the fact; adopting a convention is a policy decision.
- Suggested fix: one convention for two-way state bindings, for example `<property>_binding(Signal<bool>)`. Breaking.
- Destination: TODO.md.

### claude-B2-1 Owners whose drop tears down state lack `#[must_use]`

- Rubric item and kind: B2, project rule (AGENTS.md "Use `#[must_use]` for results/handles/builders whose discard is a mistake").
- Items: `ParameterHost<T>` (parameter_binding.rs:495), `AnyParameterHost` (parameter_binding/erased.rs:255), `SeriesSource<T>` (live.rs:627) and its constructor `Runtime::series_source` (runtime.rs:4075); all configurations.
- Observation (signature, with the drop behavior read in source): dropping a `ParameterHost` removes the parameter (`impl Drop`, parameter_binding.rs:626-635), and `AnyParameterHost` drops the typed owner inside (erased.rs:252-253). `SeriesSource::new` moves the `SeriesConsumer` into an `Rc` and registers only a `Weak` (live.rs:638-650), so discarding the source drops the consumer and closes the bridge. Their siblings carry the attribute with a reason: `RemoteHost` (remote.rs:420 "dropping the authority owner makes the binding unavailable", which wraps a `ParameterHost`, remote.rs:421-423) and `SeriesConsumer` (core live/series.rs:292 "dropping the consumer closes the bridge"). `Runtime::parameter` returns `(Parameter<T>, ParameterHost<T>)` (parameter_binding.rs:958), where the lint checks tuple elements, so the attribute would warn on a discarded call.
- Severity: medium (a silent removal of the parameter or the data feed). Confidence: high.
- Suggested fix: `#[must_use = "dropping the host removes the parameter"]` on `ParameterHost`/`AnyParameterHost`, and `#[must_use = "dropping every source closes the bridge"]` on `SeriesSource`. Not breaking (new warnings only).
- Destination: TODO.md.

### claude-B2-2 Signal constructors lack `#[must_use]`

- Rubric item and kind: B2, project rule (same sentence).
- Items: `Runtime::signal` (runtime.rs:4048), `StateOwner::signal`, `EventContext::signal`, `UpdateContext::signal`, `Signal::sibling`; `Signal`/`ReadSignal` have no type-level attribute (attributes.txt).
- Observation (signature): a created and discarded signal has no effect. The comparable constructors carry the attribute: `ListController::new`, `ZoomController::new`, `ToolWindowController::new`, `DraftEditorController::new`, `EditorSession::new` (`#[must_use]` per semantic list) and `Signal::read_only`.
- Severity: low. Confidence: high.
- Suggested fix: `#[must_use]` on the creating functions (not on the type, so `set`-style returns stay unaffected). Not breaking.
- Destination: TODO.md.

### claude-B2-3 `#[must_use]` on identities and receipts whose discard is normal (F19 re-examined)

- Rubric item and kind: B2, project rule ("A value whose discard is normal does not"); policy question, since the rubric lists receipts among the types that carry the attribute.
- Items: `CommandId` (runtime/commands.rs:377-380), `HeldInputId` (runtime/held.rs:5-8), `TaskReceipt` (runtime/tasks.rs:27), `RevealReceipt` (runtime/reveal.rs:132), `ListRevealReceipt` (virtual_list.rs:229), `ToastReceipt` (notifications.rs:195), `CommandReceipt` (runtime/commands.rs:1606), `PresentationReceipt` (runtime/native_windows.rs:949); all configurations.
- Observation (signature, with source evidence that the discard is normal): dropping `CommandId` keeps the command registered, and `HeldInputId` is "removed explicitly or with its owner". Deferred-task errors "propagate from flush and remain in the receipt" (tasks.rs:104-105). `ListRevealReceipt`'s own rustdoc tells callers to discard it: "a caller that does not need it discards it explicitly (`let _ = controller.reveal(key)?;`)" (virtual_list.rs:223-226). Two of the reasons concede the point ("retain the receipt if the application needs …", tasks.rs:27, reveal.rs:132). The guide and the examples carry workarounds: crates/pert-ui/src/guide/surfaces.md:314 `let _receipt = post.push(…)`; guide/diagnostics.md:36 discards a `CommandId` and warns (F19); examples/diagram_editor/app.rs:600, 821, 852 `let _receipt =`; examples/dynamic_documents.rs:26 `let _operation =`; examples/inventory/app.rs:484 `let _ =`.
- Severity: low (annoyance). Confidence: high on the facts; the policy is the maintainer's.
- Suggested fix: remove the type-level attribute from these types and keep `#[must_use]` only where dropping loses the only report of a failure. Or keep it and record in contracts "Public naming rules" that receipts are must_use by design, with the `let _ =` idiom shown once. Not breaking.
- Destination: TODO.md (F19 is already routed there).

### claude-B2-4 Drop-cancelling tokens without a reason

- Rubric item and kind: B2, project rule ("with a reason where the discard is a likely slip").
- Items: `PresentationRequest` (runtime/native_windows.rs:1011-1015, "Dropping an unfinished request cancels it"; `impl Drop` at 1050-1057), `PreparedDetach` (native_windows.rs:241-245, "Dropping this token leaves the view unchanged").
- Observation (signature): bare `#[must_use]`, while the comparable host-answered tokens give reasons: `DirectoryRequest` (file_browser.rs:196), `RequestTicket` (async_source.rs:28), `Pending` (inspection.rs:565).
- Severity: low. Confidence: high.
- Suggested fix: add reasons, such as "dropping an unfinished request cancels it". Not breaking.
- Destination: TODO.md.

### claude-B3-1 Collection callbacks belong to the first description

- Rubric item and kind: B3, project rule (contracts.md:164 "texts, bindings, ranges, permissions and callbacks follow the new description"; contracts "Component notifications": removing a callback discards pending payloads; contracts.md:460-461 for user-write callbacks).
- Items: `VirtualList::on_selection_change`, `on_activate`, `on_text_input`, `reorderable`, `placement`; `Tree::on_selection_change`, `on_activate`, `reorderable`; `Table::on_selection_change`, `on_activate`, `on_sort`, `cell_editor`; all configurations.
- Observation (behavior, documented on the items): a same-key redescription ignores the new callbacks. virtual_list.rs:428-437 ("keeps everything the first description gave it: … the `on_activate`, `reorderable` and `placement` callbacks … A later description's values are ignored"), virtual_list.rs:840-841 ("Like the other callbacks, it belongs to the first description of a mounted list"), tree.rs:885-891, tree.rs:1053, table.rs:1218-1222, 1554-1555, 1579-1581. A consumer cannot replace or remove `on_selection_change` without a new key, and a new closure that captures new state is silently not used. Family exception considered: contracts.md:169-171 covers "controller and options", and for ColorPicker, in the same list, contracts.md:479-481 says "The handler follows same-key redescription … whose retained picker otherwise keeps its binding, options and drafts". So "options" excludes callbacks there, and the exception sentence does not cover these callbacks. `Tabs::on_select`, which the list's rustdoc cites as its model (virtual_list.rs:835-836), is replaced and removable (tabs.rs:950-953; test tests/surfaces/tab_notifications.rs:391).
- Severity: medium. Confidence: high.
- Suggested fix: replace (and allow removing) these callbacks on compatible redescription, as Tabs does, or name the exception in contracts.md "Component redescription". Breaking: behavior.
- Destination: TODO.md.

### claude-B3-2 `on_rejection` means different coverage in one editor family (F6 re-examined)

- Rubric item and kind: B3, project rule ("within a family, one name means one timing"; a difference no contract explains that misleads a consumer).
- Items: `IntegerInput`, `DecimalInput`, `InlineInteger`, `InlineDecimal::on_rejection` (number_inputs.rs:2086-2096, macro invoked at 2537-2540) against `TextField`/`TextArea::on_rejection` (text_fields.rs:300-318); all configurations.
- Observation (behavior, documented): `TextField::on_rejection` reports the normalizer's refusals and its own length limits ("a length limit (`Self::min_length`, `Self::max_length`) refuses", text_fields.rs:306-308). The number inputs' callback reports only the normalizer, "A draft that is invalid for the field's own reasons (unparsable, out of range) is not a refusal" (number_inputs.rs:2089-2091). Their rustdoc nevertheless opens "with the rules of [`TextField::on_rejection`]" (number_inputs.rs:2086-2087). The built-in parse and range messages are visible only through inspection (slice 3 F6, hit by two runs). No contract section explains the split.
- Severity: medium (two testers needed a workaround). Confidence: high.
- Suggested fix: report built-in parse and range rejections through `on_rejection`, as TextField does for its limits, or drop the "same rules" cross-reference and document the difference in "Text and draft editing". Breaking: behavior, if fixed in code.
- Destination: TODO.md (F6's destination).

### claude-B3-3 Command receipt callbacks take `EventContext` second

- Rubric item and kind: B3, project rule (signature shapes per family).
- Items: `Button::command` (controls.rs:549-552), `Button::command_icon` (controls.rs:588-592), `MenuItem::command` (menu.rs:400-404): `on_receipt: impl FnMut(CommandReceipt, &mut EventContext)`; all configurations.
- Observation (signature): every other UI-thread component callback, including the same types' plain activation (`Button::new`, `MenuItem::new`), takes `&mut EventContext` first (callbacks.txt, and the 63 non-`on_*` callbacks). The rustdoc (controls.rs:520-532) gives no reason for the order.
- Severity: low. Confidence: high.
- Suggested fix: `FnMut(&mut EventContext, CommandReceipt)`. Breaking.
- Destination: TODO.md.

### claude-B3-4 `Accordion::on_change` shape differs from the other selection and expansion notifications

- Rubric item and kind: B3, project rule (signature shapes per family).
- Items: `Accordion::on_change(FnMut(&mut EventContext, &[String], ChangeOrigin))` (collapsible.rs:1049) against `Tabs::on_select(&SelectionChange)` (tabs.rs:607; struct with `previous`, `selected` and `origin`, runtime/notify.rs:181-188), the collections' `on_selection_change(&ListSelectionChange)` (list_selection.rs:86-99, with previous values and `origin`) and `ToolWindowLayout::on_change(&ToolWindowChange)` (tool_windows/mod.rs:218-242, with `origin`); all configurations.
- Observation (signature): Accordion passes the origin positionally and no previous state, while its family passes one change struct. The rustdoc (collapsible.rs:1024-1032) gives no reason. Geometry callbacks use the positional `(value, origin)` shape, but they are a separate family under contracts "Geometry notifications".
- Severity: low. Confidence: high.
- Suggested fix: an `ExpansionChange { previous, expanded, origin }` struct passed by reference. Breaking.
- Destination: TODO.md.

### claude-B3-5 Proposal: `on_select` and `on_selection_change` for one timing

- Rubric item and kind: B3, consistency proposal ("Further naming uniformity").
- Items: `Tabs::on_select` (tabs.rs:607) and `VirtualList`/`Tree`/`Table::on_selection_change` (virtual_list.rs:872, tree.rs:1054, table.rs:1556).
- Observation (signature): the same settled selection notification has two names. The list's rustdoc defines its timing by reference to `Tabs::on_select` (virtual_list.rs:835-836).
- Severity: low. Confidence: high on the fact.
- Suggested fix: one name, for example `on_selection_change` on Tabs. Breaking.
- Destination: TODO.md.

### Reviewed without a finding

- User-write `on_change` timing: `ValueBinding::choose` returns `true` only for a changed local write or an admitted remote submission (control_binding.rs:29-41, 77-79 `set_changed`, 129-145), and every boolean and single-choice invocation is gated on it (choice_controls.rs:207-214, 816-821; select.rs:1431-1437). Handlers are replaced on redescription (choice_controls.rs:233, 933; inline_controls.rs:226; select.rs:990; multi_select.rs:1414; tests menu_changes.rs:146, multi_select.rs:1297). Errors propagate after the write.
- Reconciliation that publishes signals: one hit, `Menu`'s private `enabled_signal`/`visible_signal` via `publish_trusted` (menu.rs:1472, 1484). It is a library-internal signal read with `get()` by `ContextMenuOpener::open` (menu.rs:1361), with the reason in a comment. It is not application-observable, so I recorded no finding. The other hits are `Cell`s.
- Permission composition: all 44 composing builders and the helpers `restrict_enablement`/`restrict_editing` (text_fields.rs:1953-1966), `Headers::restrict` (collapsible.rs:98-103) and `edge_resizable` (region_layout.rs:213-220) use `and`. `disabled()` and `read_only()` delegate to `enabled(false)` and `editable(false)`, or to a fixed-denial flag that `enabled` keeps (menu.rs:557-571).

## Inventory gaps

1. **builders.txt omits fallible builders.** Its header is "Methods taking self and returning Self", but builder methods returning `Result<Self, _>` are not listed, while B1 and B2 apply to them. The 49 entries appear only in errors.txt, for example `IntegerInput::range`/`step`/`normalize` (number_inputs.rs:1925/1948/2076), `Slider::decimal_places` (numeric_controls.rs:1328), `FloatingWindow::position`/`initial_size`/`fixed_size` (windows.rs:276/289/303), `Container::limits` (containers.rs:193), `ThemeMetrics::with_metric` (runtime/theme.rs:290), `UiStrings::with` (strings.rs:601), `ViewState::with` (workspace.rs:248), `pert_ui::App::size`/`min_size`/`max_size` (app.rs:342/353/366). They are found with `grep -E '\((mut )?self[,)].*-> Result<Self' candidates/errors.txt`.
2. **callbacks.txt lists only `on_*` names.** Callback-taking builders and constructors with other names are absent, although B3's timing and shape rules apply to them: `Button::new`/`icon`/`command`/`command_icon`, `Link::new`, `MenuItem::new`/`command`, `Chip::removable`, `Breadcrumb::new`, `HistoryButtons::new`, `HistoryNavigator::new`, `QuickPick::new`, `Tabs`/`VirtualList`/`Tree::reorderable`, `VirtualList::placement`, `Toast::action`, `Table::cell_editor`, `CommandPalette::outcome_text`, `FileBrowserController::new` (`request`), `Runtime::register_command`, `register_held_input`, `defer`, `spawn`, `start_interval` and `mailbox`. The semantic lists contain them (63 functions with an `impl Fn*` parameter not named `on_*`). claude-B3-3 was found there.

The configuration markers on the facade entries checked (`App::*` and `Svg::*`) match the feature gates in source.

## Roles relied on

| Path | Role | Source reference |
|---|---|---|
| Component description types with builders (Button, Checkbox, … ZoomView; every type owning a builders.txt entry or a fallible builder) | builder | builders.txt; each type is `#[must_use]` (attributes.txt) or every builder method is, or it returns `Result` |
| Semantics, SemanticItem, ToolWindowChange, WorkspaceSettings, ClickCounter, UiZoom, Permission, Rect, PointerEvent | value with builder-style methods | diagnostics.rs:515-640; method-level `#[must_use]` |
| PickItem | data builder (QuickPick row) | quick_pick.rs:34-80 |
| ParameterHost, AnyParameterHost, RemoteHost | handle (authority owner, drop removes) | parameter_binding.rs:495, 626; erased.rs:247-255; remote.rs:420-423 |
| SeriesSource | handle (owns the consumer; drop closes) | live.rs:600-650 |
| SeriesConsumer, SeriesEndpoint, Mailbox, InspectionService, EditSession | handle (drop closes or cancels) | attributes.txt reasons |
| ListController, TableController, TreeController, DialogController, QuickPickController, CommandPaletteController, FileBrowserController, WindowController, ToolWindowController, ZoomController, DraftEditorController, EditorSession, ListSelection, FieldHandle | handle (controller) | semantic list; constructor or type `#[must_use]` |
| Signal, ReadSignal, Parameter, AnyParameter, Notifier, UiSender, LiveWaker, InspectionClient | handle (shared binding or sender; discard harmless except at creation) | signal.rs; mailbox.rs |
| BackgroundTask, IntervalTimer | handle (no Drop; discard leaves work running, normal) | `rg 'impl Drop'` (none for these) |
| TaskReceipt, RevealReceipt, ListRevealReceipt, ToastReceipt, CommandReceipt, PresentationReceipt, AuthorityReceipt | receipt | runtime/tasks.rs:17-28; reveal.rs:121-133; virtual_list.rs:220-229; notifications.rs:185-196; commands.rs:1593-1607; native_windows.rs:936-950; remote.rs:80 |
| CommandId, HeldInputId | identity of a registration (drop keeps it) | commands.rs:377-380; held.rs:5-8 |
| PresentationRequest, PreparedDetach, DirectoryRequest, RequestTicket, Pending, PickPending | request token | native_windows.rs:1011-1015, 241-245; file_browser.rs:196; async_source.rs:28; inspection.rs:565 |
| FlushReport, WorkspaceReport | result report (discard normal) | semantic list |
| Value types such as Capacity, ParameterId, FontId, NativeWindowId and Timestamp | value with an invariant or an identity (type `#[must_use]`; discard of a constructed value is a mistake) | attributes.txt |
| Remaining types (enums, errors, events, layout values) | other; not B2 candidates | types.txt |
