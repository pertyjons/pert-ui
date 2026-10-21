# claude report: G5 (discovery, D1 and D2)

Commit audited: d4ba108 (`crates/` unchanged at HEAD 8b9c1d3; provenance.json, toolchain rustc 1.99.1).

## Dispositions

| item | disposition | coverage |
|---|---|---|
| D1, paths | findings (1) | All 14 round 1 friction entries of category *discover* or *compile* (r1-1-claude 2, r1-1-codex 3, r1-2-claude 3, r1-2-codex 1, r1-3-claude 2, r1-3-codex 1, r1-4-claude 1, r1-4-codex 1) read in full, plus r1-4-claude 15:57 *understand* (paths in item examples) and a text search of all FRICTION/summary/acceptance files for import, prelude, root and re-export mentions; the triage reports (group A in full, B–E searched). Facade module layout checked in all 11 `api/pert-ui/*.txt` listings (only `cpu`, `images`, `svg`, `guide`, `prelude` are modules; every widget, including `Svg`, is at the root). Nameability check: every type that a runtime or core listing signature names is exported at its crate root (only the private `Sealed` supertrait is not); `pert_ui::cpu` re-exports both pert-ui-cpu types. One throwaway experiment (toy crates, see D1-1) on rustc's import suggestions. |
| D2, prelude | findings (8, of which 2 are policy questions) | Root imports extracted mechanically from all 81 `.rs` files of the 8 round 1 submissions (`src/`, `tests/`), the packaged examples (live_mixer, dynamic_documents, diagram_editor, log_explorer, inventory, getting_started) and apps/gallery: 57 distinct non-prelude names imported or path-qualified from `pert_ui::` (plus `cpu::CpuRenderer`), each classified below; the four glob importers (r1-1-codex, r1-3-codex, dynamic_documents, gallery) were scanned for non-prelude names used. Prelude membership parsed from crates/pert-ui-runtime/src/lib.rs:328-470 and crates/pert-ui/src/lib.rs:65-74 (288 runtime names + 6 facade names). Reverse direction: all 288 prelude names checked against the rule by group. Closure check: every method of every prelude type in semantic/pert-ui-core.txt, pert-ui-runtime.txt and pert-ui.txt, every public field of a prelude type (candidates/fields.txt), every enum-variant payload of a prelude enum (api/pert-ui-runtime/default.txt, api/pert-ui-core/default.txt), and the full `Component` trait (runtime.rs:1019-2476) scanned for root types the prelude lacks; every root type implementing `Component` (candidates/types.txt) checked for membership. The guide's code blocks (crates/pert-ui/src/guide/*.md) scanned for root imports. |

### Classification of the 57 non-prelude names (D2 input)

| names | used by | rule says | result |
|---|---|---|---|
| `Modifiers`, `KeyChord`, `CommandKey`, `CommandScope` | 6–8 r1 code bases; diagram_editor, inventory, log_explorer, live_mixer; guide/diagnostics.md:29, surfaces.md | keyboard types an application configures; `CommandKey` is the field and constructor type of the prelude's `CommandKeyInput` | included, missing: D2-1 |
| `CommandId`, `CommandReceipt`, `CommandStatus`, `CommandSource` | diagram_editor, inventory, log_explorer, live_mixer | types prelude widget constructors take and prelude controllers return | included, missing: D2-1 |
| `HeldInputId` | diagram_editor | keyboard (held keys) an application registers | likely included: D2-1 (medium confidence) |
| `ListDrop`, `DropPosition` | r1-2-claude (and pilot) | types widget builders take | included, missing: D2-2 |
| `LevelMeter`, `MeterAxis` | live_mixer | widgets with their builder types | included, missing: D2-3 |
| `Semantics`, `SemanticRole`, `SemanticValue`, `SemanticAction`, `InspectValue` | r1-1-*, r1-3-*, r1-4-*, diagram_editor, inventory, live_mixer, gallery | inspection, root; but `ComponentDetails` (prelude) is built from them | conflict in the rule: D2-4 |
| `EditOutcome`, `PowerExponent`, `ParameterSnapshot` | live_mixer, gallery; guide/state.md:96,140,197 | state and parameters (spec payload, binding outcome, binding state) | included, missing: D2-5 |
| `DeliveryMonitor`, `DeliverySnapshot`, `SinkError`, `SinkErrorKind`, `EditToken`, `ParameterValue` | live_mixer, dynamic_documents | "bindings and hosts" vs "host-integration" | ambiguous: policy question in D2-5 |
| `ViewScale`, `OpaqueColor`, `MaskImage` | diagram_editor, gallery; guide/appearance.md:96,145 | type a prelude builder takes (`ScaleLimits::new`), theme configuration (`Palette::from_seeds`), drawing type `Painter` records and a test reads back | included, missing: D2-6 |
| `NativeWindowId` | r1-1-claude | facade: "App and what App reports" (`App::on_close` handler) | included, missing: D2-7 |
| `CaptureRequest`, `CaptureTarget`, `FrameImage`, `InspectionQuery`, `Selector`, `DebugMode`, `ValidationCode`, `ComponentInspection` (glob) | 7 of 8 r1 runs, gallery, diagram_editor, log_explorer | capture, inspection, diagnostics: root | consistent (usage noted in D2-8) |
| `Mailbox`, `UiSender`, `SendError`, `Capacity`, `Delivery`, `SeriesConfig`, `SeriesEndpoint`, `SeriesSource`, `Timestamp`, `series_bridge` | live_mixer, log_explorer, diagram_editor; guide/state.md:197 | live-data plumbing: root | consistent (but see D2-3 for `SeriesSource`) |
| `Clipboard`, `ClipboardError`, `PresentationAction`, `PresentationStatus`, `list_directory` | inventory, log_explorer, r1-1-claude, diagram_editor | host integration (`list_directory` is documented "for a host to call on its own worker thread", file_browser.rs:273-274) | consistent |
| `ApplicationError` | all four application probes' `main.rs`, inventory/app.rs, gallery | "Other error types stay at the crate root" | consistent (usage noted in D2-8) |
| `ClickCounter` | diagram_editor | not named by `Component` signatures; kept out by a recorded decision (plans/archive/diagram-probe-improvements.md:433-434) | consistent |
| `ContentQuery` | log_explorer | a runtime search service, not a widget, state or `Component` type | consistent |
| `check_redescription` | r1-3-claude | conformance test helper, not a type of any included group | consistent |
| `cpu::CpuRenderer` | 7 r1 code bases, diagram_editor, gallery | feature module; a module path is not a finding | consistent |

Redundant root imports of prelude names are common (r1-1-claude imported 20, r1-4-codex 11; the guide itself writes `use pert_ui::{prelude::*, EditOutcome}` and `use pert_ui::Orientation` beside the prelude at layout.md:67-68); r1-1-codex glob-imported both (`use pert_ui::{prelude::*, *}`). They are not departures, but they show that consumers do not predict membership (r1-2-claude 15:47: "The prelude's membership rule explains it, but the split is hard to predict").

## Findings

### claude-D1-1 Root-only names a facade consumer needs get no compiler route to their path

- **Item and kind:** D1, project rule (AGENTS.md "Ordinary application code uses the public facade"; the rubric: a finding is an item needed and found late).
- **Paths and configurations:** `pert_ui::{Modifiers, KeyChord, CommandKey, CommandScope}` and every other root name missing from the prelude (D2-1 to D2-7); all configurations.
- **Observation (behavior):** r1-2-claude FRICTION.md:88-96 (*discover*): `Modifiers` with only the prelude gave E0422 with "a struct with a similar name exists … `Notifier`" (r1-2-claude diagnostics.md:4-19); the pilots got no help at all for `CommandScope`, `KeyChord`, `CommandKey` (triage group-A.md:97, :124). The tester resolved it from the prelude page. Throwaway experiment (toy crates in the scratch directory, rustc 1.99.1): a consumer naming a missing type got **no** `use` suggestion in every shape tried: a type glob re-exported from a transitive dependency (`pub use inner::*`, the facade's shape, crates/pert-ui/src/lib.rs:28-29), the same plus an explicit `pub use inner::Modifiers`, the consumer also depending on `inner` directly, and a type defined in the direct dependency itself (both via `pub use input::Modifiers` and a plain root `pub struct Direct`). In the same compile, `HashMap` got "consider importing … `use std::collections::HashMap`". So triage's U1 is not caused by the re-export structure: on this toolchain rustc suggests no imports from non-std dependencies at all, and no re-export layout can make it point to `pert_ui::Modifiers`.
- **Rule:** AGENTS.md "Ordinary application code uses the public facade, bindings and defaults"; rubric D1: "A finding is demonstrated friction: an item that the probes or round 1 needed, found late".
- **Severity:** low (annoyance; one compile cycle and a documentation search per missing name, recurring across most code bases). **Confidence:** high for the observation; the toolchain result is from toy crates, not from pert-ui itself.
- **Suggested fix:** the fix cannot be a re-export shape. Either membership (D2-1 to D2-7) or a pointer: the prelude page (runtime lib.rs:294-327, facade lib.rs:46-63) could list by name the root items an ordinary application most often adds (shortcuts, capture, semantics), and the rustdoc of prelude items that return or take a root-only type (`KeyboardState::modifiers`, `EventContext::modifiers`, `MenuItem::shortcut`, `CommandKeyInput::new`) already links it, so the gap is the prelude page. Not breaking.
- **Destination:** app-author slice 9 (prelude shape), with D2-1.
- Related, not re-recorded: F20 (item examples import `pert_ui_runtime::…`, r1-4-claude FRICTION.md:37-42) is path friction too, already in slice 3's table with destination S9.

### claude-D2-1 The shortcut and command family is missing although the rule includes it (slice 3's F3, extended)

- **Item and kind:** D2, project rule (the prelude's membership rustdoc).
- **Paths and configurations:** `pert_ui_runtime::{KeyChord, Modifiers, CommandKey, CommandScope}` (high confidence); `{CommandId, CommandReceipt, CommandStatus}` (high); `{CommandSource, CommandContext, CommandEntry, CommandOutcome}` (medium); `{HeldInputId, HeldCancellation}` (medium). All configurations.
- **Observation (signature):**
  - `KeyChord`: `MenuItem::shortcut(self, chord: KeyChord)` (menu.rs:499), a builder of a prelude widget; `Runtime::register_command(…, scope: CommandScope, chords: Vec<KeyChord>, …)` (commands.rs:542).
  - `Modifiers`: `KeyChord::new(key, modifiers: Modifiers)` (commands.rs:34), `Runtime::native_modifiers` (commands.rs:1572), returned by the prelude's `KeyboardState::modifiers` (input.rs:426) and `EventContext::modifiers` (runtime.rs:3168), taken by `DropOperations::preferred` and `KeyEvent::requests_context_menu` (input.rs:268). It is the only item of the root group `pub use input::{…}` (lib.rs:268) that the prelude leaves out (lib.rs:448).
  - `CommandKey`: the prelude's `CommandKeyInput`, which `Component::text_key_event` takes, has public fields `base`/`logical: Option<CommandKey>` (commands.rs:277, :284) and its only constructor is `CommandKeyInput::new(physical, action, base: CommandKey)` (commands.rs:298); a component reading the event matches on `CommandKey`, the enum "a component matches on to read its events".
  - `CommandId`, `CommandReceipt`: taken by prelude widget constructors `Button::command(ui, command: &CommandId, on_receipt: FnMut(CommandReceipt, …))` (controls.rs:549), `MenuItem::command` (menu.rs:400), `Tooltip::command` (popover.rs:435); `CommandPaletteController::receipt() -> Result<Option<CommandReceipt>, _>` (command_palette.rs:339); `CommandStatus` is what that receipt's `status()` returns (commands.rs:1612).
  - `CommandContext` is taken by prelude controllers `CommandPaletteController::open` (command_palette.rs:274) and `ContextMenuOpener::open` (menu.rs:1355), and is built from `command_context(CommandSource)`; `CommandEntry` is the parameter of the prelude's `CommandMatcher::new` closure; `CommandOutcome` that of `CommandPalette::outcome_text`.
  - `HeldInputId`/`HeldCancellation`: `Runtime::register_held_input(owner, keys, modifiers, action, cancel: FnMut(KeyCode, HeldCancellation)) -> HeldInputId`; diagram_editor stores it (app.rs:198).
  - Use: the four shortcut names in 6–8 of 10 tester code bases (triage group-A.md:68-83), in all four application probes, and in the guide's own examples (diagnostics.md:29-30, surfaces.md).
- **Rule:** runtime lib.rs:302-307: "the widgets, containers and surfaces, with the types their builders take and their controllers and status values return; … layout, theme, typography, keyboard and shape types an application configures"; lib.rs:308-310: "the [`Component`] trait together with every type its method signatures name, the enums a component matches on to read its events".
- **Severity:** low (annoyance, but the most frequent root import). **Confidence:** high, except as marked.
- **Suggested fix:** add the high-confidence names; decide the medium ones with D2-4's closure question. Additive, not breaking.
- **Destination:** app-author slice 9.

### claude-D2-2 The reorder callback types are missing (F3's second half, extended)

- **Item and kind:** D2, project rule.
- **Paths and configurations:** `pert_ui_runtime::{ListDrop, DropPosition, ListDropPoint, ListDropPlacement}`; all configurations.
- **Observation (signature):** `VirtualList::reorderable(self, on_drop: impl FnMut(&mut EventContext, &ListDrop<'_>) -> Result<bool, RuntimeError>)` (virtual_list.rs:970), likewise `Tabs::reorderable` (tabs.rs:535) and `Tree::reorderable` (tree.rs:1016); `ListDrop::position(&self) -> &DropPosition` (list_drag.rs:81), the enum the handler matches. `VirtualList::placement(self, rule: impl Fn(&[String], &ListDropPoint) -> Option<ListDropPlacement>)` (virtual_list.rs:1046). r1-2-claude and pilot-2-claude imported `ListDrop` and `DropPosition` from the root (r1-2-claude summary.md:28).
- **Rule:** lib.rs:302-303 "with the types their builders take".
- **Severity:** low. **Confidence:** high.
- **Suggested fix:** add the four names. Not breaking.
- **Destination:** app-author slice 9.

### claude-D2-3 Two widgets are outside the prelude, and their constructor type is one the rule keeps out (includes a policy question)

- **Item and kind:** D2, project rule; the `SeriesSource` part is a **policy question**.
- **Paths and configurations:** `pert_ui_runtime::{LevelMeter, Trace, MeterAxis}`; all configurations.
- **Observation (signature):** `LevelMeter` and `Trace` implement `Component` (live_widgets.rs:210, :516) and are the only root `Component` implementors outside the prelude (the other 75 runtime implementors, and the facade's `Svg`, are in it). `LevelMeter::axis(self, axis: MeterAxis)` (live_widgets.rs:163). Both constructors take `SeriesSource<f32>` (live_widgets.rs:148, :493), which the rule keeps at the root as live-data plumbing; the crate root's map lists them under "Worker results and live data" with `SeriesSource` and `series_bridge` (root.md map). live_mixer imports `LevelMeter` and `MeterAxis`; the mixer probe recorded the same gap (plans/archive/mixer-probe-improvements.md:95-99, :614-615).
- **Rule:** lib.rs:302-303 "the widgets, containers and surfaces, with the types their builders take" against lib.rs:320-322 "Inspection, profiling, capture, live-data plumbing and other host-integration types also stay at the crate root". The two clauses conflict for these widgets: the second excludes the first's builder type.
- **Severity:** low. **Confidence:** high that `LevelMeter`, `Trace` and `MeterAxis` fall under the widget clause.
- **Suggested fix:** add the three names; the maintainer decides whether `SeriesSource` follows them or the rule states that a live widget's source stays at the root. Not breaking.
- **Destination:** app-author slice 9.

### claude-D2-4 "A custom component needs no other import" is false; the rule's closure is undefined (includes a policy question)

- **Item and kind:** D2, project rule (the prelude rustdoc's own guarantee); defining the closure is a **policy question**.
- **Paths and configurations:** `pert_ui_runtime::{Semantics, SemanticRole, SemanticValue, SemanticAction, InspectValue, ValidationHints, OverlapPolicy, BoxModelInspection}`, `WorkspaceValue`, `DragPreview`, `DRAG_THRESHOLD`, `CommandKey`, `Modifiers`; all configurations.
- **Observation (signature and use):**
  - The guarantee is stated three times: runtime lib.rs:324-326 "so a custom component can be written from `use pert_ui::prelude::*` alone"; facade lib.rs:53-55 "the [`Component`] trait with every type its method signatures name, so a custom widget needs no other import"; root.md:39-40 "[`prelude`] holds the common imports, including everything a custom [`Component`] names".
  - `Component::inspect` returns the prelude's `ComponentDetails`, whose public fields are `semantics: Option<Semantics>`, `validation: ValidationHints`, `box_model: BoxModelInspection`, `properties: BTreeMap<String, InspectValue>` (diagnostics.rs:646-658). Every custom component in the probes and round 1 that describes itself imports the inspection types from the root: diagram_editor canvas.rs:866-879 (`Semantics::new(SemanticRole::Group)`), r1-3-claude picker.rs:265-271, r1-3-codex picker.rs:158-170. The custom-widgets guide asks for `Semantics` (custom-widgets.md:162) and `OverlapPolicy::Allow` in `ComponentDetails::validation` (custom-widgets.md:124).
  - `Component::workspace_state` returns `ViewState`, which takes data only through `ViewState::with(name, value: WorkspaceValue)` (workspace.rs:248); the trait's own example imports `WorkspaceValue` (runtime.rs:1872, :1885).
  - A drag source calls `EventContext::start_drag(payload: DragPayload, permitted: impl Into<DropOperations>, preview: DragPreview)` (drag.rs:132): the prelude has `DragPayload` and `DropOperations` but not `DragPreview`, nor `DRAG_THRESHOLD`, which "Custom sources should use" (drag.rs:12-15).
  - The prelude does take some one-hop types (`Notifier`, returned by `EventContext::notifier`, runtime.rs:3206; `Magnification`, the `PointerKind::Pinch` payload; `DragPayload`) and omits others of the same distance (`Modifiers`, `CommandKey`, `DragPreview`, `WorkspaceValue`, the `ComponentDetails` field types). The rule text, "every type its method signatures name", does not say whether field, variant and constructor types of those types count, so membership is not predictable from it.
- **Rule:** the guarantee quoted above, against lib.rs:320-322 ("Inspection … types also stay at the crate root"), which keeps `Semantics` and `InspectValue` out.
- **Severity:** low (an extra import, but the rustdoc promises the opposite). **Confidence:** high.
- **Suggested fix:** decide the closure (for example: types named by `Component` signatures plus the public field, variant-payload and constructor-parameter types of those types) and either add the names it yields (the list above) or narrow the three sentences to what holds. Not breaking either way.
- **Destination:** app-author slice 9.

### claude-D2-5 Parameter types the rule includes are missing; the host-delivery family is unclassified (includes a policy question)

- **Item and kind:** D2, project rule; the host-delivery part is a **policy question**.
- **Paths and configurations:** `pert_ui_core::{PowerExponent, EditOutcome, ParameterSnapshot, PreviewOutcome}` (rule-included); `pert_ui_runtime::{DeliveryMonitor, DeliverySnapshot, TerminalEvent, PreviewEvent, SinkError, SinkErrorKind, DeliveryResolution, DeliveryStatus}`, `pert_ui_core::{EditToken, ParameterValue}` (unclassified); all configurations.
- **Observation (signature and use):**
  - `PowerExponent` is the payload of the prelude's `ParameterMapping::Power(PowerExponent)` (pert-ui-core parameter.rs:37), the only way to build a power mapping; live_mixer.rs:359 and gallery controls.rs:553 import it.
  - `EditOutcome` is returned by the prelude's `EditSession::commit` and `cancel` (parameter_binding.rs:797, :868); the guide's local-gesture example writes `use pert_ui::{prelude::*, EditOutcome};` (guide/state.md:96), and state.md:140 and :197 import it too; live_mixer and gallery import it. `PreviewOutcome` is what `EditSession::update` returns (parameter_binding.rs:743). `ParameterSnapshot` is what `Parameter::snapshot` returns as `ReadSignal<ParameterSnapshot<T>>` (parameter_binding.rs:316); gallery main.rs:183-184 stores it.
  - The prelude's `ParameterHost::on_outcome` returns `DeliveryMonitor<T>` and takes a sink returning `Result<(), SinkError>` over `&TerminalEvent<T>` (delivery.rs:500); `on_preview` likewise with `PreviewEvent`; `resolve_delivery(&EditToken, DeliveryResolution)`. live_mixer imports `DeliveryMonitor`, `DeliverySnapshot`, `SinkError`, `SinkErrorKind` and writes `pert_ui::EditToken` and `pert_ui::ParameterValue` inline (live_mixer.rs:276-294, :372); dynamic_documents uses `SinkError`, `SinkErrorKind` through a glob.
- **Rule:** lib.rs:304-305 "state and parameters: [`Runtime`], signals, parameter specifications, bindings and hosts". `PowerExponent` is part of a specification and `EditOutcome`/`PreviewOutcome`/`ParameterSnapshot` are what a binding returns. Whether a host's sink, event and monitor types are "hosts" or "host-integration types" (lib.rs:320-322) the rule does not say.
- **Severity:** low. **Confidence:** high for `PowerExponent` and `EditOutcome`, medium for `PreviewOutcome` and `ParameterSnapshot`; the host family is unresolved.
- **Suggested fix:** add the four rule-included names; decide the host family. Not breaking.
- **Destination:** app-author slice 9.

### claude-D2-6 Builder, controller, theme and drawing types the prelude's own members require

- **Item and kind:** D2, project rule.
- **Paths and configurations (all configurations):**
  - `ViewScale` (high): `ScaleLimits::new(a: ViewScale, b: ViewScale)` (zoom_view.rs:33) is the constructor of the prelude's `ZoomView::scale_limits(ScaleLimits)` argument (zoom_view.rs:355); `ViewTransform::new(scale: ViewScale, offset)` (transform.rs:347) builds what `ZoomController::set` takes and `Component::child_transform` returns. diagram_editor imports it (editor.rs:10, app.rs:15).
  - `QuarterTurn` (high): `Label::turned(self, turn: QuarterTurn)` (controls.rs:186), a prelude widget builder; also `Painter::icon_turned`, `draw_mask_turned`, `draw_image_turned`. Gallery transforms.rs:363-438.
  - `OpaqueColor` (high): `Palette::from_seeds(mode, primary: OpaqueColor, secondary: Option<OpaqueColor>)` (theme.rs:838), theme configuration; guide/appearance.md:96 and :145 import it; gallery screenshot.rs:15 and pages/tools.rs:517.
  - `MaskImage`, `ImageId`, `MaskId`, `VectorId`, `VectorGeometry`, `TextLayout` (medium): `Painter::draw_mask(&mut self, image: &MaskImage, …)` (drawing.rs:1487); `DisplayList::masks() -> &[MaskImage]`, `vectors()`, `image(ImageId)`; `DrawCommand::DrawImage { image: ImageId }`, `DrawMask { mask: MaskId }`, `DrawVector { vector: VectorId }`; `Painter::text` returns `TextLayout`. Gallery's test reads `MaskImage` back from a `DisplayList` (tests/text_rendering.rs:436).
  - `ListingErrorKind`, `LinkTarget` (medium): the prelude's `ListingError` (brought in "for widget configuration", lib.rs:317-318) has the single constructor `ListingError::new(kind: ListingErrorKind, message)` (file_browser.rs:83) besides `From<io::Error>`, and the prelude's `EntryKind::Symlink(LinkTarget)`.
  - `EditorStart`, `EditorSession` (medium): the prelude's `CellEdit::start() -> &EditorStart` and `session() -> &EditorSession` (table/editing.rs:52, :59); `TextField::session(&EditorSession)` (text_fields.rs:805) and the number inputs' and `Select`'s `session` builders.
  - `FilterCounts`, `FilterOutcome` (medium): returned by the prelude's `ListFilter::counts`/`filter` (collection_filter.rs:308, :318) and `TableController`/`TreeController::filter_counts`/`filter` (table.rs:707, :730; tree.rs:468, :515).
  - `WorkspaceSettings` (medium): `Workspace::new(settings: WorkspaceSettings, …)` (workspace.rs:518), the workspace document the rule names.
- **Observation:** signature (each listed signature names the type).
- **Rule:** lib.rs:302-307 "with the types their builders take and their controllers and status values return"; "layout, theme, typography, keyboard and shape types an application configures"; lib.rs:309-311 "the drawing types [`Painter`] records and a test reads back".
- **Severity:** low (`ViewScale` and `OpaqueColor` were needed by a probe or the guide; the rest only by the gallery's root glob). **Confidence:** as marked.
- **Suggested fix:** add the high-confidence names; settle the medium ones with D2-4's closure. Not breaking.
- **Destination:** app-author slice 9.

### claude-D2-7 The facade prelude's "App and what App reports" omits two reported types and misstates its errors

- **Item and kind:** D2, project rule (facade prelude rustdoc).
- **Paths and configurations:** `pert_ui::{NativeWindowId, WindowError}` (runtime types, re-exported) relative to `pert_ui::prelude`; configurations with `native`: default, cpu, serde, images, image-formats, svg, no-default-desktop-cpu.
- **Observation (signature):** `App::on_close(self, handler: impl FnMut(&mut Runtime, &NativeWindowId) -> Result<NativeCloseAction, RuntimeError>)` (app.rs:465) and `App::on_window_error(self, handler: impl FnMut(&mut Runtime, &WindowError))` (app.rs:480): the prelude has `NativeCloseAction` and `ErrorAction` (what the handlers return) but not what `App` reports to them. r1-1-claude imported `NativeWindowId` from the root beside `NativeCloseAction`. In the reverse direction, `AppShutdownError` (an `Error`, api/pert-ui/default.txt:104) is in the prelude, while the facade rustdoc's error list says "alongside `AppError` with `desktop`. Other errors … stay at the crate root" (lib.rs:58-60).
- **Rule:** runtime lib.rs:324-325 "adds only its native startup types (`App` and what `App` reports)"; facade lib.rs:51 "`App` and the types it reports".
- **Severity:** low. **Confidence:** high for `NativeWindowId` and the wording; medium for `WindowError` (an error type, which the error sentence would keep out).
- **Suggested fix:** add `NativeWindowId` (and `WindowError`, or say why not) to the facade prelude, and name `AppShutdownError` in the error sentence. Not breaking.
- **Destination:** app-author slice 9.

### claude-D2-8 Names the rule keeps at the root that most applications import (policy question)

- **Item and kind:** D2, **policy question** (not a departure from the rule).
- **Paths and configurations:** `CaptureRequest`, `CaptureTarget` (7 of 8 round 1 code bases, diagram_editor, gallery), `pert_ui::cpu::CpuRenderer` (7 code bases; `cpu` feature), the semantics types (see D2-4; 6–7 code bases), `ApplicationError` (`pert_ui::ApplicationError::new(…)` in every application probe's main.rs: diagram_editor main.rs:67, inventory main.rs:51, log_explorer main.rs:53, live_mixer.rs:788; inventory app.rs:40), `Mailbox`/`UiSender`/`Capacity` (three probes; guide/state.md:197).
- **Observation:** the rule excludes them explicitly (lib.rs:318-322), and they are consistent with it. Their use is driven by testing (capture, semantics) and by host failures (`ApplicationError`, which `AppError`'s rustdoc names as the way to wrap a foreign error, app.rs:7-11).
- **Rule:** lib.rs:318-322.
- **Severity:** low. **Confidence:** high on the counts.
- **Suggested fix:** none required; input to slice 9's decision on the prelude's shape, for example a testing-oriented module or prelude, as the testing chapter (F2) is planned.
- **Destination:** app-author slice 9.

No reverse-direction departures were found among the 288 runtime prelude names: every member is a widget, container or surface (75 of the 77 runtime `Component` implementors are in it, and the facade adds `Svg`), a builder, controller or status type of one, a state, parameter, binding or workspace type, a layout, theme, typography, keyboard or shape type, a type named by a `Component` signature, an event enum or payload (`Magnification`), a drawing type, or one of the seven errors the rustdoc lists (lib.rs:313-318). One-hop members such as `Notifier` are counted under D2-4's closure question, not as departures.

## Inventory gaps

1. **Prelude membership is not inventoried.** No candidate list says which names the prelude holds; `inventory.py` resolves each item to its shortest path, "the root path rather than the prelude's when both exist" (inventory.py:338-341), and types.txt carries no prelude column. D2's membership had to be parsed by hand from lib.rs:328-470. Suggested: a `prelude.txt` (name, defining path, configurations).
2. **Free functions and free constants are missing from every list unless they return `Result`.** `pert_ui_core::series_bridge` (imported by live_mixer, named in root.md's map), `pert_ui_runtime::DRAG_THRESHOLD: f32` (drag.rs:15) and `pert_ui::svg::MAX_NESTING: usize` (api/pert-ui/svg.txt:167) appear in no candidate or semantic list (`grep` over inputs/). `list_directory`, `check_redescription`, `redescribe` and the `svg`/`images` functions appear only in errors.txt (and numeric.txt for `svg::rasterize*`). Besides D2, the two constants are candidates for N3 (a bare `f32`/`usize` with a unit or limit) and C4 (a documented limit as a free constant rather than an associated one), which the semantic lists, seeded with types, cannot reach.
3. **Enum-variant payloads and public fields are not in the semantic lists.** The semantic lists hold methods only; the closure types that reach prelude types through fields (`ComponentDetails.semantics`, `CommandKeyInput.base`) or variants (`ParameterMapping::Power(PowerExponent)`, `EntryKind::Symlink(LinkTarget)`, `DrawCommand::DrawMask { mask: MaskId }`) had to come from fields.txt and the listings. Not wrong, but D2's closure needs all three sources.

## Roles relied on

| path | role | source reference |
|---|---|---|
| `LevelMeter`, `Trace` | widget (implements `Component`) | live_widgets.rs:210, :516 |
| `MeterAxis` | builder argument of a widget | live_widgets.rs:163 |
| `SeriesSource` | live-data plumbing (root by rule) | root.md map row "Worker results and live data"; live.rs:627 |
| `KeyChord`, `Modifiers`, `CommandScope` | keyboard configuration value | commands.rs:27-60, :542; menu.rs:499; input.rs:285 |
| `CommandKey` | event enum read by a component; field/constructor type of `CommandKeyInput` | commands.rs:7-13, :277-298 |
| `CommandKeyInput` | type named by a `Component` signature | runtime.rs (Component::text_key_event) |
| `CommandId` | handle taken by widget constructors | controls.rs:549; menu.rs:400; popover.rs:435 |
| `CommandReceipt` / `CommandStatus` | receipt / its status | commands.rs:1607, :1612; command_palette.rs:339 |
| `CommandContext`, `CommandEntry`, `CommandOutcome`, `CommandSource` | values taken by prelude controllers or builder closures | command_palette.rs:274, :294, :582; menu.rs:1355 |
| `HeldInputId` | handle of a registered held-key action | runtime/held.rs:8 |
| `ListDrop`, `DropPosition`, `ListDropPoint`, `ListDropPlacement` | builder-callback payloads of widgets | virtual_list.rs:970, :1046; list_drag.rs:47, :81 |
| `ComponentDetails` | type named by `Component::inspect`; plain data with public fields | diagnostics.rs:646-658 |
| `Semantics`, `SemanticRole`, `SemanticValue`, `SemanticAction`, `InspectValue`, `ValidationHints`, `OverlapPolicy`, `BoxModelInspection` | inspection data (root by rule) | diagnostics.rs; validation.rs |
| `ViewState` / `WorkspaceValue` | workspace document entry / its value | workspace.rs:236-334, :39 |
| `DragPreview` | value taken by `EventContext::start_drag` | drag.rs:191, runtime/drag.rs:132 |
| `PowerExponent` | validated value inside a parameter specification | pert-ui-core parameter.rs:37, :63 |
| `EditOutcome`, `PreviewOutcome`, `ParameterSnapshot` | outcomes and state a binding returns | parameter_binding.rs:316, :743, :797 |
| `DeliveryMonitor`, `TerminalEvent`, `PreviewEvent`, `SinkError`, `SinkErrorKind`, `DeliveryResolution`, `EditToken`, `ParameterValue` | unresolved: "bindings and hosts" or "host-integration"; the rule does not decide | delivery.rs:500, :537, :622 |
| `ViewScale` | validated value with an invariant; argument of `ScaleLimits::new` and `ViewTransform::new` | transform.rs:267-302; zoom_view.rs:33 |
| `ScaleLimits` | builder argument of `ZoomView` (prelude) | zoom_view.rs:355 |
| `OpaqueColor` | theme seed value | theme.rs:838 |
| `QuarterTurn` | builder argument of `Label`, drawing parameter | controls.rs:186 |
| `MaskImage`, `ImageId`, `MaskId`, `VectorId`, `VectorGeometry`, `TextLayout` | drawing resources recorded by `Painter` and read back from `DisplayList` | drawing.rs:1096-1262, :1487 |
| `ListingErrorKind`, `LinkTarget` | file-browser configuration values | file_browser.rs:42, :58, :83 |
| `EditorStart`, `EditorSession` | editor-session values returned by `CellEdit` / taken by editor builders | table/editing.rs:52, :59; text_fields.rs:805 |
| `FilterCounts`, `FilterOutcome` | controller return values | collection_filter.rs:308, :318 |
| `NativeWindowId` | identity `App::on_close` reports | app.rs:465 |
| `WindowError` | error `App::on_window_error` reports | app.rs:480 |
| `Notifier`, `Magnification`, `DragPayload` | one-hop types the prelude includes | runtime.rs:3206; pointer.rs:116; drag.rs:45 |
| `ApplicationError` | error carrier for host failures (root by rule) | error.rs:46; app.rs:7-11 |
| `CaptureRequest`, `CaptureTarget`, `FrameImage`, `InspectionQuery`, `Selector`, `DebugMode`, `ValidationCode` | capture, inspection and diagnostics (root by rule) | capture.rs; inspection.rs; diagnostics.rs; validation.rs |
| `Mailbox`, `UiSender`, `SendError`, `Capacity`, `Delivery`, `SeriesConfig`, `SeriesEndpoint`, `Timestamp`, `series_bridge` | live-data plumbing (root by rule) | mailbox.rs; live.rs; pert-ui-core live/ |
| `Clipboard`, `ClipboardError`, `PresentationAction`, `PresentationStatus`, `list_directory` | host integration (root by rule) | text_input.rs:192; native_windows.rs:919, :984; file_browser.rs:273-274 |
| `ClickCounter` | component helper kept out by a recorded decision | plans/archive/diagram-probe-improvements.md:433-434 |
| `ContentQuery` | runtime search request (not a widget, state or `Component` type) | content_search.rs:22 |
