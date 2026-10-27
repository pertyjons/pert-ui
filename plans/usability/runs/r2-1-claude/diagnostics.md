# Distinct diagnostics

```text
warning: unused `TaskReceipt` that must be used
   --> src/lib.rs:247:13
    |
247 | /             cx.spawn(
248 | |                 move || save(&path, &written).map(|()| path),
249 | |                 move |result| {
250 | |                     saving.set(false)?;
...   |
258 | |                 },
259 | |             )?;
    | |______________^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
    = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
    |
247 |             let _ = cx.spawn(
    |             +++++++
```

```text
error[E0425]: cannot find type `Semantics` in this scope
  --> tests/headless.rs:37:63
   |
37 | fn sem(driver: &Driver, root: &WidgetId, key: &str) -> Result<Semantics, Box<dyn std::error::Error>> {
   |                                                               ^^^^^^^^^ not found in this scope
   |
help: you might be missing a type parameter
   |
37 | fn sem<Semantics>(driver: &Driver, root: &WidgetId, key: &str) -> Result<Semantics, Box<dyn std::error::Error>> {
   |       +++++++++++
```

```text
warning: unused import: `CLOSE_REFUSED_MESSAGE`
 --> tests/headless.rs:6:37
  |
6 | use settings_editor::{settings_app, CLOSE_REFUSED_MESSAGE};
  |                                     ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: unused import: `time::Duration`
 --> tests/headless.rs:7:26
  |
7 | use std::{path::PathBuf, time::Duration};
  |                          ^^^^^^^^^^^^^^
```

```text
warning: unused import: `CLOSE_REFUSED_MESSAGE`
 --> tests/headless.rs:7:37
  |
7 | use settings_editor::{settings_app, CLOSE_REFUSED_MESSAGE};
  |                                     ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: unused import: `time::Duration`
 --> tests/headless.rs:8:26
  |
8 | use std::{path::PathBuf, time::Duration};
  |                          ^^^^^^^^^^^^^^
```

```text
error[E0433]: cannot find type `InspectValue` in this scope
  --> tests/headless.rs:72:41
   |
72 |     assert_eq!(props.get("unit"), Some(&InspectValue::Text("px".into())));
   |                                         ^^^^^^^^^^^^ use of undeclared type `InspectValue`
```

```text
error[E0433]: cannot find type `InspectValue` in this scope
  --> tests/headless.rs:73:50
   |
73 |     assert_eq!(props.get("display_value"), Some(&InspectValue::Text("4096".into())));
   |                                                  ^^^^^^^^^^^^ use of undeclared type `InspectValue`
```

```text
error[E0433]: cannot find type `InspectValue` in this scope
  --> tests/headless.rs:74:50
   |
74 |     assert_eq!(props.get("shows_display"), Some(&InspectValue::Bool(true)));
   |                                                  ^^^^^^^^^^^^ use of undeclared type `InspectValue`
```
