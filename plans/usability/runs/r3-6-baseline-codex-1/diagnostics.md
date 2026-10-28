# Distinct diagnostics

```text
error[E0433]: cannot find type `SplitPane` in this scope
   --> src/main.rs:175:9
    |
175 |         SplitPane::new(SplitAxis::LeftRight, list.key("notes"), tabs.key("tabs"))
    |         ^^^^^^^^^ use of undeclared type `SplitPane`
```

```text
error[E0433]: cannot find type `SplitAxis` in this scope
   --> src/main.rs:175:24
    |
175 |         SplitPane::new(SplitAxis::LeftRight, list.key("notes"), tabs.key("tabs"))
    |                        ^^^^^^^^^ use of undeclared type `SplitAxis`
```

```text
error[E0433]: cannot find type `SplitSizing` in this scope
   --> src/main.rs:176:21
    |
176 |             .sizing(SplitSizing::first(Length::from(220))).key("split").with_grow(1),
    |                     ^^^^^^^^^^^ use of undeclared type `SplitSizing`
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:130:13
    |
130 |             cx.defer(move |ui| app.open(ui, &name))?;
    |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
    = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
    |
130 |             let _ = cx.defer(move |ui| app.open(ui, &name))?;
    |             +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:154:9
    |
154 |         cx.defer(move |ui| a.request_close(ui))?;
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
154 |         let _ = cx.defer(move |ui| a.request_close(ui))?;
    |         +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:163:13
    |
163 |             cx.defer(move |ui| a.open(ui, &name))?;
    |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
163 |             let _ = cx.defer(move |ui| a.open(ui, &name))?;
    |             +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:192:66
    |
192 |             Button::new("Create", move |cx| { let a = a.clone(); cx.defer(move |ui| a.new_note(ui))?; Ok(()) }).key("create"),
    |                                                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
192 |             Button::new("Create", move |cx| { let a = a.clone(); let _ = cx.defer(move |ui| a.new_note(ui))?; Ok(()) }).key("create"),
    |                                                                  +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:204:17
    |
204 |                 cx.defer(move |ui| { if a.save()? { a.close_dialog.accept()?; a.remove_current(ui)?; } Ok(()) })?;
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
204 |                 let _ = cx.defer(move |ui| { if a.save()? { a.close_dialog.accept()?; a.remove_current(ui)?; } Ok(()) })?;
    |                 +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:209:17
    |
209 |                 cx.defer(move |ui| { b.close_dialog.accept()?; b.remove_current(ui) })?;
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
209 |                 let _ = cx.defer(move |ui| { b.close_dialog.accept()?; b.remove_current(ui) })?;
    |                 +++++++
```
