# Codex report: G5 discovery

Audited commit **d4ba108f66b7c8ffbf350ae01bedc95d147cd7e6**, using the frozen inputs; the current checkout has no subsequent changes under `crates/` or `api/`.

Read-only audit. No files were modified, no agents were delegated, and no compilation or tests were run.

## Dispositions

| Item | Disposition | Coverage |
|---|---|---|
| D1 | **Findings:** facade-path translation friction and late discovery of existing entry points. No probe or submission requires private APIs or a direct dependency on a defining library package. | Examined imports and qualified references across **81 Rust files**: 17 packaged-example files, 39 gallery files, and 25 round 1 submission files. Read all **48 dated entries** in the eight round 1 friction logs, the relevant triage records, and the round 1 findings and diagnostics list. Followed discovery leads into source and rustdoc. |
| D2 | **Findings:** omitted members under the documented membership rule. **No findings** of clearly unwanted existing members. | Processed **534 type candidates** and their semantic-list entries: core 40, runtime 479, CPU 2, wgpu 4, facade 9. Cross-checked all **530 builder**, **307 constructor**, and **60 callback** candidate rows for relevant signature dependencies. Checked all **288 runtime-prelude members**, five native facade additions, and `Svg`; checked all **63 declared `Component` methods**. Extracted 102 facade `use` declarations, including 37 root globs; expanded explicit imports into **167 distinct paths**. Including qualified references and names actually used through root globs, checked **356 distinct names/paths**. No sampling. |

The import accounting below distinguishes actual uses through root globs from unused names those globs also make available. The reverse membership check covers the complete preludes, including members unused by the probes.

For compact source citations below:

- `R/` means `crates/pert-ui-runtime/src/`.
- `C/` means `crates/pert-ui-core/src/`.
- `F/` means `crates/pert-ui/src/`.
- `U/` means `plans/usability/runs/`.

These prefixes are repository-relative.

## Findings

### codex-D1-1 Inlined item examples require facade-path translation

- **Rubric/rule:** D1, **project rule**.
- **Items:** facade re-exports of runtime/core items, including `Runtime`, `Button`, `TextField`, and `Semantics`.
- **Configurations:** all configurations exposing the corresponding item.
- **Observation — signature:** `F/lib.rs:28` inlines `pert_ui_runtime::*`. Item examples consequently retain defining-crate imports: for example, `R/runtime/tree.rs:410`, `R/controls.rs:533`, and `R/signal.rs:319`. The root documentation explains the substitution at `F/root.md:42`, but that explanation is separate from the item pages. The tester explicitly reports having to infer and translate these paths at `U/r1-4-claude/FRICTION.md:36`.
- **Rule:** AGENTS.md:71: **“Ordinary application code uses the public facade, bindings and defaults.”** D1 permits a finding where the runs demonstrate discovery friction; it does not require a missing re-export.
- **Exception:** defining-package doctests legitimately need to compile independently. That explains their imports, but does not remove the demonstrated facade-reader friction. The existing root explanation is acknowledged.
- **Severity:** **low**.
- **Confidence:** **high**.
- **Suggested fix:** show the facade spelling on inlined item pages, or provide a local explanatory pointer there. Preserve independently buildable defining-package doctests and acyclic dependencies. **Nonbreaking.**
- **Destination if not chosen:** **app-author slice 9** for presentation of facade documentation.

### codex-D1-2 Existing application entry points are discovered late across reference pages

- **Rubric/rule:** D1, **project rule**.
- **Items/configurations:** the entry points below; **all**.
- **Observation — signature:** the necessary public APIs exist, but the tested workflow’s starting documentation does not lead directly to them. These are grouped instances of the same discovery pattern.

| Workflow and entry points | Verified evidence |
|---|---|
| Start background work from a button: `EventContext::defer` → `Runtime::spawn` | `U/r1-1-claude/FRICTION.md:38` records finding `spawn` late and composing the bridge independently. The submission uses it at `U/r1-1-claude/submission/src/lib.rs:255`. `R/runtime/tasks.rs:96` describes deferred runtime access without a background-work example; `R/runtime/application.rs:139` documents `spawn` assuming a runtime is already available. `F/guide/state.md:272` teaches the mailbox/thread route. |
| Build editable application-record rows: `Signal::sibling`, `ListItems`, `VirtualList`, and filtering | `U/r1-2-claude/FRICTION.md:38` records the composition search; `:58` records late discovery of `sibling`. The submission constructs row bindings at `U/r1-2-claude/submission/src/lib.rs:68`. `R/signal.rs:306` explains `sibling` through popup state; `R/virtual_list.rs:50` supplies no state owner to `build`. `F/guide/layout.md:370` shows `Each` with labels, while `:405` introduces collections without this composition. The retained alternative exists in `crates/pert-ui/examples/inventory/app.rs:107`. |
| Keep an owner paired with its retained draft editor: `Component::retain_initial_children` | `U/r1-2-codex/FRICTION.md:52` records discovering the method after a failed rename. The submission implements it at `U/r1-2-codex/submission/src/main.rs:107`. `R/runtime.rs:1722` states the relevant retained-controller rule; `R/text_fields/draft.rs:437` gives the owner contract without pointing to that redescription requirement. |
| Read one component’s semantics: `Runtime::query` with `Selector::id` | `R/runtime/inspect.rs:207` documents the UI-thread query and its ID selection. In contrast, `R/runtime/tree.rs:396` teaches full-snapshot lookup from `inspection_id`, and `F/guide/diagnostics.md:87` presents queries through the cross-thread service. Both test helpers follow the snapshot route: `U/r1-4-codex/submission/tests/ui.rs:35` and `U/r1-4-claude/submission/tests/support/mod.rs:44`. |
| Replace an editor’s contents: `Runtime::text_command` and `TextCommand::SelectAll` | `U/r1-1-claude/FRICTION.md:77` records finding this on the runtime reference page. `R/runtime.rs:5096` introduces the entry point as a clipboard action, although `R/text_input.rs:132` includes selection. The task 4 replacement helpers instead assemble a keyboard chord: `U/r1-4-codex/submission/tests/ui.rs:65` and `U/r1-4-claude/submission/tests/support/mod.rs:114`. |

- **Rule:** AGENTS.md:71–74 requires ordinary facade use and documented public extension points. D1 specifically identifies **“an item that the probes or round 1 needed, found late or only through a deep path.”**
- **Exceptions:** the existing callback, reconciliation, and editor contracts explain the APIs’ responsibilities. They do not require these missing workflow pointers. In particular, the retained-child fix should follow `Component::retain_initial_children`; it should not change reconciliation merely to simplify the example.
- **Severity:** **medium**: the logs demonstrate substantial reference searching and application workarounds.
- **Confidence:** **high**.
- **Suggested fix:** add focused cross-links and complete facade examples at the starting points above. Explain the retained-row alternative and its lifecycle. **Nonbreaking.**
- **Destination if not chosen:** **TODO.md** for the concrete cross-links/examples; **app-author slice 9** for the combined list and testing walkthroughs. New convenience APIs remain separate optional capabilities for **follow-ups**.

### codex-D2-1 The prelude omits UI types covered by its membership rule

- **Rubric/rule:** D2, **project rule**.
- **Observation — signature:** the following **63 types** exist at the public root but are absent from the runtime prelude at `R/lib.rs:328`. The facade inherits that absence through `F/lib.rs:73`. **36** were observed in the corpus’s non-prelude imports or references; those are marked **†**.
- **Configurations:** every listed runtime item exists in runtime `default` and `serde`; every listed core item exists in core `default` and `serde`. Their facade paths exist in **all 11 audited facade configurations**.
- **Rule:** `R/lib.rs:302` includes **“the widgets, containers and surfaces, with the types their builders take and their controllers and status values return”**; `:304` includes parameter specifications, bindings, hosts, and the workspace document; `:306` includes configurable layout, theme, typography, keyboard, and shape types; `:310` includes drawing types used to record and read scenes.

| Instances, at their defining package | Role and source evidence |
|---|---|
| **Runtime:** `Modifiers`†, `KeyChord`†, `CommandKey`†, `CommandScope`†, `CommandContext`†, `CommandEntry`†, `CommandId`†, `CommandReceipt`†, `CommandStatus`†, `CommandOutcome`†, `CommandSource`† | Keyboard configuration and command-widget inputs/results. `R/input.rs:285`; `R/runtime/commands.rs:27`, `:298`, `:340`, `:352`, `:365`, `:1585`, `:1607`; `Button::command` at `R/controls.rs:549`; `MenuItem::shortcut` at `R/menu.rs:499`; matcher input at `R/command_palette.rs:46`, controller result at `:339`, formatter input at `:582`. |
| **Runtime:** `LevelMeter`†, `Trace`†, `MeterAxis`† | Widgets and a widget builder’s configuration enum: `R/live_widgets.rs:131`, `:148`, `:163`, `:481`, `:494`. |
| **Runtime:** `FilterTarget`, `FilterCounts`, `FilterOutcome` | Collection configuration and controller results: `R/collection_filter.rs:100`, `:115`, `:353`, `:529`; `TableController::filter` and `filter_counts` at `R/table.rs:707`, `:730`. |
| **Runtime:** `ListDrop`†, `DropPosition`†, `ListDropPoint`, `ListDropPlacement`, `ListDropMark` | Reordering callback/configuration types and their results: `R/virtual_list.rs:970`, `:1046`, `:524`, `:562`, `:592`; `ListDrop::position` at `R/list_drag.rs:81`. |
| **Runtime:** `EditorSession`, `EditorAttachment`, `EditorStart`, `EditorCompletion`, `EditorTrigger`, `EditorOutcome` | Editor binding and local editor-session state. `TextField::session` at `R/text_fields.rs:805`; `CellEdit::start/session` at `R/table/editing.rs:52`; session declarations and operations at `R/editor_session.rs:14`, `:29`, `:50`, `:74`, `:152`, `:194`, `:202`, `:219`. |
| **Runtime:** `WindowPlacement` | Floating-window callback configuration/result: `R/windows.rs:25`, `:393`. |
| **Runtime:** `AxisMapping`†, `Transform2D`†, `QuarterTurn`†, `ViewScale`†, `OpaqueColor`† | Configurable layout, shape, and theme values. `R/axis_mapping.rs:37`; `R/transform.rs:31`, `:172`, `:267`; `Label::turned` at `R/controls.rs:186`; `ScaleLimits::new` at `R/zoom_view.rs:33`; palette inputs at `R/runtime/theme.rs:838`. |
| **Runtime:** `TextLayout`†, `FittedText`, `MaskImage`†, `MaskId`, `ImageId`, `PathElement`, `VectorId`, `VectorGeometry`, `VectorStyle` | Text/drawing values and scene readback resources. `R/text.rs:176`, `:262`; `R/text/fit.rs:58`; `R/drawing.rs:1026`, `:1096`, `:1115`, `:1222`, `:1250`, `:1350`, `:1487`; `R/resources.rs:85`; `R/geometry.rs:158`, `:206`; `R/vector_scene.rs:7`, `:16`, `:24`, `:72`. |
| **Runtime:** `FontManager`, `IconManager`, `IconGlyph` | Application typography configuration: `R/runtime/fonts.rs:38`, `:92`, `:149`, `:162`, `:175`; the runtime exposes the managers at `:226`, `:232`. |
| **Runtime:** `LinkTarget`†, `ListingErrorKind`† | Configuration of the already-included `EntryKind` and `ListingError`: `R/file_browser.rs:23`, `:35`, `:42`, `:58`, `:83`. The gallery constructs both at `apps/gallery/src/pages/data/files.rs:28`, `:38`. |
| **Runtime:** `WorkspaceSettings`†, `WorkspaceValue`†, `IgnoredEntry` | Workspace-document values and report results: `R/workspace.rs:39`, `:248`, `:418`, `:518`, `:719`, `:763`. |
| **Runtime:** `AnyParameter`†, `AnyParameterHost`†. **Core:** `AnySpec`†, `AnyValue`†, `ParameterValue`†, `ParameterFamily`, `ParameterSnapshot`†, `Revision`, `EditToken`†, `EditOutcome`†, `PreviewOutcome`, `PowerExponent`† | Parameter specifications, bindings, hosts, snapshots, and local edit values. `Runtime::parameter_any` at `R/parameter_binding/erased.rs:15`, binding/host declarations at `:58`, `:255`; `R/parameter_binding.rs:316`, `:351`, `:730`, `:743`, `:797`; `C/erased.rs:24`, `:112`; `C/family.rs:17`, `:47`; `C/edit.rs:12`, `:82`, `:116`, `:139`, `:163`; mapping configuration at `C/parameter.rs:37`, `:63`. |

**Exception review.** `R/lib.rs:320` says:

> “Inspection, profiling, capture, live-data plumbing and other host-integration types also stay at the crate root.”

That covers transport, delivery plumbing, capture services, diagnostics, and backend helpers. It does not cover the meter/trace widgets themselves, their orientation enum, local editor sessions, ordinary collection configuration, typography configuration, or workspace values. “Host” in the editor-session contract means an owning UI component such as a table (`docs/contracts.md:931`), rather than an external integration exemption. The explicitly listed error exclusions were respected.

This re-examination confirms F3’s six named omissions. It also corrects the round 1 impression that `CommandKeyInput`, `Point`, `PointerEvent`, and `PointerKind` are missing: they are already exported at `R/lib.rs:382`, `:446`.

- **Severity:** **low**: extra imports and reference searches; no runtime defect asserted.
- **Confidence:** **high**.
- **Suggested fix:** add the missing re-exports in the defining runtime prelude, including its core re-exports. This is **additive**, with downstream glob-import ambiguity worth checking. Narrowing the documented rule instead would be a **maintainer policy decision**, not an audit resolution.
- **Destination if not chosen:** **TODO.md** for the verified membership deviations; broader prelude organization belongs to **app-author slice 9**.

**Complete import cross-check**

All names below are facade-relative. Unless an exception is stated, their availability is feature-independent across the audited facade configurations.

The following **251 observed names are already in the prelude** and satisfy its rule:

| Range | Names |
|---|---|
| A–F | Accordion, AlertDialog, Align, App, AppError, ArcOrigin, ArrangeContext, Banner, Breadcrumb, BreadcrumbSegment, Button, CellEdit, ChangeOrigin, Checkbox, ChildReveal, Chip, Choice, ChoiceId, ChoiceOption, Collapsible, CollectionFilter, Color, ColorPicker, ColorRole, ColorSwatch, Column, CommandKeyInput, CommandMatcher, CommandPalette, CommandPaletteController, Component, ComponentDetails, Constraints, Container, ContextMenu, CursorIcon, DecimalInput, Density, Dialog, DialogController, DialogOutcome, DialogStatus, DirectoryEntry, DirectoryRequest, DirectoryResponse, DisplayList, Divider, DraftEditor, DraftEditorController, DraftSelection, DragEvent, DragOutcome, DragPayload, DragPhase, DrawCommand, DrawError, DropOperation, DropOperations, DropResponse, Dynamic, Each, EdgeMode, Element, EntryKind, ErrorAction, EventContext, ExpandMode, Field, FieldHandle, FieldStatus, FileBrowser, FileBrowserController, FillRule, FilterMatcher, FloatingWindow, FontData, FontRole, Form, FormStatus |
| G–M | Grid, Group, HistoryButtons, HistoryEntry, HistoryNavigator, Icon, Image, ImageFit, ImageSampling, InlineChoice, InlineDecimal, InlineInteger, InlineReadout, InlineSelect, InlineSlider, InlineText, InlineToggle, InputOutcome, InputRejection, IntegerInput, IntoElement, Invalidation, Justify, KeyAction, KeyCode, KeyEvent, KeyTransition, KeyboardState, Knob, Label, Length, LineCap, LineJoin, Link, LinkUnderline, ListController, ListFilter, ListItems, ListRevealReceipt, ListRevealStatus, ListSelection, ListSelectionChange, ListSource, ListingDelivery, ListingError, Magnification, MeasureContext, Menu, MenuBar, MenuItem, MetricRole, MotionPreference, MultiSelect, MultiSelectCommit |
| N–S | NativeCloseAction, NavigationHistory, NormalizedValue, Notifier, NumberFormat, Orientation, Painter, Palette, Parameter, ParameterId, ParameterMapping, ParameterSpec, Path, PathBuilder, PickChoice, PickItem, PickPending, Point, PointerButton, PointerEvent, PointerKind, Popover, ProgressBar, ProgressStatus, Property, PropertyGrid, QuickPick, QuickPickController, RadioGroup, RasterImage, ReadSignal, Rect, RegionLayout, RichText, Row, Runtime, RuntimeError, ScrollArea, ScrollAxes, SearchField, SearchKey, Section, SegmentedControl, Select, SelectableText, SelectionChange, SelectionMode, Shape, Show, Signal, Size, SizeLimits, Slider, SortDirection, Spinner, SplitAxis, SplitPane, SplitSizing, StateOwner, StringRole, StripeGroup, StrokeStyle, Svg |
| T–Z | Tab, Table, TableColumn, TableController, TableItems, TableSort, TableSource, Tabs, TextArea, TextColor, TextCommand, TextField, TextOverflow, TextRenderingMode, TextSource, TextSpan, TextStyle, ThemeManager, ThemeMode, Toast, ToastHost, ToastQueue, Toggle, ToolEdge, ToolGroup, ToolWindow, ToolWindowChangeKind, ToolWindowController, ToolWindowLayout, ToolWindowMode, Toolbar, Tooltip, Track, Tree, TreeController, TreeItems, TreeSource, UiStrings, UiZoom, UpdateContext, View, ViewState, ViewTransform, Viewport, VirtualList, WidgetId, WindowCloseAction, WindowLayer, WindowStatus, Workspace, WorkspaceError, WorkspaceReport, Wrap, ZoomController, ZoomView |

Feature exceptions: `App`, `AppError`, `ErrorAction`, and `NativeCloseAction` require `native`; `Svg` requires `svg`. The gates are `F/lib.rs:66` and `:69`, with feature relationships at `crates/pert-ui/Cargo.toml:16`.

The **36 observed missing names** are exactly the †-marked names in the finding’s instance table.

The remaining **69 observed names/paths have no membership finding**:

| Disposition | Names |
|---|---|
| Other errors intentionally remain outside the prelude | ApplicationError, ClipboardError, GeometryError, LiveError, NumberInputError, ParameterError, SendError, SinkError, TextEditError |
| Capture/rendering integration | CaptureRequest, CaptureTarget, FrameImage, `cpu::CpuRenderer` |
| Inspection, diagnostics, profiling, and searches over inspected content | AnyParameterSnapshot, ComponentInspection, InspectValue, InspectionQuery, Selector, SemanticAction, SemanticRole, SemanticValue, Semantics, DebugMode, DebugOverlay, ValidationCode, ProfileOperation, ProfileSnapshot, ContentField, ContentHit, ContentJump, ContentQuery, ContentTarget |
| Component/conformance helpers without an explicit membership requirement | ClickCounter, DRAG_THRESHOLD, DragPreview, check_redescription |
| Delivery, host operations, loading, and plumbing | AnyDeliveryMonitor, AuthorityResponse, Capacity, Clipboard, Delivery, DeliveryMonitor, DeliverySnapshot, Freshness, HeldInputId, IntervalTimer, Mailbox, NativeWindowId, NativeWindowOptions, PresentationAction, PresentationReceipt, PresentationStatus, PublishOutcome, RemoteHost, RevealOutcome, RevealReceipt, RevealRefusal, RevealStatus, SeriesConfig, SeriesEndpoint, SeriesSource, SinkErrorKind, TaskStatus, Timestamp, UiSender, `images::ImageFormat`, `images::decode`, list_directory, series_bridge |

Here, `cpu::CpuRenderer` requires `cpu`; `images::ImageFormat` and `images::decode` require `images`. Decoder availability additionally depends on format features; neither is a widget configuration type. See `F/lib.rs:32`, `:34`, and `F/images.rs:1`. No serde-only imported name was found.

**Reverse check:** all existing 288 runtime-prelude members fit the stated categories. `ComponentDetails` and `ContentRows`, despite their diagnostic/search uses, are explicitly required by `Component` signatures (`R/runtime.rs:2043`, `:2059`). Directory request/response types serve included controllers (`R/file_browser/controller.rs:509`, `:749`). The seven documented runtime configuration/component errors are intentional (`R/lib.rs:313`). All five native additions and `Svg` satisfy the facade rule (`F/lib.rs:50`, `:65`). No removal is proposed.

## Inventory gaps

**Incomplete conditional-method coverage in the facade semantic list.**

`App::software_rendering` exists under `cpu` at `crates/pert-ui/src/app.rs:411–412`, and appears in both:

- `api/pert-ui/cpu.txt:95`
- `api/pert-ui/no-default-desktop-cpu.txt:95`

It also appears correctly in `plans/usability/api-audit/inputs/candidates/builders.txt:536`, marked for those two configurations. However, the `App` section in `plans/usability/api-audit/inputs/semantic/pert-ui.txt:5` ends after `App::run` at line 14 without this method.

Thus the method is **not missing from the entire inventory union**, but the semantic list’s claim to contain every public type’s methods is false for this conditional method. Include it, with its configuration marker, before cross-verification.

No other inventory gap was established within D1/D2.

## Roles relied on

The instance table in `codex-D2-1` supplies the individual paths, classifications, and source references for all 63 omitted types. Additional classifications relied on are:

| Path(s) | Role | Source reference |
|---|---|---|
| Existing `pert_ui_runtime::prelude` export groups | Widgets; their configuration, controllers and results; state/parameter/workspace APIs; layout/theme/typography/input/shape APIs; component signatures and drawing APIs | `R/lib.rs:330` through `:476`; `R/runtime.rs:1019`; `crates/pert-ui/tests/foundation/prelude.rs:7` |
| `pert_ui::{App, AppError, AppShutdownError, ErrorAction, NativeCloseAction}` | Native startup, policy and reported failures | `F/lib.rs:69`; `F/app.rs:30`, `:108`, `:184`, `:202`, `:285` |
| `pert_ui::Svg` | Feature-gated widget | `F/svg/component.rs:42`, `:76` |
| `Clipboard`, `Mailbox`, `UiSender`, `SeriesSource`, `RemoteHost`, delivery monitors | Host services, transport endpoints and delivery plumbing | `R/text_input.rs:192`; `R/mailbox.rs:88`, `:249`; `R/live.rs:627`; `R/parameter_binding/remote.rs:421`; `R/parameter_binding/delivery.rs:247` |
| `AnyParameterSnapshot`, semantics and inspection-query families | Captured/inspected descriptions, distinct from UI bindings | `C/erased.rs:179`; `R/diagnostics.rs:71`, `:162`, `:464`; `R/inspection.rs:200` |
| Content-search family | Resumable traversal of inspected component/row descriptions | `R/content_search.rs:238`, `:314` |
| Capture family and CPU renderer | Owned capture results and backend capture service | `R/capture.rs:317`, `:487`, `:532`; `crates/pert-ui-cpu/src/lib.rs:241` |
| `VectorRasterizer` | Renderer-side fallback, distinct from recorded vector data | `R/vector_scene.rs:112` |
| `ClickCounter`, `DragPreview`, `check_redescription` | Optional component/conformance helpers; no explicit prelude-membership clause requires every helper | `R/click.rs:66`; `R/drag.rs:191`; `R/conformance.rs:76` |

No role essential to the recorded findings remains unresolved.