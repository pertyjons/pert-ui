# Diagnostics survey (API consistency audit, slice 4)

Run by a Claude agent on 2026-10-21 in `target/diag-survey/`, outside the
repository. This directory keeps the record: the cases' sources
(`cases/`, the consumer's `src/bin/`, with its manifest as
`cases/Cargo.toml.txt`), the control crates (`control/`), the runner
(`run.sh`), the raw messages (`baseline/`, `proto/`) and the prototype's
changes (`prototype.patch`, never applied to the repository). Paths below
name the original locations; `<repo>` stands for the checkout.

Procedure: `plans/usability/api-audit/rubric.md`, "Surveys" → "Diagnostics".
Inputs: slice 3's misleading diagnostics (`plans/developer-usability.md`,
"Compile diagnostics that misled testers", the runs' `diagnostics.md`), F11
and F19. Library source: branch `developer-usability`, unchanged since
d4ba108 (the facade is 0.3.52). The real repository was not modified;
`git status --porcelain` printed nothing after the survey.

**Measured** means the message below was produced by the commands in this
file. **Inferred** marks reasoning that no run here checked.

## Setup and commands

- Toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)` and `cargo 1.99.1`, pinned
  by a copy of `rust-toolchain.toml` in each consumer. Builds are `--offline`.
- Baseline consumer: `target/diag-survey/consumer` (edition 2024, its own
  `[workspace]`, the workspace's `Cargo.lock` copied in), with
  `pert-ui = { path = "<repo>/crates/pert-ui" }` and the
  default features (`desktop`; `AppError` needs them). Each case is
  one bin under `src/bin/`, and its header comment names the mistake.
- Prototype: `target/diag-survey/repo` is a `cp -r` of `crates/`,
  `Cargo.toml`, `Cargo.lock` and `rust-toolchain.toml`. Its `Cargo.toml` drops
  the `apps/*` members, which were not copied.
  `target/diag-survey/consumer-proto` contains the same bins and points at
  `repo/crates/pert-ui`. `target/diag-survey/prototype.patch` holds every
  change to the copy.
- Control crates: `target/diag-survey/control/{inner,mini,user}` test rustc's
  behaviour outside pert-ui.
- Shared target directory: `CARGO_TARGET_DIR=<repo>/target/diag-survey/target`.
- Runner: `target/diag-survey/scripts/run.sh <consumer> <out>` runs, for each bin,
  `cargo check --offline --quiet --color never --bin <name>` and saves stderr
  to `baseline/<name>.txt` or `proto/<name>.txt`. Each `.txt` has the
  complete message. The excerpts below drop blank gutter lines, the "For more
  information" and "could not compile" trailers, and warnings that the case
  does not cover (an unused `prelude::*` import, for example).
- The copy builds without warnings. `z00_positive_control` (valid use of every
  changed item, including `Err(RuntimeError::Closed)?` into `AppError`) compiles
  against the copy (exit 0).

## Summary

Key: **Fix?** means the baseline message leads to the fix. **Attr** says whether
`#[diagnostic::on_unimplemented]` applies (E0277/E0599 from an unsatisfied
bound on a pert-ui trait). The verdict and the recommended test follow.

| Case | Error | Fix? | Attr | Verdict | Test |
|---|---|---|---|---|---|
| c01 non-Component to `Element::new` | E0277 | yes | yes | helps a little | trybuild if adopted |
| c02 `Component` missing `update` | E0046 | yes | no | cannot help, not needed | none |
| c03 `update` without `Result` | E0053 | yes | no | cannot help, not needed | none |
| c04 borrowing component (`'static`) | E0478+E0803 | partly | no | cannot help (lifetime); document `'static` | compile_fail (optional) |
| c05 `reconcile(&Self)` | E0053 | yes | no | cannot help, not needed | none |
| c06 `Element` given to `Element::new` | E0277 | yes (rustc help) | yes | helps (note) | trybuild if adopted |
| c07 `update` returns `DrawError` | E0053 | yes | no | cannot help, not needed | none |
| l01 `Vec` to `ListSource::new` | E0277 | no | yes | helps | trybuild |
| l02 `ListItems` missing `build` | E0046 | yes | no | not needed | none |
| l03 `key -> Option<&str>` | E0053 | yes | no | not needed | none |
| l04 borrowing `ListItems` | E0521 | partly | no | cannot help (lifetime) | none |
| l05 `Rc<impl>` to `ListSource::new` | E0277 | yes | yes | helps (note) | trybuild |
| l06 impl straight into `VirtualList::new` | E0277 on `From` | no | no (std `From`) | cannot help; API-side only | none |
| t01 map to `TreeSource::new` | E0277 | no | yes | helps | trybuild |
| t02/t03 missing `build` / `children(String)` | E0046/E0053 | yes | no | not needed | none |
| t04 borrowing `TreeItems` | E0521 | partly | no | cannot help | none |
| b01 `Vec<Vec<_>>` to `TableSource::new` | E0277 | no | yes | helps | trybuild |
| b02/b03 missing `cell` / `cell(.., usize)` | E0046/E0053 | yes | no | not needed | none |
| b04 borrowing `TableItems` | E0521 | partly | no | cannot help | none |
| r01 non-`ContentRows` in `content_rows` | E0277 (unsize) | yes | yes | helps | trybuild |
| r02/r03 missing `revision`/`reveal`, wrong `reveal` | E0046/E0053 | yes | no | not needed | none |
| r04 rows borrowing from `self` | lifetime | **misleading** | no | cannot help; document | none |
| k01 `String` to `set_clipboard` | E0277 | no | yes | helps | trybuild |
| k02–k04 missing/wrong `Clipboard` items | E0046/E0053 | yes | no | not needed | none |
| k05 borrowing clipboard | E0597 | yes | no | cannot help, not needed | none |
| s01 `impl StateOwner` (sealed) | E0277 on `Sealed` | mostly | **yes** (on `Sealed`) | helps; renaming the seal removes a stray note | compile_fail (new) + trybuild |
| s02 non-owner to `&impl StateOwner` | E0277 | partly | yes | helps | trybuild |
| i01/i02 `.key` without the trait imported | E0599 | yes | no | not needed | none |
| i03 `.key` on a non-Component | E0599 | partly | yes (via `Component`) | helps | trybuild |
| i04 own `impl IntoElement` | E0119 | yes | no (coherence) | not needed | compile_fail (optional) |
| i05 own `impl IntoElement` for non-Component | E0277 | yes | yes (via `Component`) | helps a little | none extra |
| i06 bare components as children | E0271 | no | no | cannot help | none |
| m01 `Modifiers` not imported (→ `Notifier`) | E0422/E0433 | **misleading** | no | cannot help; prelude decision | none |
| m02 `CommandScope`/`KeyChord`/`CommandKey` not imported | E0433 | no | no | cannot help; prelude decision | none |
| m03/m04 F11: `AppError::from`/`?` names `DrawError` | E0277 | **misleading** | no; `do_not_recommend` **yes** | helps | trybuild |
| m05 F11: `paint()?` into `RuntimeError` | E0277 on `From` | cause only | no (std `From`) | cannot help; API decision | none |
| m06 `Modifiers::CONTROL` | E0599 | no | no; `doc(alias)` **yes** | helps (with a caveat) | trybuild if adopted |
| m07 E0614 cascade after E0609 | E0609+E0614 | first yes, second misleading | no | cannot help (rustc) | none |
| m08 F19: dropped `CommandId` | `unused_must_use` | pushes `let _` | `#[must_use = ".."]` adds a note | F19's fix is API-side | none or trybuild |

## Cases

### Component (`pert_ui_runtime::runtime::Component`; only `update` is required; `Component: Any`, so `'static`)

**c01: a type without an implementation passed to `Element::new`.**
`struct Panel; Element::new("panel", Panel)`. Baseline (measured):

```text
error[E0277]: the trait bound `Panel: pert_ui::Component` is not satisfied
  |     ------------          ^^^^^ unsatisfied trait bound
help: the trait `pert_ui::Component` is not implemented for `Panel`
    = help: the following other types implement trait `pert_ui::Component`:
              Accordion
              Banner ... and 69 others
note: required by a bound in `pert_ui::Element::new`
```

It points to the fix (implement `Component`). The list of 77 widgets is noise.
The attribute applies. Prototype (measured):

```text
error[E0277]: `Panel` is not a pert-ui component
  |                     ------------          ^^^^^ `Panel` does not implement `Component`
    = note: implement `Component` for `Panel`; only `update` is required, and the type must own its data (`'static`)
    = note: an `Element` from `.key(..)` or `Element::new` is already a described child: pass it in a children list instead of describing it again
    = help: the following other types implement trait `pert_ui::Component`: ... and 69 others
```

Verdict: helps a little. The note adds the required method and the `'static`
rule. The attribute does not remove rustc's implementor list (measured).

**c02: missing `update`.** E0046 `not all trait items implemented, missing: update`,
with `help: implement the missing item: fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { todo!() }`
(measured). This points to the fix. The attribute does not apply (a missing-item
error). Verdict: not needed.

**c03: `fn update(&mut self, _cx: &mut UpdateContext) {}`.** E0053 `expected
Result<(), RuntimeError>, found ()`, plus `help: change the output type to match
the trait` (measured). This points to the fix. The attribute does not apply.

**c04: `struct Title<'a>(&'a str); impl<'a> Component for Title<'a>`.**
Measured:

```text
error[E0478]: lifetime bound not satisfied
  = note: but lifetime parameter must outlive the static lifetime
error[E0803]: cannot infer an appropriate lifetime for lifetime parameter `'a` due to conflicting requirements
```

The message says `'static` but not why. The `Any` supertrait is not mentioned,
and `Component`'s rustdoc does not state the `'static` requirement (read).
The attribute does not apply (a lifetime error). Verdict: cannot help. State
"components own their data (`'static`)" in the `Component` rustdoc (inferred
remedy). The c01 note also carries it for the cases that reach E0277.

**c05: `fn reconcile(&mut self, next: &Self)`.** E0053 `expected Panel, found
&Panel`, with a help that rewrites the parameter to `next: Panel` (measured).
This points to the fix. The attribute does not apply.

**c06: `Element::new("outer", Label::new("x").key("inner"))`.** Baseline
(measured): E0277 `the trait bound pert_ui::Element: pert_ui::Component is not
satisfied`, the implementor list, then `help: consider removing this method call,
as the receiver has type pert_ui::Label and pert_ui::Label: pert_ui::Component
trivially holds`. rustc's own help is the fix. The prototype adds the headline and
both notes. The second note fits this case. The first, "implement `Component`
for `pert_ui::Element`", is wrong here. Stable `on_unimplemented` has no `on(..)`
filters, so each note must read correctly for every `Self`. Verdict: helps if
the notes are worded for both cases, for example "if this is an `Element`, it
is already described".

**c07: `update` returning `Result<(), DrawError>`.** E0053 `expected RuntimeError,
found pert_ui::DrawError`, with a type rewrite (measured). This points to the fix.
The attribute does not apply.

### ListItems (`len`, `key`, `build` required; `ListSource::new(impl ListItems + 'static)`)

**l01: `ListSource::new(vec!["a".to_owned(), ..])`.** Baseline (measured):
`the trait bound Vec<String>: ListItems is not satisfied` and `required by a bound
in ListSource::new`. Nothing says that an own type must implement the trait. The
attribute applies. Prototype (measured):

```text
error[E0277]: `Vec<String>` cannot be used as list rows
   |                     --------------- ^^^^^^^^^ `Vec<String>` does not implement `ListItems`
   = note: implement `ListItems` (`len`, `key`, `build`) on a type that owns or shares the rows, such as a struct holding an `Rc<Vec<_>>`
   = note: `ListSource::new` takes the implementation itself, not an `Rc` of it
```

Verdict: helps.

**l02: missing `build`; l03: `fn key(..) -> Option<&str>`.** E0046 with the
`fn build(&self, _: usize) -> Option<pert_ui::Element>` stub, and E0053 `expected
String, found &str` with a type rewrite (measured). These point to the fix. The
attribute does not apply.

**l04: `struct Rows<'a>(&'a [String])` built in `fn source(rows: &[String]) -> ListSource`.**
E0521 `borrowed data escapes outside of function … argument requires that '1
must outlive 'static` (measured). This points to the cause, not to the remedy (own
or `Rc`-share the rows). The attribute does not apply (a lifetime error).

**l05: `ListSource::new(shared.clone())` with `shared: Rc<Rows>`.** Baseline
(measured): `Rc<Rows>: ListItems is not satisfied` and `help: the trait ListItems
is implemented for Rows`. This points to the fix. The prototype adds the headline
and the note "`ListSource::new` takes the implementation itself, not an `Rc` of
it" (measured). Verdict: helps.

**l06: `VirtualList::new(Rows(vec![]), 24.0)`, where a bound `ReadSignal<ListSource>` is expected.**
Measured, unchanged by the prototype:

```text
error[E0277]: the trait bound `ReadSignal<ListSource>: From<Rows>` is not satisfied
help: `ReadSignal<T>` implements trait `From<T>`: From<Signal<T>>, From<&Signal<T>>, From<&ReadSignal<T>>
    = note: required for `Rows` to implement `std::convert::Into<ReadSignal<ListSource>>`
```

It does not lead to the fix (`ListSource::new(rows)` in a signal). The failing
trait is std's `From`/`Into`, which pert-ui cannot annotate. Verdict: the attribute
cannot help. Only an API change could, such as a pert-ui-owned conversion trait
in that position that carries the attribute (inferred, and an API decision).

### TreeItems (`roots`, `children`, `build` required)

**t01: `TreeSource::new(BTreeMap<String, Vec<String>>)`.** Baseline: `the trait
bound BTreeMap<..>: TreeItems is not satisfied` (measured). Prototype: ``
`BTreeMap<String, Vec<String>>` cannot be used as a tree hierarchy `` with the
note `implement TreeItems (roots, children, build) on a type that owns or shares
the hierarchy` (measured). Verdict: helps.

**t02 (missing `build`) and t03 (`children(&self, _key: String)`).** E0046 with a
stub, and E0053 `expected &str, found String` with a rewrite (measured).
Not needed.

**t04: a borrowing implementation.** E0521, the same as l04 (measured). Cannot help.

### TableItems (`len`, `key`, `cell` required)

**b01: `TableSource::new(Vec<Vec<String>>)`.** Baseline: `the trait bound
Vec<Vec<String>>: TableItems is not satisfied` (measured). Prototype:
`` `Vec<Vec<String>>` cannot be used as table rows `` with a note naming
`len`, `key` and `cell` (measured). Verdict: helps.

**b02 (missing `cell`) and b03 (`cell(&self, index, column: usize)`).** E0046 with
a `fn cell(&self, _: usize, _: &str) -> Option<pert_ui::Element>` stub, and
E0053 `expected &str, found usize` (measured). Not needed.

**b04: a borrowing implementation.** E0521, as l04 (measured). Cannot help.

### ContentRows (`len`, `key`, `text`, `revision`, `reveal` required; returned as `Rc<dyn ContentRows>`)

**r01: `content_rows` returns `Some(self.0.clone())` with `self.0: Rc<Lines>`, where `Lines` has no implementation.**
Baseline (measured):

```text
error[E0277]: the trait bound `Lines: pert_ui::ContentRows` is not satisfied
  |     ^^^^^^^^^^^^^^ unsatisfied trait bound
help: the trait `pert_ui::ContentRows` is not implemented for `Lines`
  = note: required for the cast from `Rc<Lines>` to `Rc<dyn pert_ui::ContentRows>`
```

The attribute applies to the unsizing cast too (measured):
`` `Lines` does not offer searchable rows `` with the note `implement
ContentRows (len, key, text, revision, reveal) for Lines, answering from data it
owns`. Verdict: helps.

**r02 (missing `revision` and `reveal`) and r03 (`reveal` returns `Result<(), RuntimeError>`).**
E0046 lists both stubs, including `fn reveal(&self, _: &str) ->
Result<ListRevealReceipt, RuntimeError>`. E0053 says `expected ListRevealReceipt,
found ()` (measured). Not needed.

**r04: rows that borrow from the component.** `fn content_rows(&self) ->
Option<Rc<dyn ContentRows>> { Some(Rc::new(Lines(&self.0))) }`. Measured:

```text
error: lifetime may not live long enough
   |                     -     ^^^^^^^ returning this value requires that `'1` must outlive `'static`
help: to declare that the trait object captures data from argument `self`, you can add an explicit `'_` lifetime bound
15 |     fn content_rows(&self) -> Option<Rc<dyn ContentRows + '_>> { ... }
```

**Misleading:** following the help (r04b, measured) fails with ``error: `impl`
item signature doesn't match `trait` item signature … expected `fn(&'1 Log) ->
Option<Rc<(dyn pert_ui::ContentRows + 'static)>>` ``. The real fix is to share
the rows through an `Rc` that the component holds. The attribute does not apply
(a lifetime error). Verdict: cannot help. `Component::content_rows` and
`ContentRows` could state that the returned rows own or share their data
(inferred remedy). This case is new; it is not one of slice 3's.

### Clipboard (`read_text`, `write_text` required; `Runtime::set_clipboard(impl Clipboard + 'static)`)

**k01: `ui.set_clipboard(String::new())`.** Baseline: `the trait bound String:
pert_ui::Clipboard is not satisfied` (measured). Prototype:
`` `String` is not a clipboard service `` with the note `implement Clipboard
(read_text, write_text) on a type that owns its storage, such as a struct holding
an Rc<RefCell<String>>` (measured). An earlier draft of the note read "such as
`Rc<RefCell<String>>`". That suggests a type that does not implement the trait,
so a note must name a wrapper rather than a std type. Verdict: helps.

**k02 (missing `write_text`), k03 (`read_text(&self)`) and k04 (`Result<String, io::Error>`).**
E0046 with a stub, E0053 `types differ in mutability` with a `&mut self` rewrite,
and E0053 `expected ClipboardError, found std::io::Error` (measured). Not needed.

**k05: `Shared<'a>(&'a mut String)` passed to `set_clipboard`.** E0597 `` `held` does
not live long enough … argument requires that `held` is borrowed for `'static` ``,
with `note: requirement that the value outlives 'static introduced here` pointing
at `impl crate::Clipboard + 'static` (measured). This points to the fix. Cannot
help, not needed.

### The sealed StateOwner

**s01: `impl StateOwner for MyOwner` (a wrapper around `Runtime`).** Baseline (measured):

```text
error[E0277]: the trait bound `MyOwner: pert_ui_runtime::signal::sealed::Sealed` is not satisfied
   = note: `MyOwner` implements similarly named trait `unicode_width::private::Sealed`, but not `pert_ui_runtime::signal::sealed::Sealed`
   = help: the following other types implement trait `pert_ui_runtime::signal::sealed::Sealed`: &O, &mut O, DescribeContext<'_>, EventContext, UpdateContext, pert_ui::Runtime
   = note: `StateOwner` is a "sealed trait", because to implement it you also need to implement `pert_ui_runtime::signal::sealed::Sealed`, which is not accessible; ...
   = help: the following types implement the trait: ...
```

rustc already explains sealing. The headline names a private path, and a note
about `unicode_width`'s `Sealed` is noise. The rubric expects the attribute not to
help with sealed-trait errors. Measured, it does: the error is an unsatisfied
supertrait bound on the private `sealed::Sealed`, and an `on_unimplemented` on
that private trait fires:

```text
error[E0277]: `StateOwner` is sealed and cannot be implemented for `MyOwner`
   |                     ^^^^^^^ not one of pert-ui's state owners
   = note: only `Runtime`, `UpdateContext`, `EventContext` and `DescribeContext` own state; hold one of them, or take `&impl StateOwner` and pass one through
```

The stray `unicode_width` note is still there with the attribute. Renaming the
private trait to `StateOwnerSeal` (s01b, measured) removes it. Verdict: helps.
The attribute goes on the seal, not on `StateOwner`. A distinct seal name stops
the similar-name note.

**s02: `ListController::new(&Model)`.** Baseline: `the trait bound Model:
StateOwner is not satisfied` with the implementor list (measured). Prototype:
`` `Model` cannot create signals ``, with the label `expected a Runtime,
UpdateContext, EventContext or DescribeContext` and the note `pass the runtime or
the context of the callback you are in` (measured). Verdict: helps.

### IntoElement

**i01: `use pert_ui::Label;` then `Label::new("x").key("title")`.** Measured:

```text
error[E0599]: no method named `key` found for struct `pert_ui::Label` in the current scope
help: trait `IntoElement` which provides `key` is implemented but not in scope; perhaps you want to import it
  2 + use pert_ui::IntoElement;
```

This points to the fix. The suggestion uses the crate root path, which works, and
`use pert_ui::prelude::*` also brings the trait in. No `pert_ui_runtime` path is
suggested. i02, which imports several root items, gives the same help once per
call site (three errors for three `.key` calls). The attribute does not apply (an
import error). Not needed.

**i03: `struct Panel; Panel.key("panel")`.** Baseline (measured): E0599 `the method
key exists for struct Panel, but its trait bounds were not satisfied`, with
`Panel: pert_ui::Component which is required by Panel: pert_ui::IntoElement` and
`note: the trait pert_ui::Component must be implemented`. It then lists
`ContentRows`, `IntoElement`, `ListItems` and `TableItems` as candidates to
implement, which is noise: `key` is a method name shared by four traits. The
attribute on `Component` also changes the E0599 headline, label and notes
(measured): `` error[E0599]: `Panel` is not a pert-ui component ``. Verdict: helps.
The candidate list stays.

**i04: `impl IntoElement for Panel {}` where `Panel: Component`.** E0119 `conflicting
implementations … conflicting implementation in crate pert_ui_runtime: impl<C>
pert_ui::IntoElement for C where C: pert_ui::Component, C: 'static` (measured).
This points to the fix (delete the impl). The attribute does not apply (a
coherence error).

**i05: `impl IntoElement for Panel` where `Panel` is not a Component.** E0277 on the
`Component` supertrait (measured). `Component`'s attribute fires (measured), so
the message reads `` `Panel` is not a pert-ui component ``. The
`on_unimplemented` on `IntoElement` itself fired in no case. No pert-ui API takes
an `IntoElement` bound (read: rg finds only the definition and the blanket impl),
so that attribute is unnecessary.

**i06: `Column::new([Label::new("a"), Label::new("b")])`.** Measured:

```text
error[E0271]: type mismatch resolving `<[Label; 2] as IntoIterator>::Item == Element`
note: required by a bound in `pert_ui::Column::new`
   = note: this error originates in the macro `flow_component`
```

It does not mention `.key(..)`. This is a projection mismatch, not an unsatisfied
pert-ui trait, so the attribute cannot help (inferred from the error kind; no
prototype was tried).

### Slice 3's misleading diagnostics (all reproduced)

**m01: `Modifiers { alt: true, ..Modifiers::default() }` with `use pert_ui::prelude::*`.**
Measured: E0422 and E0433 `help: a struct with a similar name exists … Notifier`,
the same as round 1. Unimported names with no similar name in scope get no help
at all (m02, m02b, m02d, m02e, m02f, measured). rustc 1.99.1 suggested
**no** import from any non-std dependency in these probes. That covered items
re-exported from `pert_ui_runtime` (`Label`, `Modifiers`, `CommandScope`), items
defined in `pert_ui` itself (`App`, `AppError`), and a control crate's own item
(`control/user` → `mini::OwnThing`, no suggestion). The facade's glob
re-export is therefore not the cause. Std items got `consider importing` (m02c).
The attribute does not apply (a resolution error). Verdict: cannot help. Among the
arrangements these probes tried, only prelude membership routed a consumer to the
name (a pointer on the prelude page is the documentation counterpart). That is
app-author slice 9's decision. The probes do not establish a general compiler
limitation.

**m02: `CommandScope`, `KeyChord` and `CommandKey` not imported.** Measured:
three E0433 `use of undeclared type …` errors, with no help. This has the same
cause as m01 and cannot be helped. A prelude or guide change is needed.

**m03/m04 (F11): `AppError::from(io::Error)` and `?` of an `io::Error` in `-> Result<_, AppError>`.**
Baseline (measured):

```text
error[E0277]: `?` couldn't convert the error to `DrawError`
  |              ------------------------ expected `DrawError` because of this
  |  ... ^ the trait `From<std::io::Error>` is not implemented for `DrawError`
  = help: `DrawError` implements trait `From<T>`: From<ApplicationError> ... and 10 others
  = note: required for `pert_ui::AppError` to implement `From<std::io::Error>`
```

This is misleading. It names `DrawError` as the expected type, which comes from
the blanket `impl<E> From<E> for AppError where DrawError: From<E>` in
`crates/pert-ui/src/app.rs`. The failing trait is std's `From`, so
`on_unimplemented` cannot apply. `#[diagnostic::do_not_recommend]` is stable on
1.99.1: it was accepted without a feature gate or warning, as measured. On that
impl it gives (measured):

```text
error[E0277]: `?` couldn't convert the error to `pert_ui::AppError`
  |              ------------------------ expected `pert_ui::AppError` because of this
  |  ... ^ the trait `From<std::io::Error>` is not implemented for `pert_ui::AppError`
  = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
```

`AppError::from` (m03) likewise becomes `the trait bound pert_ui::AppError:
From<std::io::Error> is not satisfied`. Valid conversions still compile
(`z00_positive_control`). With the attribute, rustc no longer lists the
convertible error types, because the only impl is the hidden blanket one.
The `AppError` rustdoc should therefore list them (inferred). Verdict: helps,
and it resolves F11's first half.

**m05 (F11): `fn frame(ui: &mut Runtime) -> Result<DisplayList, RuntimeError> { Ok(ui.paint()?) }`.**
Measured: `` `?` couldn't convert the error to `pert_ui::RuntimeError` … the trait
`From<DrawError>` is not implemented for `pert_ui::RuntimeError` ``, with a list
of 19 `From` impls. The message gives the cause correctly but no fix. The trait is
std's `From`, so no attribute can help. The fix is an API decision for the audit:
a conversion, or a documented helper error type for tests. This is F11's
second half.

**m06: `Modifiers::CONTROL`.** Baseline (measured): E0599 `no associated function
or constant named CONTROL found for struct Modifiers`, with no hint about the
`control` field or `Modifiers::PRIMARY`. The attribute does not apply (a missing
item). rustc does use `#[doc(alias)]` in E0599 similar-name suggestions, which is
stable. With `#[doc(alias("CONTROL", "CTRL", "COMMAND", "CMD"))]` on
`Modifiers::PRIMARY` (measured):

```text
help: there is an associated constant `PRIMARY` with a similar name
3 + fn main() { let _ = Modifiers::PRIMARY; }
```

Verdict: helps with a caveat. `PRIMARY` is Command on macOS, which suits the
round 1 testers' shortcuts but is wrong for someone who means Control itself. An
alias should go only where the suggested item really is the intent (inferred).
Adding constants is an API decision.

**m07: `match entry.properties.get(key) { Some(InspectValue::Number(n)) => Ok(*n), .. }`.**
Measured: E0609 `no field properties on type &ComponentInspection` with
`help: … entry.details.properties`, then `E0614 type f64 cannot be dereferenced`
on `*n`. The first error has the fix. The second is a rustc cascade: with the
scrutinee type unknown, the pattern binds by value. No library mechanism can
help (inferred from the error kinds).

**m08 (F19): `ui.register_command(..)?;` with the `CommandId` dropped.** Measured
under `#![deny(unused_must_use)]`: `unused CommandId that must be used` and
`help: use let _ = ... to ignore the resulting value`. The warning pushes `let _`
for a value whose drop is harmless. A `#[must_use = ".."]` reason adds a note
(measured): `= note: dropping a CommandId keeps the command registered; keep it
only to unregister or invoke the command later`. But a must_use reason that says
dropping is fine contradicts the lint. F19's fix is to remove `#[must_use]` from
`CommandId` (an API decision), not to change the diagnostic (inferred). Also
recorded in slice 3: round 1 saw the same lint on `TaskReceipt` from `cx.defer`,
whose `must_use` text ("retain the receipt if the application needs the
operation's outcome") already explains itself.

## Where the attribute applies, and where it does not

- **Applies (measured):** E0277 for a pert-ui trait bound at a call
  (`Element::new`, `ListSource::new`, `TreeSource::new`, `TableSource::new`,
  `set_clipboard`, `&impl StateOwner`), in an unsizing cast to `Rc<dyn ContentRows>`,
  for a supertrait bound on an impl (`IntoElement: Component`, `StateOwner: Sealed`),
  and in E0599 "method exists but its trait bounds were not satisfied" (i03).
  The attribute replaces the headline and the label and adds notes. rustc's
  "following other types implement" list stays.
- **Does not apply (measured):** missing items (E0046), signature
  mismatches (E0053), lifetime and borrow errors (E0478, E0803, E0521, E0597,
  "lifetime may not live long enough"), coherence (E0119), unresolved
  names (E0422, E0425, E0433), a missing associated item (E0599 without
  candidates), projection mismatches (E0271), and every failure of std's
  `From`/`Into` (l06, m03–m05).
- **Other stable mechanisms (measured on 1.99.1):** `#[diagnostic::do_not_recommend]`
  on a blanket impl (F11, m03/m04), `#[doc(alias)]` on an inherent associated
  item (m06), a distinct name for a private seal trait (s01b), and
  `#[must_use = "reason"]` (m08).
- **Constraint (measured, c06):** stable `on_unimplemented` takes only
  `message`, `label` and `note` with `{Self}`/generic placeholders. It has no
  per-type conditions, so a note must be true for every type that can fail the
  bound. Note wording should not suggest a std type that would itself fail (k01).

## Testing

Measured: on stable 1.99.1, rustdoc does **not** check the error code of
`compile_fail,E0XXX`. A probe annotated `compile_fail,E0599` that actually fails
with E0308 passed (`control/mini`). A `compile_fail` doctest therefore proves
only that the snippet fails to compile, for any reason; a typo or a missing import
passes as well. It suits type-level restrictions that must hold on every
toolchain, as AGENTS.md says, but not wording.

Per case:

- **trybuild snapshot** (wording; pinned toolchain only): every case whose message
  pert-ui changes. If the attributes are adopted: c01, c06, i03, l01, l05, t01,
  b01, r01, k01, s01, s02, m03/m04 (do_not_recommend), m06 (if the alias is
  adopted), and m08 (if a must_use reason is kept).
- **compile_fail doctest** (rejection only, every toolchain): s01, sealing
  `StateOwner` is a type-level guarantee and there is no such doctest yet. The
  existing `compile_fail` doctests (read) cover `!Send` types, the
  `ReadSignal::set` capability, choice-source exclusivity and a few others, but
  none tries to implement `StateOwner`. Optional: c04/i04, if the `'static` rule and the
  blanket-impl rule are to be shown as documented restrictions.
- **No test:** rustc-owned messages that pert-ui cannot affect and that already
  lead to the fix: c02, c03, c05, c07, l02, l03, t02, t03, b02, b03, r02, r03,
  k02–k05, i01, i02. The same goes for the cases no mechanism helps: c04, l04,
  t04, b04, r04, l06, i06, m01, m02, m05 and m07. Their remedies are
  documentation, prelude or API decisions, which ordinary doctests and the API
  listing cover. The trait signatures are already pinned by the `api/` listings.

Overall recommendation (inferred, not run here; trybuild is not in the local
registry cache, so it could not be measured offline):

- Put the snapshot tests in a separate `publish = false` workspace member (for
  example `tests/diagnostics`) with `trybuild` as its only dev-dependency.
  Adding the dependency follows the dependency row of the gate table, and each
  case is a file under `ui/` with a committed `.stderr`.
- Run it in its own CI job on the pinned toolchain from `rust-toolchain.toml`,
  on Linux only. Exclude it from the stable matrix (macOS, Windows, stable
  Linux), because rustc wording changes between releases. A toolchain bump
  regenerates the snapshots with `TRYBUILD=overwrite`, and the diff is reviewed
  like the `api/` listing. AGENTS.md's gate table and `ci.yml` would gain the job
  in the same change.
- Keep the suite to the cases pert-ui shapes (the list above). The measured
  messages quote pert-ui source lines with line numbers (for example
  `layout.rs:375` in "required by a bound in `Element::new`"). If trybuild keeps
  those line numbers after its path normalization, unrelated edits to those
  files churn the snapshots (inferred). A small suite keeps that churn reviewable.
- Keep `compile_fail` doctests for type-level guarantees only, and say in
  `docs/verification.md` that they do not check the error.

## Recommendations for the audit's findings table (inferred from the measurements)

1. Adopt `#[diagnostic::do_not_recommend]` on `impl<E> From<E> for AppError` (F11),
   and list the accepted error types in `AppError`'s rustdoc.
2. Add `#[diagnostic::on_unimplemented]` to `Component`, `ListItems`, `TreeItems`,
   `TableItems`, `ContentRows`, `Clipboard`, `StateOwner` and the private seal,
   with notes worded for every `Self`. Skip `IntoElement`, whose attribute never
   fires.
3. Rename the private seal trait so that dependency traits named `Sealed` no
   longer produce a similar-name note.
4. State the `'static` (own or `Rc`-share the data) requirement in the
   `Component`, `ListItems`, `TreeItems`, `TableItems`, `ContentRows` and
   `Clipboard` rustdoc. Explain r04 at `Component::content_rows`, where rustc's
   `+ '_` help leads nowhere.
5. Decide F19 by removing `#[must_use]` from `CommandId`. Decide m05 (F11's second
   half), l06 and m06 as API questions. Leave m01/m02 to slice 9's prelude decision;
   no compiler mechanism helps them.
