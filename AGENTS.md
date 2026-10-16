# Project instructions for pert-ui

## Scope and current phase

These instructions apply throughout this repository. More specific nested
instructions govern their subtree. User instructions take precedence. Code,
comments, documentation, UI strings and commit messages are in English; respond
to the user in their language.

This is a public Rust GUI library built from scratch, with general widgets and
opinionated defaults. Application requirements guide its capabilities, not
domain-specific widget types. Future unit presets may compose general
parameter/mapping/formatting APIs; do not bake application semantics into the
base controls or data model.
The Cargo workspace implements normalized values, general numeric parameter
specifications, a local edit protocol, UI-local signals/bindings, retained component
lifecycle/scheduling, multi-key input and inspection contracts. Both headless and
native galleries use the public facade. The native gallery has retained layout,
text/IME, standard controls, numeric inputs, scrolling, splits, tabs, overlapping
in-view windows, modal/nonmodal surfaces and bounded notifications in one
winit/wgpu window, plus experimental retained native multi-window support.
Live-data bridges from a producer thread to widgets exist, and the workspace
arrangement is capturable and restorable through a separately versioned document.
The library ships no MCP server: applications build their own tooling transport on
the public inspection contract. No library has
been published. Do not describe proposed APIs, platforms or performance targets
as verified; [CHANGELOG.md](CHANGELOG.md) and the design notes record what exists.

Read the relevant design before changing it:

- [Architecture and direction](docs/architecture.md)
- [State and interaction contracts](docs/contracts.md)
- [Rendering and platform contract](docs/rendering-platform.md)
- [Verification and evidence](docs/verification.md)
- [Application manual](docs/manual.md)

Earlier consumer inventories are historical research. Their architecture and
sibling-repository paths are not library requirements. AccessKit, legacy OS
support, X11, browser rendering and plugin hosting are outside the initial
scope. The CPU renderer (`pert-ui-cpu`, the facade's `cpu` feature) is in
scope: it renders scenes and serves captures without a GPU, and with
`App::software_rendering` native windows present its frames through
softbuffer instead of creating a GPU device. Software presentation is opt-in,
with no automatic fallback.

## Separation of concerns

Use the package boundaries in [architecture.md](docs/architecture.md).
`pert-ui-runtime` must build without window or GPU dependencies. `pert-ui-wgpu`
consumes the public `DisplayList`; its explicit backend interop API may accept
wgpu surface targets, but no backend type may leak into runtime or facade APIs.
Keep dependencies acyclic and directed toward the core. A directory name alone is
not proof of a boundary: inspect imports, Cargo dependencies and the public
signatures.

- Core quantities, identities, parameter contracts and edit transitions must be
  testable without a window, GPU or operating-system services.
- The facade exposes pert-ui-owned types. winit, wgpu, Parley and OS handles stay
  behind internal platform, renderer and text boundaries. Do not casually re-export
  a dependency or expose its types through bounds, errors or callback signatures.
- Runtime/state, layout, widgets, text, renderer and platform integration have
  separate responsibilities. Widgets express drawing through `Painter`, not GPU
  commands; platform adapters translate events, not application actions.
- No production or development dependency, build script, test or shipped example
  may require a specific consumer, a sibling checkout, a developer home directory,
  a specialized device or an application-owned tooling server. Domain adapters belong in the host.
- The library does not own the application's data model, background processor
  or undo history. Local state, authoritative edits and lossy telemetry
  are separate contracts with explicit ownership and failure behavior.
- Ordinary application code uses the public facade, bindings and defaults.
  Custom widgets must be possible using documented public extension points.
  Avoid privileged paths that make built-in widgets impossible for consumers
  to reproduce through the public contract.
- Application consumers choose content and domain meaning, not repaint timers,
  renderer types or cache bookkeeping. Keep the supported default path complete.
- Add abstractions for demonstrated responsibilities. Do not create an empty
  crate/module for every eventual widget, hypothetical backend or future plugin.

## Public API and documentation are one deliverable

Every exposed item, including public fields, variants and trait methods, needs
useful rustdoc in the same change. Private implementation should stay private;
use `pub(crate)` when only crate-internal access is necessary. Do not hide a
consumer-required API behind `#[doc(hidden)]` to bypass documentation.

Document purpose, defaults, valid values/units, ownership, thread restrictions,
lifecycle, errors and relevant performance behavior. Describe gesture ordering,
cancellation, freshness and dropped-data semantics where applicable. Public traits
must explain how external implementations maintain their invariants.

Each important use case needs a complete example importing only the public API.
Rustdoc examples should run as doctests. `no_run` is appropriate for opening
windows or requiring a GPU, but such examples must still compile; native gallery
checks cover execution. `compile_fail` verifies actual type-level restrictions.
Do not use `ignore` to conceal a broken or unfinished normal-use example.

The root `[workspace.lints]` denies `missing_docs`, `unsafe_code`,
`rustdoc::broken_intra_doc_links`, `rustdoc::private_intra_doc_links` and
`clippy::missing_errors_doc`; every
member inherits them with `[lints] workspace = true`. New packages must inherit
them, and no module may relax them without discussing it with the user. Treat
rustdoc warnings as errors in documentation builds. Lints detect missing
text/links, not semantic completeness; review the generated documentation from a
first-time consumer's perspective.

Public examples and packaged docs must work without workspace-only files. Distinguish
API sketches in design notes from supported examples. Never advertise a design
sketch as an available function. When behavior changes, update rustdoc, examples,
tests and affected design notes together.

Do not state volatile details that carry no meaning for the reader. Exact counts
of tabs, pages, widgets, crates or test cases, and enumerations of every item in
a growing list, go stale on the next ordinary change and produce documentation
corrections instead of information. Describe what exists and what it
demonstrates; name individual items only where the specific item matters, and
point to the code or the gallery guide when a reader needs the current set.
Numbers that are part of a contract, such as documented capacities, limits,
supported versions and the MSRV, stay and must be exact.

Explicit control adornments are separate from bound caption content. Use
replaceable icon slots for control properties; reserve accumulating spans for
explicit RichText composition. The component API review applies this to Label,
Button, Link and Chip first; other families remain separately tracked.

Choice controls select option ownership at construction: `new` takes a fixed
list, `bound` follows a dynamic list and `for_parameter` follows its specification.
Do not add a second source through a builder. Validate dynamic lists during
update and preserve the last valid presentation when validation fails.
Availability uses restrictive Permission composition and stays separate from
parameter edit authority. Composite controls must document when descendants
observe redescription. Never publish signals from reconciliation or discard a
refused write; use the public event-phase notification contract when required.

## Component API conventions

Ordinary property builders replace; capability restrictions compose with
`Permission::and` and a same-key description replaces earlier restrictions.
Keep the explicit family exceptions in [contracts.md](docs/contracts.md), including
retained collection controllers, editor drafts and geometry observation. Do not
change authority, callback timing or cancellation merely for spelling symmetry.
Required state owners lead constructor arguments. Fixed, reactive and parameter
choice sources are exclusive constructor paths. Explicit control adornment slots
remain independent of captions; rich-content composition is a separate operation.

User-write callbacks report successful local writes or proposal submission, never
remote acceptance. Failures do not undo accepted state or replay notifications.
Component-owned selection/expansion changes report their origin after layout
settles through the public notification contract. Validate fixed descriptions
before adoption where their validity is locally knowable; validate reactive data
when observed and preserve the last valid presentation on failure.

The unpublished component review explicitly permits documented breaking migrations;
it does not waive separately versioned persistence or authorize publication.
Subsequent ordinary fixes still follow the compatibility policy below.

## Rust and data invariants

- Centralize dependency versions in root `[workspace.dependencies]`; members inherit
  them and select their own features/platform constraints. Published local libraries
  still require a version beside the workspace path; Cargo normalizes inheritance
  when packaging.
- Follow the toolchain/MSRV policy; use stable Rust and the selected edition.
  Use `Self` in impls, conventional conversion traits and focused public types.
- Give domain quantities and identities appropriate validated types with private
  invariants. Keep IDs distinct from indices, labels, display order and samples/
  ticks. Reordering or recreating an item must not retarget a stale event.
- Key tables of defaults or per-variant data (palettes, metrics, lookup tables)
  by their typed role or variant, never by array position. Check completeness and
  duplicates at compile time, for example with const evaluation plus an exhaustive
  match, so adding or reordering a variant cannot silently misassign values.
- Reject invalid external values, including non-finite numbers where forbidden.
  Use `TryFrom`/fallible constructors; clamp only for documented control-limit
  behavior. Keep presentation formatting separate from canonical values.
- Return typed errors, using `thiserror` where appropriate. Do not discard I/O,
  command, persistence or state-reconstruction failures. Avoid `unwrap`/`expect`
  in production paths. An explicit fallback must have defined semantics.
- Use `#[must_use]` for results/handles/builders whose discard is a mistake.
  Avoid global mutable state, implicit background executors and public shared
  mutexes as a default ownership model.
- Ordinary property builders replace earlier values. Capability restrictions
  use `Permission` and combine with `and`; same-key redescription replaces the
  description's permission rather than accumulating old restrictions. Document
  any family exception and cancellation on revocation. Keep parameter
  editability independent of application enablement.
- Deny unsafe code in ordinary modules. Discuss any new unsafe boundary with the
  user before introducing it; document safety invariants and test the exception.
  Dependency internals are not covered by our crate's unsafe lint.
- Version persisted schemas separately from crate versions. Validate closed
  schemas, required fields, identities and numeric ranges. Test round trips and
  rejection of invalid/unsupported versions; never reinterpret old data silently.
  Persist deterministic workspace state separately from application data.

## Scheduling and real-time boundaries

Keep physical pressed-key state separate from committed text and IME composition.
Support simultaneous keys without a single-key shortcut model. Classify OS repeats
and reset component-owned held actions on focus loss, hiding and unmount.

Keep inspection/diagnostic contracts independent of rendering and of any tooling
transport; MCP and other protocols belong in the application, not the library.
Capture owned snapshots on the UI thread; remote tooling must not hold live UI
handles. Hosts explicitly expose additional app state. Unknown sizes/styles stay
absent until measured. Event history is bounded and disabled by default, with
success/failure/cancellation distinguished; inspection must not schedule polling.
Do not claim a working cross-thread inspection path from headless DTOs alone.
Future inspection must also capture application windows, views and widgets using
their actual rendered frame and stable target identity. Whole-display capture is
a distinct OS service. Preserve scale, crop/clipping and frame/state correlation;
keep GPU readback/encoding outside time-critical callbacks and image bytes outside
event history. Implement capture APIs with real backends, not placeholder image methods.
See [inspection and capture contracts](docs/contracts.md).

Retain layout/text/drawing data for unchanged widgets. Binding changes invalidate
only their dependencies. No unconditional redraw loop, hidden-view polling or
whole-root rebuild merely because a live-data update occurred. Visibility must
stop unnecessary presentation work, and unmount must release subscriptions.

Time-critical producer callbacks must not allocate, lock, panic, log, perform I/O,
wake the OS event loop or destroy heap-owned data. Their endpoints need
preallocation, bounded work and explicit overflow/reclamation behavior. A
generic channel or atomics do not prove real-time safety. Keep callback-safe
endpoints separate from ordinary UI senders.

Telemetry can be lossy with explicit freshness/gap semantics. Committed edits and
terminal gesture outcomes must not silently disappear. Test full queues, removal,
shutdown and conflicting updates. State precisely which operations are measured
or proven bounded; do not claim zero CPU or cross-platform safety from compilation.

## Verification and review

Use the smallest gate covering the change. `.github/workflows/ci.yml` is the
authoritative workspace gate; when this table, the commands below or the workflow
change, update the others in the same change so they stay in step. Run only
checks that exist; never claim a check you did not run.

| Change | Required checks |
|---|---|
| Markdown only | Author review, links and whitespace; review consistency with existing decisions. |
| Rustdoc/public examples | Targeted doctests, example compilation and warning-free rustdoc. |
| Package-local ordinary implementation | Formatting, targeted Clippy and package tests, including affected doctests. |
| Public API, identity, scheduling, concurrency, persistence, real-time boundary or cross-package change | Workspace checks below, focused contract tests and semantic review before merge. |
| Dependencies, features, platform integration or release | Workspace checks, MSRV, supported feature configurations, packaging/consumer checks and affected native platform checks. |

Workspace gate, as CI runs it (`--locked` keeps `Cargo.lock` authoritative):

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
```

A change to the live-data protocols in `pert-ui-core`'s `live` module also
runs their `loom` models, which CI runs on Linux after the release-profile
tests: `RUSTFLAGS="--cfg loom" cargo test --locked -p pert-ui-core --test loom
--release`. The models are the memory-ordering evidence; a stress test alone
is not.

The dependency/feature/platform/release row additionally requires what CI runs
after that: release-profile tests (`cargo test --locked --release --workspace`),
the `pert-ui` no-default-features configuration (Clippy, tests and rustdoc with
`-p pert-ui --no-default-features`), the image decoder configurations (Clippy,
tests and rustdoc with `-p pert-ui --features image-formats`, Clippy and tests
with `-p pert-ui --no-default-features --features png`, and Clippy and tests with
`-p pert-ui --features images`, the decoder-free API), the SVG configurations
(Clippy, tests and rustdoc with `-p pert-ui --features svg`, and Clippy and tests
with `-p pert-ui --no-default-features --features svg`), the serde
configurations (Clippy, tests and rustdoc with `-p pert-ui --features serde`,
Clippy and tests with `-p pert-ui --no-default-features --features serde`, and
Clippy and tests with `-p pert-ui-core --features serde`, whose own
serialization tests the facade runs do not execute), the CPU renderer
configuration (Clippy, tests and rustdoc with `-p pert-ui --features cpu`), the
headless gallery
(`cargo run --locked -p pert-ui-gallery -- --headless`) and library packaging
(`cargo package --locked --workspace --exclude pert-ui-gallery --exclude pert-ui-native-checks`).

CI tests the declared MSRV, which `rust-toolchain.toml` pins for local work, on
Linux and stable Rust on Linux, macOS and Windows. Define supported feature
configurations when features are added; include default and relevant non-default
builds. Do not impose an incoherent `--all-features` combination. Run
release-profile boundary tests when debug/release behavior can differ. GUI/GPU
execution needs native evidence on the platform matrix; headless CI alone does
not establish it. `cargo run --locked -p pert-ui-native-checks` runs the opt-in
native checks one process each on an unlocked desktop session with a hardware
GPU; record the platform it ran on with its result.

CI runs `cargo clean` between the base, image/SVG, serde/CPU and packaging
groups to reclaim build artifacts on hosted runners. These cleanup steps do not
replace any check or belong to the local verification commands above.

For high-risk changes, obtain an independent human or fresh-context review before
merge/release. Prefer a different model family when an AI reviewer is used. Reviewers
inspect the declared scope without authoring fixes or delegating the review.
If a reviewer or platform is unavailable, record the missing evidence explicitly;
continue reversible implementation work without claiming that gate passed.
Routine edits receive author review. Recheck repaired risks without repeatedly
rerunning unrelated expensive checks.

## Reporting deviations and improvements

Every agent working in this repository is also its reviewer. When you see code,
documentation, tests or configuration that breaks these instructions or the
linked design documents, or you see a concrete improvement, say so in your reply.
Silence is not acceptable, and neither is quietly widening the task to fix it.

- Findings inside the current task's scope are fixed as part of the task and
  mentioned in the reply.
- Findings outside the scope are reported to the user and recorded in
  [TODO.md](TODO.md) in the repository root; create the file if it is missing.
  Do not fix them without instruction: an unrelated change makes the task harder
  to review, and prioritization belongs to the user.
- Record verified findings, not hunches. A lint, `rg` or `ast-grep` hit is a
  lead; read the code and the governing rule before recording it. Point to the
  file, item or document, name the rule or design decision involved, describe
  the observed deviation or the proposed improvement and its benefit, and
  suggest a fix.
- Before adding an entry, search TODO.md for an existing one and update it
  instead of duplicating. When a change resolves an entry, remove it in the same
  change; the changelog and Git history record what was done.
- Read the TODO.md entries touching the files you are about to change and
  mention the relevant ones to the user. Resolving one is in scope only when the
  user includes it or it is necessary for the task.

TODO.md is a flat list under a `# TODO` heading, one item per finding in the
form below, newest last. It is not a changelog, a feature backlog, a place for
design proposals (those belong in `docs/`) or a substitute for fixing in-scope
work. Do not leave `TODO`/`FIXME` comments in shipped code as the only record of
a known deviation; either resolve them or add the TODO.md entry.

```markdown
- [ ] 2026-09-15 `crates/pert-ui-runtime/src/example.rs` `Example::new` — Rust
  and data invariants: clamps a non-finite input instead of rejecting it. Return
  an error from a fallible constructor and add a rejection test.
```

## Publishing and compatibility

Build for public consumption from the first working slice. Library package names,
license, owner/repository metadata and asset attribution must be settled before
release. Do not copy a consumer's author, license or release version implicitly.
The sample app is `publish = false`; every runtime dependency of a published
library must itself be registry-resolvable. Keep version requirements alongside
local paths for workspace library dependencies intended for publication.

For each library package, inspect `cargo package --list`, verify packaging, and
run `cargo publish --dry-run` when its dependency versions are registry-resolvable.
Do not use `--no-verify` to claim success. Before the first interdependent release,
exercise the packaged artifacts with a temporary registry or an isolated consumer
using only extracted packages, then publish dependencies in order when authorized.
Distinguish this local proof from an actual crates.io dry run.

The external consumer must compile documented examples and a custom widget without
workspace patches, sibling paths or private imports. Verify that fonts, shaders,
license files and README content are included where needed, and that docs.rs can
build without opening a display or requiring a GPU. Keep build scripts deterministic
and avoid build-time downloads or reliance on untracked generated files.

Use the compatibility policy in [contracts.md](docs/contracts.md): documented minor-version breaks
before 1.0, compatible patch releases and separately versioned persisted formats.
While the library remains unpublished, the component API review may make
breaking changes without compatibility aliases or a version bump solely for
unpublished compatibility. Record replacement usage and update repository
consumers, examples and documentation in the same change; persisted schema
rules still apply. The version compatibility rules govern published versions.
Keep a changelog and review exported API changes, including defaults and errors.
Prefer hiding backend churn behind the facade. An intentional breaking change
must be explicit in the task/release scope; do not silently widen an ordinary fix.
Public release requires completed review, package verification, relevant native
checks and an explicit publication instruction. Preparing packages is not uploading
them, and a request to prepare this repository is not a request to publish it.

## Working practice

`ast-grep` (verified locally with version 0.45.3, installed through npm) is the
tool for structural Rust searches and scoped rewrites. It may be missing in other
environments: run `ast-grep --version` before relying on it, and if it is absent
say so and fall back to `rg` rather than presenting a text search as a
structural result. Use `rg` for file discovery and literal text; use `ast-grep`
when matching code structure across whitespace or nested expressions. Prefer the
full command name to the potentially ambiguous `sg` alias.

```sh
ast-grep run --lang rust --pattern 'NormalizedValue::try_from($VALUE)' crates apps
ast-grep run --lang rust --pattern '$RECEIVER.checked_add($$$ARGS)' --json=compact crates
ast-grep run --lang rust --pattern '$VALUE.unwrap()' --files-with-matches crates apps
```

Single-quote patterns so the shell preserves metavariables. `$VALUE` matches one
syntax node; `$$$ARGS` matches zero or more. Scope searches to relevant paths and
use JSON for programmatic inspection. For ambiguous patterns inspect
`--debug-query=ast`; consult `ast-grep run --help` for installed options.
Preview a rewrite with `--rewrite` before applying it with `--update-all`, inspect
the result, and run the checks appropriate to the code change. AST matching is
syntactic: it does not establish Rust type resolution, macro-expanded behavior or
safety. No project configuration is needed for these searches; introduce reusable
`scan` rules with rule tests only when a repeated check warrants them.
See the official [pattern guide](https://ast-grep.github.io/guide/pattern-syntax.html)
and [quick start](https://ast-grep.github.io/guide/quick-start.html).

Use `rg` for discovery and text searches. Inspect existing work before edits and
preserve unrelated changes. When Git metadata is available, inspect status and
staged/unstaged diffs before committing, run `git diff --check`, and stage only
intended paths. If Git metadata is absent, report that limitation and review files
directly; do not claim a clean Git diff or invent a commit.

Keep changes small enough to review and tie tests to behavior or contracts.
No consumer-specific release scripts, evidence gates, tooling endpoints or
widget conventions apply here. Updating this file does not authorize changes
to a sibling checkout. This file is the single authoritative instruction
file in this repository; `CLAUDE.md` only contains `@AGENTS.md` so that Claude
Code imports it. Put instructions here, never there.
