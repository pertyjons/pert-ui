# claude verification: G4

Source read at HEAD (crates/ unchanged since d4ba108). Paths below are relative
to `crates/pert-ui-runtime/src/` unless they name another package.

## The other auditor's findings

| other ID | matched by | verdict | evidence | remarks on severity, kind or fix |
|---|---|---|---|---|
| codex-T1-1 (113 types without `Debug`) | claude-T1-1 | matched | Same 113 types and configurations; e.g. `ListSelection` list_selection.rs:62, `Runtime` runtime.rs:3508, `pert_ui::App` pert-ui/src/app.rs:285. | Agree. Codex rates all low; I rate the stored state-owner group (`ListSelection`, the controllers, managers, `UiSettings`) medium, because a `#[derive(Debug)]` on an app struct holding one fails. |
| codex-T1-2 (keys and tokens without `Hash`) | partly: claude-T1-2 covers `CommandId`, `FontId`, `HeldInputId`, `KeyCode`, `Modifiers`, `CommandKey`, `KeyChord`, `PointerButton`, `WorkspacePath`, `ColorRole`, `MetricRole`, `FontRole`, `Icon` | confirmed (unmatched part) | Unmatched: `Revision`, `EditSequence` (pert-ui-core edit.rs:6, :30, derive `Eq, Ord`, no `Hash`); `RequestTicket<K>` (async_source.rs:26-45, hand-written `PartialEq`/`Eq` on key, generation and `Arc::ptr_eq`, no `Hash`); `ThemeMode`, `Density` (runtime/theme.rs:13, :24); `TextStyle` (text.rs:122); `ProfileOperation` (profiling.rs:12, has `Ord`); `ValidationCode` (validation.rs:112, has `Ord`). None implements `Hash`. | Low, rule kind right. The need is weaker for the `Ord` types (`Revision`, `EditSequence`, `ProfileOperation`, `ValidationCode`), where a `BTreeMap` already works; codex's own role table calls `Revision`/`EditSequence` scoped protocol tokens rather than keys. `RequestTicket`'s `Hash` must hash only the key, generation and `Arc` pointer to stay consistent with its `Eq` (async_source.rs:39-45). Fix right. |
| codex-T1-3 (validated scalars and exact records omit `Eq`/`Ord`/`Hash`) | none (claude spot-checked `Default`/`Ord` only) | confirmed (facts); need partly | Scalars: `Capacity` derives `Eq, Hash` but no `PartialOrd`/`Ord` (pert-ui-core live/mod.rs:181); `PowerExponent` has `PartialOrd` and a hand-written `Eq`, no `Ord`/`Hash` (pert-ui-core parameter.rs:58-66, validated to 0.1..=10); `Magnification` (pointer.rs:106, rejects <= 0 at :120), `UiZoom` (runtime/settings.rs:21, 0.75..=2), `ViewScale` (transform.rs:266, 1/32..=32) derive only `PartialEq, PartialOrd`; `PresentationRate` derives only `PartialEq` (runtime/presentation.rs:9). Their invariants exclude NaN and -0, so `Eq`/`Ord`/`Hash` are sound, as `NormalizedValue` does (pert-ui-core normalized.rs:49-62). `MaskImage`/`RasterImage` have hand-written byte equality (drawing.rs:1121-1128, resources.rs:226-233) and could add `Eq`. The listed records derive `PartialEq` without `Eq` although every field is `Eq` given `T: Eq`. I checked `EditOutcome` (pert-ui-core edit.rs:163-185), `ParameterSnapshot`, `Sample`, `SampleWindowEntry`, `SampleWindow`, `RemoteCommit`, `AuthorityResponse`, `AuthorityStatus`, `TerminalEvent`, `PreviewEvent`, `DeliverySnapshot`, `Telemetry`, `InspectionReply`, `ListSelectionChange`, `WorkspaceReport`, `EventHistory`, `ProfileStats`/`Sample`/`Snapshot`, `SourceInspection` and `PresentationStatus` (native_windows.rs:918; `WindowError` is `Eq`). | Low; rule kind right. The consumer need is plausible for the scalars: `Eq`/`Hash` for presets or cache keys, and `Ord` for `Capacity`. It is weak for the records. Tests need only `PartialEq`, and the parameter records are mostly instantiated at `f32`, where conditional `Eq` gives nothing. I would keep the scalar half as the finding and the record half as a consistency proposal. |
| codex-T3-1 (`ComponentInspection`, `InspectionSnapshot`, `ValidationCode` exhaustive) | claude-T3-1 | matched | Codex names three types; claude-T3-1 names the same three plus further DTOs and status enums. | Agree on proposal/policy question, low, breaking. |
| codex-T4-1 (`AnyValue`/`AnySpec` as `Result` errors without `Error`) | claude-T4-9 | matched | Same items: pert-ui-core family.rs:61, :71; erased.rs:86-93, :142-156. | Kind and severity are disputed. Codex calls it a medium-severity departure. Contracts require only "Every public failure has a typed error" (docs/contracts.md:1111). The `Error` requirement is the rubric's reading of the C-GOOD-ERR guideline, and std's own `TryFrom<Vec<T>> for [T; N]` returns the input as its error. So it is a guideline departure with std precedent, and I keep it as a **policy question** at low severity: `.map_err` is one call, and the runtime reports the same mismatch as the typed `BindingError::KindMismatch` (parameter_binding.rs:70). It goes to the maintainer. |
| codex-T4-2 (`CpuRenderError` lacks `Eq`; `RedescriptionError` lacks `Clone`/`PartialEq`) | claude-T4-5 | matched | pert-ui-cpu lib.rs:14-16; conformance.rs:12-14. | Agree. |
| codex-T4-3 (`RestoreRejection` causes not in `source()`; `Participant` prints its cause twice) | claude-T4-3 | matched | workspace.rs:643-653, :702, :712. | Severity differs: codex medium, claude low. The cause is still reachable by matching the variant and appears once in `Display`, so I keep low. Fix agrees. |
| codex-T4-4 (messages omit the rejected value) | partly: claude-T4-6 covers `RuntimeError::{AmbiguousKey, DuplicateKey}`, `LayoutError::{DuplicateKey, ReservedKey, UnknownKey}`, `FontError::UnknownIcon` and F23 | confirmed (unmatched part) | Unmatched instances are unit variants with category-only text: `RuntimeError::ChildTypeChanged` "retained child key changed component type" (runtime.rs:363-365); `LayoutError::InvalidWeight`/`InvalidRatio` (layout.rs:43, :46); `ParameterError::{InvalidRange, OutOfRange, OffLattice, DuplicateOption, UnknownOption}` (pert-ui-core parameter.rs:550-575); `LiveError::CapacityExceeded` (pert-ui-core live/mod.rs:168); `NormalizedValueError::OutOfRange` (pert-ui-core normalized.rs:119); `GeometryError::{InvertedBounds, ViewScaleOutOfRange, ZoomOutOfRange}` (layout.rs:92-109; the latter two print the allowed range but not the value); `NumberInputError::{InvalidRange, InvalidStep, InvalidPrecision, OutOfRange, OffStep}` (text_fields/number_inputs.rs:16-34); `DrawError::InvalidImage` (drawing.rs:30); `PathError::InvalidParameter` (geometry.rs:21); `FontError::MissingGlyph` (text/fonts.rs:135); `NumberLocaleError::SameSeparators` (number_format.rs:155); `TextEditError::{InvalidComposition, InvalidSelection}` (text_input.rs:21, :25). | Codex's medium is right only for the key and identity errors (`ChildTypeChanged` belongs with them). Most added instances validate one argument the caller still holds at the call site, so they are low. Adding payloads breaks `Copy` on several enums (`LayoutError`, `ParameterError`, `GeometryError`); that cost is the policy question claude-T4-6 records. |
| codex-T4-5 (`LimitError::{CaptureSize, FrameSize}` for one limit) | claude-T4-4 | matched | render_limits.rs:58-102; pert-ui-wgpu lib.rs:679; pert-ui/src/platform/presenter.rs:209-210, :254, :314-315. | claude-T4-4 also counts `CaptureError::TooLarge` and `CpuRenderError::TooLarge`, and reports `LimitError`'s doc claiming CPU-renderer use. No factual conflict. |
| codex-T4-6 (fallible `TryFrom`/`FromStr` impls lack `# Errors`) | none (claude recorded the omission only as inventory gap 1) | partly confirmed | All 15 have no own `# Errors` heading. Ten of them forward in one doc line to a documented contract: `Capacity` "Validates like [`Capacity::new`]" (pert-ui-core live/mod.rs:213); `PowerExponent` (parameter.rs:91); `Length` (containers.rs:40); `PresentationRate` (runtime/presentation.rs:33); `Color::from_str` "as [`Color::from_hex`] does" (drawing.rs:746); `TryFrom<AnySpec>` and the four macro `TryFrom<AnyValue>` impls, which state their single condition (pert-ui-core erased.rs:89, :145). Five have no doc at all: `ChoiceId`'s `TryFrom<&str>`/`TryFrom<String>` (pert-ui-core choice.rs:64, :72), `Magnification` (pointer.rs:132-136), `UiZoom` (runtime/settings.rs:75-79) and `ViewScale` (transform.rs:307-311). | The substantive departure is those five undocumented impls (low). For the other ten, only the heading form is at issue, which is codex's policy question (whether trait impls may forward). The same question covers the 26 serde `serialize`/`deserialize` entries version 2 adds (see the inventory delta). Fix right. |
| codex-T4-7 (`# Errors` sections omit conditions) | partly: claude-T4-1 covers `set_viewport`, `inspection_service`, `ListSelection::{set_active, select, clear}`, `TreeController::{set_expanded, toggle, set_source, filter}`, `TableController::{set_source, set_sort, filter, set_active_column}`, `UiSettings::{set_motion, set_debug_overlay}`; claude-T4-2 covers `EditSession::update` (category only) and `ViewState::{new, with}`, `WorkspacePath::new`, `WorkspaceValue::text` | partly confirmed (unmatched part) | **Confirmed:** `ListController::reveal`, `TreeController::reveal` and `TableController::reveal` (virtual_list.rs:289-291, tree.rs:538-540, table.rs:850-852) and `ZoomController::{set, set_changed, fit}` (zoom_view.rs:156-180) list only "`WriteDuringUpdate`, `Closed` or `BorrowConflict`" but write through `Signal::set`. These are further instances of claude-T4-1. Codex is also right that `ListSelection`'s section omits `Closed` and `BorrowConflict` (list_selection.rs:163-164), although "like any signal write" points to `Signal::set`. `Renderer::recreate` names conditions without the variant; all are `RenderError::Surface` (pert-ui-wgpu lib.rs:502-523). That is a further claude-T4-2 instance. **Partly:** `Runtime::mailbox` documents only `Closed` but calls `writable()` (runtime.rs:4109-4116). Only `WriteDuringInspection` is reachable among the omitted variants, from a command `enabled(&Runtime)` predicate run under `Phase::Inspect` (runtime/commands.rs:854-862). `WriteDuringDelivery` is reachable only if a parameter sink reaches a shared runtime. Update, layout and paint run inside `&mut Runtime` methods (runtime.rs:5318, runtime/compose.rs:22, runtime/tree.rs:1935), so a `&Runtime` method cannot be called then. `Runtime::set_visible` (`&mut self`, runtime.rs:5019-5030): the omitted `Closed` (after `shutdown`, runtime.rs:6128/6203) is real, and `WriteDuringDelivery` is reachable as for `mailbox`; the other phase variants are not. `EditSession::update`: "finished-edit errors" (parameter_binding.rs:739) plausibly covers `EditError::Pending` (:752), so codex's specific omission is weak. claude-T4-2's category-only point covers it. | Medium is too high for codex: every omitted failure is still returned as a typed error, and several omitted variants are unreachable. I keep low. Fix right. |
| codex-T4-8 (`InputRejection`, `ListingError` user-text `Display`) | claude-T4-10 | matched | text_fields/normalize.rs:18-25; file_browser.rs:76-83. | Agree: a policy question. |
| codex-T5-1 (`Semantics`, `SemanticItem` invariants on public fields) | none | confirmed | `Semantics` documents sorted, duplicate-free actions, no actions when disabled, and an absent `value` when `protected` (diagnostics.rs:475-497). Only the builders enforce this (:546-550, :556-563, :596-603, :624-629). `#[non_exhaustive]` (:458) blocks struct literals in other crates but not field assignment on a value from `Semantics::new`. The derived `Deserialize` (:459-463) also skips the builders. `describe_component` passes a component's semantics through unchanged apart from the name (runtime/inspect.rs:145-161). Only content search re-checks `protected` (runtime/content_search.rs:282). `SemanticItem` has the same pattern (diagnostics.rs:350-367, builders :390-427). | Confirmed; I missed it, since my T5 disposition treated these as plain records. Severity: medium, not high. Only the component author can assign the field and leak the value, and that author already holds the secret. The doc promise "never exposes its content through inspection" (:622-623) is what misleads. A cheaper fix than privatizing is non-breaking: normalize at the consumption boundary in `describe_component` (clear `value` when `protected`, sort and deduplicate actions, clear them when disabled). |
| codex-T6-1 (`ChoiceId` lacks `AsRef<str>`; `FontData` lacks `From`) | partly: claude-T6-1 covers `ChoiceId` | confirmed (unmatched part) | `FontData::from_static(&'static [u8])` and `from_owned(Vec<u8>)` (text/fonts.rs:50-66) have no `From` impls (`rg 'for FontData'` finds only `Debug`, :68). | Low. The benefit is small, because the only consumer, `FontManager::register(&self, data: FontData)` (runtime/fonts.rs:64), takes the concrete type, not `impl Into<FontData>`. Fix right, non-breaking. |

## My findings disputed

- **claude-T2-1 (`Divider` and `Svg` are `Send`).** Codex records T2 "no
  findings" and calls `Send`-only "consistent with that representation".
  - The facts hold: `Divider` (views.rs:906-911, a `bool`, a `Spacing` and two
    `Cell<f32>`) and `pert_ui::Svg` are `Send + !Sync`. They are the only
    `Component` implementors in types.txt that are `Send`.
  - The classification is wrong. A description holds no runtime handle, so it
    is not a "UI-thread type" in the rule's sense. C-SEND-SYNC also favours
    `Send` where possible.
  - **Corrected:** this is a consistency proposal or policy question, not a
    rule departure. The question is whether descriptions should be uniformly
    `!Send` so a later field cannot remove an incidental `Send`, or document
    their `Send`. Low.
- **claude-T2-2 (`RichText`/`TextSpan` `!Send` undocumented).** This is in
  the same T2 "no findings".
  - **Maintained** as a policy question with medium confidence on the need.
  - The facts hold (types.txt: both `!Send, !Sync`; text/rich.rs:73-87
    states no thread restriction), and no probe demonstrated the need.
    Codex's no-finding is consistent with that.
- **claude-T4-7 (F11).** Codex: F11 "does not establish a conversion-policy
  departure" (contracts.md:1131).
  - **Withdrawn as a T4 finding.** The error policy is met (claude-T4-7 said
    so). The remaining problem is the compiler diagnostic, and C-GOOD-ERR
    covers error values, not compiler messages.
  - The diagnostics survey already owns it
    (plans/usability/api-audit/diagnostics/survey.md:91-92, :408-445, :559).
    It recommends `#[diagnostic::do_not_recommend]`, which corrects my fix
    note that no attribute can help.
- **claude-T4-8 (F30).** Codex: F30 "is not current".
  - **Maintained, narrowed.** I agree the source loss is resolved:
    `From<Box<dyn Error + Send + Sync>>` keeps the box (error.rs:79-84).
  - What remains is a doc pointer on `ApplicationError::new` (error.rs:48-51
    does not mention it) and the consistency proposal for `RuntimeError`.
    No conflict of fact.
- **claude-T4-1 (my own instances, re-checked against codex-T4-7).**
  - **Corrected in part.** For `Runtime::set_viewport` and
    `Runtime::inspection_service` (`&self`), the omitted `WriteDuringLayout`
    cannot occur: layout runs inside `&mut Runtime` methods.
  - The omitted `WriteDuringDelivery` can occur only when a parameter sink
    reaches a shared runtime. The two instances stay, at low severity.
  - The handle instances (`ListSelection`, the controllers, `FieldHandle`,
    `UiSettings`) stand: handles are callable from component callbacks in any
    phase.
- **claude-T5-1 (`CaptureJob`).** Codex processed all 286 fields and flagged
  only `Semantics`/`SemanticItem`.
  - **Maintained.** `capture_scene` renders `job.window` and
    `job.include_overlay` without looking the request up by id
    (runtime/capture.rs:296-310), and `complete_capture` answers by id alone
    (:321-344).
  - `CaptureJob`'s fields are public and the type is not `#[non_exhaustive]`
    (capture.rs:531-539), so a host can build or edit one.
- **Other claude findings codex does not mention** (claude-T1-3, claude-T4-2's
  further instances, claude-T4-11, claude-T6-1's `into_element` part, claude-T3-1's
  additional types): codex states nothing that contradicts them. Re-checked
  facts: `Color` has `FromStr` but no `Display` (types.txt row). No public function
  returns `Result<_, WindowError>`: `rg 'WindowError>' api/` finds no `fn` line.
  All are maintained.

## Inventory delta

**Entries processed (version 2 additions or changes for T1–T6):**
- errors.txt: all 41 new entries;
- conversion-traits.txt: all 152 rows, rewritten as `impl Trait for Target`;
- types.txt:
  - the 76 rows that gain `[configs: serde]` traits;
  - the 42 "(for another type)" marks;
  - the 4 renamed `Error` paths.

A script diff of every types.txt row's trait and auto-trait sets against
version 1 found no change in any T1 trait (`Debug` … `Display`) and no
auto-trait change. The only changes are the serde traits, the moved
other-type impls and the full `core::error::Error` path. fields.txt and
non-exhaustive.txt are unchanged, so T3 and T5 have no delta.

**What the entries show:**
- **T4, errors.txt:** the 15 `TryFrom`/`FromStr` entries are those in
  codex-T4-6, already verified above.
  - The 26 serde `serialize`/`deserialize` entries return foreign
    `S::Error`/`D::Error` and have no `# Errors` heading. Each validating type
    documents its rejection rule in a `# Serialization` section instead, for
    example `WorkspacePath` (workspace.rs:350-353), `ViewState` (workspace.rs:220), `Viewport`
    (drawing.rs:822) and `AnyParameterSnapshot` (pert-ui-core erased.rs:168).
    No new finding: this joins codex-T4-6's policy question.
- **T4, conversions:** the version 2 targets confirm upward conversions, with
  `EditError`/`DeliveryError` normalized into `RuntimeError::Binding`
  (runtime.rs:390-400).
  - `ParameterError` has two routes into `RuntimeError`: `Parameter` directly,
    and `Binding(Edit(InvalidValue))`.
  - The runtime never produces `RuntimeError::Parameter` itself (`rg
    ParameterError` in pert-ui-runtime finds no producer). Its domain
    validation always goes through `EditError::InvalidValue` (pert-ui-core
    edit.rs:463, :585, :610). No failure has two representations, so this is
    not a finding.
  - `From<io::Error> for ListingError` keeps a kind and the OS text
    (file_browser.rs:100-110), which the foreign-failure rule allows.
- **T6:** one new finding, below.

### claude-T6-d1 Only two validated scalars convert back to their raw number with `From`

- Rubric item T6. Kind: **consistency proposal**. C-CONV-TRAITS allows the
  impls; the inconsistency is the finding.
- Paths: `pert_ui_core::PowerExponent` and `pert_ui_runtime::UiZoom`. All
  configurations. Signature.
- Version 2 shows `impl From<PowerExponent> for f32`
  (pert-ui-core parameter.rs:96-100) and `impl From<UiZoom> for f64`
  (runtime/settings.rs:81-85). Version 1 filed these as `From<PowerExponent>`
  and `From<UiZoom>` under the type, with no target.
- The other validated single-number types read their value only through
  `get()`, with no `From<Self>` impl: `NormalizedValue`, `Capacity`,
  `Length`, `Magnification`, `ViewScale`, `PresentationRate`.
  - `rg 'impl From<.*> for (f32|f64|usize)'` finds only the two above.
  - So `f32::from(exponent)` compiles but `f32::from(position)` does not.
  - `PowerExponent`'s impl exists because its serde attribute
    `into = "f32"` needs it (parameter.rs:59-63).
- Rule: C4 says a raw value is read with `get()`. T6's proposal asks that
  conversion spellings be consistent.
- Severity: low. Confidence: high.
- Fix, one of:
  - add `From<Self>` for the raw type on every validated scalar;
  - keep `get()` as the one documented reading, and serialize
    `PowerExponent` through a private conversion instead of a public `From`
    (breaking only for the `From` removal; unpublished).
- Destination: TODO.md if adopted.

**Residual inventory gap, not a finding:** errors.txt version 2 still omits
the fallible trait methods implemented for primitive types. These are
`TryFrom<AnyValue>::try_from` and `ParameterValue::{from_any, spec_from_any}`
for `f32`, `i64` and `bool`, generated at pert-ui-core erased.rs:142-155 and
family.rs:400. Only the `ChoiceId` instances appear, although
conversion-traits.txt lists all four `TryFrom<AnyValue>` impls. The trait
declarations document their `# Errors` (family.rs:56-71).
