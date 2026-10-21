# claude report: G1 (construction: C1, C2, C3, C4)

Commit audited: d4ba108 (library source unchanged since; frozen inputs in
`plans/usability/api-audit/inputs/`, toolchain 1.99.1).

## Dispositions

| Item | Disposition | Coverage |
|---|---|---|
| C1 choice sources | **no findings** | Choice families taken from the semantic lists and the source (see "Roles relied on"): the six `Choice<T>` controls (RadioGroup, SegmentedControl, InlineChoice, Select, InlineSelect, MultiSelect: all 17 constructors), the menu family (Menu's 4 constructors, ContextMenu::new, MenuBar::new, MenuItem::radio/toggle/submenu), QuickPick (items via `QuickPickController::open`), CommandPalette (`open`/`open_with`), keyed navigation containers (Tabs, Accordion, Breadcrumb) and the boolean families (Checkbox, Toggle, InlineToggle: `new` + `for_parameter`). builders.txt (all 538 lines) searched for builders taking options, lists or specifications: 16 hits read, none adds or replaces an option source (MultiSelect::filter filters; the `.choices` builder is gone, guarded by the `compile_fail` doctest at choice_controls.rs:893-900). Source read: choice_controls.rs:13-128, 660-1062, 1406-1650; select.rs:100-460, 860-1090, 1336-1850; multi_select.rs:30-460, 660-960, 1100-1310; menu.rs:280-420, 660-860, 925-945; quick_pick.rs:440-480. Tests read: pert-ui/tests/controls/component_bound_choices.rs (invalid reactive list keeps last valid semantics and paint for all six; same-key switches parameter→bound→fixed→parameter for the five single-choice controls), component_choice_sources.rs. One throwaway probe (see C1 notes). Error-type differences between families (QuickPick and Breadcrumb report `LayoutError`, menus `ChoiceError`, for blank/duplicate keys) are left to T4. |
| C2 constructor names | **findings** (2, both consistency proposals) | constructors.txt: all 307 entries, grouped by prefix (`new`, `bound`, `for_parameter`, `from_`, `with_`, `try_`, other). The prefixes were also checked across the whole API, not only constructors: every `pub fn with_*`, `from_*`, `try_*` in crates/ (rg) and every receiverless free function in the committed api/ listings. C-CTOR: every constructor is a static inherent method or a trait-declared conversion constructor (`HostValue::from_canonical`, `ParameterValue::from_any`); the free functions that build public types (`series_bridge`, `images::decode`/`load`, `svg::rasterize`/`rasterize_mask`) are a channel-style pair constructor and cross-crate decoders that cannot be inherent (orphan rule), so not findings. `try_` has one constructor use (`AnyParameterSnapshot::try_new`, the checked form of `new`), consistent with the proposal; `from_` keeps one meaning (construct from a given representation). |
| C3 state owners lead | **findings** (1 project rule, 1 consistency proposal) | state-owners.txt: all 79 entries, order and names read; plus the constructors the list misses by role (see inventory gaps), all of which lead with their state. The owner (`&impl StateOwner` / `&Runtime`) is first in every constructor that takes one; departures concern the bound state's position. |
| C4 validated values | **findings** (2 project rule, 2 policy questions, 1 mixed) | semantic lists: all 534 types classified by category (see "Roles relied on"); the ~45 invariant-bearing value types read at their definitions and constructors; conversion-traits.txt (all 198 entries) for `TryFrom`; every `pub const` in the api/ listings for MIN_/MAX_ placement; rustdoc of crates/ searched for documented numeric limits without a constant (rg), each hit read; slice 3's F33 (`SizeLimits::new` has no infallible fixed-size form) re-examined: C4 says one fallible entry point is enough, so it is not a C4 departure. |

## Findings

### claude-C2-1 `with_` has four meanings

- **Item / kind:** C2, consistency proposal.
- **Paths, configurations:** all configurations.
  - Constructor taking externally owned state: `Menu::with_state` (menu.rs:761), `Menu::icon_with_state` (menu.rs:803), `CollectionFilter::with_query` (collection_filter.rs:538).
  - std capacity constructor: `NavigationHistory::with_capacity` (navigation_history.rs:257).
  - Property builder (most builders in the API are bare nouns, e.g. `Select::placeholder`): `ParameterSpec::with_mapping`/`with_step` (core parameter.rs:342, 413), `StrokeStyle::with_cap`/`with_join`/`with_miter_limit` (geometry.rs:112-127), `Element::with_label`/`with_detachable`/`with_searchable`/`with_grow`/`with_rich_label` (layout.rs:437-615), `ThemeMetrics::with_metric` (runtime/theme.rs:290), `Palette::with_color`/`with_default_color` (theme.rs:875-882), `ToolWindowChange::with_kind`/`with_placement`/`with_overlay`/`with_mode`/`with_geometry` (tool_windows/mod.rs:263-288), `VirtualAxis::with_count`/`with_lanes` (virtual_axis.rs:170-188), `SeriesConfig::with_ring_capacity`, `Color::with_alpha` (drawing.rs:504).
  - Scoped closure access: `Painter::with_clip` (drawing.rs:1605), `SeriesSource::with_window` (live.rs:684).
- **Observation:** signature. A constructor `Menu::with_state(open, label, items)` reads like a builder, while a builder elsewhere is `with_grow(weight)`.
- **Rule:** rubric C2 proposal "`from_*` and `with_*` each keep one meaning across the API". std precedent supports `with_capacity` and closure-taking `with_*`, so the cleanest split is: keep those two, and rename the state-supplying constructors and the `with_` builders.
- **Severity:** low. **Confidence:** high (observation), the remedy is a policy choice.
- **Fix:** policy question for the maintainer: pick the meaning(s) `with_` keeps; e.g. name the externally owned state constructors by what they take (`Menu::controlled`, `CollectionFilter::for_query`) and drop the `with_` prefix from property builders to match the bare-noun builders. Breaking (unpublished, permitted by the component review).
- **Destination:** app-author slice 9 (naming shape) or TODO.md if adopted.

### claude-C2-2 `Color::rgba` is linear beside `Color::from_srgb`

- **Item / kind:** C2, consistency proposal.
- **Paths, configurations:** `pert_ui_runtime::Color::rgba` (drawing.rs:258), `Color::from_srgb` (drawing.rs:327), `Color::from_srgb8` (drawing.rs:358); all configurations.
- **Observation:** signature. Two fallible constructors with the identical signature `(f32, f32, f32, f32) -> Result<Self, DrawError>` differ only in the colour space: the unprefixed `rgba` takes linear channels ("Creates a linear color. RGB input is not interpreted as sRGB bytes", drawing.rs:254), the prefixed `from_srgb` takes gamma-encoded channels. The name that reads as the default (CSS's `rgba`) is the one that does not take the values a design tool or CSS gives, and the linear form is the only one without `from_`.
- **Rule:** C2 proposal that `from_*` keeps one meaning; here the pair breaks the naming symmetry of one type's representation constructors. Not a C-CTOR departure (`from_srgb` is a legitimate named conversion constructor).
- **Severity:** medium (a consumer passing CSS values to `rgba` gets visibly lighter colours with no error). **Confidence:** high on the observation, medium on the consumer impact.
- **Fix:** name the linear form for its space (`Color::from_linear` or `linear_rgba`), keeping `from_srgb`/`from_srgb8`. Breaking.
- **Destination:** TODO.md.

### claude-C3-1 Bound state after other parameters

- **Item / kind:** C3, project rule (AGENTS.md "Required state owners lead constructor arguments"; rubric: "The state it binds … follows the owner, if any, and comes before the other parameters").
- **Paths, configurations:** all configurations.
  - `Checkbox::new(label, value: impl Into<Signal<bool>>)` and `Toggle::new(label, value)` (choice_controls.rs:525, macro shared by both; listed at :647, :652), while the sibling `InlineToggle::new(value, name)` (inline_controls.rs:92) leads with the signal.
  - `MenuItem::toggle(key, label, value)` (menu.rs:299) and `MenuItem::radio(key, label, selection, value)` (menu.rs:333).
  - `FileBrowserController::new(ui, roots, selection: ListSelection, request)` (file_browser/controller.rs:509).
  - `TableController::new(ui, columns, source, selection: ListSelection)` (table.rs:574).
  - `TreeController::new(ui, source, selection: ListSelection)` (tree.rs:272).
- **Observation:** signature. The owner leads in every case; the bound `Signal`/`ListSelection` comes after labels, keys, roots, columns or sources.
- **Exception considered:** no contract sentence exempts these. For MenuItem the key leads every keyed item constructor (`MenuItem::new`, `Tab::new`, `Section::new`, `Property::new`, `Element::new`), so whether an identity key or the bound state leads is a **policy question**; for Checkbox/Toggle there is no such reason and the sibling InlineToggle already uses the rule's order.
- **Severity:** low (the argument types differ, so a swap fails to compile). **Confidence:** high.
- **Fix:** `Checkbox::new(value, label)`, `Toggle::new(value, label)`; controllers `(ui, selection, …)`; decide the key-versus-state rule for keyed items. Breaking; update examples, guide and gallery together.
- **Destination:** TODO.md (Checkbox/Toggle/controllers); the keyed-item question to the checkpoint.

### claude-C3-2 Owner and controller parameter names vary

- **Item / kind:** C3, consistency proposal (rubric: candidates "with order and names").
- **Paths, configurations:** all configurations.
  - State owner named `runtime` in `ListController::new` (virtual_list.rs:275) and `ZoomController::new` (zoom_view.rs:140); `ui` in every other constructor of state-owners.txt.
  - Controller parameter named `state` in `Dialog::new(state: DialogController, …)` (dialog.rs:195) and `AlertDialog::new` (dialog.rs:506); `controller` in `CommandPalette::new`, `QuickPick::new`, `DraftEditor::new`, `Table::new`, `Tree::new`, `FileBrowser::new`.
- **Observation:** signature (parameter names show in rustdoc and IDE hints). "state" for a `*Controller` also collides with the `*State` role suffix (passive captured data) of contracts "Public naming rules".
- **Severity:** low. **Confidence:** high.
- **Fix:** `ui` and `controller` throughout. Not breaking (parameter names are not API).
- **Destination:** TODO.md.

### claude-C4-1 `TextSelection::new` clamps where sibling offset inputs reject

- **Item / kind:** C4, project rule (AGENTS.md "Reject invalid external values … clamp only for documented control-limit behavior"; rubric "Non-finite and out-of-range inputs are rejected, not clamped").
- **Paths, configurations:** `pert_ui_runtime::TextSelection::new` (text_selection.rs:41-48); all configurations.
- **Observation:** behavior, documented: "Creates a selection, clamping out-of-range offsets to text end and snapping offsets inside clusters to their start." The other public inputs of UTF-8 byte offsets reject the same mistakes: `DraftSelection::Range` ("rejected with `TextEditError::InvalidSelection`, never clamped", text_fields/draft.rs:7-9, check at :224-233), `TextPreedit::new` (`InvalidComposition`, text_input.rs:55-80) and `RichText::highlighted` (`HighlightError::OutOfBounds`/`NotGraphemeBoundary`, text/fit.rs:230-245). A stale offset kept across a text change therefore silently becomes a caret at the end for a custom widget.
- **Exception considered:** the clamp is documented but is not control-limit clamping (a drag or step reaching a bound); snapping a pointer position (`at_point`, `word_at_point`) is legitimate, an explicit offset is an input.
- **Severity:** low (custom-widget authors). **Confidence:** high.
- **Fix:** make `new` fallible with `TextEditError::InvalidSelection` (or add a checked constructor and document `new` as the snapping helper the built-in editors use, if the maintainer wants both). Breaking if `new` changes.
- **Destination:** TODO.md.

### claude-C4-2 Documented limits without MIN_/MAX_ constants

- **Item / kind:** C4, project rule (contracts "Public naming rules": "Documented limits are associated `MIN_*` or `MAX_*` constants of the type they bound").
- **Paths, configurations:** all configurations.
  - `PresentationRate` (runtime/presentation.rs:4-25): "from 1 through 1000 inclusive", no constants, while the comparable validated scalars `UiZoom` and `ViewScale` expose `MIN`/`MAX` (runtime/settings.rs:30-33, transform.rs:275-278). Its raw read is `hertz()` rather than `get()`; the rule's "unless the unit itself is the point of the type" may cover that, so that part is a lower-confidence note.
  - `ViewState::new` kind tags and `ViewState::with` value names: "1 to 64 bytes" (workspace.rs:235, :612 `WorkspaceError::InvalidName`), no constant; the neighbouring workspace limits are constants (`Workspace::MAX_STRING_BYTES`, workspace.rs:504-506).
  - Outside value types (the rule's sentence is general): `VectorRasterizer` documents "at most 1,048,576 generated points per vector" and "a separate budget of 1,048,576 edges" (vector_scene.rs:103-107) backed by the private `MAX_EDGES` (geometry/raster.rs:11), beside the public `VectorRasterizer::MAX_COVERAGE_WORK` (vector_scene.rs:126).
- **Observation:** signature (missing items) plus rustdoc.
- **Severity:** low. **Confidence:** high for PresentationRate and ViewState, medium for VectorRasterizer (whether a work budget counts as a limit "of the type").
- **Fix:** `PresentationRate::MIN`/`MAX`, `ViewState::MAX_NAME_BYTES`, `VectorRasterizer::MAX_FLATTENED_POINTS`/`MAX_STROKE_EDGES`, referenced from the errors' Display and rustdoc. Not breaking (additions).
- **Destination:** TODO.md.

### claude-C4-3 Limits placed on another type than the one they bound (policy question)

- **Item / kind:** C4, project rule, recorded as a **policy question**: the rule says "of the type they bound" but does not decide where a limit shared by several types goes.
- **Paths, configurations:** all configurations (svg only in `svg`, `no-default-svg`).
  - `TextEditError::MAX_TEXT_BYTES` (text_input.rs:29-41): bounds editors, `TextPreedit`, labels, rich text and toasts; its rustdoc gives the reason ("It lives on this error because it bounds every text path").
  - `NumberInputError::MAX_PRECISION` (text_fields/number_inputs.rs:42-48): bounds `decimal_places` on DecimalInput, InlineDecimal and the numeric controls; no reason given.
  - `TextError::MAX_FONT_PIXELS` (text.rs:76-86): a raster cap, not a rejection limit, on the error type.
  - `Workspace::MAX_PATH_BYTES`/`MAX_STRING_BYTES` (workspace.rs:504-506): bound `WorkspacePath::new` (:362-378), `WorkspaceValue::text` (:104) and path segments.
  - `pert_ui::svg::MAX_NESTING` (pert-ui/src/svg.rs:92): a module constant bounding `Svg::new`/`Svg::tinted` and the module's functions.
  - Related form difference: `PowerExponent::MIN`/`MAX` are `f32` (core parameter.rs:67-70) where `NormalizedValue`, `UiZoom`, `ViewScale` and `Capacity` type theirs as `Self`.
- **Observation:** signature.
- **Severity:** low. **Confidence:** high on the facts.
- **Fix:** decide the rule for shared limits (error type with a stated reason, as MAX_TEXT_BYTES does, or the principal bounded type with links from the others) and for the constant's type, then state it in contracts "Public naming rules". Non-breaking if constants are added and kept; moving them is breaking.
- **Destination:** TODO.md after the policy decision.

### claude-C4-4 `get()` rule and string-valued validated values (policy question)

- **Item / kind:** C4, project rule (contracts: "Their raw value is read with `get()` unless the unit itself is the point of the type"), recorded as a **policy question**.
- **Paths, configurations:** `pert_ui_core::ChoiceId::as_str` (core choice.rs:47-52); `ParameterId::key` (parameter_id.rs:97) as a component of a composite identity; all configurations.
- **Observation:** signature. `ChoiceId` is a validated single-value type whose raw value is read with `as_str`, not `get`. C-CONV favours `as_str` for a borrowed `&str` view, and every numeric validated scalar does use `get` (Capacity, NormalizedValue, PowerExponent, Sequence, Timestamp, Revision, EditSequence, EditSource, Incarnation, HistoryEntryId, Length, Magnification, ViewScale, UiZoom).
- **Severity:** low. **Confidence:** medium (whether the rule is meant to cover strings).
- **Fix:** state in the rule that string-valued validated types use `as_str` (C-CONV), or add `get`. Not breaking either way if `as_str` stays.
- **Destination:** contracts wording at the checkpoint; otherwise none.

### claude-C4-5 `ProgressStatus::of` clamps an overshoot (policy question)

- **Item / kind:** C4, project rule (AGENTS.md "clamp only for documented control-limit behavior"), **policy question**.
- **Paths, configurations:** `pert_ui_runtime::ProgressStatus::of` (progress.rs:41-70); all configurations.
- **Observation:** behavior, documented: `completed > total` becomes a full track rather than an error, "because a count that overshoots an estimate is ordinary", and `total == 0` becomes `Indeterminate`. The clamp is deliberate and explained, but it is not a control limit, which is the only clamp the rule names.
- **Severity:** low. **Confidence:** high on the behavior.
- **Fix:** either name presentation conveniences as an allowed documented clamp in the rule, or add a checked form. Not breaking if only the rule changes.
- **Destination:** checkpoint (policy); TODO.md only if a checked form is wanted.

## C1 notes (no finding)

- **Exclusive constructors.** RadioGroup, SegmentedControl, InlineChoice, Select and InlineSelect offer `new` (fixed, `Result<_, ChoiceError>`), `bound` (reactive) and `for_parameter` (on the `ChoiceId` impl); MultiSelect offers `new` and `bound` (a choice parameter is single-valued, so no `for_parameter` is expected). No builder sets a source (semantic list; choice_controls.rs:893-900 `compile_fail`).
- **Fixed lists** are validated in `new` before the control exists (choice_controls.rs:727-735, select.rs:1381-1390, multi_select.rs:1227-1236; menus via `validate_menu`, menu.rs:932-945, recursively for submenus; QuickPick at `open`, quick_pick.rs:476-480; Tabs/Accordion through `check_description` before mount or reconcile, tabs.rs:914, collapsible.rs:592/1208).
- **Reactive lists** are validated in `update` (choice_controls.rs:1001-1012, select.rs:1044-1052 and :380-392, multi_select.rs:709 and :919); the stored list is replaced only after validation, so an invalid list leaves the last valid presentation (initially empty, `with_binding(…, Vec::new())`, choice_controls.rs:785-787).
- **Same-key redescription** replaces binding and source together: RadioGroup::reconcile (choice_controls.rs:906-970), SelectAnchor/OptionList::reconcile (select.rs:968-1033, :289-345), MultiSelect `Shared::adopt` (multi_select.rs:207-252); the fixed list applies only when neither a `bound` source nor a parameter supplies options.
- **Probe.** MultiSelect's keyboard path reads the derived chain over the raw option source (`Shared::key`, multi_select.rs:357-398), which could act on an invalid list the presentation does not show. A throwaway program against the facade (scratch copy, `--offline`, separate target dir) opened the popup, set `[1, 1-duplicate, 3]`, and pressed Down, Down, Enter: every `key_input` returned the `Choice(DuplicateValue)` error, the value stayed `[]` and the rows stayed `One, Two`. No departure.

## Inventory gaps

1. **constructors.txt omits free-function constructors.** It lists only associated functions, so `pert_ui_core::series_bridge(SeriesConfig) -> (SeriesEndpoint<T>, SeriesConsumer<T>)`, `pert_ui::images::decode`/`load -> Result<RasterImage, ImageError>` and `pert_ui::svg::rasterize`/`rasterize_mask` (api/pert-ui-core/default.txt, api/pert-ui/images.txt, api/pert-ui/svg.txt) are absent although C-CTOR applies to them. Assessed by hand under C2; no finding.
2. **state-owners.txt matches by type name, not role.** Constructors that bind state through a handle without the `*Controller` suffix or a `Signal`/`ReadSignal`/`ListSelection` type are missing: `HistoryButtons::new(history: NavigationHistory<T>, content, restore)`, `ToastHost::new(queue: ToastQueue, content)`, `LevelMeter::new`/`Trace::new(source: SeriesSource<f32>, scale)`, `Slider::new`/`Knob::new`/`InlineSlider::new(parameter: Parameter<f32>)` and every `for_parameter` without `ui`. All lead with that state, so no finding is hidden. Conversely `App::new` is matched only through its closure's `&mut Runtime` (a false positive).
3. **conversion-traits.txt rows under the source type do not name the target.** Each `From` impl appears under both types, e.g. `pert_ui_core::ChoiceId impl From<ChoiceId> erased.rs:156` is `From<ChoiceId> for AnyValue`, and `pert_ui_runtime::Length impl From<Length> theme.rs:327` is `From<Length> for Spacing`; a reader cannot tell the implementing type from the row. Not a missing entry; the row should print `impl From<X> for Y`.

## Roles relied on

| Path | Role | Source |
|---|---|---|
| RadioGroup, SegmentedControl, InlineChoice, Select, InlineSelect, MultiSelect | choice controls (option lists of `Choice<T>`) | contracts "Choice sources"; choice_controls.rs:660-690, select.rs:1336-1361, multi_select.rs:1124-1208 |
| Menu, ContextMenu, MenuBar, MenuItem (radio/toggle rows) | choice-capable action menus, fixed item lists | menu.rs:660-700, 299-363 |
| QuickPick / QuickPickController, CommandPalette / CommandPaletteController | pickers with per-session item lists (contracts "Menus, pickers and dialogs") | quick_pick.rs:469-480 |
| Tabs, Accordion, Breadcrumb | keyed navigation containers, not option controls | tabs.rs:914, collapsible.rs:592, breadcrumb constructor |
| VirtualList, Table, Tree, FileBrowser | controller-owned collections (contracts "Component redescription"), not choice families | contracts.md:166-170 |
| Checkbox, Toggle, InlineToggle | boolean controls (no options) | choice_controls.rs:495-545, inline_controls.rs:85-95 |
| Capacity, ChoiceId, NormalizedValue, ParameterId, ParameterSpec, PowerExponent, Sequence, SeriesConfig, ChoiceOption | invariant-bearing values (core) | live/mod.rs:183-215, choice.rs:32-75, normalized.rs:47-107, parameter_id.rs:44-118, parameter.rs:63-90/151 |
| Timestamp, EditSequence, EditSource, Revision, Incarnation | values without a rejecting invariant (any u64; or runtime-issued) | live/mod.rs:47-130, edit.rs:12-64 |
| Length, Magnification, ViewScale, UiZoom, PresentationRate, Point, Size, Rect, Constraints, SizeLimits, AxisMapping, Transform2D, Viewport, VirtualAxis, ScaleLimits, ViewTransform, SplitSizing, Track, StrokeStyle, Path | invariant-bearing geometry/layout values | containers.rs:10-40, pointer.rs:5-130, transform.rs:262-310, runtime/settings.rs:22-80, runtime/presentation.rs:4-25, layout.rs:120-290, drawing.rs:52-125/890-1000, virtual_axis.rs:120-170, zoom_view.rs:14-60, split.rs:96-125, containers.rs:955-975, geometry.rs:100-215 |
| PhysicalExtent | plain value, every pair valid | drawing.rs:846-884 |
| Color, OpaqueColor, NumberLocale, ContentQuery, FrameImage, RasterImage, MaskImage, TextPreedit, TextSelection, RichText, WorkspacePath, WorkspaceValue, ViewState, NativeWindowOptions, Svg | invariant-bearing values (runtime/facade) | drawing.rs:254-370/667, number_format.rs:218-245, content_search.rs:22-100, capture.rs:352-400, resources.rs:126-216, drawing.rs:1135-1180, text_input.rs:44-100, text_selection.rs:30-48, text/fit.rs:230-245, workspace.rs:22-120/225-275/360-385, runtime/native_windows.rs:182-235, pert-ui/src/svg/component.rs:42-100 |
| WidgetId, ImageId, MaskId, VectorId, FontId, HeldInputId, NativeWindowId, CommandId, HistoryEntryId | runtime-issued identities/handles, no consumer constructor | semantic list (no constructors); runtime/native_windows.rs:139-158, navigation_history.rs:10-24 |
| ParameterSnapshot, PickItem, Modifiers, ShortcutBinding, TableSort, WindowPlacement, Sample, inspection and report structs | plain data with public fields (T5's domain), not C4 values | fields.txt |
| ProgressStatus | enum value with a convenience constructor | progress.rs:20-70 |
| Remaining types (components, controllers, contexts, errors, unit enums, traits) | not invariant-bearing values for C4 | semantic lists, by kind and suffix |
