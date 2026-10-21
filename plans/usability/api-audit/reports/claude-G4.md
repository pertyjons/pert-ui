# claude report: G4 (types: T1, T2, T3, T4, T5, T6)

Commit audited: d4ba108 (crates/ unchanged since; inputs frozen in plans/usability/api-audit/inputs/).

## Dispositions

| Item | Disposition | Coverage |
|---|---|---|
| T1 common traits | findings: claude-T1-1, T1-2, T1-3 | candidates/types.txt: all 534 entries (523 types, 11 traits) tabulated for `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Default`, `Display`; every hand-written `Debug` impl in the five packages read (56 public, found by `rg 'Debug for'`; none prints an empty string; nine print only `Name { .. }`: `FilterMatcher`, `CommandMatcher`, `PickMatcher`, `PickPending`, `ZoomController`, `Signal`, `TreeSource`, `CpuRenderer`, `DraftEditorController`, which C-DEBUG-NONEMPTY permits); source of the identity, key and role types read. `Default`/`Ord` gaps were spot-checked only (no gap a consumer plausibly needs was found). |
| T2 Send and Sync | findings: claude-T2-1, T2-2 | Auto-trait column of all 523 types in candidates/types.txt, conditional bounds of the 56 generic types checked; rustdoc thread statements read for every `Send`-differing outlier (`Divider`, `Svg`, `SeriesEndpoint`, `Pending`, `UiSender`, `RichText`, `FontId`, `pert_ui_wgpu::Renderer`, `CpuRenderer`, `VectorRasterizer`). UI-thread handles, controllers, widgets and contexts are `!Send + !Sync`; data DTOs, errors and cross-thread endpoints are `Send`; `Sync` appears only where shared references cross (`InspectionClient`, `LiveWaker`, `UiSender`, `Pending`, `WakeFlag`). |
| T3 `#[non_exhaustive]` | finding (proposal): claude-T3-1; project rule: no findings | candidates/non-exhaustive.txt: all 251 entries (196 enums, 55 structs with public fields). Every error enum is `#[non_exhaustive]` except `SendError<T>` (mailbox.rs:20-24) and `SinkErrorKind` (parameter_binding/delivery.rs:50-53), both of which state in rustdoc that the set is closed: the rule is met. The two entries flagged as both `#[non_exhaustive]` and "closed" (`EditOutcome`, `BindingError`) were read: the word matched an explanation, not a contradiction. |
| T4 errors | findings: claude-T4-1 … T4-11 (T4-7, T4-9, T4-10 carry policy questions) | candidates/errors.txt: all 540 function entries (their `# Errors` texts extracted from the source and read; macro-generated ones read in the macro bodies) and all 46 `std::error::Error` impls; for each error type the derives, every `#[error]` attribute and hand-written `Display`, `source()` behaviour and `From` impls (candidates/conversion-traits.txt) were read. Slice 3's F11 (T4-7), F23 (T4-6) and F30 (T4-8) re-examined. All `Display` texts begin lowercase without a trailing period except the user-message types in T4-10. Application failures: `ApplicationError` keeps the source for downcasting (error.rs:46-112); no library variant standing in for a host failure was found. |
| T5 public fields | finding: claude-T5-1 | candidates/fields.txt: all 286 fields of the 55 structs. Other structs are library-produced snapshots, events and reports, or plain input records (`Modifiers`, `KeyEvent`, `CommandKeyInput`, `ShortcutBinding`, `TextInputState`, `PickItem`, `TableSort`, `ValidationHints`, `ComponentDetails`, `WindowPlacement`) without a cross-field invariant; their values are checked where adopted (for example `QuickPickController::open` rejects blank or duplicate `PickItem` keys, quick_pick.rs:480). Out of this group's scope, noted for C4: `TableController::set_sort` (table.rs:742) stores a `TableSort` naming any column without the `UnknownKey` check `set_active_column` (table.rs:757) applies. `ParameterSnapshot<T>` documents domain invariants on public fields, but it is passive captured data and `AnyParameterSnapshot::try_new` (erased.rs:239) checks them where a snapshot is adopted from elsewhere; not a finding. |
| T6 conversion traits | finding: claude-T6-1; proposal part: no findings | candidates/conversion-traits.txt: all 198 entries; candidates/conversion-methods.txt: all 24 entries; constructor list scanned for single-argument fallible constructors (every validated single-value type with one has `TryFrom`: `Capacity`, `ChoiceId`, `PowerExponent`, `NormalizedValue`, `Length`, `Magnification`, `PresentationRate`, `UiZoom`, `ViewScale`). |

## Findings

### claude-T1-1 Many public types implement no `Debug`, including state handles applications store

- Rubric item T1; guideline C-DEBUG ("all public types implement Debug").
- Paths and configurations (all configurations where the type exists; `pert_ui::App`
  in every desktop configuration, `pert_ui::Svg` in `svg` and `no-default-svg`):
  113 of the 523 non-trait types in candidates/types.txt have no `Debug`
  (signature). Grouped:
  - **Cloneable state owners and handles an application keeps in its own structs**
    (the most costly: a `#[derive(Debug)]` on an app-state struct fails):
    `ListSelection`, `ListController`, `ListFilter`, `TableController`,
    `TreeController`, `FieldHandle`, `FormStatus`, `ContextMenuOpener`,
    `ThemeManager`, `FontManager`, `IconManager`, `UiSettings`,
    `NumberFormat<T>`, `FilterTarget`, `ChildInfo`, `Runtime`,
    `VectorRasterizer`, `pert_ui_wgpu::Renderer`, `pert_ui::App`.
    Sibling handles of the same role do implement it by hand
    (`DialogController` dialog.rs:55, `QuickPickController` quick_pick.rs:438,
    `ZoomController` zoom_view.rs:191, `ToolWindowController`
    tool_windows/mod.rs:2283, `DraftEditorController` text_fields/draft.rs:165,
    `CommandPaletteController` command_palette.rs:214, `Signal`/`ReadSignal`
    signal.rs:200/207), so the gap is inconsistent within the controller role.
  - **Widget and container builders** (`Button`, `Label`, `Slider`, `Table`,
    `Tree`, `VirtualList`, `Column`, `Row`, `Container`, `Divider`, … 80-odd
    `#[must_use]` description types, and `Dynamic<T>`, `Each<T>`, `Element`,
    `pert_ui::Svg`).
  - **Callback contexts** (`EventContext`, `UpdateContext`, `MeasureContext`,
    `ArrangeContext`, `DescribeContext`, `CellEdit`, `Painter`).
- Rule: C-DEBUG; the rubric's T1 "Every public type implements `Debug`".
- Severity: low for builders and contexts (rarely stored), medium for the
  state-owner group (forces hand-written `Debug` or wrapper types in every
  application struct that holds one).
- Confidence: high (listing; the `missing_debug_implementations` lint survey
  will enumerate the same set).
- Fix: derive or hand-write `Debug` for the state-owner group first (a
  `finish_non_exhaustive` form like `ZoomController`'s is enough); decide in the
  lint survey whether builders and contexts get it too. Non-breaking.
- Destination: TODO.md.

### claude-T1-2 Identity and key types without `Hash`, inconsistent with their siblings

- Rubric item T1; guideline C-COMMON-TRAITS (the rubric's own example: "a key or
  identity type without `Hash`").
- Paths (all configurations), signature:
  - Identities: `CommandId` (runtime/commands.rs:380, `Clone, Debug` +
    hand-written `PartialEq`/`Eq` on serial and `Rc` pointer), `FontId`
    (text/fonts.rs:86, same pattern), `HeldInputId` (runtime/held.rs:8,
    `derive(Clone, Debug, PartialEq, Eq)` over `(WidgetId, u64)`, both of which
    are `Hash`). Siblings with the same `Rc`-identity design implement `Hash`:
    `WidgetId`, `NativeWindowId`; plain identities `ImageId`, `MaskId`,
    `VectorId`, `HistoryEntryId`, `ParameterId`, `EditToken` all do.
  - Keys: `KeyCode` (input.rs:11, `Ord` but no `Hash`), `Modifiers`
    (input.rs:285), `CommandKey` (runtime/commands.rs:7) and therefore
    `KeyChord` (runtime/commands.rs:27), `PointerButton` (pointer.rs:40),
    `WorkspacePath` (workspace.rs:355, `Ord`, no `Hash`).
  - Role enums used as table keys: `ColorRole`, `MetricRole`, `FontRole` and
    `Icon` lack `Hash` while `StringRole` has it.
- Rule: C-COMMON-TRAITS; T1 "a key or identity type without `Hash`" is the named
  finding shape. An application mapping chords to its own actions, or command
  ids to its own data, cannot use a `HashMap`; `KeyChord` has no `Ord` either,
  so not even a `BTreeMap` works for it.
- Severity: low (workaround: a `BTreeMap` where `Ord` exists, or a wrapper).
- Confidence: high.
- Fix: `Hash` consistent with the existing `Eq` (hash the serial for the
  `Rc`-identity types, derive for the rest). Non-breaking.
- Destination: TODO.md.

### claude-T1-3 `Color` parses from text (`FromStr`) but has no `Display`

- Rubric item T1 (C-COMMON-TRAITS "user-facing values implement Display"); low.
- `pert_ui_runtime::Color` (drawing.rs:210), all configurations, signature:
  `impl FromStr` (drawing.rs:744) and `Color::to_hex` (drawing.rs:479), but no
  `Display`, so `format!("{color}")` and `to_string()` do not round-trip what
  `parse` accepts. A color is user-facing in `ColorPicker`'s hex field.
- Severity low; confidence high; fix: `Display` writing `to_hex()`'s text,
  non-breaking. Destination: TODO.md.

### claude-T4-1 `# Errors` sections understate the write-phase errors `writable()` returns

- Rubric item T4 (Documentation); project rule, contracts "Error policy":
  "Every public `Result` has an `# Errors` rustdoc section naming the variants
  and conditions, including forwarded failures."
- **Behavior.** `Signal::set_changed` (signal.rs:474-476) and the settings,
  viewport and inspection writers below call `Shared::writable`
  (pert-ui-runtime/src/runtime.rs:691-711), which refuses `Closed`,
  `WriteDuringUpdate`, `WriteDuringPaint`, `WriteDuringLayout`,
  `WriteDuringInspection` and `WriteDuringDelivery`. These sections list a
  narrower set, so a consumer reading them expects a write from layout or a
  parameter delivery callback to succeed:
  - "update, paint or inspection" (omit layout and delivery):
    `ListSelection::set_active` (list_selection.rs:165, and `select`/`clear`
    which point to it), `TreeController::set_expanded` (tree.rs:343, and
    `toggle`/`set_source`/`filter`), `TableController::set_source`
    (table.rs:640, and `set_sort`/`filter`), `TableController::set_active_column`
    (table.rs:757), `FieldHandle::set` (field_status.rs:323, and `clear`,
    `set_changed`, `set_modified`), `FieldHandle::remove` (field_status.rs:379),
    `Runtime::set_viewport` (runtime.rs:3928, calls `writable()` at 3933),
    `Runtime::inspection_service` (runtime.rs:4137, `writable()` at 4141).
  - "update, layout, paint or inspection" (omits delivery):
    `UiSettings::set_motion` (runtime/settings.rs:355, `writable()` at 356),
    `UiSettings::set_debug_overlay` (runtime/settings.rs:456, `writable()` at 457).
  Siblings document the full set (`Signal::set` signal.rs:466,
  `UiSettings::set_strings` settings.rs:415, `ThemeManager::set_mode`).
- Severity: low (a refused write is still reported); confidence: high (each listed
  method was read: it calls `writable()` directly, as `TreeController::set_expanded`
  does at tree.rs:344, or writes through `Signal::set`, as
  `ListSelection::set_active` does at list_selection.rs:167-169).
- Fix: list the five phase variants or point to `Signal::set`'s list. Non-breaking.
- Destination: TODO.md.

### claude-T4-2 `# Errors` sections that name categories instead of variants

- Rubric item T4 (Documentation); same project rule as claude-T4-1.
- Observation (signature-level documentation): processing the 540 `Result`
  functions in candidates/errors.txt (texts extracted from the source, macro
  bodies read at number_inputs.rs:1916-2446, numeric_controls.rs:1526-1600 and
  text_fields.rs:205-770), these sections name neither the variant nor a
  matchable condition, or describe conditions with no variant:
  - category only ("phase/closed errors", "counter errors", "a runtime
    read/lifecycle error", "normal edit admission failures", "the usual
    phase/closed errors"): `EditSession::update`/`commit`/`cancel`
    (parameter_binding.rs:743/797/868), `ParameterHost::write_base`/
    `replace_spec`/`set_effective`/`remove` (parameter_binding.rs:528/543/590/608),
    `AnyParameterHost::remove` (parameter_binding/erased.rs:328),
    `RemoteHost::on_submit`/`clear_submission_sink`/`respond`/`resynchronize`
    (parameter_binding/remote.rs:453/478/564/663), `Parameter::begin_edit`
    (parameter_binding.rs:351), `Runtime::parameter` (parameter_binding.rs:954)
    and `remote_parameter` (remote.rs:701), `FloatingWindow::position`/
    `initial_size`/`fixed_size` (windows.rs:276/289/303),
    `Runtime::cancel_input` (runtime.rs:4887), `Runtime::set_presentation_rate`
    (runtime/presentation.rs:49), `Runtime::service_inspection` (runtime.rs:4163),
    `EventContext::set_timer` (runtime.rs:3412), `EventContext::start_drag`
    (runtime/drag.rs:132), `Permission::get` (permission.rs:147),
    `FileBrowserController::selected_paths` (file_browser/controller.rs:657).
    (Trait methods that consumers implement, such as `ContentRows::reveal`,
    were not counted: their variants are the implementation's.)
  - conditions without variants: `ParameterState::begin`/`replace_spec`/
    `write_base` (pert-ui-core edit.rs:367/606/581; for `replace_spec` the
    variant of "an incompatible base or overlay" cannot be inferred),
    `Runtime::mount` (runtime.rs:4203), `mount_tree` (runtime/tree.rs:88),
    `children` (tree.rs:132), `invalidate`/`set_visible`/`unmount`/`focus`/`event`
    (runtime.rs:5090/5021/5064/4284/4239), `Runtime::paint` (runtime/compose.rs:22:
    "Rejects pending updates or an invalid runtime phase", which are
    `DrawError::PendingUpdates` and `DrawError::Runtime(WriteDuring…)`),
    `Color::rgba` (drawing.rs:258; its siblings name `DrawError::InvalidColor`),
    `MenuBar::new` (menu_bar.rs:662; `Menu::new` names the `ChoiceError` variants),
    `ViewState::new` (workspace.rs:236, which states only the naming rule, not
    that `WorkspaceError::InvalidName` results), `ViewState::with` (248),
    `Workspace::new` (518), `Workspace::decode` (865), `WorkspacePath::new`
    (364), `WorkspaceValue::text` (105).
  - "a [`GeometryError`]" without the variant: `FileBrowser::new`
    (file_browser/view.rs:87), `Table::new` (table.rs:1441), `Tree::new`
    (tree.rs:914), `VirtualList::new`/`estimated` (virtual_list.rs:635/659),
    while `VirtualAxis::new` (virtual_axis.rs:150) names the three variants for
    the same check.
  - a nested variant named without its `RuntimeError` wrapper:
    `pert_ui::App::size`/`min_size`/`max_size` (pert-ui/src/app.rs:342/353/366)
    and `NativeWindowOptions::size` (runtime/native_windows.rs:222) name
    `GeometryError::Empty` for a `RuntimeError` return.
- Severity: low (most conditions are recoverable from context, but a consumer
  writing a `match` must guess the variant); confidence: high.
- Fix: name the variants in each list; for the binding family a shared
  paragraph that the sections link to would keep them short. Non-breaking.
- Destination: TODO.md.

### claude-T4-3 `WorkspaceError::Participant` prints its cause twice; `RestoreRejection` hides its typed cause from `source()`

- Rubric item T4 (Causes); project rule, contracts "Error policy": "A wrapped
  error is transparent or exposed through `source()`, so a chain prints each
  cause once."
- Signature (`#[error]` attributes), all configurations:
  - `WorkspaceError::Participant` (pert-ui-runtime/src/workspace.rs:643-653):
    `#[error("workspace participant `{path}` could not build its state: {source}")]`
    with `#[source] source: Box<WorkspaceError>`. `Display` already contains the
    cause and `source()` returns it, so a chain reporter prints the cause twice.
  - `RestoreRejection::State(WorkspaceError)` (workspace.rs:702) and
    `RestoreRejection::Runtime(RuntimeError)` (workspace.rs:712) format the
    typed cause into the message (`: {0}`) and mark no source, so `source()`
    is `None`: the cause is reachable by matching the variant but not through
    the chain, and is neither transparent nor a source.
- Severity: low; confidence: high (thiserror takes a source only from a
  `source` field or `#[source]`/`#[from]`).
- Fix: drop `{source}` from `Participant`'s message; give the two
  `RestoreRejection` variants `#[source]` and drop `: {0}`. Display text change
  only; non-breaking in signature.
- Destination: TODO.md.

### claude-T4-4 One frame-size limit, four reporting variants; `LimitError`'s rustdoc claims the CPU renderer uses it

- Rubric item T4 (Conversions: "a shared limit has one reporting variant");
  project rule, contracts "Error policy": "One limit has one reporting variant
  across paths that share the limit."
- Signature and behavior. `FrameImage::MAX_BYTES` is reported as:
  `CaptureError::TooLarge` (capture.rs:366/368, runtime/capture.rs:104),
  `pert_ui_cpu::CpuRenderError::TooLarge { width, height }` (pert-ui-cpu
  lib.rs:24-31, raised at 185/200 by `CpuRenderer::render`),
  `LimitError::CaptureSize` (raised by `pert_ui_wgpu::Renderer::render_offscreen`,
  pert-ui-wgpu lib.rs:679) and `LimitError::FrameSize` (render_limits.rs:94-101).
  The two public renderers thus report the same capture limit through
  different variants for the same operation (render a frame into a
  `FrameImage`). `LimitError`'s rustdoc (render_limits.rs:36-41) and its module
  doc ("shared by every renderer so applications and hosts handle them alike")
  say "the CPU renderer reports the frame and capture sizes it cannot hold",
  but `pert-ui-cpu` exposes no `LimitError`; only the facade's private presenter
  maps `CpuRenderError::TooLarge` into `LimitError::FrameSize`/`CaptureSize`
  (pert-ui/src/platform/presenter.rs:209-210, 314-315).
- Severity: low (custom hosts using both renderers write two arms; the doc
  sentence misleads a reader of `pert_ui_cpu`); confidence: high.
- Fix: `CpuRenderError::Limit(LimitError)` replacing `TooLarge`, or correct the
  `LimitError` doc to say the facade maps it. Breaking for the first option
  (unpublished).
- Destination: TODO.md.

### claude-T4-5 Two library error types derive less than their data permits

- Rubric item T4 (Type and traits); project rule, contracts "Error policy":
  "Library-owned errors derive `Clone`, `Debug`, `PartialEq` and `Eq` where
  their data permits".
- Signature:
  - `pert_ui_cpu::CpuRenderError` (pert-ui-cpu/src/lib.rs:16) derives `Clone,
    Debug, PartialEq` but not `Eq`; every field (`DrawError`, `u32`) is `Eq`,
    and its sibling `pert_ui_wgpu::RenderError` derives `Eq`.
  - `RedescriptionError` (pert-ui-runtime/src/conformance.rs:14) derives only
    `Debug`; its data (`String`, `RuntimeError`, `WidgetId`, `Box<Self>`,
    `Box<ComponentDetails>`) is all `Clone + PartialEq` (not `Eq`, because
    `ComponentDetails` holds floats). A test asserting
    `assert_eq!(redescribe(..).unwrap_err(), …)` cannot.
- Severity: low; confidence: high.
- Fix: add the derives. Non-breaking.
- Destination: TODO.md.

### claude-T4-6 Error messages that omit the value that failed (re-examines slice 3's F23)

- Rubric item T4 (Display: "says what failed, with which value"); guideline
  C-GOOD-ERR; slice 3's F23.
- Signature (`#[error]` text and variant data), all configurations:
  - Keys: `RuntimeError::AmbiguousKey` "several descendants share the key"
    (runtime.rs:319), `RuntimeError::DuplicateKey` "duplicate component key"
    (runtime.rs:311), `LayoutError::DuplicateKey`/`UnknownKey`/`ReservedKey`
    (layout.rs:25/33/29), `FontError::UnknownIcon` "unknown Material icon name"
    (text/fonts.rs:138), `RuntimeError::ShortcutConflict` "ambiguous shortcut
    registration" (runtime.rs:219), `ChoiceError::DuplicateValue`
    (choice_controls.rs:118). All are unit variants, so neither `Display` nor
    a `match` can say which key, name or chord.
  - Paths: `pert_ui::images::load` returns `ImageError::Io` "reading the image
    failed" (pert-ui/src/images.rs:243) with an `io::Error` source that does
    not contain the path; `list_directory` returns a `ListingError` whose
    message is the OS text alone (file_browser.rs:100-110, 296).
  - Contrast within the same API: `WorkspaceError::DuplicatePath`/`DuplicateKey`,
    `RestoreRejection`, `UiStringError`, `RedescriptionError`, `HighlightError`
    and `FileBrowserError::DuplicateRoot` carry and print the offending value.
- F23 re-examined: confirmed at runtime.rs:315-320; once the error is
  propagated with `?` (out of a callback, a flush or a test helper), nothing
  says which key was ambiguous or duplicated.
- Severity: low (medium for `AmbiguousKey`/`DuplicateKey` in a large tree);
  confidence: high.
- Fix: carry the key/name/path (`AmbiguousKey(String)` and so on; `LayoutError`
  and `ChoiceError` would lose `Copy`; `ChoiceError::DuplicateValue` is
  not generic over the control's value type, so it can carry only an index or
  a label). Breaking
  (unpublished). The `Copy` cost is a policy question.
- Destination: TODO.md.

### claude-T4-7 The facade's blanket `From` for `AppError` names `DrawError` in diagnostics (re-examines F11)

- Rubric item T4 (Conversions); guideline C-GOOD-ERR; slice 3's F11.
- Signature: `impl<E> From<E> for AppError where DrawError: From<E>`
  (pert-ui/src/app.rs:160-175, desktop configurations). Conversions point upward
  and keep one form (the impl normalizes `DrawError::Runtime` into
  `AppError::Runtime`), so the error policy itself is met; but `?` on an error
  outside the set fails with an unsatisfied `DrawError: From<E>` bound, which
  names the wrong type (F11's first half, confirmed).
- F11's second half (`Runtime::paint`'s `DrawError` does not convert into
  `RuntimeError`) is the contract's explicit decision: "`DrawError` has no such
  conversion, since drawing fails in paint and host paths that already return
  it" (contracts.md "Error policy"). Not a departure; whether test helpers
  deserve a conversion is a **policy question**.
- Severity: low; confidence: high.
- Fix: generate explicit `From` impls for `AppError` from the
  `leaf_conversions!` list (error.rs:122-145) so the diagnostic names `AppError`;
  the diagnostics survey covers whether `#[diagnostic::on_unimplemented]` can
  help (it cannot attach to `From`). Non-breaking.
- Destination: TODO.md (with the diagnostics survey).

### claude-T4-8 F30 is resolved at this commit except for `?` into `RuntimeError`

- Rubric item T4 (Application failures); slice 3's F30. Re-examined, not a new
  departure.
- `ApplicationError` now has `From<Box<dyn Error + Send + Sync + 'static>>`
  (pert-ui-runtime/src/error.rs:79-84), keeping the whole boxed error (and its
  source chain) in the `Arc`. `ApplicationError::new` still rejects a box
  (`Box<dyn Error>` is not `Error`), and its rustdoc (error.rs:48-51) does not
  point to the `From` impl; `RuntimeError` has no `From<Box<dyn Error + Send +
  Sync>>`, so `?` on a boxed error in a callback needs
  `.map_err(ApplicationError::from)?`.
- Severity: low; confidence: high.
- Fix: a sentence on `new` pointing to `From`; optionally `From<Box<dyn Error +
  Send + Sync>>` for `RuntimeError` (and through it `DrawError`/`AppError`),
  which is a **consistency proposal**. Non-breaking.
- Destination: TODO.md (update F30's entry).

### claude-T4-9 `AnyValue` and `AnySpec` are error types that do not implement `Error`

- Rubric item T4 (Type and traits: "it is typed and implements
  `std::error::Error`"); **policy question**.
- Signature: `ParameterValue::from_any -> Result<Self, AnyValue>`
  (pert-ui-core/src/family.rs:61), `spec_from_any -> Result<_, AnySpec>`
  (family.rs:71), `TryFrom<AnyValue>` for `f32`/`i64`/`bool`/`ChoiceId`
  (erased.rs:142-156, `type Error = AnyValue`) and `TryFrom<AnySpec>` for
  `ParameterSpec<T>` (erased.rs:86-93). Returning the input on a mismatch is
  the std idiom (`TryFrom<Vec<T>> for [T; N]`), but neither type implements
  `Error` or `Display`, so `?` into `Box<dyn Error>` or `RuntimeError` fails,
  while the runtime's erased host reports the same mismatch as
  `BindingError::KindMismatch` (parameter_binding.rs:70).
- Severity: low; confidence: high on the facts.
- Fix options: accept as the std idiom and say so in the rubric/contract, or
  return a `KindMismatch<V>` error carrying the value (as `SendError<T>` does).
  Breaking for the second.
- Destination: TODO.md if adopted.

### claude-T4-10 User-visible message errors conflict with the lowercase `Display` rule

- Rubric item T4 (Display); **policy question**.
- `InputRejection` (text_fields/normalize.rs:22-25, `#[error("{message}")]`,
  rustdoc example "Name must not be empty") and `ListingError`
  (file_browser.rs:76-79, `#[error("{message}")]`, a "user-visible message",
  from `io::Error::to_string()` such as "No such file or directory (os error
  2)") use `Display` for user text that begins in uppercase; `SinkError`
  prefixes a lowercase kind. The error policy says "Display messages begin in
  lowercase", and these types document the opposite on purpose.
- Severity: low; confidence: high.
- Fix: either record a contract exception for user-message errors, or keep
  `Display` lowercase diagnostic text and expose the user text only through
  `message()`.
- Destination: TODO.md (policy).

### claude-T4-11 The callback-boundary exception names `WindowError`, which no public method returns

- Rubric item T4 (Conversions: "the callback-boundary exceptions are as
  contracts lists them"); low; documentation of the contract.
- The contract ("Error policy") and `RuntimeError`'s rustdoc
  (runtime.rs:158-163) justify `From<WindowError> for RuntimeError`
  (runtime/native_windows.rs:891-905) by "an application callback that calls a
  runtime method returning `WorkspaceError`, `WindowError` or `CaptureError`".
  The committed listing (api/pert-ui-runtime/default.txt) has no public function
  returning `Result<_, WindowError>`; the only producer is the facade's private
  `presentation_action` (pert-ui/src/platform/windows.rs:50-54), and a consumer
  meets `WindowError` only inside `PresentationStatus::Failed`. The impl and
  the 18 leaf conversions into `WindowError` (native_windows.rs:916) match the
  contract's list; the stated reason does not apply to `WindowError`.
- Severity: low; confidence: high.
- Fix: say in the contract and rustdoc that `WindowError` arrives through
  `PresentationStatus::Failed` (and is built by custom hosts), or drop it from
  the exception. Non-breaking for the doc.
- Destination: TODO.md.

### claude-T2-1 `Divider` and `Svg` are `Send`, unlike every other widget description

- Rubric item T2; guideline C-SEND-SYNC; project rule AGENTS.md "Scheduling and
  real-time boundaries" as the rubric applies it: "UI-thread types are neither."
- Signature (auto-trait lines of candidates/types.txt, from the listings):
  `pert_ui_runtime::Divider` (views.rs:906, all configurations) is `Send +
  !Sync`, because its fields are a `bool`, a `Spacing` and two `Cell<f32>`
  (views.rs:906-911); `pert_ui::Svg` (pert-ui/src/svg/component.rs:42, `svg`
  and `no-default-svg`) is `Send + !Sync` (a `resvg` tree and `RefCell`s).
  Every other widget, container and surface description in the inventory is
  `!Send + !Sync` (they hold `TextSource`, signals or `Rc`). Neither rustdoc
  says anything about threads. The `Send` is incidental, so the public API
  promises it: giving `Divider` a bound label or `Svg` a `TextSource`
  description later would remove `Send`, a breaking change that nothing
  announced.
- Severity: low; confidence: high.
- Fix: a private `PhantomData<Rc<()>>` (or `*const ()`) marker so both are
  `!Send` like their family, or a rustdoc statement that they are `Send` on
  purpose. Breaking for the first (unpublished).
- Destination: TODO.md.

### claude-T2-2 `RichText` and `TextSpan` are `!Send` without saying so

- Rubric item T2 ("assessed separately against each type's ownership and
  rustdoc"; "Data that moves between threads is `Send`"); **policy question**.
- Signature: `RichText` (text/rich.rs:87) and `TextSpan` are `!Send + !Sync`
  in every configuration because a span can name a `FontId` (`Rc` identity,
  text/fonts.rs:86-90) and an inline icon an `IconGlyph`. The rustdoc
  (text/rich.rs:73-84) calls it "Owned display text" and states no thread
  restriction, unlike `FontId` (text/fonts.rs:75-83, with a `compile_fail`
  `Send` check) and `UiStrings` ("UI-thread data", strings.rs:537).
  Styled text is the one description value an application might plausibly
  build on a worker (for example highlighted log lines handed to the UI through
  `Runtime::spawn`, whose result must be `Send`); `RichText::highlighted`
  could otherwise run there. No probe or round 1 run demonstrated the need.
- Severity: low; confidence: high on the facts, medium on the need.
- Fix: state the UI-thread restriction in `RichText`'s rustdoc; whether styled
  text should become `Send` (font identity by `Arc` or by a sendable key) is a
  design decision. Non-breaking for the doc.
- Destination: TODO.md (doc); follow-ups (a sendable form).

### claude-T3-1 Growing inspection DTOs and status enums are exhaustive (proposal)

- Rubric item T3; **consistency proposal** ("Other enums and structs with
  public fields that are expected to grow are reported as proposals").
- Signature, all configurations. Expected to grow, with in-source evidence that
  they already have:
  - `InspectionSnapshot` (diagnostics.rs:849): fields documented as added in
    schema 5, 10 and 11 (diagnostics.rs:856-863); `ComponentInspection`
    (diagnostics.rs:667): fields added in schema 11 and 12 (676, 686);
    `ValidationCode` (validation.rs:114): "Schema 14 added this code"
    (validation.rs:150). None is `#[non_exhaustive]` and none says it is
    closed, so each schema revision is a breaking change to struct literals,
    exhaustive patterns and exhaustive matches.
  - Same shape without recorded growth yet: `QueryResult`, `WindowInspection`,
    `SourceInspection`, `ParameterInspection`, `ProfileSnapshot`,
    `ProfileStats`, `ProfileSample`, `ValidationReport`, `ValidationIssue`,
    `ValidationMarker`, `EventRecord`, `EventHistory`, `DragInspection`,
    `Capture`, `CaptureJob`; enums `EventKind` (21 variants), `ProfileOperation`
    (31), `Selector` (9), `InspectValue`, `DiagnosticCommand`,
    `SurfaceDismissal` (10), `SurfaceKind` (8), `PresentationStatus` (7),
    `PresentationAction`, `ToastStatus` (7), `TaskStatus`, `CommandStatus`,
    `CommandSource`, `DialogOutcome`, `AuthorityStatus`.
  - The API already marks the comparable `Semantics`, `SemanticItem`,
    `SemanticRole`, `SemanticAction`, `KeyCode`, `PointerButton`,
    `ToolWindowChange`, `CommandEntry` and `ContentHit` `#[non_exhaustive]`,
    and `Semantics` grew in schema 13 (diagnostics.rs:492-496) without a break;
    the inspection DTOs beside it did not get the same treatment.
- Severity: low (no consumer constructs most of these; matches and literals
  in tooling code break on each schema bump); confidence: high on the facts.
- Fix: `#[non_exhaustive]` on the library-produced inspection DTOs and status
  enums, or a rustdoc "closed" statement for each that is meant to stay closed.
  Breaking (unpublished).
- Destination: TODO.md if adopted.

### claude-T5-1 `CaptureJob` is a request token with public, mutable fields that `capture_scene` trusts

- Rubric item T5; guideline C-STRUCT-PRIVATE; project rule AGENTS.md "Give
  domain quantities and identities appropriate validated types with private
  invariants."
- Signature and behavior: `CaptureJob { pub id: u64, pub window: u64, pub
  include_overlay: bool }` (pert-ui-runtime/src/capture.rs:531-539, all
  configurations) is produced by `Runtime::pending_captures`
  (runtime/capture.rs:272-283) from the pending request. The three fields form
  one invariant (the request `id` is for that `window` with that overlay
  choice), but `Runtime::capture_scene(&CaptureJob)`
  (runtime/capture.rs:296-310) renders whatever `job.window` and
  `job.include_overlay` say, without looking the request up by `id`, and
  `complete_capture(job: u64, ..)` (runtime/capture.rs:321-344) then answers
  the request by `id` alone. A host that builds or edits a job (the fields are
  public and the type has no constructor contract) renders a different window
  or overlay mode than the request asked for; the later extent check
  (runtime/capture.rs:357-366) catches only a size difference, so a
  same-sized window's pixels are silently attributed to the wrong target.
  `id` and `window` are also bare `u64` identities.
- Severity: low (it takes a custom host that builds or alters a job, but the
  result is then silently wrong rather than an error); confidence: high on the
  code path, medium on how likely a host is to edit a job.
- Fix: private fields with accessors (the type is only produced by the
  runtime), and have `capture_scene` look up the pending request by id (or
  take the id), so the job cannot disagree with its request. Breaking
  (unpublished).
- Destination: TODO.md.

### claude-T6-1 Standard conversion traits that apply but are missing

- Rubric item T6; guideline C-CONV-TRAITS ("where they apply"); low.
- Signature, all configurations:
  - `ChoiceId` (pert-ui-core/src/choice.rs:32) wraps `Arc<str>` and offers
    `as_str()` (choice.rs:50) but neither `AsRef<str>` nor `Borrow<str>`
    (consistent with its derived `Eq`/`Hash`/`Ord`), so a
    `HashMap<ChoiceId, _>` cannot be queried with a `&str` and the id cannot
    be passed where `impl AsRef<str>` is taken (for example
    `ContentQuery::new(text: impl AsRef<str>)`).
  - `Tab::into_element` (tabs.rs:119), `Section::into_element`
    (collapsible.rs:849), `Property::into_element` (property_grid.rs:88) and
    `FloatingWindow::into_element` (windows.rs:494) are infallible conversions
    to `Element` with no `From<_> for Element`, so a list of them cannot be
    collected with `.map(Into::into)` into the `Vec<Element>` that
    `Runtime::reconcile_children` takes. The named methods are legitimate; the
    trait impls would add the conventional spelling.
- No inherent method duplicates a trait implementation under a confusable name
  (checked all 24 entries of candidates/conversion-methods.txt and searched the
  sources for inherent `from_str`, `to_string`, `into_iter`, `as_ref`, `from`,
  `try_from`); `Color::from_hex` beside `FromStr for Color` is a legitimate
  named constructor.
- Severity: low; confidence: high. Fix: add the impls; non-breaking.
- Destination: TODO.md.

## Inventory gaps

1. **errors.txt omits the error positions of standard conversion-trait impls.**
   It lists inherent and trait-declared functions, but not `TryFrom::Error` or
   `FromStr::Err`. Evidence: `NormalizedValueError` appears only as an
   `impl Error` line, although it is the error of `impl TryFrom<f32> for
   NormalizedValue` (pert-ui-core/src/normalized.rs:88); `HexColorError` of
   `impl FromStr for Color` (drawing.rs:744) appears only through
   `Color::from_hex`; `AnyValue` as the error of the four `TryFrom<AnyValue>`
   impls (erased.rs:142-156) and `AnySpec` of `TryFrom<AnySpec>` (erased.rs:86)
   appear only through `from_any`/`spec_from_any`. No error type is missed
   overall (each is reachable through another line), but an auditor working
   from errors.txt alone would not see these positions.
2. **conversion-traits.txt files an impl under its source type without naming
   the target.** For example `pert_ui_cpu::CpuRenderError impl
   From<CpuRenderError> crates/pert-ui-cpu/src/lib.rs:42` is
   `impl From<CpuRenderError> for CaptureError`; `pert_ui_core::ParameterError
   impl From<ParameterError> edit.rs:223` is `From<ParameterError> for
   EditError`; `pert_ui_runtime::Length impl From<Length> theme.rs:327` is
   `From<Length> for Spacing`. The target is recoverable only by opening the
   location. Foreign source types also lose their path:
   `ListingError impl From<Error>` and `ImageError impl From<Error>` are
   `From<std::io::Error>`, indistinguishable from `std::error::Error` in the
   list.
3. No gap found in types.txt, fields.txt or non-exhaustive.txt: the 55 structs
   with public fields agree between fields.txt and non-exhaustive.txt, the 196
   enums between types.txt and non-exhaustive.txt, and no public tuple-struct
   field exists (`rg 'pub struct \w+\(\s*pub '` finds none).

## Roles relied on

| Path | Role | Source reference |
|---|---|---|
| `ListSelection`, `ListController`, `ListFilter`, `TableController`, `TreeController`, `FieldHandle`, `FormStatus`, `ContextMenuOpener`, `ThemeManager`, `FontManager`, `IconManager`, `UiSettings` | UI-thread state owner / controller handle that applications store | list_selection.rs:62, virtual_list.rs:178, collection_filter.rs:271, table.rs:574, tree.rs:272, field_status.rs:238-379, menu.rs:1355, runtime/theme.rs:1022-1187, runtime/fonts.rs:64-208, runtime/settings.rs:223-472 (cloneable, signal-backed, `!Send`) |
| `DialogController`, `QuickPickController`, `ZoomController`, `ToolWindowController`, `DraftEditorController`, `CommandPaletteController` | controller (comparison set for T1-1) | dialog.rs:55, quick_pick.rs:438, zoom_view.rs:191, tool_windows/mod.rs:2283, text_fields/draft.rs:165, command_palette.rs:214 |
| widget/container/surface types with `#[must_use]` (`Button` … `ZoomView`, `Divider`, `pert_ui::Svg`) | UI-thread description (builder consumed by `Element::new`) | views.rs:899-911 (`Divider`), pert-ui/src/svg/component.rs:20-55 |
| `EventContext`, `UpdateContext`, `MeasureContext`, `ArrangeContext`, `DescribeContext`, `CellEdit`, `Painter` | callback context (borrowed, UI thread) | runtime.rs, layout.rs |
| `CommandId`, `FontId`, `HeldInputId`, `WidgetId`, `NativeWindowId` | identity (`Rc`-scoped to one runtime, `!Send`) | runtime/commands.rs:378-388, text/fonts.rs:75-95, runtime/held.rs:5-8, runtime.rs:140, runtime/native_windows.rs:160 |
| `KeyCode`, `Modifiers`, `CommandKey`, `KeyChord`, `PointerButton`, `WorkspacePath`, `ColorRole`, `MetricRole`, `FontRole`, `StringRole`, `Icon` | key / lookup value | input.rs:8-11, 278-292, runtime/commands.rs:6-31, pointer.rs:37-40, workspace.rs:341-355 |
| `Color` | user-facing value | drawing.rs:210, 479, 744 |
| `RichText`, `TextSpan` | owned display value (description data) | text/rich.rs:68-90 |
| `InspectionSnapshot`, `ComponentInspection`, `ValidationCode` and the other T3-1 types | library-produced inspection DTO / status (passive captured data) | diagnostics.rs:640-870, validation.rs:100-155 |
| `CaptureJob` | request token produced by the runtime for a host | capture.rs:523-539, runtime/capture.rs:262-344 |
| `ParameterSnapshot<T>` | passive captured data (not invariant-enforcing) | pert-ui-core/src/edit.rs:100-131, erased.rs:210-250 |
| every type in T4 | error carrier | definitions listed per finding |
| `AnyValue`, `AnySpec` | erased value returned as the error of a family mismatch (value, not error carrier) | erased.rs:86-156, family.rs:55-72 |
| `InputRejection`, `ListingError` | user-message error carrier | text_fields/normalize.rs:4-38, file_browser.rs:68-110 |
