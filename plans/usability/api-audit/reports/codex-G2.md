# Codex report: G2, builders and callbacks

Audited commit: `d4ba108f66b7c8ffbf350ae01bedc95d147cd7e6`; `crates/` matches that commit.

## Dispositions

Counts exclude headers and count defining items once across configurations. Candidate entries were not sampled. Existing tests were read selectively as corroboration; no tests were executed. No files were modified.

| Item | Disposition | Coverage |
|---|---|---|
| B1 | Findings: one policy question concerning boolean shortcuts. No additional verified replacement, Permission-composition or revocation departures found. | Processed all **530 builders**, including **108 macro-generated entries**, and all **78 bool-parameter entries**. Read builder bodies and shared macro implementations; supplemented these with **50 consuming `Result<Self, …>`/`Option<Self>` methods** from the semantic lists. Checked Permission composition, redescription, revocation and composite forwarding in controls, editors, collections, geometry controls, menus, history controls and tool windows. |
| B2 | Findings: unnecessary annotations on registration identities; missing annotations on parameter owners. | Cross-checked all **530 builders**, **813 attribute entries** and **534 type entries**. Used all five semantic lists for role triage: **534 types and 1,992 method entries**. Every listed consuming builder has protection through its function, return type or enclosing type; findings concern other roles. Re-examined slice 3 F19. |
| B3 | Findings: pending RegionLayout reports survive callback removal; signals are published during reconciliation. | Processed all **60 callback entries**, their registration/replacement and invocation paths, and public notification payloads. Structurally screened **79 `reconcile` definitions**, then followed relevant helpers. Checked tests covering writes, proposals, refusals, callback failure, replacement/removal, origin, delivery order, editor completion and geometry. |

B3 coverage by callback family:

| Family | Candidate entries | Principal source and corroborating tests read |
|---|---:|---|
| Boolean, single-choice, menu, MultiSelect and color user writes | 11 | `control_binding.rs:77`, `control_binding.rs:129`, `choice_controls.rs:207`, `select.rs:1410`, `menu.rs:175`, `multi_select.rs:307`, `color_picker.rs:263`; `tests/controls/component_callbacks.rs:32`, `tests/surfaces/menu_changes.rs:110`, `tests/collections/multi_select.rs:1257`, `tests/controls/color_picker.rs:1144` |
| Editor save/rejection/modification/refusal/submit, draft editing and search keys | 21 | `text_fields.rs:1484`, `text_fields.rs:1713`, `text_fields/normalize.rs:99`, `text_fields/number_inputs.rs:1193`, `text_fields/inline_text.rs:254`, `text_fields/draft.rs:682`, `text_fields/search.rs:400`; `tests/editing/editor_sessions.rs:390`, `tests/editing/editor_permissions.rs:50` |
| Selection and expansion | 5 | `tabs.rs:1064`, `collapsible.rs:1245`, `virtual_list.rs:906`, `virtual_list.rs:1664`, `tree.rs:1272`, `table.rs:1467`; `tests/collections/selection_notifications.rs:120`, `tests/surfaces/tab_notifications.rs:391` |
| Geometry | 5 | `geometry_report.rs:68`, `views.rs:631`, `split.rs:619`, `region_layout.rs:363`, `property_grid.rs:598`, `windows.rs:1358`; `tests/surfaces/geometry_callbacks.rs:219`, `tests/surfaces/geometry_callbacks.rs:380` |
| Tool-window changes | 1 | `tool_windows/mod.rs:1017`, `tool_windows/mod.rs:1417`; `tests/audit/audit_notification_order.rs:159` |
| Activation, dismissal, close, sorting and collection text input | 9 | `color_swatch.rs`, `file_browser/controller.rs:874`, `windows.rs:564`, `popover.rs:340`, `table.rs:1473`, `table.rs:1930`, `virtual_list.rs:2088`, `virtual_list.rs:2141`; associated facade tests |
| Typed/erased parameter sinks and remote submission | 5 | `parameter_binding/delivery.rs:340`, `parameter_binding/delivery.rs:500`, `parameter_binding/erased.rs:335`, `parameter_binding/remote.rs:157`; `tests/parameter_delivery.rs:46`, `tests/parameter_delivery.rs:159` |
| Native App policies | 3 | `crates/pert-ui/src/app.rs:452`, `app.rs:465`, `app.rs:480`, `app.rs:491`, `app.rs:503`, `app.rs:516` |

Paths in that table are relative to `crates/pert-ui-runtime/` unless otherwise stated. The additional GPU notification family was checked at `crates/pert-ui-wgpu/src/lib.rs:78`, `lib.rs:391` and `lib.rs:432`; its driver-thread signature is explicitly documented and is not an EventContext-family inconsistency.

## Findings

### codex-B1-1 Policy question: additional one-way boolean shortcuts

- **Rubric/rule:** B1, **project rule**; policy question about the scope of the boolean-builder convention.
- **Paths/configurations:** All configurations of `pert-ui-runtime` and its facade re-exports.
- **Observation — signature:** These ordinary flags expose a no-argument operation that sets one boolean value, without a corresponding boolean setter:

  | Paths | Evidence |
  |---|---|
  | `Button::frameless` | `crates/pert-ui-runtime/src/controls.rs:600`, implementation at `:604` |
  | `Group::frameless` | `crates/pert-ui-runtime/src/views.rs:166` |
  | `Field::hide_label` | `crates/pert-ui-runtime/src/field_status.rs:493` |
  | `InspectionQuery::{parameters, windows, sources, validation, application}` | `crates/pert-ui-runtime/src/inspection.rs:246`, `:258`, `:263`, `:268`, `:273` |
  | `Semantics::names_children` | `crates/pert-ui-runtime/src/diagnostics.rs:631` |
  | `PointerEvent::outside` | `crates/pert-ui-runtime/src/pointer.rs:189` |

  For example, `Button::frameless` documents “Default false” and unconditionally assigns `true`. An application choosing the property from a boolean must branch around construction instead of supplying that boolean.

- **Rule:** `docs/contracts.md:1156`: “A builder for an ordinary boolean property takes a `bool`”; `:1158` names `disabled()` and `read_only()` as no-argument shortcuts.
- **Exception assessment:** These are not Permission restrictions. The governing sections do not state an exception for these additional shortcuts. Whether the rule should also permit them is the policy question; their documented behavior is not a functional defect.
- **Severity/confidence:** **Low**; **medium** confidence in rule applicability, high confidence in the signatures and behavior.
- **Suggested fix/breaking:** Either approve and document the additional shortcut convention, or provide boolean setters. Changing existing signatures is breaking; additive setters and a documented shortcut exception need not be.
- **Destination if not chosen:** **TODO.md**, recording the unresolved convention.

### codex-B2-1 Registration identities warn on legitimate discard

- **Rubric/rule:** B2, **project rule**. Includes the independently verified substance of slice 3 F19.
- **Paths/configurations:** `pert_ui_runtime::{CommandId, HeldInputId}`; **all**.
- **Observation — signature and behavior:**
  - `CommandId` has `#[must_use]` at `crates/pert-ui-runtime/src/runtime/commands.rs:379`. Registration stores its own cloned identity and callback at `:565`; lifetime is explicitly controlled by unregistering or removing the scope, as documented at `:511`.
  - `HeldInputId` has `#[must_use]` at `crates/pert-ui-runtime/src/runtime/held.rs:7`. Registration likewise retains its own identity at `:115`; its documented lifetime follows its mounted owner at `:61`.

  Discarding either returned identity does not cancel the registration. Keeping an action for its application/view lifetime without later manipulating its identity is legitimate, but callers must suppress the warning.

- **Rule:** AGENTS.md:181 requires the annotation where “discard is a mistake”; B2 explicitly excludes values whose discard is normal.
- **Exception assessment:** These are passive registration identities, not RAII cancellation guards. Their documented lifetime rules establish the distinction.
- **Severity/confidence:** **Low**; **high**.
- **Suggested fix/breaking:** Remove the type-level annotations. Preserve `Result` handling and explicit unregister APIs. **Nonbreaking**.
- **Destination if not chosen:** **TODO.md**.

### codex-B2-2 Local parameter owners lack `must_use`

- **Rubric/rule:** B2, **project rule**.
- **Paths/configurations:** `pert_ui_runtime::{ParameterHost<T>, AnyParameterHost}`; **all**.
- **Observation — signature and behavior:**
  - `ParameterHost<T>` has no `#[must_use]` at `crates/pert-ui-runtime/src/parameter_binding.rs:495`.
  - Its rustdoc says dropping the owner removes the incarnation and rejects an active edit (`:410`); its `Drop` calls `remove_and_release(true)` at `:627`.
  - `AnyParameterHost` also lacks the annotation at `crates/pert-ui-runtime/src/parameter_binding/erased.rs:255`; its documentation explicitly says dropping it drops the typed owner (`:247`).
  - The sibling ownership type `RemoteHost<T>` already carries a reason explaining that dropping it makes the binding unavailable at `crates/pert-ui-runtime/src/parameter_binding/remote.rs:420`.

  This is an owning lifetime token whose accidental unused expression can remove a parameter, rather than a passive handle that is safe to discard. Existing teardown evidence includes `crates/pert-ui-runtime/tests/parameter_delivery.rs:404`.

- **Rule:** AGENTS.md:181: “Use `#[must_use]` for results/handles/builders whose discard is a mistake.”
- **Exception assessment:** No documented exception applies. Returning the owner inside a `Result` or tuple does not annotate the owner type itself.
- **Severity/confidence:** **Medium**; **high**.
- **Suggested fix/breaking:** Annotate both ownership types with a reason explaining removal on drop. No signature or runtime behavior break; introduces appropriate unused-value diagnostics.
- **Destination if not chosen:** **TODO.md**.

### codex-B3-1 RegionLayout callback removal does not discard pending resize reports

- **Rubric/rule:** B3, **project rule**, geometry notifications.
- **Paths/configurations:** `pert_ui_runtime::RegionLayout::on_resize` and generated `SplitPane` descendants; **all**.
- **Observation — behavior:**
  - RegionLayout promises that removing its callback “also drops changes not yet delivered” at `crates/pert-ui-runtime/src/region_layout.rs:253`.
  - It unconditionally installs a forwarding `SplitPane::on_resize` closure, even when the external callback slot is empty (`:363`).
  - SplitPane discards pending reports during redescription only when its incoming or previous callback is `None` (`crates/pert-ui-runtime/src/split.rs:496`). The unconditional forwarding closures keep both `Some`.
  - RegionLayout’s own reconciliation provides no additional discard operation (`region_layout.rs:271`).

  Consequently this sequence preserves the old pending report: change a described extent with a callback; redescribe without the callback; reinstall a callback before flushing. At delivery, the retained report reaches the newly installed forwarding closure. `GeometryReport::take` consumes the retained payload only at delivery (`crates/pert-ui-runtime/src/geometry_report.rs:68`).

  The existing region test changes extents with an intervening flush; it does not cover removal and reinstallation before delivery (`crates/pert-ui/tests/surfaces/geometry_callbacks.rs:453`).

- **Rule:** `docs/contracts.md:355`: “A description replaces or removes the callback; one without a callback discards changes not yet delivered.”
- **Exception assessment:** RegionLayout explicitly adopts SplitPane’s geometry contract (`region_layout.rs:241`). Its composite forwarding does not document an exception.
- **Severity/confidence:** **Medium**; **high**. Established by source tracing; not reproduced in an executed test.
- **Suggested fix/breaking:** Preserve callback absence when constructing forwarding splits, or explicitly propagate removal to their pending reports. Add a removal/reinstallation-before-flush test. **Nonbreaking bug fix**.
- **Destination if not chosen:** **TODO.md**.

### codex-B3-2 Reconciliation publishes library-owned signals

- **Rubric/rule:** B3, **project rule**. Includes a policy question about internal lifecycle exceptions.
- **Paths/configurations:** `ContextMenu`; parameter-bound `Slider`, `Knob`, `InlineSlider`, `IntegerInput`, `DecimalInput`, `InlineInteger` and `InlineDecimal`; **all**.
- **Observation — behavior:**
  - **Direct publication:** `ContextMenu::reconcile` calls `enabled_signal.publish_trusted(enabled)` at `crates/pert-ui-runtime/src/menu.rs:1472`. The private-field comment explicitly acknowledges publication from reconciliation (`:1258`).
  - **Publication through cancellation:** numeric reconciliation abandons an active parameter session on rebinding or fixed denial (`crates/pert-ui-runtime/src/numeric_controls.rs:568`, `:585`). `abandon` drops the session (`:407`). Number-editor adoption does the same at `crates/pert-ui-runtime/src/text_fields/number_inputs.rs:1274` and `:1309`, through parameter-source helpers at `:616` and `:761`.
  - `EditSession::drop` invokes cancellation (`crates/pert-ui-runtime/src/parameter_binding.rs:902`), which immediately publishes the parameter snapshot at `:678`; only terminal sink delivery is deferred. `Inner::publish` calls `Signal::publish_trusted` at `:178`. That function changes signal storage and invalidates dependents immediately (`crates/pert-ui-runtime/src/signal.rs:431`).

- **Rule:** AGENTS.md:132: “Never publish signals from reconciliation”; B3 repeats that requirement.
- **Exception assessment:** `docs/contracts.md:167` requires cancellation of the old parameter gesture at a safe point. `ParameterHost` rustdoc also says lifecycle cleanup applies immediately (`parameter_binding.rs:432`). Neither explicitly exempts trusted signal publication from the reconciliation rule. The implementation’s “library-owned” comment is not a governing exception.
- **Severity/confidence:** **Medium**; **high** confidence in publication paths. Whether immediate internal lifecycle snapshots should be exempt is a **policy question**.
- **Suggested fix/breaking:** ContextMenu’s opener gate can use nonsignal shared state. For parameter cleanup, resolve the publication-phase contract while preserving cancellation and terminal outcomes; either defer publication appropriately or approve a narrow lifecycle exception. Private storage changes need not break API; changing observable snapshot timing requires contract review.
- **Destination if not chosen:** **TODO.md**.

## Inventory gaps

### Conditional App method missing from the semantic inventory

`plans/usability/api-audit/inputs/semantic/pert-ui.txt:5` lists `App` and its methods but omits `App::software_rendering`.

The method exists at `crates/pert-ui/src/app.rs:412` under `#[cfg(feature = "cpu")]`. It is correctly present in:

- `plans/usability/api-audit/inputs/candidates/builders.txt:536`, for `cpu` and `no-default-desktop-cpu`;
- `api/pert-ui/cpu.txt:95`;
- `api/pert-ui/no-default-desktop-cpu.txt:95`.

Thus the mechanical builder inventory contains it, but the semantic method inventory is incomplete across configurations. Merge conditional methods into that inventory and retain their configuration annotations.

No other inventory gap was verified. The additional fallible consuming builders are not gaps: `builders.txt` explicitly limits itself to `self -> Self`, and the semantic lists contain those methods.

## Roles relied on

`R` below means `pert_ui_runtime`; brace notation groups paths with the same classification. These are the classifications material to the findings and boundary decisions.

| Path | Role | Source reference |
|---|---|---|
| `pert_ui_core::{ParameterSpec, SeriesConfig}` | Specification/configuration builders | `crates/pert-ui-core/src/parameter.rs:151`; `src/live/mod.rs:244` |
| `R::{Button, Label, Chip, Link, Checkbox, Toggle, InlineToggle, RadioGroup, SegmentedControl, InlineChoice}` | Component descriptions with property builders | `controls.rs:445`; `chip.rs:72`; `link.rs:76`; `choice_controls.rs:490`; `inline_controls.rs:54` |
| `R::{Select, InlineSelect, MultiSelect}` | Composite component descriptions; retained popup state does not exempt their user-write handlers from replacement | `select.rs:1362`; `select.rs:1740`; `multi_select.rs:1209`, `:1410` |
| `R::{ColorPicker, ColorSwatch}` | Component descriptions; picker retains binding/options but replaces handler and Permission | `color_picker.rs:386`, `:712`; `color_swatch.rs:96` |
| `R::{TextField, TextArea, IntegerInput, DecimalInput, InlineInteger, InlineDecimal, InlineText, SearchField, DraftEditor}` | Editor descriptions/builders; drafts follow editor-specific callback contracts | `text_fields.rs:154`, `:203`; `text_fields/number_inputs.rs:166`, `:1916`; `text_fields/inline_text.rs:66`; `text_fields/search.rs:107`; `text_fields/draft.rs:582` |
| `R::{Slider, Knob, InlineSlider}` | Parameter-control descriptions/builders | `numeric_controls.rs:1276`, `:1586` |
| `R::{Accordion, Collapsible, Tabs}` | Component descriptions owning expansion/selection, or following explicit bound state | `collapsible.rs:464`, `:947`; `tabs.rs:263` |
| `R::{VirtualList, Tree, Table, FileBrowser}` | Retained collection composites; initial controller/options/callback exceptions apply | `virtual_list.rs:428`; `tree.rs:901`; `table.rs:1553`; `file_browser/view.rs:70`; `docs/contracts.md:169` |
| `R::{SplitPane, RegionLayout, PropertyGrid, ScrollArea, FloatingWindow, ToolWindowLayout}` | Geometry-owning component descriptions with settled notifications | `split.rs:198`; `region_layout.rs:43`; `property_grid.rs:222`; `views.rs:329`; `windows.rs:195`; `tool_windows/mod.rs:749` |
| `R::{Menu, MenuBar, MenuItem, ContextMenu, Popover, Dialog, AlertDialog, QuickPick, CommandPalette}` | Surface/action descriptions; documented session and ownership exceptions apply | `menu.rs:222`, `:675`, `:1250`; `menu_bar.rs:624`; `popover.rs:193`; `dialog.rs:177`, `:493`; `quick_pick.rs:866`; `command_palette.rs:505`; `docs/contracts.md:246` |
| `R::{HistoryNavigator, HistoryButtons}` | Composite/adapter descriptions with explicitly different history-redescription contracts | `history_navigator.rs:111`, `:339`, `:486`, `:527` |
| `R::{PickItem, SemanticItem, Semantics}` | Descriptive data with builders; `disabled` fields are descriptive booleans, not Permission authorities | `quick_pick.rs:11`; `diagnostics.rs:350`, `:464` |
| `R::{InspectionQuery, ContentQuery, CaptureRequest, NativeWindowOptions, NumberFormat, WorkspaceSettings}` | Value/configuration builders | `inspection.rs:200`; `content_search.rs:22`; `capture.rs:121`; `runtime/native_windows.rs:186`; `number_format.rs:38`; `workspace.rs:418` |
| `R::{RichText, PathBuilder}` | Explicit accumulating content construction, not ordinary replacement properties | `text/rich.rs:87`, `:109`; `geometry.rs:382`, `:422` |
| `R::{Rect, TextSelection, UiZoom}` | Value transformations; consuming methods are not component property builders | `drawing.rs:153`; `text_selection.rs:62`; `runtime/settings.rs:56` |
| `R::Permission` | Restriction algebra; accumulation is required | `permission.rs:43`, `:79` |
| `R::{ParameterHost, AnyParameterHost, RemoteHost}` | Owning parameter lifetime tokens | `parameter_binding.rs:410`; `parameter_binding/erased.rs:247`; `parameter_binding/remote.rs:420` |
| `R::{CommandId, HeldInputId}` | Passive registration identities; discard does not unregister | `runtime/commands.rs:377`, `:565`; `runtime/held.rs:5`, `:115` |
| `R::{BackgroundTask, IntervalTimer}` | Optional cancellation handles; discard deliberately leaves work running | `runtime/application.rs:61`, `:79` |
| `R::{Signal, ReadSignal, Parameter, AnyParameter, WidgetId, NativeWindowId}` | Shared bindings or identities; not unique lifetime owners | `signal.rs:177`, `:192`; `parameter_binding.rs:283`; `parameter_binding/erased.rs:58`; `runtime.rs:121`; `runtime/native_windows.rs:125` |
| `R::{DraftEditorController, EditorSession, EditorAttachment, ListController, ToolWindowController, ZoomController}` | Shared UI controllers/session handles; relevant constructors/builders/attachment returns have function-level protection where type-level protection is absent | `text_fields/draft.rs:153`, `:180`; `editor_session.rs:152`, `:194`; `virtual_list.rs:178`; `tool_windows/mod.rs:2279`, `:2292`; `zoom_view.rs:126`, `:139` |
| `R::{AuthorityReceipt, CommandReceipt, ListRevealReceipt, PresentationReceipt, RevealReceipt, TaskReceipt, ToastReceipt}` | Retained request/status receipts | `parameter_binding/remote.rs:80`; `runtime/commands.rs:1607`; `virtual_list.rs:229`; `runtime/native_windows.rs:950`; `runtime/reveal.rs:132`; `runtime/tasks.rs:28`; `notifications.rs:196` |
| `R::{DeliveryMonitor, AnyDeliveryMonitor, Pending, PickPending}` | Delivery monitors or pending-response handles | `parameter_binding/delivery.rs:247`; `parameter_binding/erased.rs:223`; `inspection.rs:565`; `quick_pick.rs:319` |
| `R::{ChangeOrigin, SelectionChange, ListSelectionChange, ToolWindowChange, EditorCompletion, TerminalEvent, PreviewEvent, AnyTerminalEvent, AnyPreviewEvent, RemoteCommit}` | Notification payloads/correlation data, not owning guards | `runtime/notify.rs:152`, `:181`; `list_selection.rs:86`; `tool_windows/mod.rs:218`; `editor_session.rs:74`; `parameter_binding/delivery.rs:130`, `:156`; `parameter_binding/erased.rs:114`, `:202`; `parameter_binding/remote.rs:12` |
| `pert_ui_core::{SeriesEndpoint, SeriesConsumer}`; `R::{EditSession, Mailbox, InspectionService, PreparedDetach}` | Ownership/work handles with consequential discard; annotations are present | `crates/pert-ui-core/src/live/series.rs:138`, `:292`; runtime `parameter_binding.rs:706`, `mailbox.rs:249`, `inspection.rs:1053`, `runtime/native_windows.rs:245` |
| `pert_ui_wgpu::Frame`; `R::WorkspaceReport` | Results whose unexamined outcomes can lose recovery/rejection information; annotations are appropriate | `crates/pert-ui-wgpu/src/lib.rs:87`; runtime `workspace.rs:746` |
| `pert_ui::App`; `pert_ui::Svg` | Application/component builders | `crates/pert-ui/src/app.rs:285`; `src/svg/component.rs:42` |

Other type entries were triaged as values, enums, errors, snapshots, traits, contexts or rendering services rather than unique lifetime handles/receipts. No unresolved role affected a disposition.