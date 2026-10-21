**1. G2-12: confirmed, by source tracing; not executed.**

Claude’s added claim holds when the callback is installed before notification delivery, the retained divider survives, and its committed preference differs from the reporting baseline:

- `RegionLayout` unconditionally installs a forwarding closure, even when its application callback slot is empty ([region_layout.rs:363](<repo>/crates/pert-ui-runtime/src/region_layout.rs:363)).
- Consequently, `SplitPane::changed` sees a callback and records changes ([split.rs:339](<repo>/crates/pert-ui-runtime/src/split.rs:339)). Restore records `Restore` ([split.rs:587](<repo>/crates/pert-ui-runtime/src/split.rs:587)); replacing the described sizing records `Redescription` ([split.rs:511](<repo>/crates/pert-ui-runtime/src/split.rs:511)).
- Installing the application callback through another description replaces one forwarding closure with another. Both split callbacks are `Some`, so reconciliation does **not** discard the pending report ([split.rs:496](<repo>/crates/pert-ui-runtime/src/split.rs:496)).
- Delivery consumes the report and invokes the current forwarding closure, which finds the newly installed handler ([split.rs:619](<repo>/crates/pert-ui-runtime/src/split.rs:619), [region_layout.rs:366](<repo>/crates/pert-ui-runtime/src/region_layout.rs:366)).

For example: start with a settled width of 100 and no callback; redescribe width 160 without a callback; install a callback while retaining width 160 before delivery. That callback receives `(160, Redescription)`. A pending Restore follows the same path if the intervening description preserves the restored preference.

This is conditional: an unchanged final preference suppresses delivery ([geometry_report.rs:68](<repo>/crates/pert-ui-runtime/src/geometry_report.rs:68)); delivery while the slot remains empty consumes the report.

The behavior conflicts with the geometry contract’s requirement that a description without a callback discard undelivered changes ([contracts.md:355](<repo>/docs/contracts.md:355)) and RegionLayout’s corresponding promise ([region_layout.rs:253](<repo>/crates/pert-ui-runtime/src/region_layout.rs:253)).

**Severity/fix:** no change to the proposed **low** severity or existing fix. The addition broadens the demonstrated case to changes recorded without any application callback. Preserve callback absence in generated splits, or explicitly propagate callback absence/removal to their report state. Regression coverage should include both Restore and Redescription recorded without a callback, followed by installation before delivery.

**2. Inventory version 4: new findings — none.**

Processed [Version 4’s delta description](<repo>/plans/usability/api-audit/inputs/CHANGES.md:15), all eight concrete serde method rows for `ParameterSpec<f32/i64/bool/ChoiceId>` in `candidates/errors.txt` and the semantic list, and `ParameterValue::FAMILY` in `candidates/constants.txt` and the semantic list.

- **T4, serde configurations:** reviewed all four serializers and deserializers ([parameter.rs:636](<repo>/crates/pert-ui-core/src/parameter.rs:636)). Serializers forward serializer failures; deserializers forward format failures and rebuild through validating constructors/builders, converting validation failures through `D::Error::custom`. Their missing implementation-specific `# Errors` sections are already explicitly covered by **G4-20**, subject to **G4-19’s** standard-trait forwarding policy question ([findings.md:686](<repo>/plans/usability/api-audit/findings.md:686)). The type-level serialization contract documents reconstruction and rejection ([parameter.rs:142](<repo>/crates/pert-ui-core/src/parameter.rs:142)). Splitting the previously collapsed rows reveals no additional departure.
- **C4, all configurations:** `ParameterValue::FAMILY` is a typed family discriminator, not a numeric bound or raw-value accessor ([family.rs:50](<repo>/crates/pert-ui-core/src/family.rs:50)). The trait is sealed, and the implementations map each canonical type to its corresponding family ([family.rs:398](<repo>/crates/pert-ui-core/src/family.rs:398)). C4 requires no `MIN_*`/`MAX_*` naming or fallible construction for this constant.

No files were modified.