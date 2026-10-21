# Inventory version 4 delta review (claude)

Auditor: claude. Read-only. Source at HEAD is unchanged from the audited commit
`d4ba108` (`git diff --stat d4ba108 HEAD -- crates` is empty). Inputs: rubric.md
(C4, T4), findings.md (G1, G4, V3 sections), inputs/CHANGES.md "Version 4".

## Result: none

No new finding. Each addition is already covered by findings.md or departs
from no rubric item.

## What I processed

### Serde rows of `ParameterSpec<f32/i64/bool/ChoiceId>` (T4)

The rows are `errors.txt` lines 15–18 (`Deserialize::deserialize`, `D::Error`)
and 51–54 (`Serialize::serialize`, `S::Error`), all `[configs: serde]`. I
matched them to `crates/pert-ui-core/src/parameter.rs:637-736` (module
`serde_impls`, `:587`) and checked them against each T4 sub-item.

- **Type and traits:** the error types are serde's associated `D::Error` and
  `S::Error`, which are foreign and chosen by the format. T4's derive and
  `std::error::Error` requirements cover library-owned errors, so no
  departure.
- **Causes and conversions:** each `deserialize` rebuilds the specification
  through the public constructors and passes `ParameterError` through
  `D::Error::custom` (`:661`, `:692`, `:714`, `:736`). That turns the typed
  library cause into a string, but serde's `de::Error` contract leaves no
  typed path, and every other hand-written serde impl the audit already
  reviewed does the same (G4-20). The case is shared with those impls and
  imposed by the trait, so I record no finding for these rows. `serialize`
  only forwards the serializer's error.
- **`Display`:** the forwarded text is `ParameterError`'s Display
  (`parameter.rs:~540-582`). Its value-less variants (`InvalidRange`,
  `OutOfRange`, `OffLattice`, `DuplicateOption`, `UnknownOption`) are already
  G4-14, with the `Copy` question G4-15. These rows add no new variant.
- **Documentation:** none of the eight methods has its own `# Errors`. G4-20
  already names `ParameterSpec<f32/i64/bool/ChoiceId>` explicitly, and the
  forwarding question is G4-19. The type's `# Serialization` section
  (`parameter.rs:142-149`) says that reading rebuilds through the
  constructors and rejects an invalid domain or default, never clamping.
  That matches the code. The f32 path is `new`, then `with_mapping`. The i64
  path is `stepped`, then `with_step`, which validates the step lattice again
  (`:413-421`). The bool path is `boolean` and the choice path is `choices`.
  `read_only` is applied afterwards by `finish` (`:630-635`), so nothing is
  clamped or reinterpreted. Codex's G4 verification
  (`reports/verification/codex-G4.md:64`) had already covered the same eight
  locations by hand.

### Trait constant `ParameterValue::FAMILY` (C4)

`constants.txt:10`; `crates/pert-ui-core/src/family.rs:51`; semantic list
`inputs/semantic/pert-ui-core.txt:232`.

- It is a type-level family tag of the closed `ParameterFamily` enum
  (`family.rs:16-27`) on the sealed trait (`:47-49`). It is neither a
  documented limit, so the `MIN_*`/`MAX_*` sub-item does not apply, nor a
  domain value with an invariant (classification: type tag, not
  invariant-bearing).
- Uses: `ParameterSpec::family` (`parameter.rs:208-212`) and the runtime's
  `BindingError::KindMismatch { expected: T::FAMILY, .. }`
  (`crates/pert-ui-runtime/src/parameter_binding/erased.rs:270`). Both are
  consistent with its rustdoc.
- `ParameterFamily`'s absence from the runtime prelude is already G5-12, and
  claude's G5 verification names the constant there.

### Inventory completeness

The version 4 rows agree with the source: 4 types × 2 directions = 8 rows. They
replace the collapsed row, which CHANGES.md counts as "6 rows" added. There is
one trait-associated constant in pert-ui-core's traits. I found no inventory
gap in the delta.
