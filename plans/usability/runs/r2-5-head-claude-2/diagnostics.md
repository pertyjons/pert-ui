# Distinct diagnostics

```text
error[E0277]: the trait bound `String: std::error::Error` is not satisfied
   --> src/lib.rs:201:71
    |
201 |             .map_err(|error| RuntimeError::from(ApplicationError::new(error.to_string())))?;
    |                                                 --------------------- ^^^^^^^^^^^^^^^^^ the trait `std::error::Error` is not implemented for `String`
    |                                                 |
    |                                                 required by a bound introduced by this call
    |
note: required by a bound in `pert_ui::ApplicationError::new`
   --> /kit/vendor/pert-ui-runtime-0.3.75/src/error.rs:59:28
    |
 59 |     pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
    |                            ^^^^^ required by this bound in `ApplicationError::new`
help: consider removing this method call, as the receiver has type `std::io::Error` and `std::io::Error: std::error::Error` trivially holds
    |
201 -             .map_err(|error| RuntimeError::from(ApplicationError::new(error.to_string())))?;
201 +             .map_err(|error| RuntimeError::from(ApplicationError::new(error)))?;
    |
```

```text
error[E0425]: cannot find type `Semantics` in this scope
  --> tests/headless.rs:77:83
   |
77 | fn cell(driver: &Driver<'_>, root: &WidgetId, row: &str, path: &[&str]) -> Result<Semantics, Box<dyn std::error::Error>> {
   |                                                                                   ^^^^^^^^^ not found in this scope
   |
help: you might be missing a type parameter
   |
77 | fn cell<Semantics>(driver: &Driver<'_>, root: &WidgetId, row: &str, path: &[&str]) -> Result<Semantics, Box<dyn std::error::Error>> {
   |        +++++++++++
```

```text
warning: unused import: `pert_ui::prelude`
 --> tests/real_clock.rs:6:5
  |
6 | use pert_ui::prelude::*;
  |     ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```
