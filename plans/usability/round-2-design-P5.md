# Round 2 fix plan, part P5 design checkpoint: rows in display order

Part P5 of [the round 2 fix plan](round-2-fix-plan.md#p5-rows-in-display-order-0380)
closes F9 and R2-6 by decision D4 (approved with the recommended option on
2026-10-28) and the TODO.md entry on `Each::new`'s placeholder key (D4's
sub-decision). The plan leaves to this checkpoint the operation's definition
and name, the keyless conversion and its coherence, and the migration list;
this document settles them, and the documentation and tests they need,
before any code. Codex reviews it; the author approves it together with
Codex's review under the maintainer's delegation of 2026-10-28.

Every reference to library code is to the committed tree at 5942c54, whose
library code is unchanged since 26e6b5a (0.3.75), the commit the plan cites;
P1's uncommitted working-tree edits are not cited, and guide line numbers
are re-located by content when P5 starts. Path prefixes: `R/` =
`crates/pert-ui-runtime/src/`, `F/` = `crates/pert-ui/src/`, `G/` =
`crates/pert-ui/src/guide/`; **T** `R/testing.rs`, **TR**
`R/runtime/tree.rs`, **D** `R/described.rs`, **L** `R/layout.rs`, **VL**
`R/virtual_list.rs`, **U** `~/pert-ui-usability/` (frozen runs).

P5 lands after P2 (glob membership and the listing-driven `testing` check;
`plans/usability/round-2-design-P2.md`, revision 3, approved at 07eb0a4),
P3 and P4 (focus and idle in the driver;
`plans/usability/round-2-design-P4.md`, revision 3, approved at 5942c54).
Section 5 states what P5 takes from them.

Status: **revision 3, approved 2026-10-28** by the author together with
Codex's review (second recheck: ready for approval). Revision 1 was reviewed by Codex (needs
revision: two blocking points, one should-fix, one nit); revision 2 answered
each, and Codex's recheck left one should-fix point, which revision 3
answers ([review record](#review-record)). The author approves P5-D1 and P5-D2
as recommended, under the maintainer's delegation of 2026-10-28 (plan,
status line); the checkpoint was approved once Codex found it ready.

## What testers built by hand, and what the library says

- **Rows in display order (F9, results row at
  `plans/archive/developer-usability.md:432`, round 2 at `:890-896`).** Both
  task 4 runs addressed rows correctly as `list/<key>`, and both had to walk
  around `Each`'s default container to read them in order:
  - 4-codex read `Runtime::children` of the `Each`, asserted that the one
    child is keyed `items` and read its children
    (U `r2-4-codex/work/bookmarks/tests/ui.rs:76-88`), relying on an
    internal key no address reaches (triage `runs/triage-r2/group-2.md:40`);
  - 4-claude walked every descendant, kept those that `find(list, [key])`
    resolves to themselves, and sorted them by `bounds().y()`
    (U `r2-4-claude/work/bookmarks/tests/behaviors.rs:101-134`).
  The testing chapter shows display order only with `within`
  (`G/testing.md:236-261`); its other example uses `Selector::children`,
  whose results are in incarnation order (`R/inspection_service.rs:371`).
- **What `Runtime::children` lists (R2-6, results row at
  `plans/archive/developer-usability.md:924`).** "Ordered immediate children,
  including locally or ancestrally hidden ones" (TR:137) says nothing about
  structural wrappers, which it lists and addresses skip (triage group 2 N1,
  `runs/triage-r2/group-2.md:133`), or about a `VirtualList`, whose children
  are its mounted rows in index order (group 1 N7,
  `runs/triage-r2/group-1.md:242`).
- **The placeholder key (TODO.md:136-145).** `Each::new`'s `build` returns an
  `Element` (D:347), and `Each` overwrites its key (D:350-353), so every
  caller writes a key that never takes effect: `Each`'s own examples
  (`.key("task")`, D:290; `.key("tag")`, D:310), the guide (`G/layout.md:405`,
  `G/testing.md:221`, `:248`), the gallery (`apps/gallery/src/pages/chips.rs:70`)
  and the task 4 fixture (`plans/usability/task4-app/src/lib.rs:117`,
  `.key("row")`).

## 1. Today's behavior, from the code

- **Addressing markings.** An element's `Addressing` is `Own` (its key is its
  segment), `Structural` (no segment, for itself or its descendants), `Slot`
  (its slot name, for itself and its descendants) or `Part` (its part name,
  for itself only; descendants address as if it were structural)
  (L:694-752). The marking is crate-private (L:742); the public builders are
  `Element::structural`, `slot` and `part` (L:707, :720, :733).
- **`Runtime::find`** (TR:225-345) walks from `root`'s children, matching one
  segment per addressable level: `Own` and `Slot` contribute a segment and
  are inherited, `Part` contributes one and is not inherited, `Structural`
  contributes none (TR:319-324). A match on the last segment by an `Own` or
  `Slot` element stops the descent below it; a `Part` is descended whether
  or not it matched (TR:332-342). It skips the drag preview's root and its
  subtree (TR:305, :315-317) and returns `AmbiguousKey` when two components
  match (TR:326-329). Its traversal is a stack (TR:314), so it has no order.
- **The drag preview** is mounted as a root of its own
  (`R/runtime/drag.rs:348-350`, `mount_tree`), so it is never a descendant
  of a component an application mounted; `find`'s skip is defensive.
- **`Runtime::children`** (TR:137-149) returns the entry's logical children
  in order; **`Runtime::key`** (TR:127-136) returns the element's key, which
  for a slot or part is not its segment and for a structural wrapper is no
  segment at all.
- **`Each`** (D:258-406): its default container is
  `Column::new(children).key("items").structural()` (D:355); `within`'s
  element is used as it is (D:373-379), so its key is a segment. Each item
  is `build`'s element with its key replaced, its other properties (label,
  grow, addressing marking) kept (D:350-353).
- **`VirtualList`** requests its rows as its own children, keyed by the
  source's key (VL:2003-2009), in `VirtualAxis::mount_order`: pinned rows
  before the window, the window (visible lines plus overscan, VL:822-827),
  pinned rows after it, by index (`R/virtual_axis.rs:403-425`). Pinned are a
  row with focus within or pointer capture and the `keep_mounted` row
  (VL:1988-1997, :896-905). A row outside that set is not mounted; a
  `ListController` reveal mounts it (VL:347, VL:400-402).
- **`Table`**'s children are its body, a structural `VirtualList` over its
  rows with `keep_mounted` on the edited row (`R/table.rs:1541-1547`), and a
  structural edit-end reporter without children (`R/table.rs:2262-2277`).
  Each row is `Element::new(row_key, TableRow { .. })` (`R/table.rs:541-546`);
  rows are in the application's order (`R/table.rs:241`); a
  `TableController::reveal` mounts one (`R/table.rs:906`).
- **`Selector::Children(path)`** matches each component whose workspace path
  is `path` plus one segment (`R/runtime/inspect.rs:390-392`), in
  incarnation order (`:367-368`); it needs an addressable parent's path.
- **`Driver`** keeps no state and reads through public `Runtime` calls
  (`find`, T:352-365; `semantics`, T:367-393). `TestError` is
  `#[non_exhaustive]` and keeps a runtime failure as `Runtime` (T:702-713).

## 2. The rules

### 2.1 `Driver::address_children`

`driver.address_children(&parent)` lists the components one address segment
below `parent`, in child order, each with its segment.

1. **Walk.** Starting from `parent`'s children, depth first in pre-order
   over each component's logical children in order (the order
   `Runtime::children` lists), each visited component is classified by its
   marking, exactly as `find` classifies it (TR:319-324):
   - `Own` (its key) and `Slot` (its slot name): **returned** with that
     segment; its descendants are not visited;
   - `Part` (its part name): **returned** with that segment, and its
     descendants are visited, since they address without it;
   - `Structural`: not returned; its descendants are visited;
   - the drag preview's root: neither returned nor descended, as in `find`
     (rule 6).
2. **Equivalence with `find`, the contract the rustdoc states.** A component
   is returned with segment `s` exactly when `Runtime::find(&parent, &[s])`
   matches it: `find` returns it, or fails with `AmbiguousKey` because a
   second returned component has the same `s`. Both are returned, in child
   order; the rustdoc says that `find` refuses that address. This relative
   contract is the definition; nothing else defines the result.
   **The relation to `Selector::children` is restricted** and stated only
   in this form: when `parent` is marked `Own` or `Slot`, has the workspace
   path `p`, and no other `Own`- or `Slot`-marked component in the runtime
   has the path `p`, the returned set is the set `Selector::children(p)`
   matches. Why exactly then: a component's path is its segment after the
   segments its `Own` and `Slot` ancestors contribute, parts and structural
   levels contributing none (`R/runtime/workspace.rs:257-274`), so a path
   `p` plus one segment ends below the nearest `Own` or `Slot` ancestor whose
   path is `p`, reached through structural and part levels only, which is
   what rule 1 walks; the selector compares absolute paths
   (`R/runtime/inspect.rs:390-392`), runtime-wide. Outside that case they
   differ, and the rustdoc says so in one sentence: below a part, whose
   name its descendants do not carry, `Selector::children` of the part's
   path finds none of what `address_children(&part)` lists; and when two
   components share the path `p` (a duplicate address through structural
   levels), `Selector::children(p)` also returns the other one's children.
3. **Relative to `parent`, not to its address.** A structural `parent`, a
   slot or a part is valid; the result is what `find` resolves from it. The
   parent's own marking does not matter, because `find` starts below it.
4. **What is included**, as for `find` and `children`: hidden components
   (TR:137), components presented in a detached window, and every mounted
   row a virtualized collection retains offscreen. **Not included:** a row
   that is not mounted; the documented route to it is the collection's
   controller (`ListController::reveal`, `TableController::reveal`), which
   mounts it, after which a settle lists it.
5. **Errors:** `TestError::Runtime` with `ForeignRuntime` or `Unmounted` for
   `parent`, as `Runtime::children`. A leaf returns an empty list. Nothing
   else fails: the call reads, flushes nothing and moves no clock.
6. **The drag preview** cannot be reached from an application's component
   (section 1), so rule 1's skip never applies in practice; it keeps the
   walk and `find` one definition, and the rustdoc does not mention it.
7. **Cost:** one visit per component reachable from `parent` through
   structural and part levels, plus one `String` per returned component; no
   inspection query runs.

**Implementation, no second definition.** `find`'s per-entry match
(TR:319-324) moves into one crate-private function of `tree.rs` returning
the entry's segment and whether descendants inherit it; `find` and a new
crate-private `Runtime::address_children_of(&self, &WidgetId) ->
Result<Vec<(String, WidgetId)>, RuntimeError>` beside it both call it, so
the two cannot drift. The walk is an explicit stack with children pushed in
reverse, which gives pre-order. `Driver::address_children` wraps it. The
markings are crate-private (L:742), so no public call reproduces the walk;
the public routes stay `find` for one segment and, for tooling,
`Selector::children` by path, with the difference rule 2 states ([Not in
this part](#not-in-this-part)).

### 2.2 What it gives per collection

Each result is a test in section 7 and a sentence on the page named.

| Parent | Result |
|---|---|
| `Each`, default container | its rows in list order, each under its item key (the container is structural) |
| `Each` with `within` | the one container, under its key; `address_children(&driver.find(&each, &["tags"])?)` gives the rows |
| `VirtualList` | every mounted row in index order: the visible lines, the overscan, a row pinned by focus or a pointer gesture, the `keep_mounted` row |
| `Table` | its mounted rows in source order, with the same retention; the body and the edit-end reporter are structural and the reporter has no children |
| a row the application marks `Element::structural` | the row is looked through: its addressable children appear in its place |

**Limits, documented:** child order is description order, which for every
built-in collection is display order (a lane grid's is reading order), and
for an external container whatever order it describes; an item keeps the
addressing marking its element had, `Each` replacing only its key (D:352).

### 2.3 `EachItem`: what `Each::new`'s `build` returns

1. `build` returns any `R: EachItem`. `EachItem` is implemented for every
   `C: Component + 'static` and for `Element`, and for nothing else.
2. **A component** becomes `Element::new(item_key, component)`, exactly what
   `component.key(..)` produced before `Each` replaced the key, so nothing
   mounted, laid out or painted changes.
3. **An `Element`** has its key replaced and keeps everything else, as today
   (D:350-353). It stays for items of different component types, which one
   closure can return only as `Element`, and for an item that needs an
   element builder (`with_grow`, `with_label`, `structural`, `slot`,
   `part`). Such an element still needs a key to exist; any key does, and
   the rustdoc says it is replaced. This is the residual of the TODO entry,
   stated rather than hidden.
4. **Sealed, and the conversion uncallable.** The conversion is a method
   of a supertrait in a private module, `Sealed::into_item(self, key:
   String, token: Token) -> Element`, where `Token` is a struct in that
   private module with a private field and a `pub(crate)` constructor (the
   sealed-method pattern). A supertrait's method is callable through the
   public trait's bound, so the private module alone would not suffice: a
   generic `fn f<R: EachItem>(r: R) -> Element { r.into_item(key) }`
   compiles when the method takes only the key (Codex's review of revision
   1). With the token, outside the crate:
   - the trait cannot be implemented (`Sealed` cannot be named);
   - the method cannot be called with a value: `Token` cannot be named
     (E0603), constructed (private field and constructor, E0624) or
     obtained: no public function, method or constant returns one, and it
     implements no trait that supplies a value (`Default`, E0277; `Clone`
     or `Copy`, so even a token in hand could not be duplicated, E0277). It
     derives `Debug`, which the denied `missing_debug_implementations` lint
     requires of every `pub` type, also an unreachable one, and which only
     formats a token. A call with fewer arguments is E0061;
   - the only call that compiles passes a diverging expression, such as
     `todo!()`, which panics before the method runs (rustc warns that the
     call is unreachable), so no conversion ever executes; unsafe code that
     conjures the zero-sized token is outside the contract, as for any
     private invariant.

   A callable conversion would be a second way to key a component beside
   `IntoElement::key` (AGENTS.md, "Component API conventions"), and every
   type an item can be already implements the trait: a custom component
   through its `Component` implementation (decision [P5-D2](#decisions)).
   Checked on the stand-in crate (rule 5), built with the workspace's
   denied lints (`missing_docs`, `unsafe_code`,
   `missing_debug_implementations`, `-D warnings`, and through
   `clippy-driver` the denied Clippy lints of `Cargo.toml:50-`): the crate
   builds clean only with `Debug` derived on `Token`, and the cases above
   give exactly these results.
5. **Coherence.** `impl<C: Component + 'static> EachItem for C` and `impl
   EachItem for Element` do not overlap: both traits and `Element` are local
   to the runtime, `Element` does not implement `Component`, and downstream
   crates cannot implement `Component` for `Element` (orphan rule) or
   `EachItem` for anything (sealed). Checked with rustc 1.99.1, edition
   2024, on a stand-in crate in the session scratchpad: both impls compile,
   a closure returning a component or an `Element` compiles, an external
   `impl EachItem for Mine` fails with rustc's sealed-trait note.
6. **The message for anything else** comes from
   `#[diagnostic::on_unimplemented]` on `EachItem` and
   `#[diagnostic::do_not_recommend]` on the blanket implementations, as
   `AppError`'s conversion does (`F/app.rs:196-199`): without the latter,
   rustc's help names `Component` and lists component types rather than the
   two accepted kinds (same stand-in check). Wording:
   - message ``"`{Self}` cannot be an `Each` item"``;
   - label ``"an `Each` item is a component or an `Element`"``;
   - note ``"return the component itself, such as `Label::new(text)`, or an `Element` for items of different component types"``.
7. **Inference.** The closure's return type was fixed to `Element`; it is now
   inferred. A body whose type came only from that expectation, such as
   `|_, item| make(item).into()`, no longer compiles (E0283). No caller in the
   repository is of that form (section 6); the CHANGELOG states it with its
   replacement.

## 3. Public API

In `pert_ui_runtime::testing`, re-exported by `pert_ui::testing`:

```rust
impl<'a> Driver<'a> {
    /// The components one address segment below `parent`, in child order,
    /// each with the segment `Runtime::find` resolves it by. [2.1 rules 1-5
    /// in prose: the equivalence with `find`, the restricted relation to
    /// `Selector::children` and where they differ (a part, a shared path),
    /// relative to `parent`, hidden and detached included, unmounted rows
    /// excluded with the controller route, duplicates both returned; the
    /// table of 2.2 and its limits; that `Runtime::children` stays the
    /// structure walk for hosts and custom components; # Errors; an example:
    /// an `Each` of bare `Label`s reordered by its binding lists its rows in
    /// the new order, and `rows[0].id` is `driver.find(&list, &["ship"])?`.]
    pub fn address_children(&self, parent: &WidgetId) -> Result<Vec<AddressChild>, TestError>;
}

/// One component below a parent at one address segment, as
/// `Driver::address_children` lists it. [That `segment` is what
/// `Runtime::find` takes, which for a slot or part is its name and not
/// `Runtime::key`; that only the driver constructs it.]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AddressChild {
    /// The address segment this component introduces below the parent: an
    /// element's key, a slot's or a part's name.
    pub segment: String,
    /// The component.
    pub id: WidgetId,
}
```

At the crate roots of both packages (the facade's `pub use
pert_ui_runtime::*`, F:28-29), defined in D beside `Each`, its only user:

```rust
/// What an `Each`'s `build` returns for one item: any component, which
/// `Each` describes under the item's key, or an `Element`, whose own key
/// `Each` replaces. [2.3 rules 2-4; that code never names or implements
/// it or calls its conversion; `compile_fail` examples of `impl EachItem
/// for MyType {}`, of a `build` returning a `String`, and of a generic
/// `fn f<R: EachItem>(r: R) -> Element` calling `r.into_item(..)`.]
#[diagnostic::on_unimplemented(/* 2.3 rule 6 */)]
pub trait EachItem: sealed::Sealed {}
#[diagnostic::do_not_recommend]
impl<C: Component + 'static> EachItem for C {}
impl EachItem for Element {}
// mod sealed {                                  // private module
//     #[derive(Debug)]                          // required by the denied lint
//     pub struct Token(());                     // private field
//     impl Token { pub(crate) fn new() -> Self { Self(()) } }
//     pub trait Sealed { fn into_item(self, key: String, token: Token) -> Element; }
// }
// with the same two impls (the blanket one also `#[diagnostic::do_not_recommend]`):
// the conversion `Each` calls, `build(cx, value).into_item(key(value), Token::new())`.
// No public function returns a `Token`, and it implements no `Default`,
// `Clone` or `Copy`: nothing outside the crate supplies one.

impl<T: Clone + 'static> Each<T> {
    /// [D:338-343, with: `build` returns the item's content, any component
    /// or an `Element` (`EachItem`).]
    pub fn new<R: EachItem>(
        items: impl Into<ReadSignal<Vec<T>>>,
        key: impl Fn(&T) -> String + 'static,
        build: impl Fn(&DescribeContext<'_>, &T) -> R + 'static,
    ) -> Self;
}
```

- **Names.** `address_children` keeps the plan's name: it pairs with
  `Runtime::children` and says what differs. `AddressChild` pairs with it.
  `EachItem` says what the bound is for; `IntoItem` would read like
  `IntoIterator<Item = ..>` and the `*Items` source traits.
- **`AddressChild`, not `WidgetId`** (decision [P5-D1](#decisions)): the
  segment is what a test asserts and passes to `find`, and for a slot or
  part `Runtime::key` returns something else (section 1), the confusion of
  keys and addresses R2-6 is about. Public fields with `#[non_exhaustive]`,
  as `FlushReport` and P4's `QuietReport`.
- **`Each<T>` keeps its one type parameter:** `R` is erased into the
  `BuildItem<T>` closure (D:256), so `Each<T>` annotations do not change.
- **No `#[must_use]`** on the method: like `Driver::find` and
  `Runtime::children`, it only reads.
- **Anti-regrowth (AGENTS.md).** `address_children` retires, as taught ways,
  the walk through the `items` container and the bounds-sorted descendant
  walk; `Selector::children` stays for one-query semantics, `children` for
  structure. A component returned from `build` retires the placeholder
  `.key(..)`; `Element` stays where one closure needs it (2.3 rule 3).

## 4. Documentation

Text describes P5's own commit.

- **`Each`** (D:258-328): `build` gives each item's content, a component or
  an `Element` (`EachItem`); the first example returns `Label::new(..)` with
  no key; a short example of items of two component types returns
  `Element`s and keeps today's assertion that the element's own key does not
  resolve (D:295-296); the `within` example drops `.key("tag")`; a sentence
  points to `Driver::address_children` for the items in order in a test.
  `Each::new`'s rustdoc (D:338-343) likewise.
- **`Runtime::children`** (TR:137-141) states that it lists structural
  wrappers, such as `Each`'s default container and a `Table`'s body, which
  addresses skip, and that a virtualized collection lists its mounted rows
  in index order, offscreen retained ones included; for the components one
  address segment below, `Driver::address_children`. **`Runtime::key`**
  (TR:127-128) states that every component has a key, a structural wrapper's
  is no address segment and a slot's or part's segment is its name.
- **`VirtualList`** (VL:400-410): `Runtime::children` lists the mounted rows
  in index order, pinned ones included. **`Table`** (`R/table.rs`, the type's
  page): `Driver::address_children(&table)` lists the mounted rows.
- **`Selector::Children` and `Selector::children`**
  (`R/inspection_service.rs:65-67`, `:100-103`): results are in incarnation
  order, as every query result, and match by absolute path, so a part's
  descendants are not children of the part's path and two components with
  one path pool their children; `Driver::address_children` lists one
  parent's in child order, relative to it as `find` resolves.
- **The testing chapter's "List rows"** (`G/testing.md:203-261`) is
  rewritten, each case a doctest with a bare component as `build`'s result:
  an `Each` read in order with `address_children`, and `Selector::children`
  for the semantics in one query, said to be in incarnation order; an `Each`
  with `within` through its container; a `VirtualList` whose offscreen row is
  revealed with `ListController::reveal`, settled and then listed; a `Table`
  whose rows are listed. A closing sentence: `Runtime::children` stays the
  structure walk for hosts and custom components. The chapter's introduction
  (as P2 and P4 leave it) adds that the driver lists rows in order.
- **The prelude pages** (`R/lib.rs:375-376`, `F/lib.rs:99-100`): the
  `DescribeContext` bullet becomes "The `DescribeContext` an `Each`,
  `Dynamic` or `Show` closure receives, and the `EachItem` an `Each`'s
  `build` returns, are inferred; code that names either imports it from the
  crate root", and the "where the other names are" row naming
  `DescribeContext` (`R/lib.rs:400`, `F/lib.rs:126`, as P2 leaves it) adds
  `EachItem`. This is L2's conversions exception (section 5).
- **`testing` module pages** (T:1-30, F's `testing` page): the driver's
  description adds that it lists a component's address children in order;
  `AddressChild` is listed with the driver's results, in the shape P2 leaves
  them.
- **The layout chapter** (`G/layout.md:405`) drops `.key("task")`.
- **docs/contracts.md** (`:250-252`): "`Each` keys each item's element"
  becomes "`Each` keys each item's content, a component or an element, with
  the key it gives". **docs/manual.md** (`:177`): `|_, item| element`
  becomes `|_, item| component`.
- No change to the READMEs: neither shows `Each` (`git grep`).

## 5. Module membership and dependencies

- **`AddressChild`** is defined in `pert_ui_runtime::testing` beside
  `Driver`, and joins the facade's `#[doc(inline)]` re-export of `Driver`
  and `TestError` (F:275-276) in every configuration. It is not at either
  root, as `Driver` is not.
- **`EachItem`** is at both roots only. `Each` is a prelude member, so L2's
  closure (`layering-design-L2.md:122-134`) applies to its constructor; the
  bound falls under its conversions exception (`:145-147`): an application
  writes components and elements, never the trait, and calls no method of
  it. No prelude name changes; section 7's prelude-only test is L2's fixture
  for the changed constructor.
- **Dependency on P2 (revision 3, approved at 07eb0a4).** P2's "Later
  parts" (`round-2-design-P2.md:752-759`) expected P5's `address_children`
  to add no type beyond `WidgetId` and `String`; this design adds
  `AddressChild` (P5-D1), and P2's rule covers it:
  - **Assertion 1** of P2's listing-driven check (`round-2-design-P2.md`,
    section 4, `:317-327`): `testing`'s exports are seeds (P2 2.2), so
    `AddressChild` is a seed and `Driver::address_children`'s return type;
    its fields are `String` and `WidgetId`, a prelude member, so nothing
    further is reached and no exclusion is added.
  - **Assertion 3:** the facade's `testing` holds every runtime `testing`
    name, which the facade re-export above satisfies; omitting it fails the
    check.
  - **Assertions 2 and 4** are untouched: no exclusion and no `Runtime`
    seed changes.
  - **Parsing:** `EachItem` is reached from no seed, but every listing line
    is parsed, and an unknown form fails the check (P2 section 4,
    "Parsing"). Its lines are a trait declaration with a supertrait bound
    and `impl … for` trait blocks, both forms P2 parses; `Sealed` and `Token`
    are unreachable and not listed. P5 runs the check on the regenerated
    listings and records the result in its Progress entry.
  - P2's `every_name` and `every_glob_combines` lists
    (`crates/pert-ui/tests/foundation/modules.rs`, as P2 leaves them) gain
    `AddressChild` in `pert_ui::testing`, and the root lists gain
    `EachItem`. P2 renames the facade's `svg` module to `svg_images`
    (P2-D1), which P5 does not touch.
- **Dependency on P4 (approved, revision 3):** none in substance. P5's
  tests join the driver's test binary P4 also extends
  (`crates/pert-ui-runtime/tests/testing.rs`), and P5's chapter and module
  page edits apply to the text P4 leaves.
- **Later parts.** P8's table section in the testing chapter reads rows with
  `address_children` (plan constraint "P5 before P8"); P5 documents rows
  only, not a table cell's address, which is P8's (R2-22).

## 6. Migration manifest

`Each::new` accepts every `build` that compiled before (it returned an
`Element`), so no consumer has to change for the build to pass; the dead
keys are removed so that the repository teaches the new form.

| Consumer | P5 |
|---|---|
| `Each`'s rustdoc (D:290, :310) and `Each::new` | bare components; the replaced-key assertion moves to the mixed-types `Element` example (section 4) |
| Guide: `G/layout.md:405`, `G/testing.md:221`, `:248` (whole "List rows" rewritten) | bare components |
| Gallery: `apps/gallery/src/pages/chips.rs:69-71` | `Label::new(format!("• {tag}"))`, no `.key("label")`; its `within` container stays |
| Facade tests that return one component: `tests/foundation/described.rs:252-254`, `:278-280`, `:440-442`; `tests/foundation/capture.rs:672-674`; `tests/foundation/failure_sites.rs:203`; `tests/foundation/standard_traits.rs:379`; `tests/collections/content_search.rs:1062`; `tests/foundation/each_addresses.rs`'s `label` and `scrolling` helpers (`:24-36`, returning `Label` and `Column`) | dead key removed |
| Facade tests whose subject is the `Element` path: `tests/foundation/described.rs:221` (`.key("ignored")`), the structural items of `tests/foundation/each_addresses.rs:320`, `:331`, `:382`, `:392`, `tests/surfaces/workspace.rs:1451`'s `scrolling_item` (its key is the item's) | unchanged: they test or need an `Element` (2.3 rule 3) |
| `plans/usability/task4-app/src/lib.rs:104-118` | the `Row` returned without `.key("row")`; `Cargo.toml:8` requires `0.3.80`; fixture check |
| docs/contracts.md, docs/manual.md | section 4 |
| Runtime: `Runtime::children`, `key`, `VirtualList`, `Table`, `Selector` rustdoc; prelude and `testing` pages | section 4 |
| `crates/pert-ui-runtime/tests/testing.rs`; `crates/pert-ui/tests/foundation/` | gain section 7's tests |
| Diagnostics fixtures (`tests/diagnostics/ui/`) | gain a case for 2.3 rule 6; the existing snapshots are rerun, since a new blanket impl of a sealed trait over `Component` could change a listed candidate |
| Packaged examples and their headless sessions, the native checks, the READMEs | unchanged: none calls `Each::new` (`git grep`), so none changes |
| Frozen: the submissions and probes under `plans/usability/runs/` and U, such as U `r2-4-codex/work/bookmarks/tests/ui.rs:76-88` and U `r2-4-claude/work/bookmarks/tests/behaviors.rs:101-134` | not migrated |

## 7. Tests

The driver's tests are in `crates/pert-ui-runtime/tests/testing.rs`, the
driver's test binary (the plan's "facade area binary" rule is for facade
items); `EachItem`'s are in the facade's `foundation` binary
(`tests/foundation/each_addresses.rs`), where `Each`'s addressing tests are,
written with `use pert_ui::prelude::*` alone. Every window has a viewport.

- **New-behavior tests** each name the assertion that detects a wrong
  implementation and a compiling mutation that must make it fail; the
  Progress entry records each mutation's result. A test that fails only
  because a new item does not compile is not evidence.
- **Contract and compatibility tests** check rules P5 keeps or states.

A helper, `agrees_with_find(runtime, parent, &rows)`, asserts 2.1 rule 2's
definition for every result below: each row's `find(parent, &[segment])` is
`Ok(Some(id))`, or `Err(AmbiguousKey)` when another row has the segment. A
second helper, `agrees_with_selector`, asserts the restricted relation only
where its condition holds: tests 1, 3 and 5, whose parents are `Own`-marked
with a unique path, compare the ids as a set with those
`Selector::children` of the parent's path matches. Tests 11 and 12 show
the two cases outside it.

### New-behavior tests

1. **An `Each` in list order after a reorder:** rows are `Row`s of keyed
   `Label`s (`name`, `url`), items `[write, ship]`, then set to
   `[ship, write]` and settled. Named assertion: the segments are
   `["ship", "write"]`, the ids are `find`'s. Mutations: return
   `Runtime::children(parent)` (one child, `items`); visit an `Own`
   element's descendants too (the labels appear); pop the stack without
   reversing (`["write", "ship"]`); sort by `inspection_id` (incarnation
   order, `["write", "ship"]`).
2. **`within`:** an `Each` with `within(|tags| Wrap::new(tags).key("tags"))`.
   Named assertion: `address_children(&each)` is one child with segment
   `tags`, and `address_children` of it lists the rows. Mutation: descend
   below a returned `Own` element (the rows appear at the first level).
3. **A reordered `VirtualList`:** a source of `[a, b, c]` republished as
   `[c, a, b]`. Named assertion: segments `["c", "a", "b"]`. Mutation: sort by
   `inspection_id`.
4. **A scrolled `VirtualList`:** 100 `Button` rows of 20 px in a 100 px
   viewport, `overscan(1)`, row 0 focused and row 60 `keep_mounted`, then
   scrolled to row 40 with `ListController::reveal`. Named assertion: the
   result is row 0, the rows whose bounds meet the list's with one overscan
   line on each side, and row 60, in index order, and has no row 80; after `controller.reveal("row-80")` and a
   settle, row 80 is listed. Mutation: keep only rows whose bounds meet the
   list's (rows 0 and 60 and the overscan disappear).
5. **A `Table`:** three rows. Named assertion: the segments are the row keys
   in source order, and neither `body` nor the reporter's key appears.
   Mutation: return `Runtime::children(parent)`.
6. **A structural parent:** the `Each`'s default container, found by
   `find_descendant(&each, "items")`. Named assertion: the result equals
   `address_children(&each)`. Mutation: compute the result as
   `Selector::children` of the parent's own path (a structural parent has
   none: empty).
7. **A part and its descendant:** `root → Element::new("frame", Column[
   Label.key("row") ]).part("panel")`. Named assertions:
   `address_children(&root)` is `[panel, row]` in that order, with
   `find(&root, &["panel"])` and `find(&root, &["row"])` resolving them;
   `address_children(&panel)` is `[row]`. Mutations: do not descend a part
   (`row` missing); descend a part without returning it (`panel` missing).
8. **Two children with one segment through structural levels:** two
   structural wrappers, each holding a child keyed `a`. Named assertion:
   both are returned, in order, and `find(&root, &["a"])` is
   `Err(AmbiguousKey)`. Mutation: keep the first of each segment.
9. **A slot:** an element keyed `internal` and marked `.slot("left")`, with a
   keyed child. Named assertion: one child, segment `left` (not `internal`),
   which `find(&root, &["left"])` resolves; the slot's child is not listed.
   Mutations: use `Runtime::key` as the segment; treat a slot as structural.
10. **A bare component as an `Each` item** (facade, prelude only):
    `Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str()))`
    with exactly one item, `"ship"`, so that a constant key mounts. Named
    assertion: after the first flush, `find(&each, &["ship"])` resolves and
    its key is `ship`. Mutation: the component impl keys with a fixed string
    instead of the item's key (the flush succeeds with one item, and `find`
    returns `None`).
11. **Below a part, the selector differs:** test 7's tree mounted as a root
    keyed `root`. Named assertion: `address_children(&panel)` is `[row]`
    and `find(&panel, &["row"])` resolves it, while
    `Selector::children(["root", "panel"])` matches nothing (`row`'s path is
    `["root", "row"]`, `R/runtime/workspace.rs:271-274`). Mutation: compute
    the result as `Selector::children` of the parent's path (empty).
12. **A shared path, the selector differs:** `root → [Structural s1 →
    Own("list") → [a], Structural s2 → Own("list") → [b]]`, the two `list`s
    sharing the path `["root", "list"]`. Named assertion:
    `address_children(&list1)` is `[a]` and `address_children(&list2)` is
    `[b]`, while `Selector::children(["root", "list"])` matches both `a` and
    `b`. Mutation: the same selector-based computation (`list1` also gets
    `b`).

### Contract and compatibility tests

- **A leaf** returns an empty list; **an unmounted parent** returns
  `TestError::Runtime(RuntimeError::Unmounted)`, one of another runtime
  `ForeignRuntime`.
- **Hidden and detached are included, as in `find`:** a child hidden with
  `View::visible_signal(false)` and a panel presented in a detached window
  (the setup of `click_refuses_another_window_and_a_refused_reveal`,
  `tests/testing.rs:334-384`) are listed.
- **A row marked `Element::structural`** is looked through: its keyed
  children are listed in its place, and `find` of their shared key across
  two rows is `AmbiguousKey` (the limit of 2.2).
- **The drag preview:** while a `VirtualList` row is dragged with a preview,
  `address_children` of the list and of its parent are what they were before
  the drag (the preview is a root of its own, rule 6).
- **The `Element` path is unchanged:** `described.rs:214-245`'s
  `.key("ignored")` test, the structural-item tests in `each_addresses.rs`
  and every other existing `Each` test pass unchanged; a closure returning
  `Element`s of two component types compiles and addresses by item key.
- **`Runtime::children` is unchanged:** the existing tests and the `Table`
  example that reaches the body as `children(&table)[0]`
  (`R/table.rs:1386`) pass.
- **A duplicate item key with bare components** keeps the previous items:
  an `Each` of bare `Label`s whose list becomes `["a", "a"]` fails the
  flush, `find(&each, &["a"])` still resolves the earlier component, and a
  valid list recovers, as `tests/foundation/described.rs:248-270` checks
  for the `Element` path.
- **`EachItem` is sealed and its conversion uncallable:** `compile_fail`
  doctests on its page: `impl EachItem for MyType {}`; a `build` returning
  `String`; a generic `fn convert<R: EachItem>(value: R) -> Element {
  value.into_item("chosen".to_owned()) }` (E0061 with the token); and the
  same with `Default::default()` as the token (E0277). `compile_fail`
  proves the restriction, not the message; the stand-in check of 2.3 rule
  4 recorded which error each case gives.
- **The message:** a diagnostics case `tests/diagnostics/ui/e01_each_item_not_component.rs`
  (`build` returning a `String`) with its `.stderr` snapshot, generated with
  `TRYBUILD=overwrite` and reviewed like the `api/` diff; the existing
  snapshots unchanged.
- **Module membership** (P2's tests, section 5): `every_name` imports
  `AddressChild` from `pert_ui::testing` and `EachItem` from the root;
  `every_glob_combines` resolves `AddressChild`; P2's listing-driven check
  passes on the regenerated listings.
- **Doctests:** `Each`, `Each::new`, `EachItem`, `Driver::address_children`,
  `Runtime::children`, the testing chapter's "List rows", the layout
  chapter's example.

## 8. Scope, version and evidence

- **`api/` diff:** added `Driver::address_children`, `AddressChild` with its
  fields and derived impls, `EachItem` with its two implementations;
  changed `Each::new`'s signature (`build` returns `R: EachItem`). In
  `api/pert-ui-runtime/{default,serde}.txt` and every `api/pert-ui/*.txt`,
  since the facade's root and `testing` carry them in every configuration.
- **Collision review:** `AddressChild` and `EachItem` occur nowhere in the
  repository (`git grep` at 5942c54). `AddressChild` joins only the
  `testing` globs, where no maintained file defines it; `EachItem` joins no
  glob except a crate-root glob, as in the gallery's `use pert_ui::*`
  (`apps/gallery/src/gallery.rs:22` and others), whose files define no item
  of that name.
- **CHANGELOG** for 0.3.80, citing this checkpoint:
  - `Added (F9):` `Driver::address_children` and `AddressChild` (`segment`,
    `id`): the components one address segment below a parent, in child
    order, each as `Runtime::find` resolves it; an `Each`'s rows without its
    container, a `VirtualList`'s and a `Table`'s mounted rows in index
    order.
  - `Breaking (TODO.md, Each::new's placeholder key):` `Each::new`'s `build`
    returns any `EachItem`: a component, which `Each` keys with the item's
    key, or an `Element`, whose key it replaces as before. Every closure that
    returned an `Element` compiles unchanged; one whose return type was
    inferred only from the expected `Element`, such as `|_, item|
    make(item).into()`, needs its type named. Replacement usage: return the
    component, `|_, task| Label::new(task.as_str())`, without `.key(..)`.
  - `Added:` `EachItem`, sealed, implemented for every `Component` and for
    `Element`, at the crate roots.
  - `Documentation (R2-6):` `Runtime::children` and `Runtime::key` say what
    structural wrappers, slots, parts and virtualized collections list;
    `VirtualList`, `Table` and `Selector::children` say the order they give;
    the testing chapter's "List rows" reads an `Each`, an `Each` with
    `within`, a `VirtualList` and a `Table` in order; contracts.md and the
    manual describe `Each`'s items as components.
- **TODO.md:** the `Each::new` entry (TODO.md:136-145) is removed; the
  finding below is added.
- **Version:** 0.3.80 after P4's 0.3.79: the workspace version, the internal
  version requirements and `Cargo.lock`. If the order changes, the heading
  follows the actual version.
- **Gate:** the public-API row, as the plan states: the workspace checks
  (formatting, Clippy, tests including P2's listing-driven check and
  section 7, the packaged examples' headless sessions, doctests,
  warning-free rustdoc), the API listing gate with its diff reviewed against
  the CHANGELOG, the diagnostics snapshots (`cargo test --locked -p
  pert-ui-diagnostics -- --ignored`, the new case included), the fixture
  check (`plans/usability/run.sh prepare` from P5's commit, `cargo test` in
  a copy of the fixture with `run.sh exec`), `git diff --check`, the
  mutations of section 7 recorded in the Progress entry, and Codex's
  implementation review.
- **Native evidence:** not applicable. A bare component mounts the same
  element `.key(..)` produced (2.3 rule 2), and `address_children` only
  reads; no presentation or platform code changes.
- **Persistence, real time, backends:** none involved; workspace and
  inspection paths are unchanged (2.3 rule 2 keeps every address).

## Corrections to the plan

- **The drag-preview case cannot fail.** The plan's P5 tests list "a drag
  preview (never returned)" among the tests that fail when reverted. The
  preview is mounted as a root of its own (`R/runtime/drag.rs:348-350`), so
  no application component has it as a descendant and no mutation of the
  skip is observable; it is a contract test (section 7), and the skip is
  kept so that the walk and `find` stay one definition.
- **The result carries the segment.** D4 describes the result as the
  components `find` resolves; for a slot or a part the segment is not
  `Runtime::key` (section 1), and the plan's own part test speaks of
  results "under its name", so the result is `AddressChild` (P5-D1). P2's
  approved "Later parts" expected no new type; section 5 records the
  difference and how P2's rule covers it.
- **"`Element` still accepted for mixed rows"** also covers an item that
  needs an element builder, and in both cases the element keeps a key `Each`
  replaces (2.3 rule 3): the placeholder is gone for single-component items,
  not for every item.
- **Breaking in inference only.** The plan lists `Each::new` as breaking;
  every existing caller compiles unchanged, and only a body typed by the
  expected `Element` breaks (2.3 rule 7). The CHANGELOG keeps a `Breaking:`
  entry with that statement.
- **Where `EachItem` lives.** D4 places the trait "beside `IntoElement`" in
  L; it is `Each`'s bound only, so it is defined in D beside `Each`. Its path
  at the crate roots is the same.

## Findings outside this part

- `R/virtual_list.rs` `ListItems::build` and `R/tree.rs` `TreeItems::build`
  — developer usability (the same kind as the `Each::new` TODO entry P5
  resolves): both return an `Element` whose key the collection replaces
  (VL:30-31, :58-59, :151-156; `R/tree.rs:44`, :131-132), so an
  implementation writes a placeholder key, as the diagram editor's rows do
  (`crates/pert-ui/examples/diagram_editor/app.rs:427`, `.key("row")`), or
  computes its key twice (`apps/gallery/src/pages/data.rs:25-27`). They are
  trait-object methods, so `EachItem`'s generic return does not apply; a fix
  needs an object-safe keyless row value, a breaking change to every
  implementation, outside D4's scope. P5's implementation records it in
  TODO.md (searching first) and does not fix it.

## Not in this part

- **A `Runtime` method for hosts and tooling.** Tooling selects by path
  with `Selector::children`, which coincides with `address_children` under
  2.1 rule 2's condition and differs below a part or for a shared path, and
  hosts walk structure with `children`; no run needed rows in order outside
  a test. A `Runtime` form of the walk would be a second way to the
  driver's result; a tool that needs the relative walk can be designed with
  its own evidence.
- **Rows that are not mounted:** a collection's controller mounts them; a
  `WidgetId`-based reveal cannot locate a row without a component (D4).
- **A table cell's address and header items:** P8.
- **`Tree`:** the rule applies to it as to any component; no run read a
  tree's rows, so neither the chapter nor section 7 names it.

## Decisions

- **P5-D1: what `address_children` returns.**
  - **Recommended: `Vec<AddressChild>`** with the segment and the id. A test
    asserts segments and passes them to `find`; for slots and parts the
    segment differs from `Runtime::key`, which a list of ids would leave a
    test to read, repeating R2-6's confusion. One small type in `testing`.
    Approved by the author as recommended.
  - **Alternative A: `Vec<WidgetId>`,** as `Runtime::children`. No new type,
    but the segment of a slot or part is then unreadable without an
    inspection query, and a test reads keys with a second call per row.
  - **Alternative B: `Vec<(String, WidgetId)>`.** No new type, but the
    signature does not say which string, and `rows[0].1` reads worse than
    `rows[0].id`.
- **P5-D2: `EachItem` sealed.**
  - **Recommended: sealed, with an uncallable conversion,** the conversion
    on a private supertrait that takes a crate-constructed token (2.3 rule
    4). Every possible item is already covered (a custom component through
    `Component`), a callable conversion would be a second way to key a
    component beside `IntoElement::key`, and the library keeps the freedom
    to change the conversion. Approved by the author as recommended.
  - **Alternative: an open trait** with a public `into_item(key)`, so an
    application type that is not a component could be an item. No such type
    is needed (it can implement `Component`), and the method duplicates
    `.key(..)`.

## Review record

- **Revision 1 (2026-10-28):** written from the code at 5942c54 (library
  code as at 26e6b5a); the coherence and the diagnostic were checked on a
  stand-in crate with rustc 1.99.1.
- **Revision 1, Codex review (2026-10-28):** needs revision, two blocking
  points, one should-fix and a nit; record
  `~/pert-ui-usability/codex-prompts/codex-design-P5.out` (read against
  07eb0a4). Each point was checked against the code before revision 2
  applied it; none was rejected.
  - **Blocking, the `Selector::children` equivalence:** revision 1 claimed
    set equality for every addressable parent. False below a part (a part's
    descendants omit its name, `R/runtime/workspace.rs:257-274`) and for a
    shared path (the selector matches absolute paths runtime-wide,
    `R/runtime/inspect.rs:390-392`). Revision 2 keeps the relative `find`
    contract as the definition, states the relation to the selector only
    for an `Own`- or `Slot`-marked parent whose path no other `Own`- or
    `Slot`-marked component has, with the reason it holds exactly then
    (2.1 rule 2), restricts the selector helper to tests 1, 3 and 5, adds
    tests 11 and 12 for the two differences, and corrects the rustdoc,
    `Selector`'s documentation and "Not in this part".
  - **Blocking, the conversion was callable:** a supertrait's method is
    callable through `R: EachItem` without naming the private trait.
    Revision 2 gives it a token parameter of an unnameable, unconstructible
    type (2.3 rule 4), checked on the stand-in crate: the generic call
    without a token is E0061, with `Default::default()` E0277, naming the
    token E0603 and E0624, and a diverging `todo!()` compiles but never
    reaches the method. The `compile_fail` doctests cover the generic call.
  - **Should-fix, test 10's mutation:** with several items a constant key
    fails the first flush before the named assertion runs. Test 10 starts
    with one item, so the mutation reaches `find`; duplicate rejection with
    bare components is a separate contract test.
  - **Nit, P2's status:** P2 is approved at 07eb0a4 (revision 3);
    section 5 cites its settled assertions and its "Later parts"
    expectation directly.
- **Revision 2, Codex recheck (2026-10-28):** one should-fix point; record
  `~/pert-ui-usability/codex-prompts/codex-design-P5-r2.out`. `Token` had
  no `Debug`, which the denied `missing_debug_implementations` lint
  (`Cargo.toml:44`) rejects: the stand-in crate with the lint denied
  confirmed it fires for the `pub` struct in the private module. Revision 3
  derives `Debug` on `Token`, words the restriction as what it is (no
  public operation supplies a token; no `Default`, `Clone` or `Copy`)
  instead of "implements no trait", and repeats the stand-in checks with
  the workspace's denied rustc and Clippy lints: the crate builds clean,
  and every case of 2.3 rule 4 gives the error recorded there.
- **Revision 3, Codex second recheck (2026-10-28):** no new findings; the
  remaining point is resolved and earlier corrections hold. Ready for
  approval; approved by the author under the maintainer's delegation, with
  P5-D1 and P5-D2 as recommended.
