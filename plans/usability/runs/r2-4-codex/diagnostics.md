# Distinct diagnostics

```text
error[E0433]: cannot find type `SemanticRole` in this scope
  --> tests/ui.rs:95:36
   |
95 |         assert_eq!(semantics.role, SemanticRole::Button);
   |                                    ^^^^^^^^^^^^ use of undeclared type `SemanticRole`
   |
help: an enum with a similar name exists
   |
95 -         assert_eq!(semantics.role, SemanticRole::Button);
95 +         assert_eq!(semantics.role, SemanticValue::Button);
   |
```

```text
error[E0277]: the `?` operator can only be applied to values that implement `Try`
   --> tests/ui.rs:225:13
    |
225 |             driver.runtime_mut().clear_focus()?;
    |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the `?` operator cannot be applied to type `()`
    |
    = help: the nightly-only, unstable trait `Try` is not implemented for `()`
```
