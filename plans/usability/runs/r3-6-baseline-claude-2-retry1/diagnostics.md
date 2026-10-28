# Distinct diagnostics

```text
error: unexpected closing delimiter: `}`
   --> tests/acceptance.rs:404:1
    |
312 | fn scratch_novis() -> Result {
    |                              - this opening brace...
...
354 | }
    | - ...matches this closing brace
...
404 | }
    | ^ unexpected closing delimiter
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
   --> tests/bisect.rs:344:1
    |
195 | fn scratch_novis() -> Result {
    | ---------------------------- previous definition of the value `scratch_novis` here
...
344 | fn scratch_novis() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
    |
    = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
   --> tests/bisect.rs:390:1
    |
241 | fn scratch_nost() -> Result {
    | --------------------------- previous definition of the value `scratch_nost` here
...
390 | fn scratch_nost() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
    |
    = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
   --> tests/bisect.rs:493:1
    |
195 | fn scratch_novis() -> Result {
    | ---------------------------- previous definition of the value `scratch_novis` here
...
493 | fn scratch_novis() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
    |
    = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
   --> tests/bisect.rs:539:1
    |
241 | fn scratch_nost() -> Result {
    | --------------------------- previous definition of the value `scratch_nost` here
...
539 | fn scratch_nost() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
    |
    = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
   --> tests/bisect.rs:642:1
    |
195 | fn scratch_novis() -> Result {
    | ---------------------------- previous definition of the value `scratch_novis` here
...
642 | fn scratch_novis() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
    |
    = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
   --> tests/bisect.rs:688:1
    |
241 | fn scratch_nost() -> Result {
    | --------------------------- previous definition of the value `scratch_nost` here
...
688 | fn scratch_nost() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
    |
    = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
   --> tests/bisect.rs:791:1
    |
195 | fn scratch_novis() -> Result {
    | ---------------------------- previous definition of the value `scratch_novis` here
...
791 | fn scratch_novis() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
    |
    = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
   --> tests/bisect.rs:837:1
    |
241 | fn scratch_nost() -> Result {
    | --------------------------- previous definition of the value `scratch_nost` here
...
837 | fn scratch_nost() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
    |
    = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
   --> tests/bisect.rs:940:1
    |
195 | fn scratch_novis() -> Result {
    | ---------------------------- previous definition of the value `scratch_novis` here
...
940 | fn scratch_novis() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
    |
    = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
   --> tests/bisect.rs:986:1
    |
241 | fn scratch_nost() -> Result {
    | --------------------------- previous definition of the value `scratch_nost` here
...
986 | fn scratch_nost() -> Result {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
    |
    = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:1089:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
1089 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:1135:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
1135 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:1238:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
1238 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:1284:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
1284 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:1387:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
1387 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:1433:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
1433 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:1536:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
1536 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:1582:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
1582 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:1685:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
1685 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:1731:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
1731 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:1834:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
1834 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:1880:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
1880 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:1983:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
1983 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:2029:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
2029 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:2132:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
2132 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:2178:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
2178 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:2281:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
2281 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:2327:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
2327 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:2430:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
2430 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:2476:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
2476 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:2579:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
2579 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:2625:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
2625 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:2728:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
2728 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:2774:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
2774 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:2877:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
2877 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:2923:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
2923 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:3026:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
3026 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:3072:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
3072 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:3175:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
3175 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:3221:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
3221 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:3324:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
3324 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:3370:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
3370 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:3473:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
3473 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:3519:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
3519 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:3622:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
3622 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:3668:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
3668 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:3771:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
3771 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:3817:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
3817 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:3920:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
3920 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:3966:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
3966 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:4069:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
4069 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:4115:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
4115 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:4218:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
4218 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:4264:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
4264 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_novis` is defined multiple times
    --> tests/bisect.rs:4367:1
     |
 195 | fn scratch_novis() -> Result {
     | ---------------------------- previous definition of the value `scratch_novis` here
...
4367 | fn scratch_novis() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_novis` redefined here
     |
     = note: `scratch_novis` must be defined only once in the value namespace of this module
```

```text
error[E0428]: the name `scratch_nost` is defined multiple times
    --> tests/bisect.rs:4413:1
     |
 241 | fn scratch_nost() -> Result {
     | --------------------------- previous definition of the value `scratch_nost` here
...
4413 | fn scratch_nost() -> Result {
     | ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `scratch_nost` redefined here
     |
     = note: `scratch_nost` must be defined only once in the value namespace of this module
```

```text
warning: methods `listed` and `menu` are never used
   --> tests/bisect.rs:54:8
    |
 30 | impl Editor {
    | ----------- methods in this implementation
...
 54 |     fn listed(&self) -> Result<Vec<String>> {
    |        ^^^^^^
...
113 |     fn menu(&mut self, item: &str) -> Result {
    |        ^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```
