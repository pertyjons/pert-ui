# Distinct diagnostics

```text
warning: unused import: `prelude::*`
  --> src/picker.rs:18:29
   |
18 | use pert_ui::{component::*, prelude::*};
   |                             ^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error[E0425]: cannot find type `ThemeMode` in this scope
  --> src/app.rs:75:11
   |
75 |     mode: ThemeMode,
   |           ^^^^^^^^^ not found in this scope
```

```text
warning: unused import: `pert_ui::prelude`
 --> src/main.rs:5:5
  |
5 | use pert_ui::prelude::*;
  |     ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error[E0433]: cannot find type `SemanticRole` in this scope
   --> tests/headless.rs:134:32
    |
134 |     assert_eq!(semantics.role, SemanticRole::Slider);
    |                                ^^^^^^^^^^^^ use of undeclared type `SemanticRole`
    |
help: an enum with a similar name exists
    |
134 -     assert_eq!(semantics.role, SemanticRole::Slider);
134 +     assert_eq!(semantics.role, SemanticValue::Slider);
    |
```

```text
error[E0433]: cannot find type `InspectValue` in this scope
   --> tests/headless.rs:136:58
    |
136 |     assert_eq!(picker.details.properties.get("x"), Some(&InspectValue::Number(0.125)));
    |                                                          ^^^^^^^^^^^^ use of undeclared type `InspectValue`
```

```text
error[E0433]: cannot find type `InspectValue` in this scope
   --> tests/headless.rs:137:58
    |
137 |     assert_eq!(picker.details.properties.get("y"), Some(&InspectValue::Number(0.875)));
    |                                                          ^^^^^^^^^^^^ use of undeclared type `InspectValue`
```
