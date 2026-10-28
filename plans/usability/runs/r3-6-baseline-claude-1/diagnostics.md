# Distinct diagnostics

```text
warning: unused import: `pert_ui::prelude`
 --> tests/explore.rs:1:5
  |
1 | use pert_ui::prelude::*;
  |     ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error[E0425]: cannot find type `Semantics` in this scope
  --> tests/notes.rs:43:47
   |
43 |     fn semantics(&self, id: &WidgetId) -> Res<Semantics> {
   |                                               ^^^^^^^^^ not found in this scope
   |
help: you might be missing a type parameter
   |
34 | impl<Semantics> Harness {
   |     +++++++++++
```
