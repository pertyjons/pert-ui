# Distinct diagnostics

```text
error[E0308]: mismatched types
  --> src/headless.rs:88:82
   |
88 |         let pending=ui.request_capture(CaptureRequest::new(CaptureTarget::window(ui.primary_window())));
   |                                                            --------------------- ^^^^^^^^^^^^^^^^^^^ expected `&NativeWindowId`, found `NativeWindowId`
   |                                                            |
   |                                                            arguments to this function are incorrect
   |
note: associated function defined here
  --> /kit/vendor/pert-ui-runtime-0.3.52/src/capture.rs:90:12
   |
90 |     pub fn window(window: &crate::NativeWindowId) -> Self {
   |            ^^^^^^
help: consider borrowing here
   |
88 |         let pending=ui.request_capture(CaptureRequest::new(CaptureTarget::window(&ui.primary_window())));
   |                                                                                  +
```
