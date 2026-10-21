# Distinct diagnostics

```text
error[E0609]: no field `properties` on type `&ComponentInspection`
   --> tests/headless.rs:146:42
    |
146 |     let number = |key: &str| match entry.properties.get(key) {
    |                                          ^^^^^^^^^^ unknown field
    |
help: one of the expressions' fields has a field of the same name
    |
146 |     let number = |key: &str| match entry.details.properties.get(key) {
    |                                          ++++++++
```

```text
error[E0614]: type `f64` cannot be dereferenced
   --> tests/headless.rs:147:45
    |
147 |         Some(InspectValue::Number(n)) => Ok(*n),
    |                                             ^^ can't be dereferenced
```

```text
error[E0609]: no field `semantics` on type `&ComponentInspection`
   --> tests/headless.rs:150:28
    |
150 |     let text = match entry.semantics.as_ref().and_then(|s| s.value.clone()) {
    |                            ^^^^^^^^^ unknown field
    |
help: one of the expressions' fields has a field of the same name
    |
150 |     let text = match entry.details.semantics.as_ref().and_then(|s| s.value.clone()) {
    |                            ++++++++
```

```text
error[E0609]: no field `semantics` on type `&ComponentInspection`
   --> tests/headless.rs:195:23
    |
195 |     let shown = entry.semantics.as_ref().and_then(|s| s.value.clone());
    |                       ^^^^^^^^^ unknown field
    |
help: one of the expressions' fields has a field of the same name
    |
195 |     let shown = entry.details.semantics.as_ref().and_then(|s| s.value.clone());
    |                       ++++++++
```

```text
error[E0277]: the trait bound `DrawError: From<std::io::Error>` is not satisfied
  --> src/main.rs:21:40
   |
21 |             app::mount(ui).map_err(|e| AppError::from(std::io::Error::other(e.to_string())))?;
   |                                        ^^^^^^^^ the trait `From<std::io::Error>` is not implemented for `DrawError`
   |
   = help: `DrawError` implements trait `From<T>`:
             From<ApplicationError>
             From<BindingError>
             From<ChoiceError>
             From<ClipboardError>
             From<DeliveryError>
             From<EditError>
             From<FontError>
             From<GeometryError>
           and 10 others
   = note: required for `AppError` to implement `From<std::io::Error>`
```

```text
error[E0308]: mismatched types
  --> src/main.rs:53:84
   |
53 |         let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(ui.primary_window())));
   |                                                              --------------------- ^^^^^^^^^^^^^^^^^^^ expected `&NativeWindowId`, found `NativeWindowId`
   |                                                              |
   |                                                              arguments to this function are incorrect
   |
note: associated function defined here
  --> /kit/vendor/pert-ui-runtime-0.3.52/src/capture.rs:90:12
   |
90 |     pub fn window(window: &crate::NativeWindowId) -> Self {
   |            ^^^^^^
help: consider borrowing here
   |
53 |         let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(&ui.primary_window())));
   |                                                                                    +
```
