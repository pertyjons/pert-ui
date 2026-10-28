# Round 2 fix plan, part P10 design checkpoint: derivations over several bindings

Part P10 of [the round 2 fix plan](round-2-fix-plan.md#p10-derivations-over-several-bindings-0385)
closes R2-3 by decision D10 (approved with the recommended option on
2026-10-28): `ReadSignal::combine` over a tuple of bindings, replacing the
two-input method. The plan gives P10 a checkpoint for the signature, the
arity limit and the batching contract; this document settles them, with the
exact API, behavior rules, migration manifest, tests, CHANGELOG, gate and
native evidence, before any code. Codex reviews it; the author approves it
together with Codex's review under the maintainer's delegation of
2026-10-28.

Every reference to library code is to `bfb11cd` (library code at P3,
0.3.78). `R/signal.rs` is unchanged since 26e6b5a, the commit the plan
cites, so the plan's line numbers for it still hold. Path prefixes are the
plan's: `R/` = `crates/pert-ui-runtime/src/`, `F/` = `crates/pert-ui/src/`,
`G/` = `crates/pert-ui/src/guide/`; `T/` = `crates/pert-ui/tests/`, `GA/` =
`apps/gallery/src/`, `D/` = `tests/diagnostics/`, `RL` =
`api/pert-ui-runtime/default.txt`, `U` = `~/pert-ui-usability/` (frozen).

P10 lands after P2 to P9 (0.3.77 to 0.3.84). It depends on none of their
items: no design among P2 to P8 names `combine` (`rg combine` over
`round-2-design-P{2,4,5,6,7,8}.md`), and P9 has no checkpoint. Because the
migration below is defined by a search, the implementer reruns it at
P10's base commit and migrates every hit, including any a later part adds.
**P12 depends on P10:** its settings-form recipe derives the form's dirty
state with this `combine` (plan, "P10 and P11 before P12"). Nothing in P10
names an item of P11 or P12.

Status: **revision 3, approved 2026-10-28** by the author together with
Codex's review, under the maintainer's delegation. Revision 1 was
reviewed by Codex (needs revision, five should-fix points and a nit);
revision 2 answered each, and Codex's recheck left one should-fix point
(what an equal result stops), which revision 3 answers
([review record](#review-record)). **P10-D1**
(the function receives `&(A, B, …)`) is approved by the author as
recommended under the maintainer's delegation of 2026-10-28. Codex's
recheck of revision 2 found every earlier point resolved and left only the
narrow point revision 3 answers exactly as Codex proposed (wording, a
contract test, no API or algorithm change); the author approved revision 3
without a further recheck.

## What testers built by hand, and what the library says

- **Both task 1 runs chained four `combine` calls to build a tuple.** The
  Claude run derived the edited settings from five field signals
  (`plans/usability/runs/r2-1-claude/submission/src/lib.rs:164-175`), the
  Codex run the same (`plans/usability/runs/r2-1-codex/submission/src/main.rs:69-82`);
  each then compared the record with an `applied` signal in a fifth
  `combine` (`:176`; `:83`). Every intermediate closure clones the tuple
  built so far (`|(p, f), q| (p.clone(), *f, *q)`).
- **Both wrote `.read_only()` on every input**, because the second
  parameter is `&ReadSignal<U>` (`R/signal.rs:800`) and the first call
  started from a `Signal`; deref coercion would have accepted `&signal`, as
  `Signal`'s page says (`R/signal.rs:184-188`), but neither run relied on it.
- **Triage** (`plans/usability/runs/triage-r2/group-1.md:57`, `:238`;
  Codex's narrowing, `codex-review.md:15`): the finding is the missing n-ary
  derivation; `map` already projects read-only, and a writable field
  projection is a separate follow-up (D10).

## 1. Today's behavior, from the code

- **`map`** (`R/signal.rs:733-787`): one input; checks the phase
  (`:763`), evaluates once under the evaluation guard (`:764-767`), stores
  the result in a new signal and a `Derived` node whose `compute` publishes
  `map(&source.get()?)` (`:778`) and which holds one subscription on its
  input (`:781`). It performs no runtime check, having one input.
- **`combine`** (`R/signal.rs:789-835`): `&self`, `other: &ReadSignal<U>` and
  `Fn(&T, &U) -> V`. It returns `ForeignRuntime` when the two inputs' shared
  runtime state differs, before anything else (`:803-806`), then checks the
  phase (`:807`) and evaluates once (`:808-811`); the node reads both inputs
  in order and publishes (`:821`), holding two subscriptions (`:824-827`).
  Its rustdoc has no example.
- **The derived node** (`R/signal.rs:696-731`): `invalidate` sets `dirty`
  once and forwards to dependents only on the first change (`:703-708`);
  `refresh` evaluates only when dirty (`:709-722`). A publication of an input
  invalidates its computations and queues them by pointer in the runtime's
  pending map (`:571-584`), which the flush settles once each
  (`R/runtime.rs:581`). An untracked `get` refreshes a dirty node first
  (`R/signal.rs:634-645`). This is the "at most once per batch of input
  writes" of D10, and it is per node, not per input.
- **Equality** (`R/signal.rs:537-545`): `publish` returns early when the new
  value equals the stored one: no revision, no invalidation, no subscriber
  notified.
- **Retention:** the node's `compute` closure owns clones of its inputs,
  each with its `computation` handle (`ReadSignal`'s fields,
  `R/signal.rs:229-232`), so a derived input stays alive and current as
  long as the result does; the inputs hold the node only weakly
  (`subscribe_computation`, `:836-851`), and dropping the node removes it
  from the pending map (`:724-731`). Dropping the last clone of the result
  therefore drops the node, its function and its input clones.
- **Phases:** creation needs `check_evaluation` (`R/runtime.rs:567-573`):
  in update it is allowed (only `Closed` fails), in any other phase the
  write rules apply (`R/runtime.rs:711-730`): `WriteDuringPaint`,
  `WriteDuringLayout`, `WriteDuringInspection`, `WriteDuringDelivery`.
  `map`'s `# Errors` (`R/signal.rs:744-746`) names only input read errors,
  writes and cycles, not these phase errors.
- **Callers** (search at `bfb11cd`, section 6): in the runtime, two
  chains building a tuple (`R/tool_windows/options.rs:129-133`,
  `R/file_browser/controller.rs:559-578`) and one a fold over a dynamic list
  (`R/permission.rs:213-215`); the examples, including the `inventory`
  chain (`crates/pert-ui/examples/inventory/app.rs:161-171`); the gallery;
  two guides; three facade tests; the task 4 fixture.

## 2. The rules

### 2.1 Signature

`combine` becomes an associated function in an impl block whose type
parameter is the **output**, so that `ReadSignal::combine(...)` infers it
from the function (an associated function in `impl<T> ReadSignal<T>` that
did not mention `T` would need a turbofish):

```rust
impl<T: Clone + PartialEq + 'static> ReadSignal<T> {
    pub fn combine<I: CombineInputs>(
        inputs: I,
        map: impl Fn(&I::Values) -> T + 'static,
    ) -> Result<Self, RuntimeError>;
}
```

- **Inputs** are a tuple of 2 to 12 elements, each a `Signal<A>`,
  `ReadSignal<A>`, `&Signal<A>` or `&ReadSignal<A>` (section 3): the
  argument forms a constructor taking `impl Into<ReadSignal<T>>` accepts
  (`R/signal.rs:275-294`), so `&signal`, `&binding` and a binding a call
  returns (`filter.items()`) all work, with no `.read_only()`. An element
  is cloned into the node; a reference is not kept.
- **The function receives `&(A, B, …)`**, the tuple of the inputs' values
  in input order (decision P10-D1). The pattern `|(a, b)|` binds `a: &A`
  and `b: &B` by Rust's default binding modes, so a body written for
  today's `|a, b|` compiles unchanged inside `|(a, b)|`. An annotated
  closure names the tuple (`|values: &(u32, String)|`); a named function
  takes `&(A, B)`. A closure written with two parameters fails with E0593
  ("expected closure that takes 1 argument"); the rustdoc says so.
- **The limit is 12**, the arity to which the standard library implements
  its tuple traits (`Debug`, `PartialEq`, `Hash`, `Default`), so a Rust
  reader meets no new number; it is twice the largest derivation round 2
  wrote (five fields and the applied record). It is exact and documented,
  and a contract number (AGENTS.md, "Numbers that are part of a contract").
- **No 1-tuple:** a derivation from one binding is `map`; a `(&a,)` input
  would be a second way to write it and is a compile error (section 2.3).
  More than 12 inputs combine in steps (a `combine` of a tuple of derived
  bindings), or related fields live in one signal that `map` projects.
- **`map` stays** as it is (D10). `map` is a method; `combine` is not,
  since no input is privileged.

The shape was compiled with the pinned toolchain (1.99.1) in a standalone
probe outside the workspace, with the workspace's `missing_docs` and
`missing_debug_implementations` denied, `-D warnings`, Clippy and rustdoc
(no Cargo build): `|(a, b, c, d, e)|` over mixed `&Signal`, `&ReadSignal`
and owned inputs, a 12-tuple, a repeated input, a named function, a
generic caller over `I: CombineInputs`, `ForeignRuntime` for an input of
another runtime; the 1-tuple, 13-tuple and non-binding cases fail with
messages of the form section 2.3 fixes.

### 2.2 Behavior

1. **One runtime.** Every input must belong to the runtime of the first;
   otherwise the call returns `ForeignRuntime`. This is checked first,
   before any phase check, read or evaluation: the function is not called
   and nothing is subscribed (today's order, `R/signal.rs:803-807`).
2. **Phase.** Then the phase check of `map` (`check_evaluation`): allowed
   during update and outside a phase; `Closed` after shutdown; the phase's
   write error during layout, paint, inspection or delivery.
3. **Initial evaluation.** The inputs are read in tuple order, each as an
   untracked `get` that settles a dirty derived input, under the evaluation
   guard; then the function runs once on the tuple. The first read error in
   tuple order is returned and the function is not called.
4. **One node.** The result is one derived node holding one subscription
   per tuple position. Writes to any number of inputs mark it dirty once and
   run no callback; it evaluates at most once per batch of input writes,
   when the runtime settles derived values before component updates or at an
   earlier explicit `get`, reading every input in order. A chain of
   intermediate nodes is not an acceptable implementation: it adds nodes
   whose equality checks D10 does not ask for and that a reader cannot see.
5. **Equality.** The result is published through `publish`: an equal result
   advances no revision and notifies no component subscribed to the result
   (`R/signal.rs:543-545`). It does not stop a derived binding downstream
   from evaluating again: the node marks its dependents dirty when it
   becomes dirty, before it evaluates (`R/signal.rs:704-707`, `:823`), so a
   `map` or `combine` over the result runs its function at the next settle
   and publishes its own result, which its own equality check then
   filters. This is today's algorithm for `map` and `combine`, unchanged
   (`R/signal.rs:783`).
6. **Retention and release.** The node owns a clone of every input (with
   its computation, so a derived input given by value stays current) and the
   function for the result's lifetime; clones of the result share the node.
   Dropping the last clone releases the subscriptions, the input clones and
   the function with whatever it captured.
7. **A repeated input** (`(&a, &a)`) is allowed and behaves as two
   positions reading the same value; its two subscriptions are released
   together (they share the node's key on the input, `R/signal.rs:836-842`,
   as today's `a.combine(&a, f)` does).
8. **Inside the function**, the runtime is in its evaluation phase, so a
   write the function attempts is refused: the write call returns
   `WriteDuringUpdate` to the function and changes nothing, at the initial
   evaluation and at every later one. This is an operation-level
   restriction, not an error of `combine`: the function returns a `T`, so a
   function that keeps the write's result stores it as its value, exactly
   as `map` does today (`T/foundation/foundation.rs:370-375`, where
   `guarded.get()?` is `Err(WriteDuringUpdate)` and the source is
   unchanged). No input borrow is held while the function runs: the values
   are clones (P10-D1).
9. **Errors of later evaluations** are returned by the read that settles the
   node, as for `map`: an explicit `get` of the result, or the flush that
   settles derived values (`R/signal.rs:634-645`, `R/runtime.rs:581`), with
   the first input read error in tuple order.
10. **Timing class:** evaluation (`docs/contracts.md:605-620`).

### 2.3 Compiler messages

Both public traits carry `#[diagnostic::on_unimplemented]`, as
`StateOwner` does (`R/signal.rs:82-86`), and their seals carry a "sealed"
message, as `StateOwnerSeal` does (`R/signal.rs:116-120`):

- **`CombineInputs`:** message ``"`{Self}` is not a tuple of 2 to 12 bindings"``,
  label ``"expected a tuple of signals or read bindings, such as `(&a, &b)`"``,
  note ``"`ReadSignal::combine((&a, &b), |(a, b)| ...)` derives from 2 to 12 bindings; `ReadSignal::map` derives from one, and more than 12 combine in steps"``.
- **`CombineInput`:** message ``"`{Self}` is not a signal or read binding"``,
  label ``"not an input `ReadSignal::combine` can observe"``, note
  ``"each element of the tuple is a `Signal` or `ReadSignal`, or a reference to one; capture a fixed value in the function instead"``.
- **The seals:** ``"`CombineInputs` is sealed and cannot be implemented for `{Self}`"``
  and the same for `CombineInput`, with the note that only pert-ui's
  tuples and bindings implement them.

The old method call `a.combine(&b, f)` fails with rustc's E0599 ("this is
an associated function, not a method"), whose suggestion
`ReadSignal::<T>::combine(&b, f)` is not the replacement; following it
produces the `CombineInputs` error above, whose note is. The CHANGELOG
says so. The probe showed a 13-tuple's message also writes the long type
name to a hashed file (`…long-type-<hash>.txt`), so the snapshots use the
1-tuple and the non-binding cases (section 7); the 13-tuple is a
`compile_fail` doctest.

## 3. Public API

In `R/signal.rs`, beside `map`; re-exported at the crate root
(`R/lib.rs:317` becomes `pub use signal::{CombineInput, CombineInputs,
ReadSignal, Signal, StateOwner};`), and so at the facade's root through its
glob (`F/lib.rs:29`).

```rust
/// One input of [`ReadSignal::combine`]: a [`Signal`] or a [`ReadSignal`],
/// or a reference to one.
#[diagnostic::on_unimplemented(/* section 2.3 */)]
pub trait CombineInput: sealed::CombineInputSeal {
    /// The type of the value the input holds, which the derivation reads.
    type Value: Clone + 'static;
}
impl<T: Clone + 'static> CombineInput for Signal<T> { type Value = T; }
impl<T: Clone + 'static> CombineInput for ReadSignal<T> { type Value = T; }
impl<B: CombineInput> CombineInput for &B { type Value = B::Value; }

/// The inputs of [`ReadSignal::combine`]: a tuple of 2 to 12
/// [`CombineInput`]s.
#[diagnostic::on_unimplemented(/* section 2.3 */)]
pub trait CombineInputs: sealed::CombineInputsSeal {
    /// The inputs' values as a tuple in input order, `(A::Value, B::Value, …)`;
    /// the derivation's function receives a reference to it.
    type Values: 'static;
}
// One impl per arity from 2 to 12, generated by a private macro:
impl<A: CombineInput, B: CombineInput> CombineInputs for (A, B) {
    type Values = (A::Value, B::Value);
}
// … up to (A, B, C, D, E, F, G, H, I, J, K, L).

impl<T: Clone + PartialEq + 'static> ReadSignal<T> {
    /// (section 5)
    pub fn combine<I: CombineInputs>(
        inputs: I,
        map: impl Fn(&I::Values) -> T + 'static,
    ) -> Result<Self, RuntimeError> { /* … */ }
}
```

- **Removed:** `ReadSignal<T>::combine<U, V>(&self, &ReadSignal<U>, impl
  Fn(&T, &U) -> V + 'static) -> Result<ReadSignal<V>, RuntimeError>`
  (RL:3575 and its prelude twin).
- **Sealing and the internal operations.** The public traits have
  associated types only. Their seals live in the existing private
  `sealed` module (`R/signal.rs:111-127`) and carry the crate's operations,
  each taking an unconstructible `sealed::Token` (a public struct in that
  private module with a private field), so they can be neither implemented
  nor meaningfully called outside the crate, although a generic caller's
  bound brings supertrait methods into scope:
  `CombineInputSeal::read_signal(&self, Token) -> &ReadSignal<<Self as
  CombineInput>::Value> where Self: CombineInput`, and
  `CombineInputsSeal::derive<T, F>(&self, Token, F) -> Result<ReadSignal<T>,
  RuntimeError> where Self: CombineInputs, T: Clone + PartialEq + 'static,
  F: Fn(&<Self as CombineInputs>::Values) -> T + 'static`; the output bounds
  are the public constructor's, which `publish`'s equality needs
  (`R/signal.rs:455`, `:537`), since an impl cannot add bounds its trait
  method lacks. Their signatures use only public
  types, so `private_interfaces` and `private_bounds` stay silent; the
  impls spell the projections in full (the impl for `&'a B` names `'a`),
  which the probe needed. The listing shows the seals only as supertrait
  paths, as `StateOwner`'s (RL, `pub trait pert_ui_runtime::StateOwner:
  pert_ui_runtime::signal::sealed::StateOwnerSeal`).
- **Implementation:** `map` and the macro share one private constructor of
  the derived node (initial value, compute closure, subscriptions), so the
  rules of section 2.2 live in one place; `derive` builds the tuple of
  cloned inputs, checks the runtimes, then follows `map`'s sequence.
  Behavior of `map` is unchanged.
- **`#[must_use]`:** none added; `Result` is already must-use.
- **Dependencies:** none; `pert-ui-runtime` keeps building without window or
  GPU dependencies.

## 4. Module membership and the prelude closure

- **`ReadSignal` is a prelude member** (`R/lib.rs:418`), so L2's closure
  rule applies to `combine` (plan, "Prelude closure";
  `layering-design-L2.md`, sections 1.2 and 1.3). `combine` returns
  `Result<Self, _>` and is a constructor. Its arguments are a tuple of
  `Signal`s and `ReadSignal`s, both members, and a closure over the
  application's own values; it returns `ReadSignal` and `RuntimeError`,
  both members. **No name joins the prelude.** `CombineInputs` and
  `CombineInput` are bounds an application satisfies by writing a tuple,
  never names it writes, the same exception as the conversion traits
  (`impl Into<TextSource>`): they stay at the crate root, and the prelude
  pages say so (section 5). Only generic code over inputs names them.
- **No thematic module, `component` or `testing`** holds them; a custom
  component derives with the same call and names neither trait. P2's
  `testing` rule is unaffected: no `testing` member reaches `ReadSignal`'s
  methods except through the prelude stop.
- **Collision review:** `CombineInput` and `CombineInputs` occur nowhere in
  the repository (`git grep` at `bfb11cd`), in no module of either crate
  and in no listed dependency path. `every_glob_combines`
  (`T/foundation/modules.rs:163-…`) lists module names, not root-only ones,
  so it does not change.
- **P2's listing check** (`D/tests/testing_closure.rs`) must pass on the
  regenerated listings. The tuple impls render as trait blocks for a
  non-pert owner (`impl<A: …> pert_ui_runtime::CombineInputs for (A, B)`,
  then `pub type (A, B)::Values = …`), which its parser treats like a
  primitive owner (the first segment is not a `pert_ui` path,
  `testing_closure.rs:554-560`), and the `&B` impl like
  `IntoElement for C` (RL:22526-22527). If the regenerated listing shows a
  form the parser reports as unknown, the implementer adds that form with a
  self-test in the same part (same-kind fix) and records it.

## 5. Documentation

All in the commit that changes the code (plan, "Text describes the code of
its own commit"):

- **`ReadSignal::combine`'s rustdoc:** the rules of 2.1 and 2.2 in prose:
  inputs (the forms, same runtime, the limit 2 to 12 with the two ways past
  it), the function's argument (`|(a, b)|` binds references; an annotated
  closure names `&(A, B)`; two parameters fail), the timing-class line,
  at-most-once per batch, equal results (no revision and no component
  notified, while a derived binding over the result still evaluates again,
  rule 5), retention and release, repeated
  inputs; `# Errors` listing what the call itself returns, in order:
  `ForeignRuntime` (first, before anything is read), `Closed`, the four
  phase errors, then the first input read error in tuple order, such as
  `BorrowConflict` for a reentrant read. A separate paragraph, not under
  `# Errors`, states rules 8 and 9: a write inside the function is refused
  with `WriteDuringUpdate` returned to the function, and a later
  evaluation's errors come from the read that settles it. `map`'s rustdoc
  gets the same split (section 5, same kind). A runnable example:
  a settings form's dirty flag over three field signals and an applied
  record, `ReadSignal::combine((&name, &port, &secure, &applied), |(name,
  port, secure, applied)| (name, port, secure) != (&applied.0, &applied.1,
  &applied.2))`, asserting `false`, a write, `true`.
- **`CombineInput`'s rustdoc:** what implements it, that the trait is
  sealed and named only by generic code; a runnable example mixing
  `&signal`, `&binding`, an owned binding a call returns and an owned
  signal clone in one tuple.
- **`CombineInputs`'s rustdoc:** the limit and why there is no 1-tuple; a
  runnable 12-tuple; `compile_fail` doctests for a 13-tuple and for
  `(&count,)`, each with a comment that it fails for arity; a generic
  example, `fn changed<I>(inputs: I, applied: I::Values) ->
  Result<ReadSignal<bool>, RuntimeError> where I: CombineInputs,
  I::Values: PartialEq`, comparing the values with an applied tuple.
- **Same kind, in touched code:** `map`'s `# Errors` gains the `Closed` and
  phase errors it already returns (section 1), and its sentence on writes
  from the function moves out of `# Errors` into the operation-level
  paragraph, as for `combine` (`R/signal.rs:744-746`); `combine` gains the
  example it lacked.
- **`Signal`'s page** (`R/signal.rs:184-191`): `&signal` works wherever
  `&ReadSignal` is taken, such as `UpdateContext::read`; `map` derives from
  it directly; and it is one input of `ReadSignal::combine`'s tuple as it
  is ([`CombineInput`]).
- **The prelude pages** (`R/lib.rs:374-376`, `F/lib.rs:98-100`), in the
  conversion bullet: `ReadSignal::combine` takes its inputs as a tuple of
  signals and read bindings; `CombineInputs` and `CombineInput`, which only
  generic code names, are at the crate root.
- **`docs/contracts.md:613-614`:** "Binding derivations (`ReadSignal::map`,
  and `ReadSignal::combine` over two to twelve bindings)".
- **`G/state.md`,** after the paragraph on coalesced writes (`:76-79`): one
  paragraph saying that a value derived from several bindings is one
  `ReadSignal::combine` over a tuple of them, with an inline call, evaluated
  at most once per batch of writes however many inputs changed, an equal
  result updating no component that reads it (a binding derived from it
  still evaluates, rule 5). No recipe: P12 writes the settings recipe.
- **`plans/follow-ups.md`, "Writable model projection"** (`:27-32`):
  round 2's evidence: both task 1 runs bound each control to its own field
  signal and derived the record with chained `combine` calls, which P10's
  tuple form replaces; neither needed to write through a projection, so the
  follow-up stays for a control bound to a field of one model signal, with
  its validation, revision and conflict rules.
- **Migrated texts** (section 6): `R/described.rs:169` (a doctest),
  `R/collection_filter.rs:567-574` (inline code), `G/layout.md:629`.
  `R/described.rs:22` and `G/layout.md:518` link `ReadSignal::combine`
  and stay valid.

## 6. Migration manifest

The replacement is mechanical: `a.combine(&b, |x, y| body)` becomes
`ReadSignal::combine((&a, &b), |(x, y)| body)` with the body unchanged; a
chain that builds a tuple becomes one call over all its inputs; `.read_only()`
on an input is dropped. Search at P10's base with
`ast-grep run --lang rust --pattern '$R.combine($$$A)' crates apps
plans/usability/task4-app` and `rg -n 'combine\(' crates apps docs
plans/usability/task4-app` (rustdoc and Markdown text); at `bfb11cd`
they find exactly the rows below.

| Consumer | Site at `bfb11cd` | P10 |
|---|---|---|
| Runtime | `R/permission.rs:213-215`, a fold over the conditions | `combined = ReadSignal::combine((&combined, condition), \|(a, b)\| *a && *b)?`: the count is dynamic, so it stays a pairwise fold |
| Runtime | `R/tool_windows/options.rs:87` | `ReadSignal::combine((&holds, &permission), …)` |
| Runtime | `R/tool_windows/options.rs:129-133`, a chain | one call, `ReadSignal::combine((&current, &builder.floatable, &builder.detachable), move \|(mode, float, detach)\| may_switch(*mode, target, *float, *detach))` |
| Runtime | `R/file_browser/controller.rs:559-578`, a chain through `shown` | one call over `(&current, &listings, &filter)`; `shown` goes, the body and its cache are unchanged |
| Runtime | `R/multi_select.rs:408`, `:440` | two-input calls |
| Runtime rustdoc | `R/described.rs:169` (doctest), `R/collection_filter.rs:567-574`, `R/signal.rs:184-191` | migrated; section 5 |
| Examples | `inventory/app.rs:161-171`, a chain | one call over `(selection.selected(), &query, &published)` |
| Examples | `diagram_editor/app.rs:774`, `diagram_editor/editor.rs:122` | two-input calls; `editor.rs` passes `self.nodes.selected()` by value |
| Headless sessions | the five packaged examples | unchanged; they run in the gate |
| Gallery | `GA/pages/text.rs:650` | `ReadSignal::combine((&query, &highlight), …)` |
| Guide | `G/layout.md:629`; `G/state.md` | migrated; one paragraph added (section 5) |
| Facade tests | `T/foundation/foundation.rs:139`, `:147`, `:359` | migrated; the `ForeignRuntime` assertion keeps its meaning |
| Facade tests | `T/foundation/main.rs` | registers the new `combine` module (section 7) |
| Contracts | `docs/contracts.md:613-614` | section 5 |
| Diagnostics | `D/ui/` | two new cases and snapshots (section 7) |
| Fixture | `plans/usability/task4-app/src/lib.rs:66`, `:72`; `Cargo.toml:8` | migrated; the dependency requirement becomes `0.3.85` |
| Native checks, platform code, `docs/manual.md`, READMEs | none | unchanged: no `combine` |
| Follow-ups | `plans/follow-ups.md:27-32` | section 5 |
| Frozen: the submissions and probes under `plans/usability/runs/` (such as `r2-1-claude/submission/src/lib.rs:164-176`) and U; archived and audit design notes (`plans/usability/api-audit/design-6f2.md:533`); earlier CHANGELOG entries | — | not migrated: history |

## 7. Tests

- **New-behavior tests** each name the assertion that detects a wrong
  implementation and a compiling mutation that must make it fail; a test
  that fails only because a new item does not compile is not evidence.
- **Contract and compatibility tests** check rules P10 keeps.

The facade tests are one new module, `T/foundation/combine.rs`, in the
`foundation` binary beside today's derivation tests
(`T/foundation/foundation.rs:100-149`, `:354-375`). Each counts the
function's calls in a `Rc<Cell<usize>>` it captures.

### New-behavior tests

1. **Five inputs evaluate once per batch** (`combine.rs`). Field signals
   `prefix = "img"`, `format = 1_u8`, `quality = 80_u8`, `width = 640_u32`,
   `open = false`; `dirty = ReadSignal::combine((&prefix, &format,
   &quality, &width, &open), move |values| { count; *values != applied })`
   with `applied` the initial tuple captured. An observer, a test component
   whose `update` reads `dirty` with `UpdateContext::read`
   (`R/runtime.rs:2941`) and counts its updates in a shared `Cell`, is
   mounted and flushed. It observes `dirty` itself, with no derived node
   between, so no other equality check stands between the combined node
   and the component. Steps and named
   assertions (the update count is the observer's):
   (a) after creation and the first flush, one call;
   (b) all five inputs written to other values: still one call before the
   flush (writes run nothing), two after it, one observer update,
   `dirty.get()? == true`;
   (c) only `open`, the last position, written back: three calls after the
   flush and no observer update (equal result);
   (d) the other four written back: four calls, one observer update,
   `dirty.get()? == false`.
   Every input write in (b) to (d) changes its input's value, so the only
   publication on the path from the inputs to the observer that takes the
   equality return is the combined result's in (c).
   Mutations: evaluating in `Derived::invalidate` (`let _ = self.refresh();`
   after setting `dirty`) makes (b) count six before the flush; subscribing
   only the first four positions in the macro leaves (c) at two calls;
   removing `publish`'s equality return (`R/signal.rs:543-545`) makes (c)
   update the observer. That mutation is global, but by the previous
   sentence it changes only the combined node's publication in this test,
   which is what the assertion needs.
2. **Inputs are retained and released with the last result.** Three
   inputs, the first two passed **by value** so the test keeps no strong
   handle to them: `doubled = a.map(|v| v * 2)?`; `held`, a
   `Signal<Rc<()>>` created from an input token the test then drops,
   keeping only its `Weak`; and `&b`. The function captures a second token,
   the mapper token, whose `Weak` the test keeps, and returns
   `d + b` (the `Rc` it reads is a temporary clone, dropped after each
   evaluation). After `a.set(5)?`, `sum.get()? == 10 + b`. Then
   `let copy = sum.clone(); drop(sum);`: both tokens are alive and, after
   `a.set(6)?`, `copy.get()? == 12 + b`. `drop(copy)` makes both `Weak`s
   `None`, and later writes to `a` and `b` and a flush succeed. Named
   assertions: `copy` follows `a` after the original is dropped; the input
   token survives until the last result drops and is gone after it; the
   mapper token likewise. Mutations: the macro storing each input as
   `ReadSignal { inner, computation: None }` (the value cell without its
   computation) leaves `copy` at the stale doubled value; a
   `std::mem::forget` of a clone of the tuple of retained inputs in
   `derive` (the inputs leak while the node and the mapper drop) keeps the
   input token alive with the mapper token gone; a `std::mem::forget` of a
   clone of the node keeps both alive.
3. **A binding of another runtime is refused before anything runs.**
   `(&a, &b, &foreign)` and `(&foreign, &a, &b)`, with `foreign` from a
   second `Runtime`: both return `Err(RuntimeError::ForeignRuntime)` and
   the function was called zero times. A third case passes a **dirty
   derived input**, `(&doubled, &foreign)` with `doubled = a.map(..)`
   counting its own calls and `a` written just before: the call returns
   `ForeignRuntime` and `doubled`'s function was not called (the input was
   not read, so not refreshed). Mutations: comparing only the first two
   positions accepts the first tuple; evaluating before the check counts one
   call; reading the inputs before the check (for example building the
   initial tuple first) refreshes `doubled`.
4. **The arity is 2 to 12.** On `CombineInputs`' page: `compile_fail`
   doctests for a 13-tuple and for `(&count,)`, beside a runnable
   12-tuple. Mutation: adding the 13 or the 1 arity to the macro's list
   makes the corresponding doctest compile, which fails it. The messages
   are checked by the snapshots below, not by `compile_fail` (AGENTS.md,
   "Verification and review").

### Contract and compatibility tests

- **The migrated derivation tests** (`foundation.rs:100-149`, the diamond at
  `:354-375`) pass unchanged in meaning: batching through a chain, an
  untracked chained read current before the flush, a diamond consistent,
  `ForeignRuntime` for two runtimes. Their captured-value and
  write-refusal assertions (`:361-375`) exercise `map`, not `combine`, and
  stay as they are; the next items cover `combine`.
- **A write inside `combine`'s function is refused** (rule 8, `combine.rs`):
  `guarded = ReadSignal::combine((&a, &b), move |_| writer.set(9))?` with
  `writer` a clone of `a`: creation succeeds, `guarded.get()?` is
  `Err(WriteDuringUpdate)` and `a` is unchanged (initial evaluation); after
  `b.set(..)?`, `guarded.get()?` is again `Err(WriteDuringUpdate)`, the
  function ran a second time (counted) and `a` is still unchanged (later
  evaluation). Mutation: dropping the evaluation guard around `derive`'s
  initial evaluation lets the first write succeed (`a == 9`).
- **Phase admission and refusal through `combine`** (rule 2, `combine.rs`):
  a custom component holding two signals calls `ReadSignal::combine` in its
  `update` and in its `paint` (`Component::paint(&self, &mut Painter)`,
  `R/runtime.rs:2196`), recording each result in a shared
  `Rc<RefCell<Vec<_>>>`; after mount, flush and `ui.paint()`, the update's
  call was `Ok` and the paint's `Err(WriteDuringPaint)`. Mutation: removing
  the phase check from `derive` makes the paint's call `Ok`.
- **`ForeignRuntime` precedes `Closed`** (rules 1 and 2): with `ui`
  shut down (`Runtime::shutdown`, `R/runtime.rs:6569`) and a second
  runtime open, `ReadSignal::combine((&a, &foreign), ..)`, `a` from `ui`
  and `foreign` from the open runtime, returns
  `ForeignRuntime`, not `Closed`. Mutation: checking the phase of the first
  input before the runtimes returns `Closed`.
- **What an equal result stops** (rule 5, `combine.rs`): `flag =
  ReadSignal::combine((&a, &b), |(a, b)| a + b > 10)`, a `map` over `flag`
  counting its calls, and an observer component reading `flag` directly
  (as in test 1), mounted and flushed; then `a` and `b` written to values
  that leave `flag` equal, and a flush. Named assertions: the downstream
  `map` ran once more (today's algorithm: dependents are marked dirty
  before the node evaluates, `R/signal.rs:704-707`), and the observer was
  not updated. It records the semantics the rustdoc states; a change that
  stopped downstream evaluation would be a new contract, not this part.
- **Input forms and repetition** (`combine.rs`): one tuple of `&signal`,
  `&read_binding`, an owned `ReadSignal` a call returned and an owned
  `Signal` clone evaluates and follows each; `(&a, &a)` follows `a` and
  drops cleanly.
- **The prelude alone** (`combine.rs`, a child module importing only
  `pert_ui::prelude::*`): `ReadSignal::combine((&a, &b), |(a, b)| a + b)`
  compiles and evaluates, so an application needs no root import (section 4).
- **After shutdown** `ReadSignal::combine` returns `Closed` (rule 2).
- **Doctests:** `combine`'s, `CombineInput`'s and `CombineInputs`' examples;
  the migrated `R/described.rs` doctest.
- **Diagnostics snapshots** (`D/ui/`, run with `cargo test --locked -p
  pert-ui-diagnostics -- --ignored`): `d01_combine_single_input.rs`
  (`ReadSignal::combine((&count,), |(n,)| *n)`) and
  `d02_combine_not_a_binding.rs` (`(&count, 3_u32)`), each importing
  `pert_ui::prelude::*`, with their `.stderr` generated by
  `TRYBUILD=overwrite` and reviewed: the messages, labels and notes of
  section 2.3. No existing snapshot changes; if one does, the diff is
  reviewed and recorded.
- **Existing behavior** of the migrated runtime sites: the runtime's and
  the facade's tests of the file browser, multi-select, tool-window menus
  and `Permission::signal`, the gallery tests and the `inventory` and
  `diagram_editor` headless sessions pass unchanged.
- **P2's listing check** passes on the regenerated listings (section 4).
- **Fixture check** (plan, "Two checks recur"): a kit from P10's commit
  (`plans/usability/run.sh prepare <kit-dir>`) and `cargo test` in a copy of
  `plans/usability/task4-app` with `run.sh exec`.

## 8. Scope, version and evidence

- **`api/` diff** (`api/pert-ui-runtime/default.txt` and `serde.txt`; the
  facade listings re-export the runtime root by one glob line,
  `api/pert-ui/default.txt:2`, and do not change): removed the method
  `ReadSignal<T>::combine<U, V>(&self, …)`; added the associated function
  `ReadSignal<T>::combine<I: CombineInputs>(I, impl Fn(&I::Values) -> T +
  'static) -> Result<Self, RuntimeError>`, the traits `CombineInput` with
  `Value` and its impls for `Signal<T>`, `ReadSignal<T>` and `&B`, and
  `CombineInputs` with `Values` and its impls for the tuples of 2 to 12,
  each at the root, and `combine` at each of `ReadSignal`'s three listing
  paths: `component`, `prelude` and the root (RL:3573-3575, `:20025`,
  `:36967`).
- **Breaking:** yes, `combine`'s form.
- **CHANGELOG** for 0.3.85, citing this checkpoint:
  - `Breaking (R2-3):` `ReadSignal::combine` is an associated function
    over a tuple of 2 to 12 signals or read bindings, whose function
    receives a reference to the tuple of their values. Replacement:
    `a.combine(&b, |a, b| ...)` becomes `ReadSignal::combine((&a, &b),
    |(a, b)| ...)` with the body unchanged; a chain building a tuple
    becomes one call, `ReadSignal::combine((&a, &b, &c), |(a, b, c)|
    ...)`; `.read_only()` on an input is no longer needed; an annotated
    closure names the tuple, `|values: &(A, B)|`. rustc's suggestion for
    the old call, `ReadSignal::<T>::combine(&b, f)`, is not the
    replacement; the error that follows it names the tuple form.
  - `Added (R2-3):` `CombineInput` (`Value`), implemented for `Signal`,
    `ReadSignal` and references to them, and `CombineInputs` (`Values`),
    implemented for tuples of 2 to 12 inputs; sealed, at the crate root,
    not in the prelude (an application writes a tuple, not the trait), with
    compiler messages for a tuple outside the limit and an element that is
    not a binding.
  - `Changed:` the file browser, the tool-window menus and the `inventory`
    example derive with one `combine` where they chained two (no behavior
    change).
  - `Documentation (R2-3):` `map`'s errors name `Closed` and the phase
    errors (same kind); `Signal`'s page; the prelude pages;
    contracts.md's evaluation class; guide/state.md; plans/follow-ups.md's
    "Writable model projection".
  - `Tests:` section 7, and the diagnostics cases `d01`, `d02`.
- **Version:** 0.3.85 after P9's 0.3.84: the workspace version, the internal
  version requirements and `Cargo.lock`; the task 4 fixture's requirement
  follows. If the order changes, the heading follows the actual version.
- **Gate:** the public-API row, as the plan states: the workspace checks
  (formatting, Clippy, tests including section 7 and P2's listing check,
  the packaged examples' headless sessions, doctests, warning-free rustdoc,
  the diagnostics snapshots, which the `on_unimplemented` messages make
  required), the API listing gate with its diff reviewed against the
  CHANGELOG, the fixture check, the mutations of section 7 recorded in the
  Progress entry, and Codex's implementation review.
- **Native evidence:** not applicable. P10 changes no platform, rendering or
  input code; derived bindings behave identically headless and native, and
  the migrated runtime sites keep their behavior (section 7).
- **Persistence, real time, backends:** no persisted schema, live-data
  endpoint or backend type is involved; signals stay UI-thread local.
- **TODO.md:** the `diagram_editor/app.rs` `close_requested` entry is P12's;
  the `R/described.rs` `Each::new` entry is P5's. The entry on rustdoc
  and guide examples with unused imports (`TODO.md:153-168`) lists none of
  the examples P10 edits; P10's new and migrated examples import nothing
  they do not use.

## Corrections to the plan

- **The impl block's parameter is the output.** D10's "associated function"
  compiles as `ReadSignal::combine(...)` only in an impl whose type
  parameter is the result type (section 2.1); otherwise every call needs a
  turbofish.
- **The function's argument.** D10's replacement `|(a, b)| ...` holds for a
  function of `&(A, B)` through default binding modes; an annotated closure
  names `&(A, B)`, not `(&A, &B)` (section 2.1, P10-D1).
- **The consumer list** omits the runtime's rustdoc text and doctest
  (`R/described.rs:169`, `R/collection_filter.rs:567-574`,
  `R/signal.rs:184-191`) and the two prelude pages; section 6 adds them,
  and the native checks, which have no call.
- **The arity test is two kinds of evidence.** A `compile_fail` doctest
  proves the restriction, not the message (AGENTS.md); the messages are
  trybuild snapshots, so the plan's conditional "if the tuple trait gets an
  `on_unimplemented` message" is met and the snapshots are required. The
  13-tuple is not snapshotted (section 2.3).
- **"Keeps today's semantics" (D10) is a clarification, not a change:**
  D10 lists "equal results invalidate nothing". In today's algorithm, which
  P10 keeps unchanged for `map` and `combine`, an equal result suppresses
  the publication, the revision advance and the notification of components
  subscribed to it, but a derived binding over it has already been marked
  dirty and evaluates again (`R/signal.rs:704-707`, `:783`, `:823`). Rule 5,
  the rustdoc, the guide paragraph and a contract test say so.
- **"Inputs are released"** needs evidence for both the function and the
  inputs: test 2 observes the function's captured token and an input
  signal whose value holds a token, each through a `Weak`, with retention
  of a by-value derived input as the counterpart.

## Findings outside this part

None. The two same-kind gaps found in touched code (`map`'s incomplete
`# Errors`, `combine`'s missing example) are fixed here (section 5).

## Not in this part

- **A writable field projection:** a follow-up, updated with round 2's
  evidence (section 5).
- **`combine` over a slice or `Vec` of bindings of one type**
  (`Permission::signal`'s case): one call site, served by its fold; no run
  needed it.
- **Avoiding the clone of each input per evaluation:** today's `map` and
  `combine` clone too (`R/signal.rs:766`, `:778`, `:810`, `:821`); P10-D1
  keeps that cost and claims no change to it.

## Decisions

- **P10-D1: what the function receives.**
  - **Recommended: a reference to the tuple of cloned values,
    `Fn(&(A, B, …)) -> T`.** It matches `map`'s `Fn(&T)` and keeps today's
    evaluation model: each input is read with `get`, which clones and
    releases its borrow before the function runs, so no input `RefCell` is
    borrowed while application code executes and no publication path that
    borrows mutably (`publish_trusted`, `R/signal.rs:489-490`) can meet a
    held borrow. The signature has no lifetime-generic associated type, the
    `|(a, b)|` pattern binds references as today's parameters were, and
    `CombineInputs::Values` is a plain tuple a generic caller can compare
    (`I::Values: PartialEq`, section 5's example). Cost: one clone per input
    per evaluation, as today, and a closure written with two parameters gets
    rustc's generic E0593 message.
  - **Alternative A: a tuple of references, `for<'a> Fn((&'a A, &'a B, …))`,**
    through a generic associated type. The probe compiled it; it could read
    the inputs without cloning (and drop the `Clone` bound on inputs), and
    rustc then suggests the tuple pattern for a two-parameter closure. It
    holds every input's borrow while application code runs, so each
    mutable publication reachable from inside an evaluation would need an
    audit against a panic, and it puts a higher-ranked projection in the
    signature and its diagnostics, for a saving no measurement has asked
    for.
  - **Alternative B: the values by value, `Fn((A, B, …))`.** The function
    could move values out, but every migrated body that dereferences
    (`*a && *b`) changes, so the migration is no longer mechanical, and it
    diverges from `map`.

## Review record

- **Revision 1 (2026-10-28):** written from the code at `bfb11cd` (library
  code at P3, 0.3.78; `R/signal.rs` as at 26e6b5a), with the signature,
  sealing, messages and arity limit compiled in a standalone probe on the
  pinned toolchain outside the workspace.
- **Revision 1, Codex review (2026-10-28, against `7bff14d`, whose library
  code is `bfb11cd`'s):** needs revision, five should-fix points and a nit;
  the tuple direction, the arity, the prelude placement, the migration and
  the persistence boundary were found to fit the plan. Its record is
  `~/pert-ui-usability/codex-prompts/codex-design-P10.out`. Each point was
  checked against HEAD before revision 2 applied it; none was rejected.
  1. **The sealed `derive` lacked the output bounds:** `publish` needs
     `PartialEq` (`R/signal.rs:455`, `:537`) and an impl cannot add bounds.
     Section 3 states `T: Clone + PartialEq + 'static`; the probe's seal
     already had them, so the probe did not change and was not rerun.
  2. **The equality test passed through a `map` that filters equality
     itself:** test 1 now observes `dirty` with a counting component that
     reads it directly, and states why the global `publish` mutation then
     changes only the combined node's publication (every input write
     changes its value).
  3. **Release of the inputs was not evidenced:** test 2 adds an input
     signal whose value is a token, held by no other owner, observed
     through a `Weak`, with a mutation that leaks the retained inputs while
     the mapper drops; the plan correction no longer says "only".
  4. **The new evaluation path lacked contract tests:** added a dirty
     derived input not refreshed by the `ForeignRuntime` refusal (in test
     3), `ForeignRuntime` before `Closed`, a write refused inside
     `combine`'s function at the initial and a later evaluation, and phase
     admission in `update` and refusal in `paint`; the migrated
     write-refusal assertion is attributed to `map`
     (`T/foundation/foundation.rs:370-375`).
  5. **Constructor errors and refused writes were conflated:** rule 8 and
     section 5 put the refused write (returned to the function, stored in
     its value) outside `# Errors`; new rule 9 places later evaluations'
     errors at the read that settles them; `map`'s rustdoc gets the same
     split (same kind).
  6. **Nit:** the volatile count of runtime call sites is gone; the `api/`
     diff names `ReadSignal`'s three listing paths (RL:3573-3575,
     `:20025`, `:36967`); the TODO note says the unused-imports entry
     (`TODO.md:153-168`) lists none of P10's examples.
- **Revision 2, Codex recheck (2026-10-28):** all six points resolved; one
  new should-fix point. Its record is
  `~/pert-ui-usability/codex-prompts/codex-design-P10-r2.out`. Checked
  against HEAD and accepted: `Derived::invalidate` marks dependents dirty
  on the node's first invalidation, before any evaluation
  (`R/signal.rs:704-707`, wired at `:783` and `:823`), so an equal combined
  result does not stop a downstream derived binding from evaluating again;
  equality suppresses only the publication, the revision advance and
  component notification. Revision 3 qualifies rule 5, the rustdoc and the
  guide paragraph prescriptions, records it under the plan corrections as
  a clarification of D10's "keeps today's semantics" with the algorithm
  unchanged, and adds the contract test "What an equal result stops".
  Nothing else changed.
- **Approval (2026-10-28):** revision 3 changes only the qualification
  Codex's recheck asked for; approved by the author under the maintainer's
  delegation, with P10-D1 as recommended.
