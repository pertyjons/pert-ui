# codex delta review: inventory version 3

Reviewed inventory version 3 at `245053b`, against library source unchanged since `d4ba108`. Read-only; no files modified, fixes authored, builds or probes executed.

Coverage differs from CHANGES.md: the supplied inventory contains **31 `Option<number>` rows**, not 41, and **three added foreign-target error rows**, not eight. Comparison with version 2 confirms 29 newly added numeric rows plus two amended rows. The constants and prelude counts match: 93 and six.

## C4 — Associated constants

**Entries processed:** all **93 rows** of `candidates/constants.txt`. Read declarations and documentation, followed limit enforcement, and distinguished bounds from defaults, identity values, operation sets and enumeration slices.

### codex-C4-v3-1 Table layout can violate its documented minimum column width

- **Item and rule kind:** C4, **project rule**; documented-limit enforcement.
- **Items and configurations:** `pert_ui_runtime::TableColumn::MIN_WIDTH`, table column layout; all runtime configurations and facade re-exports.
- **Observation — behavior:** [`TableColumn::MIN_WIDTH`](<repo>/crates/pert-ui-runtime/src/table.rs:68) is `40.0`, documented as “The narrowest a column can be laid out or dragged”. Construction initializes that minimum, and `width(self, width: Length) -> Self` respects it.

  However, [`Shared::resolve`](<repo>/crates/pert-ui-runtime/src/table.rs:355) first applies `.max(column.minimum)` and then `.min(f32::MAX - start)`. With a first fixed column of `f32::MAX`, the next column starts at `f32::MAX` and receives resolved width **zero**. These widths are accepted: `Length::new` admits finite nonnegative values, and `TableController::new` checks column identities without imposing a combined-width bound.

  This is a source-derived witness. The existing [`finite_extreme_fixed_widths_keep_layout_valid` test](<repo>/crates/pert-ui/tests/collections/table_columns.rs:577) covers such descriptions but checks successful layout, painting and finite scrolling, without asserting the minimum.
- **Rule:** C4’s documented-limit check requires the advertised bound to hold. Documented control-limit clamping explains raising a narrow requested width to 40; no public exception authorizes reducing the resolved width to zero at coordinate saturation.
- **Severity:** **low** — requires extreme finite widths, but contradicts an unconditional public guarantee.
- **Confidence:** **high**.
- **Suggested fix and compatibility:** preserve the minimum while handling coordinate saturation, or explicitly document the saturation exception and its presentation consequences. Add a focused assertion for the chosen contract. No signature break is necessary; rejecting previously accepted descriptions would change their behavioral contract.
- **Destination:** **TODO.md**.

**Findings already recorded that these entries extend or corroborate:**

- **`codex-C4-1`:** `Workspace::MAX_DOCUMENT_BYTES` remains vulnerable to unchecked growth through `set_settings` (`workspace.rs:541`, `:552`, `:866`). Already recorded; no new finding.
- **`codex-C4-3` / `claude-C4-3`:** confirmed shared or external ownership of workspace, text-byte, precision and font-raster limits. An additional instance is [`TextLimit::MAX_RASTER_SIDE`](<repo>/crates/pert-ui-runtime/src/text.rs:107): its 8192-pixel bound applies to prepared text, while the constant belongs to the reporting classifier. Enforcement exists at `text.rs:1092` and `:1391`. This extends the existing **shared-limit ownership policy question**, without establishing an enforcement failure.
- **`claude-C4-2`:** `VectorRasterizer::MAX_COVERAGE_WORK` is exposed and enforced; it does not supply the separately documented point and stroke-edge constants already identified as missing.

No additional ownership or enforcement departure established. In particular, C4 does not require `PowerExponent::MIN/MAX` to have type `Self`.

## N3 — Optional numeric values

**Entries processed:** all **31 `Option<number>` rows**, without sampling.

| Classification | Rows | Result |
|---|---:|---|
| Geometry: `Constraints::{new,width,height}`, five text-preparation methods, `TableColumn::fixed_width`, `ToolWindowChange::with_geometry` | 10 | Extend the existing geometry-unit findings |
| `TextField` / `TextArea` minimum and maximum lengths | 4 | Extend the existing text-length unit finding |
| Decimal-formatting `places` parameters | 2 | Decimal context and “places” identify precision; no finding |
| Indices, event cursors, counts, scores, flex weight, application-defined domain values and workspace numbers | 15 | No library-defined unit omitted |

**New findings:** none beyond the recorded patterns.

**Findings already recorded that these entries extend:**

- **`codex-N3-1` / `claude-N3-2` / `claude-N3-d1`:** all ten geometry rows use `Option<f32>` without a pixel unit in the numeric parameter or method name. Seven are already explicitly covered by `claude-N3-d1`. The additional instances are:
  - `Constraints::new(width: Option<f32>, height: Option<f32>)`
  - `Constraints::width(self) -> Option<f32>`
  - `Constraints::height(self) -> Option<f32>`

  Source: [`layout.rs:262`](<repo>/crates/pert-ui-runtime/src/layout.rs:262), `:282`, `:287`. These represent logical measurement space; `bounded(Size)` supplies the same stored quantities. They extend the existing primitive-geometry pattern rather than introduce another finding.

- **`claude-N3-d2`:** the four text-length builders expose `count: Option<usize>` while counting **grapheme clusters**, established by the shared macro’s [`min_length`](<repo>/crates/pert-ui-runtime/src/text_fields.rs:446) and [`max_length`](<repo>/crates/pert-ui-runtime/src/text_fields.rs:458). The earlier finding already records these methods as the contrasting meaning of “length”. The new candidate rows extend its direct N3 coverage: `grapheme_count` would express the unit through the parameter name without changing call syntax.

`AxisMapping::tick_step` returns a step in the application-selected domain; its input explicitly says `min_pixels`. No fixed application unit should be introduced into that general mapping API.

## T4 — Foreign-target fallible conversions

**Entries processed:** the **three actual added rows**:

- `<f32 as TryFrom<AnyValue>>::try_from`
- `<i64 as TryFrom<AnyValue>>::try_from`
- `<bool as TryFrom<AnyValue>>::try_from`

Also inspected the **six still-unlisted** primitive `ParameterValue::{from_any,spec_from_any}` implementations, covering all **nine concrete operations** behind this gap.

**New findings:** none.

**Findings already recorded that these entries extend:**

- **`codex-T4-1` / `claude-T4-9`:** [`value_conversions!`](<repo>/crates/pert-ui-core/src/erased.rs:142) sets `type Error = AnyValue`. [`family_conversions!`](<repo>/crates/pert-ui-core/src/family.rs:407) returns the rejected `AnyValue` or `AnySpec` unchanged on a family mismatch. Both carriers derive `Clone`, `Debug` and `PartialEq`, but implement neither `Display` nor `std::error::Error`. Thus there is no diagnostic `Display` text to assess. This confirms the existing error-carrier finding and its recorded policy question about rejected-input errors.
- **`codex-T4-6`:** the three `TryFrom` methods have a summary explaining mismatch behavior, but no `# Errors` section (`erased.rs:145`). They were already explicitly included in that finding.
- The six `ParameterValue` implementation methods inherit complete mismatch conditions from the trait declarations’ `# Errors` sections at [`family.rs:59`](<repo>/crates/pert-ui-core/src/family.rs:59) and `:69`. They add no documentation finding.

**Residual inventory gap:** version 3 still omits `from_any` and `spec_from_any` for each of `f32`, `i64` and `bool`. Their implementations are at `family.rs:407`, `:418`, instantiated at `:434–436`. This is the residual gap already recorded in the G4 verification, not a newly discovered API defect. CHANGES.md’s claim of eight added error rows should be corrected.

## D2 — Cross-package prelude membership

**Entries processed:** all **six** corrected types:

`ChoiceId`, `ChoiceOption`, `NormalizedValue`, `ParameterId`, `ParameterMapping`, `ParameterSpec`.

**New findings:** none.

All six are explicitly exported by [`pert_ui_runtime::prelude`](<repo>/crates/pert-ui-runtime/src/lib.rs:367). Runtime first re-exports their core definitions at `lib.rs:284`; the facade then re-exports the runtime prelude at [`pert-ui/src/lib.rs:73`](<repo>/crates/pert-ui/src/lib.rs:73). These paths are unconditional across the inventoried runtime and facade configurations.

Their membership satisfies the prelude’s state-and-parameter rule. Version 3 corrects inventory metadata; it changes no API.

**Findings already recorded that these entries extend:** none. Neither **`codex-D2-1`** nor **`claude-D2-5`** listed these six as omissions. Those findings and the earlier D2 dispositions require no amendment from this delta.