# Part L2 design checkpoint: a layered prelude

Part L2 of [the layering plan](layering-plan.md#part-l2-a-layered-prelude-0372)
(N1 and N2 in the [complexity inventory](complexity-inventory.md#needs-a-decision))
changes public imports, so its shape is settled here before any code. Codex
reviews it, then the maintainer approves it. Line references are to the
committed code at 2a406a4 (0.3.72, after part L4a); L4a took 0.3.72, so L2
is **0.3.73**. Name lists were produced from `crates/pert-ui-runtime/src/lib.rs`
and the `api/` listings at that commit with throwaway scripts and are sorted
with `LC_ALL=C`; the dependency audits of sections 1.2 and 2.3 were run the
same way.

Status: **revision 3, approved 2026-10-27** with the recommended answers to
D1 (the widened membership rule, 195 names) and D2, by the author together
with Codex's review of revision 1 and its recheck of revision 2, under the
maintainer's delegation of 2026-10-26. Revision 3 applies the recheck's four
points exactly as Codex specified them; L2's implementation review verifies
them. L2 changes no behavior: every rule below is about which
module re-exports which name, and every test is an import test.

## What users and the library built by hand

- **Round 1, F3:** the prelude lacked keyboard and command names and
  `ListDrop`, and "rustc suggests no `use pert_ui::…` for facade items"
  (`developer-usability.md:414`, G5-6 at `api-audit/findings.md:929`). 6e
  answered by growing the prelude to 368 runtime names under G5-26's rule
  (`api-audit/fix-plan.md:521-536`). The compiler claim still holds: with
  rustc 1.99.1, a crate that globs a dependency's `prelude` and names an item
  the dependency exports only from its root and another module gets a bare
  E0425 "cannot find type" with no import suggestion (reproduced in a scratch
  crate for this checkpoint). A name that leaves the prelude is therefore found
  only through documentation; section 6 makes the prelude page say where
  every family lives.
- **The inventory:** the prelude holds 70% of the root's names, 59 of them
  component-authoring and 38 parameter-authority names
  (`complexity-inventory.md:59-61`, `:110-113`), so its docs.rs page is not
  readable as "what a first program needs".
- **The library's own programs do not rely on it consistently.** The
  editable-list recipe imports `ListDrop` and `ListDropPosition` from the
  root although the prelude holds them (`guide/layout.md:544`), as does the
  round 2 fixture for `CommandScope` and `KeyChord`
  (`plans/usability/task4-app/src/lib.rs:7`); the testing chapter's
  examples add root imports for `SemanticValue` and `ClickCounter`
  (`guide/testing.md:29`, `:76`), and the state chapter's worker example for
  `KeyAction` and `KeyCode` (`guide/state.md:315`). Many guide examples name
  root items or glob the crate root (`use pert_ui::*` in the controls,
  surfaces, appearance and custom-widgets chapters), as most facade tests and
  gallery pages do, and are untouched by the prelude.
- **A live-data module was left open:** G5-11 kept `SeriesSource` and its
  bridge at the root and left "a thematic live-data module" to slice 9
  (`api-audit/decisions.md:1467-1490`, `plans/app-author-improvements.md:377-382`).

## 1. The membership rule (decisions LD1, LD6; D1)

### 1.1 Representative programs

The plan's "guide's settings form" exists only as the controls chapter's
four-control example mounted under the key `settings`; it serves as R2. No
other guide recipe is a settings form ([corrections](#corrections-to-the-plan)).

| | Program | Application part | Its tests | Names the application part writes |
|---|---|---|---|---|
| R1 | First window | `crates/pert-ui/examples/getting_started.rs` (whole file; also the getting-started chapter's doctest, `src/guide.rs:49`) | the testing chapter (below) | `App`, `AppError`, `Button`, `Column`, `Field`, `Form`, `Label`, `ScrollArea`, `ScrollAxes`, `TextField`; `.key` is `IntoElement::key` (`layout.rs:410-412`) |
| R2 | Settings form | `src/guide/controls.md:223-237` | none | `Checkbox`, `Choice`, `Column`, `Element`, `RadioGroup`, `Runtime`, `Toggle` |
| R3 | Editable list of records | `src/guide/layout.md:545-646` (`Task`, `Rows`, `edit`, `reorder`, the mounted tree) | `:647-708` | `Button`, `Checkbox`, `CollectionFilter`, `Column`, `Element`, `HiddenActive`, `InlineText`, `ListDrop`, `ListDropPosition`, `ListFilter`, `ListItems`, `ListSelection`, `ListSource`, `Row`, `Runtime`, `RuntimeError`, `SelectionMode`, `Signal`, `TextCommand`, `VirtualList` |
| R4 | Dialogs | `src/guide/surfaces.md:219-242` (alert) and `:254-281` (form dialog) | the key presses and outcome check, `:232-240` | `AlertDialog`, `Button`, `Column`, `Dialog`, `DialogController`, `DialogOutcome`, `Element`, `Label`, `Row`, `Runtime`, `TextField` |
| R5 | Menu bar with a command | `src/guide/surfaces.md:121-147` | `:148-161` | `Button`, `Column`, `CommandScope`, `Element`, `KeyChord`, `MenuBar`, `MenuItem`, `Runtime` |
| R6 | Bookmarks: a form, a list from data, a shortcut | `plans/usability/task4-app/src/lib.rs`, `src/main.rs` | round 2's task 4 submissions, outside the repository | `App`, `AppError`, `Button`, `Column`, `CommandScope`, `Each`, `Field`, `Form`, `KeyChord`, `Label`, `Row`, `Runtime`, `RuntimeError`, `ScrollArea`, `ScrollAxes`, `TextField`, `WidgetId` |
| R7 | Background work from a button | `src/guide/state.md:312-332` (`Button` and `cx.spawn`) | `:333-341` | `Button`, `Runtime` |

**The representative tests** are every example of the testing chapter
(`src/guide/testing.md`) and the harness parts of R3, R4, R5 and R7. Beyond
the prelude they write `Viewport`, `KeyAction`, `FocusDirection` (R5's
`focus_next`), `SemanticValue`, `ClickCounter`, `Driver` and the existing
`testing` names. A harness is not application code: names only a harness
writes count toward `testing`, never the prelude (plan, "Membership").

**The representative custom components** are the getting-started
indicator (`src/guide/getting-started-continued.md:118-152`), the
custom-widgets chapter's `Marker` (`src/guide/custom-widgets.md:22-39`), the
facade test's `Swatch` (`crates/pert-ui/tests/foundation/prelude.rs:12-170`)
and the diagram editor's canvas (`examples/diagram_editor/canvas.rs`).

### 1.2 The rule

The approved plan's rule is program-based: "The prelude becomes the names
the representative programs require" (`layering-plan.md:173-178`), with
G5-26's operational test (a name is required when a program must write it:
an argument it constructs, an enum it matches, a value it keeps past the
call; not when a value is only passed on, `api-audit/decisions.md:1546-1556`).
Applied to the programs alone it yields about 45 names (the union of the last
column above, with `IntoElement`), and it leaves out every control the seven
programs happen not to use (`Tabs`, `ProgressBar`, `Select`, `Table`, …), each
reached by a bare E0425 with no suggestion.

**Decision D1 is a proposed widening of that rule.** The recommendation,
used in the rest of this document, keeps the program clause and adds a
component clause and application-wide names, closed under one audit:

1. **Program clause.** Every name the application part of a representative
   program writes. Beyond the clauses below it contributes `TextCommand`
   (R3 begins a rename), `CommandScope` and `KeyChord` (R5, R6 register a
   shortcut).
2. **Component clause.** Every built-in component that belongs to no
   thematic module, is not an authoring building block, and has a
   constructor that needs no thematic family's name (66 types, section 2.1).
   `Slider`, `Knob` and `InlineSlider` fail the last test: their only
   constructors take a `Parameter` (`api/pert-ui-runtime/default.txt`,
   `Slider::new`, `Knob::new`, `InlineSlider::new`), so they are parameter
   controls and join `parameter` with `Knob`'s `ArcOrigin`.
3. **Application-wide names.** `Runtime`, `Signal`, `ReadSignal`, `Element`,
   `WidgetId`, `RuntimeError`; `IntoElement`, whose `key` method every program
   calls; `EventContext`, the context of every control callback, written
   when a callback is a named function (`examples/live_mixer.rs`,
   `examples/diagram_editor/app.rs`); `StateOwner`, the owner bound a helper
   that creates controllers writes (`examples/diagram_editor/editor.rs:79`);
   `Permission`, which composing an `enabled` restriction writes; and
   `ApplicationError`, the error an application constructs (the `main` of
   `live_mixer`, `diagram_editor`, `log_explorer` and `inventory`, G5-32).
   The facade adds `App` and everything it reports (G5-28, G5-30) and the
   feature widget `Svg`.
4. **Closure.** The set is closed under G5-26's test applied to every
   member, not only one step from a component: each argument type of a
   member's constructors and builders (methods returning `Self` or
   `Result<Self, _>`), each callback value it matches, each value it returns
   that an application keeps, and each enum payload or public field it
   constructs is a member, unless it falls under an exception below. The
   audit over the listing finds no other argument. This is what brought in
   `Constraints` (`SizeLimits::new`, `layout.rs:239`), `TextSpan` and
   `TextColor` (`RichText::span`, `tinted_icon`, `text/rich.rs:119`, `:128`),
   `NumberInputError` (the parser `NumberFormat::new` takes returns it,
   `number_format.rs:6`), and `CommandKey`, `KeyCode` and `Modifiers`
   (`KeyChord::new`, `named`, `physical_fallback`), which revision 1 had
   moved out.

**Exceptions**, each a documented home, listed on the prelude page (section 6):

- **Thematic paths of a member:** `for_parameter` (`Parameter`, `ChoiceId`),
  `Button::command`, `MenuItem::command`, `Tooltip::command` (`CommandId`,
  `CommandReceipt`), and `CommandScope::Window`'s `NativeWindowId`
  (`runtime/commands.rs:349`), which the facade prelude holds with `native`
  and `commands` and `windowing` hold everywhere.
- **Authoring and host paths:** `session(EditorSession)` on the editors
  (`component`).
- **Conversions:** arguments taken as `impl Into<TextSource>` or
  `impl Into<IconSource>` are written as strings, signals and `Icon`s;
  `TextSource` stays at the root and `IconSource` in `component`.
- **Closure contexts:** `DescribeContext`, which every `Each`, `Dynamic` and
  `Show` closure receives and infers.
- **Errors a callback receives:** `TextEditError` in
  `on_binding_refusal`; other errors stay at the crate root, and `?` converts
  the leaf errors `RuntimeError` wraps.
- **Services (decision D2):** `NumberLocale` for `NumberFormat`'s localized
  constructors; `FontId` and `FontRole` for `TextSpan::font` and
  `font_role`.
- **Not required:** `ProgressStatus::Determinate` carries a
  `NormalizedValue`, but `ProgressStatus::of(done, total)` builds it from
  counts (`progress.rs:30`, `:55`); a fraction needs `parameter`'s
  `NormalizedValue`. `Runtime` and `EventContext` are hubs whose methods
  span every family; their arguments belong to those families.

With D1 as recommended, **an application configures every prelude
component through the prelude alone, except along these named paths**; the
vocabulary fixtures (section 8.1) are the test.

The result is **195 runtime names**: 66 components, 112 vocabulary names,
11 application-wide names, 3 program-clause names and their closure's 3
(`CommandKey`, `KeyCode`, `Modifiers`), plus the facade's 7 under `native`
and `Svg` under `svg`. 194 are today's members; `NumberInputError` joins
from the root; 174 leave. The plan's estimate of 120 to 150 assumed the
program-based rule with fewer components
([corrections](#corrections-to-the-plan)).

### 1.3 The completeness guarantee (LD6)

By audience, each a test (section 8):

- **Applications:** the application part of R1 to R7 compiles with
  `use pert_ui::prelude::*` alone.
- **Tests:** the representative tests compile with `prelude` and `testing`.
- **Custom components:** the representative custom components compile with
  `prelude` and `component`.

The prelude promises nothing beyond these programs and the closure of
section 1.2; the `api/` listing shows every change to it. Later parts keep
these guarantees (plan, "Rules for every part") **and the whole rule**, not
only the names the representative programs write: a part that adds or
changes a public API of a prelude member re-applies the closure of section
1.2 (clause 4 and its exceptions) to it, adds every name the closure reaches
to the prelude (and to the family modules of section 2.3 the same way), and
adds a prelude-only fixture that writes those names (section 9, "Later
parts"). A part that makes a representative program write a new name puts it
in the glob that program's audience uses.

## 2. Membership

Every one of today's 368 runtime prelude names and 8 facade prelude names
appears exactly once in 2.1 or 2.2.

### 2.1 The prelude after L2

**Components (66):** `Accordion`, `AlertDialog`, `Banner`, `Breadcrumb`,
`Button`, `Checkbox`, `Chip`, `Collapsible`, `CollectionFilter`,
`ColorPicker`, `ColorSwatch`, `Column`, `Container`, `ContextMenu`,
`DecimalInput`, `Dialog`, `Divider`, `Dynamic`, `Each`, `Field`,
`FileBrowser`, `Form`, `Grid`, `Group`, `HistoryButtons`,
`HistoryNavigator`, `Image`, `InlineChoice`, `InlineDecimal`,
`InlineInteger`, `InlineReadout`, `InlineSelect`, `InlineText`,
`InlineToggle`, `IntegerInput`, `Label`, `Link`, `Menu`, `MenuBar`,
`MultiSelect`, `Popover`, `ProgressBar`, `PropertyGrid`, `QuickPick`,
`RadioGroup`, `Row`, `ScrollArea`, `SearchField`, `SegmentedControl`,
`Select`, `SelectableText`, `Show`, `Spinner`, `Table`, `Tabs`, `TextArea`,
`TextField`, `ToastHost`, `Toggle`, `Toolbar`, `Tooltip`, `Tree`, `View`,
`VirtualList`, `Wrap`, `ZoomView`.

The other eleven of the 77 types implementing `Component` belong to a
family: `InlineSlider`, `Knob`, `Slider` (`parameter`), `CommandPalette`
(`commands`), `LevelMeter`, `Trace` (`live`), `RegionLayout`, `SplitPane`,
`ToolWindowLayout`, `WindowLayer` (`windowing`) and `DraftEditor`, a building
block for custom components (`component`).

**Vocabulary (112)**, from the members' signatures in
`api/pert-ui-runtime/default.txt`:

| Members | Vocabulary |
|---|---|
| Layout containers: `Column`, `Row`, `Grid`, `Wrap`, `Group`, `Divider`, `Container`, `View`, `ScrollArea` | `Align`, `ChangeOrigin`, `Color`, `ColorRole`, `Constraints`, `CursorIcon`, `Justify`, `Length`, `MetricRole`, `ScrollAxes`, `Size`, `SizeLimits`, `Spacing`, `Track` |
| Text, labels and editors | `Icon`, `InputRejection`, `LinkUnderline`, `NumberFormat`, `NumberInputError`, `QuarterTurn`, `RichText`, `SearchKey`, `TextColor`, `TextOverflow`, `TextSpan`, `TextStyle` |
| Choice and value controls | `Choice`, `MultiSelectCommit`, `Orientation` |
| Disclosure and tabs: `Accordion`, `Tabs` | `ExpandMode`, `ExpansionChange`, `Section`, `SelectionChange`, `Tab` |
| Forms: `Field`, `Form`, `PropertyGrid` | `FieldHandle`, `FieldStatus`, `FormFields`, `FormSummary`, `Property` |
| Surfaces: dialogs, menus, popovers, toasts, `QuickPick` | `ContextMenuController`, `DialogController`, `DialogOutcome`, `DialogStatus`, `MenuItem`, `PickChoice`, `PickItem`, `PickMatcher`, `PickPending`, `PickStatus`, `Point`, `QuickPickController`, `SurfaceDismissal`, `Toast`, `ToastController`, `ToastReceipt`, `ToastStatus` |
| Collections: `VirtualList`, `CollectionFilter`, `Table`, `Tree` | `CellEdit`, `DropOperation`, `DropOperations`, `FilterCounts`, `FilterMatcher`, `FilterOutcome`, `FilterTarget`, `HiddenActive`, `ListController`, `ListDrop`, `ListDropMark`, `ListDropPlacement`, `ListDropPoint`, `ListDropPosition`, `ListFilter`, `ListItems`, `ListRevealReceipt`, `ListRevealStatus`, `ListSelection`, `ListSelectionChange`, `ListSource`, `SelectionMode`, `SortDirection`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `TreeController`, `TreeItems`, `TreeSource` |
| Files: `FileBrowser` | `DirectoryEntry`, `DirectoryRequest`, `DirectoryResponse`, `EntryKind`, `FileBrowserController`, `LinkTarget`, `ListingDelivery`, `ListingError`, `ListingErrorKind` |
| Navigation: `Breadcrumb`, `HistoryNavigator`, `HistoryButtons`, `Toolbar` | `BreadcrumbSegment`, `HistoryEntry`, `HistoryEntryId`, `HistorySnapshot`, `NavigationHistory`, `ToolGroup`, `ToolbarOverflow` |
| Media and progress: `Image`, `ProgressBar` | `ImageFit`, `ImageSampling`, `ProgressStatus`, `RasterImage` |
| Zoom: `ZoomView` | `ScaleLimits`, `ViewScale`, `ViewTransform`, `WheelGesture`, `ZoomController` |

Examples: `Point` is what `ContextMenuController` and `ZoomView` take;
`DropOperation` is what `ListDrop::operation` returns for a reorder callback
to match; `CursorIcon` and `SizeLimits` are `Container::cursor` and
`Container::size_limits`, and `SizeLimits::new` takes `Constraints`;
`ChangeOrigin` is the value `ScrollArea::on_scroll` and `SelectionChange`
carry; `LinkTarget` is `EntryKind::Symlink`'s payload.

**Application-wide (11):** `ApplicationError`, `Element`, `EventContext`,
`IntoElement`, `Permission`, `ReadSignal`, `Runtime`, `RuntimeError`,
`Signal`, `StateOwner`, `WidgetId`.

**Program clause (3) and its closure (3):** `CommandScope`, `KeyChord`,
`TextCommand`; `CommandKey`, `KeyCode`, `Modifiers`.

**Facade, unchanged** (`crates/pert-ui/src/lib.rs:86-98`): with `native`,
`App`, `AppError`, `AppShutdownError`, `ErrorAction`, `NativeCloseAction`,
`NativeWindowId`, `WindowError`; with `svg`, `Svg`.

### 2.2 Names leaving the prelude, by destination (174)

Every name stays at the crate root; "root" below means only there.

| Destination | Names | Reason |
|---|---|---|
| `component` (51) | `ArrangeContext`, `ChildInfo`, `ChildReveal`, `Component`, `ContentRows`, `DisplayList`, `DraftAlign`, `DraftEditor`, `DraftEditorController`, `DraftSelection`, `DragEvent`, `DragOutcome`, `DragPayload`, `DragPhase`, `DragRole`, `DrawCommand`, `DrawError`, `DropResponse`, `EditorSession`, `EditorStart`, `FillRule`, `IconSource`, `ImageId`, `KeyTransition`, `KeyboardState`, `LineCap`, `LineJoin`, `Magnification`, `MaskId`, `MaskImage`, `MeasureContext`, `MotionPreference`, `Notifier`, `Painter`, `Path`, `PathBuilder`, `RestoreRejection`, `Shape`, `StrokeStyle`, `SurfaceKind`, `TextCommandOutcome`, `TextInputState`, `TextKeyOutcome`, `TextLayout`, `TextPreedit`, `UpdateContext`, `VectorGeometry`, `VectorId`, `ViewState`, `WorkspaceError`, `WorkspaceValue` | layer 2: implementing `Component`, drawing, hosting an editor session. All but the four `Draft*` names are members of `component` today; those four join it (2.4). |
| `component` and `testing` (7) | `CommandKeyInput`, `FocusDirection`, `InputOutcome`, `KeyAction`, `PointerButton`, `PointerEvent`, `PointerKind` | input events a component reads and a test sends by hand (2.4) |
| `component` and `inspection` (3) | `ComponentDetails`, `Invalidation`, `KeyEvent` | authoring names that inspection records carry (2.3) |
| `component`, `inspection` and `testing` (1) | `Viewport` | the same, and every harness sets one |
| `component`, `inspection` and `windowing` (1) | `Rect` | geometry a component draws, a capture region and a tool window's floating bounds |
| `commands` (7) | `CommandEntry`, `CommandMatcher`, `CommandPalette`, `CommandPaletteController`, `CommandPaletteStatus`, `HeldCancellation`, `HeldInputId` | layer 1, commands and shortcuts |
| `commands` and `component` (6) | `CommandContext`, `CommandId`, `CommandOutcome`, `CommandReceipt`, `CommandSource`, `CommandStatus` | the same; `component` keeps them for components that run commands |
| `parameter` (42) | `AnyDeliveryMonitor`, `AnyParameter`, `AnyParameterHost`, `AnyParameterSnapshot`, `AnyPreviewEvent`, `AnySpec`, `AnyTerminalEvent`, `AnyValue`, `ArcOrigin`, `AuthorityReceipt`, `AuthorityResponse`, `AuthorityStatus`, `ChoiceId`, `ChoiceOption`, `DeliveryMonitor`, `DeliveryResolution`, `DeliverySnapshot`, `DeliveryStatus`, `EditOutcome`, `EditSession`, `EditToken`, `InlineSlider`, `Knob`, `Parameter`, `ParameterFamily`, `ParameterHost`, `ParameterId`, `ParameterMapping`, `ParameterSnapshot`, `ParameterValue`, `PowerExponent`, `PreviewEvent`, `PreviewOutcome`, `RemoteCommit`, `RemoteHost`, `ResponseDisposition`, `Revision`, `SinkError`, `SinkErrorKind`, `Slider`, `SubmissionDelivery`, `TerminalEvent` | LD2; the parameter-only controls (1.2) |
| `parameter` and `live` (1) | `ParameterSpec` | LD2; a live meter's range (2.3) |
| `parameter` and `windowing` (1) | `NormalizedValue` | LD2; a split's ratio (2.3) |
| `windowing` (22) | `EdgeMode`, `Expansion`, `FloatingWindow`, `RegionEdge`, `RegionLayout`, `SplitAxis`, `SplitPane`, `SplitRule`, `SplitSizing`, `StripeGroup`, `ToolEdge`, `ToolWindow`, `ToolWindowChange`, `ToolWindowChangeKind`, `ToolWindowController`, `ToolWindowLayout`, `ToolWindowMode`, `WindowCloseAction`, `WindowController`, `WindowLayer`, `WindowPlacement`, `WindowStatus` | layer 1, windows, regions, splits and tool windows |
| `live` (3) | `LevelMeter`, `MeterAxis`, `Trace` | the live widgets join their data family (G5-11) |
| root (29) | theme: `Density`, `OpaqueColor`, `Palette`, `ThemeManager`, `ThemeMetrics`, `ThemeMode`; settings: `TextRenderingMode`, `UiSettings`, `UiZoom`; fonts and icons: `FontData`, `FontId`, `FontManager`, `FontRole`, `IconGlyph`, `IconManager`; strings and locale: `NumberLocale`, `StringRole`, `TextSource`, `UiMessage`, `UiStrings`; workspace: `IgnoredEntry`, `Workspace`, `WorkspacePath`, `WorkspaceReport`, `WorkspaceSettings`; not required: `DescribeContext`, `HexColorError`, `PhysicalExtent`, `VirtualAxis` | the services are decision D2 (25 names); the last four are exceptions of 1.2 or no member's argument (`VirtualAxis` is window arithmetic for a custom virtualized component, `PhysicalExtent` a host's surface size, `HexColorError` an error `Color::from_hex` returns) |

### 2.3 Thematic modules

**Rule.** A module holds its family and is closed one level under the same
audit as the prelude: each argument of a member's constructors and builders,
and each enum payload or public field type of a member, is in the module or
in the prelude, except where an exception below says otherwise. A name added
by this closure is not itself chased further; its own dependencies are in its
family's module. Each module is tested with `prelude` and that module only
(section 8.1). The audit over the listing finds nothing outside these lists
and exceptions.

- **`parameter` (54):** the 42 and `ParameterSpec`, `NormalizedValue`
  above, and the root-only `BindingError`, `DeliveryError`, `EditError`,
  `EditSequence`, `EditSource`, `HostValue`, `NormalizedValueError`,
  `ParameterError`, `ParameterIdError`, `ParameterState`: the core's
  parameter items (`crates/pert-ui-core/src/lib.rs:12-26`, without its live
  items), the runtime's binding and remote-authority items
  (`pert-ui-runtime/src/lib.rs:253-256`, `:288-293`) and the parameter-only
  controls. The family's errors are included: a host matches them
  (`ParameterError::NonFinite`, `guide/state.md:100`). **Excluded:**
  `ParameterInspection` (`inspection`); `Choice` is a control's option.
- **`commands` (20):** `CommandContext`, `CommandEntry`, `CommandId`,
  `CommandInfo`, `CommandKey`, `CommandMatcher`, `CommandOutcome`,
  `CommandPalette`, `CommandPaletteController`, `CommandPaletteStatus`,
  `CommandReceipt`, `CommandScope`, `CommandSource`, `CommandStatus`,
  `HeldCancellation`, `HeldInputId`, `KeyChord`, `KeyCode`, `Modifiers`,
  `NativeWindowId`. `CommandInfo` (what `Runtime::command_info` returns,
  `runtime/commands.rs:851-855`) and `NativeWindowId` (`CommandScope::Window`,
  `CommandContext::window`) join from the root. **Excluded:** raw key
  events and the host's key entry (`KeyEvent`, `KeyAction`, `KeyTransition`,
  `KeyboardState`, `KeyDispatch`, `CommandKeyInput`, `TextKeyOutcome`), in
  `component`, `testing` or the root; shortcut validation records
  (`inspection`).
- **`windowing` (30):** the 22 above and `DetachedClosePolicy`,
  `NativeWindowId`, `NativeWindowOptions`, `NormalizedValue` (`SplitRule::Ratio`,
  `split.rs:148`), `PresentationReceipt`, `PresentationStatus`, `Rect`
  (`ToolWindowChange::with_geometry`, `tool_windows/mod.rs:301`) and
  `WindowError`. **Excluded:** a host's presentation plumbing
  (`PresentationRequest`, `PresentationAction`, `PreparedDetach`,
  `PresentationRate`), which L3's N3 settles; `App`'s `NativeCloseAction`,
  which stays with `App` in the prelude; `WindowError::Drawing`'s
  `DrawError`, an error payload under the error rule.
- **`inspection` (61):** `BorderInspection`, `BoxModelInspection`,
  `Capture`, `CaptureError`, `CaptureRequest`, `CaptureTarget`,
  `ComponentDetails`, `ComponentInspection`, `DebugMode`, `DebugOverlay`,
  `Depth`, `Detail`, `DiagnosticCommand`, `DragInspection`, `EventHistory`,
  `EventKind`, `EventRecord`, `EventStatus`, `FailurePhase`, `FailureSite`,
  `FlushReport`, `FrameError`, `FrameImage`, `InspectValue`,
  `InspectionClient`, `InspectionQuery`, `InspectionReply`,
  `InspectionService`, `InspectionSnapshot`, `Invalidation`, `KeyEvent`,
  `LimitError`, `NativeWindowId`, `OverlapPolicy`, `ParameterInspection`,
  `Pending`, `ProfileOperation`, `ProfileSample`, `ProfileSnapshot`,
  `ProfileStats`, `QueryResult`, `Rect`, `RequestError`, `ResourceKind`,
  `Selector`, `SemanticAction`, `SemanticItem`, `SemanticRole`,
  `SemanticValue`, `Semantics`, `ShortcutBinding`, `ShortcutScope`,
  `SourceInspection`, `ValidationCode`, `ValidationHints`,
  `ValidationIssue`, `ValidationMarker`, `ValidationReport`, `Viewport`,
  `WaitError`, `WindowInspection`. The closure added `Rect` and
  `NativeWindowId` (`CaptureTarget::region`, `capture.rs:97`),
  `ComponentDetails` and `Invalidation` (`ComponentInspection`'s fields),
  `KeyEvent` (`EventKind::KeyInput`) and `Viewport` (`WindowInspection`);
  `ShortcutBinding`'s `KeyCode` and `Modifiers` (`validation.rs:44-63`) are in
  the prelude. **Excluded:** `CaptureJob`, which a host serves
  (`Runtime::complete_capture`, L3); the facade's `CpuRenderer`, a renderer,
  in `testing` and `cpu`; `CaptureError::Drawing`'s `DrawError` (error
  rule); and the record fields of other families, which a tool that matches
  them takes from that family's module: `ParameterInspection` and
  `EventRecord::parameter` (`AnyParameterSnapshot`, `DeliveryStatus`,
  `ParameterId`: `parameter`), `SourceInspection` (`Delivery`, `Freshness`:
  `live`).
- **`live` (29):** `Capacity`, `Delivery`, `DrainReport`, `Freshness`,
  `Incarnation`, `LevelMeter`, `LiveError`, `LiveItem`, `LiveWaker`,
  `Mailbox`, `MeterAxis`, `ParameterSpec`, `PublishOutcome`, `Sample`,
  `SampleWindow`, `SampleWindowEntry`, `SendError`, `Sequence`,
  `SeriesConfig`, `SeriesConsumer`, `SeriesEndpoint`, `SeriesSource`,
  `Telemetry`, `Timestamp`, `Trace`, `UiSender`, `WakeError`, `WakeFlag`,
  `series_bridge`: the core's bridge (`pert-ui-core/src/lib.rs:19-23`), the
  runtime's UI side (`pert-ui-runtime/src/lib.rs:304-306`), the live widgets,
  `ParameterSpec`, which `LevelMeter::new` and `Trace::new` take for their
  range, and the mailbox family. Mailboxes are included by evidence:
  contracts.md treats producer data and mailboxes as one section ("Producer
  data and mailboxes", `contracts.md:833`), they share `Capacity`, and a
  producer application uses both (`examples/live_mixer.rs:250-253`).
  **Excluded:** `ParameterSpec`'s own builders' names (`ParameterMapping`,
  `ChoiceId`, `ChoiceOption`: `parameter`, one level); `BackgroundTask`,
  `IntervalTimer` and `TaskReceipt`, the handles of `spawn` and
  `start_interval`, which stay at the root (and `TaskReceipt` in
  `component`); `SourceInspection` (`inspection`).

### 2.4 `component` and `testing`

- **`component`** keeps G5-26's rule ("every name implementing `Component`
  writes", `pert-ui-runtime/src/lib.rs:596-624`) and its 126 names. It gains
  five: `ClickCounter`, which a component that counts presses stores and
  constructs (`examples/diagram_editor/canvas.rs:77`, `:100`), and
  `DraftAlign`, `DraftEditor`, `DraftEditorController` and `DraftSelection`:
  `DraftEditor` is the editor a custom component embeds
  (`complexity-inventory.md:207`), its packaged user is a custom component
  (`examples/diagram_editor/canvas.rs`, which migrates to the glob, section
  7), and without this the custom-component guarantee (1.3) would need a
  root import for it. The rustdoc adds "and the built-in editor a component
  embeds" to the membership.
- **`testing`** keeps 6f1's rule ("what a headless test writes besides the
  driver", `pert-ui-runtime/src/testing.rs:1-24`) and gains 10 names:
  `Viewport` (every harness sets one), `KeyAction` (R3, R4, R5, R7),
  `FocusDirection` (R5), `SemanticValue` (`guide/testing.md:29`),
  `ClickCounter` (`:76`), and the names the testing chapter teaches for input
  by hand: `CommandKeyInput` (`command_key_input`), `PointerEvent`,
  `PointerKind`, `PointerButton` (`pointer_input`) and `InputOutcome`, which
  `pointer_input` and `key_input` return (the `Swatch` harness asserts it,
  `tests/foundation/prelude.rs:192`). The packaged examples' headless
  sessions write the pointer names (`examples/diagram_editor/session.rs`,
  `inventory/session.rs`, `log_explorer/session.rs`, `live_mixer.rs`).
  `KeyCode`, `Modifiers`, `CommandKey` and `Point` are in the prelude. The
  facade's `testing` (`crates/pert-ui/src/lib.rs:186-199`) re-exports the
  same list.

## 3. Module names and the private-module collisions

`windows`, `inspection` and `live` are private runtime modules today
(`crates/pert-ui-runtime/src/lib.rs:37`, `:229`, `:240`); a crate cannot also
define public modules of those names.

- **`windows` becomes the public module `windowing`; the private module
  keeps its name.** A public root module named `windows` breaks consumers
  that glob the crate root and depend on the Microsoft `windows` crate, which
  is in this workspace's own `Cargo.lock`. Reproduced with rustc 1.99.1 in a
  scratch crate: `use ui::*;` (a crate with `pub mod windows`) followed by
  `use windows::win32::Hwnd;` is E0659 "`windows` is ambiguous … a name from
  a glob import and an outer scope during import or macro resolution", and
  the same path outside a `use` silently resolves to the glob's module
  (E0433 "could not find `win32` in `windows`"). The root glob is how the
  guide's controls, surfaces, appearance and custom-widgets chapters, most
  facade tests and the gallery import (`use pert_ui::*`). `window`
  (singular) was considered and rejected as reading like one window rather
  than the windowing family. Keeping the private `windows` module also leaves
  the `api/` listing's private-path spellings unchanged (14 lines name
  `pert_ui_runtime::windows::WindowCloseAction`).
- **`inspection` and `live` keep their public names; the private modules
  are renamed** `inspection_service` (file `inspection.rs` →
  `inspection_service.rs`) and `live_source` (`live.rs` → `live_source.rs`).
  The internal paths that change are `crate::inspection::…` at
  `runtime.rs:494`, `:660` and `runtime/capture.rs:14`, `:64`, `:77`, and
  `crate::live::…` at `runtime.rs:487`, `:488`, `:491`, `:546`, `:550`,
  `:602`, `:2973`, `:4312`, `:6824`, `:6851`, plus the root re-exports
  `lib.rs:230-233` and `:304-306`; the implementation finds any others by
  compiling. No `api/` line names either private path. No crate named
  `inspection` or `live` is in `Cargo.lock`.
- **`parameter` and `commands`** collide with nothing at the runtime's root:
  `runtime::commands` is nested in the private `runtime` module
  (`runtime.rs:3`), and the core's private `parameter` and `live` modules are
  in another crate.

The plan's names otherwise stand: `parameter`, `commands`, `inspection`,
`live`, and `windowing` for "windows".

## 4. Collision review

As 6e required, every combination of globs a program might write was
checked for distinct items with the same name, and against common
consumer-local and dependency names.

- **Between pert-ui globs: none by construction.** Every runtime module
  re-exports `crate::X` items, so a name in several modules is one item
  (`KeyCode` in `prelude`, `commands` and `component`; `Rect` in `component`,
  `inspection` and `windowing`), which Rust accepts for globs. The facade's
  additions (`App` and its reports, `Svg`, `Driver`, `TestError`,
  `HeadlessApp`, `CpuRenderer`) have no runtime namesake. Glob conflicts are
  diagnosed only where a name is resolved, so the fixture of section 8.1
  resolves every exported name.
- **Explicit imports are not globs.** Importing one item twice by name is
  E0252 even through two paths to the same item (reproduced with rustc
  1.99.1). The exhaustive membership lists therefore use one child module
  per list (8.1), and `testing.rs` drops from its private `use crate::{…}`
  (`testing.rs:57-60`) every name its new `pub use` adds (section 5).
- **Modules against dependency crates.** The prelude and the thematic
  modules contain no modules, so only the root glob exposes `parameter`,
  `commands`, `windowing`, `inspection` and `live` as names. Section 3 removes
  the one known conflict. A consumer who depends on a crate with one of the
  other names and globs the root gets E0659 in a `use` path, which rustc's
  help resolves with `::name`; the prelude page says to glob the prelude, not
  the root. **Outside L2:** the facade's existing root module `svg`
  (`crates/pert-ui/src/lib.rs:38-39`) has the same hazard against the `svg`
  crate under the root glob; the implementation records it in TODO.md.
- **Consumer-local items** shadow glob imports without error, so local
  `Trace`, `Tree`, `View`, `Field`, `Form`, `Chip` or `Row` types in the
  corpus are unaffected (for example `examples/log_explorer/app.rs:155`,
  `examples/inventory/model.rs:54`). Glob against glob is an error only where
  the name is used; the corpus's three such sites are safe: the gallery's
  `mod live` (`apps/gallery/src/main.rs:13`) with files that glob both
  `super::*` and `pert_ui::*` (`interaction_check.rs:20`, `:1148`;
  `workspace_check.rs:2`, `:248`; `live.rs:10`, `:853`) and name no `live::`
  path; the facade's `platform::windows` (`platform/mod.rs:45`) and the test
  binaries' `mod windows` and `mod commands` (`tests/surfaces/main.rs:33`,
  `tests/collections/main.rs:14`), which are local items where they are named.
- **Standard-library globs.** No prelude name after L2 is an item of
  `std::io`, `std::fmt`, `std::collections`, `std::sync`, `std::rc`,
  `std::cell`, `std::time` or `std::path`. `component` keeps `Path` and
  `Component`, which equal `std::path::Path` and `std::path::Component`; a
  component that globs both `std::path::*` and `pert_ui::component::*` must
  name one explicitly. Today the prelude carries both conflicts; L2 confines
  them to component authors.
- **The facade's exposure.** The facade's root glob
  (`#[doc(inline)] pub use pert_ui_runtime::*`, `crates/pert-ui/src/lib.rs:28-29`)
  would re-export the runtime's new modules with the runtime's page text.
  The facade defines its own module for each, as it does for `prelude` and
  `component` (`:86`, `:143`), and its local modules shadow the glob. Every
  re-export in every module is `#[doc(no_inline)]`, so each item is
  documented once, at the crate root.

## 5. Public API

`crates/pert-ui-runtime/src/lib.rs`:

```rust
mod inspection_service; // was `inspection`
mod live_source;        // was `live`
mod windows;            // unchanged; the public module is `windowing`

/// The names an application's first program writes, for
/// `use pert_ui_runtime::prelude::*`. (Rustdoc: section 6.)
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::{/* the 195 names of section 2.1 */};
}

/// The names implementing `Component` writes (unchanged rule, plus the
/// press counter and the embedded editor).
pub mod component {
    #[doc(no_inline)]
    pub use crate::{/* today's 126 names */};
    #[doc(no_inline)]
    pub use crate::{
        ClickCounter, DraftAlign, DraftEditor, DraftEditorController, DraftSelection,
    };
}

/// Authoritative values: parameters, edit gestures, hosts, external
/// authority and the parameter-only controls.
pub mod parameter {
    #[doc(no_inline)]
    pub use crate::{/* the 54 names of section 2.3 */};
}

/// Commands, shortcuts and the command palette.
pub mod commands { #[doc(no_inline)] pub use crate::{/* 20 names */}; }

/// Windows inside a view and native windows, regions, splits and tool
/// windows.
pub mod windowing { #[doc(no_inline)] pub use crate::{/* 30 names */}; }

/// Inspection, capture, profiling and validation for an application's own
/// tooling and tests.
pub mod inspection { #[doc(no_inline)] pub use crate::{/* 61 names */}; }

/// Data from other threads: lossy live series and owned values through
/// mailboxes.
pub mod live { #[doc(no_inline)] pub use crate::{/* 29 names */}; }
```

`crates/pert-ui-runtime/src/testing.rs:57-65` becomes, so that no name is
imported twice (E0252):

```rust
use crate::{
    CommandKey, FlushReport, KeyChord, KeyCode, Modifiers, Point, RevealOutcome,
    RevealRefusal, Runtime, RuntimeError, Semantics, TextCommand, TextCommandOutcome,
    WidgetId,
};
use std::time::{Duration, Instant};

#[doc(no_inline)]
pub use crate::{
    CaptureRequest, CaptureTarget, ClickCounter, CommandKeyInput, Detail, FocusDirection,
    FrameImage, InputOutcome, InspectionQuery, KeyAction, PointerButton, PointerEvent,
    PointerKind, Selector, SemanticValue, Viewport,
};
```

`crates/pert-ui/src/lib.rs`: `prelude` and `component` keep their code
(`:86-98`, `:143-146`); `testing`'s `no_inline` list (`:187-190`) gains the
10 names; and five modules are added, each with its own rustdoc:

```rust
pub mod parameter { #[doc(no_inline)] pub use pert_ui_runtime::parameter::*; }
pub mod commands { #[doc(no_inline)] pub use pert_ui_runtime::commands::*; }
pub mod windowing { #[doc(no_inline)] pub use pert_ui_runtime::windowing::*; }
pub mod inspection { #[doc(no_inline)] pub use pert_ui_runtime::inspection::*; }
pub mod live { #[doc(no_inline)] pub use pert_ui_runtime::live::*; }
```

`crates/pert-ui/src/guide.rs` gains `pub mod authoritative_values` (section
6). `pert-ui-core` stays flat: the runtime's modules gather its items. No
signature, behavior, persisted format or real-time endpoint changes; no
backend type is re-exported.

**Module rustdoc intent**, each with one example that globs the prelude and
the module and compiles: purpose and audience (the layer-1 need); its
membership rule, closure and stated exclusions (section 2.3); that a name
may also be in the prelude or another module and the globs combine; that
items are documented at the crate root. `parameter`'s example is the state
chapter's first parameter example with a `Slider`; `live`'s builds a bridge
and a `LevelMeter`; `windowing`'s a `SplitPane`; `commands`' registers a
command with a named key; `inspection`'s queries one component's semantics.

## 6. Documentation

- **The prelude pages** (`pert-ui-runtime/src/lib.rs:329-387`,
  `crates/pert-ui/src/lib.rs:47-85`) are rewritten: the membership rule
  (1.2) in short paragraphs; the guarantee (1.3); the exceptions of 1.2 with
  the module or root import each needs; and "Where the other names are", a
  table mapping each family to its module or to the root (section 2.2),
  which replaces "Root names to add". The sentence "a custom widget that lays
  out, draws and handles input needs no other import"
  (`crates/pert-ui/src/lib.rs:53-56`, runtime `:371-372`) is removed: a custom
  component globs `component` too.
- **`component`** (`pert-ui-runtime/src/lib.rs:588-625`,
  `crates/pert-ui/src/lib.rs:100-120`): "Applications glob prelude; code that
  implements Component also globs this module" stays; the membership adds the
  embedded editor.
- **`testing`** (`pert-ui-runtime/src/testing.rs:1-24`,
  `crates/pert-ui/src/lib.rs:148-161`): the list names the input names.
- **The crate page** (`crates/pert-ui/src/root.md:37-51`): prelude for
  application code, `component` for custom components, `testing` for tests,
  and the five thematic modules by need. The runtime's README gets the same
  paragraph.
- **The custom-widgets chapter's "Imports"** (`guide/custom-widgets.md:13-44`):
  a widget globs `prelude` and `component`.
- **"Authoritative values" (LD2).** A new chapter
  `crates/pert-ui/src/guide/authoritative-values.md`, module
  `crate::guide::authoritative_values`, after the state chapter. It receives
  the state chapter's parameter sections unchanged in substance: "Parameters"
  (`guide/state.md:90-121`), "Local parameter gestures" (`:123-158`),
  "Parameter families and host notifications" (`:160-209`), "Updating
  bindings after a sink" (`:211-268`), "Effective overlays and remote
  outcomes" (`:270-301`) and "Dynamic applications and external authority"
  (`:391-419`), with an opening paragraph on when a value is authoritative.
  Its examples import `use pert_ui::{parameter::*, prelude::*};`, adding
  `component::*` where they implement a component. The state chapter keeps
  "Choosing a way", "Signals and controls", "Worker results and live data"
  and "Bounded updates and recovery"; its introduction (`:1-5`) and the
  `for_parameter` row's link (`:18`) point to the new chapter, and its title
  becomes "State". Links updated: `guide.rs:7-9` and `:64-66` (chapter list
  and module title), `root.md:59-61` (the map's parameter row links the new
  chapter), `guide/controls.md:217`. `tests/foundation/readme.rs:58-83` adds
  the new page to its list.
- **Representative programs in the guide** are reshaped so the application
  part is module-level items, including a `build` function where the harness
  needs handles, with `use pert_ui::prelude::*;` only, and the harness is the
  doctest's `fn main` beginning with `use pert_ui::testing::*;` (a `use` in a
  function body does not reach module-level items). R2 has no harness and
  keeps its shape. This is the application-shaped example slice 9 asks for
  (`plans/app-author-improvements.md:356-358`).

## 7. Migration manifest

Rule: application code that names a family's names globs that module;
code implementing `Component` globs `component`; tests, example sessions and
headless doc examples that set a viewport or send input glob `testing`; a
root-only name is imported by name. Nothing changes beyond what compilation
requires, except the representative programs (made prelude-only, section
6), the representative custom components (made `prelude` + `component`
only) and the new chapter's taught globs.

**Method.** Revision 1 scanned whole files, so an import inside one module
appeared to satisfy another. This inventory scans **by import scope**: each
inline `mod` block and each doc example is its own scope, a scope inherits
its parent's imports only through `use super::*`, and a scope's needs are the
removed names it writes minus its globs' members and its explicit imports.
It covers every maintained file at 2a406a4 that globs a prelude or
`component`.

**Maintained, migrated:**

| Consumer (scope) | Needs |
|---|---|
| `examples/getting_started.rs` | `use pert_ui::prelude::*;` at module level, `fn main() -> Result<(), AppError>` (R1) |
| `examples/live_mixer.rs` | `parameter` (`EditToken`, `Parameter`, `ParameterHost`, `ParameterId`, `ParameterMapping`, `ParameterSpec`, `ParameterValue`, `Slider`), `windowing` (`SplitAxis`, `SplitPane`), `testing` in its session (`PointerButton`, `PointerEvent`, `PointerKind`, `Viewport`) |
| `examples/diagram_editor/app.rs` | `commands` (`CommandPalette`, `CommandPaletteController`), `windowing` (`EdgeMode`, `ToolEdge`, `ToolWindow`, `ToolWindowLayout`, `WindowCloseAction`), `component` (`InputOutcome`, `Shape`) |
| `examples/diagram_editor/canvas.rs` | `use pert_ui::{component::*, prelude::*};`: its root import list (`:9-13`) is removed: every name it lists is in one of the two globs, including `ClickCounter`, `DraftEditor`, `DraftEditorController` and `DraftSelection`, so the component guarantee runs through `component` (8.1); the names it writes that leave the prelude are `ArrangeContext`, `Component`, `DrawError`, `InputOutcome`, `Invalidation`, `MeasureContext`, `Painter`, `PointerButton`, `PointerKind`, `Rect`, `Shape`, `UpdateContext` |
| `examples/diagram_editor/session.rs`, `inventory/session.rs`, `log_explorer/session.rs` | `testing` (`PointerButton`, `PointerEvent`, `PointerKind`, `Viewport`), `windowing` for `WindowCloseAction` in the diagram session |
| `examples/inventory/app.rs`, `log_explorer/app.rs` | `windowing` (`RegionLayout`; `SplitAxis`, `SplitPane` in the log explorer) |
| `apps/gallery/src/main.rs:22-25` | `component` (`Component`, `UpdateContext` for its `Readout`), `parameter` (`ParameterId`, `ParameterSpec`), `testing` (`KeyAction`) |
| facade tests: `foundation/prelude.rs` | rewritten (8.1) |
| `foundation/component_module.rs` (outer scope, `:10`) | `testing` for its harness (`FocusDirection`, `KeyAction`, `PointerButton`, `PointerEvent`, `PointerKind`, `Viewport`, `:198`, `:211-239`); `mod widget` and `mod every_name` are unchanged |
| `foundation/public_api.rs` | `parameter` (`NormalizedValue`, `ParameterId`, `ParameterMapping`, `ParameterSpec`) |
| `controls/component_workflows.rs` | `component`, `parameter` (`Knob`, `Slider`, `ParameterId`, `ParameterSpec`), `testing` |
| `surfaces/window_close.rs` | `component`, `parameter` (`Parameter`, `ParameterHost`), `windowing` (`WindowCloseAction`), `testing` |
| `foundation/readme.rs`, `foundation/main.rs` | the new chapter in the page list; `mod modules; mod vocabulary;` (8.1) |
| runtime tests: `input_contracts.rs`, `live_sources.rs`, `runtime.rs`, `testing.rs` | `component`; `parameter` in `runtime.rs` (`ParameterId`, `ParameterSnapshot`, `ParameterSpec`); `testing` in `runtime.rs` and `testing.rs`; root `Density`, `ThemeMode` in `input_contracts.rs` |
| runtime doc examples: `collection_filter.rs`, `color_swatch.rs`, `runtime.rs`, `runtime/content_search.rs`, `runtime/pointer.rs`, `runtime/reveal.rs` (three examples), `text_fields/inline_text.rs`, `virtual_list.rs`, `testing.rs` | `testing` (`Viewport`; `KeyAction`, pointer names where written); `reveal.rs`'s `ChildReveal` example adds `component`; `testing.rs:32` drops `use pert_ui::SemanticValue;` |
| `text_fields/draft.rs` doc example | `component` (it embeds a `DraftEditor` in a component) and `testing` |
| `crates/pert-ui-runtime/README.md` (examples at `:19`, `:50`, `:222`, `:268`) | `parameter`; `component` at `:50`, `:222` |
| facade docs: `lib.rs` module pages, `root.md` | section 6 |
| guide: `controls.md:192` | `testing` (`KeyAction`, `Viewport`) |
| guide: `custom-widgets.md:52` | `component` |
| guide: `getting-started-continued.md:57-74` | `parameter` (`ParameterId`, `ParameterSpec`, `Slider`) |
| guide: `getting-started-continued.md:88-102` | `windowing` (`SplitAxis`, `SplitPane`), root `Workspace` |
| guide: `getting-started-continued.md:118-152` (indicator) | `use pert_ui::{component::*, prelude::*};` |
| guide: `layout.md:82` | `parameter` (`ParameterId`, `ParameterSpec`, `Slider`), `testing` (`Viewport`) |
| guide: `layout.md:540` (R3), `surfaces.md` (R4, R5), `state.md:312` (R7), `testing.md` (drops `:29`, `:76`) | sections 1.1 and 6 |
| guide: `state.md:92`, `:125`, `:168`, `:225` | move to the new chapter with `parameter` (`component` at `:125`, `testing` at `:225`) |
| diagnostics fixtures | `r01_contentrows_not_impl.rs:2` becomes `use pert_ui::{component::*, prelude::*};`; `s01_stateowner_impl.rs:2` becomes `use pert_ui::{UiSettings, prelude::*};`. Both on the same line, so the snapshots' line numbers stay; the expected `.stderr` diff is none. The other thirteen fixtures name only names that stay in the prelude, or root imports. |
| `plans/usability/task4-app/src/lib.rs:7` | removed: `CommandScope` and `KeyChord` stay in the prelude (R6) |
| unaffected by the scan | `examples/dynamic_documents.rs`, `diagram_editor/main.rs`, `diagram_editor/editor.rs`, `tests/presentation/svg.rs` (its prelude scope names only `Svg`), `app/headless.rs`, `guide/diagnostics.md`, `docs/manual.md:24`, `:102`, the other gallery files and `apps/native-checks` (root imports, `pert-ui-focus-probe.rs:8`) |
| records | `api-audit/decisions.md` (G5-26, G5-11), `plans/app-author-improvements.md:356-382`, `plans/developer-usability.md:41-46`, `layering-plan.md` progress (section 9) |

**Frozen, not migrated:** the round 1 submissions under
`plans/usability/runs/` (`r1-1-claude`, `r1-1-codex`, `r1-2-claude`,
`r1-2-codex` and `r1-4-claude` would need imports); the probes and kit
copies under `~/pert-ui-usability/`; the audit's diagnostic corpus
`plans/usability/api-audit/diagnostics/` (`cases/` and `control/`),
evidence at its own commit; the reserved task text
`plans/usability/reserved/5-job-monitor.md`; and the CHANGELOG's history.

## 8. Tests

L2 changes no behavior, so there are no new-behavior tests and 6f3's
behavioral-mutation rule does not apply. Every test below is a contract
test of an import guarantee; its evidence is the compile failure a named
removal causes, recorded in the progress entry as 6e recorded "fail to
compile with the re-exports reverted".

### 8.1 Facade tests (`crates/pert-ui/tests/foundation/`)

- **`prelude.rs`, rewritten.** Header contract: "application code of the
  representative programs needs only `pert_ui::prelude::*`, their tests add
  `pert_ui::testing::*`, and a custom component adds
  `pert_ui::component::*`".
  - `Swatch` (today `:12-170`) moves into `mod widget { use
    pert_ui::{component::*, prelude::*}; … }`. The display-list check
    (today `:197-203`, `DisplayList`, `DrawCommand`) moves into the same module
    as `pub fn paints_a_fill(ui: &mut Runtime) -> Result<bool, DrawError>`:
    `Runtime::paint` returns `DrawError`, which does not convert into
    `RuntimeError` (`runtime/compose.rs:27-32`), and the test's boxed error
    takes either.
    The driving test globs `prelude` and `testing` and keeps every assertion:
    the pointer press with `InputOutcome::Handled` (`testing`), the Space key,
    the press count and `widget::paints_a_fill`. Removing `MeasureContext`
    from `component` or `InputOutcome` from `testing` fails it.
  - `representative_programs_import_only_their_globs`: `include_str!`s
    `examples/getting_started.rs`, `examples/diagram_editor/canvas.rs` and
    the guide pages, finds each representative block by its heading and index
    (and fails if it cannot), and asserts the ordered list of `pert_ui::…`
    paths in it: R1, R2 `["pert_ui::prelude::*"]`; R3, R4, R5, R7
    `["pert_ui::prelude::*", "pert_ui::testing::*"]` with the second inside
    `fn main`; every testing chapter block `["pert_ui::prelude::*",
    "pert_ui::testing::*"]`; the indicator, `Marker` and the canvas
    `["pert_ui::{component::*, prelude::*}"]` and nothing else, the canvas
    included: no named root import is allowed in any of them. Adding
    `use pert_ui::KeyChord;` to R5's application part fails it, naming R5.
  - `the_native_prelude_holds_what_app_reports` (`:301-310`) stays.
  - `the_prelude_holds_the_types_its_members_take_and_return`
    (`:213-299`) is replaced by `modules.rs`.
- **`vocabulary.rs`, new: the closure is sufficient.** One child module per
  group of section 2.1's vocabulary table, each with
  `use pert_ui::prelude::*;` only, constructs or configures a member with
  every name of its group in a constructor, builder, callback pattern or
  match: for example `SizeLimits::new(Size::new(10.0, 10.0)?,
  Constraints::new(None, Some(40.0))?)?`,
  `RichText::new().span(TextSpan::new("a").color_role(ColorRole::Primary))
  .tinted_icon(Icon::Settings, TextColor::Inherit)`,
  `NumberFormat::new("n", |v: f64| v.to_string(), |s| s.parse().map_err(|_|
  NumberInputError::…))`, `KeyChord::new(CommandKey::Named(KeyCode::Delete),
  Modifiers::…)`, a `Table` with its controller, columns, sort and cell
  editor. Every one of the 112 vocabulary names and the program clause's
  closure appears at least once. Removing `Constraints` or `TextSpan` from
  the prelude fails it.
- **`modules.rs`, new.**
  - `mod every_name` with **one child module per list**, so no name is
    imported twice (E0252): `mod prelude_names`, `mod component_names`,
    `mod testing_names` (the complete module: the existing `CaptureRequest`,
    `CaptureTarget`, `Detail`, `Driver`, `FrameImage`, `InspectionQuery`,
    `Selector`, `TestError`, `HeadlessApp` under `native` and `CpuRenderer`
    under `cpu`, and the 10 additions), `mod parameter_names`,
    `mod commands_names`, `mod windowing_names`, `mod inspection_names`,
    `mod live_names`, each `#[allow(unused_imports)] use
    pert_ui::<module>::{…};` with every member of sections 2.1, 2.3 and 2.4
    (the `native` and `svg` names under `cfg`), so a removed re-export fails
    to compile even where no program uses it, as `component_module.rs:159`
    does for `component`.
  - **Module sufficiency:** one child module per thematic module, each with
    `use pert_ui::{prelude::*, <module>::*};` only, constructs the members
    whose closure names section 2.3 added: `CaptureTarget::region(&id,
    Rect::new(…)?)` and a match on `EventKind::KeyInput(KeyEvent { .. })`
    (`inspection`); `ToolWindowChange::…with_geometry(None, Some(Rect::new(…)?))`
    and a match on `SplitRule::Ratio(NormalizedValue)` (`windowing`);
    `CommandScope::Window(id)` (`commands`); `LevelMeter::new(source,
    ParameterSpec::new(…)?)` (`live`); `ShortcutBinding::new(KeyCode::Enter,
    "Save")` with `modifiers: Modifiers::…` (`inspection`, with the prelude's
    key names); a `Slider` bound to a parameter (`parameter`). The window
    identity comes from a helper in the test's own scope.
  - **`every_glob_combines`:** one module globs `pert_ui::*`, `prelude`,
    `component`, `testing`, `parameter`, `commands`, `windowing`,
    `inspection` and `live`, and resolves **every** name the eight module
    lists export with `#[allow(unused_imports)] use self::{A as _, B as _,
    …};`. A `use self::Name as _` resolves the name in every namespace in
    which it exists (types, values such as `series_bridge` and
    `DRAG_THRESHOLD`), and any glob-ambiguous name is E0659. The list is the
    union of the complete `every_name` lists, feature-gated names under the
    same `cfg`, so it resolves every name any of the eight modules exports,
    including the facade-only `Driver`, `TestError`, `HeadlessApp` and
    `CpuRenderer`, which are not crate-root names; names only the root glob
    exports collide with nothing else that is globbed.
    **Guard verified:** in a scratch crate with rustc 1.99.1, two modules
    globbing one item resolved without error, and adding a third glob that
    re-exports a different item under the same name failed with E0659
    "ambiguous because of multiple glob imports of a name in the same module".
    The implementation repeats that check once on the fixture with a
    temporary conflicting re-export of a `testing`-only name (for example a
    different item exported as `commands::Driver`) and records the E0659.
- **Doctests as guarantees:** R1 (the example and its guide inclusion),
  R2, R3, R4, R5, R7, the testing chapter, the indicator, `Marker`, the
  `component` page's `Lamp` (`pert-ui-runtime/src/lib.rs:631-692`) and the
  new modules' examples. Named removals that fail them: `ScrollAxes` from the
  prelude (R1), `HiddenActive` (R3), `DialogOutcome` (R4), `KeyChord` (R5),
  `SemanticValue` from `testing` (the testing chapter), and `DraftEditor`
  from `component`, which fails `examples/diagram_editor/canvas.rs` once its
  root import list no longer names it (section 7).
- **R6** is outside the workspace by design (its own `[workspace]`,
  `plans/usability/task4-app/Cargo.toml:10-11`). The gate builds a scratch
  copy against the workspace without warnings, as 6e did for the round 2
  fixture (`api-audit/fix-plan.md:547-548`), and the progress entry records
  it; it is not a CI test.

### 8.2 Diagnostics and listings

- `cargo test --locked -p pert-ui-diagnostics -- --ignored`: the s01
  fixture must still produce the sealed-trait diagnostic and r01 the
  `ContentRows` diagnostic; any `.stderr` change is reviewed like the `api/`
  diff and must not turn either into a missing-name error.
- The `api/` diff is reviewed by its header lines: the
  `pub (struct|enum|trait|fn|use) pert_ui_runtime::<module>::` lines are
  compared, sorted, with sections 2.1 to 2.4 (section 9).

### 8.3 Feature configurations

The facade's prelude differs with `native` and `svg`, and `testing` with
`native` and `cpu`. `native` cannot be enabled alone
(`crates/pert-ui/src/lib.rs:23-26`), so the configurations are the default
(`desktop`), `--no-default-features`, `--no-default-features --features
desktop-cpu`, `--features svg` and `--features cpu`: Clippy, tests and
doctests in each, and rustdoc with `-D warnings` in each. The rendered pages
of the prelude, `component`, `testing` and the five modules are reviewed in
the default and `--no-default-features` builds.

## 9. Scope, versions and evidence

- **Version:** 0.3.73, a workspace patch bump with the internal
  requirements and `Cargo.lock`.
- **`api/` diff:** runtime: `pub mod pert_ui_runtime::{commands, inspection,
  live, parameter, windowing}` with their members' blocks; the 174 names'
  `pert_ui_runtime::prelude::` blocks removed and `NumberInputError`'s added;
  five `component` blocks and 10 `testing` re-exports added. The listing
  repeats each item's block under every path (`complexity-inventory.md:12-14`),
  so this diff is thousands of lines and mechanical; the review checks it by
  header lines. Facade: five `pub mod` lines with their globs,
  `pub mod pert_ui::guide::authoritative_values`, and the 10 `testing`
  re-exports in each configuration's file. The private renames change no
  listing line.
- **CHANGELOG 0.3.73:**
  - Breaking (L2): the prelude holds the names an application's first
    program writes and the built-in components with their vocabulary (the
    rule of section 1.2); 174 names leave it. Replacement imports: a custom
    component adds `use pert_ui::component::*;`; a test adds
    `use pert_ui::testing::*;` (for `Viewport`, `KeyAction`,
    `FocusDirection`, `InputOutcome` and pointer events); parameter and
    authority names, `Slider`, `Knob` and `InlineSlider`
    `use pert_ui::parameter::*;`; command names other than `CommandScope`,
    `KeyChord`, `CommandKey`, `KeyCode` and `Modifiers`
    `use pert_ui::commands::*;`; windows, regions, splits and tool windows
    `use pert_ui::windowing::*;`; `LevelMeter`, `MeterAxis` and `Trace`
    `use pert_ui::live::*;`; theme, settings, fonts, strings, workspace,
    `DescribeContext`, `HexColorError`, `PhysicalExtent` and `VirtualAxis` by
    name from the root, for example `use pert_ui::{ThemeMode, Workspace};`.
    Each moved name is listed.
  - Added: the modules `parameter`, `commands`, `windowing`, `inspection`
    and `live` in `pert-ui-runtime` and `pert-ui`, with their members;
    `NumberInputError` in the prelude; 10 names in `testing`;
    `ClickCounter`, `DraftAlign`, `DraftEditor`, `DraftEditorController` and
    `DraftSelection` in `component`.
  - Documentation: the guide's "Authoritative values" chapter, with the
    parameter sections moved from the state chapter (links to
    `guide::state#parameters` and its sibling anchors move to
    `guide::authoritative_values`); the prelude, `component` and `testing`
    pages and the crate pages; representative examples shaped as an
    application part and a test.
- **Gate:** AGENTS.md's public-API row: the workspace checks (formatting,
  Clippy, tests, the five headless example sessions, doctests, warning-free
  rustdoc and the diagnostics snapshots), the API listing gate, the focused
  tests of section 8 and semantic review; plus the feature configurations of
  8.3, the R6 fixture build, the recorded collision-guard check (8.1),
  `git diff --check`, and a link check over `docs/`, `plans/` and the guide
  for the moved anchors.
- **Records:**
  - `api-audit/decisions.md` G5-26 (`:1525`): "The application-prelude rule
    is superseded by layering part L2 (0.3.73, layering-design-L2.md); the
    `component` rule stays." G5-11 (`:1467`): the live-data module exists.
  - App-author slice 9 (`plans/app-author-improvements.md:360-365`,
    `:377-382`): the smaller prelude and the thematic modules, including the
    live-data module, are met and leave its inputs; the tutorial and
    application-shaped examples stay.
  - `plans/developer-usability.md:41-46`: L2 lands on this branch before
    round 2, so round 2 meets the prelude and module layout (plan, "Round 2
    and attribution").
  - `layering-plan.md`: the L2 entry in progress, and the membership bullet
    (`:173-178`) records D1's decision.
- **Later parts:** L3's N3 moves host methods off `Runtime`; the
  representative tests call `Runtime::set_viewport`, `key_input` and
  `focus_next`, so N3 must keep them reachable with `prelude` and `testing`
  (as inherent methods, or through a trait `testing` re-exports). L3's N7
  changes the testing chapter's `Each` paths, not its imports. L4a
  (0.3.72) made `EventContext::defer` runtime-lived, added
  `defer_while_shown` and removed `defer_detached` without a new type; R3's
  deferred rename uses the call L4a left, and no representative program
  writes a new name because of it. **L4b** (one draft policy) proposes
  `Table::on_edit_end` with `CellEditEnd` and `CellEditEndReason`
  (`layering-design-L4b.md:415-438`); `Table` is a prelude member, so the
  closure applies to the builder whether or not a representative program
  calls it. `CellEditEnd` and `CellEditEndReason` join the prelude, and with
  them the vocabulary the reason carries: `EditorCompletion` (the
  `Completed` payload, root-only today), its `EditorOutcome` and
  `EditorTrigger`, and `FocusDirection` (`EditorTrigger`'s Tab payload,
  `editor_session.rs:29-39`, `:76-80`); the last three stay in `component`
  too, and `FocusDirection` in `testing`. L4b adds a prelude-only fixture to
  `vocabulary.rs` whose `on_edit_end` callback matches each
  `CellEditEndReason`, the completion's `EditorOutcome` and `EditorTrigger`,
  and the Tab trigger's `FocusDirection`; L4b's own checkpoint records this
  instead of "beside `CellEdit`" (`layering-design-L4b.md:590`).
- **TODO.md:** no entry touches the prelude, the modules or the guide
  chapters at 2a406a4. The implementation adds one for the `svg` root module
  (section 4).

## Corrections to the plan

- **Rule and size:** the plan's membership is program-based
  (`layering-plan.md:173-178`) with an estimate of 120 to 150 names. Its own
  rule gives about 45; D1's recommended widening gives 195. The estimate fits
  neither, and D1 is a change to the approved rule, not a refinement of it.
- **The settings form** is the controls chapter's four-control example
  (`guide/controls.md:223-237`), not a recipe.
- **`windows`** cannot be the public name (E0659 against the `windows`
  crate, section 3); the module is `windowing`.
- **`guide/state.md:63`** is now `:90`: L1 added the "Choosing a way" table
  above the parameter sections.
- **"native, svg, cpu":** `native` needs a renderer feature
  (`crates/pert-ui/src/lib.rs:23-26`), and `cpu` changes `testing`, not the
  prelude; section 8.3 lists the real configurations.
- **Layer 1's "windows and surfaces"** (the layer table) and the
  representative dialog and menu: the surfaces chapter's components (dialogs,
  menus, popovers, toasts) are layer 0; layer 1's surfaces are the in-view
  and native windows of `windowing`.
- **Sliders are layer 1:** `Slider`, `Knob` and `InlineSlider` take only a
  `Parameter`, so the "controls" of layer 0 do not include them.
- **Consumers:** most facade tests and gallery pages, and four guide
  chapters, glob the crate root rather than the prelude, so the prelude change
  does not reach them; the root glob is where the module names matter
  (section 4).

## Decisions for the maintainer

- **D1: widening the approved membership rule.**
  - **Recommended:** the plan's program clause plus the component clause
    (every built-in component outside the thematic families with a
    constructor needing no family name), the application-wide names, and the
    closure of section 1.2: 195 runtime names. An application configures any
    prelude component through the prelude alone, except along the named
    paths of 1.2 (a family's constructor, an editor session, a service, a
    received error), which the prelude page lists; this matters because rustc
    suggests no import for a missing facade name. It changes the plan's
    approved rule (`layering-plan.md:173-178`), which the maintainer records.
  - **Alternative:** the plan's rule as approved: the program clause alone,
    about 45 names, the smallest readable page; every other control and its
    vocabulary is a root import (`use pert_ui::{Tabs, Table};`), and a
    forgotten one is a bare E0425.
- **D2: application-wide services** (theme, UI settings, fonts and icons,
  strings and locale, the workspace document: 25 names).
  - **Recommended:** root only, listed on the prelude page's "Where the other
    names are". No representative program writes them; they are reached
    through `Runtime` methods (`theme`, `ui_settings`, fonts, workspace
    capture) and configured once, not per control. They are 1.2's service
    exceptions (`NumberLocale`, `FontId`, `FontRole`).
  - **Alternative:** keep them in the prelude, as members under 6e's rule:
    a theme switch, a localized number format or a saved layout needs no
    import, at the cost of 25 names that no layer-0 component's own
    constructors need.

## Review record

- **Revision 1, Codex review (2026-10-27):** needs revision, with seven
  should-fix points. It confirmed the 368 to 192 and 176 partition, the
  `windowing` name, the private-module renames and the E0659 and E0433
  analysis, found D2 compatible with the representative-program guarantees,
  and found import-contract tests without behavioral mutations appropriate.
  Revision 2 (rebased on 2a406a4, version 0.3.73 after L4a's 0.3.72)
  addresses each point:
  1. **D1's rule:** the rule is closed under one audit of every member
     (1.2, clause 4), which adds `Constraints`, `TextSpan`, `TextColor`,
     `NumberInputError` and keeps `CommandKey`, `KeyCode`, `Modifiers`; the
     parameter-only `Slider`, `Knob` and `InlineSlider` move to `parameter`;
     the remaining paths are named exceptions; `vocabulary.rs` constructs
     every vocabulary name with the prelude alone; D1 is described as a
     widening of the approved rule (`layering-plan.md:173-178`). The prelude
     is 195 names; 174 leave.
  2. **Module dependencies:** each module is closed one level under the same
     audit (2.3): `Rect` and `NativeWindowId` join `inspection` and
     `windowing` as needed, `NormalizedValue` joins `windowing`,
     `NativeWindowId` joins `commands`, `ComponentDetails`, `Invalidation`,
     `KeyEvent` and `Viewport` join `inspection`, `ParameterSpec` joins
     `live`; `ShortcutBinding`'s key names are in the prelude; the remaining
     cross-family record fields and error payloads are stated exclusions;
     each module has a `prelude` + module sufficiency fixture (8.1).
  3. **The Swatch harness:** `InputOutcome` joins `testing` (what
     `pointer_input` and `key_input` return), and the display-list check
     moves into the component scope as a helper, keeping every assertion.
  4. **Missed consumers:** the inventory is redone by import scope (section
     7); `getting-started-continued.md:57-74` and `:88-102`,
     `component_module.rs`'s outer harness and the other scope-level needs
     are added, and the file-wide scan's false entries are dropped.
  5. **E0252:** one child module per exhaustive list, and `testing.rs`'s
     private import list without the names its `pub use` adds (section 5),
     both reproduced as E0252 in a scratch crate.
  6. **The collision fixture:** it resolves every exported name with
     `use self::{… as _}`, across namespaces; the guard was verified with a
     conflicting re-export in a scratch crate, and the implementation repeats
     that check on the fixture.
  7. **The `DraftEditor` experiment:** `canvas.rs` migrates to
     `use pert_ui::{component::*, prelude::*};` and drops the `Draft*` root
     imports, so removing `DraftEditor` from `component` fails it, and the
     import-scope test guards the canvas's imports.
- **Revision 2, Codex recheck (2026-10-27):** needs revision, with four
  should-fix points; points 1, 2, 4, 5 and 7 of the first review were
  resolved, and the partition (194 kept, 1 added, 174 out) and the modules'
  one-level closure were confirmed on a sample of 32 names. Revision 3
  addresses each point:
  1. **The paint helper** returns `Result<bool, DrawError>`, since
     `DrawError` does not convert into `RuntimeError`
     (`runtime/compose.rs:27-32`).
  2. **The canvas:** `ClickCounter` joins `component` (the canvas stores and
     constructs it, `canvas.rs:77`, `:100`), the canvas's root import list is
     removed, and the import-scope test allows the representative custom
     components no named root import (2.4, 5, 7, 8.1, 9).
  3. **The collision fixture** enumerates the complete `testing` module,
     including `Driver`, `TestError`, `HeadlessApp` under `native` and
     `CpuRenderer` under `cpu`, drops the assumption that every module
     member is a crate-root name, and its recorded guard conflicts on a
     `testing`-only name (8.1).
  4. **Later parts** keep the whole closure rule (1.3); L4b's
     `on_edit_end` brings `CellEditEnd`, `CellEditEndReason`,
     `EditorCompletion`, `EditorOutcome`, `EditorTrigger` and
     `FocusDirection` into the prelude, with a prelude-only fixture in L4b
     (section 9).
