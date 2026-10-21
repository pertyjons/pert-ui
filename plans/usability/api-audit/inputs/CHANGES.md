# Inventory versions

**Version 1** was commit 6e39e18, built at d4ba108. Both auditors worked
from it. Their reports name the inventory gaps they found.

**Version 2** was built at 9e48cd1. The script was fixed for every gap,
and the library source is unchanged since d4ba108. The cross-verification
reviewed what version 2 adds, so a shared omission in version 1 cannot
hide a finding.

**Version 3** is this directory, built at aacbdd3. It closes the gaps the
cross-verification found in version 2. A separate delta review by both
auditors covers what it adds.

| Gap (reported by) | Fix | Entries version 2 adds or changes |
|---|---|---|
| Conversions filed under their source type without the target (codex G1, G3, G4; claude G1, G3, G4) | `conversion-traits.txt` names every implementation as `impl Trait for Target`. In `types.txt`, an implementation rustdoc files under another type is marked "(for another type)". | every row of `conversion-traits.txt` |
| Distinct `TryFrom` targets collapsed into one row (codex G4) | Same fix | `AnyValue`'s conversions |
| `bound` matched `bounded` (codex G1) | Constructor groups match `new`, `bound`, `for_parameter` and `default` exactly | 2 |
| C3 candidates missed state-bearing parameters (codex G1, claude G1) | Matching also covers names ending in `Host`, `Session`, `Binding`, `State`, `Selection`, plus `NavigationHistory`, `ToastQueue`, `SeriesSource` and `Parameter` | 18 |
| Conditional methods and traits lost when configurations merged (codex G2, G4, G5) | Traits and methods carry their own `[configs: ...]` | `App::software_rendering`; serde traits |
| Hand-written trait methods (`TryFrom`, `FromStr`, `Debug`) missing (codex G4, claude G4) | They appear in the semantic lists, and their `Result`s join `errors.txt` | 41 in `errors.txt` |
| Fallible builders missing (claude G2) | `builders.txt` lists `self -> Result<Self, _>` as `[fallible]` | 49 |
| Callbacks not named `on_*` missing (claude G2) | `callbacks.txt` lists every function taking a closure as `[closure]` | 64 |
| Prelude membership not recorded (claude G5) | `types.txt` and `free-items.txt` say whether the package's prelude holds the item | a column |
| Free functions, constants and modules in no list (claude G1, G5) | New `free-items.txt` | 30 |
| Field and variant payload types missing from the semantic lists (claude G5) | The semantic lists show public fields and variants with their payloads | every struct with public fields and every enum |
| A bare `Error` was ambiguous (claude G4) | Named by its full path | 4 |

Version 3 adds:

| Gap (reported by) | Fix | Entries version 3 adds or changes |
|---|---|---|
| Numbers wrapped in `Option` in no list (claude G3 verification) | `numeric.txt` includes `Option<number>` parameters and returns | 41 |
| Associated constants in no list (claude G3 verification) | New `constants.txt`; the semantic lists show them with their types | 93 |
| Fallible conversions into foreign types missing from `errors.txt`, such as `TryFrom<AnyValue> for f32` (claude G4 verification) | They join `errors.txt` | 3 |
| The prelude column was per package, so core types in pert-ui-runtime's prelude read "no" (claude G5 verification) | They read "yes (via pert_ui_runtime::prelude)" | 6 |

Gaps that remain are recorded instead. The version 3 delta reviews checked
them by hand and found no finding hidden in them:
- `ParameterValue::{from_any, spec_from_any}` for `f32`, `i64` and `bool`
  are not in `errors.txt`.
- Numbers inside `Range` or arrays, such as `TextSelection::range`, are in
  no N3 list. The `builder` mark in
`bool-params.txt` is mechanical, so a `Copy` value's `self -> Self`
operation, such as `TextSelection::moved`, also matches it (claude G3).
