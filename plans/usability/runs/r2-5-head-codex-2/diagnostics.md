# Distinct diagnostics

```text
warning: fields `root`, `jobs`, and `table` are never read
  --> src/main.rs:52:18
   |
52 | struct Monitor { root:WidgetId, jobs:Shared, table:TableController }
   |        -------   ^^^^           ^^^^         ^^^^^
   |        |
   |        fields in this struct
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: unused `TaskReceipt` that must be used
  --> src/main.rs:75:3
   |
75 | /   cx.defer(move |ui| {
76 | |    let id=js.borrow().len()+1;
77 | |    let mut rng=Random(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64 | 1);
78 | |    let total=if config.deterministic {if id==2 || id==4 {500}else{50}}else{rng.range(50,500)};
...  |
98 | |    Ok(())
99 | |   })?; Ok(())
   | |_____^
   |
   = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
   = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
   |
75 |   let _ = cx.defer(move |ui| {
   |   +++++++
```

```text
error[E0277]: the `?` operator can only be applied to values that implement `Try`
   --> src/main.rs:189:2
    |
189 |  d.runtime_mut().shutdown()?;
    |  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ the `?` operator cannot be applied to type `()`
    |
    = help: the nightly-only, unstable trait `Try` is not implemented for `()`
```
