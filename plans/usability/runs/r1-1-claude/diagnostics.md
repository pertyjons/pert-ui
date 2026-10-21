# Distinct diagnostics

```text
warning: unused `TaskReceipt` that must be used
   --> src/lib.rs:255:13
    |
255 | /             cx.defer(move |ui| {
256 | |                 ui.spawn(
257 | |                     move || save(&target, &written),
258 | |                     move |result| {
...   |
269 | |                 .map(|_| ())
270 | |             })?;
    | |_______________^
    |
    = note: retain the receipt if the application needs the operation's outcome
    = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
    |
255 |             let _ = cx.defer(move |ui| {
    |             +++++++
```
