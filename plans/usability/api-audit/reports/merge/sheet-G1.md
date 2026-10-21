# Merge sheet: G1

Inputs: `reports/codex-G1.md`, `reports/claude-G1.md`,
`reports/verification/codex-G1.md`, `reports/verification/claude-G1.md`
(all under `plans/usability/api-audit/`), audited commit `d4ba108`; `crates/`
and `docs/` are unchanged at HEAD `245053b`, where the merger read the
source. Runtime paths are relative to `crates/pert-ui-runtime/src/` unless
prefixed `core/` (`crates/pert-ui-core/src/`) or `facade/`
(`crates/pert-ui/src/`).

## Findings

| # | Rubric item, rule kind | Finding (one or two sentences, naming the items) | Sig/beh | Configs | Severity | Confidence | Fix (breaking?) | Destination | Sources and status | Slice 3 |
|---|---|---|---|---|---|---|---|---|---|---|
| G1-1 | C1, project rule | `RadioGroup`, `SegmentedControl`, `InlineChoice`: a same-key redescription with a different value binding clears `name` and `selected` in `reconcile` (`choice_controls.rs:939-943`) before `update` validates the new reactive list (`:1010`), so an invalid list leaves no selection (inspection `:1305`, paint `:1263`, `:1366`) instead of the last valid presentation (`docs/contracts.md:45`). Wrappers delegate at `:1509`, `:1647`; Claude's probe shows `Select`, `InlineSelect` and `MultiSelect` unaffected. | behavior (Claude executed a probe for all three wrappers) | all | Codex medium, Claude low. **Propose low:** after the failed flush `Runtime::paint` refuses with `DrawError::PendingUpdates` (`runtime/compose.rs:50`, observed in Claude's probe) and the next valid update restores `selected`, so only inspection sees the cleared state, and it needs a binding change and an invalid list in one flush. | high | Keep the presentation caches until the new binding and list validate, still cancelling input aimed at the old binding; add the binding-change failure case for the three wrappers (existing tests `crates/pert-ui/tests/controls/component_bound_choices.rs:37`, `:99` do not cover it). Not breaking. | TODO.md | codex-C1-1; **confirmed** (Claude verification with probe; Claude corrected its C1 "no findings") | - |
| G1-2 | C3, project rule | Bound state follows other parameters in `Checkbox::new` and `Toggle::new` (`label, value`; shared macro `choice_controls.rs:525`, expansions `:647`, `:652`), `MenuItem::toggle` (`menu.rs:299`), `MenuItem::radio` (`menu.rs:333`), `TableController::new` (`table.rs:574`), `TreeController::new` (`tree.rs:272`) and `FileBrowserController::new` (`file_browser/controller.rs:509`); the owner leads in all. The sibling `InlineToggle::new(value, name)` (`inline_controls.rs:92`) already follows the rule. | signature | all | low | high | Bound signal or selection first, immediately after `ui` where present; `MenuItem` depends on G1-3. Preserve authority and cancellation. Breaking (permitted while unpublished); update examples, guide and gallery together. | TODO.md | codex-C3-1, claude-C3-1; **both** (all seven constructors matched) | - |
| G1-3 | C3, policy | **Question:** may an identity key lead keyed-item constructors (`MenuItem::new`, `Tab::new`, `Section::new`, `Property::new`, `Element::new` all start with the key) ahead of the bound state, which would exempt `MenuItem::toggle`/`radio` from G1-2? Codex: no existing exception; permitting one is the maintainer's decision. | signature | all | low | high | Either move the state before the key (breaking) or state a key-first exception in contracts "Public naming rules" (not breaking). | checkpoint | claude-C3-1 (policy part); Codex verification agrees it needs the maintainer; **policy** | - |
| G1-4 | C4, project rule (and C-VALIDATE) | `Workspace::set_settings` (`workspace.rs:551-554`) replaces settings without the `MAX_DOCUMENT_BYTES` check `Workspace::new` applies (`:541-543`); `encode` adds the settings line (`:811-823`) and `decode` rejects over-limit text (`:865-868`), so a document the library built and saved cannot be loaded. | behavior (Codex derived a byte witness; Claude executed it: 1,048,576 bytes before, 1,048,596 after `with_theme_mode(Some(ThemeMode::Light))`, `decode` returns `Err(TooLarge)`) | all | Codex high, Claude medium. **Propose medium:** it is reachable only within the bounded length of the settings line (`WorkspaceSettings` has three bounded optional fields, `:418-422`) of the 1 MiB limit, although when reached the host loses the saved workspace. | high | (a) Codex: make `set_settings` fallible with a typed error, keeping the previous settings (breaking); or (b) Claude: `Workspace::new` and `decode` check entries against `MAX_DOCUMENT_BYTES` minus the longest settings line, so no settings replacement can overflow (not breaking). Either way add the boundary round-trip test (`crates/pert-ui/tests/surfaces/workspace.rs:60` covers only oversized construction). | TODO.md | codex-C4-1; **confirmed** (Claude verification with probe; severity and fix alternative added) | - |
| G1-5 | C4, project rule; policy | `ChoiceId::as_str` (`core/choice.rs:48-52`) reads a validated single-value string without `get()` (`docs/contracts.md:1152`, no string exception). Both agree the rule as written is not met. **Question:** adopt a string exception (`as_str`, as C-CONV favours for a borrowed `&str`) or add `get`? | signature | all | low | Codex high, Claude medium (whether the rule is meant to cover strings) | Amend contracts "Public naming rules" for string-valued validated types, or add `get` beside `as_str` (both not breaking); renaming `as_str` is breaking. | checkpoint; else TODO.md | codex-C4-2 (`ChoiceId` row), claude-C4-4 (`ChoiceId`); **both**, **policy** | - |
| G1-6 | C4, policy | **Question:** does the `get()` rule apply to compound values? `Color::channels` (`drawing.rs:296-304`), `Rect::values` (`drawing.rs:193-201`) and `Transform2D::entries` (`transform.rs:156-160`) return their whole private arrays; each type also has per-component accessors, and the rustdoc names destructuring and backend upload. Claude: not departures, since the rule presupposes one raw value; expects a wording clarification ("single-value types only"). | signature | all | low | high on facts; medium on applicability | Clarify the rule as single-value only (not breaking), or rename to `get` (breaking; Claude: `get()` returning an array would say less). | checkpoint | codex-C4-2 (compound rows); Claude: facts confirmed, departure not established; **policy** | - |
| G1-7 | C4, project rule | Documented limits without associated constants: `PresentationRate` accepts 1 to 1000 Hz (`runtime/presentation.rs:4-25`) with no `MIN`/`MAX`, while `UiZoom` and `ViewScale` expose theirs (`runtime/settings.rs:30-33`, `transform.rs:275-278`); `ViewState` kind tags and value names are 1 to 64 bytes (`workspace.rs:235`, check `:789`, error `WorkspaceError::InvalidName` `:612-614`) with no constant beside the `Workspace::MAX_*` ones (`:504-506`). | signature | all | low | high | Add `PresentationRate::MIN`/`MAX` and `ViewState::MAX_NAME_BYTES`, referenced from the errors' Display and rustdoc. Not breaking. | TODO.md | codex-C4-3 (`PresentationRate`, `ViewState` rows), claude-C4-2 (same); **both** | - |
| G1-8 | C4, project rule | `VectorRasterizer` documents budgets of 1,048,576 generated points per vector and 1,048,576 stroke edges (`vector_scene.rs:103-107`), backed by the private `MAX_EDGES` (`geometry/raster.rs:11`, enforced `:39`, `:239`), beside the public `VectorRasterizer::MAX_COVERAGE_WORK` (`vector_scene.rs:126`). | signature | all | low | medium (Claude: whether a work budget is a limit "of the type"); Codex: supported by the general sentence at `docs/contracts.md:1153` | Add `VectorRasterizer::MAX_FLATTENED_POINTS` and `MAX_STROKE_EDGES`. Not breaking. | TODO.md | claude-C4-2 (`VectorRasterizer`); **confirmed** | - |
| G1-9 | C4, policy | **Question:** where does a limit shared by several types live, and what type does a constant have? Limits on another type than the one they bound: `TextEditError::MAX_TEXT_BYTES` (`text_input.rs:29-41`, reason stated at `:31`; bounds `TextPreedit`, editors, rich text, toasts), `Workspace::MAX_PATH_BYTES`/`MAX_STRING_BYTES` (`workspace.rs:504-506`; bound `WorkspacePath::new` and `WorkspaceValue::text`), `NumberInputError::MAX_PRECISION` (`text_fields/number_inputs.rs:42-48`, no reason given), `TextError::MAX_FONT_PIXELS` (`text.rs:76-86`; both note it is a raster cap, not an input limit) and the module constant `pert_ui::svg::MAX_NESTING` (`facade/svg.rs:92`, applied through `Svg::new`). Sub-question: `PowerExponent::MIN`/`MAX` are `f32` (`core/parameter.rs:67-70`) where `NormalizedValue`, `UiZoom`, `ViewScale`, `Capacity` use `Self`; Codex: no rule requires `Self`, so not a departure. `docs/contracts.md:1141` requires one reporting variant per shared limit but says nothing about constant ownership. | signature | all; `svg::MAX_NESTING` only in the svg configurations | low | high on facts; medium on the convention's intended breadth | Decide the rule (error type with a stated reason, as `MAX_TEXT_BYTES` does, or the principal bounded type with links from the others, possibly aliases to one definition) and the constant type, and state it in contracts "Public naming rules". Additions not breaking; moving or removing constants breaking. | checkpoint, then TODO.md | codex-C4-3 (`TextPreedit`, `WorkspacePath`, `WorkspaceValue` rows), claude-C4-3; **both** for the matched rows, Codex confirmed the rest (`MAX_PRECISION`, `MAX_FONT_PIXELS`, `MAX_NESTING`); `PowerExponent` type refuted as a departure, kept as sub-question; **policy** | - |
| G1-10 | C4, policy | **Question:** do intrinsic zero, one or nonnegativity bounds need `MIN_*`/`MAX_*` constants? `StrokeStyle` miter limit at least 1 (`geometry.rs:121-128`); `Color` channels and `SplitSizing::ratio` in `0..=1` (`drawing.rs:254-262`, `split.rs:103-110`); nonzero `Capacity` (only `MAX`, `core/live/mod.rs:186-202`), `ContentQuery` limit and budget (`content_search.rs:73-94`), `VirtualAxis` lanes (`virtual_axis.rs:187-192`); nonnegative `Length`, `Size`, `Constraints`, `Rect`, `Viewport` (`containers.rs:19-22`, `layout.rs:138`, `:257`, `drawing.rs:55`, `:893`; `Size::ZERO` exists). Claude reads the rule as covering chosen limits only, and keeps one low proposal: the unit interval, which `NormalizedValue` bounds with `MIN`/`MAX` while `Color` and `SplitSizing` take raw `f32` (see G1-18). | signature | all | low | high on facts; medium on breadth | Document intrinsic domain bounds as outside the rule (not breaking), or add the constants (not breaking). | checkpoint | codex-C4-3 (rows unmatched by Claude); Claude: facts confirmed, departure not established; **policy** (Codex also flags it as policy) | - |
| G1-11 | C4, project rule | `TextSelection::new` clamps out-of-range explicit offsets to the text end and snaps offsets inside clusters (`text_selection.rs:41-48`; Parley 0.12.0 `editing/cursor.rs:17`, `:29`), while sibling offset inputs reject: `DraftSelection::Range` (`text_fields/draft.rs:7-9`, `:224-233`), `TextPreedit::new` (`text_input.rs:55-80`), `RichText::highlighted` (`text/fit.rs:230-245`). An explicit offset is an input, not control-limit clamping; pointer snapping (`at_point`, `word_at_point`) is legitimate. | behavior (documented) | all | low | high | Make `new` fallible with `TextEditError::InvalidSelection`, or add a checked constructor; Codex: keeping a clamping `new` then needs an explicit exception, documenting its use is not enough; preserve backward selections (anchor need not precede focus); the siblings differ on character versus grapheme boundaries. Breaking if `new` changes. | TODO.md | claude-C4-1; **confirmed** (Codex withdrew its control-limit classification) | - |
| G1-12 | C4, policy | **Question:** may a documented presentation convenience clamp? `ProgressStatus::of` turns `completed > total` into a full track (`progress.rs:41-62`, `completed.min(total)` at `:64`) and `total == 0` into `Indeterminate` (`:58-59`), documented and reasoned ("a count that overshoots an estimate is ordinary") and asserted in `crates/pert-ui/tests/presentation/progress.rs:155`, but not a control limit, the only clamp AGENTS.md names. | behavior | all | low | high | Name presentation conveniences as an allowed documented clamp in the rule (not breaking), or add a checked form; Codex: a checked form alone does not settle the existing constructor's status. | checkpoint; TODO.md only if a checked form is wanted | claude-C4-5; Codex confirmed and corrected its own classification; **policy** | - |
| G1-13 | C2, consistency proposal | **Question:** which meaning does `with_` keep? It has four: constructors taking externally owned state (`Menu::with_state` `menu.rs:761`, `Menu::icon_with_state` `:803`, `CollectionFilter::with_query` `collection_filter.rs:538`); the std capacity constructor (`NavigationHistory::with_capacity` `navigation_history.rs:257`); property builders (`ParameterSpec::with_mapping`/`with_step` `core/parameter.rs:342`, `:413`; `StrokeStyle::with_*` `geometry.rs:112-127`; `Element::with_*` `layout.rs:437-615`; `ThemeMetrics::with_metric`; `Palette::with_color`/`with_default_color`; `ToolWindowChange::with_*`; `VirtualAxis::with_count`/`with_lanes`; `SeriesConfig::with_ring_capacity`; `Color::with_alpha` `drawing.rs:504`) where most builders are bare nouns; scoped closure access (`Painter::with_clip` `drawing.rs:1605`, `SeriesSource::with_window` `live.rs:684`). | signature | all | low | high on the observation; the remedy is a policy choice | Claude: keep the capacity and closure forms, name state-supplying constructors by what they take (`Menu::controlled`, `CollectionFilter::for_query`) and drop `with_` from builders. Codex: bare nouns collide with existing getters (`Color::alpha` `drawing.rs:293`, `Element::detachable` `layout.rs:508`). Breaking. | app-author slice 9 (naming), or TODO.md if adopted | claude-C2-1; **confirmed** (Codex corrected its C2 "no findings"); **policy** | - |
| G1-14 | consistency proposal (outside the C2 prefix rule) | **Question:** should the linear constructor be named for its space? `Color::rgba` (`drawing.rs:254-262`) takes linear channels with the same `(f32, f32, f32, f32) -> Result<Self, DrawError>` signature as `Color::from_srgb` (`:327`), beside `from_srgb8` (`:358`); the unprefixed, CSS-like name is the linear one, so normalized sRGB fractions from a design tool are silently read as linear (0 to 255 values are rejected). | signature | all | low (Claude lowered it from medium; Codex: no severity attaches to a C2 finding) | high on facts; medium on consumer impact | `Color::from_linear` or `linear_rgba`, keeping `from_srgb`/`from_srgb8`. Renaming breaking; an alias not. | TODO.md if adopted | claude-C2-2; refuted as a C2 departure and conceded, maintained by Claude as a separate consistency proposal, which Codex calls a "separate usability proposal"; **policy** | - |
| G1-15 | consistency proposal (outside C3's order rule) | **Question:** use `ui` and `controller` throughout? The owner parameter is `runtime` in `ListController::new` (`virtual_list.rs:275`) and `ZoomController::new` (`zoom_view.rs:140`), `ui` elsewhere; the controller parameter is `state` in `Dialog::new` (`dialog.rs:195`) and `AlertDialog::new` (`dialog.rs:506`), `controller` elsewhere (e.g. `quick_pick.rs:888`, `table.rs:1441`). | signature (rustdoc and IDE hints) | all | low | high | Rename the parameters. Not breaking. | TODO.md if adopted | claude-C3-2; Codex partly confirmed (facts hold; C3 states order, not names; the `*State` argument disputed, see Disputed facts 3); **policy** | - |
| G1-16 | C1, policy | **Question:** do lazy collections and keyed content fall under "Choice sources"? `VirtualList::new` and `VirtualList::estimated` take only `impl Into<ReadSignal<ListSource>>` under `new` and a sizing name (`virtual_list.rs:635-641`, `:659-665`); `ToolWindowLayout::new(center)` takes the center, and the builders `window`/`windows` add the keyed tool windows its stripes switch between (`tool_windows/mod.rs:765`, `:788-804`, `:807-809`). Neither the collection exception (`docs/contracts.md:166-170`) nor the tool-window admission contract (`tool_windows/mod.rs:695-699`) settles constructor naming. | signature | all | low | high on signatures; medium on applying the choice vocabulary | Claude prefers one contract sentence placing collections and containers outside "Choice sources" (not breaking); Codex leaves the choice between that exception and a constructor migration (breaking) to the maintainer, without parameter constructors. | TODO.md, as a contract clarification | codex-C1-2; Claude partly confirmed (holds for `VirtualList`; would drop `ToolWindowLayout`, see Disputed facts 2); Codex maintains both as policy; **policy** | - |
| G1-17 | B3 (outside G1), project rule | `ContextMenu::reconcile` publishes its enabled signal with `publish_trusted` (`menu.rs:1472`), which replaces the value and invalidates subscribers (`signal.rs:431-445`), against AGENTS.md "Never publish signals from reconciliation". The signal is private: created at `menu.rs:1411`, held by the menu (`:1260`) and the private `OpenerLink` (`:1286-1293`), and read with `get()` by `ContextMenuOpener::open` (`:1361`). | behavior | all | Codex medium, Claude low. **Propose low:** no component, binding or application code can subscribe to the signal (merger checked `menu.rs:1260-1293`, `:1361`), so the publication is unobservable. | high | Codex: deliver through the event-phase notification contract. Claude: that contract is inapt (nobody is notified); replace the private `Signal<bool>` with a shared `Rc<Cell<bool>>`, likewise `visible_signal` (published from lifecycle hooks at `:1484`, `:1487`). Not breaking. | G2 (B3) verification; TODO.md | codex-B3-1 (labelled incidental, outside G1); **confirmed** (Claude: fact holds; severity and fix corrected) | - |
| G1-18 | C4, consistency proposal (C-NEWTYPE) | `SplitRule::Ratio(f32)` (`split.rs:141-145`), produced by `SplitSizing::rule` (`:123-133`) and reported through `SplitPane::on_resize` (`:322`), documents a `0..=1` share but carries a raw `f32`, while its siblings carry `Length` and `NormalizedValue` exists (used by `ProgressStatus::Determinate`, `progress.rs:30`). No public function takes a `SplitRule`. | signature | all | low | high on facts | `SplitRule::Ratio(NormalizedValue)`. Breaking (variant payload). | TODO.md, or the checkpoint with G1-10's unit-interval question | claude-C4-d1 (Claude's inventory delta); **delta-unverified** | - |

Notes:
- Slice 3: both auditors re-examined F33 (`SizeLimits::new` has no
  infallible fixed-size form) and found no C4 departure: C4 asks for one
  fallible entry point (`layout.rs:209`). No G1 finding extends an F-number.
- Inventory gaps both reported (conversion rows attributed to the source
  type, `Constraints::bounded` grouped as `[bound]`, state-owner candidates
  matched by type name, Claude's missing free-function constructors) were
  addressed in inventory version 2. Codex's inventory delta found nothing
  new; Claude's found G1-18.
- Out-of-group remarks, not G1 findings: Claude leaves error-type
  differences for blank or duplicate keys (QuickPick and Breadcrumb report
  `LayoutError`, menus `ChoiceError`) to T4, and notes for T6 that only
  `UiZoom` and `PowerExponent` convert back to their raw type with `From`.

## Disputed facts

1. **`ParameterId::key` is a whole-value raw accessor that the `get()`
   rule covers** (claude-C4-4 lists it beside `ChoiceId::as_str`).
   - Claude: `core/parameter_id.rs:97`, already described in its report as
     "a component of a composite identity"; Claude's verification does not
     answer the refutation.
   - Codex: `ParameterId` holds session, object, key and incarnation
     (`core/parameter_id.rs:44-48`), and `key` (`:99`) exposes one
     component, so it adds no confirmed instance.
   - Settled by: the struct fields and the accessor. **The code supports
     Codex:** `pub struct ParameterId { session: u128, object: u128, key:
     String, incarnation: u64 }` (`core/parameter_id.rs:44-49`), with
     separate accessors `session` (`:88`), `object` (`:94`), `key` (`:99`)
     and `incarnation` (`:105`); equality includes the incarnation. Claude's
     `:97` is the doc comment above `key`.

2. **`ToolWindowLayout`'s tool windows are alternatives selected through
   the stripes, so its `window`/`windows` builders add an option source**
   (codex-C1-2).
   - Codex: stripes provide keyboard selection and activation among the
     keyed alternatives (`tool_windows/mod.rs:601`); the builders add them
     (`:788`, `:807`).
   - Claude: they are content panels whose stripe buttons expand and
     collapse them (`:601-605`); by that reading every keyed container
     (Tabs, Column) would be a choice family.
   - Settled by: whether the stripe behaves as a selection among values or
     as independent show/hide toggles. **The code supports each side in
     part:** Enter or Space "expands or collapses the button's tool window"
     (`:601-607`), and docked and overlay tool windows are exclusive per
     edge group ("at most one expanded tool window per edge group",
     `enforce_groups`, `:986-988`; a moved window "collapses the one
     expanded in its new group", `:570`), which resembles single selection;
     but floating tool windows are not exclusive (`:586-587`), a
     `ToolWindow` carries content and actions, not an option value
     (`:303-307`), and `ToolWindowLayout::new` binds no selection
     (`:765-777`). Whether that counts as a choice family is the policy
     question in G1-16.

3. **The parameter name `state` for a `*Controller` collides with the
   contracts' `*State` role suffix** (claude-C3-2).
   - Claude: "state" for a controller collides with the `*State` suffix
     for passive captured data (contracts "Public naming rules").
   - Codex: that sentence governs type names, not parameter names
     (`docs/contracts.md:1159`).
   - Settled by: the contract sentence. **The text supports Codex on
     scope:** "Cloneable UI-thread handles that drive a widget are
     `*Controller`; passive captured data are `*State`"
     (`docs/contracts.md:1159-1160`) names types. Claude's point is a
     vocabulary echo rather than a claim that the rule applies, so this
     removes the rule basis but not the proposal in G1-15.

## Withdrawn and refuted parts

- Claude's C1 disposition "no findings": withdrawn by Claude after
  codex-C1-1; it had tested an invalid list without a binding change and a
  binding change with valid lists, not both together.
- Codex's C2 disposition "no findings": corrected by Codex, which accepts
  claude-C2-1 because the rule says "across the API".
- Codex's classification of `TextSelection::new` as documented
  control-limit clamping: withdrawn by Codex (explicit offsets are inputs).
  Claude's verification had proposed it as a policy question; that is moot.
- Codex's classification of `ProgressStatus` without a finding: corrected by
  Codex, which accepts claude-C4-5 as a policy question (G1-12).
- claude-C4-2, `PresentationRate::hertz()` instead of `get()`: withdrawn by
  Claude; the unit exception applies (`runtime/presentation.rs:4`,
  `:23-25`).
- claude-C2-2, medium severity and "a consumer passing CSS values gets
  visibly lighter colours with no error": withdrawn by Claude; 0 to 255
  channels are rejected (`drawing.rs:259-262`), only normalized sRGB
  fractions are misread.
- claude-C2-2 as a departure from the C2 prefix proposal: refuted by Codex
  (`from_srgb` keeps the `from_` meaning), conceded by Claude; kept as a
  consistency proposal (G1-14).
- claude-C4-4, `ParameterId::key`: refuted by Codex and not withdrawn; see
  Disputed facts 1.
- claude-C4-3, `PowerExponent::MIN`/`MAX` typed `f32` as a departure:
  refuted by Codex (neither C4 nor the contract requires `Self`); kept as a
  sub-question of G1-9.
- claude-C3-2 as a C3 departure, and its `*State` argument: refuted by
  Codex (C3 governs order); kept as proposal G1-15; see Disputed facts 3.
- codex-C4-2, `Color::channels`, `Rect::values`, `Transform2D::entries` as
  departures: not established per Claude (no single raw value); Codex had
  already marked their applicability a policy question (G1-6).
- codex-C4-3, intrinsic zero, one and nonnegativity bounds as departures:
  not established per Claude; Codex had already marked them a policy
  question (G1-10).
- codex-C1-2 as a C1 project-rule departure: not established per Claude;
  Codex maintains it as a policy question only (G1-16). Its
  `ToolWindowLayout` instance: Claude would drop it, Codex maintains it; see
  Disputed facts 2.
- codex-B3-1, medium severity and the event-phase-notification fix:
  corrected by Claude to low and a private `Rc<Cell<bool>>` (G1-17); Codex
  did not respond.
