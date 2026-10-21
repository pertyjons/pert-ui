# API audit findings

Slice 4 of [the developer-usability plan](../../developer-usability.md):
the merged result of the API consistency audit, audited at commit d4ba108.
[The rubric](rubric.md) defines the items, the rule kinds and the finding
record.

## How the result was reached

- **Inputs:** [`inventory.py`](inventory.py) built the frozen inputs. Its
  versions are recorded in [`inputs/CHANGES.md`](inputs/CHANGES.md).
- **Independent audit:** two auditors, Codex and a fresh Claude agent per
  rubric group, audited independently. Their reports are under
  [`reports/`](reports/), with the brief and each group's assignment.
- **Cross-verification:** each auditor verified the other's findings
  against the source and answered disputes of its own
  ([`reports/verification/`](reports/verification/)).
  - Both reviewed what later inventory versions added (the inventory
    deltas, and `*-delta3.md`).
  - Each verified the other's delta findings (`*-dverify.md`).
- **Merging and disputes:** a fresh Claude agent per group drafted a merge
  sheet ([`reports/merge/`](reports/merge/)). The author settled each disputed fact against the source; the
  "Author's resolution" sections record it. Policy questions are left to
  the maintainer.

**Statuses:**
- **`both`:** both auditors reported it.
- **`confirmed`:** one reported it and the other confirmed it.
- **`partly`:** only the confirmed instances are kept, and the refuted ones
  are listed.
- **`policy`:** a question for the maintainer.

Withdrawn and refuted parts are listed per group.

**Surveys:** the [lint survey](lints/survey.md) and the [diagnostics
survey](diagnostics/survey.md) are separate records.

## Summary

127 findings remain after verification:
- **Severity:** 15 findings are medium or have a medium instance, and the
  rest are low. No finding is high after verification.
  - The two first rated high were lowered to medium. G1-4 (a workspace
    that cannot be loaded after `set_settings`) is triggered only by a
    settings line near the size limit. G4-25 (a protected semantic value)
    leaks only through the component's own author.
- **Policy questions:** the policy questions some of them carry are
  listed below, grouped.

### Medium severity

| # | Finding | Fix (breaking?) | Slice 3 |
|---|---|---|---|
| G1-4 | `Workspace::set_settings` skips the document-size check, so a saved workspace can fail to load. | Check the limit, by a fallible setter (breaking) or reserved room in `new`/`decode` (not breaking). | - |
| G2-2 | `Table`, `VirtualList`, `Tree` and `FileBrowser` keep their first description's permissions on same-key redescription, against contracts "Permissions and availability". | Adopt the new permission, or name the exception in contracts (policy). | - |
| G2-9 | `ParameterHost`, `AnyParameterHost` and `SeriesSource` tear down state on drop but lack `#[must_use]`. | Add it with a reason (not breaking). | - |
| G2-13 | Numeric reconciliation cancels an active parameter session and publishes `Parameter::snapshot` at once, against "never publish signals from reconciliation" (the parameter path; the `ContextMenu` instance is low). | Defer the cancellation's publication to the next safe point, or name the exception (policy). | - |
| G2-14 | The collections ignore new callbacks on same-key redescription, unlike Tabs. | Replace them as Tabs does, or name the exception (policy). | - |
| G2-20 | `on_rejection` covers built-in rejections on the text editors but not on the number inputs. Whether that is a departure is a policy question; the problem is F6's. | Report them (a behavior change), or state the difference in contracts. | F6 |
| G3-1 | `status()` and `snapshot()` return owned values on some types and signals on others. The reactive form has four spellings. | One convention: `x()` owned, `x_binding()` reactive (breaking). | - |
| G3-6 | `pert_ui_wgpu::Renderer` takes physical sizes as two bare `u32`. | Take `PhysicalExtent` (breaking). | - |
| G3-11 | `Component::tab_navigation(cx, reverse, repeat)` takes two adjacent `bool`s a consumer's implementation can swap silently (the other boolean flags in the row are low). | An enum or a direction type (breaking). | - |
| G4-1 | 113 public types lack `Debug`, among them the state handles applications store. | Hand-written `Debug`, state owners first (not breaking). | - |
| G4-14 | Key and identity errors (`AmbiguousKey`, `DuplicateKey`, `ChildTypeChanged`) do not name the value. | Carry it (breaking). | F23 |
| G4-25 | `Semantics` and `SemanticItem` can expose a protected value through public fields. | Normalize at `describe_component` (not breaking), or make the fields private. | - |
| G5-2 | `EventContext::defer` does not lead to `Runtime::spawn`. | Cross-links and a guide example. | F13 |
| G5-3 | Per-row editable bindings are composed without a documented route. | Cross-links and a recipe. | F14, F16 |
| G5-4 | `DraftEditor`'s owner contract does not name `retain_initial_children`. | Cross-link. | F17 |

### Policy questions

Each question is the maintainer's to decide. The finding rows give the
evidence and the alternatives.

**Construction:**
- **G1-3:** may an identity key lead keyed-item constructors ahead of the
  bound state?
- **G1-5:** may `ChoiceId::as_str` keep its name as a string exception to
  `get()`, or is `get` added?
- **G1-6:** does the `get()` rule cover compound values such as
  `Color::channels`?
- **G1-9:** where does a limit shared by several types live, and what type
  does its constant have?
- **G1-10:** do intrinsic zero, one or nonnegativity bounds need `MIN_*` or
  `MAX_*` constants?
- **G1-12:** may a documented convenience clamp, as `ProgressStatus::of`
  does and as `Table`'s width setters do (see V3-2)?
- **G1-13:** which of its four meanings does `with_` keep?
- **G1-14:** should `Color::rgba` be named for linear colour beside
  `from_srgb`?
- **G1-15:** do parameters use `ui` and `controller` throughout?
- **G1-16:** do lazy collections and keyed content fall under "Choice
  sources"?

**Builders and callbacks:**
- **G2-1:** do no-argument boolean shortcuts (`Button::frameless`,
  `QuickPick::nonmodal` and others) get `bool` setters, or does the
  contract allow them?
- **G2-2, G2-14:** do the collections adopt a redescription's permissions
  and callbacks as Tabs does, or does contracts name their exception?
- **G2-3:** do `FloatingWindow::closable`, `minimizable` and
  `App::resizable` take `Permission` like their siblings?
- **G2-4:** do `disabled(bool)` and `read_only(bool)` on `PickItem`,
  `SemanticItem` and `Semantics` follow the no-argument naming rule?
- **G2-5:** are the accumulating builders (`FloatingWindow::title_action`,
  `ToolWindowLayout::window(s)`, `Semantics::action(s)`) family exceptions
  to document, or do they replace?
- **G2-6:** is there a convention for two-way boolean bindings
  (`Collapsible::bound` against `ToolWindowLayout::names`)?
- **G2-8:** do receipts whose failures also propagate (`TaskReceipt`,
  `ToastReceipt`) keep `#[must_use]`? This joins slice 3's F19.
- **G2-13:** may reconciliation publish a private signal nothing can
  observe (`ContextMenu`)? The parameter path's public publication is a
  finding regardless.
- **G2-17:** do the settled selection notifications share one name
  (`Tabs::on_select`, the collections' `on_selection_change`)?
  contracts.md:527-534 documents a difference between them, which is why
  Claude withdrew the same pair under N2.
- **G2-18:** does `MultiSelect` take a matcher value like `QuickPick`?
- **G2-19:** does `Runtime::set_wake_delivery` take `impl Fn` like the
  other closure-taking functions?
- **G2-20:** does B3's allowance for an editor's "own documented
  contracts" cover rustdoc, or does an exception need contracts.md? The
  problem is F6's either way.

**Names and roles:**
- **G3-16:** may `ParameterState`, an active state machine, keep the
  `*State` suffix?
- **G3-17:** does "passive captured data are `*State`" cover snapshots,
  inspection records and reports, or are `*Snapshot` and `*Inspection`
  named as suffixes?
- **G3-18:** does `*Outcome` cover operation results?
- **G3-19:** is an application-owned coordinator such as `FormStatus` a
  controller?
- **G3-21:** does the receipt rule exempt cross-thread one-time replies
  (`Pending<T>`)?

**Types:**
- **G4-6:** are component descriptions uniformly `!Send`, or do `Divider`
  and `Svg` document their `Send`?
- **G4-7:** should styled text become `Send` so a worker can build it?
- **G4-8, G4-9:** do the growing inspection records and status enums
  become `#[non_exhaustive]`?
- **G4-10:** may `AnyValue` and `AnySpec` be returned as errors, as std's
  array `TryFrom` does, or do they get a typed mismatch error?
- **G4-15:** is losing `Copy` on four error enums acceptable, to carry
  G4-14's rejected values?
- **G4-19, G4-20:** may standard-trait implementations (`TryFrom`,
  `FromStr`, serde) forward their `# Errors` to the type's documented
  rules?
- **G4-21:** should `DrawError` convert into `RuntimeError` (F11's second
  half)? contracts.md excludes it today.
- **G4-22:** does `RuntimeError` get `From<Box<dyn Error + Send + Sync>>`
  (F30)?
- **G4-23:** may user-message carriers (`InputRejection`, `ListingError`)
  keep capitalized `Display` text?

**Discovery:**
- **G5-11:** does `SeriesSource` follow the live widgets into the prelude?
- **G5-16:** are `ParameterHost`'s sink and monitor types "hosts" or "host
  integration"?
- **G5-26, G5-27:** how far does the prelude rule's "every type its
  signatures name" reach: fields, payloads, constructor parameters, one-hop
  types?
- **G5-30:** is `WindowError` in the facade prelude?
- **G5-32:** does slice 9 add a testing-oriented prelude for capture and
  host types?

## G1: Construction (C1–C4)

Inputs: `reports/codex-G1.md`, `reports/claude-G1.md`,
`reports/verification/codex-G1.md`, `reports/verification/claude-G1.md`
(all under `plans/usability/api-audit/`), audited commit `d4ba108`; `crates/`
and `docs/` are unchanged at HEAD `245053b`, where the merger read the
source. Runtime paths are relative to `crates/pert-ui-runtime/src/` unless
prefixed `core/` (`crates/pert-ui-core/src/`) or `facade/`
(`crates/pert-ui/src/`).

### Findings

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
| G1-18 | C4, consistency proposal (C-NEWTYPE) | `SplitRule::Ratio(f32)` (`split.rs:141-145`), produced by `SplitSizing::rule` (`:123-133`) and reported through `SplitPane::on_resize` (`:322`), documents a `0..=1` share but carries a raw `f32`, while its siblings carry `Length` and `NormalizedValue` exists (used by `ProgressStatus::Determinate`, `progress.rs:30`). No public function takes a `SplitRule`. | signature | all | low | high on facts | `SplitRule::Ratio(NormalizedValue)`. Breaking (variant payload). | TODO.md, or the checkpoint with G1-10's unit-interval question | claude-C4-d1 (Claude's inventory delta); **confirmed** by codex as a proposal | - |

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

### Disputed facts

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

#### Author's resolution

The author settled each disputed fact against the source the merge sheet cites:
- **1, `ParameterId::key`:** Codex holds. `key` is one of four fields
  (`crates/pert-ui-core/src/parameter_id.rs:44-49`, `:99`), not the
  whole value, so it is no `get()` departure. The instance leaves G1-5.
- **2, `ToolWindowLayout` as a choice family:** not a fact either way.
  Docked windows are exclusive per stripe group, floating ones are not, and
  a window carries content rather than an option value. The question stays
  with the maintainer as policy question G1-16.
- **3, the parameter name `state`:** Codex holds on the rule basis.
  `docs/contracts.md:1159-1160` governs type names, so the `*State` argument
  falls away. Proposal G1-15 remains as a consistency proposal.

### Withdrawn and refuted parts
- **G1-17** (codex-B3-1, `ContextMenu::reconcile` publishing a private
  signal) is removed from G1. It duplicated the `ContextMenu` instance of
  G2-13, where it is kept.

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

## G2: Builders and callbacks (B1–B3)

Inputs: `plans/usability/api-audit/reports/{codex,claude}-G2.md` and
`reports/verification/{codex,claude}-G2.md`. Both audits read commit
`d4ba108`; `crates/` is unchanged at HEAD `245053b`, where the source checks
below were made. Source paths without a crate prefix are under
`crates/pert-ui-runtime/src/`. Neither verification answered the other's
verification (each answered the other's original report), so a correction
made in one verification and not contested in the other is applied here.

### Findings

| # | Rubric item, rule kind | Finding (one or two sentences, naming the items) | Sig/beh | Configs | Severity | Confidence | Fix (breaking?) | Destination | Sources and status | Slice 3 |
|---|---|---|---|---|---|---|---|---|---|---|
| G2-1 | B1, project rule; policy question on scope | No-argument builders set one ordinary boolean with no `bool` setter, although contracts.md:1156-1158 reserve no-argument shortcuts for `disabled()`/`read_only()`: `Button::frameless` (controls.rs:604), `Group::frameless` (views.rs:166), `Field::hide_label` (field_status.rs:493), `InspectionQuery::{parameters, windows, sources, validation, application}` (inspection.rs:246-276), `Semantics::names_children` (diagnostics.rs:631), `PointerEvent::outside` (pointer.rs:189; claude: a weak instance, it marks a synthetic event value). Added by claude in verification and confirmed by codex: `RegionLayout::fixed` (region_layout.rs:151), `Divider::vertical` (views.rs:928), `Toolbar::attached_below` (toolbar.rs:479), `Dialog::require_response` (dialog.rs:226), `QuickPick::nonmodal` (quick_pick.rs:920), `CommandPalette::nonmodal` (command_palette.rs:540), `pert_ui::App::software_rendering` (pert-ui/src/app.rs:412). Policy question: should the rule permit these one-way shortcuts? | sig | all; `App::software_rendering`: cpu, no-default-desktop-cpu | low (both) | codex medium on applicability, high on facts; claude confirms | Document the shortcut convention (nonbreaking), or add `bool` setters (additive) / change signatures (breaking) | TODO.md | codex-B1-1; confirmed by claude (instances extended, siblings `ContentQuery::hidden(bool)`, `CaptureRequest::include_overlay(bool)` take `bool`); claude's added instances verified by codex: seven confirmed (`RegionLayout::fixed`, `Divider::vertical`, `Toolbar::attached_below`, `Dialog::require_response`, `QuickPick::nonmodal`, `CommandPalette::nonmodal`, `App::software_rendering`), `Element::structural` and `View::fixed` refuted (not ordinary boolean properties); status: confirmed, policy | - |
| G2-2 | B1, project rule; policy question (scope of the controller exception) | A same-key redescription keeps the first description's permission and binding for `Table::resize_permission`, `Table::reorder_permission`, `VirtualList::reorder_permission`, `Tree::reorder_permission` and `FileBrowser::folder_filter` (table.rs:1218-1222, 1389-1391, 1429-1431; virtual_list.rs:428-437, 982-985; tree.rs:885-891, 1033-1035; file_browser/view.rs:106; test pert-ui/tests/audit/audit_collection_permissions.rs:685-737), against contracts.md:308-309. Policy question: does contracts.md:169-171 "controller and options" cover permissions? | beh | all | medium (both) | high on behavior (both); exception scope open | Adopt the new permission on compatible redescription (behavior break), or name the exception in contracts "Permissions and availability" | TODO.md | claude-B1-1; confirmed by codex, which corrected its own roles-table disposition ("initial controller/options/callback exceptions apply") to an unresolved exception-scope question; status: confirmed, policy | - |
| G2-3 | B1, project rule; policy question on scope | `FloatingWindow::closable(bool)` and `minimizable(bool)` (windows.rs:327, 333) take a `bool` and replace, while `movable`/`resizable` on the same type take `impl Into<Permission>` and compose (windows.rs:346, 412); `pert_ui::App::resizable(bool)` (pert-ui/src/app.rs:375) likewise. Policy question: are title-button presence and a startup option "capability restrictions"? | sig | all; `App::resizable`: cpu, default, image-formats, images, no-default-desktop-cpu, serde, svg | low (both) | claude medium; codex confirms facts | Take `impl Into<Permission>` (calls compile, `false` then `true` changes behavior), or document them as fixed options; codex: Permission conversion is not automatically the right fix | TODO.md | claude-B1-2; confirmed by codex (title buttons built conditionally at windows.rs:538, 562); status: confirmed, policy | - |
| G2-4 | B1, project rule; policy question on scope | `PickItem::disabled(bool)` (quick_pick.rs:78), `SemanticItem::disabled(bool)` (diagnostics.rs:392), `Semantics::disabled(bool)` (diagnostics.rs:579) and `Semantics::read_only(bool)` (diagnostics.rs:598) take a `bool` under the names contracts.md:1158 reserves for no-argument shortcuts; the `disabled` setters also clear offered actions (diagnostics.rs:394, 581). Policy question: does the rule apply to descriptive data builders? | sig | all | low (both) | high on facts (both); applicability open | Rename (breaking), or record that data descriptions take the state as `bool` (nonbreaking) | TODO.md | claude-B1-3; confirmed by codex (its roles table already called these descriptive booleans); status: confirmed, policy | - |
| G2-5 | B1; claude: project rule, codex: proposal; policy question | Builders that add rather than replace, with no family exception in contracts.md: `FloatingWindow::title_action` (windows.rs:421-426), `ToolWindowLayout::window`/`windows` (tool_windows/mod.rs:788-809), `Semantics::action`/`actions`, `SemanticItem::action` (diagnostics.rs:420, 556, 566); related builders replace (`ToolWindow::actions`, tool_windows/mod.rs:457; `Semantics::items`, diagnostics.rs:571). Policy question: are list-element composition builders ordinary properties, and should contracts name them? | beh | all | low (both) | claude medium; codex confirms accumulation | Name accumulating composition in contracts "Public naming rules" (e.g. `add_*`), or make `title_action` take all elements at once; codex: a type or behavior change can break callers even without a rename | TODO.md | claude-B1-4; codex partly confirmed: accumulation confirmed for every instance, classification as an ordinary-property violation not established (correction applied: rule kind is the policy question); status: confirmed, policy | - |
| G2-6 | B1, consistency proposal | Two-way boolean state bindings have no convention: `Collapsible::bound(Signal<bool>)` with `expanded(bool)` (collapsible.rs:533, 525) against `ToolWindowLayout::names(Signal<bool>)` with `show_names(bool)` (tool_windows/mod.rs:924, 916); `bound` is also the choice-source constructor name. | sig | all | low (both) | high on facts | One convention, e.g. `<property>_binding(Signal<bool>)` (breaking) | TODO.md | claude-B1-5; confirmed by codex (choice-source rules do not prohibit these builders); status: confirmed, policy | - |
| G2-7 | B2, project rule | `CommandId` (runtime/commands.rs:377-380) and `HeldInputId` (runtime/held.rs:5-8) are `#[must_use]`, but discarding them keeps the registration (commands.rs:565; held.rs:115), so application-lifetime registrations need `let _ =` (guide/diagnostics.md:36). | sig | all | low (both) | high (both) | Remove the type-level attribute; keep `Result` handling and the unregister APIs (nonbreaking) | TODO.md | codex-B2-1, claude-B2-3 (identity part), matched in claude's verification; status: both | F19 (extends to `HeldInputId`) |
| G2-8 | B2, project rule; policy question (receipt discard) | `TaskReceipt` (runtime/tasks.rs:27) and `ToastReceipt` (notifications.rs:195) are `#[must_use]` although their failures also propagate (tasks.rs:104-105; notifications.rs:646), and the guide and examples discard them with `let _receipt` (guide/surfaces.md:314; examples/diagram_editor/app.rs:600, 821, 852; examples/dynamic_documents.rs:26). Policy question: keep receipts `must_use` by design, or only where dropping loses the only failure report? | sig | all | low (both) | high on facts; policy open | Remove the attribute where the outcome is also reported, keep it where the receipt is the only report (codex: sound; unconditional removal is not), or document receipts as must_use by design with the `let _ =` idiom (nonbreaking) | TODO.md | claude-B2-3 (receipt part); codex partly confirmed: Task and Toast confirmed; Command, Reveal, ListReveal and Presentation receipts refuted, see D1; status: partly, policy | F19 (adjacent) |
| G2-9 | B2, project rule | Owners whose drop tears down state lack `#[must_use]`: `ParameterHost<T>` (parameter_binding.rs:495; Drop removes the parameter, :626-635), `AnyParameterHost` (parameter_binding/erased.rs:255), `SeriesSource<T>` (live.rs:627) and `Runtime::series_source` (runtime.rs:4075; the source holds the consumer in an `Rc`, live.rs:645-650); siblings `RemoteHost` (remote.rs:420) and `SeriesConsumer` (core live/series.rs:292) carry a reason. | sig | all | medium (both) | high (both) | Add `#[must_use = …]` with a reason; for `SeriesSource` the reason must say dropping the last source (all clones) closes the bridge (correction from codex's verification). Nonbreaking | TODO.md | codex-B2-2 and claude-B2-1 (`ParameterHost`, `AnyParameterHost`): both; `SeriesSource`/`Runtime::series_source` (claude only): confirmed by codex; status: both | - |
| G2-10 | B2, project rule | Signal-creating functions lack `#[must_use]`, so a created and discarded signal goes unnoticed: `Runtime::signal` (runtime.rs:4048), `StateOwner::signal`, `EventContext::signal`, `UpdateContext::signal` (runtime.rs:2604, 2909), `Signal::sibling` (signal.rs:66, 327); comparable controller constructors carry it. | sig | all | low (both) | high (both) | Function-level `#[must_use]`, not type-level (nonbreaking) | TODO.md | claude-B2-2; confirmed by codex; status: confirmed | - |
| G2-11 | B2, project rule | `PresentationRequest` (runtime/native_windows.rs:1011-1015; Drop cancels, :1050-1057) and `PreparedDetach` (native_windows.rs:241-245) carry a bare `#[must_use]`, while comparable tokens give a reason (`DirectoryRequest`, file_browser.rs:196; `RequestTicket`, async_source.rs:28; `Pending`, inspection.rs:565). Correction applied: `PreparedDetach` has no Drop impl; dropping it leaves the movement uncommitted. | sig | all | low (both) | high (both) | Add reasons (nonbreaking) | TODO.md | claude-B2-4; confirmed by codex with the correction; status: confirmed | - |
| G2-12 | B3, project rule (geometry notifications) | `RegionLayout::on_resize` promises that removing the callback drops undelivered changes (region_layout.rs:253-255), but it always installs a forwarding `SplitPane::on_resize` closure (region_layout.rs:363-372), and SplitPane discards a pending report only when the incoming or previous callback is `None` (split.rs:496-500); a report recorded under one callback reaches a callback reinstalled before delivery (contracts.md:355-356). Claude adds, confirmed by codex (`reports/verification/codex-v4.md`): a Restore or Redescription recorded while the layout had no callback reaches a callback installed in the same flush, when the retained preference differs from the reporting baseline. | beh | all | codex medium, claude low; propose low: it needs a callback to appear between the record and its delivery in one flush (a delivery while the slot is empty consumes the report, region_layout.rs:367-369), and it delivers an extra report of the current sizing, not wrong data | high (codex, by source tracing, not executed; claude confirms) | Keep callback absence when building the forwarding splits, or propagate removal to their reports; add a remove-then-reinstall-before-flush test (nonbreaking) | TODO.md | codex-B3-1; confirmed by claude; status: confirmed | - |
| G2-13 | B3, project rule; policy question (lifecycle exception) | Reconciliation publishes signals: `ContextMenu::reconcile` publishes its private `enabled_signal` (menu.rs:1472); numeric reconciliation abandons an active parameter session on rebind or fixed denial (numeric_controls.rs:568, 585; text_fields/number_inputs.rs:1274, 1309), whose `EditSession::drop` cancels and publishes the public `Parameter::snapshot` at once (parameter_binding.rs:902, 678, 178; signal.rs:431), only the terminal event being deferred. Policy question: is immediate lifecycle cleanup an exception to AGENTS.md:132? Codex reads contracts.md:167 ("cancelled at the next safe point") as the cleanup requirement; claude reads it as departed from, since the state is published at once. | beh | all | parameter path medium (both); ContextMenu instance codex medium, claude low; propose low for that instance: its only reader is `ContextMenuOpener::open`, an event-time `get()` (menu.rs:1360-1361), and no component or computation can subscribe to the private field | high on the publication paths (both) | ContextMenu: non-signal shared state; parameters: defer the cancellation's publication, or record a narrow lifecycle exception in contracts. Private storage need not break API; changing snapshot timing needs contract review | TODO.md | codex-B3-2; claude reviewed the ContextMenu instance without a finding, then withdrew that and accepted it as a low instance; parameter path confirmed by claude; status: both (ContextMenu), confirmed (parameters), policy | - |
| G2-14 | B3, project rule; policy question (scope of the controller exception) | Collection callbacks belong to the first description; a same-key redescription ignores new ones: `VirtualList::{on_selection_change, on_activate, on_text_input, reorderable, placement}`, `Tree::{on_selection_change, on_activate, reorderable}`, `Table::{on_selection_change, on_activate, on_sort, cell_editor}` (virtual_list.rs:428-437, 807, 839-841; tree.rs:885-891, 1052-1053; table.rs:1218-1222, 1554-1555, 1579-1581); `Tabs::on_select`, the list's cited model, replaces and removes (tabs.rs:950-953). Policy question: does contracts.md:169-171 "options" cover callbacks? ColorPicker's handler follows redescription (contracts.md:478-480). | beh | all | medium (both) | high on behavior (both); exception scope open | Replace and allow removal on compatible redescription as Tabs does (behavior break), or name the exception in contracts "Component redescription". Codex: contracts.md:457 (user-write) does not govern all these families | TODO.md | claude-B3-1; codex partly confirmed: retention confirmed for all instances, the unambiguous project-rule breach disputed; codex corrected its own disposition to the same scope question; status: confirmed, policy | - |
| G2-15 | B3, project rule (signature shape) | `Button::command`, `Button::command_icon` and `MenuItem::command` take `on_receipt: FnMut(CommandReceipt, &mut EventContext)` (controls.rs:552, 592; menu.rs:404), while every other UI-thread component callback takes `&mut EventContext` first (e.g. breadcrumb.rs:182, quick_pick.rs:891); no reason is documented. | sig | all | low (both) | high (both) | `FnMut(&mut EventContext, CommandReceipt)` (breaking); owned receipt delivery stays | TODO.md | claude-B3-3; confirmed by codex; status: confirmed | - |
| G2-16 | B3, project rule (signature shape) | `Accordion::on_change(FnMut(&mut EventContext, &[String], ChangeOrigin))` (collapsible.rs:1049, delivered at :1249) passes the origin positionally, while `Tabs::on_select` (`&SelectionChange`, tabs.rs:607-609), the collections' `on_selection_change` (`&ListSelectionChange`, list_selection.rs:86) and `ToolWindowLayout::on_change` (`&ToolWindowChange`, tool_windows/mod.rs:218) pass one change struct; no reason is documented. Correction applied: the missing previous state is not a family departure (`ToolWindowChange` has none); adding `previous` is a proposal. | sig | all | low (both) | high (both) | An `ExpansionChange` payload passed by reference (breaking); `previous` as an optional proposal | TODO.md | claude-B3-4; codex partly confirmed (shape confirmed, previous-state part refuted); status: partly | - |
| G2-17 | B3, consistency proposal | The same settled selection notification has two names: `Tabs::on_select` (tabs.rs:607) and `VirtualList`/`Tree`/`Table::on_selection_change` (virtual_list.rs:872, tree.rs:1054, table.rs:1556), whose rustdoc defines its timing by `Tabs::on_select` (virtual_list.rs:834-836). | sig | all | low (both) | high on facts | One name, e.g. `on_selection_change` on Tabs (breaking; no change of timing) | TODO.md | claude-B3-5; confirmed by codex; status: confirmed, policy | - |
| G2-18 | B3, consistency proposal (narrowed in verification); policy question whether match rules are a callback family | `MultiSelect::filter(impl Fn(&str, &Choice<T>) -> bool)` (multi_select.rs:1382) takes a bare closure, while `QuickPick::matcher(PickMatcher)` (quick_pick.rs:926), `CommandPalette::matcher(CommandMatcher)` (command_palette.rs:546) and `CollectionFilter::matcher(FilterMatcher)` (collection_filter.rs:587) take a matcher value with presets; `filter` is also the controllers' query operation (table.rs:707). | sig | all | low (claude) | claude high on facts, medium on applicability | An optional matcher value or adapter beside `filter`, keeping access to the choice value that `filter` gives; replacing `filter` outright would lose it (additive) | TODO.md | claude-B3-d1 (claude's inventory delta); **partly** confirmed by codex: the facts hold, but the policies take different inputs, so it is a low consistency proposal, not a B3 departure | - |
| G2-19 | B3, consistency proposal (narrowed in verification) | `Runtime::set_wake_delivery(&self, deliver: Box<dyn Fn() -> bool + Send + Sync>)` (runtime.rs:4183) is the only closure-taking public function that requires a `Box`; its sibling `pert_ui_wgpu::Renderer::new` takes `impl Fn(Notice) + Send + Sync + 'static`, and no reason is documented. | sig | all | low (claude) | high (claude) | `impl Fn() -> bool + Send + Sync + 'static`, boxed inside (callers passing a `Box` still compile) | TODO.md | claude-B3-d2 (claude's inventory delta); **partly** confirmed by codex: the boxing burden is real, but host wake delivery is a separate callback contract, so it is a low consistency proposal | - |
| G2-20 | B3; **policy question** on the rubric's reading | `on_rejection` reports built-in rejections on `TextField`/`TextArea` (their length limits) but not on the number inputs, which reject parse and range errors without calling it; the number inputs' rustdoc still says "with the rules of `TextField::on_rejection`". The difference is documented in the number inputs' rustdoc, not in contracts.md. **Question:** does the rubric's B3 allowance for an editor's "own documented contracts" cover rustdoc, or does an exception need a contracts.md sentence? Either way the usability problem stays slice 3's F6. | beh | runtime, all | medium (the F6 problem) | high on facts | Report built-in rejections through `on_rejection` on the number inputs too (a behavior change), or state the difference in contracts.md and correct the rustdoc (documentation). | Maintainer; the problem stays F6's | claude-B3-2; refuted by codex as a documented editor difference; the classification is a policy question (see the author's resolution of D2) | F6 |

### Disputed facts

#### D1. Do the command, reveal, list-reveal and presentation receipts carry outcomes reported nowhere else? (claude-B2-3, receipt part)

- **Claim (claude):** discarding a receipt is normal, so `CommandReceipt`
  (runtime/commands.rs:1606), `RevealReceipt` (runtime/reveal.rs:132),
  `ListRevealReceipt` (virtual_list.rs:229) and `PresentationReceipt`
  (runtime/native_windows.rs:949) should not be `#[must_use]`, like
  `TaskReceipt` and `ToastReceipt`. Evidence: `ListRevealReceipt`'s rustdoc
  tells callers to discard it explicitly (virtual_list.rs:223-226), and the
  reasons on reveal.rs:132 and tasks.rs:27 are conditional ("retain the
  receipt if the application needs …").
- **Counter (codex):** these four carry outcomes that are not otherwise
  available: command errors are retained, not returned
  (runtime/commands.rs:1655), reveal results are stored (runtime/reveal.rs:680),
  virtual_list.rs:228 calls the list receipt the only report, and
  presentation completion stores its result (runtime/native_windows.rs:1044).
- **What the source shows:**
  - `CommandReceipt`: supports codex. `flush_commands`' rustdoc says "An
    action's own error is retained in its receipt, not returned"
    (runtime/commands.rs:1655-1657); `CommandStatus::Finished` holds the
    `Result` (commands.rs:1589). I found no other report in the code read.
  - `RevealReceipt`: supports codex. `RevealStatus::Finished(Result<…>)`
    (runtime/reveal.rs:115) is stored by `dispatch_reveal_requests`
    (reveal.rs:665-687) and not returned; the type's reason is conditional
    (reveal.rs:132), which is claude's point.
  - `ListRevealReceipt`: the rustdoc states both sides. Discarding "leaves
    the request in place; it only gives up its outcome, so a caller that
    does not need it discards it explicitly" (virtual_list.rs:223-226), and
    the attribute's reason says "the receipt is the only report of whether
    the row was found and mounted" (virtual_list.rs:227).
  - `PresentationReceipt`: supports claude for `App` consumers. `App`
    reports failed presentation through `on_window_error`, "The receipt
    retains the typed failure independently" (crates/pert-ui/src/app.rs:476-477;
    the doctest at app.rs:277-279 discards it with `let _receipt`). For a
    host that completes requests itself (`PresentationRequest::complete`,
    native_windows.rs:1040-1047), the receipt is the only report.
- **What settles it:** the facts above are not in conflict once separated:
  the remaining question is whether "discard is a mistake" (AGENTS.md:181)
  means "loses the only report of an outcome", which is the policy question
  in G2-8. Codex judges claude's conditional fix sound.

#### D2. Is the `on_rejection` coverage difference explained by a contract? (claude-B3-2)

- **Claim (claude):** `TextField`/`TextArea::on_rejection` reports length
  limits (text_fields.rs:306-308), the number inputs' `on_rejection` reports
  only the normalizer (number_inputs.rs:2089-2091) while opening "with the
  rules of [`TextField::on_rejection`]" (number_inputs.rs:2086-2087), and no
  contract section explains the split; one name, two coverages in one editor
  family. Severity medium; slice 3 F6.
- **Counter (codex, refuted as a B3 departure):** the difference is
  documented at number_inputs.rs:2086-2095; the shared reporter reports only
  `EntryError::Rejected` (text_fields/normalize.rs:114), number validation
  produces a different error (number_inputs.rs:1039), and length checks
  deliberately produce an `InputRejection` (text_fields.rs:1410). Rubric
  B3's editor clause (rubric.md:221-222) exempts documented differences;
  both callbacks keep the same save-attempt timing.
- **What the source shows:** both sides' citations are accurate.
  - The item rustdoc documents the exclusion (number_inputs.rs:2089-2091),
    and TextField's rustdoc states the matching rule from its side: "Only
    [`InputRejection`]s are reported, not the field's own text checks"
    (text_fields.rs:317-318). `check_length` returns an `InputRejection`
    (text_fields.rs:1410), while number validation returns a
    `NumberInputError` (number_inputs.rs:1039-1045). So both editors report
    only `InputRejection`s; what differs is which built-in check is an
    `InputRejection`. The timing is the same.
  - docs/contracts.md "Text and draft editing" (contracts.md:911-926) and
    "Editor sessions" contain no sentence about `on_rejection` coverage.
- **What settles it:** whether item rustdoc counts as the editor's "own
  documented contract" (rubric.md:221-222) or an exception must cite a
  contract sentence (rubric.md:116-118), and whether the B3 names rule
  ("one name means one timing", rubric.md:225) covers coverage as well as
  timing. The code supports codex on timing and on the difference being
  documented at the item, and claude on there being no contract sentence.
  The behavior itself remains slice 3's F6 either way.

#### Author's resolution

The author settled each disputed fact against the source the merge sheet cites:
- **D1, receipts:**
  - `CommandReceipt` and `RevealReceipt` report outcomes that nothing
    else reports, so discarding them can lose information (Codex).
  - `PresentationReceipt` under `App` does not, because `on_window_error`
    also reports the failure (Claude).
  - `ListRevealReceipt`'s rustdoc states both sides.

  The facts are as the sheet records them. Whether `#[must_use]` belongs
  on each receipt is policy question G2-8.
- **D2, `on_rejection` coverage:** the facts are not in dispute. Both
  editors behave as both auditors describe.
  - What is in dispute is how the rubric reads. B3 exempts differences
    explained by an editor's "own documented contracts", and the number
    inputs' rustdoc explains this one. The exception rule asks for "the
    exact contract sentence", and contracts.md's "Editor sessions" has
    none.
  - That is not a fact the author can settle, so G2-20 goes to the
    maintainer as a policy question.
  - The usability problem stays slice 3's F6 either way.

### Withdrawn and refuted parts

- claude's "Reviewed without a finding" for the `ContextMenu` signal
  publication (menu.rs:1472): withdrawn by claude in verification; accepted
  as a low instance of codex-B3-2 (G2-13).
- codex's B1 disposition "No additional verified replacement,
  Permission-composition or revocation departures found": withdrawn by
  codex; it accepts G2-3, G2-4 and G2-6 and narrows G2-5.
- codex's roles-table statement that "initial controller/options/callback
  exceptions apply" to the collections: withdrawn by codex as an
  unqualified disposition; now the exception-scope policy question of G2-2
  and G2-14.
- codex's receipt disposition (no finding): partly withdrawn by codex for
  `TaskReceipt` and `ToastReceipt` (G2-8).
- claude-B2-3, `CommandReceipt`, `RevealReceipt`, `ListRevealReceipt` and
  `PresentationReceipt`: refuted by codex (they carry otherwise unavailable
  outcomes); see D1.
- claude-B3-2 (`on_rejection` coverage): refuted by codex as a B3
  departure (documented editor difference); reinstated as G2-20 by the
  author's resolution of D2.
- claude-B3-4, missing previous state as a family departure: refuted by
  codex, since `ToolWindowChange` has no previous state either
  (tool_windows/mod.rs:218-235); not contested.
- claude-B1-4, classification as an ordinary-property project-rule
  violation: not established per codex (collection composition; AGENTS.md
  "accumulating spans" does not settle it); kept as a policy question.
- claude-B3-1, reliance on the user-write rule (contracts.md:457): codex
  notes it does not govern all the listed callback families; the finding
  rests on contracts.md:164 and the exception scope.
- claude-B2-4, `PreparedDetach` as a drop-cancelling token: corrected by
  codex; it has no Drop impl and dropping leaves the movement uncommitted.
- claude-B1-2's Permission fix: codex notes conversion is not automatically
  correct for button-presence and startup options; either fix stays open.
- codex-B1-1, excluded by claude as not instances: `SplitPane::locked` and
  `ToolWindowLayout::locked` (delegate to Permission builders, split.rs:363,
  tool_windows/mod.rs:902), `ToolGroup::essential`/`collapse_first` and
  `Dialog::view_modal`/`application_modal` (mode selectors, not booleans).
- claude's inventory gap 1 (fallible builders missing from builders.txt):
  disputed by codex as out of the list's stated scope; moot in inventory
  version 2, which lists them as `[fallible]`.

## G3: Names and roles (N1–N5, B4)

Group G3: names and roles (N1, N2, N3, N4, N5, B4). Inputs:
`reports/codex-G3.md`, `reports/claude-G3.md`,
`reports/verification/codex-G3.md` and `reports/verification/claude-G3.md`.
Both audited d4ba108. The source checks in "Disputed facts" were made at
245053b, where the library source has not changed since d4ba108.

Path abbreviations: `C/` = `crates/pert-ui-core/src/`, `R/` =
`crates/pert-ui-runtime/src/`, `W/` = `crates/pert-ui-wgpu/src/`, `F/` =
`crates/pert-ui/src/`.

Agreed dispositions: N1 has no findings in either report. Each auditor
classified the 23 `get` methods as permitted uses, and claude-N2-4 does not
change that. For N5, C-CASE, C-CONV, C-ITER and C-ITER-TY have no findings in
either report. After the correction in G3-12, C-WORD-ORDER has none either.
In verification Codex withdrew its N2 disposition of "no additional findings"
for the proposals it confirmed.

### Findings

Every N2 row, and G3-12, is a **consistency proposal**: adopting it is a
maintainer decision even when both auditors confirm the observation.

| # | Rubric item, rule kind | Finding (one or two sentences, naming the items) | Sig/beh | Configs | Severity | Confidence | Fix (breaking?) | Destination | Sources and status | Slice 3 |
|---|---|---|---|---|---|---|---|---|---|---|
| G3-1 | N2, consistency proposal (anchored on contracts.md:1157, :1161–1162) | Owned and reactive reads share names. `status()` is owned on every `*Receipt` (e.g. `TaskReceipt::status`, R/runtime/tasks.rs:33) but returns `ReadSignal` on `DialogController`, `WindowController` and `FieldHandle` (R/dialog.rs:86). `snapshot()` is reactive on `Parameter` (R/parameter_binding.rs:316) but owned on `AnyParameter`, `DeliveryMonitor` and `ParameterState`. The reactive form is spelled four ways: `*_binding`, `*_signal` (`UiSettings::zoom_signal`, `ThemeManager::{density,mode}_signal`), `observe` (`DeliveryMonitor::observe`) and bare (e.g. `FormStatus::summary`). `Runtime::command_enabled_signal` takes the signal that `command_enabled_binding` returns (R/runtime/commands.rs:1694, :689). | sig | runtime, all (facade re-exports) | medium (both) | high | Adopt the receipt convention generally: `x()` is owned and `x_binding()` is the `ReadSignal`. Rename `*_signal` getters, the reactive `status`/`snapshot` and `observe`. Breaking. Generalising the receipt naming is a policy choice (Codex). | TODO.md; the contract wording is the maintainer's decision | claude-N2-1; **partly**. Codex confirmed the observation and the severity. Codex refutes one point: contracts.md:1157 governs boolean builders and does not reserve `_signal` for inputs. That supports neither the claimed restriction nor the "keep `_signal` for inputs only" fix as an existing rule (Disputed facts D5). | - |
| G3-2 | N2, consistency proposal | `reorderable` installs the reorder callback (the capability) on `VirtualList`, `Tree` and `Tabs` (R/virtual_list.rs:970, R/tree.rs:1016, R/tabs.rs:535), but is the Permission restriction on `ToolWindow` and `ToolWindowLayout` (R/tool_windows/mod.rs:403, :846). `resizable(impl Into<Permission>)` is the restriction on `SplitPane` and its siblings (R/split.rs:354), while `Table` uses `resize_permission` (R/table.rs:1421), and `TableColumn::resizable(bool)` and `App::resizable(bool)` declare a capability (R/table.rs:115, F/app.rs:375). | sig | runtime all; `App` in the desktop configurations | low | high | Name every Permission restriction one way (`*_permission` or the adjective). Move the list callbacks to an `on_reorder`-style name. Callback requirements, boolean properties and restrictive composition must stay distinct (contracts.md:296–298, Codex). Breaking. | TODO.md | claude-N2-2; **confirmed** | - |
| G3-3 | N2, consistency proposal | The three role-keyed tables use three accessor pairs: `Palette::color`/`with_color` (R/runtime/theme.rs:856, :875), `ThemeMetrics::get`/`with_metric` (:279, :290) and `UiStrings::get`/`with` (R/strings.rs:625, :601). | sig | runtime, all | low | high | One pair for all three, e.g. `get(role)`/`with(role, value)`. Keep validation and palette fallback semantics (Codex). Breaking. | TODO.md | claude-N2-3; **confirmed** | - |
| G3-4 | N2, consistency proposal | A typed downcast is `DragPayload::get::<T>` (R/drag.rs:53, body `downcast_ref`) but `ApplicationError::downcast_ref::<T>` (R/error.rs:60), where `ApplicationError::get` returns the untyped error (:55). | sig | runtime, all | low | high | Rename `DragPayload::get` to `downcast_ref`. Breaking. N1's classification of both `get`s is unaffected. | TODO.md | claude-N2-4; **confirmed** | - |
| G3-5 | N2, consistency proposal | One concept, two words: an edit that has begun and not yet ended is `EditorSession::is_open` (R/editor_session.rs:188) but `DraftEditorController::is_active` (R/text_fields/draft.rs:375). | sig | runtime, all | low | medium | Pick one of `is_open` or `is_active`. Breaking. | TODO.md | claude-N2-5 (first bullet); **partly**. Codex confirmed this pair. Claude withdrew `on_select`/`on_selection_change` (see Withdrawn). Codex refuted `is_done`/`is_finished` and `show_names`/`names`; the author's resolution (D6, D7) drops both pairs. | - |
| G3-6 | N3, project rule (AGENTS.md "appropriate validated types") | `pert_ui_wgpu::Renderer::{new, new_surface, resize}` take physical-pixel sizes as two bare `u32` (W/lib.rs:391, :1271, :485), while `Renderer::extent()` returns `Option<PhysicalExtent>` (:468). Only `resize`'s rustdoc names the unit. | sig | wgpu default (its only configuration) | medium (both) | high | Take `PhysicalExtent` in all three. Codex: rustdoc alone would not satisfy N3's type-or-name rule, which corrects Claude's "at least name the unit in the rustdoc" fallback. Breaking. | TODO.md | codex-N3-1 (Renderer part), claude-N3-1; **both** | - |
| G3-7 | N3, project rule; the choice of type is a consistency proposal | Logical-pixel lengths are bare `f32` in some builders but `Length` in their siblings (`TableColumn::width`, `PropertyGrid::label_width` and others). The `f32` ones are `CommandPalette::width`, `QuickPick::width` (R/command_palette.rs:611, R/quick_pick.rs:975), `VirtualList::{new, estimated}` (R/virtual_list.rs:635, :659), and `Table::new`, `Tree::new` and `FileBrowser::new` (`row_height`; R/table.rs:1441, R/tree.rs:914, R/file_browser/view.rs:87). `ThemeMetrics::with_metric` takes `Length` while `ThemeMetrics::get`, `MeasureContext::metric` and `Painter::metric` return `f32` (R/runtime/theme.rs:290, :279; R/layout.rs:744; R/drawing.rs:1461). | sig | runtime, all | low. Codex's report gave medium to its aggregate N3-1, and its verification accepts low for ordinary logical lengths. | codex high; claude medium | Take `Length` (with `GeometryError::Empty` for zero) and return `Length` from the metric readers. Alternatively, the maintainer documents "bare `f32` geometry is logical pixels" as a convention, which is a rule change. Breaking for the type change. | TODO.md | codex-N3-1 (these items), claude-N3-2; **both** | - |
| G3-8 | N3, project rule | `AxisMapping` holds domain values and pixels, both as `f64`. `visible(extent)`, `ticks(extent, ..)` and `zoomed_at(.., anchor)` name their pixel quantity only in rustdoc (R/axis_mapping.rs:88, :146, :101; docs at :86, :91, :140). The sibling parameters `pixels_per_unit` and `min_pixels` name it. | sig | runtime, all | low (both) | high | Rename the parameters to `extent_pixels` and `anchor_pixel`. Callers are unaffected because arguments are positional; only doc links change. | TODO.md | codex-N3-1 (AxisMapping part), claude-N3-3; **both** | - |
| G3-9 | N3, project rule | Further logical-pixel geometry as bare `f32`/`f64`: `MeasureContext::fitted_text(.., width: f32, ..)` (R/layout.rs:842), `ListDropPoint::across() -> f32` (R/virtual_list.rs:554), `TextLayout::{paint, paint_turned}(painter, x: f32, y: f32, ..)` (R/text.rs:262, :284; every `Painter` method takes `Point`/`Rect`/`Length`), and the 13 `VirtualAxis` methods (R/virtual_axis.rs:150–372; the unit is stated once in the type rustdoc, :74–75). | sig | runtime, all | codex medium, claude low. **Propose low**: these are logical pixels, the documented default space (docs/rendering-platform.md:9), and Codex's verification itself rates ordinary logical lengths low. The physical/logical risk is G3-6. | high | Use `Point`/`Length` where they fit. `VirtualAxis` keeps `f64` deliberately (R/virtual_axis.rs:77–86), so name the unit or adopt G3-7's convention (Claude). Breaking for type changes. | TODO.md | codex-N3-1 (unmatched part); **partly**. Claude confirmed these and refutes `svg::{rasterize, rasterize_mask}` (D1). | - |
| G3-10 | N3, project rule | `TextSelection::new(layout, anchor: usize, focus: usize)` and `anchor()`/`focus()` are UTF-8 byte offsets, which only rustdoc says (R/text_selection.rs:31, :43–55). `ResourceKind::scene_budget() -> usize` is bytes, which only rustdoc says (R/render_limits.rs:15–21), while the sibling limits name the unit (`RasterImage::MAX_BYTES`, `FontData::MAX_BYTES`). | sig | runtime, all | codex medium for the offsets and low for the budget; claude low for both. **Propose low**: a `usize` text index is a byte index by std's `str` convention, and the library is consistent about it (R/text_fields/draft.rs:7). | high | Name the offsets and accessors with `byte`/`bytes` (e.g. `scene_budget_bytes`). Renaming the getters is breaking; renaming constructor parameters is not. | TODO.md | codex-N3-2; **confirmed** | - |
| G3-11 | N4, guideline C-CUSTOM-TYPE | Boolean operation flags are opaque at call sites: `Runtime::focus_next(reverse)` (R/runtime/pointer.rs:153; call sites need comments, e.g. R/menu_bar.rs:608), `Component::tab_navigation(cx, reverse, repeat)` (R/runtime.rs:1383, the only two-bool function, a public trait method), `EventContext::request_reveal(target, focus)` (R/runtime/reveal.rs:260; `Runtime` splits it into `reveal`/`reveal_and_focus`), `TextSelection::{moved, at_point}(.., extend)` (R/text_selection.rs:62, :87; these compute a value and are not property builders), and `ToolWindowChange::new(key, expanded, origin)` (R/tool_windows/mod.rs:247). | sig | runtime, all | codex medium (low for `ToolWindowChange::new` after verification); claude low. **Propose low, but medium for `tab_navigation`**: each item documents its flag and a wrong value shows on first use, but in the trait method two adjacent bools can be swapped silently by every implementer. | codex high; claude high for the first three, medium for the rest | A `FocusDirection` enum for `focus_next` and `tab_navigation` (`KeyTransition` or an enum for `repeat`), `request_reveal_and_focus`, a selection-extension enum or `moved`/`extended` pairs, and an expansion value. `EditorTrigger::Tab { reverse: bool }` (new in inventory v2) should follow the direction type (Claude). Keep timing and cancellation (AGENTS.md:140). Breaking. | TODO.md | codex-N4-1, claude-N4-1; **both** | - |
| G3-12 | N5/N2, consistency proposal (corrected from C-WORD-ORDER) | Related names drop their family prefix. In the `ListDrop*` family, `DropPosition` (R/list_drag.rs:47) carries the prefix of the generic drag types `DropOperation` and `DropResponse`. The quick pick's status is `PickStatus` (R/quick_pick.rs:281) beside `CommandPaletteStatus` (R/command_palette.rs:120). | sig | runtime, all | low | medium | Rename to `ListDropPosition` and `QuickPickStatus`. Breaking. | TODO.md, or app-author slice 9 if treated as discovery | claude-N5-1; **confirmed** as a proposal. Correction: both auditors agree C-WORD-ORDER does not support it (word order, not missing prefixes), and Claude reclassified it. The "used only by" fact is disputed (D8). | F3 (testers had to find `ListDrop` and `DropPosition`) |
| G3-13 | B4, project rule (contracts.md:1159) | Cloneable UI-thread handles that drive one widget carry a suffix other than `*Controller`: `ContextMenuOpener` opens a mounted context menu and its clones share the menu (R/menu.rs:1295–1305). `ToastQueue` feeds one `ToastHost`, and posting invalidates that host (R/notifications.rs:253–265). `FieldHandle` is retained by `Field::form`, which follows its status (R/field_status.rs:298–304, :520). | sig (role from source) | runtime, all | low (both) | high for `ContextMenuOpener`; medium for `ToastQueue` and `FieldHandle`, which could be read as application-owned like `ListSelection` | `ContextMenuController`, `ToastController` and `FieldController`, or a scoped, documented exception in contracts. Renaming is breaking; documenting is not. | TODO.md | codex-B4-1 (these three), claude-B4-1; **both** | - |
| G3-14 | B4, project rule ("a suffix on a type in another role") | `FormStatus` is a cloneable `Rc` handle (`#[derive(Clone)]`, R/field_status.rs:214–216) that coordinates fields and moves focus (`focus_first_error`, :284), yet it carries `*Status`. Elsewhere `*Status` names only owned value enums (`FieldStatus`, `DialogStatus`, `TaskStatus`, `ToastStatus`). | sig (role from source) | runtime, all | low (both) | high (observation) | Rename (`FormController`, or another `Form…` name, depending on G3-19). Breaking. | TODO.md | claude-B4-1 (`FormStatus` part, corrected in verification); **confirmed** by Codex ("its active role and different use of `Status` are established"). Codex omitted it from its role table and accepted the omission. Whether it is a *controller* is G3-19. | - |
| G3-15 | B4, project rule (contracts.md:1161–1162) | `PendingJump` holds a `ListRevealReceipt` and forwards exactly the receipt interface, owned `status()` and `status_binding()` (R/content_search.rs:416, :422, :428). It is a receipt under another suffix, and also the continuation token for `Runtime::resume_content_jump`. | sig | runtime, all | low (both) | codex high; claude medium | Codex: a receipt name. Claude: keep the name for the continuation and replace the forwarders with `receipt(&self) -> &ListRevealReceipt`. Both accept either. Breaking. | TODO.md | codex-B4-4 (`PendingJump` part), claude-B4-2; **both** | - |
| G3-16 | B4, project rule; policy question on a state-machine exception | `pert_ui_core::ParameterState` is the "Synchronous edit state machine" with mutating `begin`, `update`, `commit`, `cancel`, `write_base` and others (C/edit.rs:272, :367, :450, :486). It carries `*State`, which the contract gives to passive captured data. It is neither cloneable nor a UI-thread widget handle, so `*Controller` would also misclassify it. | sig (role from source) | core, all (re-exported) | low (both) | codex high; claude medium | Either permit `State` for authoritative state machines in contracts (non-breaking), or rename it (e.g. `ParameterEditor`/`ParameterMachine`) and reserve `State` for the snapshot (breaking). | TODO.md (rename); contract wording to the maintainer | codex-B4-3, claude-B4-3 (first bullet); **both** | - |
| G3-17 | B4, project rule; **policy question** | Does "passive captured data are `*State`" (contracts.md:1159–1160) reach every snapshot, inspection DTO, report and persisted document? Read literally, about 20 types depart: the `*Snapshot` types (`AnyParameterSnapshot`, `ParameterSnapshot`, `InspectionSnapshot`, `ProfileSnapshot`, `HistorySnapshot`, `DeliverySnapshot`), the seven `*Inspection` types, `ChildInfo`, `CommandEntry`, `CommandInfo`, `FormSummary`, `ProfileStats`, `QueryResult`, `Telemetry`, `Workspace`, `WindowPlacement` and `ValidationReport`/`WorkspaceReport`. Only `ViewState`, `KeyboardState` and `TextInputState` use `*State`. | sig | core and runtime, all | low (both) | high (observation); medium (the rule's reach) | Amend the sentence to name `*Snapshot`/`*Inspection` (and possibly `*Report`) as suffixes for owned captures and reserve `*State` for a widget's captured state, or rename. Renames must not change serialized schemas (`InspectionSnapshot::SCHEMA_VERSION`, `Workspace::SCHEMA_VERSION`). Breaking if renamed. | Maintainer (contract edit); TODO.md if renames are chosen | codex-B4-2, claude-B4-3 (second bullet); **policy** (both). Claude confirmed Codex's unmatched instances except `ComponentDetails` (D2). Codex confirmed Claude's `WindowPlacement` and `*Report`, but treats `DrainReport` and `FlushReport` (C/live/mod.rs:312, R/runtime.rs:3483) as operation reports, distinguishable from captured state. | - |
| G3-18 | B4, project rule; N2 proposal; **policy question** | Does "A `*Outcome` names how an interaction ended" (contracts.md:1162–1163) cover operation results? Results documented as outcomes carry other suffixes: `pert_ui_wgpu::Frame` ("Surface acquisition/presentation outcome", W/lib.rs:87–92), `ChildReveal` (R/runtime/reveal.rs:7–11), `ListingDelivery` (R/file_browser.rs:246–249) and `ResponseDisposition` (R/parameter_binding/remote.rs:111–114). Conversely, handling flags and operation results carry `*Outcome`: `InputOutcome` (R/pointer.rs:216), `TextKeyOutcome`, `TextCommandOutcome`, `PreviewOutcome` (C/edit.rs:139), `PublishOutcome`, `FilterOutcome` and arguably `CommandOutcome`. | sig | core, runtime all; wgpu default | low (both) | high (roles); medium (the rule's breadth) | Amend to "the non-error result of an interaction or operation" (no code change), or rename one side (`Frame` → an outcome name; `InputOutcome` → `InputHandling` …). Preserve the local-processing and authority-acceptance distinction. Breaking if renamed. | Maintainer (contract edit); TODO.md if renames are chosen | codex-B4-5, claude-B4-4; **policy** (both). Codex confirmed Claude's inverse instances. `ContentJump` as an outcome under another suffix is disputed (D9). | - |
| G3-19 | B4; **policy question** (arising in verification) | Are application-owned coordinators that reach widgets only through handles they issue in the controller role? `FormStatus` is "Form-level coordination of field statuses, owned by the application" (R/field_status.rs:173), and no widget takes it (`Form::new` takes only children, R/form.rs:64). The same reading was raised for `ToastQueue` and `FieldHandle`. The question parallels `ListSelection`'s application ownership (contracts.md:1160–1161), which has no counterpart sentence for forms or toasts. | sig | runtime, all | low | medium | Rename to `*Controller`, or add a contract sentence placing form status (and the toast queue) beside collection selection. | Maintainer | claude-B4-1 (corrected in verification); codex verification ("whether that makes it a controller … remains a maintainer decision"); **policy** | - |
| G3-21 | B4; **policy question** | Should the receipt rule state an exception for cross-thread, one-time replies? `Pending<T>` is returned by every `InspectionClient` request and exposes `wait`, `wait_timeout` and `try_reply`, not `status()`/`status_binding()`. It is `Send` and `Sync`, and its reply is handed out once (R/inspection.rs:555–566, :588, :610, :640). | sig | runtime, all | codex low, or medium "if consumers infer the wrong threading model"; claude: no departure. **Propose low** (the rustdoc states the threading and the once-only consumption). | medium | Codex: document a cross-thread, consuming-reply exception, or design a distinct report family; do not add a UI-local binding. Claude: no exception needed. | Maintainer | codex-B4-4 (`Pending<T>` part); **policy**, with the underlying role disputed (D4) | - |
| G3-22 | N3, project rule | A logical-pixel length taken as `Length` is read back or defaulted as bare `f32`. `TableColumn::width(Length)` and `TableController::set_column_width(.., Length)` sit beside `fixed_width() -> Option<f32>` and `MIN_WIDTH: f32` (R/table.rs:96, :804, :169, :68). `ToolWindow::extent(Length)` sits beside the field `ToolWindowChange::extent: Option<f32>` and `with_geometry(extent: Option<f32>, ..)` (R/tool_windows/mod.rs:450, :236, :285). Also `DRAG_THRESHOLD: f32` beside `ClickCounter::DEFAULT_DISTANCE: Length`, `QuickPick`/`CommandPalette::DEFAULT_WIDTH: f32`, and `width: Option<f32>` on `Painter::text`/`rich_text`, `MeasureContext::text`/`rich_text` and `EventContext::text`. | sig | runtime, all | low | high for the setter/getter pairs; medium for the constants | Return `Option<Length>`, type the constants as `Length`, or adopt G3-7's convention. Breaking for the type changes. | TODO.md, with G3-7 | claude-N3-d1 (inventory delta); **confirmed** by codex | - |
| G3-23 | N3, project rule; N2 proposal | `UiStrings::MAX_LENGTH: usize = 1024` counts bytes (R/strings.rs:576–577, :516), while `TextField::max_length` and `TextArea::{min,max}_length` count grapheme clusters (R/text_fields.rs:450–458). Other byte limits name the unit (`Workspace::MAX_STRING_BYTES`, `FontData::MAX_BYTES`), and character limits say `_CHARS`. | sig | runtime, all | low | high | Rename to `UiStrings::MAX_BYTES` (or `MAX_TEXT_BYTES`). Breaking. | TODO.md | claude-N3-d2 (inventory delta); **confirmed** by codex | - |
| G3-24 | N2, consistency proposal | Ended successfully is `TaskStatus::Completed` but `ProgressStatus::Succeeded`/`EventStatus::Succeeded`. (Verification refuted the other pairs claude proposed: `DialogStatus::Idle` and `PickStatus::Closed`, and `PickStatus::Pending` and `CommandPaletteStatus::Invoking`, have documented different roles, and `CommandEntry::enabled` is a `Result<bool, _>`, not the opposite of `PickItem::disabled`.) | sig | runtime, all | low | medium | One word for the successful terminal state. Names only. Breaking. | TODO.md | claude-N2-d1 (inventory delta); **partly** confirmed by codex: only `Completed`/`Succeeded` holds; `Idle`/`Closed` and `Pending`/`Invoking` have documented different roles, and `CommandEntry::enabled` is a `Result<bool, _>` | - |
| G3-25 | N4, guideline C-CUSTOM-TYPE | `pert_ui::NativeCloseAction::Defer(ReadSignal<Option<bool>>)` decides with `Some(true)` = close and `Some(false)` = keep open (F/app.rs:207–210). The packaged example needs a comment to say so (crates/pert-ui/examples/diagram_editor/app.rs:891, :910–911), although the same enum already names `Close`/`KeepOpen`. | sig | facade: cpu, default, image-formats, images, no-default-desktop-cpu, serde, svg | low | high | `Defer(ReadSignal<Option<WindowCloseAction>>)`. Breaking. | TODO.md | claude-N4-d1 (inventory delta; cites round 1 r1-1-codex FRICTION.md:62–63); **confirmed** by codex | - |

Inventory notes; these are not findings:
- Both auditors reported that `From` implementations are rendered under their source type. Codex's delta processed 152 corrected rows in inventory v2, so v2 appears to fix this.
- Claude's notice that `bool-params.txt`'s builder mark is syntactic is in v2's header.
- Claude's delta reports two gaps that v2 does not close, both affecting N3. Numeric parameters and returns wrapped in `Option` are in no list (about 30 rows). Associated constants are in no list.
- Codex's delta: "None: no new G3 finding".
- Claude's "outside the assigned items" note on `Semantics::read_only(bool)` is for B1 and is not counted here.

### Disputed facts

**D1. `svg::{rasterize, rasterize_mask}` leave their pixel unit out of the signature (part of codex-N3-1, G3-9).**
- Codex: the `width: u32, height: u32` parameters (F/svg.rs:295, :310) name no unit.
- Claude: they size the produced raster, whose own pixels are the unit. They are like `RasterImage::from_srgba8(width, height, ..)` (R/resources.rs:168) and `MaskImage::new` (R/drawing.rs:1144), which Codex does not flag. The rustdoc says "physical pixels" (F/svg.rs:284–285, :300–301).
- Source: both functions document "exactly `width` by `height` physical pixels" (F/svg.rs:284–285, :300–301) and return an image with exactly those dimensions. Codex's N3-1 does not list `from_srgba8` or `MaskImage::new`, and its exemption names raw constructors of unit-bearing types.
- Verdict: the code supports Claude's consistency argument. Whether an output raster's dimensions need a unit in the name is the remaining N3 judgement.

**D2. `ComponentDetails` is passive captured data (part of codex-B4-2, G3-17).**
- Codex lists it among the captured observations (R/diagnostics.rs:646).
- Claude: it is "A component's explicit inspection contribution" (R/diagnostics.rs:639), supplied by the component like `Semantics`, not captured from it.
- Source: it is the return type of `Component::inspect` (R/runtime.rs:2043; implementations e.g. R/color_swatch.rs:342), and `ComponentInspection::details` documents it as "Explicitly supplied component/layout data" (R/diagnostics.rs:707–708). The runtime does write the measured border box into it while capturing (R/runtime/inspect.rs:182).
- Verdict: the code mostly supports Claude on the type's role, a component-supplied description. Codex's reading holds only for the copy embedded in `ComponentInspection`.

**D3. `EditorSession` drives a widget (part of codex-B4-1, G3-20).**
- Codex: it is cloneable and UI-thread-only, and `begin`/`end` drive the editor's session (R/editor_session.rs:116, :152, :173, :181).
- Claude: the host calls `begin` before it builds an editor. Editors attach and report into the session, and `end` only makes later reports drop and closes nothing. Control runs editor → session → host (R/editor_session.rs:104–112, :173–194).
- Source: `begin` and `end` only change the session's own cells (`edit`, `open`, `start`, `completions`; R/editor_session.rs:166–176). `end`'s rustdoc reads "for example when the host itself closes the editor; later reports for it are dropped" (:172–173). Nothing invalidates or addresses a mounted widget. The editor reads the session only when it attaches at build (`attach`, :190–197).
- Verdict: the code supports Claude, which leaves Codex's policy question (G3-20) without a departure to exempt.

**D4. `Pending<T>` is in the receipt role (part of codex-B4-4, G3-21).**
- Codex: it "reports one queued inspection request", with one-time reply consumption (R/inspection.rs:555–640).
- Claude: it carries the reply itself, not a status. It is a cross-thread one-shot reply slot, while a `*Receipt` is a UI-local status view (contracts.md:1161–1162).
- Source: the rustdoc reads "A reply that has not necessarily arrived yet … handed out once … `Send` and `Sync`" (R/inspection.rs:556–562). It has no `status()`, and `wait`, `wait_timeout` and `try_reply` return `InspectionReply<T>` (:588, :610, :640).
- Verdict: the code supports Claude's classification, a reply future rather than a status report. Codex's point that it answers one queued request is also true, so the open question is the policy one.

**D5. The contract reserves `_signal` for inputs (part of claude-N2-1, G3-1).**
- Claude: the contract makes `_signal` the suffix of a builder that takes `ReadSignal<bool>` (contracts.md:1157), so `UiSettings::*_signal` returning a signal gives `_signal` a second meaning. Claude's fix is "keep `_signal` for inputs only".
- Codex: contracts.md:1156 governs boolean builders and does not reserve `_signal` exclusively for inputs.
- Source: the sentence is "A builder for an ordinary boolean property takes a `bool`; the binding variant adds `_signal` and takes a `ReadSignal<bool>`" (docs/contracts.md:1156–1157). Nothing else in "Public naming rules" mentions `_signal`.
- Verdict: the text supports Codex. The contract defines `_signal` for boolean builders without reserving it. Claude's observation that the suffix is used in both directions stands as a proposal, not as a departure from that sentence.

**D6. `ContentSearch::is_done` and `ProgressStatus::is_finished` name one concept (claude-N2-5, second bullet).**
- Claude: both mean "work that has ended" (R/content_search.rs:310; R/progress.rs:74).
- Codex: search exhaustion differs from a success/failure/cancellation status.
- Source: `is_done` is "Whether the search has visited everything". It is set only when the tree and the offered collections are exhausted (R/runtime/content_search.rs:202, :254; `ContentStep::done`, R/content_search.rs:206–209). `is_finished` is "Whether the task has ended, whatever the outcome" and matches `Succeeded | Failed | Cancelled` (R/progress.rs:72–75).
- Verdict: the code supports Codex. A search has one way to end, which is exhaustion, while `is_finished` covers three. They are both terminal predicates, but `is_done` cannot report the failure or cancellation that `is_finished` means.

**D7. `ToolWindowLayout::show_names` and `names` are one boolean property under two names (claude-N2-5, fourth bullet).**
- Claude: both set whether stripe buttons show names (R/tool_windows/mod.rs:916, :924).
- Codex: `show_names` initialises retained, persisted state, while `names` installs application-owned writable state (:912–927). Converting `names` to a read-only boolean binding would change its contract.
- Source: both rustdocs concern "whether stripe buttons show names". `show_names(bool)` writes the layout model "only when the layout mounts", and the result is retained and persisted (:912–919). `names(impl Into<Signal<bool>>)` makes it the application's signal, and "Bound names are not persisted by the layout" (:920–927). `ToolWindowController::set_names` writes through the binding when present (R/tool_windows/mod.rs:2491–2505).
- Verdict: the code supports both in part. It is one displayed property, as Claude says, under two ownership modes, as Codex says, and the mode difference explains two builders. Claude's suggested `names`/`names_signal` pair would misdescribe `names`, because it takes a writable `Signal`, not a `ReadSignal`.

**D8. `DropPosition` is used only by `ListDrop::position` and `ListDropPlacement::position` (claude-N5-1, G3-12).**
- Codex: it also appears in the public `ListDropPlacement::new` (R/virtual_list.rs:570).
- Source: the API listing has exactly three uses: `ListDrop::position`, `ListDropPlacement::new` and `ListDropPlacement::position` (api/pert-ui-runtime/default.txt:26555, :26568, :26569).
- Verdict: the code supports Codex on the literal list. Claude's conclusion still holds, because every use is in the `ListDrop*` family.

**D9. `ContentJump` is an outcome under another suffix (claude-B4-4, G3-18).**
- Claude: its rustdoc calls it "The outcome of `Runtime::jump_to_content`" (R/content_search.rs:389).
- Codex: `ContentJump::Pending` represents unfinished work, so it is not uniformly an interaction's end. Broadening "interaction" to "operation" does not establish terminality.
- Source: the rustdoc says "The outcome of …" (R/content_search.rs:389). The `Pending(PendingJump)` variant is a continuation passed to `Runtime::resume_content_jump` "to reveal and focus it" after the row mounts (:404–411).
- Verdict: both facts are true. The code supports Codex that one variant is not an ending, so `ContentJump` needs a continuation allowance under any reading of the rule.

#### Author's resolution

The author settled each disputed fact against the source the merge sheet cites:
- **D1, `svg::rasterize`:** Claude holds. Its arguments size an output image in that image's pixels, like the image types both auditors exempt. The instance leaves G3-9.
- **D2, `ComponentDetails`:** Claude holds. It is a component's own description; only its copy inside `ComponentInspection` is captured data. The instance leaves G3-17.
- **D3, `EditorSession`:** Claude holds. Editors attach to it and report into it; it drives no widget. The instance leaves the B4 controller finding, and G3-20 has no departure to exempt, so it is withdrawn as a policy question.
- **D4, `Pending<T>`:** Claude holds on the role (a cross-thread reply carrying the answer, not a status report). Policy question G3-21 stays, at low severity.
- **D5, `_signal`:** Codex holds. The contract defines `_signal` for boolean builders without reserving it for inputs. The two-direction use stays as a consistency proposal in G3-1.
- **D6, `is_done` and `is_finished`:** Codex holds. The two predicates mean different things (exhaustion against any terminal state), so the pair leaves G3-5.
- **D7, `show_names` and `names`:** both in part. It is one property under two ownership modes, so the two builders are explained. Claude's suggested `names_signal` spelling would misdescribe `names`, so the pair leaves G3-5.
- **D8, `DropPosition`:** Codex holds on the literal list of uses. Claude's conclusion stands because every use is in the `ListDrop*` family; G3-12 remains a proposal.
- **D9, `ContentJump`:** both facts hold. One variant is a continuation, not an ending, so `ContentJump` needs an allowance under any reading; it stays in policy question G3-18.

### Withdrawn and refuted parts
- **G3-20** (codex-B4-1's `EditorSession` policy question) is withdrawn
  by the author's resolution of D3.

- claude-N2-5, `Tabs::on_select` versus `on_selection_change`: **withdrawn** by Claude. Contracts "Component notifications" distinguishes component-owned selection from a bound application-owned `ListSelection` with coalesced writes (docs/contracts.md:527–534). Codex had confirmed it as a proposal.
- Codex N2 disposition ("no additional findings in the required synonym sets"): **withdrawn** by Codex for the proposals in G3-1 to G3-5.
- claude-N5-1 rule anchor C-WORD-ORDER: **withdrawn** by Claude, which reclassified it as a consistency proposal (G3-12). Codex agrees.
- claude-B4-1, `FormStatus` as a plain controller departure: **corrected** by Claude into G3-14 (the suffix departure stands) and G3-19 (the policy question).
- codex-B4-2 scope: **qualified** by Codex, adding `WindowPlacement` and `ValidationReport` to the policy discussion and keeping operation reports distinct (G3-17).
- codex-N3-1, `svg::{rasterize, rasterize_mask}`: **refuted** by Claude and not withdrawn (D1). The raster's own pixels are the unit, as for the unflagged image constructors.
- codex-B4-2, `ComponentDetails`: **refuted** by Claude and not withdrawn (D2). It is a component-supplied description.
- codex-B4-1, `EditorSession`: **refuted** by Claude as a departure; Codex maintains it as policy (D3, G3-20).
- codex-B4-4, `Pending<T>`: **refuted** by Claude as a receipt-role departure; Codex maintains the observation and the policy question (D4, G3-21).
- claude-N2-1, the claim that `_signal` is reserved for inputs: **refuted** by Codex (D5); the rest of G3-1 stands.
- claude-N2-5, `is_done`/`is_finished` and `show_names`/`names`: **refuted** by Codex and maintained by Claude (D6, D7).
- claude-N5-1, "used only by": **refuted** in its letter by Codex (D8); the substance stands.
- claude-B4-4, `ContentJump`: **refuted in part** by Codex (D9), because its `Pending` variant is a continuation.

## G4: Types (T1–T6)

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
status `policy`, 2 from the inventory delta (since verified, see their rows). Further policy aspects inside
findings: G4-4 (rule kind), G4-10 (exception), G4-13 (exception as an
alternative fix), G4-22 (proposal part), G4-27 (`Borrow<str>`).

### Findings

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
| G4-16 | T4, project rule (`# Errors` names variants, incl. forwarded) | `# Errors` sections list fewer phase errors than `writable()` / `Signal::set` return: `ListSelection::{set_active, select, clear}`, `TreeController::{set_expanded, toggle, set_source, filter}`, `TableController::{set_source, set_sort, filter, set_active_column}`, `UiSettings::{set_motion, set_debug_overlay}`, `Runtime::{set_viewport, inspection_service}` (both); `ListController::reveal`, `TreeController::reveal`, `TableController::reveal`, `ZoomController::{set, set_changed, fit}` (codex, confirmed); `Runtime::mailbox` documents only `Closed` (codex; claude: only `WriteDuringInspection` and `WriteDuringDelivery` reachable). After the author's resolution: `FieldHandle::{set, set_changed, set_modified, clear, remove}` and `ListSelection`'s matching sections are in (D2); `EditSession::update` is in, for `EditError::Pending` (D3); `Runtime::set_visible` is in, for `WriteDuringDelivery` (D4, corrected after review); for `mailbox`, `set_viewport` and `inspection_service` only the inspection and delivery variants count (D4). | doc (beh) | all | Auditors: codex medium, claude low. Proposed: **low**: every omitted failure is still a typed error and several omitted variants are unreachable | high | List the reachable variants or link `Signal::set`'s complete list; non-breaking | TODO.md | claude-T4-1, codex-T4-7: **both**, **partly** after verification. Corrections: claude withdrew `WriteDuringLayout` for `set_viewport` and `inspection_service` (unreachable from `&Runtime`); claude narrowed codex's `mailbox` and `set_visible` to reachable variants | - |
| G4-17 | T4, project rule (`# Errors` names variants) | `# Errors` sections name a category or condition, not a variant. Category only: `EditSession::{update, commit, cancel}`, `ParameterHost::{write_base, replace_spec, set_effective, remove}`, `AnyParameterHost::remove`, `RemoteHost::{on_submit, clear_submission_sink, respond, resynchronize}`, `Parameter::begin_edit`, `Runtime::{parameter, remote_parameter, cancel_input, set_presentation_rate, service_inspection}`, `FloatingWindow::{position, initial_size, fixed_size}`, `EventContext::{set_timer, start_drag}`, `Permission::get`, `FileBrowserController::selected_paths`. Conditions without variant: `ParameterState::{begin, replace_spec, write_base}`, `Runtime::{mount, mount_tree, children, invalidate, set_visible, unmount, focus, event, paint}`, `Color::rgba`, `MenuBar::new`, `ViewState::{new, with}`, `Workspace::{new, decode}`, `WorkspacePath::new`, `WorkspaceValue::text`, `Renderer::recreate`. "A `GeometryError`" without the variant: `FileBrowser::new`, `Table::new`, `Tree::new`, `VirtualList::{new, estimated}`. Nested variant without its `RuntimeError` wrapper: `App::{size, min_size, max_size}`, `NativeWindowOptions::size`. | doc (sig) | all; `App` desktop configurations; `Renderer` wgpu | low | high | Name the variants; a shared binding-family paragraph or a link to a complete operation contract suffices (codex); non-breaking | TODO.md | claude-T4-2: **confirmed** by codex for every instance; overlaps codex-T4-7 (`EditSession::update`, `set_visible`, `ViewState`, `WorkspacePath::new`, `WorkspaceValue::text`) → **both** there; `Renderer::recreate` from codex-T4-7 confirmed by claude | - |
| G4-18 | T4, project rule (every `Result` has `# Errors`) | Five fallible conversion impls have no documentation at all: `ChoiceId`'s `TryFrom<&str>` and `TryFrom<String>` (`C/choice.rs:64,72`), `Magnification::try_from` (`R/pointer.rs:134`), `UiZoom::try_from` (`R/runtime/settings.rs:77`), `ViewScale::try_from` (`R/transform.rs:309`). | doc | all | low | high | Add sections linking the validation contract; non-breaking | TODO.md | codex-T4-6: **partly** (claude confirmed these five; the other ten forward in a doc line, see G4-19) | - |
| G4-19 | T4, policy | **Question:** may standard-trait implementations forward to a documented constructor in one line instead of an own `# Errors` section? Covers `Capacity`, `PowerExponent`, `Length`, `PresentationRate` `try_from`, `Color::from_str`, `TryFrom<AnySpec>`, the four macro `TryFrom<AnyValue>` impls, and the hand-written serde impls of G4-20. The rule says "Every public `Result`". | doc | all; serde rows in the serde configurations | low | high | State the exception, or add sections (non-breaking) | maintainer | codex-T4-6: **policy** (raised by codex; claude agreed the ten are a heading-form question and added the serde rows) | - |
| G4-20 | T4; **policy question** (joins G4-19), narrowed in verification | Hand-written `Serialize::serialize`/`Deserialize::deserialize` for `ChoiceId`, `ChoiceOption`, `NormalizedValue`, `ParameterId`, `ParameterSnapshot<T>`, `AnyParameterSnapshot`, `ParameterSpec<f32/i64/bool/ChoiceId>`, `DropOperations`, `UiZoom`, `Viewport`, `WorkspaceValue`, `WorkspacePath`, `ViewState`, `Workspace` (31 methods) have no `# Errors` section, though they reject invalid data (e.g. `C/normalized.rs:219`, `R/drawing.rs:1690`, `R/workspace.rs:1672`). | doc | serde configurations | low | high | Document forwarded format errors and validation failures, linking validation contracts, without inventing variants for `D::Error`; non-breaking | TODO.md unless G4-19 adopts the exception | codex-T4-d1: **partly** confirmed by claude: no serde method has its own `# Errors`, but every validating type states its rejection rules in a `# Serialization` section; it joins codex-T4-6's **policy** question on forwarding in standard-trait implementations, low. Note: claude's own delta read the same rows, found each validating type documents its rejection rule in a `# Serialization` section (`R/workspace.rs:350-353`, `R/drawing.rs:822`, `C/erased.rs:168`), and filed them under G4-19 rather than as a finding | - |
| G4-21 | T4, policy | **Question (F11's second half):** should `DrawError` convert into `RuntimeError` so a test helper can use `paint()?`? `docs/contracts.md:1131` explicitly excludes this conversion. | sig | all | low | high | Keep the exclusion, or add the conversion (non-breaking) | maintainer; diagnostics survey | claude-T4-7 (its policy question): **policy**; both agree the current API meets the contract. Claude withdrew T4-7's finding part (see below) | F11 |
| G4-22 | T4, project rule (application failures); proposal | F30's source loss is resolved: `From<Box<dyn Error + Send + Sync>> for ApplicationError` keeps the box (`R/error.rs:79`). Remaining: `ApplicationError::new`'s rustdoc (`R/error.rs:48-51`) does not point to that `From`, and `RuntimeError` has no `From<Box<dyn Error + Send + Sync>>`, so `?` on a boxed error needs `.map_err(ApplicationError::from)?`. | doc and sig | all | low | high | Doc pointer on `new` (non-breaking); optional `From<Box<…>>` for `RuntimeError` (consistency proposal, additive) | TODO.md (update F30's entry); maintainer for the proposal | claude-T4-8: **partly**: source-loss part resolved (codex: "F30 is not current"); codex says the doc pointer is outside G4's documentation scope and the `RuntimeError` conversion is an optional proposal (**policy**) | F30 |
| G4-23 | T4, policy | `InputRejection` and `ListingError` display application or OS user text verbatim, uppercase and with arbitrary punctuation ("Name must not be empty", `R/text_fields/normalize.rs:18-25`; `R/file_browser.rs:76-83`), against "Display messages begin in lowercase". **Question:** exempt user-message carriers (and document it), or keep `Display` diagnostic and expose user text through `message()`? | beh | all | low | high | Record a contract exception, or split diagnostic and user text; do not rewrite application prose | maintainer; TODO.md | codex-T4-8, claude-T4-10: **policy** (both) | - |
| G4-24 | T4, project rule (callback-boundary exception) | The callback-boundary exception justifies `From<WindowError> for RuntimeError` by "a runtime method returning `WindowError`", but no public method returns `Result<_, WindowError>`; consumers meet it only in `PresentationStatus::Failed` (`R/runtime/native_windows.rs:933`), and the only producer is private (`F/platform/windows.rs:50`). | doc | all | low | high | Correct the rationale in `docs/contracts.md:1125`, `R/runtime.rs:158` and `R/error.rs:15`; keep the `From` (removing it would be breaking and unnecessary) | TODO.md | claude-T4-11: **confirmed** | - |
| G4-25 | T5, guideline C-STRUCT-PRIVATE; project rule (private invariants) | `Semantics` documents sorted, duplicate-free actions, none when disabled, and no `value` when `protected` (`R/diagnostics.rs:476,481,494`); `SemanticItem` likewise plus a never-empty name (`:350-367`). Only the builders enforce this; public field assignment and the derived `Deserialize` bypass it, and `describe_component` forwards the result (`R/runtime/inspect.rs:145-161`), contradicting "never exposes its content through inspection" (`R/diagnostics.rs:622`). | sig and beh | all | Auditors after verification: codex medium (narrowed from high) for the protected-value exposure, low for action/name; claude medium. Proposed: **medium** / **low** as agreed | high | Private fields with accessors and validated deserialization (breaking), or normalize at `describe_component`: clear `value` when protected, sort/deduplicate actions, clear them when disabled (non-breaking, claude) | TODO.md | codex-T5-1: **confirmed** by claude. `PickItem` is excluded (documented plain data) | - |
| G4-26 | T5, guideline C-STRUCT-PRIVATE; project rule (validated identities) | `CaptureJob { pub id, pub window, pub include_overlay }` (`R/capture.rs:531-539`) is trusted by `Runtime::capture_scene`, which renders `job.window`/`job.include_overlay` without looking up request `id` (`R/runtime/capture.rs:296-310`), while `complete_capture` answers by `id` alone (`:321-344`); a built or edited job silently attributes a same-sized window's pixels to the wrong request. | sig and beh | all | low | high on the path, medium on likelihood | Private fields with accessors, and have `capture_scene` look up the pending request by id and reject stale or foreign jobs (codex's addition); breaking | TODO.md | claude-T5-1: **confirmed** by codex | - |
| G4-27 | T6, guideline C-CONV-TRAITS | `ChoiceId` exposes `as_str` but no `AsRef<str>` (`C/choice.rs:32,50`), so it cannot be passed to `impl AsRef<str>` APIs such as `ContentQuery::new`. Claude also proposes `Borrow<str>` for `&str` lookups in `HashMap<ChoiceId, _>`. | sig | all | low | high | Add `AsRef<str>` (and `Borrow<str>` if adopted); non-breaking | TODO.md | codex-T6-1, claude-T6-1: **both** for `AsRef<str>`; `Borrow<str>` is a proposal, not a required T6 trait (codex) | - |
| G4-28 | T6, guideline C-CONV-TRAITS | `FontData::from_owned(Vec<u8>)` and `from_static(&'static [u8])` have no matching `From` impls (`R/text/fonts.rs:43,51,64`). | sig | all | low (small benefit: `FontManager::register` takes the concrete type) | high | Add the two `From` impls, keep the named constructors; non-breaking | TODO.md | codex-T6-1: **confirmed** by claude | - |
| G4-29 | T6, guideline C-CONV-TRAITS | `Tab::into_element`, `Section::into_element`, `Property::into_element` and `FloatingWindow::into_element` (`R/tabs.rs:119`, `R/collapsible.rs:849`, `R/property_grid.rs:88`, `R/windows.rs:494`) are infallible conversions with no `From<_> for Element`, so `.map(Into::into)` into `reconcile_children`'s `Vec<Element>` fails. | sig | all | low | high | Add the `From` impls, keep the methods; non-breaking | TODO.md | claude-T6-1: **confirmed** by codex | - |
| G4-30 | T6, consistency proposal | Only `PowerExponent` (`From<PowerExponent> for f32`, `C/parameter.rs:96`) and `UiZoom` (`From<UiZoom> for f64`, `R/runtime/settings.rs:81`) convert back to their raw number; `NormalizedValue`, `Capacity`, `Length`, `Magnification` and `ViewScale` read only through `get()`, and `PresentationRate` through `hertz()`. | sig | all | low | high | Add `From<Self>` for the raw type everywhere, or keep `get()` as the one reading and serialize `PowerExponent` through a private conversion (breaking for the removal) | TODO.md if adopted | claude-T6-d1: **partly** confirmed by codex: the asymmetry holds, but `PresentationRate` reads through `hertz()`, not `get()`; low consistency proposal | - |

### Disputed facts

#### D1. Does the project rule require error types to implement `std::error::Error`? (G4-10)

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

#### D2. Do the `FieldHandle` `# Errors` sections omit forwarded variants? (G4-16)

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

#### D3. Does `EditSession::update`'s `# Errors` cover `EditError::Pending`? (G4-16, G4-17)

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

#### D4. Which omitted phase variants can `Runtime::mailbox`, `set_viewport`, `inspection_service` and `set_visible` actually return? (G4-16)

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

#### D5. Inventory version 2 error-row counts (minor, inventory only)

- **Claim (codex):** 41 added rows: 14 fallible conversions, 25 serde
  methods, two `ParameterValue` implementations.
- **Claim (claude):** 41 new entries: 15 `TryFrom`/`FromStr` and 26 serde.
- **Source:** `git diff 6e39e18 7c84f54 -- plans/usability/api-audit/inputs/candidates/errors.txt`
  adds 41 data rows: 14 `TryFrom`/`FromStr`, 25 serde, two
  `ParameterValue` (`from_any`, `spec_from_any` for `ChoiceId`). **The
  diff supports codex.** Version 3 (245053b) since added the three
  primitive `TryFrom<AnyValue>` rows both verifiers reported as a residual
  gap. No finding depends on the count.

#### Author's resolution

The author settled each disputed fact against the source the merge sheet cites:
- **D1, `std::error::Error`:** Claude holds on the source of the rule.
  - contracts.md's error policy asks for a typed error, and the `Error`
    implementation comes from C-GOOD-ERR.
  - G4-10 therefore stays a guideline finding with an exception question
    for the maintainer.
- **D2, `FieldHandle`:** a finding, at low severity. The section's own list
  (`crates/pert-ui-runtime/src/field_status.rs:320`) leaves out layout
  and delivery, and its "like any `Signal::set`" pointer does not repair an
  explicit list that is incomplete. `ListSelection`'s identical sections
  join the instance.
- **D3, `EditSession::update`:** Codex holds. "Finished-edit" is
  `EditError::Finished` (`crates/pert-ui-core/src/edit.rs:213`), and
  `Pending` is a separate variant the section omits. The instance stays.
- **D4, unreachable phase errors:**
  - `Runtime::mailbox`, `set_viewport` and `inspection_service`: Claude
    holds.
    - These take `&self`, but update, layout and paint run inside the
      runtime's own `&mut self` methods.
    - No public context exposes the `Runtime` to component code: there is
      no public accessor returning `&Runtime` and no `Deref` from a
      context.
    - Only the inspection variant (through command predicates) and the
      delivery variant are reachable, and only these remain in G4-16.
  - `set_visible`: the instance is kept for `WriteDuringDelivery`
    (corrected after review).
    - A parameter sink can capture an `Rc<RefCell<Runtime>>`. A retained
      `EditSession::update` called by host code delivers to that sink
      without borrowing the runtime.
    - Delivery enters its phase before calling the sink
      (`crates/pert-ui-runtime/src/parameter_binding/delivery.rs:423`), so
      `set_visible` fails with `WriteDuringDelivery` in `select_window`'s
      `writable()` check
      (`crates/pert-ui-runtime/src/runtime/native_windows.rs:325`), before
      its own check at `runtime.rs:5030`. Its section, "Rejects foreign or
      unmounted IDs", omits that variant.
    - Update, layout and paint stay unreachable, because the runtime is
      borrowed during its flush. After shutdown, `window_of` fails with
      `Unmounted` first.
- **D5, row counts:** Codex's breakdown is right. This is an inventory
  matter only.

### Withdrawn and refuted parts

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

## G5: Discovery (D1–D2)

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

### Findings

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
| G5-14 | D2; part of the closure **policy question** G5-26, narrowed in verification | `EditSource`, the type of the field `ParameterSnapshot::edit_source` (`C/edit.rs:118`) and the return of `EditToken::source` (`:96`), is outside the prelude. | sig | All | low | high | Add it with `ParameterSnapshot` and `EditToken` if G5-26 includes them. Additive. | S9 / TODO | codex-D2-d1, **partly** confirmed by claude: `EditSource` is reachable only through `ParameterSnapshot` and `EditToken`, both already missing, so it joins the closure question G5-26; low. | - |
| G5-15 | D2, project rule | Status values the prelude's `Parameter` returns are missing: `DeliveryStatus` (`Parameter::delivery_status`, `R/parameter_binding.rs:327`) and `AuthorityStatus` (`Parameter::authority_status`, `R/parameter_binding/remote.rs:686`). | sig | All | low | high for `DeliveryStatus`, medium for `AuthorityStatus` | Add both, or say remote-authority status stays with the host types. Additive. | S9 | claude-D2-d2, **confirmed** by codex. Claude moved `DeliveryStatus` out of its D2-5 host family. | - |
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
| G5-27 | D2, policy | **Question (extends G5-26):** one-hop types of `EventContext`, `MeasureContext` and `ChildInfo`: `TaskReceipt`, `TaskStatus` (`R/runtime/tasks.rs:121`), `RevealReceipt` (`R/runtime/reveal.rs:260`), `FittedText` (`R/layout.rs:842`), `TextLayout` (`R/runtime.rs:2924`), `PresentationReceipt`, `NativeWindowOptions`, `NativeWindowId` (`R/runtime/native_windows.rs:1289-1417`); two hops: `ShortcutBinding`, `ShortcutScope` (`R/validation.rs:102`); `EventContext::profile -> ProfileSnapshot` conflicts with the profiling exclusion. | sig | All | low | high on the signatures | As G5-26. Not breaking. | S9 | claude-D2-d3, **confirmed** by codex as a policy question. | - |
| G5-28 | D2, project rule | The facade prelude omits `NativeWindowId`, which `App::on_close` reports (`F/app.rs:465-469`), against "`App` and the types it reports" (`F/lib.rs:50-51`). | sig | Native | low | high | Add it to the facade prelude under `native`. Additive. | S9 / TODO | claude-D2-7, **confirmed**; Codex corrected its blanket host-plumbing exclusion for native configurations. | - |
| G5-29 | D2, project rule | The facade prelude's error sentence names only `AppError` (`F/lib.rs:58-60`) although `AppShutdownError` is a member (`:71`); both auditors keep the membership (payload of `AppError::Shutdown`, `F/app.rs:75`). | sig | Native | low | high | Name `AppShutdownError` in the sentence. Not breaking. | S9 | claude-D2-7 (wording part), **confirmed**. | - |
| G5-30 | D2, policy | **Question:** should `WindowError`, which `App::on_window_error` reports (`F/app.rs:480`), be in the facade prelude under "`App` and the types it reports", or stay at the root under "Other errors … stay at the crate root" (`F/lib.rs:58-60`)? | sig | Native | low | medium | Add it or state the exception. Not breaking. | S9 | claude-D2-7 (`WindowError` part), **policy**; Codex: absence factual, membership needs the conflicting clauses resolved. | - |
| G5-31 | D2, project rule | The facade prelude's rustdoc says its native members come "with the `desktop` feature" (`F/lib.rs:50-51`, `:59`), but the re-export is gated on `native` (`F/lib.rs:69-71`), which `desktop-cpu` enables without `desktop` (`crates/pert-ui/Cargo.toml:21`). | sig | no-default-desktop-cpu (understated); text in all | low | high | Say "with `native` (enabled by `desktop` and `desktop-cpu`)". Not breaking. | S9 (with G5-29) | claude-D2-d1, **confirmed** by codex. | - |
| G5-32 | D2, policy | **Question:** names the rule keeps at the root that most applications import, driven by testing and host failures: `CaptureRequest`, `CaptureTarget`, `cpu::CpuRenderer` (six round 1 submissions), the semantics types, `ApplicationError` (every application probe's `main.rs`), `Mailbox`, `UiSender`, `Capacity`. Should slice 9 add a testing-oriented module or prelude? Not a departure. | use | All (`CpuRenderer`: `cpu`) | low | high on the corrected counts | None required; input to slice 9's prelude shape. | S9 | claude-D2-8, **policy**; Codex corrected the count from seven to six (r1-1-codex only glob-imports the root). The sheet author's check agrees: no capture or CPU name in `U/r1-1-codex` or `U/r1-2-codex` submissions. | F2 (related) |

Inventory gaps reported by both auditors (`App::software_rendering` missing
from the version 1 semantic list; prelude membership, free items and
field/variant lines not inventoried) are not findings. Both verifications
record that version 2 closes them. Claude notes one residual caveat: the six core
types the runtime prelude re-exports read "prelude: no" in `types.txt`'s
core section.

### Disputed facts

#### 1. codex-D1-2 row 4: was the UI-thread `Runtime::query` with `Selector::id` "needed, found late" in round 1?

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

#### 2. codex-D2-1: is `AxisMapping` a layout type an application configures?

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

#### 3. claude-D1-1: does rustc suggest imports from non-std dependencies at all?

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

#### Author's resolution

The author settled each disputed fact against the source the merge sheet cites:
- **1, `Runtime::query` with `Selector::id` in round 1:** Claude holds. Neither r1-4 log records friction there, so the row leaves the D1 finding. The item is already F2's first sub-item.
- **2, `AxisMapping`:** Claude holds. No prelude signature names it, so it leaves G5's membership rows.
- **3, rustc's import suggestions:** bounded by measurement (corrected
  after review).
  - In the diagnostics survey's probes (`diagnostics/survey.md`,
    "Slice 3's misleading diagnostics", cases m02 to m02f), rustc 1.99.1
    suggested no import from any non-std dependency.
  - That covered items re-exported from `pert_ui_runtime`, items defined
    in `pert_ui` and a control crate's own item, while `std` items got
    "consider importing" (m02c).
  - It does not establish a general compiler limitation. Among the
    arrangements the probes tried, only prelude membership or a pointer on
    the prelude page routed a consumer to the missing names.
  - G5-6's fix is worded within that bound.

### Withdrawn and refuted parts

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

## Inventory version 3: delta findings

Both auditors reviewed what inventory version 3 adds
(`reports/verification/{codex,claude}-delta3.md`). The other auditor
verified each finding (`reports/verification/{codex,claude}-dverify.md`).

| # | Rubric item, rule kind | Finding | Sig/beh | Configs | Severity | Confidence | Fix (breaking?) | Destination | Sources and status | Slice 3 |
|---|---|---|---|---|---|---|---|---|---|---|
| V3-1 | C4, project rule | `TableColumn::MIN_WIDTH` documents a floor that the table's own layout can break. `Shared::resolve` (`crates/pert-ui-runtime/src/table.rs:359-366`) gives a column whose start has saturated at `f32::MAX` a width of 0. The test at `crates/pert-ui/tests/collections/table_columns.rs:577` builds that case without asserting the floor, and the comment at `table.rs:332` misstates it. | beh | runtime, all | low | high | Document the saturated exception, assert it in the test and correct the comment; "preserve the minimum" cannot hold in saturated `f32` coordinates (not breaking). | TODO.md | codex-C4-v3-1; **confirmed** by claude | - |
| V3-2 | C4, project rule (AGENTS.md persisted-schema ranges) | `Table`'s workspace restore accepts a stored column width between 0 and `MIN_WIDTH` (`table.rs:2028`) and then overwrites the stored preference with the floor (`table.rs:2075`), instead of rejecting it as out of range. The setters' documented clamping (`table.rs:95`, `:765`) is not part of this finding; it joins the clamping policy question G1-12. | beh | runtime, all | low | high | Reject a stored width below `MIN_WIDTH` with the restore path's out-of-range error (not breaking for valid documents). | TODO.md | claude-C4-v3-2; **partly** confirmed by codex (restore path only) | - |
| V3-3 | N3, project rule | `TextField`/`TextArea::{min_length, max_length}` count grapheme clusters, a unit stated only in rustdoc. Elsewhere "length" means bytes (`UiStrings::MAX_LENGTH`, `str::len`), and `_CHARS` constants count scalar values. | sig | runtime, all | low | high | Name the unit in the parameter (`graphemes`), or rename the builders (breaking). | TODO.md | claude-N3-v3-1; **confirmed** by codex. Codex filed it as an extension of the N3 findings; the fix is the same. | - |
| V3-4 | N3, project rule | `Constraints::{new, width, height}` take `Option<f32>`, and neither the type nor its methods say "logical pixels", unlike its sibling `Size`. | sig | runtime, all | low | high | State the unit in the type's rustdoc, or take `Length` (breaking). | TODO.md | claude-N3-v3-2; **confirmed** by codex | - |

Extensions of earlier rows, confirmed by both:
- **G3-10:** `TextSelection::range`, `TextPreedit::{new, cursor}` and
  `RichText::highlighted` are byte ranges whose unit only the rustdoc
  states.
- **G1-9:** `TextLimit::MAX_RASTER_SIDE` sits on an error-detail enum. Text
  preparation's limits are now spread over four owners.

Not findings:
- **claude-C4-v3-1:** refuted. `NumberInputError::MAX_PRECISION` bounds
  `decimal_places` on the named controls (`number_inputs.rs:43`,
  checked at `:2432`), not every formatter. Its opening sentence could
  say so.
- **claude-C4-v3-3:** three computed limits whose value no rustdoc shows
  (`Workspace::MAX_DOCUMENT_BYTES`, `VectorRasterizer::MAX_COVERAGE_WORK`,
  `Renderer::READBACK_TIMEOUT`). It is confirmed, but documentation
  completeness is outside the rubric's scope, so it goes to slice 5's
  documentation work.
