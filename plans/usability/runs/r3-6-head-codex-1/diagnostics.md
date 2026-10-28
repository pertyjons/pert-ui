# Distinct diagnostics

```text
error[E0599]: no associated function or constant named `ctrl` found for struct `pert_ui::KeyChord` in the current scope
   --> src/main.rs:137:89
    |
137 |     let new = ui.register_command("New note", CommandScope::Application, vec![KeyChord::ctrl('n')], move |_, _| e.new_note())?;
    |                                                                                         ^^^^ associated function or constant not found in `pert_ui::KeyChord`
    |
note: if you're trying to build a new `pert_ui::KeyChord` consider using one of the following associated functions:
      pert_ui::KeyChord::new
      pert_ui::KeyChord::named
      pert_ui::KeyChord::primary
      pert_ui::KeyChord::navigate_back
      pert_ui::KeyChord::navigate_forward
   --> /kit/vendor/pert-ui-runtime-0.3.88/src/runtime/commands.rs:67:5
    |
 67 |     pub fn new(key: CommandKey, modifiers: Modifiers) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
 86 |     pub fn named(key: KeyCode) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
 90 |     pub fn primary(letter: char) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
114 |     pub fn navigate_back() -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
120 |     pub fn navigate_forward() -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

```text
error[E0599]: no associated function or constant named `ctrl` found for struct `pert_ui::KeyChord` in the current scope
   --> src/main.rs:139:86
    |
139 |     let save = ui.register_command("Save", CommandScope::Application, vec![KeyChord::ctrl('s')], move |_, _| e.save())?;
    |                                                                                      ^^^^ associated function or constant not found in `pert_ui::KeyChord`
    |
note: if you're trying to build a new `pert_ui::KeyChord` consider using one of the following associated functions:
      pert_ui::KeyChord::new
      pert_ui::KeyChord::named
      pert_ui::KeyChord::primary
      pert_ui::KeyChord::navigate_back
      pert_ui::KeyChord::navigate_forward
   --> /kit/vendor/pert-ui-runtime-0.3.88/src/runtime/commands.rs:67:5
    |
 67 |     pub fn new(key: CommandKey, modifiers: Modifiers) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
 86 |     pub fn named(key: KeyCode) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
 90 |     pub fn primary(letter: char) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
114 |     pub fn navigate_back() -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
120 |     pub fn navigate_forward() -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

```text
error[E0599]: no associated function or constant named `ctrl` found for struct `pert_ui::KeyChord` in the current scope
   --> src/main.rs:141:92
    |
141 |     let close = ui.register_command("Close tab", CommandScope::Application, vec![KeyChord::ctrl('w')], move |_, _| e.close())?;
    |                                                                                            ^^^^ associated function or constant not found in `pert_ui::KeyChord`
    |
note: if you're trying to build a new `pert_ui::KeyChord` consider using one of the following associated functions:
      pert_ui::KeyChord::new
      pert_ui::KeyChord::named
      pert_ui::KeyChord::primary
      pert_ui::KeyChord::navigate_back
      pert_ui::KeyChord::navigate_forward
   --> /kit/vendor/pert-ui-runtime-0.3.88/src/runtime/commands.rs:67:5
    |
 67 |     pub fn new(key: CommandKey, modifiers: Modifiers) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
 86 |     pub fn named(key: KeyCode) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
 90 |     pub fn primary(letter: char) -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
114 |     pub fn navigate_back() -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
120 |     pub fn navigate_forward() -> Self {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

```text
error[E0433]: cannot find type `SplitPane` in this scope
   --> src/main.rs:163:9
    |
163 |         SplitPane::new(SplitAxis::LeftRight, sidebar, tabs).sizing(SplitSizing::first(Length::from(220))).key("split").with_grow(1),
    |         ^^^^^^^^^ use of undeclared type `SplitPane`
```

```text
error[E0433]: cannot find type `SplitAxis` in this scope
   --> src/main.rs:163:24
    |
163 |         SplitPane::new(SplitAxis::LeftRight, sidebar, tabs).sizing(SplitSizing::first(Length::from(220))).key("split").with_grow(1),
    |                        ^^^^^^^^^ use of undeclared type `SplitAxis`
```

```text
error[E0433]: cannot find type `SplitSizing` in this scope
   --> src/main.rs:163:68
    |
163 |         SplitPane::new(SplitAxis::LeftRight, sidebar, tabs).sizing(SplitSizing::first(Length::from(220))).key("split").with_grow(1),
    |                                                                    ^^^^^^^^^^^ use of undeclared type `SplitSizing`
```

```text
error[E0308]: mismatched types
   --> src/main.rs:207:33
    |
207 |     App::new("Notes", move |ui| build(ui, folder))
    |                                 ^^^^^^^^^^^^^^^^^ expected `Result<_, AppError>`, found `Result<Editor, RuntimeError>`
    |
    = note: expected enum `Result<_, AppError>`
               found enum `Result<Editor, pert_ui::RuntimeError>`
help: use `?` to coerce and return an appropriate `Err`, and wrap the resulting value in `Ok` so the expression remains of type `Result`
    |
207 |     App::new("Notes", move |ui| Ok(build(ui, folder)?))
    |                                 +++                 ++
```
