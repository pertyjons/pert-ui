# Distinct diagnostics

```text
warning: method `displayed` is never used
   --> src/main.rs:231:12
    |
188 |     impl Harness {
    |     ------------ method in this implementation
...
231 |         fn displayed(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    |            ^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```
