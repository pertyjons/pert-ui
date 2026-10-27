# Distinct diagnostics

```text
error[E0425]: cannot find function `headless` in this scope
   --> src/main.rs:102:51
    |
102 |  if std::env::args().any(|arg|arg=="--headless") {headless()?;} else {App::new("Background job monitor",|ui|{mount(ui,false)?;Ok(()...
    |                                                   ^^^^^^^^ not found in this scope
```

```text
error[E0599]: no variant, associated function, or constant named `Revealed` found for enum `pert_ui::ListRevealStatus` in the current scope
   --> src/main.rs:145:50
    |
145 |  let row_id=match row.status(){ListRevealStatus::Revealed(id)=>id,_=>return Err("cancel row was not revealed".into())};
    |                                                  ^^^^^^^^ variant, associated function, or constant not found in `pert_ui::ListRevealStatus`
```

```text
error[E0599]: no associated function or constant named `ALT` found for struct `pert_ui::Modifiers` in the current scope
   --> src/main.rs:162:103
    |
162 |    controller.set_active_column(Some(column.into()))?;ui.focus(&body)?;ui.native_modifiers(Modifiers::ALT)?;
    |                                                                                                       ^^^ associated function or constant not found in `pert_ui::Modifiers`
```

```text
error[E0308]: mismatched types
   --> src/main.rs:174:75
    |
174 |  let pending=ui.request_capture(CaptureRequest::new(CaptureTarget::window(ui.primary_window())));
    |                                                     --------------------- ^^^^^^^^^^^^^^^^^^^ expected `&NativeWindowId`, found `NativeWindowId`
    |                                                     |
    |                                                     arguments to this function are incorrect
    |
note: associated function defined here
   --> /kit/vendor/pert-ui-runtime-0.3.52/src/capture.rs:90:12
    |
 90 |     pub fn window(window: &crate::NativeWindowId) -> Self {
    |            ^^^^^^
help: consider borrowing here
    |
174 |  let pending=ui.request_capture(CaptureRequest::new(CaptureTarget::window(&ui.primary_window())));
    |                                                                           +
```

```text
error[E0599]: no variant, associated function, or constant named `Revealed` found for enum `pert_ui::ListRevealStatus` in the current scope
   --> src/main.rs:178:50
    |
178 |  let row_id=match row.status(){ListRevealStatus::Revealed(id)=>id,_=>return Err("last row missing".into())};
    |                                                  ^^^^^^^^ variant, associated function, or constant not found in `pert_ui::ListRevealStatus`
```

```text
error[E0277]: the `?` operator can only be applied to values that implement `Try`
   --> src/main.rs:183:2
    |
183 |  ui.clear_focus()?;ui.flush(Duration::from_secs(10))?;ui.paint()?;
    |  ^^^^^^^^^^^^^^^^^ the `?` operator cannot be applied to type `()`
    |
    = help: the nightly-only, unstable trait `Try` is not implemented for `()`
```
