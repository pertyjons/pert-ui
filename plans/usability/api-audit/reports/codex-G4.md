# Codex report: G4, types

Audited commit: `d4ba108f66b7c8ffbf350ae01bedc95d147cd7e6`.

Read-only audit; no files changed and no builds or tests run. No delegation or other auditor reports were used.

“All” below means every audited configuration exposing the defining item. Re-exports are counted once.

## Dispositions

| Item | Disposition | Coverage |
|---|---|---|
| T1 | Findings | Processed all 534 `types.txt` entries and the semantic records. Read all 55 public hand-written `Debug` implementations; all produce nonempty output. Reviewed common traits against representations and documented roles. |
| T2 | No findings | Checked all 534 type entries, including conditional auto traits, against ownership and thread documentation. Rechecked common and auto traits in all 17 frozen configurations; no configuration differences were found. The concrete types comprise 193 neither-`Send`-nor-`Sync`, 327 conditionally or unconditionally both, and three `Send`-only types. Eleven entries are interface traits. |
| T3 | Finding: consistency proposal | Processed all 251 `non-exhaustive.txt` entries: 196 enums and 55 structs with public fields. Cross-checked error declarations and relevant attributes. Error enums satisfy the project rule; `SendError` explicitly documents its closed set. |
| T4 | Findings | Processed all 586 `errors.txt` entries: 540 returning functions and 46 `Error` implementations. Read their error sections, error representations, messages, sources and conversions. Additionally examined 17 fallible conversion methods omitted from that list. Re-examined F11, F23 and F30. |
| T5 | Finding | Processed all 286 public fields across 55 structs, reading their rustdoc and relevant constructors/builders. |
| T6 | Finding | Processed all 198 conversion-list entries, resolving them against 103 distinct source sites, including macro definitions. Screened the semantic lists’ 534 type records and 1,992 method signatures for applicable conversions. No confusable inherent `from_str` duplication was found. |

The 813 attribute entries were screened for applicability: 288 type entries were cross-checked against the type and non-exhaustiveness reviews; 525 function attribute entries concern `must_use`, outside G4.

No candidate-row sampling was used. Inventory defects are recorded separately below.

## Findings

### codex-T1-1 Public concrete types lack `Debug`

- **Rule:** guideline C-DEBUG and C-COMMON-TRAITS.
- **Paths/configurations:** 113 types listed below. Runtime and wgpu items: all. `App`: `cpu`, `default`, `image-formats`, `images`, `no-default-desktop-cpu`, `serde`, `svg`. `Svg`: `no-default-svg`, `svg`.
- **Observation — signature:** these types have no `Debug` implementation. This prevents applications from deriving or using ordinary diagnostic formatting for structures containing them.
- **Departure:** T1 requires “Every public type implements `Debug`.” Callback-containing representations justify a custom implementation, rather than omission.
- **Severity/confidence:** low / high.
- **Suggested fix:** add useful implementations without unnecessarily imposing `Debug` bounds on application values or callbacks. Additive, nonbreaking.
- **Destination:** TODO.md.

Runtime filenames below are relative to `crates/pert-ui-runtime/src/`; parenthesized numbers are definition lines.

| File | Instances |
|---|---|
| `breadcrumb.rs` | `Breadcrumb` (143) |
| `chip.rs` | `Chip` (72) |
| `choice_controls.rs` | `Checkbox` (647), `Toggle` (652), `RadioGroup<T>` (697), `SegmentedControl<T>` (1441), `InlineChoice<T>` (1585) |
| `collapsible.rs` | `Collapsible` (464), `Section` (809), `Accordion` (947) |
| `collection_filter.rs` | `ListFilter` (255), `FilterTarget` (353), `CollectionFilter` (499) |
| `color_picker.rs` | `ColorPicker` (386) |
| `color_swatch.rs` | `ColorSwatch` (96) |
| `command_palette.rs` | `CommandPalette` (505) |
| `containers.rs` | `Container` (63), `Row` (893), `Column` (898), `Wrap` (936), `Grid` (1027) |
| `controls.rs` | `Label` (58), `Button` (445) |
| `described.rs` | `DescribeContext<'a>` (26), `Dynamic<T>` (178), `Show` (223), `Each<T>` (277) |
| `dialog.rs` | `Dialog` (177), `AlertDialog` (493) |
| `drag.rs` | `DragPreview` (191) |
| `drawing.rs` | `Painter` (1307) |
| `field_status.rs` | `FormStatus` (216), `FieldHandle` (304), `Field` (438) |
| `file_browser/view.rs` | `FileBrowser` (70) |
| `form.rs` | `Form` (53) |
| `history_navigator.rs` | `HistoryNavigator<T>` (111), `HistoryButtons<T>` (486) |
| `image.rs` | `Image` (76) |
| `inline_controls.rs` | `InlineToggle` (54), `InlineReadout` (579) |
| `layout.rs` | `Element` (312), `MeasureContext<'a>` (706), `ArrangeContext<'a>` (968) |
| `link.rs` | `Link` (76) |
| `list_selection.rs` | `ListSelection` (62) |
| `live_widgets.rs` | `LevelMeter` (131), `Trace` (481) |
| `menu.rs` | `MenuItem` (222), `Menu` (675), `ContextMenu` (1250), `ContextMenuOpener` (1303) |
| `menu_bar.rs` | `MenuBar` (624) |
| `multi_select.rs` | `MultiSelect<T>` (1209) |
| `notifications.rs` | `Banner` (25), `Toast` (144), `ToastHost` (398) |
| `number_format.rs` | `NumberFormat<T>` (38) |
| `numeric_controls.rs` | `Slider` (1586), `InlineSlider` (1592), `Knob` (1598) |
| `popover.rs` | `Popover` (193), `Tooltip` (404) |
| `progress.rs` | `ProgressBar` (143), `Spinner` (348) |
| `property_grid.rs` | `Property` (35), `PropertyGrid` (222) |
| `quick_pick.rs` | `QuickPick` (866) |
| `region_layout.rs` | `RegionLayout` (43) |
| `runtime.rs` | `ChildInfo` (2482), `UpdateContext` (2551), `EventContext` (2897), `Runtime` (3508) |
| `runtime/fonts.rs` | `FontManager` (38), `IconManager` (162) |
| `runtime/settings.rs` | `UiSettings` (213) |
| `runtime/theme.rs` | `ThemeManager` (1014) |
| `select.rs` | `Select<T>` (1362), `InlineSelect<T>` (1740) |
| `selectable_text.rs` | `SelectableText` (108) |
| `split.rs` | `SplitPane` (198) |
| `table.rs` | `TableController` (561), `Table` (1329) |
| `table/editing.rs` | `CellEdit<'a>` (29) |
| `tabs.rs` | `Tab` (12), `Tabs` (263) |
| `text_fields.rs` | `TextField` (154), `TextArea` (203) |
| `text_fields/inline_text.rs` | `InlineText` (66) |
| `text_fields/number_inputs.rs` | `IntegerInput` (166), `DecimalInput` (203), `InlineInteger` (247), `InlineDecimal` (274) |
| `text_fields/search.rs` | `SearchField` (107) |
| `tool_windows/mod.rs` | `ToolWindow` (303), `ToolWindowLayout` (749) |
| `toolbar.rs` | `ToolGroup` (47), `Toolbar` (327) |
| `tree.rs` | `TreeController` (258), `Tree` (901) |
| `vector_scene.rs` | `VectorRasterizer` (112) |
| `views.rs` | `View` (14), `Group` (136), `ScrollArea` (329), `Divider` (906) |
| `virtual_list.rs` | `ListController` (178), `VirtualList` (439) |
| `windows.rs` | `FloatingWindow` (195), `WindowLayer` (650) |
| `zoom_view.rs` | `ZoomView` (306) |

Other defining-package instances:

- `pert_ui_wgpu::Renderer` — `crates/pert-ui-wgpu/src/lib.rs:234`.
- `pert_ui::App` — `crates/pert-ui/src/app.rs:285`.
- `pert_ui::Svg` — `crates/pert-ui/src/svg/component.rs:42`.

### codex-T1-2 Keys and classification tokens lack `Hash`

- **Rule:** guideline C-COMMON-TRAITS, applied where the documented meaning supports hashing and applications plausibly need keyed collections.
- **Configurations:** all.
- **Observation — signature:** the following immutable identities, keys and classification tokens implement equality but not `Hash`. Applications cannot directly use them in `HashMap`/`HashSet`; external implementations require wrappers.

| Instances | Source |
|---|---|
| `Revision`, `EditSequence` | `crates/pert-ui-core/src/edit.rs:12`, `:31` |
| `CommandId`, `CommandKey`, `KeyChord` | `crates/pert-ui-runtime/src/runtime/commands.rs:380`, `:7`, `:27` |
| `HeldInputId` | `crates/pert-ui-runtime/src/runtime/held.rs:8` |
| `FontId`, `FontRole` | `crates/pert-ui-runtime/src/text/fonts.rs:86`, `:99` |
| `WorkspacePath` | `crates/pert-ui-runtime/src/workspace.rs:355` |
| `KeyCode`, `Modifiers` | `crates/pert-ui-runtime/src/input.rs:11`, `:285` |
| `PointerButton` | `crates/pert-ui-runtime/src/pointer.rs:40` |
| `RequestTicket<K>` | `crates/pert-ui-runtime/src/async_source.rs:28` |
| `ColorRole`, `MetricRole`, `ThemeMode`, `Density` | `crates/pert-ui-runtime/src/runtime/theme.rs:357`, `:74`, `:14`, `:25` |
| `Icon`, `TextStyle` | `crates/pert-ui-runtime/src/text/icons.rs:9`; `crates/pert-ui-runtime/src/text.rs:123` |
| `ProfileOperation`, `ValidationCode` | `crates/pert-ui-runtime/src/profiling.rs:14`; `crates/pert-ui-runtime/src/validation.rs:114` |

Identity equality is explicitly established for `CommandId` at `commands.rs:384`, `FontId` at `fonts.rs:90`, and `RequestTicket<K>` at `async_source.rs:45`. Hashing must preserve those identity semantics, rather than hash mutable status or displayed text.

- **Severity/confidence:** low / high.
- **Suggested fix:** implement `Hash`, conditionally on `K: Hash` for tickets. Do not add arbitrary ordering merely for symmetry. Additive, nonbreaking.
- **Destination:** TODO.md.

### codex-T1-3 Exact values expose incomplete equality and ordering traits

- **Rule:** guideline C-COMMON-TRAITS.
- **Configurations:** all.
- **Observation — signature:** representations support the following traits, useful for application state derives, preset sorting and caches, but the public implementations omit them.

| Values | Missing traits | Evidence |
|---|---|---|
| `Capacity` | `PartialOrd`, `Ord` | Validated `usize`; `crates/pert-ui-core/src/live/mod.rs:183` |
| `PowerExponent` | `Ord`, `Hash` | Finite positive scalar, already `Eq`; `crates/pert-ui-core/src/parameter.rs:63`, validation at `:76` |
| `Magnification`, `UiZoom`, `ViewScale` | `Eq`, `Ord`, `Hash` | Finite positive scalars; `crates/pert-ui-runtime/src/pointer.rs:106`, `runtime/settings.rs:22`, `transform.rs:267` |
| `PresentationRate` | `Eq`, `PartialOrd`, `Ord`, `Hash` | Finite scalar in `1..=1000`; `crates/pert-ui-runtime/src/runtime/presentation.rs:10`, `:17` |
| `MaskImage`, `RasterImage` | `Eq`, `Hash` | Equality compares integer dimensions and immutable bytes; `crates/pert-ui-runtime/src/drawing.rs:1121`, `resources.rs:220` |

The following exact-data records also omit `Eq`, conditionally on `T: Eq` where generic:

- Core: `EditOutcome<T>`, `ParameterSnapshot<T>` — `crates/pert-ui-core/src/edit.rs:163`, `:116`; `Sample<T>`, `SampleWindowEntry<T>`, `SampleWindow<T>` — `crates/pert-ui-core/src/live/mod.rs:286`, `:367`, `:382`.
- Runtime generic records: `RemoteCommit<T>`, `AuthorityResponse<T>`, `AuthorityStatus<T>` — `crates/pert-ui-runtime/src/parameter_binding/remote.rs:12`, `:40`, `:49`; `TerminalEvent<T>`, `PreviewEvent<T>`, `DeliverySnapshot<T>` — `parameter_binding/delivery.rs:130`, `:156`, `:206`; `Telemetry<T>` — `live.rs:79`; `InspectionReply<T>` — `inspection.rs:448`.
- Runtime concrete records: `ListSelectionChange` — `list_selection.rs:86`; `WorkspaceReport` — `workspace.rs:746`; `EventHistory` — `inspection.rs:339`; `ProfileStats`, `ProfileSample`, `ProfileSnapshot` — `profiling.rs:89`, `:114`, `:137`; `SourceInspection` — `live.rs:281`; `PresentationStatus` — `runtime/native_windows.rs:919`.

These records contain exact integer/string/duration/identity data and equality-capable error carriers, or generic payloads for which a conditional implementation applies. This does not require `Eq` for raw floating-point geometry or arbitrary floating-point payloads.

- **Severity/confidence:** low / high.
- **Suggested fix:** add the supported traits, retaining appropriate generic bounds. Follow `NormalizedValue`’s treatment of validated floats (`crates/pert-ui-core/src/normalized.rs:49`). Additive, nonbreaking.
- **Destination:** TODO.md.

### codex-T3-1 Evolving inspection records remain exhaustively constructible

- **Rule:** consistency proposal beyond error enums; **policy question**.
- **Paths/configurations:** `ComponentInspection`, `InspectionSnapshot`, `ValidationCode`; all.
- **Observation — signature:** these lack `#[non_exhaustive]`, although their documentation records schema growth:
  - `ComponentInspection::path` and `view_scale` record additions — `crates/pert-ui-runtime/src/diagnostics.rs:677`, `:688`.
  - `InspectionSnapshot` records added sections and explicitly defines revisions for added, removed or reinterpreted fields — `diagnostics.rs:849`, `:895`.
  - `ValidationCode::UnboundedCollection` records its addition — `crates/pert-ui-runtime/src/validation.rs:143`.
- **Departure:** T3 proposes non-exhaustiveness for public enums and field-bearing structs expected to grow. These are concrete candidates because their documented evolution already changes their shapes. No error-enum rule is violated.
- **Severity/confidence:** low / medium for the forward compatibility policy; high for the observed signatures.
- **Suggested fix:** consider `#[non_exhaustive]`, with constructors/defaults where consumers need to create records. This is breaking for exhaustive matches and struct literals. The maintainer must decide whether these records instead intentionally track schema-breaking releases.
- **Destination:** TODO.md.

### codex-T4-1 Family-mismatch results use values that are not errors

- **Rule:** guideline C-GOOD-ERR and T4’s requirement that every public `Result` error type implement `std::error::Error`.
- **Paths/configurations:** `AnyValue`, `AnySpec`, `ParameterValue::{from_any,spec_from_any}` and corresponding `TryFrom` implementations; all.
- **Observation — signature:** `ParameterValue` returns the rejected `AnyValue` or `AnySpec` directly (`crates/pert-ui-core/src/family.rs:61`, `:71`). The conversion implementations preserve that choice (`crates/pert-ui-core/src/erased.rs:86`, `:143`). Neither carrier implements `Error` or `Display` (`erased.rs:24`, `:112`).
- **Effect:** these recommended conversions cannot propagate directly into ordinary `Box<dyn Error>` error paths with `?`.
- **Exception check:** the rustdoc deliberately promises to return the input unchanged. That explains recovery ownership; it does not establish an exception to the error-trait requirement.
- **Severity/confidence:** medium / high.
- **Suggested fix:** introduce a typed family-mismatch error retaining the rejected input and expected/found families. Changing the associated error types is breaking.
- **Destination:** TODO.md.

### codex-T4-2 Library error carriers omit supported common traits

- **Rule:** project rule, `docs/contracts.md:1113`: “Library-owned errors derive `Clone`, `Debug`, `PartialEq` and `Eq` where their data permits”.
- **Configurations:** all.
- **Observation — signature:**
  - `CpuRenderError` derives `Clone`, `Debug`, `PartialEq`, but not `Eq`; all fields are `Eq` (`crates/pert-ui-cpu/src/lib.rs:14`).
  - `RedescriptionError` derives only `Debug` and `Error`, despite cloneable and partially comparable fields (`crates/pert-ui-runtime/src/conformance.rs:12`). `ComponentDetails` prevents deriving `Eq`, but permits `Clone` and `PartialEq` (`diagnostics.rs:646`).
- **Exception check:** neither retains an I/O source. The I/O exception properly applies elsewhere, including `ImageError` and `AppError`.
- **Severity/confidence:** low / high.
- **Suggested fix:** add `Eq` to `CpuRenderError`, and `Clone`/`PartialEq` to `RedescriptionError`. Additive, nonbreaking.
- **Destination:** TODO.md.

### codex-T4-3 Workspace error chains omit or repeat typed causes

- **Rule:** project rule, `docs/contracts.md:1115`: “A wrapped error is transparent or exposed through `source()`, so a chain prints each cause once.”
- **Configurations:** all.
- **Observation — behavior:**
  - `RestoreRejection::State(WorkspaceError)` and `RestoreRejection::Runtime(RuntimeError)` interpolate their causes into `Display`, but supply neither `#[source]`, `#[from]` nor transparent forwarding. Consequently their `Error::source()` is absent (`crates/pert-ui-runtime/src/workspace.rs:702`, `:712`).
  - `WorkspaceError::Participant` interpolates `{source}` into its own message and also exposes that same error through `#[source]` (`workspace.rs:643`, `:650`). A chain reporter repeats the cause.
- **Exception check:** these are library causes, not foreign diagnostics. The callback-boundary conversion exception at `docs/contracts.md:1125` does not change source-chain requirements.
- **Severity/confidence:** medium / high.
- **Suggested fix:** expose the two restoration causes and let contextual wrapper messages omit the cause text; likewise remove the embedded source text from `Participant`. Nonbreaking type signatures; observable diagnostic behavior changes.
- **Destination:** TODO.md.

### codex-T4-4 Validation and identity diagnostics omit rejected-value context

- **Rule:** T4/C-GOOD-ERR: `Display` “says what failed, with which value and, where the caller can act, what to do”.
- **Configurations:** all.
- **Observation — behavior:** several error families retain only a category, so their display cannot identify the rejected input. Verified instances include:

| Instances | Source |
|---|---|
| `RuntimeError::AmbiguousKey`, `DuplicateKey`, `ChildTypeChanged` | `crates/pert-ui-runtime/src/runtime.rs:319`, `:311`, `:365` |
| `LayoutError::{DuplicateKey,ReservedKey,UnknownKey,InvalidWeight,InvalidRatio}` | `crates/pert-ui-runtime/src/layout.rs:25`, `:29`, `:33`, `:43`, `:46` |
| `ParameterError::{InvalidRange,OutOfRange,OffLattice,DuplicateOption,UnknownOption}` | `crates/pert-ui-core/src/parameter.rs:550`, `:553`, `:566`, `:572`, `:575` |
| `LiveError::CapacityExceeded` | `crates/pert-ui-core/src/live/mod.rs:168` |
| `NormalizedValueError::OutOfRange` | `crates/pert-ui-core/src/normalized.rs:119` |
| `GeometryError::{InvertedBounds,ViewScaleOutOfRange,ZoomOutOfRange}` | `crates/pert-ui-runtime/src/layout.rs:92`, `:102`, `:109` |
| `NumberInputError::{InvalidRange,InvalidStep,InvalidPrecision,OutOfRange,OffStep}` | `crates/pert-ui-runtime/src/text_fields/number_inputs.rs:16`, `:19`, `:22`, `:31`, `:34` |
| `DrawError::InvalidImage`, `PathError::InvalidParameter` | `crates/pert-ui-runtime/src/drawing.rs:30`; `geometry.rs:21` |
| `FontError::{MissingGlyph,UnknownIcon}`, `NumberLocaleError::SameSeparators` | `crates/pert-ui-runtime/src/text/fonts.rs:135`, `:138`; `number_format.rs:155` |
| `TextEditError::{InvalidComposition,InvalidSelection}` | `crates/pert-ui-runtime/src/text_input.rs:21`, `:25` |

F23 is confirmed: `find_descendant` has the queried key available but returns the unit `AmbiguousKey` (`crates/pert-ui-runtime/src/runtime/tree.rs:242`, `:254`). Its message is only “several descendants share the key”.

- **Exception check:** “Errors carry no location” (`docs/contracts.md:1133`) does not require discarding a rejected argument. This finding asks for input context, not component paths.
- **Severity/confidence:** medium / high.
- **Suggested fix:** retain useful rejected keys, indices, numbers and bounds. For generic choices, conflicting positions can provide context without adding formatting bounds to application values. Changing variant payloads is breaking.
- **Destination:** TODO.md.

### codex-T4-5 One frame-byte limit has two variants at the same reporting level

- **Rule:** project rule, `docs/contracts.md:1141`: “One limit has one reporting variant across paths that share the limit.”
- **Paths/configurations:** `LimitError::{CaptureSize,FrameSize}`; declarations in all configurations. Software presentation producers occur in the facade’s CPU configurations.
- **Observation — behavior:** both variants explicitly represent `FrameImage::MAX_BYTES` (`crates/pert-ui-runtime/src/render_limits.rs:58`, `:97`).
  - GPU offscreen rendering returns `CaptureSize` for that bound (`crates/pert-ui-wgpu/src/lib.rs:678`).
  - Software window presentation returns `FrameSize` for the same bound (`crates/pert-ui/src/platform/presenter.rs:250`, `:254`).
- **Exception check:** the rustdoc distinguishes operation context, but the policy states no operation-context exception to the shared-limit rule.
- **Severity/confidence:** low / high.
- **Suggested fix:** consolidate the limit variant and preserve operation context separately if useful; alternatively adopt an explicit policy exception. Consolidation is breaking.
- **Destination:** TODO.md.

### codex-T4-6 Fallible conversion implementations lack `# Errors` sections

- **Rule:** project rule, `docs/contracts.md:1142`: “Every public `Result` has an `# Errors` rustdoc section naming the variants and conditions, including forwarded failures.”
- **Configurations:** all.
- **Observation — signature/documentation:** 15 of the 17 additional fallible conversion methods have no own `# Errors` section:

| Methods | Source |
|---|---|
| `ChoiceId`’s `TryFrom<&str>` and `TryFrom<String>` | `crates/pert-ui-core/src/choice.rs:64`, `:72` |
| `ParameterSpec<T>::try_from(AnySpec)` | `crates/pert-ui-core/src/erased.rs:90` |
| `TryFrom<AnyValue>` for `f32`, `i64`, `bool`, `ChoiceId` | Macro body at `crates/pert-ui-core/src/erased.rs:143`; invocations `:153`, `:154`, `:155`, `:156` |
| `Capacity::try_from` | `crates/pert-ui-core/src/live/mod.rs:214` |
| `PowerExponent::try_from` | `crates/pert-ui-core/src/parameter.rs:92` |
| `Length::try_from` | `crates/pert-ui-runtime/src/containers.rs:41` |
| `Magnification::try_from` | `crates/pert-ui-runtime/src/pointer.rs:134` |
| `PresentationRate::try_from` | `crates/pert-ui-runtime/src/runtime/presentation.rs:34` |
| `UiZoom::try_from` | `crates/pert-ui-runtime/src/runtime/settings.rs:77` |
| `ViewScale::try_from` | `crates/pert-ui-runtime/src/transform.rs:309` |
| `Color::from_str` | `crates/pert-ui-runtime/src/drawing.rs:747` |

`NormalizedValue::try_from` and `OpaqueColor::try_from` already provide sections.

- **Policy question:** if standard-trait implementations are intended to inherit constructor documentation instead, that exception needs stating. The current rule says “Every”.
- **Severity/confidence:** low / high.
- **Suggested fix:** add sections linking to the complete validation contract, including macro-generated implementations. Nonbreaking.
- **Destination:** TODO.md.

### codex-T4-7 Existing `# Errors` sections omit actual failure conditions

- **Rule:** project rule, `docs/contracts.md:1142`, including forwarded failures.
- **Configurations:** all.
- **Observation — behavior/documentation:** the inventory’s “yes” establishes a heading, not semantic coverage. Verified omissions include:

| Methods | Omission and evidence |
|---|---|
| `Runtime::mailbox` | Documents only `Closed`, but calls `writable()`, which also returns every phase-write error: `crates/pert-ui-runtime/src/runtime.rs:4109`, `:4116`; guard at `:691`. |
| `Runtime::inspection_service`, `set_viewport` | List update/paint/inspection restrictions but omit layout and delivery: `runtime.rs:4135`, `:4141`, `:3926`, `:3933`. |
| `Runtime::set_visible` | Documents only foreign/unmounted IDs; window selection and the selected implementation also reject closed/forbidden phases: `runtime.rs:5019`, `:5023`, `:5030`. |
| `ListController::reveal`, `TreeController::reveal`, `TableController::reveal` | Their enumerations omit layout, paint, inspection and delivery write errors forwarded from `Signal::set`: `virtual_list.rs:289`, `:301`; `tree.rs:538`; `table.rs:850`. |
| `ZoomController::{set,set_changed,fit}` | The root error enumeration likewise omits phase restrictions: `zoom_view.rs:156`, `:159`, `:167`, `:180`. |
| `ListSelection::{set_active,select,clear}` | The root section omits layout, delivery, shutdown and borrow failures: `list_selection.rs:163`, `:165`, `:182`, `:197`. |
| `TreeController::{set_expanded,toggle,set_source,filter}` | The root section omits layout/delivery/shutdown restrictions enforced by `writable()`: `tree.rs:342`, `:343`, `:382`, `:390`, `:468`. |
| `TableController::{set_source,set_sort,filter,set_active_column}` | Incomplete signal-write descriptions: `table.rs:639`, `:640`, `:707`, `:742`, `:757`. |
| `UiSettings::{set_motion,set_debug_overlay}` | Omit delivery refusal despite calling `writable()`: `runtime/settings.rs:354`, `:356`, `:455`, `:457`. |
| `EditSession::update` | Omits `RuntimeError::Binding(EditError::Pending)` after remote submission: `parameter_binding.rs:740`, `:750`, `:752`. |

Other sections give categories without naming the corresponding variants or linking to a complete operation contract, notably `ViewState::{new,with}`, `WorkspacePath::new`, `WorkspaceValue::text`, and `Renderer::recreate` (`crates/pert-ui-runtime/src/workspace.rs:235`, `:246`, `:360`, `:102`; `crates/pert-ui-wgpu/src/lib.rs:502`).

- **Severity/confidence:** medium / high.
- **Suggested fix:** document the complete root error contracts and link forwarding methods to them. Nonbreaking.
- **Destination:** TODO.md.

### codex-T4-8 User-facing message carriers conflict with the diagnostic style rule

- **Rule:** project rule, `docs/contracts.md:1115`; **policy question**.
- **Paths/configurations:** `InputRejection`, `ListingError`; all.
- **Observation — behavior:** both display application-supplied user-facing messages verbatim. `InputRejection` explicitly demonstrates and asserts uppercase output, “Name must not be empty” (`crates/pert-ui-runtime/src/text_fields/normalize.rs:18`, `:20`, `:23`). `ListingError` similarly accepts and displays an unrestricted user-visible message (`crates/pert-ui-runtime/src/file_browser.rs:76`, `:83`).
- **Departure:** the general rule requires lowercase messages without trailing periods. These APIs instead preserve presentation text.
- **Exception check:** editor-session policy says the editor “performs its own save, normalization and refusal” (`docs/contracts.md:938`); it explains rejection semantics, not message-style exemption.
- **Severity/confidence:** low / high.
- **Suggested fix:** decide whether user-facing message carriers are exempt, and document that exception. Automatically rewriting application prose would change behavior and should not be inferred as the fix.
- **Destination:** TODO.md.

F11 does **not** establish a conversion-policy departure: `docs/contracts.md:1131` explicitly excludes a conversion from `DrawError` back into `RuntimeError`. Its diagnostic ergonomics remain the separate survey’s concern.

F30 is **not current**: `ApplicationError` already implements `From<Box<dyn Error + Send + Sync + 'static>>`, retaining the boxed error (`crates/pert-ui-runtime/src/error.rs:79`). The generic `new` constructor’s limitation therefore no longer forces source loss.

### codex-T5-1 Public semantic fields bypass documented invariants

- **Rule:** guideline C-STRUCT-PRIVATE and the project’s private-invariant rule.
- **Paths/configurations:** `Semantics`, `SemanticItem`; all.
- **Observation — signature and behavior:**
  - `Semantics::actions` promises sorted, duplicate-free actions; a disabled component offers none (`crates/pert-ui-runtime/src/diagnostics.rs:476`, `:481`). Builders enforce this at `:558` and `:582`, but public assignments bypass it.
  - `protected` promises that `value` is absent (`diagnostics.rs:494`). The builders enforce this at `:546` and `:624`, but callers can subsequently assign `value`, or set the public `protected` field after assigning a value.
  - `SemanticItem` exposes the same disabled/action-order invariants and a never-empty optional name through public fields (`diagnostics.rs:350`, `:352`, `:356`, `:367`). Its builders enforce restrictions that field writes bypass (`:390`, `:424`).
  - Inspection takes the component’s returned semantics directly; it does not repair the protected-value relationship (`crates/pert-ui-runtime/src/runtime/inspect.rs:145`, `:152`).
- **Departure:** these are more than plain unconstrained records: their documented relationships are enforced by some construction paths and bypassable through others.
- **Severity/confidence:** high for protected-value exposure; low for action/name inconsistencies / high confidence.
- **Suggested fix:** choose an invariant-bearing API with private fields and accessors, including validated deserialization, or explicitly define raw descriptions and enforce their relationships at the consumption boundary. Privatizing fields is breaking.
- **Destination:** TODO.md.

This does not apply to `PickItem`: its documentation explicitly makes it plain data and assigns key validation to `QuickPickController::open` (`crates/pert-ui-runtime/src/quick_pick.rs:11`).

### codex-T6-1 Common wrapper conversions lack standard trait forms

- **Rule:** guideline C-CONV-TRAITS, for common, unambiguous conversions.
- **Configurations:** all.
- **Observation — signature:**
  - `ChoiceId` exposes its underlying string through `as_str`, but lacks `AsRef<str>` (`crates/pert-ui-core/src/choice.rs:32`, `:50`). Passing it to ordinary `AsRef<str>` APIs requires explicit extraction.
  - `FontData::from_owned(Vec<u8>)` and `from_static(&'static [u8])` are direct ownership/borrowing conversions, but there are no corresponding `From` implementations (`crates/pert-ui-runtime/src/text/fonts.rs:43`, `:51`, `:64`).
- **Qualification:** these named constructors are legitimate. The finding concerns missing standard trait forms, not their names. No `AsMut` is proposed for invariant-bearing or immutable representations.
- **Severity/confidence:** low / high.
- **Suggested fix:** add `AsRef<str>` for `ChoiceId` and the two lifetime-appropriate `From` implementations for `FontData`, retaining the named APIs. Additive, nonbreaking.
- **Destination:** TODO.md.

## Inventory gaps

1. **Conversion direction and target types are misrepresented.**
   `conversion-traits.txt:19` and `types.txt:23` attribute `From<ParameterError>` to `ParameterError`. The implementation is actually **for `EditError`**, generated by its variant at `crates/pert-ui-core/src/edit.rs:223`. Likewise, `conversion-traits.txt:13` attributes `From<ChoiceId>` to `ChoiceId`, although the implementation targets `AnyValue` (`erased.rs:143`, `:156`). The lists collect associated impls without recording/filtering their actual `for` type (`inventory.py:442`, `:452`).

2. **Distinct conversions collapse into indistinguishable entries.**
   `conversion-traits.txt:11` gives one `AnyValue / TryFrom<AnyValue>` row. The source defines conversions to `f32`, `i64`, `bool` and `ChoiceId` (`crates/pert-ui-core/src/erased.rs:143`, `:153`). The missing target dimension prevents exhaustive conversion review from that row alone.

3. **Fallible trait-implementation methods are absent from the error and semantic method inventories.**
   The 17 `TryFrom`/`FromStr` methods reviewed above are omitted, including `NormalizedValue::try_from` (`crates/pert-ui-core/src/normalized.rs:97`) and `Color::from_str` (`crates/pert-ui-runtime/src/drawing.rs:747`). `inventory.py:447` collects methods only for inherent implementations, then separately handles trait declarations at `:462`. This omission hides both working and missing `# Errors` sections.

4. **Configuration merging loses conditional trait implementations.**
   `types.txt:21` does not list serde traits for `NormalizedValue`, although the committed serde API contains `Serialize` and `Deserialize` at `api/pert-ui-core/serde.txt:614` and `:616`. `inventory.py:432` retains the first row text while accumulating configuration names, so later trait differences disappear. I independently checked all 17 JSON configurations for the common and auto traits relevant here; those traits did not differ.

These inventory issues should be corrected before cross-verification. They were not treated as library API defects.

## Roles relied on

The classifications below group types with the same relevant responsibility; definition references for individual finding instances are listed above.

| Path or group | Role | Source reference |
|---|---|---|
| `ChoiceId`, `ParameterId`, `EditToken`, `CommandId`, `HeldInputId`, `FontId`, `NativeWindowId`, `WidgetId`, `RequestTicket<K>`, `WorkspacePath` | Stable identities or keys; equality is not presentation order or current status | `crates/pert-ui-core/src/choice.rs:32`, `parameter_id.rs:44`, `edit.rs:82`; runtime `runtime/commands.rs:377`, `runtime/held.rs:5`, `text/fonts.rs:75`, `runtime/native_windows.rs:160`, `runtime.rs:140`, `async_source.rs:18`, `workspace.rs:342` |
| `Revision`, `EditSequence` | Scoped protocol value tokens, not global identities | `crates/pert-ui-core/src/edit.rs:5`, `:29` |
| Theme/font/icon/key/profiling/validation tokens in T1-2 | Immutable categories or dispatch keys suitable for application lookup tables | Definitions cited in T1-2 |
| `Capacity`, `PowerExponent`, `Magnification`, `UiZoom`, `ViewScale`, `PresentationRate` | Validated scalar values with finite, ordered domains | Constructors cited in T1-3 |
| Records listed under T1-3 | Passive exact-data snapshots, reports, proposals or outcomes; generic equality follows the payload bound | Field definitions cited in T1-3 |
| `MaskImage`, `RasterImage`, `FontData`, `Path`, `DisplayList` | Owned immutable resources or scenes, independent of live UI handles | Runtime `drawing.rs:1115`, `resources.rs:147`, `text/fonts.rs:43`, `geometry.rs:184`, `drawing.rs:1200` |
| `Semantics`, `SemanticItem` | Structured semantic descriptions with documented cross-field invariants | `crates/pert-ui-runtime/src/diagnostics.rs:350`, `:464` |
| `PickItem`, `CommandContext` | Plain request/description data validated at adoption or invocation | Runtime `quick_pick.rs:11`; `runtime/commands.rs:363` |
| `AnyValue`, `AnySpec` | Tagged values/specifications also used as mismatch-result carriers | `crates/pert-ui-core/src/erased.rs:24`, `:112`; `family.rs:61`, `:71` |
| `ApplicationError` | Shared typed application failure; equality is shared error identity | `crates/pert-ui-runtime/src/error.rs:7`, `:86` |
| Other 45 error implementations | Focused validation, operational, transport or aggregate error carriers; I/O-bearing exceptions assessed separately | Defining error declarations from all 46 `errors.txt` implementation entries |
| `SeriesEndpoint<T>` | Movable exclusive producer endpoint; intentionally `Send`, not `Sync` | `crates/pert-ui-core/src/live/series.rs:130`, `:138` |
| `SeriesConsumer<T>`, `WakeFlag` | Cross-thread-capable core consumer state and shared wake coordination | Core `live/series.rs:283`; `live/wake.rs:24` |
| `InspectionClient`, `Pending<T>`, `UiSender<T>`, `LiveWaker`, `DirectoryRequest`, `DirectoryResponse`, `RemoteCommit<T>` | Cross-thread request, reply, delivery or proposal data; generic bounds retained | Runtime `inspection.rs:569`, `:726`; `mailbox.rs:81`; `live.rs:156`; `file_browser.rs:196`, `:228`; `parameter_binding/remote.rs:5` |
| `Runtime`, contexts, UI controllers, bindings, owners, and records retaining UI identities | Runtime-affine capabilities/data; neither `Send` nor `Sync` | Representative ownership contracts: runtime `runtime.rs:3508`, `signal.rs:200`, `parameter_binding.rs:289`, `async_source.rs:108`, `text_fields/draft.rs:135` |
| `Divider`, `Svg` | Movable component descriptions with exclusive interior caches, no live runtime handle; `Send` only is consistent with that representation | Runtime `views.rs:906`; `crates/pert-ui/src/svg/component.rs:42` |
| `CpuRenderer`, `VectorRasterizer` | Owned renderer/cache state without UI affinity | `crates/pert-ui-cpu/src/lib.rs:61`; runtime `vector_scene.rs:112` |
| `Renderer`, `App` | Native/backend owners retaining thread-local state | `crates/pert-ui-wgpu/src/lib.rs:234`; `crates/pert-ui/src/app.rs:285` |
| Eleven public traits | Interfaces, not missing concrete `Debug` implementations | Their defining entries in the semantic/type lists |
| `HostValue::Error` | Application-defined associated error; `Error` bound established, concrete variants intentionally unresolved | `crates/pert-ui-core/src/erased.rs:408`, `:412` |