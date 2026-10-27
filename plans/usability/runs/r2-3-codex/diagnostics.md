# Distinct diagnostics

```text
warning: unused import: `prelude::*`
 --> src/picker.rs:2:29
  |
2 | use pert_ui::{component::*, prelude::*};
  |                             ^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error[E0277]: the trait bound `Spacing: From<{integer}>` is not satisfied
   --> src/main.rs:25:18
    |
 25 |     ]))).padding(24).background_role(ColorRole::Background)))
    |          ------- ^^ the trait `From<{integer}>` is not implemented for `Spacing`
    |          |
    |          required by a bound introduced by this call
    |
help: `Spacing` implements trait `From<T>`
   --> /kit/vendor/pert-ui-runtime-0.3.75/src/runtime/theme.rs:332:1
    |
332 | impl From<crate::Length> for Spacing {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `From<pert_ui::Length>`
...
337 | impl From<MetricRole> for Spacing {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `From<MetricRole>`
    = note: required for `{integer}` to implement `std::convert::Into<Spacing>`
note: required by a bound in `pert_ui::Container::padding`
   --> /kit/vendor/pert-ui-runtime-0.3.75/src/containers.rs:173:44
    |
173 |     pub fn padding(mut self, padding: impl Into<Spacing>) -> Self {
    |                                            ^^^^^^^^^^^^^ required by this bound in `Container::padding`
```

```text
error[E0533]: expected value, found struct variant `SemanticValue::Number`
  --> src/headless.rs:52:45
   |
52 |     assert_eq!(d.semantics(&x)?.value, Some(SemanticValue::Number(1.)));
   |                                             ^^^^^^^^^^^^^^^^^^^^^ not a value
   |
help: you might have meant to create a new value of the struct
   |
52 -     assert_eq!(d.semantics(&x)?.value, Some(SemanticValue::Number(1.)));
52 +     assert_eq!(d.semantics(&x)?.value, Some(SemanticValue::Number { value: /* value */, minimum: /* value */, maximum: /* value */ }));
   |
```

```text
error[E0533]: expected value, found struct variant `SemanticValue::Number`
  --> src/headless.rs:53:45
   |
53 |     assert_eq!(d.semantics(&y)?.value, Some(SemanticValue::Number(0.)));
   |                                             ^^^^^^^^^^^^^^^^^^^^^ not a value
   |
help: you might have meant to create a new value of the struct
   |
53 -     assert_eq!(d.semantics(&y)?.value, Some(SemanticValue::Number(0.)));
53 +     assert_eq!(d.semantics(&y)?.value, Some(SemanticValue::Number { value: /* value */, minimum: /* value */, maximum: /* value */ }));
   |
```

```text
error[E0277]: the trait bound `InspectionSnapshot: serde::Serialize` is not satisfied
    --> src/headless.rs:93:78
     |
  93 |     std::fs::write(output.join("inspection.json"), serde_json::to_vec_pretty(&snapshot)?)?;
     |                                                    ------------------------- ^^^^^^^^^ the trait `serde_core::ser::Serialize` is not implemented for `InspectionSnapshot`
     |                                                    |
     |                                                    required by a bound introduced by this call
     |
     = note: for local types consider adding `#[derive(serde::Serialize)]` to your `InspectionSnapshot` type
     = note: for types from other crates check whether the crate offers a `serde` feature flag
     = help: the following other types implement trait `serde_core::ser::Serialize`:
               &'a T
               &'a mut T
               ()
               (T,)
               (T0, T1)
               (T0, T1, T2)
               (T0, T1, T2, T3)
               (T0, T1, T2, T3, T4)
             and 128 others
note: required by a bound in `to_vec_pretty`
    --> /kit/vendor/serde_json-1.0.154/src/ser.rs:2231:17
     |
2229 | pub fn to_vec_pretty<T>(value: &T) -> Result<Vec<u8>>
     |        ------------- required by a bound in this function
2230 | where
2231 |     T: ?Sized + Serialize,
     |                 ^^^^^^^^^ required by this bound in `to_vec_pretty`
```

```text
error[E0433]: cannot find type `InspectValue` in this scope
  --> src/headless.rs:96:61
   |
96 |     assert_eq!(inspected.details.properties.get("x"), Some(&InspectValue::Number(0.5)));
   |                                                             ^^^^^^^^^^^^ use of undeclared type `InspectValue`
```

```text
error[E0433]: cannot find type `InspectValue` in this scope
  --> src/headless.rs:97:61
   |
97 |     assert_eq!(inspected.details.properties.get("y"), Some(&InspectValue::Number(0.5)));
   |                                                             ^^^^^^^^^^^^ use of undeclared type `InspectValue`
```

```text
error[E0433]: cannot find type `ThemeMode` in this scope
   --> src/headless.rs:100:27
    |
100 |     for (mode, file) in [(ThemeMode::Dark, "dark.png"), (ThemeMode::Light, "light.png")] {
    |                           ^^^^^^^^^ use of undeclared type `ThemeMode`
```

```text
error[E0433]: cannot find type `ThemeMode` in this scope
   --> src/headless.rs:100:58
    |
100 |     for (mode, file) in [(ThemeMode::Dark, "dark.png"), (ThemeMode::Light, "light.png")] {
    |                                                          ^^^^^^^^^ use of undeclared type `ThemeMode`
```

```text
error[E0277]: the trait bound `Spacing: From<{integer}>` is not satisfied
   --> src/main.rs:26:18
    |
 26 |     ]))).padding(24).background_role(ColorRole::Background)))
    |          ------- ^^ the trait `From<{integer}>` is not implemented for `Spacing`
    |          |
    |          required by a bound introduced by this call
    |
help: `Spacing` implements trait `From<T>`
   --> /kit/vendor/pert-ui-runtime-0.3.75/src/runtime/theme.rs:332:1
    |
332 | impl From<crate::Length> for Spacing {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `From<pert_ui::Length>`
...
337 | impl From<MetricRole> for Spacing {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `From<MetricRole>`
    = note: required for `{integer}` to implement `std::convert::Into<Spacing>`
note: required by a bound in `pert_ui::Container::padding`
   --> /kit/vendor/pert-ui-runtime-0.3.75/src/containers.rs:173:44
    |
173 |     pub fn padding(mut self, padding: impl Into<Spacing>) -> Self {
    |                                            ^^^^^^^^^^^^^ required by this bound in `Container::padding`
```

```text
error[E0433]: cannot find type `InspectValue` in this scope
  --> src/headless.rs:11:14
   |
11 |         Some(InspectValue::Number(n)) => Ok(*n),
   |              ^^^^^^^^^^^^ use of undeclared type `InspectValue`
```
