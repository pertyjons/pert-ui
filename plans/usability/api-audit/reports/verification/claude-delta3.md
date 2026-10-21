# claude delta review: inventory version 3

Commit read: 245053b (library source unchanged since d4ba108). Read first:
rubric C4, N3, T4, D2; reports claude/codex G1, G3, G4, G5 and their
verifications.

Prefixes: `C/` = crates/pert-ui-core/src/, `R/` = crates/pert-ui-runtime/src/,
`W/` = crates/pert-ui-wgpu/src/, `F/` = crates/pert-ui/src/. Rendered
rustdoc was checked in target/doc (built 2026-10-21 from the unchanged
source) where a finding depends on what a reader sees.

## 1. C4: candidates/constants.txt

**Entries processed:** all 93 rows (core 9, runtime 82, wgpu 1, facade 1).
Every limit row was read at its definition and at every enforcement site
(`rg` over crates/ for each constant name, outside tests). Classification:

- **Limits owned by the type they bound and enforced as documented (no
  finding):** `Capacity::MAX` (C/live/mod.rs:186-202),
  `NormalizedValue::{MIN, MAX}`, `UiZoom::{MIN, MAX}`
  (R/runtime/settings.rs:29-45), `ViewScale::{MIN, MAX}`
  (R/transform.rs:275-279), `ParameterState::MAX_TERMINAL_RECEIPTS`
  (C/edit.rs:300-307, enforced :384), `FontData::MAX_BYTES`,
  `FontManager::{MAX_CUSTOM_FONTS, MAX_TOTAL_BYTES}` (enforced together at
  R/text/registry.rs:53-55), `FrameImage::MAX_BYTES` (R/capture.rs:352-370;
  also CPU lib.rs:184, W/lib.rs:678, F/platform/presenter.rs:250),
  `MaskImage::MAX_PIXELS` (R/drawing.rs:1135-1146), `RasterImage::MAX_BYTES`
  (R/resources.rs:155, :170), `Path::MAX_POINTS` (R/geometry.rs:210-212,
  enforced :402), `RichText::MAX_SEGMENTS`, `UiStrings::MAX_LENGTH`
  (R/strings.rs:576, :606), `ToastQueue::{MAX_VISIBLE, MAX_PENDING}`
  (R/notifications.rs:281-285, :339), `Workspace::MAX_ENTRIES`
  (R/workspace.rs:502, :523, :906), every `Runtime::MAX_*` queue or wave
  bound (tasks.rs:84, reveal.rs:271, commands.rs:1757,
  native_windows.rs:1101, capture.rs:88, application.rs:221, :311,
  runtime.rs:3575, :3872-3878), and the retention bounds of the reporting
  types (`InspectionSnapshot::MAX_EVENTS`, diagnostics.rs:932;
  `EventStatus::MAX_MESSAGE_CHARS`, :940; `ProfileSnapshot::{MAX_STATS,
  MAX_RECENT, MAX_NESTING}` and `ProfileStats::MAX_KEY_CHARS`,
  profiling.rs:216-270). The retention constants sit on the snapshot type
  whose fields they bound; I do not count that as "the wrong type".
- **Already recorded:** `PowerExponent::{MIN, MAX}: f32` (claude-C4-3,
  disputed in codex's verification), `NumberInputError::MAX_PRECISION`,
  `TextEditError::MAX_TEXT_BYTES`, `TextError::MAX_FONT_PIXELS`,
  `Workspace::{MAX_PATH_BYTES, MAX_STRING_BYTES}` (claude-C4-3,
  codex-C4-3), `Workspace::MAX_DOCUMENT_BYTES` (codex-C4-1 for
  enforcement), `VectorRasterizer::MAX_COVERAGE_WORK` (claude-C4-2 for its
  missing siblings). New aspects of some of these are below.
- **Defaults, not limits (no finding under C4):** `ClickCounter::DEFAULT_*`
  (typed `Duration`/`Length`), `ContentQuery::DEFAULT_{LIMIT, BUDGET}`,
  `NavigationHistory::DEFAULT_CAPACITY` (no preallocation, so the
  unbounded `NonZeroUsize` of `with_capacity` is harmless,
  navigation_history.rs:252-270), `ScaleLimits::DEFAULT` (inside
  `ViewScale::MIN..=MAX`), `VirtualList::DEFAULT_OVERSCAN`,
  `QuickPick::{DEFAULT_WIDTH, VISIBLE_ROWS}` and the `CommandPalette`
  aliases (bare `f32` already in claude-N3-d1).
- **Identities, named values and tables (not limits):** `Incarnation::FIRST`,
  `Sequence::FIRST`, `Revision::INITIAL`, the eleven `Color` constants,
  `DropOperations::*`, `Modifiers::PRIMARY`, `Point::ZERO`, `Size::ZERO`,
  `SplitSizing::*`, `Track::CONTENT`, `Transform2D::IDENTITY`,
  `ViewTransform::IDENTITY`, `ViewScale::ONE`, `Icon::ALL`,
  `StringRole::ALL`, `ImageFormat::ALL`, both `SCHEMA_VERSION`s.
- **Remaining limit rows:** `TableColumn::MIN_WIDTH`,
  `TextLimit::MAX_RASTER_SIDE`, `Renderer::READBACK_TIMEOUT` give the
  findings below.

### claude-C4-v3-1 The fixed-precision number format bypasses `MAX_PRECISION`

- **Rule kind:** C4, project rule (AGENTS.md "Reject invalid external
  values"; the limit's own rustdoc).
- **Items and configurations:** `pert_ui_runtime::NumberFormat::<f64>::localized_decimal`
  and `NumberLocale::format_decimal`, relative to
  `NumberInputError::MAX_PRECISION`; all configurations.
- **Observation (behavior):** `MAX_PRECISION` is documented as "The most
  fractional digits a fixed decimal display precision may have: 15"
  (R/text_fields/number_inputs.rs:42-48). `decimal_places` enforces it on
  the number inputs (:2432-2435) and the numeric controls
  (R/numeric_controls.rs:1328-1331). The library's other fixed-precision
  path does not: `NumberLocale::format_decimal(self, value: f64, places:
  Option<u8>) -> String` formats with `format!("{value:.*}", places)` for
  any `u8` (R/number_format.rs:251-259), and
  `NumberFormat::localized_decimal(locale, places: Option<u8>, width_hint)`
  (:387-398) passes `places` through unchecked. The result is accepted by
  `DecimalInput`/`InlineDecimal::format(NumberFormat<f64>)`
  (number_inputs.rs:2028), so `localized_decimal(locale, Some(40), ..)` on a
  `DecimalInput` shows 40 fractional digits (binary expansion digits past
  the 17th, e.g. `0,1000000000000000055511151231257827021182`) where
  `decimal_places(40)` returns `InvalidPrecision`.
- **Rule:** the limit's rustdoc states a general bound on fixed decimal
  display precision; C4 "out-of-range inputs are rejected, not clamped"
  (here: neither).
- **Severity:** low. **Confidence:** medium: `NumberFormat::new` accepts
  arbitrary closures, so no limit can cover every custom format; but
  `localized_decimal` is the library's own fixed-precision constructor and
  takes the same quantity as `decimal_places`.
- **Fix:** either validate `places` in `localized_decimal` (returning
  `NumberInputError::InvalidPrecision`) and in `format_decimal`, or narrow
  the constant's rustdoc to `decimal_places` and say what
  `localized_decimal` does above 15. A shared validated
  "fractional digits" type would serve both. Breaking for the fallible
  signature; non-breaking for the documentation route.
- **Destination:** TODO.md.

### claude-C4-v3-2 `TableColumn::MIN_WIDTH` clamps explicit and restored widths

- **Rule kind:** C4; project rule for the restore path (AGENTS.md
  "Version persisted schemas … Validate closed schemas, required fields,
  identities and numeric ranges … never reinterpret old data silently");
  **policy question** for the two setters, as for claude-C4-5.
- **Items and configurations:** `pert_ui_runtime::TableColumn::width`,
  `TableController::set_column_width`, `Table`'s
  `Component::restore_workspace_state`; all configurations.
- **Observation (behavior):** the constant is owned by the right type: the
  private `minimum` field is always `MIN_WIDTH` (R/table.rs:43, :88), so the
  constant is the effective limit. It is applied by clamping, not
  rejection, on three input paths:
  - `TableColumn::width(width: Length)` stores `width.get().max(self.minimum)`
    (table.rs:95-99, documented "never below `MIN_WIDTH`");
  - `TableController::set_column_width(key, width: Length)` does the same
    (table.rs:765-816, documented);
  - `restore_workspace_state` rejects a stored width `<= 0.0` as
    `RestoreRejection::OutOfRange` (table.rs:2028-2030) but accepts any value
    in `(0, 40)` and raises it to 40 (:2075). A capture never writes such a
    value (`fixed_width` is never below the floor, :163-171), so it can come
    only from a foreign or edited document, which is then reinterpreted
    silently while a value of 0 is refused.
- **Rule:** AGENTS.md "clamp only for documented control-limit behavior";
  for the persisted path, the numeric-range validation rule quoted above.
  A user dragging an edge to the floor is control-limit clamping; an
  explicit application width or a stored value is an input (the same
  distinction as claude-C4-1).
- **Severity:** low (a column ends up wider than requested).
  **Confidence:** high on the behavior.
- **Fix:** reject restored widths below `MIN_WIDTH` with `OutOfRange`
  (behavior change only, no signature change); decide the setters with
  claude-C4-5's policy question (a layout floor as an allowed documented
  clamp, or fallible setters).
- **Destination:** TODO.md for the restore path; the checkpoint for the
  setters.

### claude-C4-v3-3 Three documented limits whose value a reader cannot see

- **Rule kind:** C4, project rule (AGENTS.md "Document purpose, defaults,
  valid values/units …" and "Numbers that are part of a contract, such as
  documented capacities, limits … stay and must be exact").
- **Items and configurations:** `pert_ui_runtime::Workspace::MAX_DOCUMENT_BYTES`
  (all), `pert_ui_runtime::VectorRasterizer::MAX_COVERAGE_WORK` (all),
  `pert_ui_wgpu::Renderer::READBACK_TIMEOUT` (wgpu default).
- **Observation (signature and rustdoc):** the three are computed
  constants (`1 << 20`, R/workspace.rs:507-508; `1 << 24`,
  R/vector_scene.rs:116-126; `Duration::from_secs(5)`, W/lib.rs:357-365).
  Rustdoc renders computed associated constants without a value: the
  generated pages read `pub const MAX_DOCUMENT_BYTES: usize`,
  `pub const MAX_COVERAGE_WORK: usize`, `pub const READBACK_TIMEOUT:
  Duration` (target/doc struct.Workspace.html, struct.VectorRasterizer.html,
  pert_ui_wgpu struct.Renderer.html), and none of the three rustdoc texts
  states the number ("The largest encoded document, in bytes."; "How long
  … waits for the GPU in total"). Literal siblings render theirs
  (`MAX_ENTRIES: usize = 4096`), and every other computed limit states it
  in prose (`FrameImage::MAX_BYTES` "64 MiB", `Capacity::MAX` "1,048,576
  items", `FontData::MAX_BYTES` "16 MiB", `ClickCounter::DEFAULT_INTERVAL`
  "500 ms").
- **Rule:** as quoted. A consumer sizing a workspace document or a capture
  wait has to read the source.
- **Severity:** low. **Confidence:** high (checked in the rendered HTML).
- **Fix:** state "1 MiB (1,048,576 bytes)", "16,777,216 steps" and "five
  seconds" in the rustdoc. Not breaking.
- **Destination:** TODO.md.

### Already recorded findings these entries extend

- **claude-C4-3** (limits on another type than the one they bound),
  extended by `TextLimit::MAX_RASTER_SIDE: u32 = 8192` (R/text.rs:104-107),
  a limit of a prepared text placed on the error-detail enum that reports
  it. With it, the limits of one operation, text preparation, live on four
  owners: `TextEditError::MAX_TEXT_BYTES` (an error, with a stated reason,
  text_input.rs:29-41), `TextError::MAX_FONT_PIXELS` (an error),
  `TextLimit::MAX_RASTER_SIDE` (an error detail) and
  `RichText::MAX_SEGMENTS` (the bounded type); the fourth,
  `TextLimit::Coverage`, has no text constant and borrows
  `MaskImage::MAX_PIXELS` (text.rs:24, :101-102, :114). The policy question
  of claude-C4-3 should cover these together.
- **claude-C4-2** is unchanged: no constants row supplies the missing
  `PresentationRate`, `ViewState` or rasterizer-budget constants.
- **codex-C4-1** is unchanged; `MAX_DOCUMENT_BYTES`'s missing value is the
  separate documentation point of claude-C4-v3-3.

## 2. N3: `Option<number>` rows of candidates/numeric.txt

**Entries processed:** all 31 rows whose parameters or return carry
`Option<number>` (29 added in version 3, 2 re-rendered: `AxisMapping::tick_step`,
`NumberLocale::format_decimal`), each read at its definition; the 9 older
rows that mention `Option` only through a non-numeric type (`Sequence::new`,
the `*Items::key/text/build/cell` index rows) were rechecked in passing.
Cross-check: every `pub fn` with an `Option<integer|float>` in the committed
api/ listings is in the list, except the scoring closures of
`CommandMatcher::new`/`PickMatcher::new` (unitless scores, no finding).

Classification:

- **Unitless counts, indices, scores, sequence numbers or
  application-defined values (no finding):** `ParameterSpec::index_of`,
  `ClickCounter::press_event` (click count), `CommandMatcher::{score,
  fuzzy_score}`, `PickMatcher::{score, fuzzy_score}`,
  `InspectionQuery::{limit_value, events_after_value}`,
  `ListDropPoint::index`, `ListItems::index_of`, `TableItems::index_of`,
  `TableColumn::flex_weight` (a weight), `WorkspaceValue::{as_integer,
  as_number}`, `NumberFormat::localized_decimal` and
  `NumberLocale::format_decimal` (`places` names its unit; see
  claude-C4-v3-1 for its range), `AxisMapping::tick_step` (`min_pixels`
  named; the return is in the application's domain unit, as claude-N3-3
  records).
- **Already recorded in claude-N3-d1:** `Painter::{text, rich_text}`,
  `MeasureContext::{text, rich_text}`, `EventContext::text` (`width:
  Option<f32>`), `TableColumn::fixed_width`, `ToolWindowChange::with_geometry`.
- **New findings:** `TextField`/`TextArea::{min_length, max_length}` and
  `Constraints::{new, width, height}`.

### claude-N3-v3-1 The editors' `min_length`/`max_length` count grapheme clusters under a name that means bytes elsewhere

- **Rule kind:** N3, project rule (AGENTS.md "Give domain quantities …
  appropriate validated types"; N3 "says so, through its type … or its
  name"); N2 consistency proposal for "characters".
- **Items and configurations:** `pert_ui_runtime::TextField::{min_length,
  max_length}`, `TextArea::{min_length, max_length}` (macro body
  R/text_fields.rs:433-461, instantiated :768-769); all configurations.
- **Observation (signature):** `max_length(self, count: Option<usize>)`
  refuses "more than `count` grapheme clusters" (text_fields.rs:450); the
  unit is in rustdoc only. In the same library "length" in a name means
  bytes: `UiStrings::MAX_LENGTH` is "in bytes" (R/strings.rs:576-577), as is
  std's `str::len`, and the same editors are also bounded by
  `TextEditError::MAX_TEXT_BYTES`, a byte count (R/text_input.rs:29-41). The
  rejection messages call the unit "characters"
  (`StringRole::AtLeastCharacters`/`AtMostCharacters`, "Character count must
  be at most {count}", R/strings.rs:186-189, :370-374), while the library's
  `_CHARS` constants count Unicode scalar values
  (`EventStatus::MAX_MESSAGE_CHARS` "Unicode characters", diagnostics.rs:807-810,
  enforced with `chars().take`, :940; `ProfileStats::MAX_KEY_CHARS`,
  profiling.rs:163-164, :227). Three counts of text, three spellings, and
  the one the editors use is named by neither.
- **Rule:** N3 (the unit through type or name). claude-N3-d2 records the
  conflict from the `UiStrings` side and renames only the constant; with
  that fix alone, `max_length` would still not name grapheme clusters and
  "length" would still read as std's byte length.
- **Severity:** low (a host mirroring a server-side "at most 100
  characters" rule gets grapheme counts; combining marks and emoji
  sequences count once). **Confidence:** high.
- **Fix:** `min_graphemes`/`max_graphemes` (breaking), or at least the
  parameter name `graphemes` (not breaking, rustdoc and IDE hints only);
  resolve together with claude-N3-d2, and use one word for grapheme counts
  in `StringRole` names.
- **Destination:** TODO.md, with claude-N3-d2.

### claude-N3-v3-2 `Constraints` states no unit anywhere

- **Rule kind:** N3, project rule.
- **Items and configurations:** `pert_ui_runtime::Constraints::{new, width,
  height}`; all configurations (prelude type; `Component::measure` receives
  it).
- **Observation (signature and rustdoc):** `Constraints::new(width:
  Option<f32>, height: Option<f32>) -> Result<Self, GeometryError>` and
  `width()`/`height() -> Option<f32>` (R/layout.rs:257-289). Neither the
  type's rustdoc ("Available measurement space, with an explicit unbounded
  value on each axis", :164-167) nor any method's says "logical pixels". Its
  sibling `Size` does ("Finite nonnegative width and height in logical
  pixels", :117), and `Constraints::bounded(Size)` (:271-277) implies the
  same unit. The exemption both auditors apply to raw constructors and
  accessors of unit-bearing types (`Size`, `Length`, `Point`; codex-N3-1
  "types whose purpose establishes the unit") presupposes that the type
  states its unit; this one does not.
- **Rule:** N3.
- **Severity:** low (logical pixels are the documented default space,
  docs/rendering-platform.md:9; every custom component reads these values
  in `measure`). **Confidence:** high.
- **Fix:** say "in logical pixels" in the type rustdoc, which brings it
  under the `Size` precedent (not breaking); or `Option<Length>` (breaking),
  with claude-N3-2's convention decision.
- **Destination:** TODO.md, with claude-N3-2.

### Already recorded findings these entries extend

- **codex-N3-2** (text byte offsets named only in rustdoc), extended by
  byte ranges that no list carries because the number sits inside
  `Range`: `TextSelection::range(self) -> Range<usize>` ("Ordered byte
  range", R/text_selection.rs:57-58), `TextPreedit::new(text, cursor:
  Option<Range<usize>>)` and `TextPreedit::cursor()` ("a UTF-8 byte
  range", R/text_input.rs:46, :95), and `RichText::highlighted(text, ranges:
  impl IntoIterator<Item = Range<usize>>)` (R/text/fit.rs:254). My
  verification's severity remark applies (low: std's byte-index
  convention).
- **claude-N3-d1** is confirmed for its five rows above; nothing new.
- **Inventory note:** numbers inside `Range` and arrays (`Color::channels`,
  `Color::to_srgb`, `Rect::values`, `Transform2D::entries`,
  `QuarterTurn::corners`) are still in no N3 list unless a bare number also
  appears. Apart from the ranges above, these are whole-representation
  accessors of unit-bearing types (discussed under codex-C4-2) and hide no
  N3 finding.

## 3. T4: fallible conversions into foreign types (candidates/errors.txt)

**Entries processed:** errors.txt version 3 adds **3** rows, not 8:
`<f32 as TryFrom<AnyValue>>::try_from`, `<i64 as …>`, `<bool as …>`
(errors.txt:49-51, generated by `value_conversions!` at C/erased.rs:136-151,
invocations :153-155). `rg 'impl(<…>)? TryFrom<'` over crates/ finds no
other fallible standard conversion into a foreign type. I read the macro,
the trait declarations `ParameterValue::{from_any, spec_from_any}`
(C/family.rs:56-71), the `family_conversions!` impls (:398-437), and the
rendered `AnyValue` page (target/doc/pert_ui_core/enum.AnyValue.html, which
shows all four `TryFrom<AnyValue> for …` impls).

- **Error type:** `type Error = AnyValue` (erased.rs:143-144); `AnyValue`
  implements neither `std::error::Error` nor `Display` (erased.rs:111-121,
  derives `Clone, Debug, PartialEq` only).
- **Display text:** none exists.
- **`# Errors`:** none; the method doc is the one line "Extracts a value of
  this family, returning the input on a mismatch.", which
  states the single condition (erased.rs:146).

**New findings:** none. **Already recorded:** claude-T4-9 and codex-T4-1
(an error position whose type implements neither `Error` nor `Display`;
both already name `f32`/`i64`/`bool`), codex-T4-6 (no `# Errors` heading;
its table names the three invocations at erased.rs:153-155), with my G4
verification's assessment that these impls state their condition and only
the heading form is at issue. The new rows add no instance.

**Inventory note:** CHANGES.md's "8" does not match the 3 rows added. The
six `ParameterValue::{from_any, spec_from_any}` implementations for `f32`,
`i64` and `bool` (`family_conversions!`, C/family.rs:434-436), which return
`Result<_, AnyValue>`/`Result<_, AnySpec>`, are still absent from
errors.txt (only the `ChoiceId` instances, errors.txt:5 and :7, and the
trait declarations, :6 and :8, appear). They hide no finding: their
`# Errors` is the trait declaration's (family.rs:59-60, :69-70) and the
error types are those of claude-T4-9.

## 4. D2: the six types now marked "prelude: yes (via pert_ui_runtime::prelude)"

**Entries processed:** all 6 (`ChoiceId`, `ChoiceOption`,
`NormalizedValue`, `ParameterId`, `ParameterMapping`, `ParameterSpec`).
Membership confirmed at R/lib.rs:366-369 (runtime prelude) and through
`pub use pert_ui_runtime::prelude::*` in the facade prelude
(F/lib.rs:65-74). Every method, field and variant line of the six in
semantic/pert-ui-core.txt was scanned for root types the prelude lacks.

**Effect on earlier findings and dispositions:** none changes.

- Both auditors already counted the six as members: my G5 verification's
  288 runtime prelude names include them ("Residual inventory caveat"), and
  codex-G5's member table lists them (codex-G5.md:104, :106) and its D2 row
  counts 288 runtime-prelude members. No finding or "no reverse-direction
  departure" disposition relied on the old "no".
- The names their signatures reach are all already classified:
  `PowerExponent` (`ParameterMapping::Power`, C/parameter.rs:37;
  claude-D2-5 already treats `ParameterMapping` as a prelude member);
  `ParameterFamily` (`ParameterSpec::family`, C/parameter.rs:210, and the
  `ParameterValue` impls; confirmed under codex-D2-1 in my verification,
  which cited only `AnySpec::family`; `ParameterSpec::family` is a further,
  direct reference from a prelude type, and strengthens that row without
  changing it); `ParameterValue`, `AnySpec`, `AnyValue`, `ParameterSnapshot`,
  `AnyParameterSnapshot` (codex-D2-1, claude-D2-5); `ParameterError`,
  `ParameterIdError`, `NormalizedValueError` (errors the rule keeps at the
  root, "Other error types stay at the crate root", R/lib.rs:313-319;
  codex-G5.md:117 lists `ParameterError` among the intentionally excluded).
- **New findings:** none.

## Summary

| Item | Entries processed | New findings | Recorded findings extended |
|---|---|---|---|
| C4 | 93 constants | claude-C4-v3-1 (fixed-precision format bypasses `MAX_PRECISION`), claude-C4-v3-2 (`MIN_WIDTH` clamps explicit and restored widths; restore path is a project-rule departure, setters a policy question), claude-C4-v3-3 (three limits whose value rustdoc does not show) | claude-C4-3 (`TextLimit::MAX_RASTER_SIDE`; text-preparation limits on four owners) |
| N3 | 31 `Option<number>` rows (+9 `Option` rows rechecked) | claude-N3-v3-1 (`min_length`/`max_length` count grapheme clusters), claude-N3-v3-2 (`Constraints` has no stated unit) | codex-N3-2 (byte ranges in `TextSelection::range`, `TextPreedit`, `RichText::highlighted`); claude-N3-d1 confirmed |
| T4 | 3 rows (CHANGES.md says 8) | none | none; already covered by claude-T4-9, codex-T4-1, codex-T4-6 |
| D2 | 6 types | none | none change; `ParameterSpec::family` adds direct evidence to the `ParameterFamily` row of codex-D2-1 |

Inventory notes: errors.txt gained 3 rows, not the 8 CHANGES.md states;
the six `ParameterValue` impl methods for `f32`/`i64`/`bool` remain
unlisted; numbers inside `Range` and arrays are still in no N3 list. None
hides a finding beyond those recorded above.
