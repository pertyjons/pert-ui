# Distinct diagnostics

```text
error[E0422]: cannot find struct, variant or union type `Modifiers` in this scope
   --> src/lib.rs:315:15
    |
315 |     let alt = Modifiers {
    |               ^^^^^^^^^
    |
   ::: /kit/vendor/pert-ui-runtime-0.3.52/src/runtime/notify.rs:242:1
    |
242 | pub struct Notifier {
    | ------------------- similarly named struct `Notifier` defined here
    |
help: a struct with a similar name exists
    |
315 -     let alt = Modifiers {
315 +     let alt = Notifier {
    |
```

```text
error[E0433]: cannot find type `Modifiers` in this scope
   --> src/lib.rs:317:11
    |
317 |         ..Modifiers::default()
    |           ^^^^^^^^^ use of undeclared type `Modifiers`
    |
help: a struct with a similar name exists
    |
317 -         ..Modifiers::default()
317 +         ..Notifier::default()
    |
```

```text
warning: field `actions` is never read
   --> src/lib.rs:116:5
    |
107 | pub struct TodoApp {
    |            ------- field in this struct
...
116 |     actions: Actions,
    |     ^^^^^^^
    |
    = note: `TodoApp` has a derived impl for the trait `Clone`, but this is intentionally ignored during dead code analysis
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: unused `CommandId` that must be used
   --> src/lib.rs:304:5
    |
304 | /     ui.register_command(
305 | |         "New task",
306 | |         CommandScope::Application,
307 | |         vec![KeyChord::primary('n')],
308 | |         move |ui, _| ui.focus(&field),
309 | |     )?;
    | |______^
    |
    = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
    |
304 |     let _ = ui.register_command(
    |     +++++++
```

```text
warning: unused `CommandId` that must be used
   --> src/lib.rs:311:5
    |
311 | /     ui.register_command(
312 | |         "Remove task",
313 | |         CommandScope::Application,
314 | |         vec![KeyChord::named(KeyCode::Delete)],
315 | |         move |_, _| remove.remove_selected(),
316 | |     )?;
    | |______^
    |
help: use `let _ = ...` to ignore the resulting value
    |
311 |     let _ = ui.register_command(
    |     +++++++
```

```text
warning: unused `CommandId` that must be used
   --> src/lib.rs:322:5
    |
322 | /     ui.register_command(
323 | |         "Move task up",
324 | |         CommandScope::Application,
325 | |         vec![KeyChord::new(CommandKey::Named(KeyCode::ArrowUp), alt)],
326 | |         move |_, _| up.move_selected(true),
327 | |     )?;
    | |______^
    |
help: use `let _ = ...` to ignore the resulting value
    |
322 |     let _ = ui.register_command(
    |     +++++++
```

```text
warning: unused `CommandId` that must be used
   --> src/lib.rs:329:5
    |
329 | /     ui.register_command(
330 | |         "Move task down",
331 | |         CommandScope::Application,
332 | |         vec![KeyChord::new(CommandKey::Named(KeyCode::ArrowDown), alt)],
333 | |         move |_, _| down.move_selected(false),
334 | |     )?;
    | |______^
    |
help: use `let _ = ...` to ignore the resulting value
    |
329 |     let _ = ui.register_command(
    |     +++++++
```

```text
warning: unused `TaskReceipt` that must be used
   --> src/lib.rs:274:13
    |
274 | /             cx.defer(move |ui| {
275 | |                 let Some(list) = list else { return Ok(()) };
276 | |                 let Some(row) = ui.find_descendant(&list, &key)? else {
277 | |                     return Ok(());
...   |
287 | |                 Ok(())
288 | |             })?;
    | |_______________^
    |
    = note: retain the receipt if the application needs the operation's outcome
    = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
    |
274 |             let _ = cx.defer(move |ui| {
    |             +++++++
```

```text
error[E0308]: mismatched types
   --> tests/headless.rs:396:40
    |
396 |     let window = CaptureTarget::window(h.ui.primary_window());
    |                  --------------------- ^^^^^^^^^^^^^^^^^^^^^ expected `&NativeWindowId`, found `NativeWindowId`
    |                  |
    |                  arguments to this function are incorrect
    |
note: associated function defined here
   --> /kit/vendor/pert-ui-runtime-0.3.52/src/capture.rs:90:12
    |
 90 |     pub fn window(window: &crate::NativeWindowId) -> Self {
    |            ^^^^^^
help: consider borrowing here
    |
396 |     let window = CaptureTarget::window(&h.ui.primary_window());
    |                                        +
```
