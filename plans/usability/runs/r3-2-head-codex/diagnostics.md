# Distinct diagnostics

```text
error[E0271]: expected `build` to return `Result<_, AppError>`, but it returns `Result<TaskList, RuntimeError>`
   --> src/main.rs:141:51
    |
141 | fn app() -> App<TaskList> { App::new("Task list", build) }
    |                             --------              ^^^^^ expected `Result<TaskList, AppError>`, found `Result<TaskList, RuntimeError>`
    |                             |
    |                             required by a bound introduced by this call
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

```text
warning: fields `tasks`, `root`, and `selection` are never read
   --> src/main.rs:128:5
    |
127 | struct TaskList {
    |        -------- fields in this struct
128 |     tasks: Signal<Vec<Task>>,
    |     ^^^^^
129 |     root: WidgetId,
    |     ^^^^
130 |     selection: ListSelection,
    |     ^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```
