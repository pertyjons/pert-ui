# Distinct diagnostics

```text
error[E0425]: cannot find type `UiSender` in this scope
   --> src/lib.rs:206:13
    |
206 |     sender: UiSender<Report>,
    |             ^^^^^^^^ not found in this scope
```

```text
error[E0433]: cannot find type `SendError` in this scope
   --> src/lib.rs:217:21
    |
217 |                 Err(SendError::Full(back)) => {
    |                     ^^^^^^^^^ use of undeclared type `SendError`
    |
help: consider importing this struct
    |
  7 + use std::sync::mpsc::SendError;
    |
```

```text
error[E0425]: cannot find type `UiSender` in this scope
   --> src/lib.rs:349:13
    |
349 |     sender: UiSender<Report>,
    |             ^^^^^^^^ not found in this scope
```

```text
error[E0425]: cannot find type `Mailbox` in this scope
   --> src/lib.rs:494:15
    |
494 |     _mailbox: Mailbox<Report>,
    |               ^^^^^^^ not found in this scope
```

```text
error[E0433]: cannot find type `Capacity` in this scope
   --> src/lib.rs:527:30
    |
527 |     let mailbox = ui.mailbox(Capacity::new(4096)?, move |report: Report| {
    |                              ^^^^^^^^ use of undeclared type `Capacity`
```

```text
error[E0277]: the trait bound `String: std::error::Error` is not satisfied
   --> src/lib.rs:582:69
    |
582 |             .ok_or_else(|| RuntimeError::from(ApplicationError::new(format!("missing {path:?}"))))
    |                                               --------------------- ^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `std::error::Error` is not implemented for `String`
    |                                               |
    |                                               required by a bound introduced by this call
    |
note: required by a bound in `pert_ui::ApplicationError::new`
   --> /kit/vendor/pert-ui-runtime-0.3.88/src/error.rs:59:28
    |
 59 |     pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
    |                            ^^^^^ required by this bound in `ApplicationError::new`
```
