# Distinct diagnostics

```text
error[E0433]: cannot find type `ApplicationError` in this scope
   --> src/picker.rs:112:24
    |
112 |             return Err(ApplicationError::from("a point picker's x and y lie in 0..=1").into());
    |                        ^^^^^^^^^^^^^^^^ use of undeclared type `ApplicationError`
```

```text
error[E0433]: cannot find type `Viewport` in this scope
  --> src/headless.rs:18:21
   |
18 |     ui.set_viewport(Viewport::new(SIZE.0, SIZE.1, 1.0)?)?;
   |                     ^^^^^^^^ use of undeclared type `Viewport`
```

```text
error[E0277]: `?` couldn't convert the error to `pert_ui::RuntimeError`
  --> src/app.rs:52:87
   |
19 | pub fn build(ui: &mut Runtime) -> Result<PickerApp, RuntimeError> {
   |                                   ------------------------------- expected `pert_ui::RuntimeError` because of this
...
52 |     let picker = ui.find(&root, &["layout", "picker"])?.ok_or("the picker is mounted")?;
   |                                                         ------------------------------^ the trait `From<&str>` is not implemented for `pert_ui::RuntimeError`
   |                                                         |
   |                                                         this can't be annotated with `?` because it has type `Result<_, &str>`
   |
   = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
   = help: `pert_ui::RuntimeError` implements trait `From<T>`:
             From<ApplicationError>
             From<BindingError>
             From<CaptureError>
             From<ChoiceError>
             From<ClipboardError>
             From<DeliveryError>
             From<EditError>
             From<FontError>
           and 11 others
```

```text
error[E0433]: cannot find type `DrawCommand` in this scope
   --> tests/picker.rs:229:49
    |
229 |             .filter(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == ring))
    |                                                 ^^^^^^^^^^^ use of undeclared type `DrawCommand`
```

```text
warning: this `if` statement can be collapsed
  --> src/picker.rs:67:9
   |
67 | /         if changed_x || changed_y {
68 | |             if let Some(on_change) = self.on_change.as_mut() {
69 | |                 on_change(cx, point)?;
70 | |             }
71 | |         }
   | |_________^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.99.0/index.html#collapsible_if
   = note: `#[warn(clippy::collapsible_if)]` on by default
help: collapse nested if block
   |
67 ~         if (changed_x || changed_y) {
68 ~             && let Some(on_change) = self.on_change.as_mut() {
69 |                 on_change(cx, point)?;
70 ~             }
   |
```
