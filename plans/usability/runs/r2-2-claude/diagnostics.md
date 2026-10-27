# Distinct diagnostics

```text
warning: function `model_titles` is never used
  --> tests/headless.rs:43:4
   |
43 | fn model_titles(app: &TodoApp) -> Result<Vec<String>, Box<dyn std::error::Error>> {
   |    ^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: function `row_title` is never used
  --> tests/headless.rs:48:4
   |
48 | fn row_title(driver: &Driver, app: &TodoApp, title: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
   |    ^^^^^^^^^
```

```text
warning: function `single_click` is never used
  --> tests/headless.rs:55:4
   |
55 | fn single_click(driver: &mut Driver, id: &WidgetId) -> TestResult {
   |    ^^^^^^^^^^^^
```
