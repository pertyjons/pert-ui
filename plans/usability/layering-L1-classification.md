# Layering part L1: classification record

Review record of [the layering plan's part L1](layering-plan.md#part-l1-documentation-layering-0371)
(0.3.71). It holds the two tables the plan requires before anything moves:
the per-obligation classification of `docs/contracts.md` (step 1) and the
callback classification from which the timing classes are derived (step 2).
Line numbers refer to the files at 46a219b, before L1.

## 1. contracts.md by obligation

Classes:

- **A, application:** an ordinary application relies on it: observable
  behavior of built-in components or of the public API an application calls.
  A rule a custom component also follows stays A when an application
  observes it.
- **W, extension:** needed only to implement a `Component` or to drive
  `Runtime` as a custom host.
- **M, maintainer:** implementation rationale or internal ordering that no
  application or extension relies on beyond an A or W rule.

Only M moves to `docs/internals.md` (decision LD3). W moves under "For
component and host authors", after the application sections. A stays in
place. A paragraph that mixes classes is split, and the guarantee it gives
the lower layer stays in that layer, reworded only where its sentence named
the moved mechanism.

| # | Section, lines | Obligation | Class | Now |
|---|---|---|---|---|
| 1 | Choice sources, 42-50 | option ownership chosen at construction; validation; same-key redescription | A | stays |
| 2 | Choice sources, 52-56 | the vocabulary's scope (content collections and keyed containers are outside) | A | stays |
| 3 | Element trees, 60-73 | keys, `WidgetId` incarnations, keyed reconciliation, `StateOwner` constructors, descriptions not `Send`; a custom component uses the same operations | A | stays |
| 4 | Captions, 79-88 | caption `TextSource` reading, subscription, redescription, `ForeignRuntime` | A | stays |
| 5 | Captions, 88-90 | custom components follow it through `TextSource::read`, `read_paint`, `same_as` | W | "Describing and redescribing components"; a pointer stays |
| 6 | Text beyond the limits, 94-111 | ellipsis cutting, `Label` refusal and recovery, help labels | A | stays |
| 7 | Placeholders, 115-122 | placeholders and library texts take `TextSource`; roles | A | stays |
| 8 | Titles, 126-133 | titles take `TextSource`; bound `with_label` read before the parent | A | stays |
| 9 | Titles, 135-141 | blank fixed and bound titles refused; breadcrumb, table, toast texts | A | stays |
| 10 | Titles, 143-147 | a detached window follows its view's title; App applies it | A | stays, reworded without the mechanism |
| 11 | Titles, 144-152 | `PresentationAction::Retitle` through `set_window_title`; receipts, re-asking, 250 ms retry | W (host) | "Custom hosts" |
| 12 | Titles, 152-153 | `NativeWindowOptions` keep the opening title | A | stays |
| 13 | Control adornments, 157-168 | icon slots separate from captions; parameter names | A | stays |
| 14 | Redescription, 172-176 | new description's texts, bindings, permissions and callbacks apply; a different binding ends the edit | A | stays |
| 15 | Redescription, 178-183 | reconciliation publishes no signal; the parameter lifecycle exception | A | stays (AGENTS.md cites it) |
| 16 | Redescription, 185-196 | controller-owned components keep controller and options; revocation on redescription | A | stays |
| 17 | Redescription, 198-206 | redescription applied without validation; up-front rejection through `check_description` | A | stays |
| 18 | Redescription, 206-207 | `redescribe` and `check_redescription` | W | "Describing and redescribing components" |
| 19 | Redescription, 209-216 | values from application data refused at adoption | A | stays |
| 20 | Redescription, 218-225 | conformance helpers' cleanup and errors | W | "Describing and redescribing components" |
| 21 | Retained generated children, 229-241 | `retain_initial_children`, part regeneration, `StateOwner::ui_settings` | W | "Retained generated children" under the W heading |
| 22 | Value-driven children, 245-261 | `Dynamic`, `Show`, `Each` reconciliation, tracking, `Show` versus `visible_signal` | A | stays |
| 23 | Controller ownership, 265-268 | one view per `ZoomController` | A | stays |
| 24 | Controller ownership, 270-275 | ColorPicker popover ownership; Popover adoption | A | stays |
| 25 | Menus, 279-283 | menu open-state ownership and callback replacement on redescription | A | stays (the plan names it) |
| 26 | Menus, 285-292 | QuickPick and CommandPalette configuration per session | A | stays |
| 27 | Menus, 294-295 | a changed surface kind is admitted again | A | stays |
| 28 | Menus, 297-298 | error recovery records the kind and retries admission | M | internals, "Surface admission and command queries" |
| 29 | Menus, 298-304 | pending and downgraded ApplicationDialog blocking; hidden dialogs | A | stays |
| 30 | Menus, 300-302 | no-flush queries read current kind rather than cached admission kind | M | internals |
| 31 | Menus, 306-308 | command queries see current declarations, evaluate no pending update, run no admission callback | A | stays |
| 32 | Menus, 306-311 | "missing from the last synchronized frontier"; one pass per listing; frontier without a second walk | M | internals |
| 33 | Menus, 309-310 | invocation and native key routing flush every root before admission | A | stays |
| 34 | Menus, 313-314 | a native-key preflush failure prevents dispatch and cancels input | A | "Failures and input cleanup" |
| 35 | Menus, 314-315 | older and external failures keep their preservation policy | M | internals, "Failure provenance and input cleanup" |
| 36 | Permissions, 319-332 | `Permission` composition; bound permissions at update; table and list restrictions | A | stays |
| 37 | Permissions, 334-340 | adjective versus `*_permission` | A | stays |
| 38 | Permissions, 342-345 | bool part properties | A | stays |
| 39 | Permissions, 347-358 | `enabled`, `disabled()`, redescription replacement, MultiSelect | A | stays |
| 40 | Permissions, 360-363 | builders replace; editors' `enabled` and `editable` | A | stays |
| 41 | Permissions, 365-372 | revoked enablement and editability per editor | A | stays |
| 42 | Permissions, 374-378 | sliders, pickers, collapsibles | A | stays |
| 43 | Permissions, 380-382 | history and geometry controls; `movable` | A | stays |
| 44 | Permissions, 384-389 | detachment permission; tool-window drag | A | stays |
| 45 | Geometry notifications, 393-402 | committed geometry callbacks, once per flush, last value | A | stays |
| 46 | Container composition, 406-410 | `leading_icon` slots; `View::fixed` | A | stays |
| 47 | Container composition, 412-419 | blank and duplicate keys and labels refused before mounting | A | stays |
| 48 | Container composition, 421-427 | an open dropdown that becomes read-only or disabled | A | stays |
| 49 | Layout, 431-435, 438-439 | minimums propagate; Row's offered height; Grid tracks | A | stays |
| 50 | Layout, 435-438 | `fills_row_height`; measurement context for custom containers | W | "Layout and focus hooks"; a pointer stays |
| 51 | Layout, 441-457, 460-462 | grow weights, shrinking, `Justify`, `Align`; Wrap and Grid do not grow | A | stays |
| 52 | Layout, 457-459, 461 | `ArrangeContext` allocation; `MeasureContext::grow` | W | "Layout and focus hooks" |
| 53 | Layout, 464-481 | containers, inactive tabs, focus on hiding and removal, `request_focus_within`, pending focus requests | A | stays |
| 54 | Layout, 483-492 | Tab stops, `descendant_tab_stops`, `item_tab_stops` | A | stays |
| 55 | Layout, 494-501 | `tab_navigation` offer and key ownership | W | "Layout and focus hooks"; the guarantee that bindings never see an unmodified Tab stays |
| 56 | Layout, 503-505 | detachment opt-out; multi-window experimental | A | stays |
| 57 | User-write callbacks, 509-524 | boolean, choice and MultiSelect `on_change` | A | stays; names its timing class |
| 58 | User-write callbacks, 525-536 | `MenuItem::on_change`, `ColorPicker::on_change` | A | stays |
| 59 | User-write callbacks, 538-546 | parameter sinks cannot write; refresh mailbox; `write_base` | A | stays; names its timing class |
| 60 | Ownership paths, 552-559 | the four ownership paths | A | stays |
| 61 | Controlled controls, 563-625 | controlled presentation, refusal, callback timing, redescription | A | stays |
| 62 | Parameters, 629-640 | identities, specifications, `NumberFormat` identity, families, terminal reservation | A | stays |
| 63 | Component notifications, 644-665 | observable selection and expansion; origins; coalescing of collections | A | stays |
| 64 | Component notifications, 667-670 | `Notifier` and `deliver_notifications` for changes made without an event context | W | "Reporting component notifications" |
| 65 | Component notifications, 672-676 | delivery after settling, order, writes settle in the same flush | A | stays, worded as the "after settling" class |
| 66 | Component notifications, 678-686 | input entry points settle; failing callbacks; removed components and callbacks | A | stays |
| 67 | Delivery order, 690 | global FIFO across visible windows | A | stays |
| 68 | Delivery order, 692-694 | hidden windows retain; hidden components in visible windows receive | A | stays |
| 69 | Delivery order, 694-696 | preparation owner, repeated settlement, frontiers not replayed | M | internals, "Notification delivery" |
| 70 | Delivery order, 696-697 | callback failure consumes its request; preparation failure retains | A | stays |
| 71 | Delivery order, 699-701 | reentrant requests: same flush or next | A | stays |
| 72 | Delivery order, 701-704 | notification and reveal budgets | M | internals |
| 73 | Delivery order, 706-709 | ToolWindows delivery slots; housekeeping coalescing | M | internals |
| 74 | Failure provenance, 713-718 | which failures cancel which input | M | internals; a one-paragraph A summary stays ("Failures and input cleanup") |
| 75 | Failure provenance, 720-735 | collective errors, headless cleanup, fresh contexts, weak provenance | M | internals |
| 76 | Failure provenance, 737-742 | command actions' and deferred tasks' returned-error boundary | A | stays |
| 77 | Failure provenance, 742-744 | reveal and parameter delivery provenance | M | internals |
| 78 | Failure provenance, 746-747 | the hook and `Notifier` are public | W | "Reporting component notifications" |
| 79 | Producer data, 751-760 | callback-safe endpoints, mailbox frontiers, `keep_alive` | A | stays; names its timing class |
| 80 | Producer data, 762-773 | `Runtime::spawn` and `EventContext::spawn` | A | stays |
| 81 | Producer data, 775-779 | `start_interval` | A | stays |
| 82 | Producer data, 781-785 | telemetry freshness and revisions | A | stays |
| 83 | Host-loaded data, 789-803 | `RequestTracker` tickets and statuses | W | "Host-loaded data" under the W heading |
| 84 | Physical keys, 809-819 | held keys apart from text; numpad; `control_key` meaning | A | stays (keyboard behavior applications see; names a W item) |
| 85 | Physical keys, 821-825 | held actions end on focus loss, hiding, unmount; modality ownership | A | stays |
| 86 | Stacking, 829-835 | `child_stacking_priority`; custom parents request layout | W | "Stacking, hit testing and pointer refresh" |
| 87 | Stacking, 832-834 | ToolWindows layering; frames raise among peers | A | stays |
| 88 | Stacking, 837-847 | pointer refresh moves; converging handlers | W | "Stacking, hit testing and pointer refresh" |
| 89 | Stacking, 849-853 | `hit_test` body region | W | "Stacking, hit testing and pointer refresh" |
| 90 | Stacking, 853-859 | shared hit order; outside-press extent; toasts | A | stays |
| 91 | Activation keys, 863-876 | activation and navigation key admission | A | stays |
| 92 | Commands, 880-904 | scopes, strings, modal and nonmodal rules, Escape, text commands | A | stays |
| 93 | Key entry, 908-914 | `command_key_input` versus `key_input` | A | stays |
| 94 | Reveal and focus, 918-936 | reveal path, refusals, receipts | A | stays (names the hook) |
| 95 | Content search, 940-944, 945-950 | `inspect`, `try_content_rows`, `mounted_rows_current` | W | "Content search hooks"; an A summary stays |
| 96 | Content search, 944-945 | hidden lists expose their bound source | A | stays |
| 97 | Content search, 950-951 | retained row children excluded after a failed reconciliation | M | internals, "Content search" |
| 98 | Content search, 952-968 | read errors, budgets, `Stale`, modality, exclusions, jumps | A | stays |
| 99 | Command palette, 972-979 | palette snapshot, closing before invocation, reopening | A | stays |
| 100 | Drag and drop, 983-990 | session outcomes; built-in reordering; application order | A | stays (built-in reordering is defined by the outcomes) |
| 101 | Closing a window, 994-1034 | admission order, editor policies, backoff, `KeepOpen`, `HeadlessApp` | A | stays |
| 102 | Closing a window, 1035 | the steps a host other than `App` follows | W (host) | "Custom hosts"; a pointer stays |
| 103 | Shutdown, 1039-1044 | sessions cancelled; cleanup errors typed in `take_shutdown_errors` | A | stays |
| 104 | Shutdown, 1044-1049 | queued cleanup causes, `OwnerRemoved`, order within a window | M | internals, "Runtime shutdown" |
| 105 | Shutdown, 1049-1050 | no global chronology promised | A | stays |
| 106 | Shutdown, 1050-1051 | hosts using `Runtime` drain the report | W (host) | "Custom hosts" (the plan names it) |
| 107 | Shutdown, 1051-1060 | App drains; `AppError::Shutdown`; timers; `keep_alive` order | A | stays (the plan names the `keep_alive` order) |
| 108 | Calling the runtime, 1064-1091 | contexts, the three runtime callbacks, flushing operations, nested flushes, deferral binding | A | stays |
| 109 | Text editing, 1095-1114 | editors, drafts, `on_rejection`, `on_binding_refusal` | A | stays |
| 110 | Editor sessions, 1118-1145 | `EditorSession` protocol | W | "Editor sessions" under the W heading |
| 111 | Editor sessions, 1147-1154 | a table's cell edits: one at a time, row kept, Tab order, when edits close | A | "Table cell edits" |
| 112 | Editor sessions, 1147-1150 | the table creates the session from its notifier, `keep_mounted` | W | "Editor sessions" |
| 113 | Search field, 1158-1169 | query, Escape order, debounce | A | stays |
| 114 | Theme palettes, 1173-1192 | palettes, seeding, equality, repaint | A | stays |
| 115 | Workspace, 1196-1209 | document, validation, complete capture | A | stays |
| 116 | Capture with children, 1213-1218 | `workspace_state_with_children`; Accordion and Tabs | W | "Workspace hooks"; an A summary stays |
| 117 | Restore with children, 1222-1231 | `restore_workspace_state_with_children`; table restore check | W | "Workspace hooks"; an A summary stays |
| 118 | Restore and active gestures, 1235-1239 | restore cancels gestures when layout changes | A | stays (the plan names it) |
| 119 | Inspection, 1243-1257 | snapshots, semantics, naming precedence, schema version | A | stays |
| 120 | Inspection, 1253-1254 | custom components reach the precedence | W | "Inspection in custom components" |
| 121 | Inspection, 1258-1267 | geometry validation, event history, secret input | A | stays |
| 122 | Rendered capture, 1271-1282 | captured frames, admission causes, typed failures, serde | A | stays |
| 123 | Rendered capture, 1284-1287 | frames a host supplies wrongly | W (host) | "Custom hosts" |
| 124 | Compatibility policy, 1291-1306 | unpublished and published compatibility | A | stays (AGENTS.md cites it) |
| 125 | Error policy, 1310-1373 | typed errors, derives, messages, conversions, `# Errors` | A | stays |
| 126 | Naming rules, 1377-1498 | names, limits, parameters order, builders, `with_`, handle and record names | A | stays |

The inventory placed the compatibility, error and naming sections among the
maintainer-only ones. Per obligation they describe the shape of the public
API a consumer relies on (closed and growing enums, typed errors, what a name
means) and contain no implementation rationale or internal ordering, so
they stay; AGENTS.md also cites the compatibility policy in contracts.md.

Summary: 126 obligations: 91 A, 23 W (4 of them a custom host's) and
12 M. Of the paragraphs the plan named, menu binding and callback
replacement (row 25), the `keep_alive` order (row 107) and restore cancelling
gestures (row 118) are A, and a custom host draining shutdown errors
(row 106) is W.

### Links checked

- Incoming fragment links to contracts.md from `docs/`, `plans/`, rustdoc,
  the guide, the crate roots, README, AGENTS.md and TODO.md all target
  sections that stay: `#identity-and-composition`, `#choice-sources`,
  `#component-redescription`, `#data-and-edit-ownership`,
  `#ownership-paths`, `#component-notifications`, `#user-write-callbacks`,
  `#workspace-persistence` and `#inspection-and-diagnostics`.
- Bare references by section name ("the compatibility policy" in AGENTS.md,
  "Error policy" in `error.rs`, "Closing a window" in the fix plan, "drag
  and drop" in the facade's drag test) name sections that stay.
- Line-number citations of contracts.md in `plans/` (the fix plan, the
  audit reports and designs, the inventory, this plan) record the file at
  their own commit and are left as they are.
- Moved text keeps its own fragment links (`#captions`, `#closing-a-window`,
  `#rendered-capture` and the others resolve in contracts.md); internals.md
  links each note back to its contracts.md section.

## 2. Callback timing

Coverage, as the plan defines it: every public API that takes a callback
the runtime calls later or during an event, from the `api/` listings at
46a219b (`pert-ui-runtime`, `pert-ui`, `pert-ui-wgpu`; `pert-ui-core` and
`pert-ui-cpu` take none). "Writes" means binding and parameter writes.

### Derived classes

Sorting the rows below by the moment each callback runs gives five moments
that cover all but eight of them:

| Class | Moment | Rows |
|---|---|---|
| In the input | inside the input call that caused it | 1-33 |
| After settling | event phase after the flush's update and layout settle | 34-40 |
| At the next flush | start of a later flush, before components update | 41-47 |
| At parameter delivery | when a parameter edit's event is delivered | 48-50 |
| Evaluation | whenever the runtime needs the value | 51-58 and the group below |

The classes are timing only. The table shows that access, coalescing,
cancellation and failure vary inside each class (rows 11, 13 and 25 differ
from row 8 in the input class; rows 34, 36 and 38 coalesce differently after
settling), so contracts.md defines them as separate properties.

### Rows

| # | API | Class | Access | Coalescing | Cancellation | Failure and replay |
|---|---|---|---|---|---|---|
| 1 | `Button::new`, `Button::icon` | in the input | writes | one call per activation | redescription replaces | propagates from the input call |
| 2 | `Button::command`, `Button::command_icon` (`on_receipt`) | in the input; the command runs at the next flush | writes | per activation | redescription replaces | propagates; neither command nor handler replayed |
| 3 | `Link::new` | in the input | writes | per activation | redescription replaces | propagates |
| 4 | `Chip::removable` | in the input | writes | once per request | redescription replaces | propagates |
| 5 | `ColorSwatch::on_activate` | in the input | writes | per completed activation | replaced | propagates |
| 6 | `Toast::action` | in the input | writes | per activation | expiry or the owner's removal skip it | propagates |
| 7 | `Breadcrumb::new` | in the input | writes | per activation | redescription replaces | propagates |
| 8 | `Checkbox::on_change`, `Toggle::on_change`, `InlineToggle::on_change` | in the input, after the write or submission | writes | each user write | redescription replaces or removes | propagates; no rollback or replay |
| 9 | `Checkbox::controlled`, `Toggle::controlled` | in the input, where `new` writes, with the proposal | writes | each activation; two before a redescription propose twice | a changed value cancels a pending activation | propagates; nothing to roll back |
| 10 | `RadioGroup::on_change`, `SegmentedControl::on_change`, `InlineChoice::on_change` | in the input | writes | each write or submission | redescription replaces | no rollback or replay |
| 11 | `Select::on_change`, `InlineSelect::on_change` | in the input, after the list closes | writes; the context is the closing list's | each write or submission | redescription replaces without closing | no rollback or replay |
| 12 | `MultiSelect::on_change` | in the input | writes | each write of the bound value | redescription replaces | the write and close stay; not replayed |
| 13 | `MenuItem::new`, `MenuItem::command` (`on_receipt`), `MenuItem::on_change` | in the input, once closing is requested | writes | each choice | the row's description | the write stays; not replayed |
| 14 | `ColorPicker::on_change` | in the input that committed | writes | each committed change | follows same-key redescription | the color stays; not repeated |
| 15 | `TextField::on_save`, `TextArea::on_save` | in the input (Enter or focus loss) | writes | each save that wrote | replaced | the text stays written |
| 16 | `IntegerInput::on_save`, `DecimalInput::on_save`, `InlineInteger::on_save`, `InlineDecimal::on_save` | in the input; a held step's repeat in its timer's flush | writes | each save | replaced | from the input call or the flush |
| 17 | `InlineText::on_save`, `InlineText::controlled` | in the input (Enter, focus leaving) | writes | each changed save | each description replaces | propagates; not replayed |
| 18 | `on_rejection` of `TextField`, `TextArea`, `InlineText`, `IntegerInput`, `DecimalInput`, `InlineInteger`, `InlineDecimal` | in the input | writes | `Some` per refused save, `None` once | replaced | a failed `None` stays pending |
| 19 | `TextField::on_submit` | in the input (fresh Enter) | writes | per Enter | replaced | propagates |
| 20 | `normalize` of the seven editors above | in the input, at the save | none: pure (`InlineText`'s may read application state) | per save | replaced | a refusal keeps the draft |
| 21 | `DraftEditorController::on_edit` | in the input | writes; may call the controller | each user change | replaced | the change stays |
| 22 | `SearchField::on_key` | in the input | writes | each reported key | replaced | propagates |
| 23 | `VirtualList::on_activate`, `Tree::on_activate`, `Table::on_activate` | in the input (Enter) | writes | each | redescription | propagates |
| 24 | `VirtualList::on_text_input` | in the input | writes | each committed text | redescription replaces or removes | propagates |
| 25 | `VirtualList::reorderable`, `Tree::reorderable`, `Tabs::reorderable` | in the input (drop, Alt with an arrow) | writes; returns whether it edited | one per drop | revocation cancels an armed or active drag | propagates; a refusal tells the source |
| 26 | `VirtualList::placement` | in the input (each move of a drag) | answers from the model | every movement | none | none: returns a placement |
| 27 | `Table::on_sort` | in the input | writes | each sort request | redescription | propagates |
| 28 | `QuickPick::new` (`choose`) | in the input | writes; answers a `PickChoice` | each choice | redescription replaces for later choices | propagates; the picker stays as it was |
| 29 | `HistoryNavigator::new`, `HistoryButtons::new` (`restore`) | in the input | writes | each move | redescription replaces | propagates |
| 30 | `FileBrowserController::on_open` | in the input | writes; may call the controller | each activation | replaced | propagates |
| 31 | `FloatingWindow::on_close` | in the input (close button) | writes; answers an action | once per activation | redescription | the window stays open; not replayed |
| 32 | `Runtime::event`, `Runtime::try_event` | in the input it dispatches | writes; may begin gestures | once | none | returned |
| 33 | `Runtime::register_held_input` (action) | in the input (key routing) | answers an `InputOutcome` | press, repeats, release | the registration's owner | an error cancels owned keys |
| 34 | `Tabs::on_select` | after settling | writes and queues | each settled change | a description without one discards | returned from the flush; change stays; the rest delivered |
| 35 | `Accordion::on_change` | after settling | writes | each change, in order | as row 34 | as row 34 |
| 36 | `VirtualList::on_selection_change`, `Tree::on_selection_change`, `Table::on_selection_change` | after settling | writes | writes between two updates merge | as row 34 | as row 34 |
| 37 | `ToolWindowLayout::on_change` | after settling | writes | each change | as row 34 | returned after every pending change |
| 38 | `ScrollArea::on_scroll`, `SplitPane::on_resize`, `RegionLayout::on_resize`, `PropertyGrid::on_resize`, `FloatingWindow::on_frame` | after settling | writes | once per flush and component, last value | as row 34 | returned; the value stays; not replayed |
| 39 | `TextField::on_modified`, `TextArea::on_modified` | after settling | writes | each change, in order | none | returned from the flush or input call |
| 40 | `TextField::on_binding_refusal`, `TextArea::on_binding_refusal` | after settling | writes | each refusal and its end; none within one flush | a description without one stops it | returned; the refusal stays |
| 41 | `EventContext::defer` | at the next flush | `&mut Runtime`, may flush | once | the widget hidden or unmounted first | from the flush and the receipt; not replayed |
| 42 | `EventContext::defer_detached` | at the next flush | as row 41 | once | the window closed, shutdown | as row 41 |
| 43 | `Runtime::defer` | at the next flush | as row 41 | once | shutdown | as row 41 |
| 44 | `EventContext::spawn` | at the next flush (the start); its result as row 45 | `work` holds no UI handle | once | the window closed, shutdown | a refusal fails the flush and the receipt |
| 45 | `Runtime::spawn` (`deliver`) | at the next flush after arrival | writes | arrival order with the mailboxes | `BackgroundTask::cancel`, shutdown | returned from that flush; a panic is `BackgroundPanicked` |
| 46 | `Runtime::mailbox` (handler) | at the next flush after arrival | writes | in order; frontiers captured per flush | closing or dropping the mailbox | returned; the value is consumed |
| 47 | `Runtime::start_interval` | at the next flush at or after the due time, or `service_intervals` | `&mut Runtime` | a late tick does not catch up | cancelling, shutdown | the timer keeps running |
| 48 | `ParameterHost::on_outcome`, `AnyParameterHost::on_outcome` | at parameter delivery | no writes | every terminal event once | the registration | retained in the monitor for `retry_outcome` |
| 49 | `ParameterHost::on_preview`, `AnyParameterHost::on_preview` | at parameter delivery | no writes | each accepted preview once, in order | the registration | retained |
| 50 | `RemoteHost::on_submit` | at parameter delivery | no writes | once per proposal | shutdown releases it | retained; no automatic retry |
| 51 | `MultiSelect::filter` | evaluation (settling derived values) | none: pure | every option on each change | replaced | none |
| 52 | `FilterMatcher::new` | evaluation | none: pure | per query and source | replaced | none |
| 53 | `PickMatcher::new`, `CommandMatcher::new` | evaluation | none: pure | per query | replaced | none |
| 54 | `NumberFormat::new`, `NumberFormat::edit_text`, `NumberFormat::unit` | evaluation; `parse` in the input at a save | none: pure | per value | replaced | a parse error refuses the draft |
| 55 | `CommandPalette::outcome_text` | evaluation | none: pure and bounded | per outcome | redescription, for later choices | none |
| 56 | `PickPending::new` (`resolve`) | evaluation | none: pure and bounded | per read | none | none |
| 57 | `Runtime::command_enabled` (predicate) | evaluation (command queries and invocation) | `&Runtime`, read-only | per query | unregistering | an error stops invocation |
| 58 | `InspectionService::set_application` (provider) | evaluation (while answering) | none | per answer | replaced | none |

### Evaluation callbacks, apart from the rows

- **Binding derivations.** `ReadSignal::map` and `ReadSignal::combine` run
  once when created, then at most once per batch of input writes or at an
  earlier explicit read, while the runtime settles derived values before
  component updates; they must be pure, and a write from them is rejected.
- **Description factories.** `Dynamic::new`, `Show::new` and `Each::new`'s
  key and build callbacks and `Each::within` run during the update of the
  component that describes from them, when the source binding was written
  with a new value or the component was described again; they must be
  pure, and a write from them is rejected. `Table::cell_editor` and the row
  traits' `ListItems::build`, `TableItems::cell` and `TreeItems::build` are
  description factories of the same kind, run in the collection's update.

### Callbacks that fit no class

These keep their own description in rustdoc, which already states it:

| API | Its own moment |
|---|---|
| `App::new` (initializer) | once in `run`, after the event loop exists and before any window opens |
| `App::on_close` | at close admission, after focus release and the flush, with the closing window selected |
| `App::on_error` | after input cleanup, outside runtime evaluation |
| `App::on_window_error` | after a failed asynchronous presentation, on the UI thread |
| `Runtime::register_command` (action) | at invocation: in the input for a routed chord, at the next flush for an invocation queued with `EventContext::invoke_command`, inside `Runtime::invoke_command` |
| `Runtime::register_held_input` (cancellation) | wherever its input is cancelled; infallible cleanup |
| `Popover::on_dismiss` | in the input that closes the surface, or where the runtime closes it (hidden or removed owner, lost focus, no space) |
| `FileBrowserController::new` (`request`) | during the call, the controller's methods and the browser's update; no writes |

### Excluded

- **Host integration**, with its own threading and side-effect contracts:
  `Runtime::set_wake_delivery` (runs on the notifying thread),
  `pert_ui_wgpu::Renderer::new`'s notice callback and
  `Renderer::render`'s presentation callback, the `Clipboard` service and
  the `Component` hooks, whose own documentation places each.
- **Outside the coverage**, because they run before the call that takes
  them returns: `Signal::update`, `Painter::with_clip`,
  `SeriesSource::with_window`, `UpdateContext::read_series`,
  `RequestTracker::retain`, the providers of `Runtime::inspect_with` and
  `Runtime::query_with`, and `testing::Driver::wait_until`'s condition.

### Rustdoc

Each API in rows 1-58 and the group's `ReadSignal::map`, `combine`,
`Dynamic::new`, `Show::new`, `Each::new`, `Each::within` and
`Table::cell_editor` name their class with a link to its contracts.md
section, beside their existing wording (95 rustdoc comments, macro-generated
ones counted once). The row traits' methods, the callbacks that fit no class
and the excluded ones are unchanged.
