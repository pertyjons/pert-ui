# Distinct diagnostics

```text
error[E0599]: no variant, associated function, or constant named `Title` found for enum `pert_ui::TextStyle` in the current scope
   --> src/lib.rs:294:56
    |
294 |         Label::new("Export settings").style(TextStyle::Title).key("title"),
    |                                                        ^^^^^ variant, associated function, or constant not found in `pert_ui::TextStyle`
```

```text
warning: unused import: `pert_ui::prelude::*`
 --> tests/explore.rs:1:5
  |
1 | use pert_ui::prelude::*;
  |     ^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: unused import: `pert_ui::prelude`
 --> tests/explore.rs:1:5
  |
1 | use pert_ui::prelude::*;
  |     ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error[E0609]: no field `type_name` on type `&ComponentInspection`
  --> tests/explore.rs:39:109
   |
39 | ...   println!("{}{:?} type={:?} sem={:?} props={:?}", "  ".repeat(depth), d.runtime().key(&c)?, comp.type_name, comp.details.semant...
   |                                                                                                       ^^^^^^^^^ unknown field
   |
   = note: available fields are: `id`, `key`, `path`, `parent`, `presentation_parent` ... and 9 others
```

```text
error[E0618]: expected function, found `pert_ui::testing::HeadlessApp<Editor>`
   --> tests/acceptance.rs:299:19
    |
276 |     let (path, mut app) = start(&dir, &sample())?;
    |                ------- `app` has type `pert_ui::testing::HeadlessApp<Editor>`
...
299 |     let mut app = app(path).headless()?;
    |                   ^^^------
    |                   |
    |                   call expression requires function
```

```text
error[E0599]: no method named `padding` found for struct `pert_ui::Column` in the current scope
   --> src/lib.rs:309:33
    |
309 |         ScrollArea::new(content.padding(Spacing::Medium).key("content")).key("settings"),
    |                                 ^^^^^^^
    |
help: there is a method `paint` with a similar name
    |
309 -         ScrollArea::new(content.padding(Spacing::Medium).key("content")).key("settings"),
309 +         ScrollArea::new(content.paint(Spacing::Medium).key("content")).key("settings"),
    |
```

```text
error[E0599]: no variant, associated function, or constant named `Medium` found for enum `pert_ui::Spacing` in the current scope
   --> src/lib.rs:309:50
    |
309 |         ScrollArea::new(content.padding(Spacing::Medium).key("content")).key("settings"),
    |                                                  ^^^^^^ variant, associated function, or constant not found in `pert_ui::Spacing`
```
