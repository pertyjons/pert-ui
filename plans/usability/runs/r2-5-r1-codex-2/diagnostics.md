# Distinct diagnostics

```text
warning: unused import: `TableSort`
 --> src/main.rs:1:111
  |
1 | use pert_ui::{prelude::*, Capacity, Mailbox, UiSender, TableItems, TableSource, TableController, TableColumn, TableSort, SortDirectio...
  |                                                                                                               ^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error[E0425]: cannot find function `headless` in this scope
   --> src/main.rs:119:50
    |
119 |     if std::env::args().any(|a|a=="--headless") {headless()?;} else {
    |                                                  ^^^^^^^^ not found in this scope
```
