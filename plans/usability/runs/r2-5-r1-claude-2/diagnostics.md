# Distinct diagnostics

```text
warning: unused `ToastReceipt` that must be used
 --> examples/probe.rs:9:5
  |
9 |     queue.push(Toast::new("Job 1 failed"))?;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
  |
9 |     let _ = queue.push(Toast::new("Job 1 failed"))?;
  |     +++++++
```
