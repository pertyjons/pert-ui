# Distinct diagnostics

```text
error[E0599]: no method named `root` found for struct `NativeWindowId` in the current scope
  --> src/main.rs:97:64
   |
97 |             let root = ui.find_descendant(&ui.primary_window().root(), "settings")?;
   |                                                                ^^^^ method not found in `NativeWindowId`
```

```text
error[E0061]: this method takes 1 argument but 2 arguments were supplied
   --> src/main.rs:136:8
    |
136 |     }).size(720.0,820.0).on_close(move |_,_| {
    |        ^^^^ ----- ----- unexpected argument #2 of type `{float}`
    |             |
    |             expected `Size`, found floating-point number
    |
note: method defined here
   --> /kit/vendor/pert-ui-0.3.75/src/app.rs:395:12
    |
395 |     pub fn size(mut self, size: crate::Size) -> Result<Self, RuntimeError> {
    |            ^^^^
help: remove the extra argument
    |
136 -     }).size(720.0,820.0).on_close(move |_,_| {
136 +     }).size(/* Size */).on_close(move |_,_| {
    |
```

```text
error[E0599]: no method named `on_close` found for enum `Result<T, E>` in the current scope
   --> src/main.rs:136:26
    |
133 | //     App::new("Photo export settings",move |ui| {
134 | ||         *slot.borrow_mut() = Some(build(ui,path)?);
135 | ||         Ok(())
136 | ||     }).size(720.0,820.0).on_close(move |_,_| {
    | ||      -                  -^^^^^^^^ method not found in `Result<pert_ui::App, pert_ui::RuntimeError>`
    | ||______|__________________|
    |  |______|
    |         method `on_close` is available on `pert_ui::App`
    |
note: the method `on_close` exists on the type `pert_ui::App`
   --> /kit/vendor/pert-ui-0.3.75/src/app.rs:579:5
    |
579 | /     pub fn on_close(
580 | |         mut self,
581 | |         handler: impl FnMut(
582 | |             &mut Runtime,
...   |
585 | |         + 'static,
586 | |     ) -> Self {
    | |_____________^
help: consider using `Result::expect` to unwrap the `pert_ui::App` value, panicking if the value is a `Result::Err`
    |
136 |     }).size(720.0,820.0).expect("REASON").on_close(move |_,_| {
    |                         +++++++++++++++++
```

```text
warning: field `root` is never read
  --> src/main.rs:35:16
   |
35 | struct Built { root: WidgetId, dirty: ReadSignal<bool>, busy: Signal<bool>, status: Signal<String> }
   |        -----   ^^^^
   |        |
   |        field in this struct
   |
   = note: `Built` has a derived impl for the trait `Clone`, but this is intentionally ignored during dead code analysis
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: unused `TaskReceipt` that must be used
  --> src/main.rs:77:9
   |
77 | /         cx.spawn(move || -> Result<(), String> {
78 | |             let data = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
79 | |             std::fs::write(path, data).map_err(|e| e.to_string())
80 | |         }, move |result| {
...  |
86 | |         })?;
   | |___________^
   |
   = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
   = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
   |
77 |         let _ = cx.spawn(move || -> Result<(), String> {
   |         +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/main.rs:101:9
    |
101 | /         cx.defer(move |ui| {
102 | |             for field in discard.borrow().iter() {
103 | |                 ui.text_command(field, pert_ui::TextCommand::DiscardDraft)?;
...   |
106 | |         })?;
    | |___________^
    |
    = note: the receipt is the only report of a `Cancelled` operation; failures also propagate from flush
help: use `let _ = ...` to ignore the resulting value
    |
101 |         let _ = cx.defer(move |ui| {
    |         +++++++
```

```text
error[E0533]: expected value, found struct variant `SemanticValue::Number`
   --> src/main.rs:208:47
    |
208 |             assert_eq!(value(&d,&b,"quality"),SemanticValue::Number(73.0));
    |                                               ^^^^^^^^^^^^^^^^^^^^^ not a value
    |
help: you might have meant to create a new value of the struct
    |
208 -             assert_eq!(value(&d,&b,"quality"),SemanticValue::Number(73.0));
208 +             assert_eq!(value(&d,&b,"quality"),SemanticValue::Number { value: /* value */, minimum: /* value */, maximum: /* value */ });
    |
```

```text
error[E0533]: expected value, found struct variant `SemanticValue::Number`
   --> src/main.rs:209:45
    |
209 |             assert_eq!(value(&d,&b,"width"),SemanticValue::Number(4096.0));
    |                                             ^^^^^^^^^^^^^^^^^^^^^ not a value
    |
help: you might have meant to create a new value of the struct
    |
209 -             assert_eq!(value(&d,&b,"width"),SemanticValue::Number(4096.0));
209 +             assert_eq!(value(&d,&b,"width"),SemanticValue::Number { value: /* value */, minimum: /* value */, maximum: /* value */ });
    |
```

```text
error[E0533]: expected value, found struct variant `SemanticValue::Number`
   --> src/main.rs:222:47
    |
222 |             assert_eq!(value(&d,&b,"quality"),SemanticValue::Number(73.0));
    |                                               ^^^^^^^^^^^^^^^^^^^^^ not a value
    |
help: you might have meant to create a new value of the struct
    |
222 -             assert_eq!(value(&d,&b,"quality"),SemanticValue::Number(73.0));
222 +             assert_eq!(value(&d,&b,"quality"),SemanticValue::Number { value: /* value */, minimum: /* value */, maximum: /* value */ });
    |
```

```text
error[E0308]: mismatched types
   --> src/main.rs:240:61
    |
240 |             d.wait_until(Duration::from_secs(5),|d| Ok(name(d,&b,"status")=="Settings saved"))?;
    |                                                        ---- ^ expected `&Driver<'_>`, found `&Runtime`
    |                                                        |
    |                                                        arguments to this function are incorrect
    |
    = note: expected reference `&pert_ui::testing::Driver<'_>`
               found reference `&pert_ui::Runtime`
note: function defined here
   --> src/main.rs:169:8
    |
169 |     fn name(d: &Driver<'_>, b: &Built, key: &str) -> String {
    |        ^^^^ --------------
```

```text
error[E0308]: mismatched types
   --> src/main.rs:262:61
    |
262 |             d.wait_until(Duration::from_secs(5),|d| Ok(name(d,&b,"status").starts_with("Save failed:")))?;
    |                                                        ---- ^ expected `&Driver<'_>`, found `&Runtime`
    |                                                        |
    |                                                        arguments to this function are incorrect
    |
    = note: expected reference `&pert_ui::testing::Driver<'_>`
               found reference `&pert_ui::Runtime`
note: function defined here
   --> src/main.rs:169:8
    |
169 |     fn name(d: &Driver<'_>, b: &Built, key: &str) -> String {
    |        ^^^^ --------------
```
