# Distinct diagnostics

```text
error[E0277]: `?` couldn't convert the error to `pert_ui::RuntimeError`
   --> src/main.rs:192:28
    |
192 |             self.ui.paint()?;
    |                     -------^ the trait `From<DrawError>` is not implemented for `pert_ui::RuntimeError`
    |                     |
    |                     this can't be annotated with `?` because it has type `Result<_, DrawError>`
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
