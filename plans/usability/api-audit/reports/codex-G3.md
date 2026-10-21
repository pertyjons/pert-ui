# codex report: G3, names and roles

Audited commit: `d4ba108f66b7c8ffbf350ae01bedc95d147cd7e6`.

Read-only audit. No files changed, no delegation, and no builds or tests executed. Findings below establish signatures and roles, not functional defects.

## Dispositions

Counts exclude headings and comments. Configuration variants are merged as in the frozen inputs.

| Item | Disposition | Coverage |
|---|---|---|
| N1 | **No findings.** All `get` methods fit the permitted uses; no `get_*` candidate exists. | All **23** entries in `getters.txt`; read their implementation and rustdoc. Classified raw-value reads, cell-like reads, single-value access and keyed lookups. |
| N2 | **Findings**, shared with `codex-B4-5`: operation-result names use inconsistent role vocabulary. No additional findings in the required synonym sets. | Scanned all **534 type names and 1,992 method names** in the semantic lists against every required synonym set. Also examined name/label/title, set/write/update, binding/signal, snapshot/state/inspection, cancel/reject/refuse, reset/clear and outcome/disposition/result. Checked relevant source and contracts. |
| N3 | **Findings.** Some lengths, coordinates, byte offsets and byte budgets expose their units only in prose. | All **242** entries in `numeric.txt`; read source rustdoc, relevant implementations and macro definitions. Distinguished dimensionless values, counts, identities, application-defined units and raw accessors on unit-bearing types. |
| N4 | **Findings.** Several operation flags remain opaque at call sites. | All **78** entries in `bool-params.txt`: **21** mechanically marked non-builders, **57** marked builders, including the sole two-boolean function. Reclassified the two functional `TextSelection` operations rather than treating their by-value signatures as property builders. |
| N5 | **No findings** under the rubric’s qualified rules. | All **24** conversion/iteration candidates, all **534** type entries and semantic names; checked implementations, related API listing signatures and collection roles. Copy receivers on `Color::to_*` are legitimate; coordinate-transform methods convert their arguments; immutable collection views need not expose mutation that breaks invariants. No incorrectly named public iterator type was found. |
| B4 | **Findings**, including policy questions about the rule’s reach. | All **534** types classified below: core **40**, runtime **479**, CPU **2**, wgpu **4**, facade-defined **9**. Read defining-item rustdoc and declarations, then methods and contracts where ownership or role required clarification. Reviewed receipt and input/reveal test source where relevant. |

The `precision`/`decimal_places` spelling difference is explained: the former is explicitly deprecated and forwards to the latter (`crates/pert-ui-runtime/src/text_fields/number_inputs.rs:2439`). Collection selection, enabled versus active state, retained visibility versus conditional mounting, successful writes versus settled notifications, and cancellation versus completion describe distinct contracts; spelling symmetry would conceal those distinctions.

## Findings

### codex-N3-1 Geometry operations leave pixel units out of their signatures

- **Rubric/rule:** N3; **project rule**. `AGENTS.md:168`: “Give domain quantities and identities appropriate validated types with private invariants.” N3 specifically requires units through a type or name.
- **Paths/configurations:** The following defining runtime APIs exist in **all** audited runtime configurations and their facade re-exports:
  - `CommandPalette::width` — `crates/pert-ui-runtime/src/command_palette.rs:611`.
  - `QuickPick::width` — `crates/pert-ui-runtime/src/quick_pick.rs:975`.
  - `FileBrowser::new` — `crates/pert-ui-runtime/src/file_browser/view.rs:87`.
  - `Table::new` — `crates/pert-ui-runtime/src/table.rs:1441`.
  - `Tree::new` — `crates/pert-ui-runtime/src/tree.rs:914`.
  - `VirtualList::{new, estimated}` — `crates/pert-ui-runtime/src/virtual_list.rs:635`, `:659`.
  - `MeasureContext::{fitted_text, metric}` — `crates/pert-ui-runtime/src/layout.rs:842`, `:744`.
  - `Painter::metric` — `crates/pert-ui-runtime/src/drawing.rs:1461`.
  - `ThemeMetrics::get` — `crates/pert-ui-runtime/src/runtime/theme.rs:279`.
  - `ListDropPoint::across` — `crates/pert-ui-runtime/src/virtual_list.rs:554`.
  - `TextLayout::{paint, paint_turned}` — `crates/pert-ui-runtime/src/text.rs:262`, `:284`.
  - `AxisMapping::{visible, zoomed_at, ticks}` — `crates/pert-ui-runtime/src/axis_mapping.rs:88`, `:101`, `:146`: specifically `extent` and `anchor`, unlike the explicitly named `pixels_per_unit` and `min_pixels`.
  - `VirtualAxis::{new, estimated, item_extent, line_extent, extent, maximum_offset, clamp_offset, line_offset, line_at, lines_per_page, window, reveal, measure_line}` — `crates/pert-ui-runtime/src/virtual_axis.rs:150`, `:164`, `:203`, `:223`, `:242`, `:247`, `:253`, `:271`, `:278`, `:303`, `:313`, `:345`, `:372`.
- **Additional paths/configurations:**
  - `Renderer::{new, new_surface, resize}` — `crates/pert-ui-wgpu/src/lib.rs:391`, `:1271`, `:485`; wgpu **default**.
  - `svg::{rasterize, rasterize_mask}` — `crates/pert-ui/src/svg.rs:295`, `:310`; facade **svg, no-default-svg**.
- **Observation — signature:** These accept or return primitive numbers for lengths or coordinates without naming the pixel unit. Rustdoc distinguishes logical/local pixels from physical pixels, but signatures such as `width(f32)`, `new(..., row_height: f32)`, `zoomed_at(..., anchor: f64)` and `resize(width: u32, height: u32)` do not. The project already provides `Length` and `PhysicalExtent`; `TableColumn::width(Length)` demonstrates the typed alternative (`crates/pert-ui-runtime/src/table.rs:96`).
- **Exceptions:** No named family exception authorizes omitting units. Raw constructors/accessors of types whose purpose establishes the unit, such as `Length`, `Point`, `PhysicalExtent` and `FrameImage`, were treated separately rather than included mechanically.
- **Severity:** **medium**—logical/physical or domain/pixel confusion can require debugging and coordinate conversion workarounds.
- **Confidence:** **high**.
- **Suggested fix:** Use existing quantity types where their range and representation fit; otherwise name primitive units explicitly, preserving the `f64` precision needed by axis arithmetic. Do not silently change coordinate spaces or validation behavior. Type/method changes are **breaking**; parameter-name clarification alone generally is not.
- **Destination:** **TODO.md**.

### codex-N3-2 Text offsets and resource budgets omit their byte unit

- **Rubric/rule:** N3; **project rule**, as quoted in `codex-N3-1`.
- **Paths/configurations:** `TextSelection::{new, anchor, focus}` and `ResourceKind::scene_budget`; **all**.
- **Observation — signature:**
  - `TextSelection::new(layout, anchor: usize, focus: usize)` and its numeric getters use UTF-8 **byte offsets**, rather than character, grapheme or UTF-16 positions. This is established by the type rustdoc and `Cursor::from_byte_index` calls (`crates/pert-ui-runtime/src/text_selection.rs:31`, `:43`, `:50`, `:54`).
  - `ResourceKind::scene_budget() -> usize` reports **bytes**, rather than a resource count. Only its rustdoc establishes that unit (`crates/pert-ui-runtime/src/render_limits.rs:15`, `:21`).
- **Exceptions:** Neither API has a naming exception for these units.
- **Severity:** **medium** for text offsets; **low** for the budget accessor.
- **Confidence:** **high**.
- **Suggested fix:** Name the offsets and accessors with `byte`/`bytes`, or introduce a suitable offset type if broader use justifies it; rename the budget accessor to include `bytes`. Renaming getters is **breaking**; renaming constructor parameter identifiers alone is not.
- **Destination:** **TODO.md**.

### codex-N4-1 Boolean operation flags are opaque at call sites

- **Rubric/rule:** N4; **guideline C-CUSTOM-TYPE**, applied to operation modes whose meaning a boolean argument does not reveal.
- **Paths/configurations:** The following runtime APIs; **all**:
  - `Runtime::focus_next(reverse)` — `crates/pert-ui-runtime/src/runtime/pointer.rs:143`, `:153`.
  - `Component::tab_navigation(cx, reverse, repeat)` — `crates/pert-ui-runtime/src/runtime.rs:1357`, `:1383`.
  - `EventContext::request_reveal(target, focus)` — `crates/pert-ui-runtime/src/runtime/reveal.rs:206`, `:260`.
  - `TextSelection::{moved, at_point}(..., extend)` — `crates/pert-ui-runtime/src/text_selection.rs:61`, `:62`, `:85`, `:87`.
  - `ToolWindowChange::new(key, expanded, origin)` — `crates/pert-ui-runtime/src/tool_windows/mod.rs:244`, `:247`.
- **Observation — signature:** `focus_next(true)` means moving backward; `request_reveal(&target, true)` additionally focuses; selection operations’ final `true` retains the anchor. `tab_navigation` has two independently meaningful boolean positions. The notification constructor’s boolean selects expansion state among several boolean properties of the reported tool window. Those meanings require declaration or documentation lookup.
- **Semantic qualification:** `TextSelection::moved` and `at_point` calculate a selection against a layout; they are not ordinary property builders despite the inventory’s mechanical `self -> Self` marker. Boolean values themselves and clearly named boolean setters were not flagged.
- **Exceptions:** No family exception requires these boolean representations. Existing timing and cancellation must remain intact; `AGENTS.md:140` says: “Do not change authority, callback timing or cancellation merely for spelling symmetry.”
- **Severity:** **medium**.
- **Confidence:** **high**.
- **Suggested fix:** Use directional, focus-policy, repeat-kind, selection-extension and expansion values, or separate clearly named operations where appropriate. Changes to existing signatures, especially the public trait method, are **breaking**.
- **Destination:** **TODO.md**.

### codex-B4-1 Widget-driving handles use several non-controller names

- **Rubric/rule:** B4; **project rule**.
- **Paths/configurations:** `ContextMenuOpener`, `FieldHandle`, `EditorSession`, `ToastQueue`; **all**.
- **Observation — signature/role:**
  - `ContextMenuOpener` is cloneable and opens one mounted context menu. Its rustdoc says “Clones share the menu” (`crates/pert-ui-runtime/src/menu.rs:1295`, `:1303`).
  - `FieldHandle` is cloneable write access to a field’s presented status, retained by `Field::form` (`crates/pert-ui-runtime/src/field_status.rs:298`, `:304`, `:323`).
  - `EditorSession` is explicitly UI-thread-only and cloneable; `begin` and `end` drive the editor’s session (`crates/pert-ui-runtime/src/editor_session.rs:116`, `:152`, `:173`, `:181`).
  - `ToastQueue` is a cloneable UI-local handle feeding one `ToastHost`; posting invalidates that host (`crates/pert-ui-runtime/src/notifications.rs:253`, `:262`).
- **Rule:** `docs/contracts.md:1159`: “Cloneable UI-thread handles that drive a widget are `*Controller`”.
- **Exceptions/policy question:** The contract expressly preserves application-owned collection selection (`docs/contracts.md:1160`), so `ListSelection` is excluded. The editor-session contract specifies its protocol (`docs/contracts.md:929`) but does not expressly exempt its naming. Whether session protocols and widget-specific senders should be exempt is a **policy question**; it is not permission to alter their behavior. The opener and field handle are the clearest departures.
- **Severity:** **low**.
- **Confidence:** **high** for the observations; **medium** for applying the controller category to `EditorSession` and `ToastQueue`.
- **Suggested fix:** Adopt controller names, or document explicit, scoped exceptions for capability handles, session protocols and notification senders. Renaming is **breaking**; documenting an exception is not.
- **Destination:** **TODO.md**.

### codex-B4-2 Captured state uses Snapshot, Inspection, Details, Summary and other names

- **Rubric/rule:** B4; **project rule**.
- **Paths/configurations:** **All**, at their defining package. Instances:
  - Core: `AnyParameterSnapshot` (`crates/pert-ui-core/src/erased.rs:179`), `ParameterSnapshot` (`crates/pert-ui-core/src/edit.rs:116`).
  - Runtime: `BorderInspection`, `BoxModelInspection`, `ComponentDetails`, `ComponentInspection`, `ParameterInspection`, `InspectionSnapshot`, `WindowInspection` (`crates/pert-ui-runtime/src/diagnostics.rs:142`, `:96`, `:646`, `:667`, `:718`, `:849`, `:961`).
  - `ChildInfo` (`crates/pert-ui-runtime/src/runtime.rs:2482`).
  - `CommandEntry`, `CommandInfo` (`crates/pert-ui-runtime/src/runtime/commands.rs:419`, `:404`).
  - `DeliverySnapshot` (`crates/pert-ui-runtime/src/parameter_binding/delivery.rs:206`).
  - `DragInspection` (`crates/pert-ui-runtime/src/drag.rs:355`).
  - `FormSummary` (`crates/pert-ui-runtime/src/field_status.rs:101`).
  - `HistorySnapshot` (`crates/pert-ui-runtime/src/navigation_history.rs:57`).
  - `ProfileSnapshot`, `ProfileStats` (`crates/pert-ui-runtime/src/profiling.rs:137`, `:89`).
  - `QueryResult` (`crates/pert-ui-runtime/src/inspection.rs:364`).
  - `SourceInspection`, `Telemetry` (`crates/pert-ui-runtime/src/live.rs:281`, `:79`).
  - `Workspace` (`crates/pert-ui-runtime/src/workspace.rs:488`).
- **Observation — signature/role:** These expose captured observations or detached views of retained state. Their rustdoc explicitly describes snapshots, captured observations, detached owned views or current aggregate state. They do not give a consumer a live controller. For example, `AnyParameterSnapshot` explicitly says capturing it involves no live handle (`crates/pert-ui-core/src/erased.rs:164`), and `HistorySnapshot` is an owned view of history and its current position (`crates/pert-ui-runtime/src/navigation_history.rs:54`).
- **Rule:** `docs/contracts.md:1159`: “passive captured data are `*State`.”
- **Exceptions/policy question:** No quoted family exception permits the listed suffixes. Whether the rule should instead allow descriptive `Snapshot`, `Inspection`, `Summary` and document names is a **policy question**, particularly for separately versioned inspection and persistence APIs.
- **Severity:** **low**—the inconsistency weakens the promised role vocabulary.
- **Confidence:** **high**.
- **Suggested fix:** Reconcile the naming rule with these established categories, or rename the Rust types consistently. Renaming is **breaking**. It must not silently change serialized schemas.
- **Destination:** **TODO.md**.

### codex-B4-3 ParameterState is an active state machine rather than passive captured data

- **Rubric/rule:** B4; **project rule**; **policy question** about a state-machine exception.
- **Path/configurations:** `pert_ui_core::ParameterState`; **all**.
- **Observation — signature/role:** Its rustdoc calls it a “Synchronous edit state machine”. It owns active edits and receipts and exposes `begin`, `update`, `commit`, `cancel` and authoritative writes. Its passive view is the separately named `ParameterSnapshot` (`crates/pert-ui-core/src/edit.rs:248`, `:272`, `:338`, `:367`, `:486`).
- **Rule:** `docs/contracts.md:1159`: “passive captured data are `*State`.” B4 also asks auditors to identify the suffix on a type in another role.
- **Exceptions:** No naming exception covers the core state machine. It is neither cloneable nor a UI-thread widget controller, so mechanically renaming it `ParameterController` would misclassify it.
- **Severity:** **low**.
- **Confidence:** **high**.
- **Suggested fix:** Explicitly permit `State` for authoritative state machines, or choose a state-machine name and reserve `State` for the snapshot. Renaming is **breaking**; a documented exception is not.
- **Destination:** **TODO.md**.

### codex-B4-4 Two queued-request reports sit outside the receipt vocabulary

- **Rubric/rule:** B4; **project rule**, with a **policy question** for cross-thread replies.
- **Paths/configurations:** `PendingJump`, `Pending<T>`; **all**.
- **Observation — signature/role:**
  - `PendingJump` contains a `ListRevealReceipt` and forwards owned `status()` and reactive `status_binding()`. Its state changes as one queued reveal completes (`crates/pert-ui-runtime/src/content_search.rs:409`, `:416`, `:422`, `:428`).
  - `Pending<T>` reports one queued inspection request. Its rustdoc specifies one-time reply consumption and queue behavior; it exposes `wait`, `wait_timeout` and `try_reply`, rather than the receipt accessors (`crates/pert-ui-runtime/src/inspection.rs:555`, `:565`, `:588`, `:609`, `:640`). `InspectionClient::snapshot` demonstrates admission returning it (`:782`).
- **Rule:** `docs/contracts.md:1161`: “A `*Receipt` reports one queued request through an owned `status()` and a `status_binding()` for reactive UI.”
- **Exceptions/policy question:** `Pending<T>` is deliberately usable across threads and cannot simply acquire a UI-local binding without changing its ownership contract. No named exception currently states that distinction. `PendingJump` already has the receipt contract and needs only naming reconciliation.
- **Severity:** **low** for `PendingJump`; **medium** if consumers infer the wrong threading or consumption model for `Pending<T>`.
- **Confidence:** **high**.
- **Suggested fix:** Use a receipt name for `PendingJump`; explicitly document a cross-thread, consuming-reply exception for `Pending<T>`, or design an appropriately distinct report family. Do not sacrifice its threading contract for symmetry. Renames/signature changes are **breaking**; documenting the exception is not.
- **Destination:** **TODO.md**.

### codex-B4-5 Operation results use inconsistent outcome names

- **Rubric/rules:** B4, **project rule**; N2, **consistency proposal** for a uniform operation-result vocabulary.
- **Paths/configurations:**
  - `pert_ui_wgpu::Frame` — wgpu **default**.
  - `pert_ui_runtime::{ChildReveal, ListingDelivery, ResponseDisposition}` — **all**.
- **Observation — signature/role:**
  - `Frame` is documented as a “Surface acquisition/presentation outcome”; its variants report presentation, retry, occlusion or required recreation (`crates/pert-ui-wgpu/src/lib.rs:87`, `:92`).
  - `ChildReveal` reports what one reveal request did (`crates/pert-ui-runtime/src/runtime/reveal.rs:7`, `:11`).
  - `ListingDelivery` reports whether one response was applied or stale (`crates/pert-ui-runtime/src/file_browser.rs:246`, `:249`).
  - `ResponseDisposition` reports whether an external response was applied, already resolved or stale (`crates/pert-ui-runtime/src/parameter_binding/remote.rs:111`, `:114`).
  - The comparable `PreviewOutcome` already uses the outcome vocabulary for applied/stale message processing (`crates/pert-ui-core/src/edit.rs:133`, `:139`).
- **Rule:** `docs/contracts.md:1162`: “A `*Outcome` names how an interaction ended, distinct from `Result` as an error carrier.”
- **Exceptions/policy question:** No naming exception covers these operation results. Whether “interaction” includes all these message-processing attempts is a **policy question**. `Frame` is the clearest mismatch: its name suggests frame data, while its documented purpose is a presentation outcome.
- **Severity:** **low**.
- **Confidence:** **high** for roles and observations; **medium** for the breadth of the suffix rule.
- **Suggested fix:** Use outcome names for the covered operations, or explicitly scope the rule more narrowly. Preserve the distinction between local processing and authority acceptance. Renaming is **breaking**.
- **Destination:** **TODO.md**.

## Inventory gaps

### Outgoing conversion implementations are attributed as incoming traits

`plans/usability/api-audit/inputs/candidates/types.txt:9` lists `From<ChoiceId>` among `ChoiceId`’s traits. The actual implementation is `From<ChoiceId> for AnyValue`, generated by `value_conversions!` (`crates/pert-ui-core/src/erased.rs:136`, `:159`); the API listing preserves that target (`api/pert-ui-core/default.txt:399`).

The same rendering problem appears at `types.txt:23`: `ParameterError` lists `From<ParameterError>`, while the listed implementation targets `EditError` (`api/pert-ui-core/default.txt:127`). `AnyValue`’s row also lists four indistinguishable `TryFrom<AnyValue>` entries without their destination types (`types.txt:7`).

The inventory therefore loses conversion direction and destination when summarizing associated implementations. Preserve `impl Trait<Source> for Destination`, or distinguish inbound and outbound implementations explicitly. This affects both core configurations. I used source and API listings rather than those summaries to establish conversion direction.

No additional verified omissions were found in the assigned lists.

## Roles relied on

This is the complete classification of the **534 semantic-list types**. Generic parameters are omitted from labels where the semantic lists omit them; classifications apply with their declared bounds.

Path/source abbreviations expand as follows:

- `C` = `pert_ui_core`; `c/` = `crates/pert-ui-core/src/`.
- `R` = `pert_ui_runtime`; `r/` = `crates/pert-ui-runtime/src/`.
- `CPU` = `pert_ui_cpu`; `cpu/` = `crates/pert-ui-cpu/src/`.
- `GPU` = `pert_ui_wgpu`; `gpu/` = `crates/pert-ui-wgpu/src/`.
- `F` = `pert_ui`; `f/` = `crates/pert-ui/src/`.

Roles: **controller**, **handle**, **receipt**, **state** (passive captured data/read-only state view), **outcome**, **invariant value**, **plain data**, **error**, **widget/builder**, **trait**, or a specified **other** role. A scalar status vocabulary is plain data; it is not automatically classified as an entire captured-state object. Generic bindings, registries and application models are distinguished from widget-specific controllers.

**pert-ui-core — 40 types**

| Path | Role | Source reference |
|---|---|---|
| C::AnyParameterSnapshot | state | c/erased.rs:179 |
| C::AnySpec | invariant value | c/erased.rs:24 |
| C::AnyValue | plain data | c/erased.rs:112 |
| C::Capacity | invariant value | c/live/mod.rs:183 |
| C::ChoiceId | invariant value | c/choice.rs:32 |
| C::ChoiceOption | plain data | c/choice.rs:97 |
| C::DrainReport | plain data; drain report | c/live/mod.rs:312 |
| C::EditError | error | c/edit.rs:189 |
| C::EditOutcome | outcome; includes documented submission stage | c/edit.rs:163 |
| C::EditSequence | invariant value; sequence identity | c/edit.rs:31 |
| C::EditSource | invariant value; source identity | c/edit.rs:59 |
| C::EditToken | invariant value; admitted-edit token | c/edit.rs:82 |
| C::HostValue | trait | c/erased.rs:408 |
| C::Incarnation | invariant value | c/live/mod.rs:47 |
| C::LiveError | error | c/live/mod.rs:163 |
| C::LiveItem | trait | c/live/item.rs:36 |
| C::NormalizedValue | invariant value | c/normalized.rs:47 |
| C::NormalizedValueError | error | c/normalized.rs:114 |
| C::ParameterError | error | c/parameter.rs:541 |
| C::ParameterFamily | plain data | c/family.rs:17 |
| C::ParameterId | invariant value | c/parameter_id.rs:44 |
| C::ParameterIdError | error | c/parameter_id.rs:132 |
| C::ParameterMapping | plain data | c/parameter.rs:14 |
| C::ParameterSnapshot | state | c/edit.rs:116 |
| C::ParameterSpec | invariant value | c/parameter.rs:151 |
| C::ParameterState | other; authoritative state machine | c/edit.rs:272 |
| C::ParameterValue | trait | c/family.rs:47 |
| C::PowerExponent | invariant value | c/parameter.rs:63 |
| C::PreviewOutcome | outcome | c/edit.rs:139 |
| C::PublishOutcome | outcome | c/live/mod.rs:143 |
| C::Revision | invariant value; revision identity | c/edit.rs:12 |
| C::Sample | plain data | c/live/mod.rs:286 |
| C::SampleWindow | invariant value; bounded ordered collection | c/live/mod.rs:382 |
| C::SampleWindowEntry | plain data | c/live/mod.rs:367 |
| C::Sequence | invariant value | c/live/mod.rs:83 |
| C::SeriesConfig | invariant value; configuration | c/live/mod.rs:244 |
| C::SeriesConsumer | other; consumer endpoint | c/live/series.rs:292 |
| C::SeriesEndpoint | other; producer endpoint | c/live/series.rs:138 |
| C::Timestamp | invariant value; producer-clock quantity | c/live/mod.rs:120 |
| C::WakeFlag | other; synchronization primitive | c/live/wake.rs:36 |

**pert-ui-runtime — 479 types**

| Path | Role | Source reference |
|---|---|---|
| R::Accordion | widget/builder | r/collapsible.rs:947 |
| R::AlertDialog | widget/builder | r/dialog.rs:493 |
| R::Align | plain data | r/containers.rs:421 |
| R::AnyDeliveryMonitor | handle; delivery observation | r/parameter_binding/erased.rs:223 |
| R::AnyParameter | handle; parameter binding | r/parameter_binding/erased.rs:58 |
| R::AnyParameterHost | other; authoritative owner | r/parameter_binding/erased.rs:255 |
| R::AnyPreviewEvent | plain data | r/parameter_binding/erased.rs:202 |
| R::AnyTerminalEvent | plain data | r/parameter_binding/erased.rs:114 |
| R::ApplicationError | error | r/error.rs:46 |
| R::ArcOrigin | plain data | r/numeric_controls.rs:36 |
| R::ArrangeContext | other; scoped layout context | r/layout.rs:968 |
| R::AuthorityReceipt | receipt | r/parameter_binding/remote.rs:80 |
| R::AuthorityResponse | plain data; authority decision | r/parameter_binding/remote.rs:40 |
| R::AuthorityStatus | plain data; status vocabulary | r/parameter_binding/remote.rs:49 |
| R::AxisMapping | invariant value | r/axis_mapping.rs:37 |
| R::BackgroundTask | handle; background-work lifecycle | r/runtime/application.rs:68 |
| R::Banner | widget/builder | r/notifications.rs:25 |
| R::BindingError | error | r/parameter_binding.rs:62 |
| R::BorderInspection | state | r/diagnostics.rs:142 |
| R::BoxModelInspection | state | r/diagnostics.rs:96 |
| R::Breadcrumb | widget/builder | r/breadcrumb.rs:143 |
| R::BreadcrumbSegment | widget/builder; segment description | r/breadcrumb.rs:28 |
| R::Button | widget/builder | r/controls.rs:445 |
| R::Capture | plain data; captured-image result | r/capture.rs:487 |
| R::CaptureError | error | r/capture.rs:164 |
| R::CaptureJob | plain data; host work description | r/capture.rs:532 |
| R::CaptureRequest | widget/builder; request builder | r/capture.rs:121 |
| R::CaptureTarget | plain data | r/capture.rs:38 |
| R::CellEdit | other; scoped editor-construction context | r/table/editing.rs:29 |
| R::ChangeOrigin | plain data | r/runtime/notify.rs:152 |
| R::Checkbox | widget/builder | r/choice_controls.rs:647 |
| R::ChildInfo | state | r/runtime.rs:2482 |
| R::ChildReveal | outcome | r/runtime/reveal.rs:11 |
| R::Chip | widget/builder | r/chip.rs:72 |
| R::Choice | widget/builder; option description | r/choice_controls.rs:16 |
| R::ChoiceError | error | r/choice_controls.rs:113 |
| R::ClickCounter | other; input state machine | r/click.rs:66 |
| R::Clipboard | trait | r/text_input.rs:192 |
| R::ClipboardError | error | r/text_input.rs:158 |
| R::Collapsible | widget/builder | r/collapsible.rs:464 |
| R::CollectionFilter | widget/builder | r/collection_filter.rs:499 |
| R::Color | invariant value | r/drawing.rs:210 |
| R::ColorPicker | widget/builder | r/color_picker.rs:386 |
| R::ColorRole | plain data | r/runtime/theme.rs:357 |
| R::ColorSwatch | widget/builder | r/color_swatch.rs:96 |
| R::Column | widget/builder | r/containers.rs:898 |
| R::CommandContext | handle; captured invocation context | r/runtime/commands.rs:365 |
| R::CommandEntry | state | r/runtime/commands.rs:419 |
| R::CommandId | handle; command identity | r/runtime/commands.rs:380 |
| R::CommandInfo | state | r/runtime/commands.rs:404 |
| R::CommandKey | plain data | r/runtime/commands.rs:7 |
| R::CommandKeyInput | plain data | r/runtime/commands.rs:269 |
| R::CommandMatcher | other; matching policy | r/command_palette.rs:40 |
| R::CommandOutcome | outcome | r/runtime/commands.rs:392 |
| R::CommandPalette | widget/builder | r/command_palette.rs:505 |
| R::CommandPaletteController | controller | r/command_palette.rs:208 |
| R::CommandPaletteStatus | plain data; status vocabulary | r/command_palette.rs:120 |
| R::CommandReceipt | receipt | r/runtime/commands.rs:1607 |
| R::CommandScope | plain data | r/runtime/commands.rs:340 |
| R::CommandSource | plain data | r/runtime/commands.rs:352 |
| R::CommandStatus | plain data; status vocabulary | r/runtime/commands.rs:1585 |
| R::Component | trait | r/runtime.rs:1019 |
| R::ComponentDetails | state | r/diagnostics.rs:646 |
| R::ComponentInspection | state | r/diagnostics.rs:667 |
| R::Constraints | invariant value | r/layout.rs:169 |
| R::Container | widget/builder | r/containers.rs:63 |
| R::ContentField | plain data | r/content_search.rs:164 |
| R::ContentHit | plain data; search-result record | r/content_search.rs:176 |
| R::ContentJump | other; result or continuation | r/content_search.rs:392 |
| R::ContentQuery | invariant value; query configuration | r/content_search.rs:22 |
| R::ContentRows | trait | r/content_search.rs:350 |
| R::ContentSearch | other; resumable search cursor | r/content_search.rs:238 |
| R::ContentSearchError | error | r/content_search.rs:436 |
| R::ContentStep | plain data; search-step report | r/content_search.rs:204 |
| R::ContentTarget | plain data | r/content_search.rs:148 |
| R::ContextMenu | widget/builder | r/menu.rs:1250 |
| R::ContextMenuOpener | controller | r/menu.rs:1303 |
| R::CursorIcon | plain data | r/pointer.rs:230 |
| R::DebugMode | plain data | r/diagnostics.rs:14 |
| R::DebugOverlay | plain data | r/diagnostics.rs:41 |
| R::DecimalInput | widget/builder | r/text_fields/number_inputs.rs:203 |
| R::Delivery | plain data | r/live.rs:23 |
| R::DeliveryError | error | r/parameter_binding/delivery.rs:11 |
| R::DeliveryMonitor | handle; delivery observation | r/parameter_binding/delivery.rs:247 |
| R::DeliveryResolution | plain data; recovery decision | r/parameter_binding/delivery.rs:188 |
| R::DeliverySnapshot | state | r/parameter_binding/delivery.rs:206 |
| R::DeliveryStatus | plain data; status vocabulary | r/parameter_binding/delivery.rs:176 |
| R::Density | plain data | r/runtime/theme.rs:25 |
| R::Depth | plain data | r/inspection.rs:144 |
| R::DescribeContext | other; scoped state-construction context | r/described.rs:26 |
| R::DetachedClosePolicy | plain data | r/runtime/native_windows.rs:171 |
| R::Detail | plain data | r/inspection.rs:162 |
| R::DiagnosticCommand | plain data; command description | r/inspection.rs:397 |
| R::Dialog | widget/builder | r/dialog.rs:177 |
| R::DialogController | controller | r/dialog.rs:47 |
| R::DialogOutcome | outcome | r/dialog.rs:11 |
| R::DialogStatus | plain data; status vocabulary | r/dialog.rs:23 |
| R::DirectoryEntry | plain data | r/file_browser.rs:119 |
| R::DirectoryRequest | plain data; correlated work request | r/file_browser.rs:196 |
| R::DirectoryResponse | plain data; response packet | r/file_browser.rs:228 |
| R::DisplayList | invariant value; retained drawing collection | r/drawing.rs:1200 |
| R::Divider | widget/builder | r/views.rs:906 |
| R::DraftAlign | plain data | r/text_fields/draft.rs:26 |
| R::DraftEditor | widget/builder | r/text_fields/draft.rs:582 |
| R::DraftEditorController | controller | r/text_fields/draft.rs:153 |
| R::DraftSelection | plain data; selection request | r/text_fields/draft.rs:12 |
| R::DragEvent | plain data; borrowed event view | r/drag.rs:254 |
| R::DragInspection | state | r/drag.rs:355 |
| R::DragOutcome | outcome | r/drag.rs:330 |
| R::DragPayload | plain data; type-erased payload | r/drag.rs:38 |
| R::DragPhase | plain data | r/drag.rs:222 |
| R::DragPreview | widget/builder; preview description | r/drag.rs:191 |
| R::DragRole | plain data | r/drag.rs:243 |
| R::DrawCommand | plain data | r/drawing.rs:1026 |
| R::DrawError | error | r/drawing.rs:24 |
| R::DropOperation | plain data | r/drag.rs:74 |
| R::DropOperations | invariant value; operation set | r/drag.rs:112 |
| R::DropPosition | plain data | r/list_drag.rs:47 |
| R::DropResponse | other; target admission/performance response | r/drag.rs:307 |
| R::Dynamic | widget/builder | r/described.rs:178 |
| R::Each | widget/builder | r/described.rs:277 |
| R::EdgeMode | plain data | r/tool_windows/mod.rs:109 |
| R::EditSession | handle; owned edit guard | r/parameter_binding.rs:706 |
| R::EditorAttachment | handle; edit-reporting endpoint | r/editor_session.rs:219 |
| R::EditorCompletion | plain data; trigger plus outcome report | r/editor_session.rs:74 |
| R::EditorOutcome | outcome; includes documented refusal | r/editor_session.rs:50 |
| R::EditorSession | controller; session protocol | r/editor_session.rs:152 |
| R::EditorStart | plain data | r/editor_session.rs:14 |
| R::EditorTrigger | plain data | r/editor_session.rs:29 |
| R::Element | widget/builder; owned component description | r/layout.rs:312 |
| R::EntryKind | plain data | r/file_browser.rs:26 |
| R::EventContext | other; scoped event context | r/runtime.rs:2897 |
| R::EventHistory | plain data; event-history batch | r/inspection.rs:339 |
| R::EventKind | plain data | r/diagnostics.rs:735 |
| R::EventRecord | plain data; historical event | r/diagnostics.rs:820 |
| R::EventStatus | plain data; recorded status vocabulary | r/diagnostics.rs:795 |
| R::ExpandMode | plain data | r/collapsible.rs:794 |
| R::FailurePhase | plain data | r/runtime.rs:77 |
| R::FailureSite | plain data; diagnostic location | r/runtime.rs:55 |
| R::Field | widget/builder | r/field_status.rs:438 |
| R::FieldHandle | controller | r/field_status.rs:304 |
| R::FieldStatus | plain data; status vocabulary | r/field_status.rs:27 |
| R::FileBrowser | widget/builder | r/file_browser/view.rs:70 |
| R::FileBrowserController | controller | r/file_browser/controller.rs:480 |
| R::FileBrowserError | error | r/file_browser.rs:261 |
| R::FillRule | plain data | r/geometry.rs:33 |
| R::FilterCounts | plain data; counts | r/collection_filter.rs:100 |
| R::FilterMatcher | other; matching policy | r/collection_filter.rs:39 |
| R::FilterOutcome | outcome | r/collection_filter.rs:115 |
| R::FilterTarget | plain data; adapter selection | r/collection_filter.rs:353 |
| R::FittedText | plain data; prepared-text result | r/text/fit.rs:58 |
| R::FloatingWindow | widget/builder | r/windows.rs:195 |
| R::FlushReport | plain data; work report | r/runtime.rs:3485 |
| R::FontData | plain data; immutable font bytes | r/text/fonts.rs:44 |
| R::FontError | error | r/text/fonts.rs:121 |
| R::FontId | handle; registered-font identity | r/text/fonts.rs:86 |
| R::FontManager | handle; registry service | r/runtime/fonts.rs:38 |
| R::FontRole | plain data | r/text/fonts.rs:99 |
| R::Form | widget/builder | r/form.rs:53 |
| R::FormStatus | other; application-owned field registry/coordinator | r/field_status.rs:216 |
| R::FormSummary | state | r/field_status.rs:101 |
| R::FrameError | error | r/capture.rs:224 |
| R::FrameImage | invariant value; image artifact | r/capture.rs:317 |
| R::Freshness | plain data | r/live.rs:47 |
| R::GeometryError | error | r/layout.rs:80 |
| R::Grid | widget/builder | r/containers.rs:1027 |
| R::Group | widget/builder | r/views.rs:136 |
| R::HeldCancellation | plain data; cancellation reason | r/runtime/held.rs:15 |
| R::HeldInputId | handle; registration identity | r/runtime/held.rs:8 |
| R::HexColorError | error | r/drawing.rs:755 |
| R::HighlightError | error | r/text/fit.rs:210 |
| R::HistoryButtons | widget/builder | r/history_navigator.rs:486 |
| R::HistoryEntry | plain data; history record | r/navigation_history.rs:29 |
| R::HistoryEntryId | invariant value; entry identity | r/navigation_history.rs:15 |
| R::HistoryNavigator | widget/builder | r/history_navigator.rs:111 |
| R::HistorySnapshot | state | r/navigation_history.rs:57 |
| R::Icon | plain data | r/text/icons.rs:9 |
| R::IconGlyph | invariant value; validated registered glyph | r/text/fonts.rs:149 |
| R::IconManager | handle; registry service | r/runtime/fonts.rs:162 |
| R::IconSource | plain data; icon description | r/text/icons.rs:237 |
| R::IgnoredEntry | plain data; restore-report reason | r/workspace.rs:719 |
| R::Image | widget/builder | r/image.rs:76 |
| R::ImageFit | plain data | r/image.rs:17 |
| R::ImageId | handle; scene resource identity | r/resources.rs:85 |
| R::ImageSampling | plain data | r/resources.rs:106 |
| R::InlineChoice | widget/builder | r/choice_controls.rs:1585 |
| R::InlineDecimal | widget/builder | r/text_fields/number_inputs.rs:274 |
| R::InlineInteger | widget/builder | r/text_fields/number_inputs.rs:247 |
| R::InlineReadout | widget/builder | r/inline_controls.rs:579 |
| R::InlineSelect | widget/builder | r/select.rs:1740 |
| R::InlineSlider | widget/builder | r/numeric_controls.rs:1592 |
| R::InlineText | widget/builder | r/text_fields/inline_text.rs:66 |
| R::InlineToggle | widget/builder | r/inline_controls.rs:54 |
| R::InputOutcome | outcome | r/pointer.rs:216 |
| R::InputRejection | error | r/text_fields/normalize.rs:24 |
| R::InspectValue | plain data; diagnostic vocabulary | r/diagnostics.rs:71 |
| R::InspectionClient | handle; cross-thread request endpoint | r/inspection.rs:721 |
| R::InspectionQuery | widget/builder; query builder | r/inspection.rs:200 |
| R::InspectionReply | plain data; correlated response | r/inspection.rs:448 |
| R::InspectionService | other; UI-thread request service | r/inspection.rs:1053 |
| R::InspectionSnapshot | state | r/diagnostics.rs:849 |
| R::IntegerInput | widget/builder | r/text_fields/number_inputs.rs:166 |
| R::IntervalTimer | handle; timer lifecycle | r/runtime/application.rs:84 |
| R::IntoElement | trait | r/layout.rs:365 |
| R::Invalidation | plain data | r/runtime.rs:41 |
| R::Justify | plain data | r/containers.rs:393 |
| R::KeyAction | plain data | r/input.rs:206 |
| R::KeyChord | plain data | r/runtime/commands.rs:27 |
| R::KeyCode | plain data | r/input.rs:11 |
| R::KeyDispatch | plain data; routing report | r/runtime/commands.rs:452 |
| R::KeyEvent | plain data | r/input.rs:241 |
| R::KeyTransition | plain data | r/input.rs:220 |
| R::KeyboardState | state; public read-only input view | r/input.rs:352 |
| R::Knob | widget/builder | r/numeric_controls.rs:1598 |
| R::Label | widget/builder | r/controls.rs:58 |
| R::LayoutError | error | r/layout.rs:10 |
| R::Length | invariant value | r/containers.rs:12 |
| R::LevelMeter | widget/builder | r/live_widgets.rs:131 |
| R::LimitError | error | r/render_limits.rs:46 |
| R::LineCap | plain data | r/geometry.rs:47 |
| R::LineJoin | plain data | r/geometry.rs:63 |
| R::Link | widget/builder | r/link.rs:76 |
| R::LinkTarget | plain data | r/file_browser.rs:42 |
| R::LinkUnderline | plain data | r/link.rs:11 |
| R::ListController | controller | r/virtual_list.rs:178 |
| R::ListDragItems | plain data; stable-key payload | r/list_drag.rs:20 |
| R::ListDrop | plain data; completed-drop report | r/list_drag.rs:64 |
| R::ListDropMark | plain data | r/virtual_list.rs:592 |
| R::ListDropPlacement | plain data | r/virtual_list.rs:562 |
| R::ListDropPoint | plain data; placement input | r/virtual_list.rs:524 |
| R::ListFilter | other; collection-data adapter | r/collection_filter.rs:255 |
| R::ListItems | trait | r/virtual_list.rs:39 |
| R::ListRevealReceipt | receipt | r/virtual_list.rs:229 |
| R::ListRevealStatus | plain data; status vocabulary | r/virtual_list.rs:200 |
| R::ListSelection | other; application-owned selection model | r/list_selection.rs:62 |
| R::ListSelectionChange | plain data; notification | r/list_selection.rs:86 |
| R::ListSource | handle; row-provider allocation | r/virtual_list.rs:79 |
| R::ListingDelivery | outcome | r/file_browser.rs:249 |
| R::ListingError | error | r/file_browser.rs:77 |
| R::ListingErrorKind | plain data; error classification | r/file_browser.rs:58 |
| R::LiveWaker | handle; wake endpoint | r/live.rs:144 |
| R::Magnification | invariant value | r/pointer.rs:106 |
| R::Mailbox | other; UI-thread receiver endpoint | r/mailbox.rs:249 |
| R::MaskId | handle; scene resource identity | r/drawing.rs:1096 |
| R::MaskImage | invariant value; coverage artifact | r/drawing.rs:1115 |
| R::MeasureContext | other; scoped measurement context | r/layout.rs:706 |
| R::Menu | widget/builder | r/menu.rs:675 |
| R::MenuBar | widget/builder | r/menu_bar.rs:624 |
| R::MenuItem | widget/builder; menu-row description | r/menu.rs:222 |
| R::MeterAxis | plain data | r/live_widgets.rs:18 |
| R::MetricRole | plain data | r/runtime/theme.rs:74 |
| R::Modifiers | plain data | r/input.rs:285 |
| R::MotionPreference | plain data | r/runtime/settings.rs:120 |
| R::MultiSelect | widget/builder | r/multi_select.rs:1209 |
| R::MultiSelectCommit | plain data; commit policy | r/multi_select.rs:20 |
| R::NativeWindowId | handle; window identity | r/runtime/native_windows.rs:131 |
| R::NativeWindowOptions | invariant value; configuration | r/runtime/native_windows.rs:186 |
| R::NavigationHistory | other; application-owned navigation model | r/navigation_history.rs:194 |
| R::Notifier | handle; generic scheduling endpoint | r/runtime/notify.rs:242 |
| R::NumberFormat | other; formatting/parsing policy | r/number_format.rs:38 |
| R::NumberInputError | error | r/text_fields/number_inputs.rs:14 |
| R::NumberLocale | invariant value | r/number_format.rs:191 |
| R::NumberLocaleError | error | r/number_format.rs:147 |
| R::OpaqueColor | invariant value | r/drawing.rs:667 |
| R::Orientation | plain data | r/numeric_controls.rs:20 |
| R::OverlapPolicy | plain data | r/validation.rs:11 |
| R::Painter | other; scoped drawing recorder | r/drawing.rs:1307 |
| R::Palette | invariant value | r/runtime/theme.rs:742 |
| R::Parameter | handle; editable parameter binding | r/parameter_binding.rs:283 |
| R::ParameterHost | other; authoritative owner | r/parameter_binding.rs:495 |
| R::ParameterInspection | state | r/diagnostics.rs:718 |
| R::Path | invariant value; immutable geometry | r/geometry.rs:199 |
| R::PathBuilder | widget/builder; geometry builder | r/geometry.rs:382 |
| R::PathElement | invariant value; validated operation | r/geometry.rs:158 |
| R::PathError | error | r/geometry.rs:12 |
| R::Pending | receipt; cross-thread consuming reply | r/inspection.rs:565 |
| R::PendingJump | receipt | r/content_search.rs:416 |
| R::Permission | plain data; capability expression | r/permission.rs:43 |
| R::PhysicalExtent | invariant value; physical-pixel quantity | r/drawing.rs:860 |
| R::PickChoice | other; callback result/continuation decision | r/quick_pick.rs:361 |
| R::PickItem | plain data; picker-row description | r/quick_pick.rs:33 |
| R::PickMatcher | other; matching policy | r/quick_pick.rs:116 |
| R::PickPending | handle; application-result observation adapter | r/quick_pick.rs:319 |
| R::PickStatus | plain data; status vocabulary | r/quick_pick.rs:281 |
| R::Point | invariant value | r/pointer.rs:4 |
| R::PointerButton | plain data | r/pointer.rs:40 |
| R::PointerEvent | invariant value; validated event | r/pointer.rs:140 |
| R::PointerKind | plain data | r/pointer.rs:62 |
| R::Popover | widget/builder | r/popover.rs:193 |
| R::PreparedDetach | handle; single-use preparation token | r/runtime/native_windows.rs:245 |
| R::PresentationAction | plain data; native-operation description | r/runtime/native_windows.rs:984 |
| R::PresentationRate | invariant value | r/runtime/presentation.rs:10 |
| R::PresentationReceipt | receipt | r/runtime/native_windows.rs:950 |
| R::PresentationRequest | handle; host completion obligation | r/runtime/native_windows.rs:1015 |
| R::PresentationStatus | plain data; status vocabulary | r/runtime/native_windows.rs:919 |
| R::PreviewEvent | plain data | r/parameter_binding/delivery.rs:156 |
| R::ProfileOperation | plain data | r/profiling.rs:14 |
| R::ProfileSample | plain data; historical measurement | r/profiling.rs:114 |
| R::ProfileSnapshot | state | r/profiling.rs:137 |
| R::ProfileStats | state | r/profiling.rs:89 |
| R::ProgressBar | widget/builder | r/progress.rs:143 |
| R::ProgressStatus | plain data; status vocabulary | r/progress.rs:22 |
| R::Property | widget/builder; property-row description | r/property_grid.rs:35 |
| R::PropertyGrid | widget/builder | r/property_grid.rs:222 |
| R::QuarterTurn | plain data | r/transform.rs:172 |
| R::QueryResult | state | r/inspection.rs:364 |
| R::QuickPick | widget/builder | r/quick_pick.rs:866 |
| R::QuickPickController | controller | r/quick_pick.rs:425 |
| R::RadioGroup | widget/builder | r/choice_controls.rs:697 |
| R::RasterImage | invariant value; immutable image artifact | r/resources.rs:147 |
| R::ReadSignal | handle; read binding | r/signal.rs:192 |
| R::Rect | invariant value | r/drawing.rs:53 |
| R::Redescribed | plain data; conformance result with identities | r/conformance.rs:76 |
| R::RedescriptionError | error | r/conformance.rs:14 |
| R::RegionEdge | plain data | r/region_layout.rs:64 |
| R::RegionLayout | widget/builder | r/region_layout.rs:43 |
| R::RemoteCommit | plain data; immutable proposal packet | r/parameter_binding/remote.rs:12 |
| R::RemoteHost | other; external-authority adapter owner | r/parameter_binding/remote.rs:421 |
| R::RequestError | error | r/inspection.rs:413 |
| R::RequestTicket | handle; request correlation token | r/async_source.rs:28 |
| R::RequestTracker | other; request-generation model | r/async_source.rs:108 |
| R::ResourceKind | plain data | r/render_limits.rs:7 |
| R::ResponseDisposition | outcome | r/parameter_binding/remote.rs:114 |
| R::RestoreRejection | error; participant refusal | r/workspace.rs:677 |
| R::RevealOutcome | outcome | r/runtime/reveal.rs:64 |
| R::RevealReceipt | receipt | r/runtime/reveal.rs:132 |
| R::RevealRefusal | plain data; reveal refusal reason | r/runtime/reveal.rs:33 |
| R::RevealStatus | plain data; status vocabulary | r/runtime/reveal.rs:110 |
| R::RichText | widget/builder; rich-content composition | r/text/rich.rs:87 |
| R::Row | widget/builder | r/containers.rs:893 |
| R::Runtime | other; runtime/state owner | r/runtime.rs:3508 |
| R::RuntimeError | error | r/runtime.rs:184 |
| R::ScaleLimits | invariant value | r/zoom_view.rs:14 |
| R::ScrollArea | widget/builder | r/views.rs:329 |
| R::ScrollAxes | plain data | r/views.rs:294 |
| R::SearchField | widget/builder | r/text_fields/search.rs:107 |
| R::SearchKey | plain data | r/text_fields/search.rs:16 |
| R::Section | widget/builder; section description | r/collapsible.rs:809 |
| R::SegmentedControl | widget/builder | r/choice_controls.rs:1441 |
| R::Select | widget/builder | r/select.rs:1362 |
| R::SelectableText | widget/builder | r/selectable_text.rs:108 |
| R::SelectionChange | plain data; notification | r/runtime/notify.rs:181 |
| R::SelectionMode | plain data | r/list_selection.rs:11 |
| R::Selector | widget/builder; query selector | r/inspection.rs:60 |
| R::SemanticAction | plain data | r/diagnostics.rs:299 |
| R::SemanticItem | plain data; semantic description | r/diagnostics.rs:350 |
| R::SemanticRole | plain data | r/diagnostics.rs:162 |
| R::SemanticValue | plain data | r/diagnostics.rs:256 |
| R::Semantics | plain data; semantic description | r/diagnostics.rs:464 |
| R::SendError | error | r/mailbox.rs:25 |
| R::SeriesSource | handle; live-data source | r/live.rs:627 |
| R::Shape | plain data | r/shapes.rs:17 |
| R::ShortcutBinding | plain data; validation declaration | r/validation.rs:44 |
| R::ShortcutScope | plain data | r/validation.rs:23 |
| R::Show | widget/builder | r/described.rs:223 |
| R::Signal | handle; mutable observable cell | r/signal.rs:177 |
| R::SinkError | error | r/parameter_binding/delivery.rs:89 |
| R::SinkErrorKind | plain data; error classification | r/parameter_binding/delivery.rs:53 |
| R::Size | invariant value | r/layout.rs:119 |
| R::SizeLimits | invariant value | r/layout.rs:177 |
| R::Slider | widget/builder | r/numeric_controls.rs:1586 |
| R::SortDirection | plain data | r/table.rs:212 |
| R::SourceInspection | state | r/live.rs:281 |
| R::Spacing | plain data; spacing description | r/runtime/theme.rs:313 |
| R::Spinner | widget/builder | r/progress.rs:348 |
| R::SplitAxis | plain data | r/split.rs:8 |
| R::SplitPane | widget/builder | r/split.rs:198 |
| R::SplitRule | plain data | r/split.rs:141 |
| R::SplitSizing | invariant value | r/split.rs:76 |
| R::StateOwner | trait | r/signal.rs:64 |
| R::StringRole | plain data | r/strings.rs:20 |
| R::StripeGroup | plain data | r/tool_windows/mod.rs:84 |
| R::StrokeStyle | invariant value | r/geometry.rs:94 |
| R::SubmissionDelivery | plain data; status vocabulary | r/parameter_binding/remote.rs:132 |
| R::SurfaceDismissal | plain data; dismissal reason | r/popover.rs:50 |
| R::SurfaceKind | plain data | r/popover.rs:8 |
| R::Tab | widget/builder; tab description | r/tabs.rs:12 |
| R::Table | widget/builder | r/table.rs:1329 |
| R::TableColumn | widget/builder; column description | r/table.rs:34 |
| R::TableController | controller | r/table.rs:561 |
| R::TableItems | trait | r/table.rs:236 |
| R::TableSort | plain data | r/table.rs:221 |
| R::TableSource | handle; row-provider allocation | r/table.rs:278 |
| R::Tabs | widget/builder | r/tabs.rs:263 |
| R::TaskReceipt | receipt | r/runtime/tasks.rs:28 |
| R::TaskStatus | plain data; status vocabulary | r/runtime/tasks.rs:6 |
| R::Telemetry | state | r/live.rs:79 |
| R::TerminalEvent | plain data; terminal-event envelope | r/parameter_binding/delivery.rs:130 |
| R::TextArea | widget/builder | r/text_fields.rs:203 |
| R::TextColor | plain data | r/text/rich.rs:6 |
| R::TextCommand | plain data | r/text_input.rs:111 |
| R::TextCommandOutcome | outcome | r/text_input.rs:143 |
| R::TextEditError | error | r/text_input.rs:11 |
| R::TextError | error | r/text.rs:54 |
| R::TextField | widget/builder | r/text_fields.rs:154 |
| R::TextInputState | state | r/text_input.rs:217 |
| R::TextKeyOutcome | outcome | r/runtime/commands.rs:327 |
| R::TextLayout | invariant value; prepared text artifact | r/text.rs:176 |
| R::TextLimit | plain data; limit vocabulary | r/text.rs:94 |
| R::TextMovement | plain data | r/text_selection.rs:8 |
| R::TextOverflow | plain data | r/text/fit.rs:39 |
| R::TextPreedit | invariant value; validated composition | r/text_input.rs:50 |
| R::TextRenderingMode | plain data | r/runtime/settings.rs:133 |
| R::TextSelection | invariant value; selection geometry | r/text_selection.rs:39 |
| R::TextSource | plain data; fixed/reactive content description | r/text_source.rs:60 |
| R::TextSpan | widget/builder; styled-span description | r/text/rich.rs:25 |
| R::TextStyle | plain data | r/text.rs:123 |
| R::ThemeManager | handle; runtime configuration service | r/runtime/theme.rs:1014 |
| R::ThemeMetrics | invariant value | r/runtime/theme.rs:244 |
| R::ThemeMode | plain data | r/runtime/theme.rs:14 |
| R::TicketStatus | plain data; freshness/admission vocabulary | r/async_source.rs:58 |
| R::Toast | widget/builder; notification description | r/notifications.rs:144 |
| R::ToastHost | widget/builder | r/notifications.rs:398 |
| R::ToastQueue | controller; widget-specific sender | r/notifications.rs:262 |
| R::ToastReceipt | receipt | r/notifications.rs:196 |
| R::ToastStatus | plain data; status vocabulary | r/notifications.rs:169 |
| R::Toggle | widget/builder | r/choice_controls.rs:652 |
| R::ToolEdge | plain data | r/tool_windows/mod.rs:20 |
| R::ToolGroup | widget/builder; toolbar-group description | r/toolbar.rs:47 |
| R::ToolWindow | widget/builder; tool-window description | r/tool_windows/mod.rs:303 |
| R::ToolWindowChange | plain data; notification | r/tool_windows/mod.rs:218 |
| R::ToolWindowChangeKind | plain data | r/tool_windows/mod.rs:191 |
| R::ToolWindowController | controller | r/tool_windows/mod.rs:2279 |
| R::ToolWindowLayout | widget/builder | r/tool_windows/mod.rs:749 |
| R::ToolWindowMode | plain data | r/tool_windows/mod.rs:158 |
| R::Toolbar | widget/builder | r/toolbar.rs:327 |
| R::ToolbarOverflow | plain data | r/toolbar.rs:15 |
| R::Tooltip | widget/builder | r/popover.rs:404 |
| R::Trace | widget/builder | r/live_widgets.rs:481 |
| R::Track | invariant value | r/containers.rs:944 |
| R::Transform2D | invariant value | r/transform.rs:31 |
| R::TranslucentColorError | error | r/drawing.rs:742 |
| R::Tree | widget/builder | r/tree.rs:901 |
| R::TreeController | controller | r/tree.rs:258 |
| R::TreeItems | trait | r/tree.rs:25 |
| R::TreeSource | handle; hierarchy-provider allocation | r/tree.rs:81 |
| R::UiMessage | plain data; localization arguments | r/strings.rs:663 |
| R::UiSender | handle; cross-thread sender | r/mailbox.rs:88 |
| R::UiSettings | handle; runtime configuration service | r/runtime/settings.rs:213 |
| R::UiStringError | error | r/strings.rs:510 |
| R::UiStrings | invariant value; validated string table | r/strings.rs:558 |
| R::UiZoom | invariant value | r/runtime/settings.rs:22 |
| R::UpdateContext | other; scoped update context | r/runtime.rs:2551 |
| R::ValidationCode | plain data | r/validation.rs:114 |
| R::ValidationHints | plain data; validation declaration | r/validation.rs:89 |
| R::ValidationIssue | plain data; diagnostic record | r/validation.rs:174 |
| R::ValidationMarker | plain data; diagnostic marker | r/validation.rs:191 |
| R::ValidationReport | plain data; validation report | r/validation.rs:220 |
| R::VectorGeometry | invariant value; immutable geometry artifact | r/vector_scene.rs:24 |
| R::VectorId | handle; scene resource identity | r/vector_scene.rs:7 |
| R::VectorRasterizer | other; retained preparation service | r/vector_scene.rs:112 |
| R::VectorStyle | plain data | r/vector_scene.rs:16 |
| R::View | widget/builder | r/views.rs:14 |
| R::ViewScale | invariant value | r/transform.rs:267 |
| R::ViewState | state | r/workspace.rs:226 |
| R::ViewTransform | invariant value | r/transform.rs:334 |
| R::Viewport | invariant value; viewport geometry | r/drawing.rs:830 |
| R::VirtualAxis | other; virtual-layout arithmetic model | r/virtual_axis.rs:123 |
| R::VirtualList | widget/builder | r/virtual_list.rs:439 |
| R::WaitError | error | r/inspection.rs:431 |
| R::WakeError | error | r/live.rs:105 |
| R::WheelGesture | plain data | r/zoom_view.rs:67 |
| R::WidgetId | handle; mounted identity | r/runtime.rs:121 |
| R::WindowCloseAction | plain data; host decision | r/windows.rs:156 |
| R::WindowController | controller | r/windows.rs:41 |
| R::WindowError | error | r/runtime/native_windows.rs:870 |
| R::WindowInspection | state | r/diagnostics.rs:961 |
| R::WindowLayer | widget/builder | r/windows.rs:650 |
| R::WindowPlacement | plain data; geometry value | r/windows.rs:25 |
| R::WindowStatus | plain data; status vocabulary | r/windows.rs:11 |
| R::Workspace | state; separately versioned document | r/workspace.rs:488 |
| R::WorkspaceError | error | r/workspace.rs:579 |
| R::WorkspacePath | invariant value | r/workspace.rs:355 |
| R::WorkspaceReport | plain data; restoration report | r/workspace.rs:746 |
| R::WorkspaceSettings | invariant value; presentation configuration | r/workspace.rs:418 |
| R::WorkspaceValue | invariant value; persistence vocabulary | r/workspace.rs:39 |
| R::Wrap | widget/builder | r/containers.rs:936 |
| R::ZoomController | controller | r/zoom_view.rs:126 |
| R::ZoomView | widget/builder | r/zoom_view.rs:306 |

**pert-ui-cpu — 2 types; default**

| Path | Role | Source reference |
|---|---|---|
| CPU::CpuRenderError | error | cpu/lib.rs:16 |
| CPU::CpuRenderer | other; renderer | cpu/lib.rs:108 |

**pert-ui-wgpu — 4 types; default**

| Path | Role | Source reference |
|---|---|---|
| GPU::Frame | outcome | gpu/lib.rs:92 |
| GPU::Notice | plain data; asynchronous notification | gpu/lib.rs:80 |
| GPU::RenderError | error | gpu/lib.rs:46 |
| GPU::Renderer | other; renderer | gpu/lib.rs:234 |

**Facade-defined types — 9**

`App`, its errors and host decisions exist in **cpu, default, image-formats, images, no-default-desktop-cpu, serde, svg**. `Svg` and `svg::SvgError` exist in **svg, no-default-svg**. Image types exist in **image-formats, images, no-default-png**.

| Path | Role | Source reference |
|---|---|---|
| F::App | widget/builder; application builder/runner | f/app.rs:285 |
| F::AppError | error | f/app.rs:30 |
| F::AppShutdownError | error | f/app.rs:108 |
| F::ErrorAction | plain data; host decision | f/app.rs:184 |
| F::NativeCloseAction | plain data; host decision/deferral | f/app.rs:202 |
| F::Svg | widget/builder | f/svg/component.rs:42 |
| F::images::ImageError | error | f/images.rs:215 |
| F::images::ImageFormat | plain data | f/images.rs:87 |
| F::svg::SvgError | error | f/svg.rs:52 |