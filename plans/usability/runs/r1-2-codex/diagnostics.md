# Distinct diagnostics

```text
warning: unused import: `CommandKeyInput`
 --> src/main.rs:2:175
  |
2 | ...ion, ListSource, SelectionMode, VirtualList, CommandKey, CommandKeyInput, CommandScope, KeyChord, Modifiers};
  |                                                             ^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
error[E0616]: field `kind` of struct `pert_ui::PointerEvent` is private
  --> src/main.rs:55:27
   |
55 |         if matches!(event.kind, PointerKind::Press(PointerButton::Primary)) {
   |                           ^^^^ private field
   |
help: a method `kind` also exists, call it with parentheses
   |
55 |         if matches!(event.kind(), PointerKind::Press(PointerButton::Primary)) {
   |                               ++
```

```text
error[E0308]: mismatched types
   --> src/main.rs:107:128
    |
107 | ...aft).key("editor")] }.key("title").with_grow(1.0).ok()?,
    |                                       --------- ^^^ expected `u16`, found floating-point number
    |                                       |
    |                                       arguments to this method are incorrect
    |
note: method defined here
   --> /kit/vendor/pert-ui-runtime-0.3.52/src/layout.rs:587:12
    |
587 |     pub fn with_grow(mut self, weight: u16) -> Self {
    |            ^^^^^^^^^
```

```text
error[E0599]: no method named `ok` found for struct `pert_ui::Element` in the current scope
   --> src/main.rs:107:133
    |
106 | / ...   Title { model: self.model.clone(), id, draft: draft.clone(), editor: None,
107 | | ...       children: vec![Label::new(title).key("label"), DraftEditor::new(draft).key("editor")] }.key("title").with_grow(1.0).ok()?,
    | |                                                                                                                              -^^ method not found in `pert_ui::Element`
    | |______________________________________________________________________________________________________________________________|
    |
```

```text
error[E0308]: mismatched types
   --> src/main.rs:131:133
    |
131 | ...me("Search tasks").key("filtered").with_grow(1.0)?,
    |                                       --------- ^^^ expected `u16`, found floating-point number
    |                                       |
    |                                       arguments to this method are incorrect
    |
note: method defined here
   --> /kit/vendor/pert-ui-runtime-0.3.52/src/layout.rs:587:12
    |
587 |     pub fn with_grow(mut self, weight: u16) -> Self {
    |            ^^^^^^^^^
```

```text
error[E0277]: the `?` operator can only be applied to values that implement `Try`
   --> src/main.rs:131:9
    |
131 | ...   CollectionFilter::with_query(&model.query, filter.clone(), list.key("list")).name("Search tasks").key("filtered").with_grow(1.0)?,
    |       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the `?` operator cannot be applied to type `pert_ui::Element`
    |
    = help: the nightly-only, unstable trait `Try` is not implemented for `pert_ui::Element`
```

```text
error[E0599]: no associated function or constant named `CONTROL` found for struct `Modifiers` in the current scope
   --> src/main.rs:138:122
    |
138 | ...Key::Character('n'), Modifiers::CONTROL)], move |ui, _| ui.focus(&target))?;
    |                                    ^^^^^^^ associated function or constant not found in `Modifiers`
```

```text
error[E0599]: no associated function or constant named `ALT` found for struct `Modifiers` in the current scope
   --> src/main.rs:143:135
    |
143 | ...ommandKey::Named(key), Modifiers::ALT)], move |_, _| m.shift(delta))?;
    |                                      ^^^ associated function or constant not found in `Modifiers`
```

```text
warning: fields `model`, `root`, `new`, `list`, `filter`, and `status` are never read
   --> src/main.rs:111:18
    |
111 | struct Handles { model: Model, root: WidgetId, new: WidgetId, list: WidgetId, filter: ListFilter, status: ReadSignal<String> }
    |        -------   ^^^^^         ^^^^            ^^^            ^^^^            ^^^^^^              ^^^^^^
    |        |
    |        fields in this struct
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: unused `CommandId` that must be used
   --> src/main.rs:138:5
    |
138 |     ui.register_command("New task", CommandScope::Application, vec![KeyChord::new(CommandKey::Character('n'), Modifiers { control: true, ..Modifiers::default() })], move |ui, _| ui.focus(&target))?;
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
    |
138 |     let _ = ui.register_command("New task", CommandScope::Application, vec![KeyChord::new(CommandKey::Character('n'), Modifiers { control: true, ..Modifiers::default() })], move |ui, _| ui.focus(&target))?;
    |     +++++++
```

```text
warning: unused `CommandId` that must be used
   --> src/main.rs:140:5
    |
140 |     ui.register_command("Remove selected task", CommandScope::Application, vec![KeyChord::named(KeyCode::Delete)], move |_, _| m.remove())?;
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
help: use `let _ = ...` to ignore the resulting value
    |
140 |     let _ = ui.register_command("Remove selected task", CommandScope::Application, vec![KeyChord::named(KeyCode::Delete)], move |_, _| m.remove())?;
    |     +++++++
```

```text
warning: unused `CommandId` that must be used
   --> src/main.rs:143:9
    |
143 | ...   ui.register_command(format!("Move {delta}"), CommandScope::Application, vec![KeyChord::new(CommandKey::Named(key), Modifiers { alt: true, ..Modifiers::default() })], move |_, _| m.shift(delta))?;
    |       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
help: use `let _ = ...` to ignore the resulting value
    |
143 |         let _ = ui.register_command(format!("Move {delta}"), CommandScope::Application, vec![KeyChord::new(CommandKey::Named(key), Modifiers { alt: true, ..Modifiers::default() })], move |_, _| m.shift(delta))?;
    |         +++++++
```
