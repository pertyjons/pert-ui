# Distinct diagnostics

```text
error[E0433]: cannot find type `ApplicationError` in this scope
  --> src/picker.rs:43:24
   |
43 |             return Err(ApplicationError::from("point coordinates must be finite and in 0..=1").into());
   |                        ^^^^^^^^^^^^^^^^ use of undeclared type `ApplicationError`
```

```text
error[E0271]: expected `mount` to return `Result<_, AppError>`, but it returns `Result<State, RuntimeError>`
   --> src/lib.rs:35:55
    |
 35 | pub fn app() -> App<State> { App::new("Point picker", mount) }
    |                              --------                 ^^^^^ expected `Result<State, AppError>`, found `Result<State, RuntimeError>`
    |                              |
    |                              required by a bound introduced by this call
    |
    = note: expected enum `Result<_, AppError>`
               found enum `Result<_, pert_ui::RuntimeError>`
note: required by a bound in `pert_ui::App::<S>::new`
   --> /kit/vendor/pert-ui-0.3.88/src/app.rs:596:50
    |
594 |     pub fn new(
    |            --- required by a bound in this associated function
595 |         title: impl Into<String>,
596 |         initialize: impl FnOnce(&mut Runtime) -> Result<S, AppError> + 'static,
    |                                                  ^^^^^^^^^^^^^^^^^^^ required by this bound in `App::<S>::new`
```
