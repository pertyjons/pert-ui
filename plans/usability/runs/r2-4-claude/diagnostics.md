# Distinct diagnostics

```text
warning: unused variable: `id`
   --> src/lib.rs:108:24
    |
108 |             let (list, id) = (remove_from.clone(), bookmark.id);
    |                        ^^ help: if this is intentional, prefix it with an underscore: `_id`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```
