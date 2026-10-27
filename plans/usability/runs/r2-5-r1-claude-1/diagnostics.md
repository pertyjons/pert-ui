# Distinct diagnostics

```text
error[E0615]: attempted to take value of method `x` on type `Rect`
  --> tests/scratch.rs:18:26
   |
18 |     let p = Point::new(b.x + b.width / 2.0, b.y + b.height / 2.0)?;
   |                          ^ method, not a field
   |
help: use parentheses to call the method
   |
18 |     let p = Point::new(b.x() + b.width / 2.0, b.y + b.height / 2.0)?;
   |                           ++
```

```text
error[E0615]: attempted to take value of method `width` on type `Rect`
  --> tests/scratch.rs:18:32
   |
18 |     let p = Point::new(b.x + b.width / 2.0, b.y + b.height / 2.0)?;
   |                                ^^^^^ method, not a field
   |
help: use parentheses to call the method
   |
18 |     let p = Point::new(b.x + b.width() / 2.0, b.y + b.height / 2.0)?;
   |                                     ++
```

```text
error[E0615]: attempted to take value of method `y` on type `Rect`
  --> tests/scratch.rs:18:47
   |
18 |     let p = Point::new(b.x + b.width / 2.0, b.y + b.height / 2.0)?;
   |                                               ^ method, not a field
   |
help: use parentheses to call the method
   |
18 |     let p = Point::new(b.x + b.width / 2.0, b.y() + b.height / 2.0)?;
   |                                                ++
```

```text
error[E0615]: attempted to take value of method `height` on type `Rect`
  --> tests/scratch.rs:18:53
   |
18 |     let p = Point::new(b.x + b.width / 2.0, b.y + b.height / 2.0)?;
   |                                                     ^^^^^^ method, not a field
   |
help: use parentheses to call the method
   |
18 |     let p = Point::new(b.x + b.width / 2.0, b.y + b.height() / 2.0)?;
   |                                                           ++
```

```text
error: unexpected closing delimiter: `}`
   --> tests/acceptance.rs:121:1
    |
 94 | fn a_failure_shows_in_its_row_and_as_a_notification() -> Fallible<()> {
    |                                                                       - this delimiter might not be properly closed...
...
113 |     }
    |     - ...as it matches this but it has different indentation
...
121 | }
    | ^ unexpected closing delimiter
```
