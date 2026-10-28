# Distinct diagnostics

```text
error[E0308]: mismatched types
  --> src/main.rs:83:25
   |
83 |         if valid_prefix(s) { Ok(s.to_owned()) }
   |            ------------ ^ expected `&str`, found `String`
   |            |
   |            arguments to this function are incorrect
   |
note: function defined here
  --> src/main.rs:22:4
   |
22 | fn valid_prefix(s: &str) -> bool {
   |    ^^^^^^^^^^^^ -------
help: consider borrowing here
   |
83 |         if valid_prefix(&s) { Ok(s.to_owned()) }
   |                         +
```

```text
error[E0277]: the trait bound `Format: Eq` is not satisfied
    --> src/main.rs:132:24
     |
 132 |               Field::new(SegmentedControl::new(&v.format, [
     |  ________________________^
 133 | |                 Choice::new(Format::PNG, "PNG"), Choice::new(Format::JPEG, "JPEG"), Choice::new(Format::WebP, "WebP"),
 134 | |             ])?.key("input")).label("Format").key("format"),
     | |______________^ the trait `Eq` is not implemented for `Format`
     |
note: required by a bound in `pert_ui::SegmentedControl::<T>::new`
    --> /kit/vendor/pert-ui-runtime-0.3.88/src/choice_controls.rs:1558:17
     |
1558 | impl<T: Clone + Eq + 'static> SegmentedControl<T> {
     |                 ^^ required by this bound in `SegmentedControl::<T>::new`
...
1564 |     pub fn new(
     |            --- required by a bound in this associated function
help: consider annotating `Format` with `#[derive(Eq)]`
     |
   6 + #[derive(Eq)]
   7 | enum Format { PNG, JPEG, WebP }
     |
```

```text
error[E0599]: `pert_ui::SegmentedControl<Format>` is not a pert-ui component
    --> src/main.rs:134:17
     |
 132 |               Field::new(SegmentedControl::new(&v.format, [
     |  ________________________-
 133 | |                 Choice::new(Format::PNG, "PNG"), Choice::new(Format::JPEG, "JPEG"), Choice::new(Format::WebP, "WebP"),
 134 | |             ])?.key("input")).label("Format").key("format"),
     | |                -^^^ `pert_ui::SegmentedControl<Format>` does not implement `Component`
     | |________________|
     |
     |
    ::: /kit/vendor/pert-ui-runtime-0.3.88/src/choice_controls.rs:1556:1
     |
1556 |   pub struct SegmentedControl<T>(RadioGroup<T>);
     |   ------------------------------ doesn't satisfy `_: Component` or `_: IntoElement`
     |
     = note: the following trait bounds were not satisfied:
             `pert_ui::SegmentedControl<Format>: pert_ui::Component`
             which is required by `pert_ui::SegmentedControl<Format>: pert_ui::IntoElement`
             `&pert_ui::SegmentedControl<Format>: pert_ui::Component`
             which is required by `&pert_ui::SegmentedControl<Format>: pert_ui::IntoElement`
             `&mut pert_ui::SegmentedControl<Format>: pert_ui::Component`
             which is required by `&mut pert_ui::SegmentedControl<Format>: pert_ui::IntoElement`
     = note: a component type implements `Component`, where only `update` is required, and owns its data (`'static`)
     = note: if this value is an `Element`, from `.key(..)` or `Element::new`, it is already described: pass it as a child instead of describing it again
```

```text
error[E0283]: type annotations needed for `pert_ui::ReadSignal<_>`
   --> src/main.rs:80:9
    |
 80 |     let marker = dirty.map(|d| if *d { "Unsaved changes".into() } else { "All changes saved".into() })?;
    |         ^^^^^^         --- type must be known at this point
    |
    = note: the type must implement `Clone`
    = note: the type must also implement `PartialEq`
note: required by a bound in `pert_ui::ReadSignal::<T>::map`
   --> /kit/vendor/pert-ui-runtime-0.3.88/src/signal.rs:850:19
    |
850 |     pub fn map<U: Clone + PartialEq + 'static>(
    |                   ^^^^^ required by this bound in `ReadSignal::<T>::map`
note: required by a bound in `pert_ui::ReadSignal::<T>::map`
   --> /kit/vendor/pert-ui-runtime-0.3.88/src/signal.rs:850:27
    |
850 |     pub fn map<U: Clone + PartialEq + 'static>(
    |                           ^^^^^^^^^ required by this bound in `ReadSignal::<T>::map`
help: consider giving `marker` an explicit type, where the type for type parameter `U` is specified
    |
 80 |     let marker: pert_ui::ReadSignal<U> = dirty.map(|d| if *d { "Unsaved changes".into() } else { "All changes saved".into() })?;
    |               ++++++++++++++++++++++++
```

```text
warning: fields `root`, `values`, and `applied` are never read
  --> src/main.rs:51:5
   |
50 | struct Editor {
   |        ------ fields in this struct
51 |     root: WidgetId,
   |     ^^^^
52 |     values: Values,
   |     ^^^^^^
53 |     applied: Signal<Settings>,
   |     ^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```
