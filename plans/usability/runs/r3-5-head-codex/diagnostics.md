# Distinct diagnostics

```text
error[E0308]: mismatched types
   --> src/lib.rs:143:72
    |
143 | pub fn app() -> App<Monitor> { App::new("Background job monitor", |ui| mount(ui, vec![])) }
    |                                                                        ^^^^^^^^^^^^^^^^^ expected `Result<_, AppError>`, found `Result<Monitor, RuntimeError>`
    |
    = note: expected enum `Result<_, AppError>`
               found enum `Result<Monitor, pert_ui::RuntimeError>`
help: use `?` to coerce and return an appropriate `Err`, and wrap the resulting value in `Ok` so the expression remains of type `Result`
    |
143 | pub fn app() -> App<Monitor> { App::new("Background job monitor", |ui| Ok(mount(ui, vec![])?)) }
    |                                                                        +++                 ++
```
