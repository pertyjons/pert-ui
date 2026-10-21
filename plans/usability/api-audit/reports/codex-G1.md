# Codex report: G1, construction

Audited commit: `d4ba108f66b7c8ffbf350ae01bedc95d147cd7e6`.

Read-only audit. No files changed, no delegation, and no other auditor’s report read. Tests were inspected, not executed. “All” below means all frozen configurations containing the defining package and its re-exports.

## Dispositions

Counts exclude headings and blank lines. No candidate rows or public types were sampled.

| Item | Disposition | Coverage |
|---|---|---|
| C1 | Findings: one behavior departure and one policy question about the convention’s scope | Screened all **307 constructor entries** and all **534 semantic types**, with their methods. Identified **23 selection/navigation/expansion families**, covering **41 constructors**, plus supporting option descriptions and controllers. Read construction, validation, update and redescription paths and relevant tests. Detailed coverage below. |
| C2 | No findings | Processed all **307 constructor entries**: 20 core, 281 runtime, one CPU, one wgpu and four facade entries. Examined the `try_`, `from_` and `with_` groups against source/rustdoc. The two trait-declared conversions are not inherent constructors. `AnyParameterSnapshot::try_new` is the checking counterpart of `new`; fallible `new` constructors do not violate the proposal, which does not require the converse. Named conversion constructors retain their ordinary conversion meaning. |
| C3 | Findings | Processed all **79 state-owner candidates**, cross-checked all 307 constructors, and examined **16 additional state-bearing constructors** omitted by that candidate list. Checked actual parameter roles and source declarations. All explicit runtime/`StateOwner` arguments lead; seven constructors place bound state after other arguments. |
| C4 | Findings | Classified all **534 semantic types**: core 40, runtime 479, CPU two, wgpu four, facade nine. Cross-checked all **534 type entries**, **198 conversion entries** and **813 attribute entries** for construction relevance. Read representations, validating entry points, relevant serialization paths, accessors, limits and focused tests for invariant-bearing values. Inventory defects are recorded separately. |

C1 family coverage:

| Families | Constructors processed | Evidence and disposition |
|---|---:|---|
| `RadioGroup`, `SegmentedControl`, `InlineChoice`, `Select`, `InlineSelect`, `MultiSelect` | 17 | Fixed validation, reactive update validation, parameter-only options and source replacement checked. Shared radio implementation has finding codex-C1-1. Relevant tests: `crates/pert-ui/tests/controls/component_bound_choices.rs:37`, `component_choice_sources.rs:7`, `parameter_controls.rs:157`, and `crates/pert-ui/tests/collections/multi_select.rs:1046`. |
| `Checkbox`, `Toggle`, `InlineToggle` | 6 | Closed boolean domains; no separately supplied option list. Signal and parameter construction paths remain exclusive. Definitions: `choice_controls.rs:525`, `inline_controls.rs:92`. |
| `Menu`, `ContextMenu`, `MenuBar` | 6 | Fixed menu trees validate sibling identities recursively before adoption: `menu.rs:834`, `menu.rs:932`, `menu.rs:1403`, `menu_bar.rs:663`. Also checked all six `MenuItem` constructors. Tests include `crates/pert-ui/tests/surfaces/menus.rs:581` and its redescription cases beginning at line 999. |
| `QuickPick`, `CommandPalette` | 2 | Controller-owned sessions supply options on opening. `QuickPickController::open` validates before replacing the session (`quick_pick.rs:480`); palette opening delegates to it (`command_palette.rs:294`). The session exception is explicit in `docs/contracts.md:254`. Tests include `crates/pert-ui/tests/collections/quick_pick.rs:111`. |
| `VirtualList`, `Table`, `Tree`, `FileBrowser`, `HistoryNavigator` | 6 | Read constructors, provider/controller ownership, updates and retained-source behavior. Retention is expressly covered by `docs/contracts.md:169`; it is not an unexplained failure to adopt replacement descriptions. Direct reactive-list naming remains the policy question in codex-C1-2. |
| `Tabs`, `Breadcrumb`, `Accordion`, `ToolWindowLayout` | 4 | Examined these under the brief’s broad “pick from options” scope. Fixed descriptors validate before mounting/reconciliation. Container admission is expressly documented at `docs/contracts.md:368`; tool-window admission is documented at `tool_windows/mod.rs:695`. Tests include `surfaces/tabs.rs:639`, `controls/breadcrumb.rs:290`, `controls/collapsible.rs:1059`, and `surfaces/tool_windows.rs:871`. Tool-window source builders are included in codex-C1-2. |

For C2, `Renderer::new_surface` was classified as a factory derived from an existing GPU context, rather than a standalone constructor: its purpose and required sharing are explicit at `crates/pert-ui-wgpu/src/lib.rs:1248`. Methods producing a value derived from a receiver were not mechanically treated as constructors merely because they return `Self`.

Slice 3’s F33 was re-examined: the absence of an infallible fixed-size convenience constructor is not a C1–C4 departure. `SizeLimits::new` correctly validates potentially inconsistent bounds (`crates/pert-ui-runtime/src/layout.rs:209`).

## Findings

### codex-C1-1 Invalid replacement options lose the previous selection presentation

- **Rubric/rule:** C1, project rule.
- **Paths/configurations:** `pert_ui_runtime::{RadioGroup, SegmentedControl, InlineChoice}`; all.
- **Observation — behavior:** When a replacement description changes the value binding, `RadioGroup::reconcile` immediately clears `self.selected` and `self.name` (`crates/pert-ui-runtime/src/choice_controls.rs:936`). The subsequent update validates replacement options at line 1010 and can return before restoring either cached value. Inspection consequently reports no selected option: it derives selection directly from `self.selected` at line 1305. The wrappers delegate reconciliation and update to this implementation at lines 1509 and 1647.

  A concrete triggering sequence is: present a valid selected group; redescribe the same key with a different value signal and a duplicate-valued reactive option list; flush. The old options remain, but their previous selected presentation does not.
- **Rule departed from:** “An invalid reactive list leaves the last valid presentation in place” (`docs/contracts.md:45`). The different-binding cancellation rule at `docs/contracts.md:165` permits ending the old interaction; it does not authorize clearing the last valid selection presentation after failed validation.
- **Existing evidence:** `component_bound_choices.rs:37` checks preservation after invalid writes to the existing option signal. Its source-switch test at line 99 uses valid replacement lists. Neither covers the combination above.
- **Severity/confidence:** **medium** / high.
- **Suggested fix:** Retain presentation caches until replacement bindings/options have been read and validated successfully, while still cancelling input directed at the old binding. Add a replacement-binding failure case across the three wrappers. **Nonbreaking.**
- **If not chosen:** TODO.md.

### codex-C1-2 Choice-source convention needs an explicit boundary for collections and keyed content

- **Rubric/rule:** C1, project rule; **policy question about applicability or a new exception**.
- **Paths/configurations:** `pert_ui_runtime::VirtualList::{new, estimated}` and `ToolWindowLayout::{new, window, windows}`; all.
- **Observation — signature:**

  | Instance | Verified shape |
  |---|---|
  | `VirtualList::new` | Directly takes `ReadSignal<ListSource>`, despite `new` denoting fixed options under C1 (`crates/pert-ui-runtime/src/virtual_list.rs:635`). |
  | `VirtualList::estimated` | The same reactive ownership path under a sizing-based constructor name (`virtual_list.rs:659`). |
  | `ToolWindowLayout` | Starts with center content; `window` and `windows` subsequently add the keyed alternatives exposed by its stripe controls (`tool_windows/mod.rs:765`, `:788`, `:807`). Stripes provide keyboard selection/activation among these alternatives (`:601`). |

- **Rule departed from:** “fixed `new` lists validate immediately, reactive `bound` lists validate during update” and “No builder can combine those sources” (`docs/contracts.md:40`). The rubric additionally states that no builder adds or replaces an option source.
- **Exception analysis:** “Components whose data belongs to a controller … keep their mounted controller and options” (`docs/contracts.md:169`) explains retained redescription behavior. It does not explicitly exempt constructor naming. The tool-window validation contract at `tool_windows/mod.rs:695` explains admission timing, not whether its child descriptions count as C1 options.
- **Severity/confidence:** **low** / high for the signatures; medium for applying the option-source vocabulary to keyed content.
- **Suggested fix:** The maintainer should choose either an explicit source-ownership exception for lazy collections and content composition, or a constructor migration matching the broader C1 definition. Do not infer that these families need parameter constructors or additional source capabilities. A naming/construction migration would be **breaking**; documenting an intentional exception would not.
- **If not chosen:** TODO.md, as a concrete contract clarification.

### codex-C3-1 Bound state follows captions or source descriptions

- **Rubric/rule:** C3, project rule.
- **Paths/configurations:** All instances below are in `pert_ui_runtime`; all configurations.
- **Observation — signature:**

  | Constructor | Current order | Source |
  |---|---|---|
  | `Checkbox::new`, `Toggle::new` | `label, value` | Shared declaration: `crates/pert-ui-runtime/src/choice_controls.rs:525`; expansions at `:647`, `:652` |
  | `MenuItem::toggle` | `key, label, value` | `crates/pert-ui-runtime/src/menu.rs:299` |
  | `MenuItem::radio` | `key, label, selection, value` | `menu.rs:333` |
  | `TableController::new` | `ui, columns, source, selection` | `crates/pert-ui-runtime/src/table.rs:574` |
  | `TreeController::new` | `ui, source, selection` | `crates/pert-ui-runtime/src/tree.rs:272` |
  | `FileBrowserController::new` | `ui, roots, selection, request` | `crates/pert-ui-runtime/src/file_browser/controller.rs:509` |

  The rustdoc identifies these signals and selections as the state each description/controller binds; they are not incidental callback arguments.
- **Rule departed from:** C3: “The state it binds … follows the owner, if any, and comes before the other parameters.” AGENTS.md states “Required state owners lead constructor arguments” at line 142.
- **Exception analysis:** The retained-controller exception in `docs/contracts.md:169` concerns mounted ownership and replacement, not constructor ordering.
- **Severity/confidence:** **low** / high.
- **Suggested fix:** Put the bound signal/selection first, immediately after `ui` where present. Preserve authority and cancellation behavior. **Breaking** argument-order changes.
- **If not chosen:** TODO.md.

### codex-C4-1 Workspace settings replacement can invalidate the document-size invariant

- **Rubric/rule:** C4, project rule and C-VALIDATE.
- **Paths/configurations:** `pert_ui_runtime::Workspace::{new, set_settings, encode, decode}`; all.
- **Observation — behavior:** `Workspace::new` rejects an encoded document exceeding `MAX_DOCUMENT_BYTES` (`crates/pert-ui-runtime/src/workspace.rs:541`). `set_settings` subsequently replaces settings without checking the resulting size (`:552`). Encoding adds the settings line (`:812`), while decoding rejects the resulting oversized text (`:865`).

  A source-derived boundary witness uses one entry, path `x`, kind `k`, and fields `k0000` through `k4095`. Give the first 4,095 fields 247 ASCII characters each and the final field 215 characters. Every name and value satisfies its local validation. With absent settings, the canonical encoding is exactly **1,048,576 bytes**. Adding `ThemeMode::Light` through `set_settings` adds the 20-byte line `settings mode=light\n`; the library’s own decoder then returns `WorkspaceError::TooLarge`.

  The byte arithmetic was checked against `encode` and `write_value` (`workspace.rs:809`, `:947`); this was not an executed Rust probe.
- **Rule departed from:** “Reject invalid external values … Use `TryFrom`/fallible constructors” (AGENTS.md:175), together with C4’s requirement that an invariant-bearing value preserve its private invariant. The advertised document limit is at `workspace.rs:508`.
- **Existing evidence:** `crates/pert-ui/tests/surfaces/workspace.rs:60` tests oversized construction, but does not cover growth through this setter.
- **Severity/confidence:** **high** / high: a successfully constructed and mutated document can be saved but cannot be loaded.
- **Suggested fix:** Validate replacement settings against the complete encoded document before adoption and return a typed error while retaining the previous settings. Add the boundary round-trip regression. Making the setter fallible is **breaking**. No persisted-schema reinterpretation is needed.
- **If not chosen:** TODO.md.

### codex-C4-2 Raw representation accessors depart from the `get` convention

- **Rubric/rule:** C4, project rule; compound-value applicability is a **policy question**.
- **Paths/configurations:** All.

  | Path | Observation and source |
  |---|---|
  | `pert_ui_core::ChoiceId::as_str` | Returns the wrapped identity string (`crates/pert-ui-core/src/choice.rs:50`); the constructor validates that string at `:40`. |
  | `pert_ui_runtime::Color::channels` | Returns the complete private channel array unchanged (`crates/pert-ui-runtime/src/drawing.rs:302`). |
  | `pert_ui_runtime::Rect::values` | Returns the complete private rectangle array unchanged (`drawing.rs:200`). |
  | `pert_ui_runtime::Transform2D::entries` | Returns the complete private coefficient array unchanged (`crates/pert-ui-runtime/src/transform.rs:158`). |

- **Observation kind:** **signature**.
- **Rule departed from:** “Their raw value is read with `get()` unless the unit itself is the point of the type” (`docs/contracts.md:1152`).
- **Exception analysis:** `ChoiceId::as_str` has no unit exception. For the compound geometric values, these are whole-representation accessors, rather than unit-selecting accessors such as `PresentationRate::hertz`. Their existing rustdoc deliberately describes destructuring/backend use (`drawing.rs:193`, `:296`); adopting an exception for such compound representations is the maintainer’s decision.
- **Severity/confidence:** **low** / high for the signatures and scalar departure; medium for compound-value applicability.
- **Suggested fix:** Rename the scalar accessor to `get`, or explicitly amend the convention. Decide the compound-value exception before renaming those accessors. Renaming/removal is **breaking**; adding a forwarding accessor is not.
- **If not chosen:** TODO.md.

### codex-C4-3 Documented limits are missing from, or owned outside, the bounded type

- **Rubric/rule:** C4, project rule. Treatment of intrinsic ranges and shared limits is a **policy question**.
- **Paths/configurations:** All.

  | Type/pattern | Verified limit and evidence |
  |---|---|
  | `PresentationRate` | Accepts 1–1000 Hz, but exposes neither bound as an associated constant (`crates/pert-ui-runtime/src/runtime/presentation.rs:4`, `:17`). |
  | `ViewState` | Kind and field names are limited to 64 bytes; no associated bound is exposed (`crates/pert-ui-runtime/src/workspace.rs:235`, `:787`). |
  | `WorkspacePath` | Its encoded-path and segment limits live on `Workspace`, rather than `WorkspacePath` (`workspace.rs:360`, `:504`, `:506`). |
  | `WorkspaceValue` | Its text limit likewise lives on `Workspace` (`workspace.rs:100`, `:506`). |
  | `StrokeStyle` | Miter limit must be at least 1; no corresponding associated bound (`crates/pert-ui-runtime/src/geometry.rs:124`). |
  | `Color`, `SplitSizing` | Channel and ratio domains are `0..=1`, without associated bounds (`drawing.rs:256`; `split.rs:103`). |
  | `Capacity`, `ContentQuery`, `VirtualAxis` | Documented integer minima are enforced without corresponding `MIN_*` constants: capacity, query limits/budget and lane count (`crates/pert-ui-core/src/live/mod.rs:194`; `content_search.rs:76`, `:90`; `virtual_axis.rs:188`). |
  | `Length`, `Size`, `Constraints`, `Rect`, `Viewport` | Nonnegative extent minima are documented/enforced but not named as `MIN_*` constants (`containers.rs:19`; `layout.rs:138`, `:257`; `drawing.rs:55`, `:893`). `Size::ZERO` supplies the value under another name. |
  | `TextPreedit` | Its byte limit lives on `TextEditError`, not the bounded value (`text_input.rs:55`, `:79`). |

- **Observation kind:** **signature**.
- **Rule departed from:** “Documented limits are associated `MIN_*` or `MAX_*` constants of the type they bound” (`docs/contracts.md:1153`). Associated-constant absence was cross-checked against the committed listings.
- **Exception analysis:** `TextEditError` deliberately centralizes its shared text limit; its rustdoc says, “It lives on this error because it bounds every text path” (`text_input.rs:31`). The governing contract says, “One limit has one reporting variant across paths that share the limit” (`docs/contracts.md:1141`). That requires one reporting variant, but does not explicitly exempt constant ownership. Whether shared limits and intrinsic zero/one bounds should be exceptions needs a policy decision.
- **Severity/confidence:** **low** / high for the declarations and omissions; medium for the intended breadth of the convention.
- **Suggested fix:** Expose meaningful bounds on their bounded types, using aliases to retain one underlying definition where appropriate; alternatively document the selected exceptions. Additions can be **nonbreaking**. Removing or moving existing constants is breaking.
- **If not chosen:** TODO.md.

### codex-B3-1 Incidental observation outside G1: reconciliation publishes a signal

This is not a B3 audit or a claim of B3 coverage.

- **Rubric/rule:** B3, project rule.
- **Path/configurations:** `pert_ui_runtime::ContextMenu::reconcile`; all.
- **Observation — behavior:** Reconciliation calls `self.enabled_signal.publish_trusted(enabled)` (`crates/pert-ui-runtime/src/menu.rs:1472`). That method replaces the signal value and invalidates computations/subscribers (`crates/pert-ui-runtime/src/signal.rs:431`).
- **Rule departed from:** “Never publish signals from reconciliation” (AGENTS.md:132). The method’s library-owned/infallible-publication rationale does not constitute an exception to phase timing.
- **Severity/confidence:** **medium** / high.
- **Suggested fix:** Deliver the change through the public event-phase notification contract. **Nonbreaking** public signature change; timing needs focused verification.
- **If not chosen:** TODO.md. It was not recorded there because this assignment expressly prohibits repository writes.

## Inventory gaps

1. **Conversion implementations are sometimes attributed to their input type.**
   `inputs/candidates/conversion-traits.txt:13` attributes `From<ChoiceId>` to `ChoiceId`; the actual implementation is `From<ChoiceId> for AnyValue` (`crates/pert-ui-core/src/erased.rs:156`; `api/pert-ui-core/default.txt:83`). Likewise, candidate line 22 attributes `From<PowerExponent>` to `PowerExponent`, whereas the target is `f32` (`parameter.rs:96`; API listing line 740). The same misattribution contaminates `types.txt` trait sets. The collector uses the type whose rustdoc impl list it is visiting without checking the impl’s actual `for` type (`inventory.py:442`, `:455`). This must be corrected before treating those rows as trait evidence.

2. **Constructor grouping matches a name prefix where an exact constructor name is intended.**
   `Constraints::bounded` is classified as `[bound]` (`inputs/candidates/constructors.txt:28`). Its actual operation constructs finite measurement constraints from a `Size`, with no reactive source (`crates/pert-ui-runtime/src/layout.rs:274`). The unanchored prefix alternative in `inventory.py:521` causes this false classification.

3. **The C3 candidate list omits state-bearing types outside its name set.**
   Sixteen constructor entries were recovered from the constructor/semantic lists: ten parameter constructors without a separate `ui` argument, three numeric parameter-control constructors, `HistoryButtons::new`, `EditorSession::new`, and `ToastHost::new`. Examples and role evidence: `numeric_controls.rs:1586`, `history_navigator.rs:502`, `editor_session.rs:159`, `notifications.rs:407`. Their state leads correctly. This is a candidate-coverage gap, not a globally missing public item; the semantic lists prevented omission from this audit.

No additional missing public item or configuration-dependent construction path was established.

## Roles relied on

Prefixes below mean `core = pert_ui_core`, `rt = pert_ui_runtime`. Runtime source paths are relative to `crates/pert-ui-runtime/src/`; core paths are relative to `crates/pert-ui-core/src/`.

“Invariant-bearing” here concerns C4 domain construction, not every internal invariant of a widget, state machine or resource handle.

| Path(s) | Role/classification | Source reference |
|---|---|---|
| `core::Capacity`, `core::Sequence` | Invariant-bearing nonzero/bounded scalar values | `live/mod.rs:183`, `:83` |
| `core::ChoiceId` | Invariant-bearing identity string | `choice.rs:32` |
| `core::NormalizedValue` | Invariant-bearing finite normalized scalar | `normalized.rs:47` |
| `core::ParameterId` | Invariant-bearing composite identity | `parameter_id.rs:44` |
| `core::ParameterSpec`, `core::PowerExponent` | Validated domain specification and bounded exponent | `parameter.rs:151`, `:63` |
| `rt::AxisMapping` | Validated finite mapping and positive normal scale | `axis_mapping.rs:37` |
| `rt::{Color, Rect, Viewport, MaskImage}` | Invariant-bearing color, geometry and image values | `drawing.rs:210`, `:53`, `:830`, `:1115` |
| `rt::{Size, Constraints, SizeLimits}` | Validated dimensions and cross-field bounds | `layout.rs:119`, `:169`, `:177` |
| `rt::{Length, Track}` | Validated extent and sizing rule | `containers.rs:12`, `:944` |
| `rt::{Point, Magnification}` | Finite coordinates and positive factor | `pointer.rs:4`, `:106` |
| `rt::ContentQuery` | Validated query with nonzero limits | `content_search.rs:22` |
| `rt::FrameImage`, `rt::RasterImage` | Validated dimensions, storage size and pixel count | `capture.rs:317`; `resources.rs:147` |
| `rt::NativeWindowOptions` | Validated title and nonempty initial size | `runtime/native_windows.rs:186` |
| `rt::NumberLocale` | Validated, unambiguous separator pair | `number_format.rs:191` |
| `rt::OpaqueColor` | Color with alpha exactly one | `drawing.rs:667` |
| `rt::{Path, StrokeStyle}` | Validated path structure and stroke parameters | `geometry.rs:199`, `:94` |
| `rt::PresentationRate` | Bounded finite hertz value; unit-named getter exception applies | `runtime/presentation.rs:10` |
| `rt::ScaleLimits` | Ordered bounds made from validated scales; every input pair is valid | `zoom_view.rs:14`, `:31` |
| `rt::SplitSizing` | Validated ratio/extent preference | `split.rs:76` |
| `rt::TextPreedit` | Bounded text with validated cursor range | `text_input.rs:50` |
| `rt::TextSelection` | Selection geometry with documented control-limit snapping/clamping | `text_selection.rs:31`, `:41` |
| `rt::ThemeMetrics` | Validated measurement table; text sizes must be nonzero | `runtime/theme.rs:244`, `:282` |
| `rt::{Transform2D, ViewScale}` | Validated invertible transform and bounded scale | `transform.rs:31`, `:267` |
| `rt::UiStrings` | Validated complete string/template table | `strings.rs:558`, `:601` |
| `rt::UiZoom` | Validated bounded zoom; serde calls the checking constructor | `runtime/settings.rs:22`, `:100` |
| `rt::{WorkspaceValue, ViewState, WorkspacePath, Workspace}` | Invariant-bearing persisted values and document | `workspace.rs:39`, `:226`, `:355`, `:488` |
| `rt::VirtualAxis` | Validated extent, positive lane count and maintained axis data | `virtual_axis.rs:123`, `:150`, `:188` |
| `core::{AnySpec, ChoiceOption, ParameterMapping, SeriesConfig}` | Invariants carried by already-validated component types; no additional fallible construction needed | `erased.rs:24`; `choice.rs:97`; `parameter.rs:14`; `live/mod.rs:244` |
| `rt::{Palette, Spacing, ProgressStatus}` | Values composed from validated colors, lengths or normalized values | `runtime/theme.rs:742`, `:313`; `progress.rs:22` |
| `rt::{PathElement, PointerEvent, VectorGeometry, VectorStyle, ViewTransform, WorkspaceSettings}` | Invariants carried by validated parts; total construction from those parts is appropriate | `geometry.rs:158`; `pointer.rs:140`; `vector_scene.rs:24`, `:16`; `transform.rs:334`; `workspace.rs:418` |
| `core::{EditToken, Incarnation, Revision}` | Issued protocol identities/counters; arbitrary public construction is intentionally unavailable | `edit.rs:82`, `:12`; `live/mod.rs:47` |
| `rt::{CommandId, FontId, HeldInputId, HistoryEntryId, ImageId, MaskId, NativeWindowId, VectorId, WidgetId}` | Issued identities, not caller-supplied quantities needing raw constructors | `runtime/commands.rs:380`; `text/fonts.rs:86`; `runtime/held.rs:8`; `navigation_history.rs:15`; `resources.rs:85`; `drawing.rs:1096`; `runtime/native_windows.rs:131`; `vector_scene.rs:7`; `runtime.rs:121` |
| `rt::{IconGlyph, TextLayout, FittedText}` | Validated/prepared outputs obtained through their owning services | `text/fonts.rs:149`; `text.rs:176`; `text/fit.rs:58` |
| `rt::{HistoryEntry, HistorySnapshot, RemoteCommit, RequestTicket}` | Owner-produced coherent protocol/captured values | `navigation_history.rs:29`, `:57`; `parameter_binding/remote.rs:12`; `async_source.rs:28` |
| `core::{AnyValue, AnyParameterSnapshot, ParameterSnapshot}` | Plain tagged/captured data, not proof of domain validity; validation occurs when checked/adopted | `erased.rs:112`, `:179`, `:213`; `edit.rs:116` |
| `rt::{Signal, ReadSignal, ListSelection, TableController, TreeController, FileBrowserController}` | Bound state/owners for C3, not captions or option descriptions | `signal.rs:177`, `:192`; `list_selection.rs:62`; `table.rs:561`; `tree.rs:258`; `file_browser/controller.rs:480` |
| `pert_ui_wgpu::Renderer::new_surface` | Factory derived from an existing shared GPU context | `crates/pert-ui-wgpu/src/lib.rs:1248` |
| All remaining semantic-list entries | Not invariant-bearing domain values for C4: operational widgets/controllers/services, traits, errors, unrestricted values, or plain descriptions/captured records validated at use. This is the exhaustive complement of the classifications above, not a sample. | Their defining source declarations and rustdoc at the locations supplied by the five semantic lists were screened. |

No unresolved classification prevents the dispositions above.