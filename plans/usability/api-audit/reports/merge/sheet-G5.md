# Merge sheet: G5

Inputs: `plans/usability/api-audit/reports/{codex,claude}-G5.md` and
`reports/verification/{codex,claude}-G5.md`, at HEAD 245053b (`crates/`
unchanged since the audited commit d4ba108). Prefixes: `R/` =
`crates/pert-ui-runtime/src/`, `C/` = `crates/pert-ui-core/src/`, `F/` =
`crates/pert-ui/src/`, `U/` = `plans/usability/runs/`.

The two verifications were written at the same time, so neither auditor's
"My findings disputed" section answers the other's verification of its own
findings. Where one side's verification corrected the other and the other
did not respond, the correction is applied and noted.

Abbreviations in the Destination column: S9 = app-author slice 9, FU =
follow-ups, TODO = TODO.md. "All configurations" means runtime and core
`default` and `serde`, and all 11 facade configurations. "Native" means the
facade configurations with `native`: default, cpu, serde, images,
image-formats, svg, no-default-desktop-cpu.

## Findings

| # | Rubric item, rule kind | Finding (one or two sentences, naming the items) | Sig/beh | Configs | Severity | Confidence | Fix (breaking?) | Destination | Sources and status | Slice 3 |
|---|---|---|---|---|---|---|---|---|---|---|
| G5-1 | D1, project rule | The facade inlines `pert_ui_runtime::*` (`F/lib.rs:28-29`), so item pages such as `Runtime`, `Button`, `TextField` and `Semantics` show examples importing `pert_ui_runtime` (`R/runtime/tree.rs:410`, `R/controls.rs:533`, `R/signal.rs:319`); the substitution is explained only on the root page (`F/root.md:42-46`). Friction: `U/r1-4-claude/FRICTION.md:37-42`. | sig | All exposing the items | low | high | Show the facade spelling on inlined item pages, or a local pointer, keeping defining-crate doctests buildable. Not breaking. | S9 | codex-D1-1, **confirmed** (Claude's report named it as F20 and did not re-record it). Claude proposes merging it into F20 rather than recording it as new. | F20 (duplicate) |
| G5-2 | D1, project rule | Starting background work from a callback: `EventContext::defer` (`R/runtime/tasks.rs:96-99`) does not point to `Runtime::spawn`, and the guide's worker section teaches only the mailbox route (`F/guide/state.md:272-277`). r1-1-claude found `spawn` by grepping and composed the bridge itself (`U/r1-1-claude/FRICTION.md:36-41`). | sig (as Codex records it) | All | medium | high | Cross-link `defer` and `spawn`; a complete facade example in the guide. Not breaking. | TODO (cross-links), S9 (walkthrough), FU (convenience API) | codex-D1-2 row 1, **confirmed**. | F13 |
| G5-3 | D1, project rule | Per-row editable bindings for application records: `Signal::sibling` is documented as popup state (`R/signal.rs:306-311`) and `ListItems::build` gets no state owner (`R/virtual_list.rs:50`); r1-2-claude spent about 25 minutes composing it (`U/r1-2-claude/FRICTION.md:38-46`, `:58-66`). The retained alternative exists in `examples/inventory/app.rs:107-110`. | sig (as Codex records it) | All | medium | high | Cross-links and a facade example of the composition, explaining the retained-row alternative. Not breaking. | TODO, S9 (recipe) | codex-D1-2 row 2, **confirmed**. Claude: the missing owner in `build` is a G1/C3 signature matter, not D1. | F14, F16 |
| G5-4 | D1, project rule | The `DraftEditor` owner contract (`R/text_fields/draft.rs:437-`) never names `Component::retain_initial_children` or redescription; r1-2-codex found it only after a failing test (`U/r1-2-codex/FRICTION.md:52-58`). The rule is at `R/runtime.rs:1722-1730`. | sig (as Codex records it) | All | medium | high | Cross-link from the owner contract; do not change reconciliation. Not breaking. | TODO | codex-D1-2 row 3, **confirmed**. Claude notes the log files it under *runtime*, but the fix is a cross-link, so D1 applies. | F17 |
| G5-5 | D1, project rule | `Runtime::text_command`'s summary calls it "a clipboard action" (`R/runtime.rs:5096`) although `TextCommand::SelectAll` touches no clipboard (`R/text_input.rs:132-134`); r1-1-claude found `SelectAll` only on the `Runtime` page (`U/r1-1-claude/FRICTION.md:79-82`). | sig (as Codex records it) | All | Codex medium (as part of D1-2), Claude low. **Propose low:** one side remark in one log; the fix is one summary line. | high | Reword the summary at `R/runtime.rs:5096`. Not breaking. | TODO | codex-D1-2 row 5, **partly**: the r1-4 Ctrl+A helpers are refuted as evidence (see below). | F2 |
| G5-6 | D1, project rule | A facade consumer gets no compiler route to root-only names: `Modifiers` with only the prelude gives E0422 suggesting `Notifier` (`U/r1-2-claude/diagnostics.md:4-19`, `FRICTION.md:88-96`), and `CommandScope`, `KeyChord`, `CommandKey` get no help (triage group-A.md:30). | beh | All | low | high for the observation | Membership (G5-7 to G5-24) or a pointer: the prelude page lists the root items ordinary applications most often add. Not breaking. | S9 | claude-D1-1, **partly**: friction confirmed; the claim that rustc suggests no imports from non-std dependencies, so no re-export layout can help, is unverified (Disputed facts 3). | F3 |
| G5-7 | D2, project rule | The shortcut and command family is missing from the prelude although prelude signatures name it (keyboard configuration, `R/lib.rs:306`; builder/controller types, `:302`): `Modifiers`, `KeyChord`, `CommandKey`, `CommandScope`, `CommandId`, `CommandReceipt`, `CommandStatus`, `CommandSource`, `CommandContext`, `CommandEntry`, `CommandOutcome` (e.g. `MenuItem::shortcut`, `R/menu.rs:499`; `Button::command`, `R/controls.rs:549`; `CommandKeyInput` fields, `R/runtime/commands.rs:277-298`). | sig | All | low | high (Claude: medium for `CommandSource`, `CommandContext`, `CommandEntry`, `CommandOutcome`) | Add to the runtime prelude. Additive; check downstream glob ambiguity. | S9 (Claude, F3's route); Codex: TODO for membership, S9 for organization | claude-D2-1, codex-D2-1, **both** (`Modifiers`, `CommandKey` also in claude-D2-4). | F3 (extends) |
| G5-8 | D2, project rule | `HeldInputId` and `HeldCancellation`, named by `Runtime::register_held_input` (`R/runtime/held.rs:51`, `:71-83`), are keyboard configuration an application registers but are outside the prelude (root at `R/lib.rs:479`). | sig | All | low | medium (Claude), high (Codex after correction) | Add both. Additive. | S9 / TODO (as G5-7) | claude-D2-1, **confirmed**; Codex withdrew its host-plumbing classification. Claude still calls the classification a maintainer question, since a `Runtime` method alone does not decide membership. | F3 (extends) |
| G5-9 | D2, project rule | Reorder callback types that widget builders take are missing: `ListDrop`, `DropPosition`, `ListDropPoint`, `ListDropPlacement`, `ListDropMark` (`VirtualList::reorderable`/`placement`, `R/virtual_list.rs:970`, `:1046`; `Tabs`, `Tree` likewise; `ListDropPlacement::new(position, mark)`, `:570`; `ListDrop::position`, `R/list_drag.rs:81`). | sig | All | low | high | Add the five names. Additive. | S9 / TODO | claude-D2-2, codex-D2-1, **both**; `ListDropMark` from Codex, confirmed by Claude, who added it to D2-2. | F3 (extends) |
| G5-10 | D2, project rule | `LevelMeter` and `Trace` are widgets (`R/live_widgets.rs:131`, `:481`; the only root `Component` implementors outside the prelude, per Claude) and `MeterAxis` is a builder argument (`:163`), all outside the prelude. | sig | All | low | high | Add the three names. Additive. | S9 / TODO | claude-D2-3, codex-D2-1, **both**. | - |
| G5-11 | D2, policy | **Question:** should `SeriesSource`, which both live widget constructors take (`R/live_widgets.rs:148`, `:493`), follow them into the prelude, or should the rule say a live widget's source stays at the root under the live-data exclusion (`R/lib.rs:320-322`)? | sig | All | low | high | Either add `SeriesSource` or state the exception. Not breaking. | S9 | claude-D2-3, **policy**; Codex: the existing exclusion explains it, changing it is policy. | - |
| G5-12 | D2, project rule | Typed parameter and binding values are missing although the rule includes parameter specifications and bindings (`R/lib.rs:304-305`): `PowerExponent` (payload of `ParameterMapping::Power`, `C/parameter.rs:37`), `EditOutcome`, `PreviewOutcome`, `ParameterSnapshot` (`R/parameter_binding.rs:316`, `:743`, `:797`), `EditToken` (`EditSession::token`, `:730`), `ParameterValue` (bound of `Parameter<T>`, `:283`), `Revision` (argument of `Parameter::begin_edit`, `:351`), `ParameterFamily` (returned by `ParameterSpec::family`, `C/parameter.rs:210`). | sig | All | low | high (Claude: medium for `PreviewOutcome`, `ParameterSnapshot`) | Add to the runtime prelude's core re-exports. Additive. | S9 / TODO | claude-D2-5, codex-D2-1, **both** for the first four; `EditToken`, `ParameterValue`, `Revision`, `ParameterFamily` **confirmed** (Claude withdrew its "unclassified" for the first two). Claude's summary also lists `ParameterFamily` as one-hop; `ParameterSpec::family` names it directly. | - |
| G5-13 | D2, project rule | The erased parameter forms are missing: `AnyParameter`, `AnyParameterHost` (`R/parameter_binding/erased.rs:15`, `:58`, `:255`), `AnySpec`, `AnyValue` (`C/erased.rs:24`, `:112`). | sig | All | low | high | Add the four names. Additive. | S9 / TODO | codex-D2-1, **confirmed**. | - |
| G5-14 | D2, project rule | `EditSource`, the type of the field `ParameterSnapshot::edit_source` (`C/edit.rs:118`) and the return of `EditToken::source` (`:96`), is outside the prelude. | sig | All | low | high | Add it. Additive. | S9 / TODO | codex-D2-d1, **delta-unverified**. | - |
| G5-15 | D2, project rule | Status values the prelude's `Parameter` returns are missing: `DeliveryStatus` (`Parameter::delivery_status`, `R/parameter_binding.rs:327`) and `AuthorityStatus` (`Parameter::authority_status`, `R/parameter_binding/remote.rs:686`). | sig | All | low | high for `DeliveryStatus`, medium for `AuthorityStatus` | Add both, or say remote-authority status stays with the host types. Additive. | S9 | claude-D2-d2, **delta-unverified**. Claude moved `DeliveryStatus` out of its D2-5 host family. | - |
| G5-16 | D2, policy | **Question:** are `ParameterHost`'s sink, event and monitor types (`DeliveryMonitor`, `DeliverySnapshot`, `TerminalEvent`, `PreviewEvent`, `SinkError`, `SinkErrorKind`, `DeliveryResolution`; `R/parameter_binding/delivery.rs:500`) "hosts" (`R/lib.rs:304-305`) or "host-integration types" (`:320-322`)? | sig | All | low | high on the signatures | Classify in the rule; add or keep out. Not breaking. | S9 | claude-D2-5 (host part), **policy**; Codex treats them as excluded host integration (`delivery.rs:48`, `:238`), a revision being policy. | - |
| G5-17 | D2, project rule | Layout, shape and theme values that prelude builders take are missing: `ViewScale` (`ScaleLimits::new`, `R/zoom_view.rs:33`; `ViewTransform::new`), `QuarterTurn` (`Label::turned`, `R/controls.rs:186`; `Painter::*_turned`), `OpaqueColor` (`Palette::from_seeds`, `R/runtime/theme.rs:838`; guide/appearance.md:96, :145). | sig | All | low | high | Add the three names. Additive. | S9 / TODO | claude-D2-6, codex-D2-1, **both**. | - |
| G5-18 | D2, project rule | Drawing types `Painter` records and a test reads back are missing: `MaskImage` (`Painter::draw_mask`, `R/drawing.rs:1487`), `ImageId`, `MaskId`, `VectorId` (`DrawCommand` payloads), `VectorGeometry` (`DisplayList::vectors`), `TextLayout` (`Painter::text`). | sig | All | low | Claude medium, Codex high | Add the six names. Additive. | S9 / TODO | claude-D2-6, codex-D2-1, **both**. | - |
| G5-19 | D2, project rule | Typography managers are missing while the analogous `ThemeManager` is in the prelude (`R/lib.rs:378`): `FontManager`, `IconManager` (`Runtime::fonts`/`icons`, `R/runtime/fonts.rs:226`, `:232`) and `IconGlyph` (payload of the prelude's `IconSource::Custom`, `R/text/icons.rs:241`). | sig | All | low | high | Add the three names. Additive. | S9 / TODO | codex-D2-1, **confirmed** (Claude added the `IconSource::Custom` evidence). | - |
| G5-20 | D2, project rule | Collection filter results and configuration are missing: `FilterCounts`, `FilterOutcome` (`ListFilter::counts`/`filter`, `R/collection_filter.rs:308`, `:318`; `TableController`/`TreeController::filter*`), and `FilterTarget` (`CollectionFilter::new`/`with_query` take `impl Into<FilterTarget>`, `:531`, `:540`). | sig | All | low; Claude: negligible for `FilterTarget` (callers never name it, no corpus use). **Propose low**, `FilterTarget` last. | Claude medium, Codex high | Add the names. Additive. | S9 / TODO | claude-D2-6, codex-D2-1, **both** for `FilterCounts`, `FilterOutcome`; `FilterTarget` **confirmed** (letter of the rule only). | F7b (related) |
| G5-21 | D2, project rule | Editor-session values that prelude items take or return are missing: `EditorSession` (`TextField::session`, `R/text_fields.rs:805`; number inputs, `Select`), `EditorStart` (`CellEdit::start`, `R/table/editing.rs:52`). | sig | All | low | Claude medium, Codex high | Add both. Additive. | S9 / TODO | claude-D2-6, codex-D2-1, **both**. | - |
| G5-22 | D2, project rule | Configuration of the prelude's file-browser types is missing: `ListingErrorKind` (the only constructor `ListingError::new(kind, message)`, `R/file_browser.rs:83`) and `LinkTarget` (`EntryKind::Symlink(LinkTarget)`). | sig | All | low | Claude medium, Codex high | Add both. Additive. | S9 / TODO | claude-D2-6, codex-D2-1, **both**. | - |
| G5-23 | D2, project rule | Workspace-document values are missing: `WorkspaceSettings` (`Workspace::new`, `R/workspace.rs:518`), `WorkspaceValue` (`ViewState::with`, `:248`; the trait's own example imports it, `R/runtime.rs:1872`), `IgnoredEntry` (item of `WorkspaceReport::ignored`, `:763`). | sig | All | low | high (Claude medium for `WorkspaceSettings`) | Add the three names. Additive. | S9 / TODO | claude-D2-6, claude-D2-4, codex-D2-1, **both** for `WorkspaceSettings`, `WorkspaceValue`; `IgnoredEntry` **confirmed** by Claude as one hop, to which the closure (G5-26) applies. | - |
| G5-24 | D2, project rule | `WindowPlacement`, the closure parameter of the prelude builder `FloatingWindow::on_frame` (`R/windows.rs:393-397`), is outside the prelude. | sig | All | low | high | Add it. Additive. | S9 / TODO | codex-D2-1, **confirmed** (no corpus use). | - |
| G5-25 | D2, project rule | The prelude promises three times that a custom component needs no other import (`R/lib.rs:324-326`, `F/lib.rs:53-55`, `F/root.md:39-40`), but `Component::inspect`'s `ComponentDetails` has public fields of root-only inspection types (`R/diagnostics.rs:646-658`) and `EventContext::start_drag` needs `DragPreview` (`R/runtime/drag.rs:132-137`); every self-describing custom component in the corpus imports them from the root. | sig and use | All | low | high | Narrow the three sentences to what holds, or add the names G5-26 decides. Not breaking. | S9 | claude-D2-4 (documentation part), **confirmed**: Codex accepted that the unconditional wording overpromises. | - |
| G5-26 | D2, policy | **Question:** does "every type its method signatures name" include field, variant-payload, constructor-parameter and one-hop types, and does the custom-component guarantee override the inspection exclusion? Instances: `Semantics`, `SemanticRole`, `SemanticValue`, `SemanticAction`, `InspectValue`, `ValidationHints`, `OverlapPolicy`, `BoxModelInspection`, `DragPreview`, `DRAG_THRESHOLD` (claude-D2-4); `EditorAttachment`, `EditorCompletion`, `EditorTrigger`, `EditorOutcome`, `Transform2D`, `FittedText`, `PathElement`, `VectorStyle` (codex-D2-1). The prelude already holds some one-hop types (`Notifier`, `Magnification`, `DragPayload`). | sig | All | low | high on the signatures | Define the closure, then add what it yields or narrow G5-25's sentences. Not breaking. | S9 | claude-D2-4 (closure), codex-D2-1 (these instances, **partly**: facts hold, membership depends on the closure), **policy**. Codex keeps the inspection types excluded (`R/lib.rs:320`). | - |
| G5-27 | D2, policy | **Question (extends G5-26):** one-hop types of `EventContext`, `MeasureContext` and `ChildInfo`: `TaskReceipt`, `TaskStatus` (`R/runtime/tasks.rs:121`), `RevealReceipt` (`R/runtime/reveal.rs:260`), `FittedText` (`R/layout.rs:842`), `TextLayout` (`R/runtime.rs:2924`), `PresentationReceipt`, `NativeWindowOptions`, `NativeWindowId` (`R/runtime/native_windows.rs:1289-1417`); two hops: `ShortcutBinding`, `ShortcutScope` (`R/validation.rs:102`); `EventContext::profile -> ProfileSnapshot` conflicts with the profiling exclusion. | sig | All | low | high on the signatures | As G5-26. Not breaking. | S9 | claude-D2-d3, **delta-unverified**, policy. | - |
| G5-28 | D2, project rule | The facade prelude omits `NativeWindowId`, which `App::on_close` reports (`F/app.rs:465-469`), against "`App` and the types it reports" (`F/lib.rs:50-51`). | sig | Native | low | high | Add it to the facade prelude under `native`. Additive. | S9 / TODO | claude-D2-7, **confirmed**; Codex corrected its blanket host-plumbing exclusion for native configurations. | - |
| G5-29 | D2, project rule | The facade prelude's error sentence names only `AppError` (`F/lib.rs:58-60`) although `AppShutdownError` is a member (`:71`); both auditors keep the membership (payload of `AppError::Shutdown`, `F/app.rs:75`). | sig | Native | low | high | Name `AppShutdownError` in the sentence. Not breaking. | S9 | claude-D2-7 (wording part), **confirmed**. | - |
| G5-30 | D2, policy | **Question:** should `WindowError`, which `App::on_window_error` reports (`F/app.rs:480`), be in the facade prelude under "`App` and the types it reports", or stay at the root under "Other errors … stay at the crate root" (`F/lib.rs:58-60`)? | sig | Native | low | medium | Add it or state the exception. Not breaking. | S9 | claude-D2-7 (`WindowError` part), **policy**; Codex: absence factual, membership needs the conflicting clauses resolved. | - |
| G5-31 | D2, project rule | The facade prelude's rustdoc says its native members come "with the `desktop` feature" (`F/lib.rs:50-51`, `:59`), but the re-export is gated on `native` (`F/lib.rs:69-71`), which `desktop-cpu` enables without `desktop` (`crates/pert-ui/Cargo.toml:21`). | sig | no-default-desktop-cpu (understated); text in all | low | high | Say "with `native` (enabled by `desktop` and `desktop-cpu`)". Not breaking. | S9 (with G5-29) | claude-D2-d1, **delta-unverified**. | - |
| G5-32 | D2, policy | **Question:** names the rule keeps at the root that most applications import, driven by testing and host failures: `CaptureRequest`, `CaptureTarget`, `cpu::CpuRenderer` (six round 1 submissions), the semantics types, `ApplicationError` (every application probe's `main.rs`), `Mailbox`, `UiSender`, `Capacity`. Should slice 9 add a testing-oriented module or prelude? Not a departure. | use | All (`CpuRenderer`: `cpu`) | low | high on the corrected counts | None required; input to slice 9's prelude shape. | S9 | claude-D2-8, **policy**; Codex corrected the count from seven to six (r1-1-codex only glob-imports the root). The sheet author's check agrees: no capture or CPU name in `U/r1-1-codex` or `U/r1-2-codex` submissions. | F2 (related) |

Inventory gaps reported by both auditors (`App::software_rendering` missing
from the version 1 semantic list; prelude membership, free items and
field/variant lines not inventoried) are not findings. Both verifications
record that version 2 closes them. Claude notes one residual caveat: the six core
types the runtime prelude re-exports read "prelude: no" in `types.txt`'s
core section.

## Disputed facts

### 1. codex-D1-2 row 4: was the UI-thread `Runtime::query` with `Selector::id` "needed, found late" in round 1?

- **Claim (Codex):** the reference pages teach a full-snapshot lookup from
  `inspection_id` (`R/runtime/tree.rs:396`) and the guide shows queries only
  through the cross-thread service (`F/guide/diagnostics.md:87`). Both task 4
  helpers follow the snapshot route (`U/r1-4-codex/submission/tests/ui.rs:35`,
  `U/r1-4-claude/submission/tests/support/mod.rs:44`).
- **Counter (Claude):** neither r1-4 log records friction here.
  `U/r1-4-claude/FRICTION.md:57` calls the `inspect().components` dump "the
  most useful thing I did", `:100` lists it under "What worked well", and
  `U/r1-4-codex/FRICTION.md:46-47` resolved lookup through `inspection_id`
  without complaint. D1 requires demonstrated friction.
- **What the source shows:** both sides' citations hold. The `inspection_id`
  example does teach `ui.inspect().components.iter().find(...)`
  (`R/runtime/tree.rs:408-417`), `Runtime::query` documents the id selector
  as "one lookup" (`R/runtime/inspect.rs:206-219`), and the guide paragraph is
  about the `InspectionService` client (`F/guide/diagnostics.md:87-93`).
  Both helpers do a linear search over `inspect().components`
  (`ui.rs:35-42`, `support/mod.rs:44-52`). No friction entry names `query`.
  Slice 3's triage records the gap as an author note (AN-1,
  `U/triage/group-E.md:51`) and under "Library observations no tester
  reported" (U1, `group-E.md:104`), and F2 already lists it. **The logs
  support Claude:** no tester recorded friction. Codex's evidence shows the
  cheaper route went unused, not that it was sought.
- **What settles it:** the author decides whether D1's "needed, found late"
  covers a route the testers never looked for. Either way it is already F2's
  first sub-item, so no new finding is needed.

### 2. codex-D2-1: is `AxisMapping` a layout type an application configures?

- **Claim (Codex):** `AxisMapping` is among the "configurable layout, shape,
  and theme values" (`R/axis_mapping.rs:37`), so it is an omission.
- **Counter (Claude):** no public signature outside `R/axis_mapping.rs`
  names it. It is a standalone helper a custom view uses to "map positions
  itself" (`R/axis_mapping.rs:7-10`), in the same class as `ClickCounter`,
  which Codex itself places among helpers without a membership requirement.
  Only the gallery uses it.
- **What the source shows:** outside its own module, `AxisMapping` appears
  only in the root re-export (`R/lib.rs:60`), in the `GeometryError` rustdoc
  (`R/layout.rs:63`, `:94`) and in a test (`crates/pert-ui-runtime/tests/errors.rs:164`).
  No widget builder, controller or `Component` signature takes or returns it.
  The type's rustdoc says a view "maps positions itself and draws undistorted,
  one `AxisMapping` per zooming axis" (`R/axis_mapping.rs:7-10`). The corpus
  use is `apps/gallery/src/pages/layout/transforms.rs`. **The code supports
  Claude:** nothing is configured with it. Whether "layout types an
  application configures" (`R/lib.rs:306-307`) covers a helper a custom view
  constructs is the remaining reading.
- **What settles it:** the author's reading of the layout clause against the
  facts above. If it is not covered, drop the instance from codex-D2-1.

### 3. claude-D1-1: does rustc suggest imports from non-std dependencies at all?

- **Claim (Claude):** in toy crates on rustc 1.99.1, a missing type got no
  `use` suggestion in every re-export shape tried, including a type defined
  directly in the dependency, while `HashMap` got one. So no re-export layout
  can make rustc point to `pert_ui::Modifiers`, and the fix cannot be a
  re-export shape.
- **Counter (Codex):** the friction holds (`U/r1-2-claude/diagnostics.md:4`,
  `FRICTION.md:92`), but the universal claim is broader than the evidence.
  Codex did not reproduce the experiments. This is unverified, not
  contradicted.
- **What the source shows:** the repository cannot settle a compiler
  behavior. The pert-ui-specific part is already verified: slice 3's pilot
  probe reproduced E0422 with the `Notifier` suggestion and no help for
  `CommandScope`, `KeyChord` and `CommandKey` (`U/triage/group-A.md:30`), and
  F3 records "rustc suggests no `use pert_ui::…` for facade items". The toy
  crates are not in the repository.
- **What settles it:** a reproduction on the pinned toolchain
  (`rust-toolchain.toml`) with a consumer of a dependency that defines and
  re-exports a type, checked for a `consider importing` help. The result only
  affects the wording of G5-6's fix. The membership findings stand either way.

## Withdrawn and refuted parts

- codex-D2-1 disposition, `HeldInputId` as host plumbing: withdrawn by Codex in verification; now G5-8.
- codex-D2-1 disposition, blanket exclusion of `NativeWindowId`: corrected by Codex for native configurations; now G5-28.
- claude-D2-5, "unclassified" for `EditToken` and `ParameterValue`: withdrawn by Claude, which accepts Codex's rule-included classification; now G5-12.
- claude-D2-5, `DeliveryStatus` in the host family: moved by Claude to its delta finding; now G5-15.
- claude-D2-2, incomplete: `ListDropMark` added by Claude after Codex's instance; now in G5-9.
- codex-D1-2 row 4 (`Runtime::query`): refuted by Claude, not withdrawn; Disputed facts 1.
- codex-D1-2 row 5, the r1-4 Ctrl+A helpers as evidence: refuted by Claude, because they drive the editor through host input as `SemanticAction`'s rustdoc recommends (`U/r1-4-claude/FRICTION.md:49`). The r1-1-claude part is kept as G5-5.
- codex-D2-1, `AxisMapping`: refuted by Claude, not withdrawn; Disputed facts 2.
- codex-D2-1, the one-hop instances `EditorAttachment`, `EditorCompletion`, `EditorTrigger`, `EditorOutcome`, `Transform2D`, `FittedText`, `PathElement`, `VectorStyle`: facts confirmed, but they are not established project-rule omissions while the closure is undefined; moved to policy G5-26.
- claude-D1-1, the universal compiler claim: unverified by Codex; Disputed facts 3. The friction part is kept as G5-6.
- claude-D2-3, the `SeriesSource` conflict as a departure: Codex says the existing exclusion covers it; kept only as policy G5-11.
- claude-D2-4, the listed inspection and helper names as project-rule omissions: not established (Codex), since inspection is expressly excluded (`R/lib.rs:320`); the documentation part is kept as G5-25 and the rest is policy G5-26.
- claude-D2-7, `WindowError` as a required member: conflicting clauses; moved to policy G5-30.
- claude-D2-8, "7 of 8 round 1 code bases" for capture and CPU names: corrected by Codex to six, because r1-1-codex only glob-imports the root; the sheet author's grep agrees. Applied in G5-32.
