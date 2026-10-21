# Merge sheet: G4

Inputs: `reports/codex-G4.md`, `reports/claude-G4.md`,
`reports/verification/codex-G4.md`, `reports/verification/claude-G4.md`
(all under `plans/usability/api-audit/`), and slice 3's F1–F34 in
`plans/developer-usability.md`. `crates/` at HEAD (245053b) is unchanged
from the audited commit d4ba108.

Path prefixes: `C/` = `crates/pert-ui-core/src/`, `R/` =
`crates/pert-ui-runtime/src/`, `F/` = `crates/pert-ui/src/`, `CPU/` =
`crates/pert-ui-cpu/src/`, `GPU/` = `crates/pert-ui-wgpu/src/`.
"All" = every audited configuration exposing the item.

Counts: 30 rows. 20 findings (`both`, `confirmed`, `partly`), 8 rows with
status `policy`, 2 `delta-unverified`. Further policy aspects inside
findings: G4-4 (rule kind), G4-10 (exception), G4-13 (exception as an
alternative fix), G4-22 (proposal part), G4-27 (`Borrow<str>`).

## Findings

| # | Rubric item, rule kind | Finding (one or two sentences, naming the items) | Sig/beh | Configs | Severity | Confidence | Fix (breaking?) | Destination | Sources and status | Slice 3 |
|---|---|---|---|---|---|---|---|---|---|---|
| G4-1 | T1, guideline C-DEBUG, C-COMMON-TRAITS | 113 public concrete types implement no `Debug`. They include the state owners and handles an application stores (`ListSelection`, `ListController`, `ListFilter`, `TableController`, `TreeController`, `FieldHandle`, `FormStatus`, `ContextMenuOpener`, `ThemeManager`, `FontManager`, `IconManager`, `UiSettings`, `NumberFormat<T>`, `FilterTarget`, `ChildInfo`, `Runtime`, `VectorRasterizer`, `pert_ui_wgpu::Renderer`, `pert_ui::App`), about 80 widget and container descriptions with `Dynamic<T>`, `Each<T>`, `Element` and `pert_ui::Svg`, and the callback contexts (`EventContext`, `UpdateContext`, `MeasureContext`, `ArrangeContext`, `DescribeContext`, `CellEdit`, `Painter`); full list in codex-T1-1. Sibling controllers (`DialogController`, `QuickPickController`, `ZoomController`, …) do implement it. | sig | all; `App` in the desktop configurations; `Svg` in `svg`, `no-default-svg` | Auditors: codex low for all, claude medium for state owners and low for the rest; codex's verification accepts medium for stored handles. Proposed: **medium** for the state-owner group, because `#[derive(Debug)]` on an app struct holding one fails; **low** for descriptions and contexts | high | Add `Debug` (a `finish_non_exhaustive` form like `ZoomController`'s), state owners first, without imposing `Debug` bounds on application values or callbacks; non-breaking | TODO.md | codex-T1-1, claude-T1-1: **both** | - |
| G4-2 | T1, guideline C-COMMON-TRAITS | Identities, keys and tokens with `Eq` but no `Hash`: `CommandId`, `FontId`, `HeldInputId`, `KeyCode`, `Modifiers`, `CommandKey`, `KeyChord` (no `Ord` either), `PointerButton`, `WorkspacePath`, `ColorRole`, `MetricRole`, `FontRole`, `Icon` (both), plus `Revision`, `EditSequence`, `RequestTicket<K>`, `ThemeMode`, `Density`, `TextStyle`, `ProfileOperation`, `ValidationCode` (codex, confirmed). Siblings `WidgetId`, `NativeWindowId`, `ImageId`, `ParameterId`, `StringRole` implement `Hash`. | sig | all | low (both) | high | `Hash` consistent with the existing identity `Eq` (hash serial/key, generation and `Arc` pointer for `CommandId`, `FontId`, `RequestTicket`; `RequestTicket` conditional on `K: Hash`); no ordering added for symmetry; non-breaking | TODO.md | codex-T1-2, claude-T1-2: **both** (codex's extra eight confirmed by claude). Correction noted: the need is weaker for the `Ord` types `Revision`, `EditSequence`, `ProfileOperation`, `ValidationCode`, where a `BTreeMap` works | - |
| G4-3 | T1, guideline C-COMMON-TRAITS | Validated scalars and immutable images omit traits their domain supports: `Capacity` (`PartialOrd`, `Ord`), `PowerExponent` (`Ord`, `Hash`), `Magnification`, `UiZoom`, `ViewScale` (`Eq`, `Ord`, `Hash`), `PresentationRate` (`Eq`, `PartialOrd`, `Ord`, `Hash`), `MaskImage`, `RasterImage` (`Eq`, `Hash`). Their invariants exclude NaN, as `NormalizedValue`'s do (`C/normalized.rs:49`). | sig | all | low | high | Add the traits following `NormalizedValue`'s treatment of validated floats; non-breaking | TODO.md | codex-T1-3 (scalar half): **confirmed** by claude (need plausible: presets, cache keys, `Ord` for `Capacity`). Claude's T1 disposition ("no `Ord` gap a consumer plausibly needs") is superseded by its own confirmation | - |
| G4-4 | T1, guideline C-COMMON-TRAITS (codex) or consistency proposal (claude) | Exact-data records derive `PartialEq` but not `Eq` (conditional on `T: Eq` where generic): `EditOutcome<T>`, `ParameterSnapshot<T>`, `Sample<T>`, `SampleWindowEntry<T>`, `SampleWindow<T>`, `RemoteCommit<T>`, `AuthorityResponse<T>`, `AuthorityStatus<T>`, `TerminalEvent<T>`, `PreviewEvent<T>`, `DeliverySnapshot<T>`, `Telemetry<T>`, `InspectionReply<T>`, `ListSelectionChange`, `WorkspaceReport`, `EventHistory`, `ProfileStats`, `ProfileSample`, `ProfileSnapshot`, `SourceInspection`, `PresentationStatus`. | sig | all | low | high on facts; need disputed | Add `Eq` with generic bounds; non-breaking | TODO.md if kept as a finding; maintainer if a proposal | codex-T1-3 (record half): **confirmed** on facts; rule kind disputed. Claude: tests need only `PartialEq` and the parameter records are mostly `f32`, so it is a consistency proposal; codex maintains. Proposed: **consistency proposal**, since T1 makes a missing trait a finding only where "a consumer would plausibly need" it, and no need is shown | - |
| G4-5 | T1, guideline C-COMMON-TRAITS (user-facing `Display`) | `Color` implements `FromStr` (`R/drawing.rs:744`) and `to_hex` (`:479`) but no `Display`, though a color is user-facing in `ColorPicker`'s hex field. | sig | all | low | high | `Display` writing `to_hex()`'s text; non-breaking | TODO.md | claude-T1-3: **confirmed**. Correction: "round-trips" holds for quantized sRGB bytes, not arbitrary linear channel values (`R/drawing.rs:467`) | - |
| G4-6 | T2, policy (corrected from rule departure) | `Divider` and `pert_ui::Svg` are `Send + !Sync`, the only `Send` component descriptions; neither rustdoc mentions threads. **Question:** should component descriptions be uniformly `!Send` (private marker) so a later field cannot remove an incidental `Send`, or should these two document their `Send` as intended? | sig | all; `Svg` in `svg`, `no-default-svg` | low | high on facts | A `PhantomData<Rc<()>>` marker (breaking) or a rustdoc statement (non-breaking) | maintainer; TODO.md if adopted | claude-T2-1: **policy**. Codex refuted it as a rule departure; claude accepted that and reclassified it | - |
| G4-7 | T2, policy | `RichText` and `TextSpan` are `!Send + !Sync` because a span can hold a UI-local `FontId` (`R/text/rich.rs:25,42,87`; `R/text/fonts.rs:86`); their rustdoc ("owned display text") states no thread restriction. **Question:** should styled text become `Send` (a sendable font key) so it can be built on a worker for `Runtime::spawn`, whose result must be `Send`? No run demonstrated the need. | sig | all | low | high on facts, medium on need | Document the UI-thread restriction (non-breaking); a sendable form is a design decision | TODO.md (doc); follow-ups (sendable form) | claude-T2-2: **policy**. Codex partly confirmed: facts hold, auto traits match ownership, need not established, doc clarification belongs to documentation review | - |
| G4-8 | T3, consistency proposal | `InspectionSnapshot`, `ComponentInspection` and `ValidationCode` are exhaustive although their rustdoc records growth (schema 5, 10–14: `R/diagnostics.rs:677,688,856,863`; `R/validation.rs:150`). **Question:** add `#[non_exhaustive]` (with constructors where consumers build records), or do these records deliberately track schema-breaking releases? | sig | all | low | high on facts, medium on policy | `#[non_exhaustive]`; breaking for literals and exhaustive matches | maintainer; TODO.md if adopted | codex-T3-1, claude-T3-1: **policy** (both) | - |
| G4-9 | T3, consistency proposal | 30 further inspection DTOs and status enums are exhaustive with no recorded growth: `QueryResult`, `WindowInspection`, `SourceInspection`, `ParameterInspection`, `ProfileSnapshot`, `ProfileStats`, `ProfileSample`, `ValidationReport`, `ValidationIssue`, `ValidationMarker`, `EventRecord`, `EventHistory`, `DragInspection`, `Capture`, `CaptureJob`, `EventKind`, `ProfileOperation`, `Selector`, `InspectValue`, `DiagnosticCommand`, `SurfaceDismissal`, `SurfaceKind`, `PresentationStatus`, `PresentationAction`, `ToastStatus`, `TaskStatus`, `CommandStatus`, `CommandSource`, `DialogOutcome`, `AuthorityStatus`; comparable `Semantics`, `SemanticItem`, `KeyCode` and others are `#[non_exhaustive]`. **Question:** extend the treatment to these? | sig | all | low | high on exhaustiveness; growth not established | `#[non_exhaustive]` (breaking); claude's alternative of a "closed" statement on each is not required by T3 for non-error enums (codex) | maintainer; TODO.md if adopted | claude-T3-1 (extension): **policy**; codex confirmed exhaustiveness of all 30 but not expected growth | - |
| G4-10 | T4, guideline C-GOOD-ERR (codex: project rule via the rubric); exception is policy | `ParameterValue::{from_any, spec_from_any}` and `TryFrom<AnyValue>` for `f32`, `i64`, `bool`, `ChoiceId`, `TryFrom<AnySpec>` for `ParameterSpec<T>` return the rejected `AnyValue`/`AnySpec` (`C/family.rs:61,71`; `C/erased.rs:86,143`), which implement neither `Error` nor `Display`, so `?` into `Box<dyn Error>` or `RuntimeError` fails. **Question:** accept rejected-input carriers as an exception (std's `TryFrom<Vec<T>> for [T; N]` idiom), or return a typed mismatch error retaining the input? | sig | all | Auditors: codex medium, claude low. Proposed: **low**, because the workaround is one `map_err` and the runtime already reports the same mismatch as `BindingError::KindMismatch` | high | Typed mismatch error carrying the input and families (breaking), or a documented exception | TODO.md if adopted | codex-T4-1, claude-T4-9: **both**; rule kind disputed (see Disputed facts D1) | - |
| G4-11 | T4, project rule (Error policy: derives) | `pert_ui_cpu::CpuRenderError` derives `Clone, Debug, PartialEq` but not `Eq` though every field is `Eq` (`CPU/lib.rs:14`); `RedescriptionError` derives only `Debug` though its data is `Clone + PartialEq` (`R/conformance.rs:12`). | sig | all | low | high | Add `Eq`; add `Clone`, `PartialEq`; non-breaking | TODO.md | codex-T4-2, claude-T4-5: **both** | - |
| G4-12 | T4, project rule (Error policy: causes) | `WorkspaceError::Participant` embeds `{source}` in its message and also exposes it through `#[source]`, so a chain prints the cause twice (`R/workspace.rs:643,650`); `RestoreRejection::State(WorkspaceError)` and `::Runtime(RuntimeError)` format the cause but expose no `source()` (`:702,712`). | beh (sig of `#[error]`) | all | Auditors: codex medium, claude low. Proposed: **low**, because each cause still appears once in `Display` and is reachable by matching | high | Drop `{source}` from `Participant`; give the restoration variants `#[source]` and drop `: {0}`; Display text change only | TODO.md | codex-T4-3, claude-T4-3: **both** | - |
| G4-13 | T4, project rule (one limit, one variant) | `FrameImage::MAX_BYTES` is reported as `LimitError::CaptureSize` (GPU offscreen, `GPU/lib.rs:678`) and `LimitError::FrameSize` (software presentation, `F/platform/presenter.rs:250,254`) (both), and also as `CaptureError::TooLarge` and `CpuRenderError::TooLarge` (`CPU/lib.rs:181`, converted at `:42`). `LimitError`'s rustdoc says the CPU renderer reports it (`R/render_limits.rs:37`), but `pert_ui_cpu` exposes no `LimitError`; only the facade's presenter maps it (`F/platform/presenter.rs:209,314`). | sig and beh | declarations all; software producers in the facade's CPU configurations | low | high | Consolidate the variants (breaking), e.g. `CpuRenderError::Limit(LimitError)`, or adopt an explicit policy exception; correct the `LimitError` rustdoc either way | TODO.md | codex-T4-5, claude-T4-4: **both** for `CaptureSize`/`FrameSize`; claude's CPU, capture and doc parts **confirmed**. Codex addition: `CpuRenderError::TooLarge` also reports reservation failure (`CPU/lib.rs:197`) | - |
| G4-14 | T4, guideline C-GOOD-ERR (`Display` names the value) | Unit variants whose message cannot say which value failed. Keys and identities: `RuntimeError::{AmbiguousKey, DuplicateKey, ChildTypeChanged, ShortcutConflict}`, `LayoutError::{DuplicateKey, ReservedKey, UnknownKey}`, `FontError::UnknownIcon`, `ChoiceError::DuplicateValue`. Argument validation: `LayoutError::{InvalidWeight, InvalidRatio}`, `ParameterError::{InvalidRange, OutOfRange, OffLattice, DuplicateOption, UnknownOption}`, `LiveError::CapacityExceeded`, `NormalizedValueError::OutOfRange`, `GeometryError::{InvertedBounds, ViewScaleOutOfRange, ZoomOutOfRange}`, `NumberInputError::{InvalidRange, InvalidStep, InvalidPrecision, OutOfRange, OffStep}`, `DrawError::InvalidImage`, `PathError::InvalidParameter`, `FontError::MissingGlyph`, `NumberLocaleError::SameSeparators`, `TextEditError::{InvalidComposition, InvalidSelection}`. Paths: `pert_ui::images::load`'s `ImageError::Io` and `list_directory`'s `ListingError` omit the path (`F/images.rs:243`; `R/file_browser.rs:100,296`). | sig and beh | all | Auditors: codex medium for all; claude low, medium for key/identity errors. Proposed: **medium** for `AmbiguousKey`, `DuplicateKey`, `ChildTypeChanged`; **low** for the rest, whose argument the caller still holds | high | Carry the key, name, value or path (`AmbiguousKey(String)` …; generic choices by position); breaking | TODO.md | codex-T4-4, claude-T4-6: **both** on `AmbiguousKey`, `DuplicateKey`, layout keys, `UnknownIcon`; codex's extras **confirmed** by claude; claude's extras (`ShortcutConflict`, `ChoiceError::DuplicateValue`, both paths) **confirmed** by codex. "Errors carry no location" does not forbid retaining rejected arguments (both) | F23 |
| G4-15 | T4, policy | **Question:** is losing `Copy` on `LayoutError`, `ParameterError`, `GeometryError` and `ChoiceError` acceptable to carry the rejected values of G4-14? | sig | all | low | high | Decision gates G4-14's fix for these enums | maintainer | claude-T4-6 (and claude's verification of codex-T4-4): **policy** | F23 |
| G4-16 | T4, project rule (`# Errors` names variants, incl. forwarded) | `# Errors` sections list fewer phase errors than `writable()` / `Signal::set` return: `ListSelection::{set_active, select, clear}`, `TreeController::{set_expanded, toggle, set_source, filter}`, `TableController::{set_source, set_sort, filter, set_active_column}`, `UiSettings::{set_motion, set_debug_overlay}`, `Runtime::{set_viewport, inspection_service}` (both); `ListController::reveal`, `TreeController::reveal`, `TableController::reveal`, `ZoomController::{set, set_changed, fit}` (codex, confirmed); `Runtime::mailbox` documents only `Closed` (codex; claude: only `WriteDuringInspection` and `WriteDuringDelivery` reachable). Disputed instances: `FieldHandle::{set, set_changed, set_modified, clear, remove}` (D2), `EditSession::update` (D3), `Runtime::set_visible` (D4). | doc (beh) | all | Auditors: codex medium, claude low. Proposed: **low**: every omitted failure is still a typed error and several omitted variants are unreachable | high | List the reachable variants or link `Signal::set`'s complete list; non-breaking | TODO.md | claude-T4-1, codex-T4-7: **both**, **partly** after verification. Corrections: claude withdrew `WriteDuringLayout` for `set_viewport` and `inspection_service` (unreachable from `&Runtime`); claude narrowed codex's `mailbox` and `set_visible` to reachable variants | - |
| G4-17 | T4, project rule (`# Errors` names variants) | `# Errors` sections name a category or condition, not a variant. Category only: `EditSession::{update, commit, cancel}`, `ParameterHost::{write_base, replace_spec, set_effective, remove}`, `AnyParameterHost::remove`, `RemoteHost::{on_submit, clear_submission_sink, respond, resynchronize}`, `Parameter::begin_edit`, `Runtime::{parameter, remote_parameter, cancel_input, set_presentation_rate, service_inspection}`, `FloatingWindow::{position, initial_size, fixed_size}`, `EventContext::{set_timer, start_drag}`, `Permission::get`, `FileBrowserController::selected_paths`. Conditions without variant: `ParameterState::{begin, replace_spec, write_base}`, `Runtime::{mount, mount_tree, children, invalidate, set_visible, unmount, focus, event, paint}`, `Color::rgba`, `MenuBar::new`, `ViewState::{new, with}`, `Workspace::{new, decode}`, `WorkspacePath::new`, `WorkspaceValue::text`, `Renderer::recreate`. "A `GeometryError`" without the variant: `FileBrowser::new`, `Table::new`, `Tree::new`, `VirtualList::{new, estimated}`. Nested variant without its `RuntimeError` wrapper: `App::{size, min_size, max_size}`, `NativeWindowOptions::size`. | doc (sig) | all; `App` desktop configurations; `Renderer` wgpu | low | high | Name the variants; a shared binding-family paragraph or a link to a complete operation contract suffices (codex); non-breaking | TODO.md | claude-T4-2: **confirmed** by codex for every instance; overlaps codex-T4-7 (`EditSession::update`, `set_visible`, `ViewState`, `WorkspacePath::new`, `WorkspaceValue::text`) → **both** there; `Renderer::recreate` from codex-T4-7 confirmed by claude | - |
| G4-18 | T4, project rule (every `Result` has `# Errors`) | Five fallible conversion impls have no documentation at all: `ChoiceId`'s `TryFrom<&str>` and `TryFrom<String>` (`C/choice.rs:64,72`), `Magnification::try_from` (`R/pointer.rs:134`), `UiZoom::try_from` (`R/runtime/settings.rs:77`), `ViewScale::try_from` (`R/transform.rs:309`). | doc | all | low | high | Add sections linking the validation contract; non-breaking | TODO.md | codex-T4-6: **partly** (claude confirmed these five; the other ten forward in a doc line, see G4-19) | - |
| G4-19 | T4, policy | **Question:** may standard-trait implementations forward to a documented constructor in one line instead of an own `# Errors` section? Covers `Capacity`, `PowerExponent`, `Length`, `PresentationRate` `try_from`, `Color::from_str`, `TryFrom<AnySpec>`, the four macro `TryFrom<AnyValue>` impls, and the hand-written serde impls of G4-20. The rule says "Every public `Result`". | doc | all; serde rows in the serde configurations | low | high | State the exception, or add sections (non-breaking) | maintainer | codex-T4-6: **policy** (raised by codex; claude agreed the ten are a heading-form question and added the serde rows) | - |
| G4-20 | T4, project rule (every `Result` has `# Errors`) | Hand-written `Serialize::serialize`/`Deserialize::deserialize` for `ChoiceId`, `ChoiceOption`, `NormalizedValue`, `ParameterId`, `ParameterSnapshot<T>`, `AnyParameterSnapshot`, `ParameterSpec<f32/i64/bool/ChoiceId>`, `DropOperations`, `UiZoom`, `Viewport`, `WorkspaceValue`, `WorkspacePath`, `ViewState`, `Workspace` (31 methods) have no `# Errors` section, though they reject invalid data (e.g. `C/normalized.rs:219`, `R/drawing.rs:1690`, `R/workspace.rs:1672`). | doc | serde configurations | low | high | Document forwarded format errors and validation failures, linking validation contracts, without inventing variants for `D::Error`; non-breaking | TODO.md unless G4-19 adopts the exception | codex-T4-d1: **delta-unverified**. Note: claude's own delta read the same rows, found each validating type documents its rejection rule in a `# Serialization` section (`R/workspace.rs:350-353`, `R/drawing.rs:822`, `C/erased.rs:168`), and filed them under G4-19 rather than as a finding | - |
| G4-21 | T4, policy | **Question (F11's second half):** should `DrawError` convert into `RuntimeError` so a test helper can use `paint()?`? `docs/contracts.md:1131` explicitly excludes this conversion. | sig | all | low | high | Keep the exclusion, or add the conversion (non-breaking) | maintainer; diagnostics survey | claude-T4-7 (its policy question): **policy**; both agree the current API meets the contract. Claude withdrew T4-7's finding part (see below) | F11 |
| G4-22 | T4, project rule (application failures); proposal | F30's source loss is resolved: `From<Box<dyn Error + Send + Sync>> for ApplicationError` keeps the box (`R/error.rs:79`). Remaining: `ApplicationError::new`'s rustdoc (`R/error.rs:48-51`) does not point to that `From`, and `RuntimeError` has no `From<Box<dyn Error + Send + Sync>>`, so `?` on a boxed error needs `.map_err(ApplicationError::from)?`. | doc and sig | all | low | high | Doc pointer on `new` (non-breaking); optional `From<Box<…>>` for `RuntimeError` (consistency proposal, additive) | TODO.md (update F30's entry); maintainer for the proposal | claude-T4-8: **partly**: source-loss part resolved (codex: "F30 is not current"); codex says the doc pointer is outside G4's documentation scope and the `RuntimeError` conversion is an optional proposal (**policy**) | F30 |
| G4-23 | T4, policy | `InputRejection` and `ListingError` display application or OS user text verbatim, uppercase and with arbitrary punctuation ("Name must not be empty", `R/text_fields/normalize.rs:18-25`; `R/file_browser.rs:76-83`), against "Display messages begin in lowercase". **Question:** exempt user-message carriers (and document it), or keep `Display` diagnostic and expose user text through `message()`? | beh | all | low | high | Record a contract exception, or split diagnostic and user text; do not rewrite application prose | maintainer; TODO.md | codex-T4-8, claude-T4-10: **policy** (both) | - |
| G4-24 | T4, project rule (callback-boundary exception) | The callback-boundary exception justifies `From<WindowError> for RuntimeError` by "a runtime method returning `WindowError`", but no public method returns `Result<_, WindowError>`; consumers meet it only in `PresentationStatus::Failed` (`R/runtime/native_windows.rs:933`), and the only producer is private (`F/platform/windows.rs:50`). | doc | all | low | high | Correct the rationale in `docs/contracts.md:1125`, `R/runtime.rs:158` and `R/error.rs:15`; keep the `From` (removing it would be breaking and unnecessary) | TODO.md | claude-T4-11: **confirmed** | - |
| G4-25 | T5, guideline C-STRUCT-PRIVATE; project rule (private invariants) | `Semantics` documents sorted, duplicate-free actions, none when disabled, and no `value` when `protected` (`R/diagnostics.rs:476,481,494`); `SemanticItem` likewise plus a never-empty name (`:350-367`). Only the builders enforce this; public field assignment and the derived `Deserialize` bypass it, and `describe_component` forwards the result (`R/runtime/inspect.rs:145-161`), contradicting "never exposes its content through inspection" (`R/diagnostics.rs:622`). | sig and beh | all | Auditors after verification: codex medium (narrowed from high) for the protected-value exposure, low for action/name; claude medium. Proposed: **medium** / **low** as agreed | high | Private fields with accessors and validated deserialization (breaking), or normalize at `describe_component`: clear `value` when protected, sort/deduplicate actions, clear them when disabled (non-breaking, claude) | TODO.md | codex-T5-1: **confirmed** by claude. `PickItem` is excluded (documented plain data) | - |
| G4-26 | T5, guideline C-STRUCT-PRIVATE; project rule (validated identities) | `CaptureJob { pub id, pub window, pub include_overlay }` (`R/capture.rs:531-539`) is trusted by `Runtime::capture_scene`, which renders `job.window`/`job.include_overlay` without looking up request `id` (`R/runtime/capture.rs:296-310`), while `complete_capture` answers by `id` alone (`:321-344`); a built or edited job silently attributes a same-sized window's pixels to the wrong request. | sig and beh | all | low | high on the path, medium on likelihood | Private fields with accessors, and have `capture_scene` look up the pending request by id and reject stale or foreign jobs (codex's addition); breaking | TODO.md | claude-T5-1: **confirmed** by codex | - |
| G4-27 | T6, guideline C-CONV-TRAITS | `ChoiceId` exposes `as_str` but no `AsRef<str>` (`C/choice.rs:32,50`), so it cannot be passed to `impl AsRef<str>` APIs such as `ContentQuery::new`. Claude also proposes `Borrow<str>` for `&str` lookups in `HashMap<ChoiceId, _>`. | sig | all | low | high | Add `AsRef<str>` (and `Borrow<str>` if adopted); non-breaking | TODO.md | codex-T6-1, claude-T6-1: **both** for `AsRef<str>`; `Borrow<str>` is a proposal, not a required T6 trait (codex) | - |
| G4-28 | T6, guideline C-CONV-TRAITS | `FontData::from_owned(Vec<u8>)` and `from_static(&'static [u8])` have no matching `From` impls (`R/text/fonts.rs:43,51,64`). | sig | all | low (small benefit: `FontManager::register` takes the concrete type) | high | Add the two `From` impls, keep the named constructors; non-breaking | TODO.md | codex-T6-1: **confirmed** by claude | - |
| G4-29 | T6, guideline C-CONV-TRAITS | `Tab::into_element`, `Section::into_element`, `Property::into_element` and `FloatingWindow::into_element` (`R/tabs.rs:119`, `R/collapsible.rs:849`, `R/property_grid.rs:88`, `R/windows.rs:494`) are infallible conversions with no `From<_> for Element`, so `.map(Into::into)` into `reconcile_children`'s `Vec<Element>` fails. | sig | all | low | high | Add the `From` impls, keep the methods; non-breaking | TODO.md | claude-T6-1: **confirmed** by codex | - |
| G4-30 | T6, consistency proposal | Only `PowerExponent` (`From<PowerExponent> for f32`, `C/parameter.rs:96`) and `UiZoom` (`From<UiZoom> for f64`, `R/runtime/settings.rs:81`) convert back to their raw number; `NormalizedValue`, `Capacity`, `Length`, `Magnification`, `ViewScale`, `PresentationRate` read only through `get()`. | sig | all | low | high | Add `From<Self>` for the raw type everywhere, or keep `get()` as the one reading and serialize `PowerExponent` through a private conversion (breaking for the removal) | TODO.md if adopted | claude-T6-d1: **delta-unverified** | - |

## Disputed facts

### D1. Does the project rule require error types to implement `std::error::Error`? (G4-10)

- **Claim (codex):** returning `AnyValue`/`AnySpec` is a departure from the
  project rule; it cites `rubric.md:324` (now `:327`) "it is typed and
  implements `std::error::Error`". Severity medium.
- **Claim (claude):** the contract requires only "Every public failure has
  a typed error" (`docs/contracts.md:1111`); the `Error` requirement is the
  rubric's reading of C-GOOD-ERR, and std returns the input as its
  `TryFrom` error, so it is a guideline departure and a policy question.
- **Source:** `docs/contracts.md:1109-1142` ("Error policy") never mentions
  `std::error::Error`; the requirement appears only in
  `plans/usability/api-audit/rubric.md:327`, under T4, whose sources are
  "project rule: contracts 'Error policy'; guideline: C-GOOD-ERR"
  (`:324-325`). **The text supports claude:** the kind is a guideline
  departure. Under `rubric.md:19-21` a guideline departure is still a
  finding unless a project rule decides otherwise, so the finding stands
  either way; only its kind and the exception question remain.
- **Settles it:** the contract's Error policy text (above); the exception
  itself is policy (G4-10).

### D2. Do the `FieldHandle` `# Errors` sections omit forwarded variants? (G4-16)

- **Claim (claude-T4-1):** `FieldHandle::{set, set_changed, set_modified,
  clear, remove}` list "update, paint or inspection, or after shutdown",
  omitting layout and delivery, which handles meet when called from
  component callbacks in any phase.
- **Claim (codex):** they link to `Signal::set` (`R/field_status.rs:320,
  328, 343, 355, 375`), whose section names the complete set
  (`R/signal.rs:457`), so the remedy already exists; not a missing-coverage
  instance, though the wording could be clarified.
- **Source:** `R/field_status.rs:320-322` reads "Rejects writes during
  update, paint or inspection, or after shutdown, like any
  [`Signal::set`]"; `set_changed` (`:328-329`), `set_modified`
  (`:343-344`) and `clear` (`:354`) refer to `set`; `remove` (`:375-378`)
  repeats the narrow list with the link. `Signal::set` (`R/signal.rs:457-464`)
  lists `Closed`, `WriteDuringUpdate`, `WriteDuringLayout`,
  `WriteDuringPaint`, `WriteDuringInspection`, `WriteDuringDelivery`,
  `BorrowConflict`. **Both facts hold:** the explicit list is narrower than
  the linked one (`WriteDuringLayout` and `WriteDuringDelivery` are
  missing), and the link to the complete list exists. Both auditors kept
  `ListSelection`'s identical pattern ("like any signal write",
  `R/list_selection.rs:163-164`) as an instance.
- **Settles it:** whether a narrower explicit list followed by a link
  satisfies "naming the variants and conditions"
  (`docs/contracts.md:1141-1142`); decide it once for `FieldHandle` and
  `ListSelection` together.

### D3. Does `EditSession::update`'s `# Errors` cover `EditError::Pending`? (G4-16, G4-17)

- **Claim (codex-T4-7):** the section omits
  `RuntimeError::Binding(EditError::Pending)` returned after remote
  submission.
- **Claim (claude):** "finished-edit errors" plausibly covers it; the
  section is weak only as a category list (claude-T4-2).
- **Source:** the section reads "runtime-phase, finished-edit,
  invalid-value or counter errors" (`R/parameter_binding.rs:740-741`);
  `Pending` is returned when the remote has a submission for the token
  (`:745-753`). `EditError::Finished` is "A preview was delivered after
  the edit ended" (`C/edit.rs:213-214`), while `EditError::Pending` is "A
  pending edit cannot be acknowledged before a terminal outcome exists"
  (`C/edit.rs:231-233`), i.e. the edit has *not* finished. **The code
  supports codex:** "finished-edit" names `Finished`, and nothing in the
  section describes a submitted-but-unanswered edit.
- **Settles it:** the variant docs above; the instance stays under G4-17
  either way.

### D4. Which omitted phase variants can `Runtime::mailbox`, `set_viewport`, `inspection_service` and `set_visible` actually return? (G4-16)

- **Claim (codex-T4-7):** `mailbox` documents only `Closed` but
  `writable()` returns every phase error; `set_visible` documents only
  foreign/unmounted IDs, but window selection and the implementation also
  reject closed and forbidden phases (`R/runtime.rs:5019,5023,5030`).
- **Claim (claude):** update, layout and paint run inside `&mut Runtime`
  methods, so a `&Runtime` method cannot run then; `WriteDuringInspection`
  is reachable from a command's `enabled(&Runtime)` predicate; delivery
  only when a parameter sink reaches a shared runtime. For `set_visible`
  (`&mut self`), `Closed` after `shutdown` is real, plus delivery.
- **Source:**
  - `writable()` checks `Closed` and all five phases
    (`R/runtime.rs:691-711`).
  - The runtime's own test notes "Application code holds the runtime only
    between flushes, so these phases are set directly"
    (`R/runtime.rs:6355-6356`).
  - Command predicates are `Fn(&Runtime, &CommandContext)`
    (`R/runtime/commands.rs:469`) and run under `Phase::Inspect`
    (`:854-862`), so `WriteDuringInspection` is reachable for the `&self`
    methods `mailbox`, `set_viewport` and `inspection_service`. This
    supports claude for these three.
  - `set_visible` calls `window_of(id)?` (`R/runtime.rs:5023`), which runs
    `check` (`R/runtime/native_windows.rs:356-358`) and returns
    `Unmounted` for an unmounted id (`R/runtime.rs:4221-4222`) before
    `writable()` (`:5030`). `shutdown` removes every root subtree and
    marks it unmounted (`R/runtime.rs:6166-6176`; `R/runtime/tree.rs:1600`),
    and a mount after shutdown is refused by `writable()`
    (`R/runtime/tree.rs:1143`). So after shutdown `set_visible` returns
    the documented `Unmounted`, not `Closed`. **The code contradicts both
    sides' `Closed` claim for `set_visible`.** Whether `WriteDuringDelivery`
    can reach it (an application keeping the runtime in a shared cell that
    a sink borrows) was not traced further.
- **Settles it:** whether a sink can obtain `&mut Runtime` during
  `Phase::Delivery` (`R/runtime.rs:719`, `enter_delivery`); if not,
  `set_visible`'s documented set is complete and the instance drops.

### D5. Inventory version 2 error-row counts (minor, inventory only)

- **Claim (codex):** 41 added rows: 14 fallible conversions, 25 serde
  methods, two `ParameterValue` implementations.
- **Claim (claude):** 41 new entries: 15 `TryFrom`/`FromStr` and 26 serde.
- **Source:** `git diff 6e39e18 7c84f54 -- plans/usability/api-audit/inputs/candidates/errors.txt`
  adds 41 data rows: 14 `TryFrom`/`FromStr`, 25 serde, two
  `ParameterValue` (`from_any`, `spec_from_any` for `ChoiceId`). **The
  diff supports codex.** Version 3 (245053b) since added the three
  primitive `TryFrom<AnyValue>` rows both verifiers reported as a residual
  gap. No finding depends on the count.

## Withdrawn and refuted parts

- claude-T4-7, finding part (F11's first half: the blanket
  `impl<E> From<E> for AppError where DrawError: From<E>` makes rustc name
  `DrawError`): **withdrawn** by claude as a T4 finding; the error policy is
  met and the compiler diagnostic belongs to the diagnostics survey
  (`diagnostics/survey.md:91-92, 408-445, 559`), which recommends
  `#[diagnostic::do_not_recommend]`. Codex added that replacing the blanket
  impl is not generally non-breaking and the leaf list misses direct
  drawing conversions such as `PathError`. Its policy question is G4-21.
- claude-T2-1 as a project-rule departure: **refuted** by codex
  (descriptions hold no runtime handle; `Component` fixes callback thread,
  not construction-time affinity, `R/runtime.rs:1011`); claude accepted and
  reclassified it as policy (G4-6).
- claude-T4-1, `WriteDuringLayout` for `Runtime::set_viewport` and
  `inspection_service`: **withdrawn** by claude (unreachable from a
  `&Runtime` method); the instances stay for the other variants.
- codex-T4-7, the omitted update/layout/paint variants of `Runtime::mailbox`
  and the phase variants of `set_visible`: **refuted** by claude as
  unreachable (see D4); `mailbox` stays for inspection and delivery.
- codex-T4-6, ten of its fifteen instances as missing documentation:
  **refuted** in part by claude: they forward in one doc line to a
  documented contract; what remains is the heading-form policy question
  (G4-19).
- claude-T4-8, F30's source loss: **resolved**; both auditors agree
  `From<Box<dyn Error + Send + Sync>>` keeps the box (`R/error.rs:79`).
- F11's second half (`DrawError` into `RuntimeError`) as a departure:
  **refuted** by both; `docs/contracts.md:1131` excludes the conversion
  explicitly.
- codex-T5-1's "high" severity: **narrowed** by codex to medium after
  claude showed only the component author, who already holds the secret,
  can set the field.
- claude-T3-1's alternative fix of a "closed" statement for every
  exhaustive non-error enum: **refuted** by codex; T3 requires that only
  for error enums.
- claude-T6-1's `Borrow<str>`: **narrowed** by codex to a proposal; it is
  not one of T6's required conversion traits.
- Out of G4's scope, not merged: claude's T5 disposition notes for C4 that
  `TableController::set_sort` (`R/table.rs:742`) skips the `UnknownKey`
  check `set_active_column` (`:757`) applies.
