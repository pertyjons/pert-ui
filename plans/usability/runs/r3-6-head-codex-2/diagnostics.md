# Distinct diagnostics

```text
error[E0433]: cannot find type `SplitPane` in this scope
   --> src/lib.rs:184:13
    |
184 |             SplitPane::new(SplitAxis::LeftRight, list.key("files"), tabs.key("tabs"))
    |             ^^^^^^^^^ use of undeclared type `SplitPane`
```

```text
error[E0433]: cannot find type `SplitAxis` in this scope
   --> src/lib.rs:184:28
    |
184 |             SplitPane::new(SplitAxis::LeftRight, list.key("files"), tabs.key("tabs"))
    |                            ^^^^^^^^^ use of undeclared type `SplitAxis`
```

```text
error[E0433]: cannot find type `SplitSizing` in this scope
   --> src/lib.rs:185:25
    |
185 |                 .sizing(SplitSizing::first(Length::from(220))).key("split").with_grow(1),
    |                         ^^^^^^^^^^^ use of undeclared type `SplitSizing`
```
