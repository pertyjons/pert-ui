# Distinct diagnostics

```text
error[E0599]: no variant, associated function, or constant named `Danger` found for enum `pert_ui::ColorRole` in the current scope
   --> src/lib.rs:280:36
    |
280 |             .color_role(ColorRole::Danger)
    |                                    ^^^^^^ variant, associated function, or constant not found in `pert_ui::ColorRole`
```

```text
warning: unused import: `pert_ui::prelude::*`
 --> tests/listing.rs:2:5
  |
2 | use pert_ui::prelude::*;
  |     ^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error[E0425]: cannot find type `WidgetId` in this scope
  --> tests/listing.rs:16:45
   |
16 | fn selected_tab(driver: &Driver<'_>, tabs: &WidgetId) -> Result<Option<String>, TestError> {
   |                                             ^^^^^^^^ not found in this scope
```

```text
error[E0433]: cannot find type `KeyCode` in this scope
  --> tests/listing.rs:48:16
   |
48 |     driver.key(KeyCode::ArrowDown)?;
   |                ^^^^^^^ use of undeclared type `KeyCode`
```

```text
error[E0433]: cannot find type `KeyCode` in this scope
  --> tests/listing.rs:49:16
   |
49 |     driver.key(KeyCode::Enter)?;
   |                ^^^^^^^ use of undeclared type `KeyCode`
```

```text
warning: field `content` is never read
  --> tests/app.rs:65:5
   |
63 | struct Ui {
   |        -- field in this struct
64 |     root: WidgetId,
65 |     content: WidgetId,
   |     ^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```
