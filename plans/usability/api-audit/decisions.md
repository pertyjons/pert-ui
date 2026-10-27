# API audit: policy decisions

The maintainer's decisions on the policy questions and consistency
proposals in [the audit findings](findings.md), slice 4 of [the
developer-usability plan](../../archive/developer-usability.md).

**How the recommendations were drafted:** a fresh Claude agent per rubric
group read the rows, the reports, the contracts and the source, and counted
the affected call sites. The author reviewed them. Codex reviewed them, and each
group's agent revised its section for Codex's points (each group ends with
its revision notes). Each recommendation applies four criteria,
in order:
1. no consumer misled into wrong behavior or data loss;
2. AGENTS.md's "do not change authority, callback timing or cancellation
   merely for spelling symmetry";
3. one learnable rule over many exceptions;
4. cost in public items and call sites.

**Options** are usually:
- **(A)** keep the code and write the rule or exception into contracts.md,
  AGENTS.md or rustdoc;
- **(B)** change the API;
- **(C)** a third way, when one exists.

**Consequence** says what the fix becomes:
- **doc-only:** slice 5;
- **additive:** slice 6, compatible;
- **breaking:** slice 6, with a changelog entry giving the replacement
  usage.

**Status:** decided. Codex approved the recommendations after two rechecks,
and the maintainer approved all of them as recommended on 2026-10-22.

## Summary

| IDs | Question | Recommended | Consequence | Decision |
|---|---|---|---|---|
| G1-3 | Order in keyed-item constructors | C: owner `ui`, key, bound state, the rest | breaking, 2 items | approved |
| G1-5 | `ChoiceId::as_str` against `get()` | A: string exception (`as_str`) | doc-only | approved |
| G1-6 | `get()` for compound values | A: single-value types only | doc-only | approved |
| G1-9 | Where shared limits live, constant types | A: validation limits on the shared format's owner, else the reporting error; rendering caps on the error type of the operation they protect; `Self` for value limits | doc-only; `PowerExponent::MIN/MAX` to `Self` (breaking, tiny) | approved |
| G1-10 | Constants for zero, one, nonnegative bounds | A: only chosen limits get constants | doc-only | approved |
| G1-12 | May a documented convenience clamp | A: presentation mappings may saturate; scroll offsets and layout of valid saved sizes stay clamped; invalid explicit text offsets, identities and stored preferences are rejected | doc-only, edits an AGENTS.md rule | approved |
| G1-13 | Meanings of `with_` | A: three meanings by receiver (constructor with a supplied part, consuming builder, scoped closure), documented | doc-only | approved |
| G1-14 | `Color::rgba` takes linear values | B: rename to `Color::from_linear` | breaking, about 90 call sites | approved |
| G1-15 | Parameter names `ui`, `controller` | B: rename in four constructors | rustdoc only | approved |
| G1-16 | Collections and keyed containers under "Choice sources" | A: outside, one sentence | doc-only | approved |
| G1-18 | `SplitRule::Ratio(f32)` | B: `NormalizedValue` (needs a const constructor or a non-const `rule`) | breaking, 1 variant | approved |
| G2-1 | No-argument boolean shortcuts | A: widen the naming rule | doc-only | approved |
| G2-2, G2-14 | Collections and redescription | B: adopt permissions and callbacks, including removal; controller, source and structure stay | breaking in behavior | approved |
| G2-3 | `closable`, `minimizable`, `App::resizable` as `bool` | A: fixed window parts, documented | doc-only | approved |
| G2-4 | `disabled(bool)` on data descriptions | A: data descriptions take `bool` | doc-only | approved |
| G2-5 | Accumulating builders | A: "Adds/Offers" builders append, documented | doc-only | approved |
| G2-6 | Two-way bindings | B: `bind_<property>` (author's correction of the draft's `_binding`) | breaking, 3 items | approved |
| G2-8 | `#[must_use]` on receipts | C: only where dropping hides an otherwise unreported outcome (failure or cancellation); `ToastReceipt` loses it | not breaking | approved |
| G2-13 | Reconciliation publishes a parameter snapshot | A: a narrow lifecycle exception, doc-only only if a test first shows dependents settle in the same flush (else a scheduling fix); `ContextMenu` gets private non-signal state | doc-only plus a private fix, or slice 6 scheduling | approved |
| G2-17 | `on_select` against `on_selection_change` | A: keep both, document the difference | doc-only | approved |
| G2-18 | `MultiSelect::filter` closure | C: keep it, add a doctest using `FilterMatcher` | doc-only | approved |
| G2-19 | `set_wake_delivery` takes a `Box` | B: `impl Fn`; `Box` callers still compile | additive | approved |
| G2-20 | Editor exceptions need contracts.md | B: yes; a contracts.md sentence that parse and range errors appear only in inspection. F6 stays a behavior decision; recommended route: report them through `on_rejection` | doc-only; F6's route changes when existing `on_rejection` handlers run (behavior) and adds `UiMessage`/`StringRole` variants (additive, both `#[non_exhaustive]`) | approved |
| G3-16 | `ParameterState` and `*State` | A: `*State` by ownership | doc-only | approved |
| G3-17 | `*Snapshot`, `*Inspection`, `*Report` | A: name these suffixes in the rule | doc-only | approved |
| G3-18 | `*Outcome` and operation results | A: the rule goes one way only; continuations allowed | doc-only | approved |
| G3-19 | `FormStatus` as a controller | A: application-owned, like `ListSelection` | doc-only | approved |
| G3-21 | `Pending<T>` and receipts | A: a cross-thread reply is not a receipt | doc-only | approved |
| G3-1 | Owned against reactive reads | C with one accepted residual risk (`Parameter::snapshot` stays a live signal, documented); 8 renames. D also renames `Parameter::snapshot` | breaking, about 20 call sites (D: about 340) | approved |
| G3-2 | `reorderable`/`resizable` | A: document the rule the code follows | doc-only | approved |
| G3-3 | Accessor pairs on role tables | C: `ThemeMetrics::metric`, `UiStrings::text`/`with_text` | breaking, 3 methods | approved |
| G3-4 | `DragPayload::get::<T>` | B: `downcast_ref` | breaking, 1 method | approved |
| G3-5 | `is_open` against `is_active` | B: `EditorSession::is_active` | breaking, 1 method, no call sites | approved |
| G3-7 | `f32` against `Length` | C: given and stored lengths are `Length`; `QuickPick`/`CommandPalette::width` change | doc-only plus 2 breaking builders | approved |
| G3-12 | Family prefixes | C: `DropPosition` to `ListDropPosition`; `PickStatus` stays | breaking, 1 type | approved |
| G3-23 | `UiStrings::MAX_LENGTH` counts bytes | B: `MAX_TEXT_BYTES` | breaking, 1 constant | approved |
| G3-24 | `Completed` against `Succeeded` | B: `TaskStatus::Succeeded` | breaking, 1 variant | approved |
| G4-6 | Incidental `Send` on `Divider`, `Svg` | B: a private `!Send` marker on every description now | breaking in auto traits only | approved |
| G4-7 | `RichText` not `Send` | A: document as UI-thread; sendable font key to follow-ups | doc-only | approved |
| G4-8, G4-9 | `#[non_exhaustive]` on growing records | C: the records only the runtime builds (10 structs, `EventKind`, `ValidationCode`); records a component builds and status enums stay exhaustive | breaking, 6 test literals | approved |
| G4-10 | `AnyValue`/`AnySpec` returned as errors | A: documented std-idiom exception | doc-only | approved |
| G4-15 | Losing `Copy` to carry keys | C: identity payloads only; `LayoutError`, `ParameterError` lose `Copy` | breaking, with G4-14 | approved |
| G4-19, G4-20 | Standard-trait impls and `# Errors` | A: one-line forwarding for `TryFrom`/`FromStr`; for serde the type's `# Serialization` section suffices (two types gain one) | doc-only | approved |
| G4-21 | `DrawError` into `RuntimeError` | A: keep the exclusion; document `Result<_, DrawError>` for test helpers | doc-only | approved |
| G4-22 | `From<Box<dyn Error + Send + Sync>>` for `RuntimeError` | A: no; doc pointers | doc-only | approved |
| G4-23 | User text in `Display` | A: documented exception for application and OS text | doc-only | approved |
| G4-4 | Proposal: more derives on records | A: not adopted | none | approved |
| G4-30 | `From` back to the raw number | B: add it to the six other validated scalars | additive | approved |
| G5-11 | `SeriesSource` in the prelude | A: stays at the root, one exclusion sentence | doc-only | approved |
| G5-16 | Parameter host sink and monitor types | B: they are "hosts"; 7 names join the prelude | additive | approved |
| G5-26, G5-27, G5-14 | How far the prelude rule reaches | D: names a consumer writes or keeps go to a new `pert_ui::component` module (about 26 names), not the prelude; `IgnoredEntry` joins the application prelude; follows slice 9's smaller prelude | additive | approved |
| G5-30 | `WindowError` in the facade prelude | B: add it under `native` | additive | approved |
| G5-32 | A testing prelude | A: none until it owns F2's helpers, root pointers instead; `ApplicationError` joins the prelude | doc-only, plus 1 additive name | approved |

## G1

Draft recommendations for the maintainer. Facts were rechecked at HEAD
`467d6ec` (branch developer-usability). Call-site counts are `rg` counts over
`apps/`, `crates/` (examples, guide, tests, READMEs, rustdoc) and are
approximate. "contracts" means `docs/contracts.md`, "naming rules" its
section "Public naming rules" (`docs/contracts.md:1149`).

### G1-3 Does an identity key lead keyed-item constructors?

- **Question:** keyed-item constructors (`MenuItem::new`, `Tab::new`,
  `Section::new`, `Property::new`, `Element::new`) start with the key. Should
  `MenuItem::toggle(key, label, value)` and `MenuItem::radio(key, label,
  selection, value)` (`menu.rs:299`, `:333`) keep the state behind the
  label, or does the bound state still come right after the key?
- **Options:** (A) a key-first exception that leaves the state after the
  label (contracts sentence only). (B) state before the key: `toggle(value,
  key, label)`. This breaks the key-first order every keyed item shares.
  (C) the key leads as identity and the bound state follows it:
  `toggle(key, value, label)`, `radio(key, selection, value, label)`.
- **Recommendation: C.** Criterion 3: one learnable order. G1-2 already
  moves `Checkbox::new`/`Toggle::new` to `(value, label)`. Under A,
  `MenuItem::toggle` would then be the only toggle with its label before its
  value. C keeps the key first, as every keyed item has it, and applies the
  same state-before-content rule after the key. The order needs no authority
  or callback change (criterion 2 does not apply). The types differ, so the
  old order fails to compile and does not misbind silently.
- **Rule text:** add one sentence to the naming rules: "Constructor
  parameters run: the state owner (`ui`), then the stable key of a keyed
  item, then the state the constructor binds, then content and options."
  AGENTS.md's "Required state owners lead" stays as it is.
- **Consequence:** breaking. Two items change, at about 25 call sites in 7
  files (`menu_changes.rs` 12, `diagram_editor/app.rs` 4, gallery
  `surfaces.rs` 4, `guide/surfaces.md` 4, `pert-ui-runtime/README.md` 3,
  `menus.rs` 3, a `menu_bar.rs` doc). Slice 6, together with G1-2.
- **Depends on:** G1-2 (same migration), G1-15 (same sentence).

### G1-5 `ChoiceId::as_str` or `get()`

- **Question:** the naming rules read a validated value's raw value with
  `get()`. `ChoiceId` (`core/choice.rs:50`) borrows its text with `as_str`.
  Is that a string exception, or is `get` added?
- **Options:** (A) a string exception: "a validated string borrows its text
  with `as_str()`", as std's `String::as_str` does (C-CONV). (B) add `get()`
  beside `as_str`, or rename it (renaming is breaking).
- **Recommendation: A.** No consumer was misled, and `as_str` is the name
  a Rust user tries first. `ChoiceId` is the only single-string validated
  newtype. The other `&str` accessors (`ParameterId::key`, `ViewState::kind`)
  read one field of a larger value. Adding `get` would give the same value
  two names, which goes against criterion 3.
- **Consequence:** doc-only. Extend the naming rules' `get()` sentence:
  "…with `get()` (a string with `as_str()`) unless the unit itself is the
  point of the type." Slice 5.
- **Depends on:** G1-6 (the same sentence).

### G1-6 Does `get()` cover compound values?

- **Question:** `Color::channels`, `Rect::values` and `Transform2D::entries`
  (`drawing.rs:302`, `:200`; `transform.rs:158`) return their whole private
  arrays and have per-component accessors. Does the `get()` rule apply to
  them?
- **Options:** (A) state that the rule covers single-value types, and that
  a compound value has named component accessors plus a whole-value
  accessor named for its content. (B) rename the three accessors to `get`
  (breaking).
- **Recommendation: A.** As Claude notes, `get()` returning `[f32; 4]` says
  less than `channels` (which channel order? which colour space?). Nothing
  misleads a consumer. The rule's wording presupposes one raw value.
- **Consequence:** doc-only. One clause in the naming rules: "Types with
  several components expose named component accessors and a whole-value
  accessor named for its content (`channels`, `entries`)." Slice 5.
- **Depends on:** G1-5 (same sentence), G1-14 (`channels` rustdoc should say
  "linear" once `rgba` is renamed).

### G1-9 Where a shared limit lives, and the type of its constant

- **Question:** some limits bound several types:
  - `TextEditError::MAX_TEXT_BYTES`;
  - `Workspace::MAX_PATH_BYTES` and `MAX_STRING_BYTES`, which bound
    `WorkspacePath` and `WorkspaceValue`;
  - `NumberInputError::MAX_PRECISION`;
  - `TextError::MAX_FONT_PIXELS`;
  - the module constant `pert_ui::svg::MAX_NESTING`.

  Where does such a limit live? The sub-question: `PowerExponent::MIN`/`MAX`
  are `f32`, while `NormalizedValue`, `UiZoom`, `ViewScale` and `Capacity`
  use `Self`.
- **Options:**
  - (A) Make today's placement the rule, for two kinds of limit. Each is
    defined once, its rustdoc says why it lives there, and there are no
    aliases.
    - A validation limit, which input fails: on the owner of the shared
      format (`Workspace` for its document, the `svg` module for SVG input,
      which has only free-function entry points), otherwise on the error
      type whose one variant reports it. Bounded constructors link to it
      from `# Errors`.
    - A rendering cap, at which output degrades instead of failing: no
      variant reports it, so it lives on the error type of the operation it
      keeps from failing, whose variant rustdoc names it as the case that is
      not an error. `TextError::MAX_FONT_PIXELS` is one: larger text is
      rasterized at 256 px and stretched (`text.rs:58-79`).
  - (B) Move each limit to a "principal" bounded type, with aliases or links
    from the others. This is breaking, and picking the principal type is
    arbitrary for text.
  - (C) For the cap only: move it to `TextLayout`, which records the scale
    used. Breaking, 1 item, 4 intra-doc links outside `text.rs`
    (`rg -n 'MAX_FONT_PIXELS' apps crates`), but the cap also bounds icon
    rasterization (`drawing.rs:1439`), which `TextLayout` does not cover.
  - Constant type: either "a bound on the value itself is `Self`, a bound on
    a measure (bytes, count, depth, digits) has that measure's integer
    type", or leave the type free.
- **Recommendation:** A for placement. It matches
  `docs/contracts.md:1141`'s "one limit has one reporting variant" and the
  existing reasoned example `MAX_TEXT_BYTES` (`text_input.rs:29-41`), and
  moves nothing. The cap clause keeps `MAX_FONT_PIXELS` where a consumer
  asking "is large text rejected?" finds the answer (`TextError::TooLarge`).
  Adopt the `Self`/measure typing rule: every value-bound
  constant except `PowerExponent`'s already follows it. A `Self` constant
  can be used directly as a valid value, and `get()` gives the raw number.
- **Consequence:**
  - Doc-only for placement: the naming-rules sentence with both kinds, plus
    a "why it lives here" sentence on `MAX_PRECISION` (reported by
    `InvalidPrecision`) and `MAX_FONT_PIXELS` (a cap no variant reports).
  - Breaking but tiny for `PowerExponent::MIN`/`MAX: Self`: 1 item pair and
    4 lines in `pert-ui-core/tests/parameters.rs`.
  - Slice 5 (rule) and slice 6 (`PowerExponent`).
- **Depends on:** G1-7 (`PresentationRate::MIN`/`MAX` should then be
  `Self`), G1-8, G1-10.

### G1-10 Do intrinsic bounds need `MIN_*`/`MAX_*` constants?

- **Question:** do intrinsic zero, one, nonzero or nonnegativity bounds need
  constants? Examples:
  - `StrokeStyle` miter limit ≥ 1;
  - `Color` channels and `SplitSizing::ratio` in `0..=1`;
  - nonzero `Capacity`, `ContentQuery` limit and budget, and `VirtualAxis`
    lanes;
  - nonnegative `Length`, `Size`, `Constraints`, `Rect` and `Viewport`.
- **Options:** (A) the rule covers limits the library chose; intrinsic
  domain bounds are stated in rustdoc and `# Errors` without constants.
  (B) add the constants (additive, but many items).
- **Recommendation: A.** A `MIN_LANES = 1` or a `Length::MIN = 0` constant
  tells a consumer nothing that the error text does not. The rule exists so
  that chosen numbers (16 KiB, 1000 Hz) are discoverable and stay in step
  with the code. The unit interval is the one case with a real type,
  `NormalizedValue`. Reported values use that type (see G1-18). Raw `f32`
  inputs validated by a fallible constructor (`Color::from_linear` under
  G1-14, `SplitSizing::ratio`) stay as they are.
- **Consequence:** doc-only. Add to the naming rules: "Limits the library
  chooses are constants; intrinsic bounds of a domain (nonzero,
  nonnegative, the unit interval) are documented on the constructor and its
  errors." Slice 5.
- **Depends on:** G1-9, G1-18.

### G1-12 May a documented presentation convenience clamp?

- **Question:** `ProgressStatus::of` (`progress.rs:52-66`) maps
  `completed > total` to a full track and `total == 0` to `Indeterminate`.
  This is documented, reasoned and tested, but it is not a "control limit",
  the only clamp AGENTS.md allows. `TableColumn::width` and
  `TableController::set_column_width` floor at `MIN_WIDTH` (V3-2's setter
  part, `table.rs:95`, `:765`).
- **Options:**
  - (A) Name a second allowed case. A constructor may saturate when it is
    total over its input type and documents what every input presents as.
    Invalid explicit text offsets, identities and stored preference values
    are still rejected, not coerced.
  - (B) Add a checked `try_of`. As Codex notes, that leaves `of`'s status
    unsettled.
- **Recommendation: A.**
  - `of(u64, u64)` has no invalid input: an overshooting count is ordinary
    data. Rejecting it would push every caller to clamp by hand, and that is
    less safe.
  - The `Table` width floor is a layout minimum and fits the existing
    control-limit wording. It needs no new rule, only the wording that says
    so.
  - The rule must keep G1-11 (`TextSelection::new` clamping explicit
    offsets) and V3-2 (restore overwriting a stored width below the floor)
    as departures, and must not forbid documented clamping. Scroll offsets
    limited to the scroll range (`VirtualAxis::clamp_offset`,
    `virtual_axis.rs:253`, used by `VirtualList`) are a control limit. Layout
    clamping a valid saved size to the current bounds without rewriting it
    (`docs/contracts.md:1015`) is a layout limit, not coercion of the
    preference. Hence "text offsets" and "stored preference values".
- **Consequence:** doc-only. In AGENTS.md "Rust and data invariants",
  "clamp only for documented control-limit behavior" becomes "clamp only for
  documented control or layout limits, or a documented presentation mapping
  that is total over its input (`ProgressStatus::of`); reject, never coerce,
  an invalid explicit text offset, identity or stored preference value". The
  naming rules link to it. The AGENTS.md edit is the user's decision.
  Slice 5.
- **Depends on:** G1-11, V3-2.

### G1-13 Which meanings does `with_` keep?

- **Question:** `with_` has four uses:
  - state-supplying constructors (`Menu::with_state`,
    `Menu::icon_with_state`, `CollectionFilter::with_query`);
  - `NavigationHistory::with_capacity`;
  - builders on value and data types (`Element::with_grow`,
    `StrokeStyle::with_cap`, `Color::with_alpha`, `Palette::with_color`,
    `ToolWindowChange::with_*`, `WorkspaceSettings::with_*`,
    `ClickCounter::with_*` and others);
  - scoped closures (`Painter::with_clip`, `SeriesSource::with_window`).

  Component builders use bare nouns.
- **Options:**
  - (A) Keep the names and document three meanings, told apart by receiver:
    - associated `Type::with_x(x, …)`: `new` with `x` supplied by the
      caller, as `Vec::with_capacity` is (`with_state`, `with_query`);
    - consuming `self.with_x(…) -> Self` (or `Result<Self, _>`): a builder on
      a value or data type. It need not have a readback getter
      (`ClickCounter::with_interval`; `ToolWindowChange::with_geometry` sets
      public fields), but a getter, where one exists, takes the bare name;
    - borrowing `(&self).with_x(closure)`: scoped access returning the
      closure's result (`Painter::with_clip`, `SeriesSource::with_window`).
  - (B) Claude's rename: drop `with_` from builders and rename the
    state-supplying constructors, keeping `with_capacity` and the closure
    forms. This touches 30 items (27 builders, 3 constructors) at about 340
    lines in 85 files, and bare names collide with existing getters
    (`Color::alpha`, `Element::grow`, `Element::detachable`,
    `Element::searchable`).
  - (C) A, plus rename only the three state-supplying constructors.
- **Recommendation: A.**
  - No round-1 run or probe recorded a cost from the prefix (r1-2's
    `with_grow` errors were about the argument type).
  - Where a getter exists, the builder/getter split is what makes `Element`
    and `Color` readable.
  - The "supplied part" meaning already matches std, so C would rename for
    spelling only.
  - B fails criterion 4.
- **Count:** `rg -n '\.with_(mapping|step|ring_capacity|kind|placement|overlay|mode|geometry|label|detachable|searchable|grow|rich_label|cap|join|miter_limit|interval|distance|count|lanes|alpha|zoom|theme_mode|density|metric|color|default_color)\b|\b(Menu::(icon_)?with_state|CollectionFilter::with_query|Self::(icon_)?with_state|Self::with_query)\b' apps crates | grep -v 'fn '`. Each name is defined once in the workspace, and the
  generic ones (`label`, `color`, `kind`, `mode`, `count`, `step`, `cap`,
  `join`) were read: all receivers are pert-ui types. The 52 `with_clip`/
  `with_window` calls are excluded.
- **Consequence:** doc-only. One sentence in the naming rules stating the
  three meanings, that a builder needs no getter but a getter takes the bare
  name, and that component descriptions use bare builder names. Slice 5.
- **Depends on:** G3-3 (`Palette::with_color`, `ThemeMetrics::with_metric`,
  `UiStrings::with` accessor pairs), G2-6 (`Collapsible::bound` as another
  state-supplying spelling).

### G1-14 Name the linear colour constructor for its space

- **Question:** `Color::rgba` (`drawing.rs:258`) takes linear channels with
  the same signature as `Color::from_srgb` (`:327`). Normalized sRGB
  fractions from a design tool therefore produce the wrong colour without an
  error (0–255 values are rejected). Should it be named for its space?
- **Options:**
  - (A) Keep `rgba` and strengthen its rustdoc and the guide.
  - (B) Rename it to `Color::from_linear`. That completes the existing pairs
    `from_srgb`/`to_srgb`, `from_srgb8`/`to_srgb8` and `from_hex`/`to_hex`,
    with `channels` as the linear reader.
  - (C) Add `from_linear` as an alias and keep `rgba`. That leaves two names
    for one thing, against AGENTS.md's no-alias stance during the review.
- **Recommendation: B.**
  - Criterion 1: the CSS-like unprefixed name is the one a consumer guesses,
    and the guess gives a wrong colour without a failure. Documentation does
    not reach a caller who never reads it.
  - The rename is mechanical, has no change of behaviour, and makes one
    learnable `from_<space>` family.
- **Consequence:** breaking. One item changes, at about 90 call sites
  (tests and wgpu/cpu smoke code mostly; `guide/appearance.md` 4,
  `pert-ui-runtime/README.md` 2, gallery 2). Update the `channels` rustdoc to
  say "linear". Slice 6.
- **Depends on:** G1-6, G1-10.

### G1-15 Use `ui` and `controller` as parameter names throughout

- **Question:** `ListController::new` and `ZoomController::new`
  (`virtual_list.rs:275`, `zoom_view.rs:140`) name the state owner
  `runtime`, while every other constructor uses `ui`. `Dialog::new` and
  `AlertDialog::new` (`dialog.rs:195`, `:506`) name the controller `state`,
  while every other constructor uses `controller`. Should the names be made
  consistent?
- **Options:** (A) leave the names. (B) rename the four parameters and the
  rustdoc prose that names them ("`runtime` is any StateOwner…").
- **Recommendation: B.** It is cheap and local (criterion 3), and it shows
  in rustdoc and IDE hints. It is not breaking: parameter names are not
  part of the Rust API, and the `api/` listings omit them (checked:
  `api/pert-ui-runtime/default.txt:6683`, `:4040`), so the listing gate does
  not change.
- **Consequence:** rustdoc/signature text only, not breaking. Four functions
  and their doc paragraphs change; there are no call-site changes. The
  parameter names go into the G1-3 sentence ("the state owner (`ui`)…, a
  controller argument is `controller`"). Slice 5 (or slice 6 alongside
  G1-2).
- **Depends on:** G1-3.

### G1-16 Do lazy collections and keyed content fall under "Choice sources"?

- **Question:** `VirtualList::new`/`estimated` (`virtual_list.rs:635`,
  `:659`) take only a reactive `ReadSignal<ListSource>` under `new` and a
  sizing name. `ToolWindowLayout::new(center)` adds its keyed windows with
  `window`/`windows` (`tool_windows/mod.rs:765-809`). Must they use the
  `new`/`bound`/`for_parameter` vocabulary?
- **Options:**
  - (A) One contract sentence in "Choice sources" (`docs/contracts.md:38`).
    The vocabulary governs controls whose options are values written into a
    selected-value binding. Collections whose rows are content (`VirtualList`,
    `Table`, `Tree`, `FileBrowser`) and keyed containers (`Tabs`,
    `ToolWindowLayout`) are outside it, and name their constructors by
    content or sizing.
  - (B) Migrate the constructors to `bound` and constructor-only window lists
    (breaking).
- **Recommendation: A.**
  - A `ToolWindow` carries content and actions, not an option value
    (Disputed facts 2), and floating windows are not exclusive.
  - `VirtualList`'s selection lives in its controller and `ListSelection`,
    not in an option binding.
  - Applying `bound` there would blur the term the choice controls depend
    on. Nothing in the evidence shows a consumer misled.
- **Consequence:** doc-only. Add one sentence to contracts "Choice sources".
  Slice 5.
- **Depends on:** G2-5 (whether `ToolWindowLayout::window(s)` is a
  documented accumulating builder), G2-6.

### G1-18 `SplitRule::Ratio(NormalizedValue)` (consistency proposal)

- **Question:** `SplitRule::Ratio(f32)` (`split.rs:141-145`), reported
  through `SplitSizing::rule` and `SplitPane::on_resize`, documents a
  `0..=1` share as raw `f32`. Its sibling variants carry `Length`, and
  `NormalizedValue` already types `ProgressStatus::Determinate`. Should the
  payload become `NormalizedValue`?
- **Options:** (A) keep `f32` and record "reported unit-interval values may
  be raw". (B) change the payload to `NormalizedValue`, keeping
  `SplitSizing::ratio(f32)` as the fallible input.
- **Recommendation: adopt (B).**
  - It is cheap and local (criterion 3). No consumer outside `split.rs`
    matches on `SplitRule::Ratio`: the tests and gallery use only
    `SplitRule::First` and `SplitSizing::ratio`.
  - With G1-10's rule it gives one convention: unit-interval values the
    library reports are `NormalizedValue`, and inputs are validated `f32`.
  - Implementation note: `SplitSizing::rule` is a `const fn`, and
    `SplitSizing::EQUAL` stores `0.5`. Either the private `Sizing::Ratio`
    stores a `NormalizedValue`, which needs a public const way to build 0.5
    in `pert-ui-core` (none exists; only `MIN`/`MAX`), or `rule` stops being
    `const`. Both are visible in the API listing and belong in the
    CHANGELOG.
- **Consequence:** breaking. One variant payload changes (possibly plus the
  constness of `rule`), at about 13 lines inside `split.rs` and no external
  call sites. Slice 6.
- **Depends on:** G1-10.

#### G1 revision notes

- Point 5 (G1-12): Codex is right. The prohibition now covers invalid explicit text offsets, identities and stored preference values ("reject, never coerce"). The reasoning keeps `VirtualAxis::clamp_offset` (a control limit) and layout clamping of valid saved sizes (`contracts.md:1015`, a layout limit) explicitly allowed. The summary row in decisions.md should read "offsets" as "explicit text offsets" and "persisted values" as "stored preference values".
- Point 9 (G1-13 count): Codex is right. B is recounted by affected item: 30 items (27 builders, 3 constructors) at 340 lines in 85 files. The 52 `with_clip`/`with_window` calls and `with_capacity` are excluded, the receivers were checked and the `rg` pattern is given.
- Point 11 (G1-9): Codex is right. Option A now separates validation limits from rendering caps and gives caps their own placement rule: on the error type of the operation they keep from failing. Option C (move to `TextLayout`) is weighed and rejected because the cap also bounds icons. The recommendation stays A, with no move.
- Point 12 (G1-13): Codex is right. Option A now has three meanings by receiver and drops the getter requirement: a builder needs no readback getter, and a getter, where one exists, takes the bare name. Scoped closure methods are a separate meaning. The summary row should read "three meanings by receiver".

## G2

Read at HEAD 467d6ec (`crates/` unchanged since the audit commit). Call-site
counts are `rg` hits over `crates/` and `apps/`. They include definitions,
rustdoc and tests, so they are upper bounds. G2-1 and G2-5 count call lines
per affected item, with receiver checks and the pattern given. Every consistency proposal of G2
(G2-6, G2-17, G2-18, G2-19) is also in the index, so each is decided once,
in index order.

### G2-1 No-argument boolean shortcuts

- **The question:** seventeen builders set one boolean with no argument
  and have no way back: `Button::frameless`, `Group::frameless`,
  `Field::hide_label`, `QuickPick`/`CommandPalette::nonmodal`,
  `Dialog::require_response`, `Toolbar::attached_below`, `Divider::vertical`,
  `RegionLayout::fixed`,
  `InspectionQuery::{parameters, windows, sources, validation, application}`,
  `Semantics::names_children`, `PointerEvent::outside` and
  `App::software_rendering`. contracts.md "Public naming rules" reserves
  no-argument shortcuts for `disabled()` and `read_only()`. Do these get
  `bool` setters, or does the rule allow them?
- **Options:**
  - (A) Widen the rule in "Public naming rules": "A no-argument builder
    moves one fixed option of a description away from its default and has
    no way back (`disabled()`, `read_only()`, `frameless()`, `nonmodal()`).
    An option the application decides from data takes a `bool`."
  - (B) Add a `bool` setter beside each (additive), or change each
    signature to take a `bool` (breaking).
- **Recommendation:** A.
  - Every instance is a mode or presentation choice that applications fix
    when they write the description.
  - No round 1 run, probe or friction entry hit one: `rg` over
    `plans/usability/runs` finds none.
  - A shortcut cannot mislead a consumer into wrong behavior; at worst a
    computed choice needs an `if`.
  - B would add seventeen methods, or, as signature changes, touch 83 call
    lines for symmetry alone: 24 in the gallery, examples, guide and
    READMEs, 24 in library source, 35 in tests. Per item: `frameless` 28,
    `InspectionQuery` 12, `RegionLayout::fixed` 11, `nonmodal` 8,
    `outside` 7, `Divider::vertical` 5, `require_response` 4,
    `software_rendering` 3, `hide_label` 2, `names_children` 2,
    `attached_below` 1.
  - Count: `rg -n '\.(frameless|hide_label|require_response|attached_below|names_children|outside|software_rendering)\(\)'`
    (each name has one definer, two for `frameless`);
    `rg -n '\.nonmodal\(\)' | rg -v 'popover.rs|runtime/surfaces.rs'`
    (drops the private `SurfaceKind` getter);
    `rg -n 'Divider::new\(\)\.vertical\(\)'` (the other 21 `.vertical()`
    calls are the private `ToolEdge` getter);
    `rg -nU 'RegionLayout::new\(\)\s*\.fixed\(\)' | rg RegionLayout`;
    `rg -n '\.(parameters|windows|sources|validation|application)\(\)' | rg -v 'client\.validation'`;
    each over `crates apps`.
- **Consequence:** doc-only: one contracts.md sentence. Rustdoc already
  states each default. Slice 5.
- **Depends on:** G2-4. The same sentence settles where a `bool` is
  required.

### G2-2, G2-14 Collections and redescription

- **The question:** a same-key redescription of `VirtualList`, `Tree` and
  `Table` keeps the first description's permission descriptors
  (`resize_permission`, `reorder_permission`) and callbacks
  (`on_selection_change`, `on_activate`, `on_text_input`, `reorderable`,
  `placement`, `on_sort`, `cell_editor`). `FileBrowser::folder_filter`
  does the same. Each type's rustdoc says so (virtual_list.rs:428-437,
  tree.rs:885-891, table.rs:1218-1222). contracts.md:164 says "permissions
  and callbacks follow the new description". contracts.md:169-171 exempts
  only "controller and options". AGENTS.md says a same-key description
  replaces earlier restrictions. Does the exception cover permissions and
  callbacks?
- **Options:**
  - (A) Name the exception in contracts.md: "Component redescription", and
    a sentence in "Permissions and availability" that the collections keep
    their first permission descriptors and callbacks.
  - (B) Adopt the new permissions and callbacks on a compatible
    redescription, including removal, as `Tabs` does. Keep the controller,
    the source binding and the structural options (axis, lanes, overscan,
    columns, row height).
  - (C) Adopt the permissions only, and record the callbacks as the
    exception.
- **Recommendation:** B, by criterion 1. The code contradicts the stated
  contract, with real consequences:
  - A redescription with a fixed `reorder_permission(false)` is ignored, so
    the user can still reorder rows the application has locked. That is a
    capability restriction (authority), not spelling.
  - A callback that captures description-time state, which is the normal
    idiom inside `Dynamic` and `Each`, keeps acting on the old state. For
    example, `on_activate` or `on_sort` can act on the previous document.
  - Within the same controller-owned family, `ColorPicker` already lets its
    permission and handler follow redescription (contracts.md:478-480, the
    diagram probe's decision), and so does `Tabs::on_select`. B therefore
    leaves one learnable rule: retained state stays, permissions and
    callbacks follow.
  - C keeps the stale-closure hazard.
- **Consequence:** breaking in behavior only; no signature changes.
  - New `reconcile` adoption in `VirtualList`, `Table` and `Tree` (none of
    them has one today), and in `FileBrowser`.
  - Removing `reorderable`, or a fixed denial, cancels an armed or active
    reorder, as revocation does. A removed `on_selection_change` discards
    its pending payloads under the notification contract. An open cell
    edit keeps its editor.
  - Whether `drag_operations` follows or stays is decided in the slice.
  - Rewrite the rustdoc paragraphs of the four types and about six method
    docs.
  - Narrow contracts.md:169-171 to "controller, source and structural
    options".
  - Flip `compatible_redescription_retains_collection_permission_bindings`
    (audit_collection_permissions.rs:685), and add replace and remove
    tests.
  - No example, gallery or guide call site changes. Slice 6d.
- **Depends on:** G2-17. With callbacks following redescription, the only
  remaining difference between the two selection notifications is
  coalescing.

### G2-3 `closable`, `minimizable` and `App::resizable`

- **The question:** `FloatingWindow::closable(bool)` and
  `minimizable(bool)` replace their value, while `movable` and `resizable`
  on the same type take `impl Into<Permission>`. `App::resizable(bool)`
  does the same. Should they take `Permission` like their siblings?
- **Options:**
  - (A) Record in contracts.md "Permissions and availability": "Which
    title-bar buttons a `FloatingWindow` has (`decorated`, `closable`,
    `minimizable`) and `App::resizable`, an attribute of the native window,
    are ordinary `bool` properties. They choose parts rather than restrict
    a capability, and `WindowController` still closes and minimizes from
    code."
  - (B) Take `impl Into<Permission>`. Calls compile, but `closable(false)`
    followed by `closable(true)` changes behavior, and presence becomes
    bindable, so the title bar must regenerate.
- **Recommendation:** A.
  - `movable` and `resizable` are Permissions because revoking them cancels
    a gesture. A button's presence has no gesture to cancel; it sits with
    `decorated(bool)`.
  - Codex's verification also notes that conversion is not automatically
    right.
  - B changes behavior for symmetry, which criterion 2 advises against.
    There is no consumer evidence for it.
- **Consequence:** doc-only: one contracts.md sentence, and a rustdoc line
  on `closable` and `minimizable` saying they are fixed part choices. 12
  `minimizable` hits; there are no `closable` call sites. Slice 5.
- **Depends on:** none.

### G2-4 `disabled(bool)` and `read_only(bool)` on data descriptions

- **The question:** `PickItem::disabled(bool)`, `SemanticItem::disabled(bool)`,
  `Semantics::disabled(bool)` and `Semantics::read_only(bool)` take a `bool`.
  contracts.md reserves these names for no-argument shortcuts. Does the
  rule apply to descriptive data builders?
- **Options:**
  - (A) State that data descriptions report state as a `bool`, and that
    the shortcut rule covers component builders whose availability is a
    `Permission`.
  - (B) Rename them, for example to `set_disabled` (breaking).
- **Recommendation:** A.
  - Of the 35 `.disabled(expr)` call sites, nearly all are computed:
    `.disabled(!self.enabled)` in controls.rs, select.rs, multi_select.rs,
    color_picker.rs and others.
  - So a `bool` is the right shape. Semantics describe state; they do not
    restrict a capability.
  - A rename would touch about 40 call sites, mostly inside built-in
    widgets, for no consumer benefit.
- **Consequence:** doc-only: a clause in the G2-1 sentence. Also a rustdoc
  line saying that `disabled(true)` withdraws actions and that a later
  `disabled(false)` does not restore them (diagnostics.rs:392-397, 579-585).
  Slice 5.
- **Depends on:** G2-1.

### G2-5 Builders that accumulate

- **The question:** `FloatingWindow::title_action`,
  `ToolWindowLayout::window` and `windows`, `Semantics::action` and
  `actions`, and `SemanticItem::action` add to earlier values. Their
  neighbors `ToolWindow::actions` and `Semantics::items` replace. Are
  element builders a documented family exception, or do they replace?
- **Options:**
  - (A) Add to "Public naming rules" a sentence such as: "An element
    builder appends one element to a described collection (a layout's tool
    windows, a title bar's actions, a description's semantic actions), and
    its plural form appends several. A builder that sets a whole list or one
    slot (`Semantics::items`, `ToolWindow::actions`, `Toast::action`)
    replaces." Fix the rustdoc of `Semantics::actions`, which does not say
    that it accumulates, and of `Toast::action`, which begins "Adds" but
    holds one action (notifications.rs:156-163), so rustdoc wording alone
    cannot carry the rule.
  - (B) Rename the accumulating builders to `add_*` (breaking).
  - (C) Make them replace, so that `title_action` takes all elements at
    once (breaking).
- **Recommendation:** A.
  - These are container composition, not ordinary properties; Codex's
    verification did not establish a violation.
  - Appending is what a caller building a layout element by element
    expects, and the rustdoc already says "Adds" or "Offers".
  - No consumer hit a surprise.
  - B would rename 126 call lines, 15 of them in the gallery, examples,
    guide and READMEs: `ToolWindowLayout::window` 92 (80 in tests),
    `title_action` 7, `Semantics`/`SemanticItem::action` and
    `Semantics::actions` 27 (22 in library source); `ToolWindowLayout::windows`
    has no call.
  - C is worse than its count: a layout with several `.window` calls would
    still compile and silently keep only the last window (criterion 1).
  - Count: `rg -n '\.title_action\('`;
    `rg -n '\.window\(\s*$|\.window\((ToolWindow|window|window_named)\b'`
    (each open call's next line checked to be a `ToolWindow`; the other
    `.window(` calls are `VirtualAxis::window` and a private lookup);
    `rg -n '\.windows\([^)]'` (all `slice::windows`);
    `rg -n '\.action\(SemanticAction::|item(\.expanded\(expanded\))?\.action\(if|\.actions\('`
    (the other `.action(` calls are `Toast::action`); each over
    `crates apps`.
- **Consequence:** doc-only: one contracts.md sentence and two rustdoc
  fixes. Slice 5.
- **Depends on:** none.

### G2-6 Two-way boolean bindings (consistency proposal)

- **The question:** two builders bind state that the component also writes:
  `Collapsible::bound(Signal<bool>)` (beside `expanded(bool)`) and
  `ToolWindowLayout::names(Signal<bool>)` (beside `show_names(bool)`).
  `PropertyGrid::shared_label_width(Signal<Length>)` is the only other one.
  No convention names them. Also, `bound` is the choice-source constructor
  name, and the `_signal` suffix means a one-way `ReadSignal`.
- **Options:**
  - (A) Keep the names and add a contracts.md sentence that lists them.
  - (B) One rule in "Public naming rules": "A builder that binds state the
    component also writes takes `impl Into<Signal<T>>` and is named
    `bind_<property>`." This gives `Collapsible::bind_expanded`,
    `ToolWindowLayout::bind_show_names` and `PropertyGrid::bind_label_width`.
    - Author's note: the draft proposed `<property>_binding`. That clashes
      with G3-1, where `x_binding()` is a reactive read.
    - With `bind_`, each spelling has one meaning:
      - `_signal`: a builder taking a one-way `ReadSignal`;
      - `bind_`: a builder taking a two-way `Signal`;
      - `_binding()`: a read returning a `ReadSignal`.
- **Recommendation:** B.
  - There is no consumer evidence, but the change is cheap and local
    (criterion 3).
  - It also removes a real ambiguity: `bound` names a different contract
    (a dynamic choice source) on choice controls.
  - It completes the boolean rule with a two-way counterpart to `_signal`.
- **Consequence:** breaking renames of three items.
  - About 15 call sites: `Collapsible::bound` 6 (gallery 1, tests 5),
    `names` about 3, `shared_label_width` about 6.
  - `Breaking:` changelog entries with the replacement usage. Slice 6a.
- **Depends on:** G1-16 (the scope of the `bound` choice-source name).

### G2-8 `#[must_use]` on receipts

- **The question:** `TaskReceipt` and `ToastReceipt` are `#[must_use]`
  although their failures also propagate through the flush. The guide,
  the examples and the gallery discard them with `let _receipt` (about 22
  sites). Do receipts stay `must_use` by design, or only where dropping
  loses the only report of an outcome, a failure or a cancellation? This
  joins slice 3's F19.
- **Options:**
  - (A) Keep `must_use` on every receipt, and document the `let _ =` idiom
    as intended.
  - (B) Remove it from both.
  - (C) Rule: a receipt is `#[must_use]` when dropping it can hide an
    outcome that nothing else reports, a failure or a cancellation.
    - Keep `TaskReceipt`. A `defer` whose owner is hidden or unmounted is
      reported as `Cancelled` only in the receipt, so a deferred save can
      silently not happen (triage group B). Give it a reason that names
      this.
    - Remove the attribute from `ToastReceipt`. A failing action
      propagates, and expiry or dismissal is presentation.
    - Keep `CommandReceipt`, `RevealReceipt`, `ListRevealReceipt` and
      `PresentationReceipt` (D1).
- **Recommendation:** C: `must_use` only where dropping hides an otherwise
  unreported outcome, a failure or a cancellation.
  - It is one rule a consumer can learn, and it matches D1's settled facts.
  - It protects against the one silent loss (criterion 1).
  - Two round 1 testers hit the `TaskReceipt` warning. Triage judged that
    its note pointed them to the fix correctly, so the warning stays but
    with an accurate reason.
  - B would hide the cancellation.
- **Consequence:** nonbreaking.
  - Remove one attribute and rewrite one reason.
  - The receipt sentence in "Public naming rules" gets the rule, and the
    `EventContext::defer` rustdoc gets a line saying that a dropped
    receipt still surfaces `Failed` through the flush but `Cancelled` only
    through the receipt.
  - The three `ToastReceipt` `let _receipt` sites (guide/surfaces.md:314,
    gallery surfaces.rs:383, 393) may become plain statements.
  - Slice 6, with G2-7 and F19.
- **Depends on:** G2-7 (F19, `CommandId`) and G3-21 (the receipt rule for
  `Pending<T>`).

### G2-13 Reconciliation that publishes signals

- **The question:** `ContextMenu::reconcile` publishes its private
  `enabled_signal` (menu.rs:1472). In numeric reconciliation, a rebind or
  a fixed denial drops the parameter's `EditSession`
  (numeric_controls.rs:565-588, number_inputs.rs:1274, 1309). The drop
  cancels the edit and publishes the public `Parameter::snapshot` at once,
  which invalidates its subscribers and computations (signal.rs:431-450,
  parameter_binding.rs:678); only the terminal event waits for the next
  safe point (parameter_binding.rs:679-686). `EditSession`'s rustdoc
  permits this: "Terminal lifecycle cleanup is permitted during an update"
  (parameter_binding.rs:698-699). contracts.md:166-167 says the gesture "is
  cancelled at the next safe point", and number_inputs.rs:1260-1261 says
  the same of the cancellation. Is immediate lifecycle cleanup an
  exception to AGENTS.md's "Never publish signals from reconciliation"?
- **Options:**
  - (A) A narrow lifecycle exception:
    - contracts.md "Component redescription": "A redescription that
      rebinds a parameter control or denies it a fixed permission cancels
      the parameter's gesture during reconciliation and publishes its
      snapshot at once; the terminal event follows at the next safe
      point."
    - AGENTS.md's sentence then refers to the exceptions contracts.md
      names.
  - (B) Disarm the control at once, and defer the cancellation and its
    publication together: the control hands its session to the runtime's
    deferred queue instead of dropping it.
  - In both options, `ContextMenu` gets non-signal shared state, such as a
    `Cell` in the opener link. It is private and has no API change.
- **Recommendation:** A for parameters, conditional on the test below, and
  the private fix for `ContextMenu`.
  - B also keeps the snapshot consistent with the parameter's state; the
    draft's claim of an inconsistent window was wrong. B is rejected on
    timing and authority, criterion 2:
    - The deferred queue runs after the flush's settle loop
      (runtime.rs:5443), so dependents would see the cancellation only in
      a further flush, unless the loop gains a new hook.
    - Until then the parameter holds an orphaned edit. A notification
      callback in the same flush that writes the parameter ends it as
      `Rejected(Conflict)` instead of `Cancelled`
      (parameter_binding.rs:516-520), and another control on the parameter
      is refused with `EditError::Busy` (pert-ui-core edit.rs:381-382).
  - The draft cited `ParameterHost`'s "applies at once"
    (parameter_binding.rs:432). That sentence concerns cleanup a sink
    causes, not reconciliation; the `EditSession` sentence above is the
    governing one.
  - Evidence before A counts as doc-only: a test that a component and a
    `Computed` reading the old parameter's snapshot both settle in the
    reconciling flush. The settle loop (runtime.rs:5465-5471) repeats while
    components are pending, but a computation queued during
    `update_pending` is refreshed only if the loop runs again; that case is
    unverified. control_reconcile.rs:98 and redescription.rs:256 check only
    the snapshot value and the terminal outcome.
  - `ContextMenu`'s signal has a single event-time `get()` reader and
    nothing can subscribe to it. Replacing it is cheap and removes the
    literal breach.
- **Consequence:**
  - Parameters: the dependency test first. If it passes, doc-only:
    contracts.md:166-167, number_inputs.rs:1260-1261 and the AGENTS.md
    wording, which the maintainer must approve. If the computation case
    fails, the settle loop needs a scheduling fix (slice 6d), and B is
    reconsidered against it.
  - `ContextMenu`: a private implementation change, nonbreaking.
  - Slice 5 for the documentation, slice 6d for the test, any scheduling
    fix and `ContextMenu`.
- **Depends on:** G2-2 and G2-14, if the collections start adopting
  permissions in `reconcile`. A fixed denial there must cancel without
  publishing, or fall under the same exception.

### G2-17 One name for settled selection notifications (consistency proposal)

- **The question:** `Tabs::on_select` and the collections'
  `on_selection_change` are both settled selection notifications.
  contracts.md:527-534 documents the difference: a collection observes its
  `ListSelection` at update and coalesces writes between updates, and its
  payload carries the active item and the selected keys. Should both have
  one name?
- **Options:**
  - (A) Keep both names. Add a sentence in "Component notifications":
    "`on_select` reports each settled tab change; `on_selection_change`
    reports a collection's bound selection as observed at its update,
    coalescing writes between updates." Rephrase `VirtualList`'s rustdoc,
    which defines its timing by reference to `Tabs::on_select`
    (virtual_list.rs:834-836).
  - (B) Rename `Tabs::on_select` to `on_selection_change` (breaking,
    about 21 hits in 10 files).
- **Recommendation:** A.
  - The difference is deliberate and documented, so a shared name would
    suggest equal semantics. Claude withdrew the same pair under N2 for
    this reason.
  - There is no consumer evidence.
  - Under criterion 2, a deliberate difference is documented rather than
    renamed.
- **Consequence:** doc-only: one contracts.md sentence and one rustdoc
  fix. Slice 5.
- **Depends on:** G2-2 and G2-14.

### G2-18 `MultiSelect::filter` against matcher values (consistency proposal)

- **The question:** `MultiSelect::filter` takes a bare
  `Fn(&str, &Choice<T>) -> bool`. `QuickPick`, `CommandPalette` and
  `CollectionFilter` take matcher values that have presets. Should
  `MultiSelect` take a matcher value too?
- **Options:**
  - (A) Keep `filter`, and record that it is a boolean filter over the
    typed choice.
  - (B) Add `MultiSelect::matcher(FilterMatcher)` beside `filter`
    (additive).
  - (C) Keep `filter`, and add a rustdoc example that reuses the presets:
    `let m = FilterMatcher::fuzzy(); .filter(move |q, c| m.matches(q, c.label()))`.
- **Recommendation:** C.
  - It gives the presets without a second way to set one rule. Round 1's
    F34 and the diagram probe both rejected two spellings for one thing.
  - It keeps access to the choice value, which `filter` provides and a
    label matcher would not.
  - There is no consumer evidence for B.
- **Consequence:** doc-only: one doctest on `MultiSelect::filter`. It must
  say that the query arrives as typed and is not trimmed, unlike the
  collection filters (F7c). Slice 5.
- **Depends on:** F7c.

### G2-19 `Runtime::set_wake_delivery` takes a `Box` (consistency proposal)

- **The question:** `set_wake_delivery(&self, deliver: Box<dyn Fn() -> bool + Send + Sync>)`
  is the only public function that takes a closure as a `Box`. Its sibling
  `pert_ui_wgpu::Renderer::new` takes `impl Fn`. Should it take `impl Fn`?
- **Options:**
  - (A) Keep the `Box`, and document that host wake delivery is a separate
    contract.
  - (B) Take `impl Fn() -> bool + Send + Sync + 'static`, boxed inside.
- **Recommendation:** B.
  - The change is cheap and local (criterion 3).
  - Existing callers that pass `Box::new(…)` still compile, because
    `Box<F>` implements `Fn`.
  - Triage group E names this method as the blocking wait for headless
    tests, so test writers will call it.
- **Consequence:** a source-compatible signature change, which counts as
  additive for callers. The `api/` listing changes.
  - 16 hits in 6 files; the internal callers (platform/mod.rs:82 and the
    tests) may drop `Box::new`.
  - Slice 6a.
- **Depends on:** none.

### G2-20 `on_rejection` coverage, and where an editor exception lives

- **The question:**
  - `TextField` and `TextArea` report their length limits through
    `on_rejection`. The number inputs do not report parse or range errors,
    yet their rustdoc says "with the rules of `TextField::on_rejection`"
    (number_inputs.rs:2086-2087) before saying that such a draft is not a
    refusal.
  - Those errors reach no application-observable route. They appear only
    as the inspection property `validation_error`
    (number_inputs.rs:1868-1874) and as the semantics' invalid flag; the
    message is fixed English. `FieldStatus` is application-owned: "The
    library presents status and never decides it" (field_status.rs:1-5).
    Probe 2 of triage group B verified this: `on_rejection` silent, field
    status normal.
  - The difference is documented in the item rustdoc but not in
    contracts.md. Does B3's allowance for an editor's "own documented
    contracts" cover rustdoc, or does an exception need a contracts.md
    sentence?
- **Options:**
  - (A) Item rustdoc suffices.
  - (B) An exception needs a contracts.md sentence; rustdoc elaborates it.
- **Recommendation:** B for where an exception lives. AGENTS.md says "Keep
  the explicit family exceptions in contracts.md", and the rubric's
  exception rule asks for "the exact contract sentence". A consumer who
  reads contracts.md should learn every difference from it.
  - Documentation does not resolve F6, which stays an open behavior
    decision for the checkpoint. The recommended route for it: the number
    inputs report built-in parse and range refusals through `on_rejection`
    with localized `InputRejection`s, as `TextField` does its length
    limits (text_fields.rs:1405-1426). One coverage rule then replaces the
    exception, and a `Field` can show the refusal through the documented
    `FieldHandle` route.
  - Criterion 1 favors that fix: two round 1 runs, with severity "slow",
    missed parse errors on the number inputs.
- **Consequence:**
  - Slice 5, doc-only, until F6 is decided: add to "Text and draft
    editing" or "Editor sessions": "`on_rejection` reports
    `InputRejection`s only: the normalizer's, and on `TextField` and
    `TextArea` the length limit. A number input's parse and range errors
    are not rejections: they appear only in inspection
    (`validation_error`) and as its semantics' invalid flag, and no
    callback or binding reports them to the application." Make
    number_inputs.rs:2086-2087 say that it shares the `Some`/`None`
    protocol, not the coverage.
  - If F6 takes the recommended route (slice 6d): a behavior change, as
    `on_rejection` runs in new cases, plus additive `UiMessage` and
    `StringRole` variants (both `#[non_exhaustive]`). The sentence then
    states the one rule.
- **Depends on:** F6, at the checkpoint.

#### G2 revision notes

- Point 1 (G2-13): Codex is right on both counts. The "applies at once" citation concerned sink-caused cleanup and is replaced by `EditSession`'s "Terminal lifecycle cleanup is permitted during an update"; option B is now "disarm at once, defer cancellation and publication together", evaluated and rejected on criterion 2 (cancellation lands after the flush's settle loop at runtime.rs:5443, and the orphaned edit turns a same-flush host write into `Rejected(Conflict)` and refuses other controls with `Busy`), and A is conditional on a test that component and `Computed` dependents settle in the reconciling flush, with a scheduling fix in slice 6d if the computation case fails. Summary row: "A: a narrow lifecycle exception, doc-only once a dependency test passes (AGENTS.md wording to approve); `ContextMenu` gets private non-signal state | doc-only plus a test and a private fix".
- Point 3 (G2-20): Codex is right. The contracts sentence no longer claims a field status; it says the errors appear only in inspection (`validation_error`) and as the semantics' invalid flag, with no callback or binding. F6 stays an open behavior decision, with the recommended route stated: report parse and range refusals through `on_rejection` with localized `InputRejection`s. Summary row: "B: yes; add the `on_rejection` sentence (inspection-only errors) and correct the rustdoc. F6 stays open; recommended: report built-in refusals through `on_rejection` | doc-only; F6's fix a behavior change".
- Point 9 (G2-1): recounted per item with receiver checks: 83 call lines (24 consumer-facing, 24 library, 35 tests), not about 90; `Divider::vertical` has 5 calls, not 36. The item list is seventeen, not about fifteen. Recommendation unchanged.
- Point 9 (G2-5): recounted the affected layout builder `ToolWindowLayout::window` (92 lines, 80 in tests) instead of `ToolWindow::new`; B renames 126 lines in total. Added that C would silently drop windows from multi-window layouts, and that `Toast::action` begins "Adds" but replaces, so the rule names element builders rather than rustdoc wording and gains a second rustdoc fix. Recommendation unchanged.
- Point 10 (G2-8): Codex is right. The heading, question and recommendation now say "an otherwise unreported outcome, a failure or a cancellation". Summary row: "C: only where dropping hides an otherwise unreported outcome (failure or cancellation); `ToastReceipt` loses it | not breaking".

## G3

Draft recommendations for the maintainer. Call-site counts are `rg` line
counts at the working tree of `developer-usability` (467d6ec). "Consumers"
means `crates/pert-ui/examples`, `apps/gallery` and `crates/pert-ui/src/guide`.
"Tests" means `crates/*/tests`. The rule text cited is docs/contracts.md
"Public naming rules" (:1149–1163) unless stated otherwise.

## Policy questions

### G3-16 `ParameterState` keeps `*State`

- **The question:** may `pert_ui_core::ParameterState`, a mutating edit state
  machine (`begin`, `update`, `commit`, `cancel`, `write_base`), keep the
  `*State` suffix, which the contract gives to "passive captured data"?
  (G3-16)
- **Options:**
  - **(A)** Keep the name and redefine the suffix by ownership rather than
    by passivity. Proposed sentence for "Public naming rules": "`*State` names
    a value its owner holds directly, never a shared handle: captured component
    or device state (`ViewState`, `KeyboardState`) or an owned state machine
    (`ParameterState`)." This is the line that sets `*State` against
    `*Controller` (a cloneable, shared handle).
  - **(B)** Rename it, for example to `ParameterMachine` (Codex), and keep
    `*State` for captures. `ParameterEditor` (Codex's other name) collides
    with the runtime's editor widgets and `EditorSession`.
- **Recommendation: (A).** The name misleads no consumer: every mutator
  takes `&mut self` and the rustdoc opens with "Synchronous edit state
  machine". The type is not in the facade (`api/pert-ui/` has no
  `ParameterState`), so ordinary applications never meet it. No candidate
  name is better. The ownership reading is one rule that covers the existing
  `*State` types without an exception (criteria 2 and 3).
- **Consequence:** doc-only. One sentence in contracts.md replaces "passive
  captured data are `*State`". No call sites. Slice 5.
- **Depends on:** G3-17, which rewrites the same sentence.

### G3-17 Suffixes for snapshots, inspection records and reports

- **The question:** does "passive captured data are `*State`" reach the
  roughly 20 owned captures named `*Snapshot`, `*Inspection` and `*Report`,
  and plain records such as `ChildInfo`, `CommandEntry` and `ProfileStats`?
  Only `ViewState`, `KeyboardState` and `TextInputState` use `*State`.
  (G3-17)
- **Options:**
  - **(A)** Amend the contract to name the suffixes that are already used.
    Proposed text, after the `*State` sentence of G3-16:
    - "An owned point-in-time copy of observable data is a `*Snapshot`
      (`ParameterSnapshot`, `InspectionSnapshot`)."
    - "One inspection record is an `*Inspection`."
    - "The summary of a finished check or batch is a `*Report`
      (`ValidationReport`, `DrainReport`)."
    - "Other passive data take a descriptive noun (`ChildInfo`,
      `CommandEntry`, `ProfileStats`)."
  - **(B)** Rename about 20 types to `*State`. Serialized schemas must not
    change.
- **Recommendation: (A).** The current names are accurate and read well.
  `ParameterSnapshot` vs `ParameterState` in particular is a useful
  distinction that (B) would erase. Renaming would touch many sites for no
  consumer benefit: `ParameterSnapshot` alone has 7 consumer, 24 test and 57
  source lines, and the `*Inspection` types have 117 source and 28 test lines.
  Criteria 2 and 4 hold.
- **Consequence:** doc-only. Three or four sentences in "Public naming rules".
  Slice 5.
- **Depends on:** G3-16 (the same paragraph). G3-18 if `*Report` is kept
  distinct from `*Outcome`.

### G3-18 `*Outcome` and operation results

- **The question:** does "A `*Outcome` names how an interaction ended" cover
  operation results? Results that the rustdoc calls outcomes carry other
  names: `pert_ui_wgpu::Frame`, `ChildReveal`, `ListingDelivery`,
  `ResponseDisposition` and `ContentJump`, whose `Pending` variant is a
  continuation. Conversely, `*Outcome` also names handling flags and
  operation results: `InputOutcome`, `TextKeyOutcome`, `PreviewOutcome`,
  `PublishOutcome` and `FilterOutcome`. (G3-18)
- **Options:**
  - **(A)** Amend the sentence to make the rule one-directional. Proposed
    text: "A `*Outcome` is the non-error result of an interaction or
    operation, distinct from `Result` as an error carrier. A result may
    instead be named for what it delivers (`Frame`, `ChildReveal`), and it
    may carry a continuation for unfinished work (`ContentJump::Pending`)."
    Keep the existing distinction between local processing and acceptance by
    the authority.
  - **(B)** Rename one side, for example `InputOutcome` → `InputHandling`
    or `Frame` → an outcome name.
- **Recommendation: (A).** No consumer is misled. Each type documents what
  it reports, and none of them is an error carrier. Renaming
  `InputOutcome` would rewrite 61 consumer lines in 9 files, 178 test lines
  and 753 source lines (criterion 4). The D9 resolution also requires an
  allowance for `ContentJump` under any reading.
- **Consequence:** doc-only. The sentence in "Public naming rules" is
  replaced. Slice 5.
- **Depends on:** G3-17 (`*Report` against `*Outcome` for operation
  summaries).

### G3-19 Application-owned coordinators and the controller role

- **The question:** is an application-owned coordinator that widgets only
  follow, such as `FormStatus` and the `FieldHandle`s it issues, in the
  `*Controller` role? (G3-19; it feeds G3-13 and G3-14)
- **Options:**
  - **(A)** Add a sentence beside the `ListSelection` sentence. Proposed
    text: "Application-owned state that widgets follow through handles the
    application keeps, such as collection selection (`ListSelection`) and
    form status (`FormStatus` and its `FieldHandle`s), is named for what it
    holds. `*Controller` is reserved for handles to state that a mounted
    widget owns."
  - **(B)** Rename to `FormController` and `FieldController` (and
    `ToastController`).
- **Recommendation: (A) for `FormStatus` and `FieldHandle`.** The source
  shows the application owns them: "owned by the application", it "holds no
  component alive and schedules nothing", and `Form::new` does not take it.
  Calling them controllers would misdescribe who owns the state (criterion
  1, in the direction of (A)).
  - `ToastQueue` is not covered. Its host owns presentation and closes the
    queue on removal, which puts it closer to the controller role, so it
    stays with G3-13.
  - (A) does not settle G3-14. `*Status` on a cloneable handle still departs
    from `*Status` as a value enum, so a non-controller `Form…` name remains
    open there.
- **Consequence:** doc-only. One sentence in "Public naming rules". Slice 5.
  `FieldHandle` leaves G3-13. Any rename of `FormStatus` under G3-14 is
  breaking: 4 consumer, 8 test and 17 source lines.
- **Depends on:** G3-13, G3-14.

### G3-21 Cross-thread one-time replies and the receipt rule

- **The question:** does the receipt rule need an exception for
  `Pending<T>`? It is the `Send + Sync`, consume-once reply that every
  `InspectionClient` request returns, with `wait`, `wait_timeout` and
  `try_reply`, not `status()`/`status_binding()`. (G3-21)
- **Options:**
  - **(A)** State the boundary in one clause after the receipt sentence:
    "A cross-thread reply that carries its answer and is consumed once, such
    as the `Pending<T>` an `InspectionClient` request returns, is not a
    receipt."
  - **(B)** No change (Claude's position: there is no departure).
  - **(C)** Design a separate report family (Codex). There is no evidence
    of a need for it.
- **Recommendation: (A).** The D4 resolution settled that `Pending<T>` is a
  reply, not a receipt, so there is nothing to change in code. One clause
  stops a reader of the rule from expecting a `status_binding()` on a type
  that must not have a UI-local binding. The cost is negligible (criterion
  3).
- **Consequence:** doc-only. One clause in "Public naming rules". Slice 5.
- **Depends on:** none.

## Consistency proposals

### G3-1 Owned and reactive reads

- **The question:** should one convention govern owned and reactive reads?
  Today `x()` is owned and `x_binding()` reactive on receipts, but bare
  names return a `ReadSignal` elsewhere (`Parameter::snapshot`, the
  `status()` of `DialogController`, `WindowController` and `FieldHandle`),
  and reactive reads are also spelled `*_signal` and `observe`. (G3-1)
- **Options:**
  - **(A)** Keep everything and document the current spread.
  - **(B)** Every reactive read becomes `x_binding()`: `Parameter::snapshot`
    (about 280 `.snapshot().get/with` test and consumer lines), the three
    reactive `status()`, `FormStatus::summary` and about 20 other bare
    getters (`ListSelection::selected`, `TableController::sort`, …).
  - **(C)** Adopt the rule only where a type offers both forms. Proposed
    text: "`_signal` names a method that takes a one-way `ReadSignal`, and
    `bind_<property>` a builder that takes a two-way `Signal`. When a type
    offers an owned and a reactive read of one value, the bare name is the
    owned read and `x_binding()` the reactive one. A type with only a
    reactive read uses the bare name." Renames: the seven
    `UiSettings`/`ThemeManager` getters `*_signal` → `*_binding` (`zoom`,
    `text_rendering`, `motion`, `strings`, `debug_overlay`, `density`,
    `mode`) and `DeliveryMonitor::observe` → `snapshot_binding`.
    `Runtime::command_enabled_signal`/`command_enabled_binding` conform.
  - **(D)** (C), plus `Parameter::snapshot` → `snapshot_binding`, so that
    a read named `snapshot` always captures.
- **Recommendation: (C), with one accepted residual risk.**
  - Different return types catch a `ReadSignal` passed where a value is
    expected, but not a retained one. `let before = parameter.snapshot();`
    compiles on `Parameter<T>`, and a later `before.get()?` returns the
    current value. The same line captures on `AnyParameter` (with `?`,
    `crates/pert-ui-runtime/tests/parameter_delivery.rs:547`) and on
    `ParameterState` (`crates/pert-ui-core/tests/edits.rs:209`), and G3-17
    gives `*Snapshot` a point-in-time meaning. Codex's point holds.
  - No consumer or run hit it: the 23 consumer calls read at once or keep
    the signal as a binding on purpose (`apps/gallery/src/main.rs:231`).
    (D) would cost 23 consumer, about 265 test and about 50 source lines
    (`rg '\.snapshot\(\)'` without `monitor`/`state`/`client`/`profiler`/
    `self` receivers or `?`, checked by hand), so (C) records the risk
    rather than removing it (criterion 4); the maintainer may choose (D).
  - (C) is one learnable rule at a cost of 10 consumer and 6 test lines for
    the `*_signal` getters and 1 consumer and 2 source lines for `observe`.
  - Rustdoc: `Parameter::snapshot` says its signal is live and `get()`
    captures; `AnyParameter::snapshot` says it is owned.
- **Consequence:** breaking but small: 8 renamed methods and about 20 call
  sites (slice 6a). The rule and both rustdoc notes are doc-only (slice
  5). The medium row is resolved without the general rename.
- **Depends on:** G2-6 (`bind_<property>` keeps `_binding()` a read and
  `_signal` one-way), G3-17 (`*Snapshot` as a point-in-time copy).

### G3-2 `reorderable` and `resizable`: capability or restriction

- **The question:** should `reorderable`/`resizable` have one meaning? Today
  they install a callback (`VirtualList`, `Tree`, `Tabs`), restrict a
  built-in capability by `Permission` (`ToolWindow`, `ToolWindowLayout`,
  `SplitPane` and siblings) or declare a capability by `bool`
  (`TableColumn`, `App`), and `Table` uses `resize_permission`. (G3-2)
- **Options:**
  - **(A)** Record the rule the code already follows. Proposed text for
    contracts "Permissions and availability" (:296–298): "A capability the
    component provides itself is restricted by the adjective taking
    `impl Into<Permission>` (`SplitPane::resizable`,
    `ToolWindowLayout::reorderable`). Where an application callback or a
    per-item flag creates the capability (`VirtualList::reorderable`,
    `TableColumn::resizable`), the adjective creates it and the restriction
    is `*_permission`."
  - **(B)** Rename all restrictions one way and move the list callbacks to
    `on_reorder`.
- **Recommendation: (A).**
  - The three meanings take different argument types, so no call site can
    confuse them silently.
  - Renaming the callback to `on_reorder` would itself mislead (criterion 1).
    The reorder callback performs the edit and returns acceptance, whereas
    `on_*` callbacks report successful writes.
  - contracts.md:296–298 already requires callback requirements, booleans
    and restrictive composition to stay distinct, and (A) states the rule
    that keeps them so.
  - The cost of (B) is 3 consumer and 23 test lines for `.reorderable(`,
    and 15 consumer and 32 test lines for `.resizable(`.
- **Consequence:** doc-only. One paragraph in contracts.md. Slice 5.
- **Depends on:** G2-3 (whether `App::resizable` and the title-button flags
  take `Permission`).

### G3-3 Accessor pairs of the role-keyed tables

- **The question:** should the three role-keyed tables share one accessor
  pair? Today they are `Palette::color`/`with_color`,
  `ThemeMetrics::get`/`with_metric` and `UiStrings::get`/`with`. (G3-3)
- **Options:**
  - **(A)** Keep the pairs and document them.
  - **(B)** Use `get(role)`/`with(role, value)` everywhere, as proposed. This
    renames `Palette::color`/`with_color` and `ThemeMetrics::with_metric`
    (28 consumer lines: 21 `Palette::color`, 6 `with_color`, 1
    `with_metric`) and separates it from `Painter::color(role)`.
  - **(C)** Use the noun pair everywhere: `ThemeMetrics::get` → `metric`
    (matching `Painter::metric` and `MeasureContext::metric`), and
    `UiStrings::get`/`with` → `text`/`with_text`. Validation and palette
    fallback stay as they are.
- **Recommendation: (C).** It aligns the tables with the context readers
  that applications already use (`Painter::color`, `Painter::metric`).
  It is cheap and local (criterion 3): no consumer line for `ThemeMetrics`,
  one gallery line (`apps/gallery/src/language.rs:108`) for `UiStrings`, and
  about 100 mechanical test lines. It also takes keyed lookups out of the
  `get()` rule, which contracts reserves for a validated value's raw value.
  (B)'s 21 `Palette::color` lines are receiver-checked: the consumers'
  other `.color(` calls are `Painter::color` or a live-widget builder.
  Pattern: `rg '(palette|light|dark|Palette::dark\(\)|\])\.color\('`.
  The count narrows (B)'s cost but leaves (C) cheaper.
- **Consequence:** breaking. Three methods are renamed; about 1 consumer
  line and about 100 test lines change. Slice 6a.
- **Depends on:** G1-13 (`with_` as a keyed property builder on a value
  type), G1-5 and G1-6 (reach of `get()`), and G3-23 (the same table's
  limit constant).

### G3-4 `DragPayload::get::<T>` as a downcast

- **The question:** should `DragPayload::get::<T>` become `downcast_ref` to
  match `ApplicationError::downcast_ref` and std's `Any`? (G3-4)
- **Options:**
  - **(A)** Keep the name.
  - **(B)** Rename it to `downcast_ref::<T>`.
- **Recommendation: (B).** This is the std name for the operation, the only
  other typed downcast in the API uses it, and it removes a `get` that is
  neither a raw-value read nor a keyed lookup. It is cheap: 1 consumer line
  (`apps/gallery/src/pages/data/drag_and_drop.rs:393`), 2 test lines and a
  few doc lines.
- **Consequence:** breaking: 1 method and about 3 call sites. Slice 6a.
- **Depends on:** G1-5 and G1-6 (reach of `get()`).

### G3-5 `is_open` and `is_active` for a begun, unended edit

- **The question:** should one word name a begun, unended edit?
  `EditorSession::is_open` and `DraftEditorController::is_active` both pair
  with `begin`/`end`. (G3-5)
- **Options:**
  - **(A)** Keep both.
  - **(B)** Rename `EditorSession::is_open` → `is_active`.
  - **(C)** Rename `DraftEditorController::is_active` → `is_open`.
- **Recommendation: (B).** The two types share a lifecycle (`begin`, `end`,
  predicate). `is_active` and "active session" are the words the draft
  editor's rustdoc already uses widely. (B) costs nothing outside the
  source: no consumer line, no test line and about 8 source and doc lines.
  (C) would change 2 consumer and 36 test lines.
- **Consequence:** breaking: 1 method and no consumer call sites. Slice 6a.
- **Depends on:** none.

### G3-7 Logical lengths as `f32` or `Length`

- **The question:** should logical-pixel lengths taken as bare `f32` become
  `Length`, and should the metric readers return `Length`? The `f32` items
  are the `width` of `CommandPalette` and `QuickPick`, the row heights of
  `VirtualList`, `Table`, `Tree` and `FileBrowser`, and `ThemeMetrics::get`,
  `MeasureContext::metric` and `Painter::metric`. (G3-7)
- **Options:**
  - **(A)** Change nothing and document the space.
  - **(B)** Adopt `Length` throughout, as proposed. The row heights still
    need `GeometryError::Empty` for zero, and the metric readers would break
    361 source lines of painter arithmetic.
  - **(C)** Write the convention the code mostly follows and fix the two
    departures from it.
    - Proposed text for "Public naming rules": "Geometry is in logical pixels
      unless its name says physical. A length given to the library and
      stored is a `Length`. A constructor whose argument must be positive may
      take `f32` and return `GeometryError`. Lengths read back for
      arithmetic are `f32` logical pixels."
    - Changes: `CommandPalette::width` and `QuickPick::width` take `Length`,
      like `TableColumn::width` and `PropertyGrid::label_width`.
- **Recommendation: (C).**
  - Logical pixels are the documented default space
    (docs/rendering-platform.md:9). No finding shows a logical length read
    as something else; the physical-pixel risk is G3-6.
  - The row-height constructors already validate and fail. Taking `Length`
    would not remove their `Result`, and it would rewrite about 12 consumer
    and about 157 test lines (criterion 4).
  - The two overlay widths are the only stored-length builders on `f32`,
    with 0 consumer and 4 test lines.
- **Consequence:** the convention is doc-only (slice 5). The two `width`
  builders are breaking (slice 6a). Under the same sentence, G3-22's
  setter/getter pairs conform. G3-9's geometry is covered except
  `TextLayout::paint(.., x, y)`, which G3-9 decides separately.
- **Depends on:** G3-9, G3-22 (the same convention), G3-6 (physical sizes).

### G3-12 `DropPosition` and `PickStatus` family prefixes

- **The question:** should `DropPosition` become `ListDropPosition`, and
  `PickStatus` become `QuickPickStatus`? (G3-12)
- **Options:**
  - **(A)** Keep both names.
  - **(B)** Rename both.
  - **(C)** Rename `DropPosition` → `ListDropPosition`, and keep
    `PickStatus`.
- **Recommendation: (C).**
  - `DropPosition` is used only in the `ListDrop*` family (D8). Its name
    places it among the generic `DropOperation`/`DropResponse` types, and
    round 1's F3 testers had to hunt for it beside `ListDrop`
    (criterion 3). The cost is 9 consumer, 8 test and 39 source lines.
  - `PickStatus` follows its own family's prefix: `PickItem`,
    `PickMatcher`, `PickPending` and `PickChoice` all use `Pick*`. Renaming
    it alone would create the inconsistency the proposal wants to remove.
- **Consequence:** breaking: 1 type and about 17 consumer and test lines.
  Slice 6a, with F3's prelude addition (slice 9) naming the new type.
- **Depends on:** G5's prelude questions (G5-26), and F3.

### G3-23 `UiStrings::MAX_LENGTH` counts bytes

- **The question:** should `UiStrings::MAX_LENGTH` (1024 bytes) name its
  unit, given that the text fields' `max_length` counts grapheme clusters
  and the other byte limits say `_BYTES`? (G3-23)
- **Options:**
  - **(A)** Keep the name and state the unit in its rustdoc.
  - **(B)** Rename it to `MAX_TEXT_BYTES`, the name `TextEditError` already
    uses for its byte limit.
- **Recommendation: (B).** It is the only `MAX_*_LENGTH`-style constant.
  A translator counting characters in a non-ASCII language would hit
  `UiStringError::TooLong` earlier than the name suggests. The cost is 1
  test line and the constant's own uses in `strings.rs` (criterion 3).
- **Consequence:** breaking: 1 constant and 1 test line. Slice 6a.
- **Depends on:** G3-3 (`with_text` pairs with `MAX_TEXT_BYTES`) and G1-9
  (where limits live).

### G3-24 `Completed` against `Succeeded`

- **The question:** should one word name a successful terminal state?
  `TaskStatus::Completed` differs from `ProgressStatus::Succeeded` and
  `EventStatus::Succeeded`. (G3-24)
- **Options:**
  - **(A)** Keep both words.
  - **(B)** Rename `TaskStatus::Completed` → `Succeeded`.
- **Recommendation: (B).**
  - `TaskStatus` has the same shape as `ProgressStatus`
    (`Pending`/`Completed`/`Cancelled`/`Failed`), and `Succeeded` is the
    majority word that pairs with `Failed`.
  - It is cheap and local: 1 consumer, 18 test and 1 source line.
  - Renaming toward `Completed` would instead change 6 consumer lines and
    two enums.
- **Consequence:** breaking: 1 variant and about 20 lines. Slice 6a.
- **Depends on:** none.

#### G3 revision notes

- Point 6 (G3-1): removed "no wrong behavior is possible"; the retained `Parameter::snapshot()` signal reading live values is now assessed with evidence and recorded as an accepted residual risk of (C), with a costed option (D) and a rustdoc mitigation; the rule text and G2-6 dependency now use G2-6's `bind_<property>` so `_binding()` stays the reactive read.
- Point 9 (G3-3): Codex is right; `Palette::color` has 21 consumer lines, not about 62 (`Painter::color` was counted). (B)'s cost is restated by affected item (28 lines with `with_color` and `with_metric`) with the rg pattern; the recommendation stays (C), which still costs 1 consumer line.

## G4

### G4-6 Incidental `Send` on `Divider` and `Svg`

- **The question:** `Divider` and `pert_ui::Svg` are the only `Send + !Sync` component descriptions (their `Cell`/`RefCell` fields are `Send`; every other description holds an `Rc`-based callback, `Dynamic` or `Element`). `Svg`'s `Send` also follows the auto traits of a dependency's type, `resvg::usvg::Tree` (`F/svg/component.rs:43`). Removing `Send` later breaks downstream code that moves the value to another thread, whatever the documentation says, and the API listings show it (`impl core::marker::Send for pert_ui::Svg`). Is that later break accepted as a policy exception, or is the auto-trait boundary fixed now? (G4-6)
- **Options:**
  - (A) A compatibility-policy opt-out in contracts.md "Public compatibility policy": "A description's auto traits are not part of its contract unless its rustdoc promises them." A later loss of `Send` still breaks code that sends the value; the sentence only declares that break permitted outside a semver-major release. It needs the maintainer's explicit acceptance as an exception to the usual treatment of auto traits.
  - (B) A stable boundary now: a private `PhantomData<Rc<()>>` marker in both types, so every description is `!Send`, and contracts.md "Element trees and identity" says "Component descriptions are UI-thread values and are not `Send`." Breaking only for code that moves them across threads, which nothing in the repository does. The facade's nesting test builds an `Svg` on a worker and returns only its error (`crates/pert-ui/tests/presentation/svg.rs:355-358`), which the marker still allows.
  - (C) Promise `Svg: Send` in rustdoc with a static assertion test, so a worker can parse one, and mark only `Divider`.
- **Recommendation: B.** Codex refuted the rule departure (`Component` fixes callback threads, not construction affinity) and no consumer was misled, so criterion 1 does not decide. Criterion 3 does: B gives one rule the compiler enforces, matching G4-7's UI-thread `RichText`, where A gives a rule that only the documentation enforces. The change is not merely symmetry: it removes a dependency-driven auto trait. The asymmetry decides the timing: adding `Send` later is compatible, removing it is not, and while the library is unpublished the marker costs two private fields and no call sites (criterion 4). C commits the contract to usvg's auto traits for a need no run showed. Workers already have `pert_ui::svg::rasterize` and `rasterize_mask`, and C remains an additive change if the need appears.
- **Consequence:** breaking in auto traits only, with no call sites. Two private marker fields, a `compile_fail` doctest per type proving `!Send`, the contracts.md sentence, the `api/` listing diff and a changelog line. Slice 6.
- **Depends on:** G4-7 (the same thread statement covers styled text).

### G4-7 `RichText` and `TextSpan` are `!Send`

- **The question:** a span can hold a UI-local `FontId`, so styled text cannot be built on a worker and returned from `Runtime::spawn`. Should it become `Send` (a sendable font key), or is the UI-thread restriction documented? (G4-7)
- **Options:** (A) document it on `RichText` and `TextSpan`: "UI-thread values, because a span can name a registered font (`FontId`). A worker returns plain text or its own data, and the UI thread builds the `RichText`." (B) Make the font key sendable. That is a design change to `FontId` and `FontManager` identity.
- **Recommendation: A.** No run demonstrated the need (codex: need not established). The auto traits match ownership, and criterion 1 is met once the restriction is stated. B goes to follow-ups.md as a design question, not to this audit.
- **Consequence:** doc-only, two rustdoc paragraphs. Slice 5.
- **Depends on:** G4-6.

### G4-8, G4-9 `#[non_exhaustive]` on inspection records and status enums

- **The question:** `InspectionSnapshot`, `ComponentInspection` and `ValidationCode` are exhaustive, although their rustdoc records growth through schemas 5 and 10–14. 30 further DTOs and status enums are exhaustive with no recorded growth. Which get `#[non_exhaustive]`? A snapshot also contains records that components build (`ComponentDetails` and its `ValidationHints`, `BoxModelInspection`, `BorderInspection` and `InspectValue`), shared values (`Viewport`, `AnyParameterSnapshot`, `Invalidation`) and status enums (`EventStatus`, `DeliveryStatus`, `DebugMode`, `DebugOverlay`). So the boundary follows who constructs a type, not containment. (G4-8, G4-9)
- **Options:**
  - (A) Keep all exhaustive and record that inspection types follow schema-breaking releases.
  - (B) Mark all 33.
  - (C) Mark only the *runtime-built records*, the types that only `Runtime::inspect` and `Runtime::validation_report` construct. These are the structs `InspectionSnapshot`, `WindowInspection`, `ComponentInspection`, `ParameterInspection`, `SourceInspection`, `DragInspection`, `EventRecord`, `ValidationReport`, `ValidationIssue` and `ValidationMarker`, and the enums `EventKind` and `ValidationCode`, whose variants name runtime operations and checks. Everything else in a snapshot follows its own policy:
    - `ComponentDetails`, `ValidationHints` and `BoxModelInspection` are built by `Component::inspect` (literal at `crates/pert-ui/examples/diagram_editor/canvas.rs:867`). They stay exhaustive and are built from `Default`.
    - `BorderInspection` (width and color) and `InspectValue` (a JSON-like value algebra, as `serde_json::Value` is) are closed.
    - Shared values keep their own type's policy; `KeyCode` and `Semantics` are already non-exhaustive.
    - Status and outcome enums are closed, `EventStatus` included (`TaskStatus`, `ToastStatus`, `DialogOutcome`, `CommandStatus`, `PresentationStatus`, `AuthorityStatus`, …).
- **Recommendation: C.** Growth is demonstrated only for runtime-built records: `SCHEMA_VERSION` is at 14, `ComponentInspection` gained fields in schemas 11 and 12, and `ValidationCode` gained a code in schema 14. A consumer reads these or decodes them with serde, which `#[non_exhaustive]` does not affect. Marking the authoring records would break custom components' `..ComponentDetails::default()` literals while `Default` already lets them grow. A new field breaks only a literal without that update syntax, and the changelog says so. For status enums, including `EventStatus`'s success/cancelled/dispatched/failed distinction, an exhaustive match turns a new state into a compile error instead of a silently ignored `_` arm (criterion 1). Codex showed that T3 asks for a "closed" statement only on error enums. Extend contracts.md "Public compatibility policy": "Inspection records that only the runtime builds are `#[non_exhaustive]` and grow with `SCHEMA_VERSION`. Records a component builds for `Component::inspect` are built from `Default`. Status and outcome enums are closed unless their rustdoc says they grow." Profile records, `Selector`, `DiagnosticCommand`, `QueryResult`, `Capture` and `CaptureJob` stay as they are (`CaptureJob` is reshaped by G4-26).
- **Consequence:** breaking for extern struct expressions and exhaustive matches of the 12 types only. Call sites outside `pert-ui-runtime/src`:
  - 6 struct expressions, all in `crates/pert-ui/tests/foundation/serialization.rs:479-516` (`ValidationReport`, `ValidationIssue` and `ValidationMarker`, twice each). They become JSON fixtures decoded with serde, which is what they test. No constructor is added, because no consumer builds these records.
  - No exhaustive match on `EventKind` or `ValidationCode`. All 49 uses are `==`, `matches!`, assertions or array elements.
  - Patterns, each excluding `-g '!crates/pert-ui-runtime/src/**'`:
    - `rg -n --type rust '\b(InspectionSnapshot|WindowInspection|ComponentInspection|ParameterInspection|SourceInspection|DragInspection|EventRecord|ValidationReport|ValidationIssue|ValidationMarker)\s*\{'`, with the `fn … -> ComponentInspection {` return types removed by hand;
    - `rg -n --type rust '(EventKind|ValidationCode)::\w+[^,;]*=>'`, which finds nothing.
  - One rustdoc sentence on each of the three authoring records says to build them from `Default`. Slice 6, plus the contracts sentence.
- **Depends on:** G4-4 (no `Eq` sweep that would freeze these records), G3-17 (suffixes for these records), G4-26, G5-26. G5-26 decides where the authoring records are exported. This rule keeps them constructible from `Default` wherever they land, so G5-26 needs no exhaustiveness change.

### G4-10 `AnyValue`/`AnySpec` returned as conversion errors

- **The question:** `ParameterValue::{from_any, spec_from_any}` and the `TryFrom<AnyValue>`/`TryFrom<AnySpec>` impls return the rejected input, which implements neither `Error` nor `Display`. Is that an accepted exception (std's `TryFrom<Vec<T>> for [T; N]`), or do they get a typed mismatch error? (G4-10)
- **Options:** (A) a contracts.md "Error policy" bullet: "A conversion that only checks a value's kind may return its input unchanged as the error, as std's array `TryFrom` does; its rustdoc says so." (B) A typed `FamilyMismatch` error that carries the input and the expected family, implements `Error`, and converts into `RuntimeError::Binding(KindMismatch)`. This changes the sealed trait's signatures and five impls.
- **Recommendation: A.** D1 settled that contracts.md asks only for a typed error, and these errors are typed. They are documented ("returning the input unchanged") and return the value for a retry. The fix is one `map_err`, and the runtime's erased paths already report `BindingError::KindMismatch`. `rg` finds no use of these conversions outside `pert-ui-core` and one internal `map_err` (`parameter_binding/erased.rs:268`), so no consumer was misled. B is affordable, but it serves only the guideline (criterion 2).
- **Consequence:** doc-only, one contracts bullet; the rustdoc already says it. Slice 5.
- **Depends on:** G4-19 (the same impls' `# Errors` form).

### G4-15 Losing `Copy` to carry rejected values

- **The question:** G4-14 makes errors carry the rejected key, value or path. That costs `Copy` on `LayoutError`, `ParameterError`, `GeometryError` and `ChoiceError`. Is that acceptable? (G4-15)
- **Options:** (A) Accept losing `Copy` wherever G4-14 adds a payload. (B) Keep `Copy` and reject G4-14 for these enums. (C) Accept losing `Copy` only where the variant carries an identity the caller cannot easily recover. That means keys (`LayoutError::{DuplicateKey, ReservedKey, UnknownKey}`) and choice identities (`ParameterError::{DuplicateOption, UnknownOption}`). Duplicate values (`ChoiceError::DuplicateValue`) carry positions (`usize`), which keeps `Copy`. Argument-validation variants (`GeometryError::*`, `LayoutError::{InvalidWeight, InvalidRatio}`, `ParameterError::{InvalidRange, OutOfRange, OffLattice}`) stay unit.
- **Recommendation: C.** contracts.md "Error policy" requires `Clone`, `Debug`, `PartialEq` and `Eq` "where their data permits", not `Copy`, so losing it breaks no rule. For the argument variants G4-14 itself rates the benefit low, because "the caller still holds the argument". Carrying an `f32` there would also cost `Eq`, which the policy asks for. `GeometryError` and `ChoiceError` keep `Copy`; `LayoutError` and `ParameterError` lose it. Rustdoc already says the four enums are non-exhaustive, so no closed-set promise is affected.
- **Consequence:** breaking (variant shapes, with or without `Copy`). Matches on the key variants change in about 15 consumer files: `LayoutError::DuplicateKey` 5, `ReservedKey` 3, `UnknownKey` 6, `ChoiceError::DuplicateValue` 8. Copies the compiler flags inside the crates also change. Slice 6, with G4-14.
- **Depends on:** G4-14 (scope of payloads), G1 decisions on choice identity.

### G4-19, G4-20 Forwarded `# Errors` in standard-trait implementations

- **The question:** may `TryFrom`, `FromStr` and serde impls rely on documentation elsewhere instead of carrying their own `# Errors` section? contracts.md says "Every public `Result`". Ten conversion impls carry a one-line forward to the validating constructor (for example `C/parameter.rs:91`, "Validates like [`PowerExponent::new`]"), and G4-18's five carry nothing. The 31 hand-written serde methods carry no doc line at all. Their documentation is the type's `# Serialization` section, which all 14 types have. Two validating derived impls have neither: `PowerExponent`'s `serde(try_from = "f32")` (`C/parameter.rs:61`) and `FrameImage`'s `try_from = "FrameImageRepr"` (`R/capture.rs:315`). (G4-19, G4-20)
- **Options:**
  - (A) A two-part rule in contracts.md "Error policy":
    - "A `TryFrom` or `FromStr` implementation states its failures in one line linking the constructor that documents them."
    - "Serde implementations, derived or hand-written, are documented at the type. A type whose deserialization validates has a `# Serialization` section naming the format and every rejection, and its impls carry no documentation of their own. They do not invent variants for `D::Error`."
  - (B) Write an `# Errors` section on every impl (41 methods).
  - (C) Add a forwarding line to each of the 31 hand-written serde impls as well as the type section.
- **Recommendation: A.** Codex is right that the serde methods have no forwarding lines; the earlier text took the type sections for method documentation. Type-level documentation is still the right place. Callers reach serde impls through a format crate (`serde_json::from_str`), never by naming the method, and rustdoc collapses trait-impl docs under the impl. C would document hand-written impls but cannot reach derived ones. The same trait would then be documented in two places depending on how it is implemented (criterion 3), while the type section covers both. `clippy::missing_errors_doc` does not apply to trait impls. B and C add 41 and 31 lines that repeat the type sections (criterion 4). Evidence: `rg -n -B4 'fn (serialize|deserialize)<' crates/pert-ui-core/src crates/pert-ui-runtime/src` shows no `///` above any of the 31 methods or their impl blocks, and `rg -n '# Serialization'` over the same paths gives the 14 sections.
- **Consequence:** doc-only, all in slice 5:
  - the two-part contracts bullet;
  - G4-18's five forwarding lines;
  - `# Serialization` sections for `PowerExponent` and `FrameImage`, which the rule newly requires.

  No serde method gains a line.
- **Depends on:** G4-10, G4-18.

### G4-21 `DrawError` into `RuntimeError` (F11's second half)

- **The question:** should `DrawError` convert into `RuntimeError` so that a test helper returning `RuntimeError` can write `paint()?`? contracts.md:1131 excludes the conversion. (G4-21)
- **Options:**
  - (A) Keep the exclusion and document the route: a headless helper returns `Result<_, DrawError>`, which already converts from `RuntimeError` and every leaf (`leaf_conversions!`); with `native`, `AppError` works too.
  - (B) Add the conversion. `DrawError::Runtime` already holds a `RuntimeError`, so the reverse needs a new `RuntimeError` variant or an `Application` wrapper. Either gives one failure two forms, against "one normalized representation at each higher level".
- **Recommendation: A.** B contradicts the contract's direction of conversion (criterion 1 favors the stated contract). The friction was two runs at annoyance level, and a doc line removes it. The diagnostics survey's m05 confirms that no attribute can help, so the documented route is the fix. Put it in the `Runtime::paint` rustdoc and in the testing recipe (slice 9's chapter, F2).
- **Consequence:** doc-only, `Runtime::paint` rustdoc plus one guide line. Slice 5; F11's first half is the survey's `#[diagnostic::do_not_recommend]`.
- **Depends on:** G4-22 (the same layering), F2's testing chapter.

### G4-22 `From<Box<dyn Error + Send + Sync>>` for `RuntimeError` (F30)

- **The question:** F30's source loss is resolved, because `ApplicationError: From<Box<…>>` keeps the box. Does `RuntimeError` also get `From<Box<dyn Error + Send + Sync>>`, so that `?` works without `.map_err(ApplicationError::from)`? (G4-22)
- **Options:** (A) Doc only: point `ApplicationError::new` and `RuntimeError::Application` to `ApplicationError::from` for boxed errors. (B) Add the impl and enter it in the leaf list, so `DrawError`, `WorkspaceError`, `WindowError` and `AppError` follow. It must unwrap a boxed `RuntimeError` so a runtime failure keeps one form.
- **Recommendation: A.** F30 was found by the author, not by a tester, and the workaround is one `map_err`. B helps only the `Send + Sync` box. The repository's examples and guide use non-`Send` `Box<dyn Error>`, which neither form can carry. B also adds a downcast-normalization path for the callback-boundary rule, more contract surface than the convenience is worth now.
- **Consequence:** doc-only, two rustdoc pointers; update F30's TODO entry. Slice 5.
- **Depends on:** G4-21.

### G4-23 Capitalized `Display` on user-message carriers

- **The question:** `InputRejection` and `ListingError` display application or OS text verbatim ("Name must not be empty", `io::Error` text), against "Display messages begin in lowercase". Exempt them, or split diagnostic `Display` from user `message()`? (G4-23)
- **Options:** (A) Add to the contracts.md "Error policy" bullet: "An error whose message is text supplied by the application or the operating system (`InputRejection`, `ListingError`) displays it verbatim, as `ApplicationError` does." (B) Make `Display` a lowercase diagnostic and keep the user text in `message()` only.
- **Recommendation: A.** Both types document their message as user-visible text, and the doctest asserts that `to_string()` equals it. Rewriting application or OS prose is ruled out (the finding's fix column). B would silently change what existing `to_string()` callers show users (criterion 1). The transparent `ApplicationError` already sets the precedent.
- **Consequence:** doc-only, one contracts clause. Slice 5.
- **Depends on:** G4-14 (`ListingError` gaining a path does not change this).

### G4-4 (proposal) `Eq` on exact-data records

- **The question:** 21 records derive `PartialEq` without `Eq`, although their data permits it (conditional on `T: Eq` where generic). Adopt `Eq`? (G4-4)
- **Options:** (A) Do not adopt: T1 makes a missing trait a finding only where a consumer plausibly needs it. Record the proposal as closed. (B) Derive `Eq` on all 21, which is additive.
- **Recommendation: A.** No need is shown: tests need only `PartialEq`, and the parameter records are mostly `f32`. Several of the records are runtime-built records that grow with the schema (`SourceInspection`, which G4-8/G4-9 marks `#[non_exhaustive]`, and `ProfileStats` and `EventHistory`), and a derived `Eq` would forbid a future float field without a break. (Author's correction: the draft also named `PresentationStatus`, a status enum that G4-8/G4-9 keeps closed.) Not cheap in commitment, and no consumer cost was demonstrated (criterion 3).
- **Consequence:** none; G4-4 is closed as not adopted, with no TODO entry.
- **Depends on:** G4-8/G4-9.

### G4-30 (proposal) Validated scalars converting back to their raw number

- **The question:** only `PowerExponent` (`From<PowerExponent> for f32`) and `UiZoom` (`From<UiZoom> for f64`) convert back. `NormalizedValue`, `Capacity`, `Length`, `Magnification` and `ViewScale` read only through `get()`, and `PresentationRate` through `hertz()`. Make this uniform? (G4-30)
- **Options:** (A) `get()` is the one reading; record the two impls as exceptions. `PowerExponent`'s impl is needed by its `serde(into = "f32")`. (B) Add `From<Self>` for the raw type to the other six and keep `get()`/`hertz()`, the std `NonZero*` idiom (`get()` plus `From<NonZeroU32> for u32`). (C) Remove the two impls and serialize `PowerExponent` through a private conversion (breaking).
- **Recommendation: B.** It gives one learnable rule matching std ("a validated scalar reads with `get()` and converts into its raw type with `From`"), instead of two unexplained exceptions. It is cheap and local: six additive impls in four files, no call site changes, and no conflict with existing `From` impls on `f32`, `f64` or `usize` (criterion 3). C breaks for symmetry alone.
- **Consequence:** additive, six `From` impls with rustdoc and a mention in the `get()` rule. Slice 6.
- **Depends on:** G1-5, G1-6 (the scope of the `get()` rule).

#### G4 revision notes

- Point 2 (G4-8/9), Codex right. The rule now covers "runtime-built records", not "every type a snapshot contains": 10 structs plus `EventKind` and `ValidationCode`. `InspectValue` moved out as a closed value algebra. Component-authoring records (`ComponentDetails`, `ValidationHints`, `BoxModelInspection`) stay exhaustive and are built from `Default`. `BorderInspection` and the status enums, `EventStatus` included, are closed. Call sites were recounted with patterns: 6 struct expressions in one test file and 0 exhaustive matches. G5-26 was added as a dependency.
- Point 7 (G4-19/20), Codex right. No serde method has a forwarding line. Option A now explicitly makes type-level `# Serialization` documentation sufficient for serde impls, because derived impls cannot carry method docs. Option C (31 per-method lines) was added and rejected. The consequence now adds `# Serialization` sections for `PowerExponent` and `FrameImage`, the two validating derived impls without one.
- Point 8 (G4-6), Codex right. A is now described as a compatibility-policy opt-out that permits a later break rather than preventing it. The recommendation changed to B, a `!Send` marker now: it gives one compiler-enforced rule, removes `Svg`'s dependency on `usvg::Tree`'s auto traits, and costs no call sites while unpublished. Adding `Send` later stays additive. The consequence moved from doc-only (slice 5) to breaking in auto traits (slice 6), with the `api/` diff and `compile_fail` doctests.

## G5

Draft recommendations for the maintainer. Sources: `findings.md` G5 rows and
the index, `reports/{claude,codex}-G5.md`, `reports/verification/*-G5.md`,
the prelude rustdoc (prefixes as in findings.md: `R/` = `crates/pert-ui-runtime/src/`,
`C/` = `crates/pert-ui-core/src/`, `F/` = `crates/pert-ui/src/`; `crates/pert-ui-runtime/src/lib.rs:294-327`,
`crates/pert-ui/src/lib.rs:46-63`, `crates/pert-ui/src/root.md:39-46`) and
app-author slice 9 (`plans/app-author-improvements.md:351-`). The prelude
rule exists only in that rustdoc; contracts.md and AGENTS.md say nothing
about the prelude, so every sentence below goes there. Every option here is
additive or doc-only. No prelude name is removed, and no consumer file
defines a local type with any of the names discussed, so a new glob
ambiguity is not expected (checked with `rg` over examples, gallery, guide
and tests). Slice 9 plans "a smaller application prelude and thematic
modules". The rules below decide what any prelude or component-authoring
module must hold, whatever shape slice 9 chooses.

The five policy questions are also all of G5's consistency proposals
(G5-11, G5-16, G5-26/27 with G5-14, G5-30, G5-32), so each heading below
covers both.

### G5-11 `SeriesSource` and the live widgets

- **The question:** both `LevelMeter::new` and `Trace::new` take `SeriesSource<f32>`
  (`R/live_widgets.rs:148`, `:493`). G5-10 adds the widgets and `MeterAxis`.
  Does `SeriesSource` follow them into the prelude, or does the live-data
  exclusion (`R/lib.rs:320-322`) keep it at the root?
- **Options:** (A) keep it at the root, and add this to the exclusion sentence:
  "A live widget's `SeriesSource`, and the bridge that feeds it, stay
  with the live-data plumbing; the widgets and their builder types are in the
  prelude." (B) Add `SeriesSource` to the runtime prelude.
- **Recommendation: A.** A `SeriesSource` comes from `Runtime::series_source(consumer, delivery)`, fed by `series_bridge`,
  `SeriesConfig`, `Capacity` and `Delivery`, all of them root plumbing. A consumer who builds one
  therefore already imports from the root, and adding the one name removes no import line. The
  mixer probe's gap covers the whole family (`series_bridge`, `SeriesConfig`, `SeriesSource`,
  `Delivery`, `Timestamp`), so moving one name only makes the boundary harder to learn. A
  thematic live-data module in slice 9 would serve the family better (criterion 3). No contract
  is violated (criterion 1).
- **Consequence:** doc-only. One sentence in the runtime prelude rustdoc, and
  the facade's mirror sentence already says "live-data … stay at the crate
  root". No call sites change. Slice 5, or slice 9 together with G5-10.
- **Depends on:** G5-10 (the widgets join the prelude). G5-26: `UpdateContext::read_series` takes
  `&SeriesSource`, so the closure must let the live-data exclusion win, which the rule under
  G5-26 does. G5-32 (thematic modules).
- **Resolved by layering part L2 (0.3.73):** the thematic live-data module exists.
  `pert_ui::live` holds `SeriesSource`, the series bridge, the mailboxes and the live
  widgets, which leave the prelude (`plans/usability/layering-design-L2.md`, section 2.3).

### G5-16 `ParameterHost`'s sink and monitor types: "hosts" or "host integration"

- **The question:** are the types that the prelude's `ParameterHost::on_outcome`,
  `on_preview`, `retry_outcome` and `resolve_delivery` take and return
  "hosts" (`R/lib.rs:304-305`) or "host-integration types" (`:320-322`)?
  The types are `DeliveryMonitor`, `DeliverySnapshot`, `TerminalEvent`, `PreviewEvent`,
  `SinkError`, `SinkErrorKind` and `DeliveryResolution` (`R/parameter_binding/delivery.rs:500-640`).
- **Options:** (A) classify them as host integration, keep them at the root, and say so in the
  exclusion sentence. (B) Classify them as part of "hosts" and add the seven names. Reword
  the rule to "hosts, with the types their sink and recovery methods take
  and return; host integration means transport endpoints (`Mailbox`, `UiSender`,
  the series bridge)".
- **Recommendation: B.**
  - `ParameterHost` is already a member. Its terminal sink is its primary use: the guide
    (`guide/state.md:157`, `:211`) and its rustdoc example call `on_outcome`.
  - A host whose delivery can fail has to construct `SinkError::new(SinkErrorKind::…)`. Both
    packaged examples that do so import those names from the root (`live_mixer.rs:26-28`,
    `:82-88`; `dynamic_documents.rs:70-73`). `live_mixer` also stores
    `DeliveryMonitor` in fields (`:164`, `:167`).
  - Under A, one member's API is split across two import sites.
  - `SinkError` is an error an application constructs to report its own failure, exactly as
    `ListingError` and `InputRejection` are, and those are already prelude members "for
    configuration". This gives one learnable rule rather than another exception (criterion 3).
  - Neither authority nor timing changes.
- **Consequence:** additive. Seven names in the runtime prelude, and the "hosts" and
  error sentences reworded. No call site has to change (live_mixer's root import list could
  shrink). Also tests, about 4 facade test files for `SinkError`. Slice 9, or 6 if
  chosen.
- **Depends on:** G5-15 (`DeliveryStatus` and `AuthorityStatus`: by the same reasoning,
  `RemoteHost`'s status values join, while `RemoteHost` itself needs the same classification,
  which no row records). G5-26 (the rule there yields the same answer for
  `DeliveryResolution`, which is constructed, and `SinkError`). G5-32 (`ApplicationError`
  follows the same constructed-error sentence).

### G5-26, G5-27 (with G5-14) How far "every type its signatures name" reaches, and where it lives

- **The question:** does the closure include field, variant-payload,
  constructor-parameter, one-hop and kept types? Does the "a custom component needs
  no other import" guarantee (stated three times: `R/lib.rs:324-326`,
  `F/lib.rs:53-55`, `root.md:39-40`) override the inspection exclusion? Does the
  result belong in the application prelude, which slice 9 plans to shrink
  (`plans/app-author-improvements.md:358-360`), or in a component-authoring module?
  G5-27 adds the one-hop types of `EventContext`, `MeasureContext` and `ChildInfo`;
  G5-14 adds `EditSource`.
- **Options:**
  - (A) Direct only: narrow the three sentences to "a component that lays out, draws and
    handles input", and list the root names a self-describing, dragging or detaching
    component adds. Doc-only.
  - (B) A mechanical one-hop closure. It is transitive in effect (`VectorStyle`), takes
    `ProfileSnapshot` against the profiling exclusion and works against slice 9.
  - (C) The rule below, added to the application prelude.
  - (D) The same rule, its result in a new re-export module `pert_ui::component` (and
    `pert_ui_runtime::component`): `Component`, the prelude's component names and the
    rest. The prelude keeps today's members; its three sentences point to the module.
- **Rule (C and D):**
  - A type is a member when using an anchor requires writing its name: an argument the
    consumer constructs; an enum it matches, including a kept receipt's status; a field or
    payload it builds or destructures for a `Component` method; or a value it keeps past
    the returning call (field, signal, collection) when the rustdoc says to keep it. That
    is the receipts (`R/runtime/tasks.rs:27`, `R/runtime/reveal.rs:131`,
    `R/runtime/native_windows.rs:946`) and `EditorAttachment`, through which the editor
    reports the end of the edit (`R/editor_session.rs:215-219`, `:245`).
  - A value used only in the returning call (passed on, compared, converted) is not
    required.
  - The excluded families still win: profiling, capture, inspection *queries and records*,
    live-data plumbing, host transport and other errors. The inspection *description*
    that `Component::inspect` returns is not excluded.
- **Recommendation: D.**
  - The guarantee is false today: every self-describing custom component in the corpus
    imports the description types from the root (`diagram_editor` `canvas.rs:866-879`,
    r1-3-claude, r1-3-codex, the custom-widgets guide at `:124` and `:162`), and rustc
    offers no import help (G5-6). Only membership or a pointer helps (criteria 1 and 3).
  - Codex is right that receiving a value does not end the need to name it. The
    consumer-defined editor stores `attachment: EditorAttachment`
    (`crates/pert-ui/tests/editing/editor_sessions.rs:606`), and the gallery keeps
    `PresentationReceipt` and `RevealReceipt` in signals (`apps/gallery/src/gallery.rs:112`,
    `multi_window.rs:66`, `pages/layout/navigation.rs:282`). The draft's "received values
    stay out" left the guarantee false; the kept-value clause covers them.
  - C and D yield the same names. C puts them into the prelude slice 9 means to shrink,
    where a later removal breaks consumers. D adds them once, to a thematic module of the
    kind slice 9 plans (criteria 3 and 4). The rule a consumer learns: "applications glob
    the prelude; implementing `Component` also globs `pert_ui::component`, which holds
    every name you have to write". Two globs re-exporting one item do not conflict.
- **Results under C and D:**
  - **In:** `Semantics`, `SemanticRole`, `SemanticValue`, `SemanticAction`, `InspectValue`,
    `ValidationHints`, `OverlapPolicy`, `BoxModelInspection`, `BorderInspection`,
    `SemanticItem`, `ShortcutBinding`,
    `ShortcutScope`, `DragPreview`, `DRAG_THRESHOLD`, `NativeWindowOptions`,
    `NativeWindowId`, `Transform2D`, `PathElement`, `EditorTrigger`, `EditorOutcome` and
    `TaskStatus`; kept: `EditorAttachment`, `TaskReceipt`, `RevealReceipt`,
    `PresentationReceipt` and its `PresentationStatus` (`RevealStatus` is in the prelude).
    26 names beyond the prelude; the module also re-exports G5-21's `EditorSession` and
    `EditorStart`. (Author's correction after Codex's recheck: `BorderInspection`, which a
    component builds for `BoxModelInspection::border`, and `SemanticItem`, which it builds
    for `Semantics::items`, were missing.)
  - **In, application prelude:** `IgnoredEntry`, as G5-23 records. Codex is right: a
    consumer matches it from the prelude's `WorkspaceReport::ignored` (`R/workspace.rs:763`,
    `crates/pert-ui/tests/surfaces/workspace.rs:345`), so "not required" contradicted the
    rule.
  - **Not required:** `FittedText` (`into_layout` gives the kept `TextLayout`, in through
    G5-18; `R/tool_windows/stripe.rs:390`), `EditorCompletion` (read by field) and
    `VectorStyle` (only a test destructuring a display list reaches it).
  - **Out:** `ProfileSnapshot` (profiling). G5-14's `EditSource`, a newtype (`C/edit.rs:59`)
    a runtime consumer only receives and compares (`Parameter::begin_edit` takes
    `Revision`), stays at the root.
- **Consequence:** additive. One module per crate, `IgnoredEntry` in the prelude, the three
  sentences rewritten. No call site must change. Root imports that could switch to the
  glob, by the pattern below (type and constant names, so the word boundary is the
  receiver check; no hit is a same-named local type):
  `rg -n '\b<Name>\b' apps crates/pert-ui/examples crates/pert-ui/tests crates/pert-ui-runtime/tests crates/pert-ui/src/guide`:
  `EditorAttachment` 2 lines in 2 files, `PresentationReceipt` 6 in 4, `RevealReceipt` 5 in
  2, `TaskReceipt` 0, `IgnoredEntry` 3 in 1; the largest, `InspectValue`, 744 in 104 (87
  facade test files). Slice 9's API-layout branch, or 6 if chosen; slice 5 narrows G5-25's
  sentences as in A meanwhile.
- **Depends on:** G5-25 (resolved here); G5-21 and G5-23 (`EditorSession`, `EditorStart`,
  `IgnoredEntry`); G5-12, G5-17 to G5-24 (direct members unaffected); G5-11 (the exclusion
  wins); G5-16 and G5-30 (the same "writes" test for hosts and `App`); G5-28 (not subsumed
  under D: `App::on_close` still needs `NativeWindowId` in the facade prelude); slice 9 (it
  names the module and decides, on its breaking branch, whether component-only names leave
  the prelude).
- **Superseded in part:** the application-prelude rule is superseded by layering part L2
  (0.3.73, `plans/usability/layering-design-L2.md`); the `component` rule stays.

### G5-30 `WindowError` in the facade prelude

- **The question:** `App::on_window_error` reports `&WindowError` (`F/app.rs:480`). Is it a member under
  "`App` and the types it reports" (`F/lib.rs:50-51`), or does "Other errors … stay at the crate root"
  (`:58-60`) keep it out?
- **Options:** (A) keep it at the root, and add "Errors `App` reports to a handler stay at the root;
  the handler's parameter type is inferred." (B) Add `WindowError` to the facade prelude under
  `native`, and name it in the error sentence beside `AppError` and `AppShutdownError`.
- **Recommendation: B.** The explicit clause already overrides the error exclusion for
  `AppShutdownError`, which is a member as the payload of `AppError::Shutdown`. Treating
  `WindowError` the same way gives one rule: "`App` and everything it reports, errors included".
  Under A, a second exception would sit inside the same sentence. A handler that tells failures
  apart matches on its variants and so writes the name, which is G5-26's criterion. Cost: one
  re-export line. Its users (about 5 facade test files and 1 runtime test file) keep
  working, and no example uses it.
- **Consequence:** additive. One name in `F/lib.rs:71` under `native`, and the error sentence
  reworded together with G5-29 and G5-31's wording fixes. Slice 9, or 5/6 with G5-28.
- **Depends on:** G5-28 (`NativeWindowId`, same clause), G5-29 and G5-31 (same sentences),
  G5-26 (the "writes" criterion).

### G5-32 A testing-oriented prelude or module

- **The question:** some root names are imported by most applications and tests
  while the rule keeps them at the root:
  - `CaptureRequest`, `CaptureTarget` and `cpu::CpuRenderer`, in six round 1
    submissions;
  - the semantics types;
  - `ApplicationError`, in every probe's `main.rs`;
  - `Mailbox`, `UiSender` and `Capacity`.

  Does slice 9 add a testing-oriented module or prelude? No rule is
  departed from.
- **Options:**
  - (A) No new module on import grounds. The prelude page lists the root names
    applications most often add (capture, the CPU renderer, inspection queries,
    `Mailbox` and `UiSender`), and slice 9's testing chapter (F2) opens with its exact
    `use` block.
  - (B) A `pert_ui::testing` module re-exporting the capture, CPU and inspection-query names.
  - (C) Put them in the main prelude, against the stated exclusions and slice 9's smaller
    prelude.
- **Recommendation: A for the testing names; `ApplicationError` joins the prelude
  (additive, one name).**
  - No tester recorded friction from these imports. claude-D2-8 records use,
    not cost, and Codex corrected the count to six.
  - F2's real cost is the missing helpers (click at bounds, replace text, key chord, find
    semantics, settle).
  - AGENTS.md: "Add abstractions for demonstrated responsibilities". A module is justified
    when it owns those helpers (FU or slice 9), not by re-exports alone. The capture names can
    be re-exported there then.
  - The semantics types are settled by G5-26 (its component module).
  - `ApplicationError` (`R/error.rs:46`, built with `new` or `From<String>`/`From<&str>`)
    is what an application constructs to wrap its own failure for `?`. It joins the
    prelude under G5-16's constructed-error sentence. It is not a testing name.
- **Consequence:** additive, one name (`ApplicationError` in the runtime prelude), with
  its error sentence; the rest is doc-only (the prelude page, slice 9's testing chapter).
  No new module now. No call site must change. By
  `rg -n '\bApplicationError\b' apps crates/pert-ui/examples crates/pert-ui/tests crates/pert-ui-runtime/tests crates/pert-ui/src/guide`
  (every hit is the type itself: `ApplicationError::new`, `::from`, a `use` line or one
  return type, `crates/pert-ui-runtime/tests/errors.rs:20`; no method is so named, so the
  word boundary is the receiver check): 45 lines in 28 files. The examples have 6 in 5
  files (`main.rs` of `diagram_editor`, `inventory` and `log_explorer`, `inventory/app.rs:6`
  and `:40`, `live_mixer.rs:788`), four of them path-qualified; gallery and tests 39 in 23;
  the guide none. Only root `use` lines could shrink. Slice 9, or 6 with G5-16.
- **Depends on:** G5-16 (the constructed-error sentence), G5-26 (semantics types), G5-11 (the
  live-data family in a thematic module), F2 (testing helpers).

#### G5 revision notes

- Point 4 (G5-26): Codex is right on all three counts (`editor_sessions.rs:606` stores `EditorAttachment`; the gallery keeps `PresentationReceipt`/`RevealReceipt` in signals; `IgnoredEntry` is matched, as G5-23 records). The rule gains a kept-value clause; `EditorAttachment`, the three receipts and `PresentationStatus` move to In, `IgnoredEntry` to the application prelude. A new option D puts the rule's result in a `component` module instead of the prelude, per slice 9 (`plans/app-author-improvements.md:358-360`), and the recommendation changes from C to D; G5-28 is no longer subsumed. Summary row: "D: names a component must write, kept receipts included, in a new `component` module; `IgnoredEntry` joins the prelude | additive (new module), prelude rustdoc rewritten".
- Point 10 (G5-32): Codex is right; the body added `ApplicationError` while the summary said doc-only. The recommendation heading and consequence now state the additive export, with a recount (45 lines in 28 files, 6 in 5 example files; pattern given). Summary row: "A: no testing module until it owns F2's helpers, root pointers instead; `ApplicationError` joins the prelude | doc-only, plus 1 additive name".
