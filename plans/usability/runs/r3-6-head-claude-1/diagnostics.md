# Distinct diagnostics

```text
error[E0308]: mismatched types
   --> src/lib.rs:139:5
    |
138 | pub fn app(dir: PathBuf) -> App<Notes> {
    |                             ---------- expected `pert_ui::App<Notes>` because of return type
139 |     App::new("Notes", move |ui| Ok(build(ui, dir)?)).size(Size::new(900.0, 600.0).expect("a size"))
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `App<Notes>`, found `Result<App<Notes>, RuntimeError>`
    |
    = note: expected struct `App<_>`
                 found enum `Result<App<_>, pert_ui::RuntimeError>`
help: consider using `Result::expect` to unwrap the `Result<pert_ui::App<Notes>, pert_ui::RuntimeError>` value, panicking if the value is a `Result::Err`
    |
139 |     App::new("Notes", move |ui| Ok(build(ui, dir)?)).size(Size::new(900.0, 600.0).expect("a size")).expect("REASON")
    |                                                                                                    +++++++++++++++++
```

```text
error[E0308]: mismatched types
   --> src/lib.rs:182:68
    |
182 |         .map_err(|e| RuntimeError::from(pert_ui::LayoutError::from(e)))?
    |                                         -------------------------- ^ expected `LayoutError`, found `GeometryError`
    |                                         |
    |                                         arguments to this function are incorrect
    |
note: associated function defined here
   --> /opt/rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/convert/mod.rs:595:8
    |
595 |     fn from(value: T) -> Self;
    |        ^^^^
```

```text
error[E0433]: cannot find type `SplitPane` in this scope
   --> src/lib.rs:208:9
    |
208 |         SplitPane::new(
    |         ^^^^^^^^^ use of undeclared type `SplitPane`
```

```text
error[E0433]: cannot find type `SplitAxis` in this scope
   --> src/lib.rs:209:13
    |
209 |             SplitAxis::LeftRight,
    |             ^^^^^^^^^ use of undeclared type `SplitAxis`
```

```text
error[E0277]: the trait bound `pert_ui::Length: From<u32>` is not satisfied
   --> src/lib.rs:213:45
    |
213 |         .sizing(pert_ui::SplitSizing::first(Length::from(SIDEBAR_WIDTH)))
    |                                             ^^^^^^ the trait `From<u32>` is not implemented for `pert_ui::Length`
    |
help: the trait `From<u32>` is not implemented for `pert_ui::Length`
      but trait `From<u16>` is implemented for it
   --> /kit/vendor/pert-ui-runtime-0.3.88/src/containers.rs:13:1
    |
 13 | impl From<u16> for Length {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^
    = help: for that trait implementation, expected `u16`, found `u32`
```

```text
warning: unused import: `pert_ui::prelude::*`
 --> tests/probe.rs:1:5
  |
1 | use pert_ui::prelude::*;
  |     ^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error: casting `&T` to `&mut T` is undefined behavior, even if the reference is unused, consider instead using an `UnsafeCell`
  --> tests/probe.rs:17:38
   |
17 |     let mut d = Driver::new(unsafe { &mut *(ui as *const Runtime as *mut Runtime) });
   |                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: for more information, visit <https://doc.rust-lang.org/book/ch15-05-interior-mutability.html>
   = note: `#[deny(invalid_reference_casting)]` on by default
```

```text
warning: unused import: `pert_ui::DialogStatus`
 --> tests/editor.rs:6:5
  |
6 | use pert_ui::DialogStatus;
  |     ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: function `toasts` is never used
  --> tests/editor.rs:66:4
   |
66 | fn toasts(driver: &Driver<'_>, notes: &Notes) -> Result<Vec<String>, TestError> {
   |    ^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: function `read` is never used
  --> tests/editor.rs:84:4
   |
84 | fn read(dir: &Path, name: &str) -> String {
   |    ^^^^
```

```text
warning: function `menu` is never used
  --> tests/editor.rs:89:4
   |
89 | fn menu(driver: &mut Driver<'_>, notes: &Notes, index: usize) -> Result<(), TestError> {
   |    ^^^^
```

```text
error[E0502]: cannot borrow `driver` as immutable because it is also borrowed as mutable
   --> tests/editor.rs:358:45
    |
358 |     driver.runtime_mut().reveal_child(&tabs(&driver, &notes)?, "alpha.txt")?;
    |     ------               ------------       ^^^^^^^ immutable borrow occurs here
    |     |                    |
    |     |                    mutable borrow later used by call
    |     mutable borrow occurs here
```

```text
error[E0618]: expected function, found `pert_ui::WidgetId`
   --> tests/editor.rs:157:20
    |
 29 | fn list(driver: &Driver<'_>, notes: &Notes) -> Result<WidgetId, TestError> {
    | -------------------------------------------------------------------------- this function of the same name is available here, but it's shadowed by the local binding
...
138 |     let list = list(&driver, &notes)?;
    |         ---- `list` has type `pert_ui::WidgetId`
...
157 |         let list = list(driver, &notes)?;
    |                    ^^^^----------------
    |                    |
    |                    call expression requires function
```
