# Round 2 fix plan, part P2 design checkpoint: glob membership

Part P2 of [the round 2 fix plan](round-2-fix-plan.md#p2-glob-membership-0377)
closes F3 by decision D3 and the TODO.md entry on the facade's `pub mod svg`.
It changes what `pert_ui::testing`, `pert_ui_runtime::testing` and both
preludes export, and L2's guarantee with them, so its rule, seeds, exclusion
list, added names and collision review are settled here before any code.

Status: **revision 3, approved 2026-10-28** by the author together with
Codex's review (second recheck: ready for approval). Revision 1 was
reviewed by Codex (needs revision, two should-fix points and a nit), and
revision 2 answered each; Codex's recheck of revision 2 left one should-fix
point and a nit, which revision 3 answers ([review record](#review-record)). The author approves
P2-D1 (`svg` renamed `svg_images`) and P2-D2 (the validation family in
`testing`) as recommended, under the maintainer's delegation of 2026-10-28
(plan, status line); the checkpoint was approved once Codex found it ready. Every reference is
to the committed code at ee55226, whose library code is unchanged since
26e6b5a (0.3.75); another agent is implementing P1 (documentation only) in
the working tree, so line numbers in the guide may move by the time P2
starts and are re-located by content. Path prefixes are the plan's: `R/` =
`crates/pert-ui-runtime/src/`, `F/` = `crates/pert-ui/src/`, `G/` =
`crates/pert-ui/src/guide/`; `RL` = `api/pert-ui-runtime/default.txt`.

The name lists in sections 2 and 3 were produced by a throwaway prototype of
section 4's check over the committed listings at ee55226 (not part of the
repository); the implementation's test is the authority and its output at
the pre-change listings is recorded in the Progress entry.

## What testers wrote by hand, and what the code says

- **Round 2 reopened F3** (`plans/archive/developer-usability.md:871-888`):
  the testing chapter says one `use` block covers a test (`G/testing.md:3-9`),
  but `testing` (`F/lib.rs:266-283`, `R/testing.rs:78-86`) lacks
  - `Semantics`, which `Driver::semantics` returns (`R/testing.rs:377`;
    1-claude, 5-head-claude-2; rustc suggested a type parameter);
  - `InspectValue`, which `ComponentDetails::properties` holds
    (`R/diagnostics.rs:756`; 1-claude, 3-claude, 3-codex);
  - `SemanticRole`, with rustc suggesting `SemanticValue::Slider` because
    `testing` re-exports `SemanticValue` (3-claude, 4-codex);
  - and `ThemeMode` left the prelude in L2's D2
    (`plans/usability/layering-design-L2.md:952-962`; 3-claude, 3-codex).
- **What the runs wrote**, counted over the round 2 submissions under
  `~/pert-ui-usability/r2-*/work` (directories, not occurrences): `Semantics`
  6, `InspectValue` 5, `SemanticRole` 4, `ThemeMode` 2, `ComponentDetails` 2
  (both in a custom component's `inspect`), `SemanticAction` 1. Every capture
  in the sixteen runs is `pending.wait()?.value?` or `try_reply()?.value?`,
  naming no capture type; two task runs read development validation in a test
  by field only (`r2-2-claude/work/tests/headless.rs:76-78`,
  `r2-3-claude/work/tests/headless.rs:178-179`).
- **Why a hand list is not enough:** 6f1 and L2 each listed `testing`'s names
  by hand from the programs they read, and `Semantics`, the return type of the
  driver's own method, was missed. A rule that follows signatures notices a new
  method's return type; section 4 makes the listings check it.

## 1. Today's membership, from the code

- **`pert_ui_runtime::testing`** re-exports `HostIntegration` and 16 names
  (`R/testing.rs:80-86`) and defines `Driver` and `TestError`; its private
  imports include `FlushReport`, `RevealRefusal` and `Semantics`
  (`R/testing.rs:73-76`), which its public signatures return or carry:
  `Driver::settle` and `advance` return `FlushReport` (`:266`, `:294`),
  `Driver::semantics` returns `Semantics` (`:377`), and
  `TestError::NotRevealed` carries a `RevealRefusal` (`:729-735`).
- **`pert_ui::testing`** re-exports the same 16 names and `HostIntegration`
  by an explicit list, `Driver` and `TestError` inline, `HeadlessApp` with
  `native` and `CpuRenderer` with `cpu` (`F/lib.rs:266-283`).
  `CpuRenderer::render` returns `pert_ui_cpu::CpuRenderError`
  (`crates/pert-ui-cpu/src/lib.rs:154`), which the facade exports only from
  `cpu` (`F/cpu.rs:39-40`).
- **What the seeds hand on** (signatures from `RL`): `Runtime::query`
  returns `QueryResult` (`R/runtime/inspect.rs:225`), whose `components`
  are `ComponentInspection`s with `ComponentDetails`
  (`R/inspection_service.rs:366-386`, `R/diagnostics.rs:745-808`);
  `Runtime::request_capture` returns
  `Pending<Result<Capture, CaptureError>>` (`RL:10394`), whose `try_reply`
  and `wait` return `InspectionReply<T>` or `WaitError`
  (`R/inspection_service.rs:590`, `:642`); `CaptureTarget::window` and
  `region` take a `NativeWindowId` and a `Rect` (`R/capture.rs:91`, `:97`);
  `InspectionQuery::depth` takes a `Depth` (`RL:11722`); `PointerKind::Pinch`
  carries a `Magnification` (`R/pointer.rs:92`); `Selector::semantic` takes a
  `SemanticRole` (`R/inspection_service.rs:118`).
- **The preludes** hold no theme name; the "where the other names are"
  tables name `ThemeMode` as the example of a root import (`F/lib.rs:126`,
  `R/lib.rs:400`). `ThemeMode` is a two-variant enum with no payload, no
  constructor and no builder (`RL`, its block), set through
  `Runtime::theme`'s `ThemeManager` (`RL:10485`, `ThemeManager::set_mode`).
- **The facade's `svg` module** (`F/lib.rs:38-39`) holds `SvgError`,
  `MAX_NESTING`, `intrinsic_size`, `rasterize` and `rasterize_mask`
  (`api/pert-ui/svg.txt:198-230`); the `Svg` widget is at the root
  (`F/lib.rs:44-45`). L2 recorded its collision hazard and left it to TODO.md
  (`plans/usability/layering-design-L2.md:458-460`, TODO.md 2026-10-27
  entry).
- **The membership tests:** `crates/pert-ui/tests/foundation/modules.rs`'s
  `every_name::testing_names` (`:78-88`) omits `HostIntegration`, and the file
  has no list for the `host` module and does not glob `host::*` in
  `every_glob_combines` (`:151-272`): L3 added `pert_ui::host` and the
  trait's re-export from `testing` without extending them. This is a defect of
  the same kind in the file P2 extends and is fixed here
  ([corrections](#corrections-to-the-plan)).

## 2. The `testing` rule

### 2.1 Wording

The module pages and the testing chapter state it in these words, adapted
to each page:

> `testing` holds the driver and its error and everything else a headless
> test writes besides the prelude: the viewport, input by hand, the query,
> validation and capture names, and every type those names hand a test. It is
> closed: each type that a member's constructors, methods and associated
> constants take or return, and each type a member's public fields and
> variants hold, is in `testing` or in the prelude, and the same holds for
> every name this adds. An error nested inside another error is reached
> through `?` and `Display` and stays where it is. The exceptions below are
> records a test reads by field or passes on, and each is named with its
> module.

### 2.2 Seeds

The check starts from:

1. **Every type `testing` exports in the configuration**, except
   `HostIntegration`. The trait is re-exported so that its methods resolve in
   a test that stands in for a host (L3,
   `plans/usability/layering-design-L3.md:326-337`); what its methods take
   and return (presentation requests, capture jobs, clipboard services) is the
   `host` module's vocabulary, and a test that writes those names globs
   `host`. Seeding it would make `testing` a copy of `host`.
2. **The `Runtime` methods `testing`'s names exist for**, which its page
   names: `set_viewport`, `key_input`, `focus_next`, `pointer_input`,
   `command_key_input` (input by hand), `inspection_id`, `query` (reading
   what is shown), `validation_report` (development validation, P2-D2) and
   `request_capture` (captures). `Runtime` is a prelude member, so its other
   methods are not seeds: they belong to their families (L2 1.2, "Runtime and
   EventContext are hubs"). The check fails if one of the nine is missing from
   the listing as an inherent `Runtime` method, so a rename cannot drop a
   seed silently.

### 2.3 What is followed

From a seed and from every name the rule adds:

- its inherent methods and associated constants (and, for a trait, its own
  methods): every pert-ui-owned type named in the signature, through
  references, slices, tuples, `Option`, `Result`, `Vec`, maps and the
  argument and return types of `impl Fn…` parameters;
- its public fields, tuple-variant payloads and struct-variant fields.

Not followed:

- trait implementations (`Clone`, `From`, `Error::source` and the others):
  a conversion is used through `?` or `.into()`, not written;
- **nested errors:** a field or payload of an error type (one the listing
  shows implementing `core::error::Error`) whose own type is an error, as
  `CpuRenderError::Drawing(DrawError)` and `Limit(LimitError)`
  (`crates/pert-ui-cpu/src/lib.rs:17-45`) and `CaptureError::Drawing`,
  `Frame` and `Limit` (`RL:24282-24294`); this is L2's error-payload rule
  (`plans/usability/layering-design-L2.md:317-318`, `:341`);
- prelude members, excluded names and types outside the pert-ui packages:
  they are required to be nowhere and are not traversed further.

### 2.4 Exclusions

Each excluded name is reachable from a seed, is not written by a test, and
lives in a module the page names. An excluded name stops the traversal.

| Name | Reached by | Reason | Where it is |
|---|---|---|---|
| `WindowInspection`, `EventHistory`, `ParameterInspection`, `SourceInspection` | `QueryResult::windows`, `events`, `parameters`, `sources` (`R/inspection_service.rs:378-384`) | answers to the window, event, parameter and live-source queries, records for an application's own tooling with their own vocabulary (core parameter snapshots, delivery, freshness); no run named them | `inspection` |
| `BoxModelInspection`, `Invalidation`, `ValidationHints` | `ComponentDetails::box_model`, `validation` (`R/diagnostics.rs:751-753`), `ComponentInspection::dirty` (`:794`) | what a component reports about its geometry, invalidation and validation hints: a test reads them by field (`G/testing.md:52`), a component writes them in `inspect` | `component` and `inspection` |
| `PhysicalExtent` | `Viewport::from_physical`, `frame_extent` (`RL:4211`) | a host's surface size; a test builds its viewport with `Viewport::new` | `host` |
| `KeyDispatch`, `KeyEvent`, `KeyTransition` | `Runtime::command_key_input` (`RL:10439`), `key_input` (`RL:10371`), `CommandKeyInput::editing_event` (`RL:2677`) | key routing a host acts on and the record a component reads; a test's key input ignores them | `host`; `component` and `inspection`; `component` |
| `DisplayList` | `CpuRenderer::render` (`crates/pert-ui-cpu/src/lib.rs:154`), `cpu` configurations only | the scene `Runtime::paint` returns, passed on to the renderer | `component` and `host` |
| `GeometryError` | `Viewport::new` (`RL:4213`), `Rect::new` and the other value constructors | a constructor's validation error, propagated with `?`, as the prelude leaves it at the root (L2 1.2, "other errors") | crate root |

Errors a test **step** returns are not excluded: a test matches them to
assert why a step failed. That is why `TestError` is a member, why
`CpuRenderError` joins (plan D3; 5-r1-claude-1 rendered frames itself), and
why `CaptureError` (a capture refused while support is undeclared, the
`Unsupported` case of 5-r1-claude-2#7, `R/capture.rs:166`) and `WaitError`
(`R/inspection_service.rs:433`) join.

### 2.5 Configurations

The facade's `testing` differs with `native` (`HeadlessApp`) and `cpu`
(`CpuRenderer`, `CpuRenderError`), and its prelude with `native` and `svg`
(`F/lib.rs:134-146`). The check runs over:

| Configuration | Listings | `testing` | Prelude |
|---|---|---|---|
| runtime | `RL` | `pert_ui_runtime::testing` | `pert_ui_runtime::prelude` |
| default | `api/pert-ui/default.txt` with the runtime, core and CPU listings | facade, `native` | facade, `native` |
| cpu | `cpu.txt` | facade, `native` and `cpu` | facade, `native` |
| no-default | `no-default.txt` | facade, neither | runtime prelude |
| desktop-cpu | `no-default-desktop-cpu.txt` | facade, `native` and `cpu` | facade, `native` |

The `svg`, image and serde configurations change neither `testing` nor the
names its seeds reach and are not checked.

## 3. Membership after P2

### 3.1 `testing`

**Added to `pert_ui_runtime::testing` and `pert_ui::testing` (23):**

| Group | Names | Reached by |
|---|---|---|
| The driver's results | `FlushReport`, `RevealRefusal` | `Driver::settle`, `advance`; `TestError::NotRevealed` |
| Semantics | `Semantics`, `SemanticRole`, `SemanticAction`, `SemanticItem` | `Driver::semantics`; `Semantics::role`, `actions`, `items` (`R/diagnostics.rs:498-517`); `Selector::semantic` |
| Queries | `QueryResult`, `ComponentInspection`, `ComponentDetails`, `InspectValue`, `Depth` | `Runtime::query`; `QueryResult::components`; `ComponentInspection::details`; `ComponentDetails::properties` and `QueryResult::application`; `InspectionQuery::depth` |
| Validation (P2-D2) | `ValidationReport`, `ValidationIssue`, `ValidationCode`, `ValidationMarker` | `Runtime::validation_report` (`R/runtime.rs:4055`), `QueryResult::validation`; their fields (`R/validation.rs:228`) |
| Captures | `Capture`, `CaptureError`, `Pending`, `InspectionReply`, `WaitError`, `NativeWindowId`, `Rect` | `Runtime::request_capture`; `Pending::try_reply`, `wait`; `CaptureTarget::window`, `region`; `FrameImage::new`, `crop` (`RL:10057`) |
| Input by hand | `Magnification` | `PointerKind::Pinch` |

**Added to `pert_ui::testing` with `cpu`:** `CpuRenderError`, beside
`CpuRenderer`.

Every added name is a crate-root item today (`CpuRenderError` a
`pert_ui_cpu` item the facade's `cpu` module re-exports), so every existing
path keeps working; `NativeWindowId` is already in the facade prelude with
`native` and joins `testing` for the runtime and `no-default`
configurations, where nothing else holds it for a test.

Before the change the check reports exactly these names, each with the path
that reached it, in every configuration, `NativeWindowId` only in the runtime
and no-default ones and `CpuRenderError` only in cpu and desktop-cpu
(prototype run at ee55226). That includes the plan's four named cases:
`Semantics` from `Driver::semantics`, `FlushReport` from `Driver::settle`
and `advance`, `InspectValue` through `QueryResult`, and
`pert_ui_cpu::CpuRenderError` from `CpuRenderer::render`.

### 3.2 The preludes

`ThemeMode` joins `pert_ui_runtime::prelude` and so the facade's, the one
exception to L2's D2 (plan D3): a theme switch and light and dark captures
are first-program code (3-claude, 3-codex). L2's closure over it adds
nothing: it has no constructor, builder or payload. `ThemeManager`, `Palette`
and the other theme services stay at the root; `ui.theme()` returns the
manager without the program naming it. No other prelude name changes.

## 4. The listing-driven check

**Location:** a new test binary `tests/diagnostics/tests/testing_closure.rs`
in the unpublished `pert-ui-diagnostics` package, not ignored, so it runs in
`cargo test --locked --workspace` on every CI platform. It reads
`env!("CARGO_MANIFEST_DIR")/../../api/…` with `str::lines` (which also drops
a checkout's `\r`), needs no dependency, and is excluded from packaging with
the package (AGENTS.md packaging command). It does not run cargo-public-api:
the API listing gate keeps the committed listings current, so a change that
forgets to regenerate them is caught by CI's `api-listing` job, not by this
test.

**Parsing** of cargo-public-api 0.52.0's `--simplified` lines (the version
`api/generate.sh` pins). Each line is one of: `pub mod`; `pub use` of one
item or of a glob, `<<target::*>>`; a type declaration (`pub struct`,
`enum`, `trait`, `union`, after any attributes, and `pub type` only as the
standalone alias defined below); a member, `pub
<path>::<member>…`, whose owner is the deepest path segment naming a
declared type of that package (so a struct-variant field belongs to its
enum); an `impl` header, which opens an inherent block (`impl X`) or a trait
block (`impl … for X`); a `pub fn` or `pub const` item, which belongs to the
open block, or to a trait declaration until the next `impl` header; and an
associated type, classified by its context:

- **in a trait block** (`impl … for X` followed by
  `pub type <path>::X::Error = …`): the associated type of that trait
  implementation, as `Magnification::Error` (`RL:3228-3229`) and
  `Capacity::Error` (`api/pert-ui-core/default.txt:373-374`). It belongs to
  the block, is not a declaration and is not followed, as nothing in a trait
  block is (section 2.3). One path can carry several such definitions, one
  per implementation, with different values: `ChoiceId::Error` is
  `ParameterError` for `TryFrom<&str>` and `AnyValue` for
  `TryFrom<AnyValue>` (`api/pert-ui-core/default.txt:406-407`, `:412-413`);
  they never enter the declaration index, so they cannot raise the ambiguity
  guard. The owner may be a primitive (`pub type bool::Error`, a core
  implementation for `bool`); it is not a pert-ui path and is ignored with
  the block.
- **after a trait declaration** and before the next `impl` header
  (`pub type pert_ui_core::HostValue::Canonical: pert_ui_core::ParameterValue`,
  the line after `pub trait pert_ui_core::HostValue`): a member of the
  trait, followed like its methods when the trait is followed.
- **a standalone alias**, `pub type <package>[::<module>…]::Name = Target`
  outside any block, whose last-but-one segment is a module: a type
  declaration of `Name`, whose one followed edge is `Target`. None exists in
  the listings at ee55226 (every `pub type` line there is one of the two
  forms above); the rule is stated so that one added later is neither
  dropped nor misread.

Rust has no stable inherent associated types, so a `pub type` line in an
inherent block fails the test as an unknown form. Any other line fails the
test with its text, so a format change cannot pass silently; a tool update
regenerates the listings and runs this test in the same change.

**Resolution.** Signatures name types by their defining path, often a
private one: the facade listing writes `pert_ui_runtime::runtime::Runtime`
and `pert_ui_runtime::drawing::DisplayList` (`api/pert-ui-cpu/default.txt`,
`render`), the runtime listing writes `pert_ui_core::family::ParameterValue`.
A type is identified by its package and its last segment, without generics.
The declaration index holds structs, enums, traits, unions and standalone
aliases only. Two guards make that safe: within a package, every declaration of a name
under any path must list the same members once the path prefix is removed,
and no name may be declared by two packages; either violation fails the test
as an ambiguity. Both hold at ee55226: the listing repeats one item's block
under each of its paths, every runtime type declared under a module path is
also declared at the root except `Driver`, `TestError` and
`HostIntegration`, which only `testing` and `host` declare, and no type name
is declared by two packages (checked with the prototype). An unresolved
`pert_ui…` path with an upper-case last segment in a followed line fails the
test, so a parse gap cannot drop an edge.

**Membership** of a configuration's `testing` and prelude is the names
declared or re-exported under that module path; a glob re-export expands to
the runtime module it names (the facade prelude's
`<<pert_ui_runtime::prelude::*>>` line, `api/pert-ui/default.txt`).

**Assertions,** each failure naming the configuration and the path that
reached the name:

1. Every reached pert-ui-owned type is in the configuration's `testing`, in
   its prelude, or excluded (section 2.4).
2. Every excluded name is reached in at least one configuration and is not a
   member of `testing`: an exclusion cannot outlive its reason or contradict
   the module.
3. In every facade configuration, `pert_ui::testing` holds every name
   `pert_ui_runtime::testing` holds, as its page promises ("re-exports this
   one", `R/testing.rs:37-39`), so the two explicit lists cannot drift.
4. The nine `Runtime` seeds exist as inherent methods.

**Self-tests** in the same binary, on inline listing fragments, one per
rule: a field type, a tuple payload, a struct-variant field, an inherent
method against a trait-impl method, a nested error, the prelude stop, an
exclusion stop, an unused exclusion, an ambiguous name, an unknown line, and
the associated-type forms copied from the committed listings: the
`Magnification::Error` block, the three `ChoiceId` `TryFrom` blocks with
their differing `Error` definitions (no ambiguity, no edge), the
`HostValue` trait declaration with `Canonical` and `Error`, a primitive
owner's `bool::Error`, and a synthetic standalone alias (a declaration with
one edge). The main check at the pre-change listings must then fail only
with the membership reports of section 3.1, never with a parse, resolution
or ambiguity failure; the Progress entry records that.

**Limits, stated in its module comment:** it checks types, not functions
or constants; it sees the x86_64 Linux view the listings record; seeds outside
`testing`'s own types are the nine named methods, so a new `Runtime` method
for tests is added to the list by the part that adds it.

The facade's `testing` keeps an explicit list rather than a glob of the
runtime module, as today: a glob re-export renders as one
`pub use pert_ui_runtime::testing::*` line on the facade page, hiding the
names a tester looks for, and assertion 3 removes the drift an explicit list
would otherwise allow.

## 5. The `svg` module (TODO.md, P2-D1)

A crate that globs the facade's root and depends on the published `svg`
crate meets the module in two ways, reproduced with rustc 1.99.1 on stand-in
crates in the session scratchpad (a library with `pub mod svg`, a crate named
`svg`, edition 2024):

- `use svg::Document;` is E0659 "`svg` is ambiguous … a name from a glob
  import and an outer scope", with the help "use `::svg` to refer to this
  crate unambiguously";
- `svg::Document::new()` outside a `use` resolves to the glob's module and
  fails with E0433 "cannot find `Document` in `svg`", with no hint of the
  glob;
- globbing `prelude` and `testing` instead compiles.

The second case misleads without any pointer to the cause, which is what L2
avoided for `windows` by naming the module `windowing`
(`plans/usability/layering-design-L2.md:400-414`). **Recommended (P2-D1):**
rename the module `svg_images`, parallel to the `images` module that holds
raster decoding (`F/lib.rs:34-35`); the `svg` feature and the root `Svg`
widget keep their names. The consumers are few (section 9). No crate named
`svg_images`, or named like any other facade root module (`cpu`, `images`,
`guide`, `host`, `testing`, `prelude`, `component`, `parameter`,
`commands`, `windowing`, `inspection`, `live`), is in the workspace's
`Cargo.lock` (checked at ee55226).

## 6. Collision review

Repeated from L2 section 4 for the globs P2 changes.

- **Between pert-ui globs: none by construction.** Every added name is the
  crate-root item already in `inspection`, `component`, `host` or `windowing`
  (`RL` declarations; `Rect` in `component`, `inspection`, `windowing`;
  `NativeWindowId` in five modules), or the facade's own `cpu` re-export of
  `CpuRenderError`, which no runtime module holds. `ThemeMode` is the root
  item. Overlapping globs of one item are valid; `every_glob_combines`
  resolves every name (section 10).
- **Explicit imports are not globs.** `R/testing.rs:73-76` drops
  `FlushReport`, `RevealRefusal` and `Semantics` from its private `use` when
  its `pub use` adds them (E0252, as L2 reproduced,
  `layering-design-L2.md:447-451`). A consumer's named import of an item a
  glob also brings, such as `use pert_ui::{ComponentInspection, SemanticRole,
  prelude::*, testing::*}` (`crates/pert-ui/tests/foundation/component_module.rs:10`)
  or `use pert_ui::ThemeMode;` beside `prelude::*`, compiles without a warning
  (checked with rustc 1.99.1 and `-D warnings`: the explicit import shadows the
  glob and `redundant_imports` is allow-by-default).
- **Consumer-local items** shadow glob imports. Local items named like an
  added name exist in two places, and neither scope imports a changed glob:
  - the gallery's `type Capture = Rc<RefCell<…>>`
    (`apps/gallery/src/workspace_check.rs:12`), in a file that globs the
    facade root, which already exports `Capture`;
  - inside the runtime, private items that already shadow the crate-root
    item of the same name their module's `use crate::*` or `use super::*`
    brings: `type Capture` (`R/menu_bar.rs:11`, beside `use crate::*` at
    `:6`), `struct Pending<T>` (`R/parameter_binding/remote.rs:142`, beside
    `use super::*` at `:2`) and `struct Rect`
    (`R/validation/development.rs:448`, beside `use super::*` at `:6`). A
    crate-root glob brings the modules `testing` and `prelude` as names, not
    their members, so P2's changes reach none of these scopes.

  The scan covered every `struct`, `enum`, `type`, `trait` and `union` named
  like an added name in `crates/`, `apps/`, `tests/` and
  `plans/usability/task4-app` at ee55226; revision 1 had scanned only tests,
  examples, the gallery and the guide. No maintained file globs `testing`
  beside a glob that defines a different item of an added name (the 29 files
  that glob `testing`: their other globs are pert-ui modules; the runtime's
  `super::*` and `crate::*` globs sit in modules whose doc examples, separate
  crates, are the only `testing::*` users).
- **Standard-library globs.** `Pending` is also `std::future::Pending`: a
  test that globs `std::future::*` and `testing::*` and writes `Pending` gets
  E0659 and names one explicitly. No other added name is an item of
  `std::io`, `fmt`, `collections`, `sync`, `rc`, `cell`, `time`, `path`,
  `future` or `task`.
- **Other graphics crates.** A test that globs another library's prelude
  holding `Rect` beside `testing::*` must name one where it writes `Rect`, as
  it must today for `Point` and `Size` with the prelude.
- **Module names against crates:** section 5.

## 7. Public API

`R/testing.rs`, replacing `:73-86` (order as rustfmt sorts it):

```rust
use crate::{
    CommandKey, KeyChord, KeyCode, Modifiers, Point, RevealOutcome, Runtime, RuntimeError,
    TextCommand, TextCommandOutcome, WidgetId,
};
use std::time::{Duration, Instant};

#[doc(no_inline)]
pub use crate::host::HostIntegration;
// Input by hand, the viewport and what the driver's steps return.
#[doc(no_inline)]
pub use crate::{
    ClickCounter, CommandKeyInput, FlushReport, FocusDirection, InputOutcome, KeyAction,
    Magnification, PointerButton, PointerEvent, PointerKind, RevealRefusal, Viewport,
};
// Reading what the runtime shows: semantics, queries and validation.
#[doc(no_inline)]
pub use crate::{
    ComponentDetails, ComponentInspection, Depth, Detail, InspectValue, InspectionQuery,
    QueryResult, Selector, SemanticAction, SemanticItem, SemanticRole, SemanticValue, Semantics,
    ValidationCode, ValidationIssue, ValidationMarker, ValidationReport,
};
// Captures.
#[doc(no_inline)]
pub use crate::{
    Capture, CaptureError, CaptureRequest, CaptureTarget, FrameImage, InspectionReply,
    NativeWindowId, Pending, Rect, WaitError,
};
```

`R/lib.rs`, in `pub mod prelude` (`:412-492`), a group of its own:

```rust
    // The theme mode a theme switch writes; the theme services stay at the root.
    #[doc(no_inline)]
    pub use crate::ThemeMode;
```

`F/lib.rs`:

```rust
#[cfg(feature = "svg")]
pub mod svg_images; // was `svg`; files `svg.rs` and `svg/component.rs` move
// …
#[cfg(feature = "svg")]
pub use svg_images::component::Svg;

pub mod testing {
    #[doc(no_inline)]
    pub use pert_ui_runtime::testing::HostIntegration;
    #[doc(no_inline)]
    pub use pert_ui_runtime::testing::{/* the runtime module's 39 re-exports */};
    #[doc(inline)]
    pub use pert_ui_runtime::testing::{Driver, TestError};

    #[cfg(feature = "native")]
    pub use crate::app::HeadlessApp;
    #[cfg(feature = "cpu")]
    #[doc(no_inline)]
    pub use crate::cpu::{CpuRenderError, CpuRenderer};
}
```

No signature, behavior, persisted format, real-time endpoint or backend
type changes; no item is added or removed except the module path
`pert_ui::svg` → `pert_ui::svg_images`.

**Rustdoc intent:**

- **`testing` pages** (`R/testing.rs:1-40`, `F/lib.rs:199-224`): the list
  grows by the groups of section 3.1, each name linked with what it is for
  (for example "[`FlushReport`], which [`Driver::settle`] and
  [`Driver::advance`] return"; "[`Semantics`] with [`SemanticRole`],
  [`SemanticAction`] and [`SemanticItem`]"; "[`ValidationReport`] from
  [`Runtime::validation_report`]"; "[`Pending`], [`InspectionReply`],
  [`WaitError`], [`Capture`] and [`CaptureError`] for
  [`Runtime::request_capture`]"); a `# Membership` section with section 2.1's
  wording; a `# Where the other names are` list of section 2.4's exceptions
  with their modules and of `HostIntegration`'s vocabulary in `host`. The
  facade page adds `CpuRenderError` with `cpu`. The runtime page's sentence on
  the facade (`R/testing.rs:37-39`) adds the CPU renderer's error. The
  facade page's "The re-exported names stay at the crate root too, except
  `HostIntegration`" (`F/lib.rs:219-221`) is already untrue for
  `CpuRenderer`, which is in `cpu` (`F/cpu.rs:39-40`), and `HeadlessApp`,
  which only `testing` exports (`F/lib.rs:278-279`); it is corrected to name
  `HostIntegration`, `cpu`'s `CpuRenderer` and `CpuRenderError` and
  `HeadlessApp` as the exceptions, a defect of the same kind on the page P2
  rewrites. The first
  example of each page gains a helper that returns `Semantics` and matches a
  `SemanticRole`, the case 1-claude and 4-codex hit.
- **Prelude pages** (`R/lib.rs:330-411`, `F/lib.rs:47-133`): a membership
  bullet "[`ThemeMode`], which a theme switch writes; [`ThemeManager`] and
  [`Palette`] stay at the crate root"; the root row's example becomes
  `use pert_ui::{Palette, Workspace};` (runtime: `pert_ui_runtime::`); the
  testing row (`F/lib.rs:119`, `R/lib.rs:393`) adds "semantics, query
  results and validation"; `svg::SvgError` (`F/lib.rs:104`) becomes
  `svg_images::SvgError`.
- **`svg_images` page** (from `F/svg.rs:1-45`): unchanged in substance, plus
  "The name is `svg_images`, not `svg`: a crate that globs the crate root
  and depends on the `svg` crate would otherwise find `svg` ambiguous in a
  `use` path and resolve it to this module elsewhere", as `windowing` says
  (`F/lib.rs:668-672`).

## 8. Documentation

- **Testing chapter** (`G/testing.md:1-17`): the introduction states
  section 2.1's rule in two sentences, names the groups `testing` holds and
  says that the window, event, parameter and source answers are in
  [`inspection`](crate::inspection) and a host's records in
  [`host`](crate::host). "Reading what the runtime shows" (`:19-54`) gains,
  in its example, a helper with a written return type: one that returns the
  `Semantics` of a component and one that reads an `InspectValue` property
  (an `IntegerInput`'s `dirty`, `R/text_fields/number_inputs.rs:2090`)
  from a `QueryResult`, asserting `SemanticRole::NumberInput`. Both blocks
  keep importing only `prelude` and `testing`, which
  `representative_programs_import_only_their_globs` checks.
- **Appearance chapter** (`G/appearance.md:507-517`), `F/root.md:78`,
  `crates/pert-ui/README.md:78`, `F/status.md:27`, `docs/contracts.md:1598`,
  `G/links/svg.md:2` and `G/links/no-svg.md:2` (the docs.rs URL
  `pert_ui/svg_images/index.html`): the module's new name, with the link label
  `` [`svg_images`] ``. `docs/manual.md:91` and `:510` name the feature and
  the widget, which keep their names.
- **ThemeMode in the guide:** where a guide example globs the prelude and
  also imports `ThemeMode` from the root (P1 may write such imports when it
  replaces the root globs in the appearance chapter), the named import is
  dropped so the chapter teaches the prelude; examples that import every name
  from the root (`G/appearance.md:26-28`, `:96`, `G/controls.md:516`) stay.
- **`docs/verification.md`** (`:337-355`, the diagnostics package): one
  paragraph on the closure check, what it reads, that it runs in the workspace
  tests and that the API listing gate keeps its input current.
- **TODO.md:** the 2026-10-27 `pub mod svg` entry is removed. The
  2026-10-23 entry on `tests/diagnostics/ui/c01_*` and `c06_*` touches the
  package P2 adds a test to and stays open.

## 9. Migration manifest

**Maintained, migrated:**

| Consumer | Change |
|---|---|
| `R/testing.rs` | the re-exports and private imports of section 7; module rustdoc |
| `R/lib.rs` | `ThemeMode` in the prelude; prelude page (section 7) |
| `F/lib.rs` | `testing` list and `cpu` line; `svg_images` declaration and `Svg` re-export; module and prelude rustdoc |
| `F/svg.rs`, `F/svg/component.rs` | moved to `F/svg_images.rs` and `F/svg_images/component.rs`; intra-doc links `crate::svg::rasterize` and `rasterize_mask` (`F/svg/component.rs:46-47`) |
| `crates/pert-ui/tests/presentation/svg.rs` | `use pert_ui::svg_images::{self, SvgError};` and every `svg::` path (`:4`, `:21-395`) |
| `crates/pert-ui/tests/foundation/modules.rs` | section 10 |
| `crates/pert-ui/tests/foundation/vocabulary.rs` | the prelude-only `ThemeMode` fixture (section 10) |
| `tests/diagnostics` | `tests/testing_closure.rs`; `Cargo.toml`'s description and `src/lib.rs`'s page name the closure check beside the snapshots |
| guide, `root.md`, `status.md`, facade README, `docs/contracts.md`, `docs/verification.md` | section 8 |
| `api/` | regenerated (section 11) |
| `CHANGELOG.md`, `TODO.md`, `Cargo.toml`, `Cargo.lock` | section 11 |

**Maintained, unaffected** (checked at ee55226): the packaged examples and
their headless sessions (none names `pert_ui::svg::`; their named imports of
names `testing` now holds, such as `examples/inventory/session.rs:8`, stay
valid); the gallery (root glob; `Svg` from the root; `apps/gallery/src`
names no `svg::` path); the native checks; the runtime's README and tests;
the diagnostics fixtures under `tests/diagnostics/ui/` (they name no changed
path; the snapshot run reviews any changed suggestion); `docs/manual.md`;
`plans/usability/task4-app` (`src/lib.rs:6`, `src/main.rs:3` glob only the
prelude and define no `ThemeMode`, so the fixture check is not part of this
part's gate). Named imports that a widened glob now covers are left as they
are: nothing requires the change.

**Frozen, not migrated:** the submissions, probes and triage under
`plans/usability/runs/` and `~/pert-ui-usability/`, the reserved tasks under
`plans/usability/reserved/`, the API audit's diagnostic corpus under
`plans/usability/api-audit/diagnostics/`, the closed design records
(`plans/usability/layering-design-L2.md` among them; this checkpoint records
the change to its D2) and the CHANGELOG's history.

## 10. Tests

P2 changes no behavior: the closure check is new test infrastructure, and
everything else is an import guarantee. Following L4b's split:

- **New-behavior tests** name the assertion that detects the old state and a
  mutation that compiles and makes that assertion fail.
- **Contract tests** check guarantees P2 keeps or extends; their evidence is
  the compile failure a named removal causes, recorded in the Progress entry as
  L2 recorded its own.

### New-behavior tests

1. **The closure check** (`tests/diagnostics/tests/testing_closure.rs`).
   Named assertion: assertion 1 of section 4 holds in all five
   configurations. Mutation: run it against the listings of ee55226 (the
   committed `api/` before the change, for example from a `git worktree` of
   that commit's `api/` beside the new test); it fails, listing exactly the
   names of section 3.1 with their paths, including `Semantics` from
   `Driver::semantics`, `FlushReport` from `Driver::settle`, `InspectValue`
   through `QueryResult` and, in cpu and desktop-cpu,
   `pert_ui_cpu::CpuRenderError` from `CpuRenderer::render`. The recorded
   output is the evidence.
2. **The payload traversal matters.** Mutation of the checker: stop
   following fields and variants. At the pre-change listings it then no longer
   reports `ComponentInspection`, `ComponentDetails`, `InspectValue`,
   `RevealRefusal`, `Magnification`, `ValidationIssue`, `ValidationCode` or
   `ValidationMarker`, which only a field or a variant holds (prototype run);
   the self-tests for fields and struct-variant fields fail. Recorded with
   test 1.
3. **Exclusion hygiene.** Named assertion: assertion 2. Mutation: add a name
   nothing reaches (for example `ProfileSnapshot`) to the exclusion list; the
   test fails naming it. A second mutation adds `Semantics` to the exclusion
   list after it joined `testing`; the test fails on the contradiction.
4. **The facade keeps the runtime's list.** Named assertion: assertion 3.
   Mutation: remove `ClickCounter` from the facade's explicit list only; the
   test fails naming it in the four facade configurations. Assertion 1 alone
   would pass: no seed's signature names `ClickCounter`, which is a seed only
   while it is a member (prototype run).

These four are the new evidence: each mutation compiles and makes its named
assertion fail. The import guarantees below are contract tests, whose
evidence is a compile failure.

### Contract and compatibility tests

- **`ThemeMode` from the prelude alone** (`foundation/vocabulary.rs`, a
  child module with `use pert_ui::prelude::*;` only): set the dark mode
  through `ui.theme().set_mode(ThemeMode::Dark)?`, assert
  `ui.theme().mode()?` is `ThemeMode::Dark`, and match both variants. The
  theme behavior it runs exists today; the guarantee is the import. Named
  removal: the prelude re-export, which fails to compile (E0433 for
  `ThemeMode`).
- **`every_name`** (`foundation/modules.rs`): `testing_names` gains the 23
  names, `HostIntegration` (missing today) and, under `cpu`,
  `CpuRenderError`; `prelude_names` gains `ThemeMode`; a new `host_names`
  lists `pert_ui::host`'s members (`F/lib.rs:527-540`). Named removals that
  fail to compile: `Semantics` from either `testing`, `CpuRenderError` from
  the facade's (with `--features cpu`), `ThemeMode` from the runtime prelude.
- **`every_glob_combines`** globs `host::*` as well and resolves, with
  `use self::{… as _}`, the complete union of the expanded `every_name`
  lists, as L2 specified (`layering-design-L2.md:769-778`): every name the
  modules export, feature-gated names under the same `cfg`. Beyond P2's
  additions, `HostIntegration`, `ThemeMode` and, under `cpu`,
  `CpuRenderError`, that union includes the nine host names the list does
  not resolve today (checked against `F/lib.rs:531-539` and
  `modules.rs:151-269` at ee55226): `CaptureJob`, `Clipboard`,
  `ClipboardError`, `KeyDispatch`, `PhysicalExtent`, `PreparedDetach`,
  `PresentationAction`, `PresentationRate` and `PresentationRequest`. A glob
  alone diagnoses an ambiguity only where a name is used, so the `host_names`
  list checks availability and only this resolution checks collisions. The
  conflicting-re-export guard L2 recorded (`layering-design-L2.md:781-787`)
  is not repeated, since the mechanism is unchanged.
- **The testing chapter and module examples** (doctests): the helper
  returning `Semantics` and the `InspectValue` read compile with `prelude`
  and `testing` only; `representative_programs_import_only_their_globs`
  (`foundation/prelude.rs`) still accepts every chapter block. Named removal:
  `InspectValue` from `testing` fails the chapter's doctest.
- **The checker's self-tests** (section 4), each against a fragment that
  violates its rule.
- **The renamed module:** `tests/presentation/svg.rs` and the module's
  doctest pass under `--features svg`; the `svg` configuration's API listing
  shows `pert_ui::svg_images` and no `pert_ui::svg`. A scratch crate outside
  the workspace, with a path dependency on the facade (`features = ["svg"]`),
  a local crate named `svg`, `use pert_ui::*;` and `use svg::Document;`,
  fails with E0659 at the parent commit and compiles at P2's; recorded, not a
  CI test.
- **Diagnostics snapshots:** `cargo test --locked -p pert-ui-diagnostics --
  --ignored`; a changed import suggestion in a `.stderr` (rustc may now list
  a `pert_ui::testing::…` path) is reviewed like the `api/` diff and must not
  turn a shaped message into a missing-name error.

## 11. Scope, version and evidence

- **Version: 0.3.77**, a workspace patch bump with the internal version
  requirements and `Cargo.lock`, after P1's 0.3.76.
- **`api/` diff:** in `api/pert-ui-runtime/default.txt` and `serde.txt`, the
  item blocks of the 23 names under `pert_ui_runtime::testing::` and
  `ThemeMode`'s under `pert_ui_runtime::prelude::` (the listing repeats each
  block under every path, so the diff is long and mechanical and is reviewed
  by its declaration lines); in every `api/pert-ui/*.txt`, 23
  `pub use pert_ui::testing::…` lines, and in `cpu.txt` and
  `no-default-desktop-cpu.txt` `pub use pert_ui::testing::CpuRenderError`; in
  `svg.txt` and `no-default-svg.txt`, `pub mod pert_ui::svg` and its items
  under `pert_ui::svg_images`, and `Svg::new` and `Svg::tinted` returning
  `pert_ui::svg_images::SvgError`. No other line changes.
- **CHANGELOG 0.3.77**, citing this checkpoint:
  - `Breaking (TODO.md svg collision):` the facade module `pert_ui::svg` is
    `pert_ui::svg_images`, so a crate that globs the crate root and depends
    on the `svg` crate no longer finds `svg` ambiguous. Replacement usage:
    `use pert_ui::svg_images::{SvgError, intrinsic_size, rasterize,
    rasterize_mask};` and `svg_images::MAX_NESTING`; the `svg` feature and
    `pert_ui::Svg` are unchanged.
  - `Added (F3):` to `pert_ui::testing` and `pert_ui_runtime::testing`, the
    names section 3.1 lists, each by name; with `cpu`, `CpuRenderError` in
    `pert_ui::testing`. A test that names what the driver, a query, the
    validation report or a capture hands it needs only `prelude` and
    `testing`.
  - `Added (F3):` `ThemeMode` to `pert_ui::prelude` and
    `pert_ui_runtime::prelude`; the other theme services stay at the root.
  - `Documentation (F3):` the `testing` pages and the testing chapter state
    the module's closure rule and where the excluded records are; the prelude
    pages' "where the other names are" rows change.
  - `Tests (F3):` `pert-ui-diagnostics` checks from the committed API
    listings that every type `testing`'s names hand a test is in `testing` or
    the prelude, in the runtime and four facade configurations; the facade's
    module tests cover the `host` module and `HostIntegration`.
- **Gate** (plan P2 and AGENTS.md's public-API row, plus the `svg`
  configurations the rename touches): the workspace checks as CI runs them
  (formatting, Clippy, tests including the new check, the five headless
  example sessions, doctests, warning-free rustdoc), the diagnostics
  snapshots, the API listing gate on x86_64 Linux, Clippy, tests and rustdoc
  with `-p pert-ui --features cpu`, `--no-default-features` and
  `--no-default-features --features desktop-cpu`, Clippy, tests and rustdoc
  with `--features svg`, Clippy and tests with `--no-default-features
  --features svg`, the scratch-crate check of section 10, a link check of the
  changed docs.rs URL and anchors, `git diff --check`, and Codex's
  implementation review. Not required: MSRV, release-profile tests,
  packaging and the fixture check (no dependency, feature definition or
  fixture-used API changes).
- **Native evidence:** not applicable; no presentation or platform code
  changes.
- **Records:** the plan's Progress entry (the pre-change output of test 1,
  the mutations of section 10 and the scratch-crate result); the coverage
  rows of F3 and the `svg` TODO are closed there.
- **Later parts** keep the rule, and the check enforces it on their
  signatures: P4's quiet-interval report type joins `testing` (it is a
  `Driver` method's return type); `TestError::Draw(DrawError)` does not
  require `DrawError`, a nested error under section 2.3, unless P4's
  checkpoint shows a test matching it, in which case P4 adds it; P5's
  `address_children` and P8's `click_item` and resolved prefix add no type
  beyond `WidgetId` and `String` if their signatures are as the plan states;
  P8's item bounds use `Rect`, already in `testing`.

## Corrections to the plan

- **The hand-checked list was incomplete.** Beyond its ten names (plan P2),
  the rule reaches `Depth`, `Magnification`, `Rect` and `NativeWindowId`
  (constructor and builder arguments of members), the capture chain
  (`Capture`, `CaptureError`, `Pending`, `InspectionReply`, `WaitError`) and,
  with P2-D2, the validation family; section 3.1 is the settled list.
- **`HostIntegration` as a seed** was not stated; seeding it would pull the
  host vocabulary into `testing` (section 2.2).
- **Nested errors** need a rule of their own, or `CpuRenderError` would bring
  `DrawError` and `LimitError` (section 2.3).
- **The listing format:** the facade listing records its prelude and other
  globbed modules as `<<…>>` lines, not names, and every listing names types
  by defining paths that are often private; "resolves each type path to its
  defining item" is done by package and name with two guards (section 4).
- **`every_name` and `every_glob_combines` are not "as for every module
  today":** neither covers the `host` module, and `testing_names` omits
  `HostIntegration` (L3's gap, fixed here as a defect of the same kind).
- **The facade's `testing` page** says every re-exported name stays at the
  crate root except `HostIntegration` (`F/lib.rs:219-221`), which is untrue
  for `CpuRenderer` and `HeadlessApp` today; P2 rewrites that page and
  corrects it (section 7).
- **The gate** gains the `svg` configurations, which the module rename
  touches.

## Not in this part

- The `host` module's own closure over `HostIntegration`'s signatures (a
  host-author guarantee L3 did not state); the check could be extended to it
  later with `host` as the module.
- Functions and constants in the closure check (section 4's limits).
- Re-exporting excluded records into `testing` for convenience.

## Decisions

- **P2-D1: the `svg` module.**
  - **Recommended, approved by the author:** rename it `svg_images`
    (section 5). It removes a silent
    misresolution, not only a diagnosed ambiguity, for a published crate an
    SVG-producing application may use, at the cost of one test file and a few
    documentation links; the maintainer accepts breaking changes that improve
    the API.
  - **Alternative A:** keep `svg` and document the `::svg` spelling on its
    page and the prelude page. Non-breaking, but helps only a reader who finds
    the page after the E0433 that names neither the glob nor the crate.
  - **Alternative B:** another name: `svg_image` (TODO.md's example; singular
    against `images`) or `vector_images` (loses the format's name, which is
    what a reader searches for).
- **P2-D2: which query answers `testing` holds.**
  - **Recommended, approved by the author:** the component answers and
    development validation:
    `Runtime::validation_report` is a seed and the validation family joins
    (section 3.1). Tests are where validation is consumed: two task runs read
    it in their tests (section "What testers wrote"), R2-21's
    `EscapesParent` was met there, and P7's and P8's tests assert validation
    codes. The other answers stay in `inspection`.
  - **Alternative A:** the component answers only; validation stays in
    `inspection`, four names fewer, and a test that matches a
    `ValidationCode` globs `inspection`.
  - **Alternative B:** every answer of `InspectionQuery`, since its builders
    (`windows`, `parameters`, `sources`, `events_after`) are `testing` names:
    no exclusion for query answers, but `testing` would hold most of
    `inspection` and the core's parameter snapshot types, which no run wrote.

## Review record

- **Revision 1 (2026-10-28):** written from the code at ee55226.
- **Revision 1, Codex review (2026-10-28, against ee55226):** needs
  revision; record `~/pert-ui-usability/codex-prompts/codex-design-P2.out`.
  It reproduced the 23 missing runtime names and the submission counts, and
  found the exports, the rename, the migration scope and the unchanged
  persisted formats consistent with the plan. Each point was checked against
  ee55226 before revision 2 applied it; none was rejected.
  1. **Associated types (should-fix).** Section 4 classified every
     `pub type` line as a declaration, while the listings hold trait-impl
     associated types (`RL:3229`, `api/pert-ui-core/default.txt:374`), one
     path with differing definitions (`ChoiceId::Error`, `:407`, `:413`) and
     trait-declared ones (`HostValue::Canonical`, `Error`), which the
     name-based index would report as ambiguous. Revision 2 classifies them
     by their block (trait implementation: not indexed, not followed; trait
     declaration: a member of the trait), defines standalone aliases
     separately (none exist at ee55226, checked over every listing), fails
     on any other `pub type` form, and adds self-tests copied from these
     listing forms; the pre-change run must fail only with membership
     reports. The prototype behind sections 2 and 3 already skipped
     associated types, so the name lists do not change.
  2. **The `ThemeMode` fixture (should-fix).** Its mutation removes an export
     and fails to compile, which is contract evidence, not a compiling
     mutation (`plans/usability/layering-design-L4b.md:813-820`). Revision 2
     moves it to the contract tests; the four checker tests remain the
     new-behavior evidence.
  3. **The collision inventory (nit).** "The repository's only local item"
     was false: the runtime defines `type Capture` (`R/menu_bar.rs:11`),
     `struct Pending<T>` (`R/parameter_binding/remote.rs:142`) and
     `struct Rect` (`R/validation/development.rs:448`). Revision 2 lists
     them, states the scan's scope, and explains why the changed globs do not
     reach their scopes (their `crate::*` and `super::*` globs bring the
     `testing` and `prelude` modules as names, not their members, and the
     local items already shadow the root items of those names).
- **Revision 2, Codex recheck (2026-10-28, against ee55226):** needs
  revision; record `~/pert-ui-usability/codex-prompts/codex-design-P2-r2.out`.
  It found revision 1's three points resolved and reproduced the missing-name
  sets in all five configurations independently. Both points were checked
  against ee55226 before revision 3 applied them; none was rejected.
  1. **The host glob's collision check (should-fix).** Revision 2 globbed
     `host::*` in `every_glob_combines` but resolved only P2's additions;
     nine of the host module's names (`F/lib.rs:531-539`) are resolved
     nowhere in `modules.rs:151-269`, and an unused glob name is never
     checked for ambiguity. Revision 3 requires the complete union of the
     expanded `every_name` lists, naming the nine (section 10).
  2. **Citation (nit).** The runtime prelude page's testing row is
     `R/lib.rs:393`; `:394` is the custom-host row. Corrected in section 7.
- **Revision 3, Codex second recheck (2026-10-28):** no remaining findings,
  no regressions. Ready for approval; approved by the author under the
  maintainer's delegation, with P2-D1 and P2-D2 as recommended.
