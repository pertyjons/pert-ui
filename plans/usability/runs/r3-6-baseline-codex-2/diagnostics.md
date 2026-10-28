# Distinct diagnostics

```text
error[E0433]: cannot find type `SplitPane` in this scope
   --> src/main.rs:192:9
    |
192 |         SplitPane::new(SplitAxis::LeftRight, list.key("notes"), tabs.key("tabs"))
    |         ^^^^^^^^^ use of undeclared type `SplitPane`
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:172:13
    |
172 |             cx.defer(move |ui| a.open(ui, &name))?; Ok(())
    |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
    = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
    |
172 |             let _ = cx.defer(move |ui| a.open(ui, &name))?; Ok(())
    |             +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:177:17
    |
177 |                 cx.defer(move |ui| b.open(ui, &name))?;
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
177 |                 let _ = cx.defer(move |ui| b.open(ui, &name))?;
    |                 +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:208:66
    |
208 |             Button::new("Create", move |cx| { let a = a.clone(); cx.defer(move |ui| a.create(ui))?; Ok(()) }).key("create"),
    |                                                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
208 |             Button::new("Create", move |cx| { let a = a.clone(); let _ = cx.defer(move |ui| a.create(ui))?; Ok(()) }).key("create"),
    |                                                                  +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:216:64
    |
216 |             Button::new("Save", move |cx| { let a = a.clone(); cx.defer(move |ui| a.finish_close(ui, true))?; Ok(()) }).key("save"),
    |                                                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
216 |             Button::new("Save", move |cx| { let a = a.clone(); let _ = cx.defer(move |ui| a.finish_close(ui, true))?; Ok(()) }).key("save"),
    |                                                                +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:217:67
    |
217 | ...   Button::new("Discard", move |cx| { let b = b.clone(); cx.defer(move |ui| b.finish_close(ui, false))?; Ok(()) }).key("discard"),
    |                                                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
217 |             Button::new("Discard", move |cx| { let b = b.clone(); let _ = cx.defer(move |ui| b.finish_close(ui, false))?; Ok(()) }).key("discard"),
    |                                                                   +++++++
```

```text
error[E0425]: cannot find type `Semantics` in this scope
   --> src/main.rs:278:50
    |
278 |         fn semantics(&mut self, id: WidgetId) -> Semantics { Driver::new(&mut self.ui).semantics(&id).unwrap() }
    |                                                  ^^^^^^^^^ not found in this scope
```

```text
error[E0425]: cannot find type `Semantics` in this scope
   --> src/main.rs:451:50
    |
451 |         fn semantics(&mut self, id: WidgetId) -> Semantics {
    |                                                  ^^^^^^^^^ not found in this scope
```
