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
base controls or data model. Both the headless and the native gallery use the
public facade. The library ships no MCP server: applications build their own
tooling transport on the public inspection contract. No library has been
published. Do not describe proposed APIs, platforms or performance targets as
verified; [CHANGELOG.md](CHANGELOG.md) and the design notes record what exists.

Read the relevant design before changing it:

- [Architecture and direction](docs/architecture.md)
- [State and interaction contracts](docs/contracts.md)
- [Implementation notes](docs/internals.md)
- [Rendering and platform contract](docs/rendering-platform.md)
- [Verification and evidence](docs/verification.md)
- [Application manual](docs/manual.md)

Earlier consumer inventories are historical research. Their architecture and
sibling-repository paths are not library requirements. AccessKit, legacy OS
support, X11, browser rendering and plugin hosting are outside the initial
scope. The CPU renderer (`pert-ui-cpu`, the facade's `cpu` feature) is in
scope: it renders scenes and serves captures without a GPU. Native windows
present its frames through softbuffer only when opted in, with
`App::software_rendering` or the `desktop-cpu` feature, which builds `App`
without wgpu; there is no automatic fallback.

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
`missing_debug_implementations`, `rustdoc::broken_intra_doc_links`,
`rustdoc::private_intra_doc_links`, `clippy::missing_errors_doc`,
`clippy::missing_panics_doc`, `clippy::return_self_not_must_use` and
`clippy::fn_params_excessive_bools`; every member inherits them with `[lints] workspace = true`. New packages must inherit
them, and no module may relax them without discussing it with the user. Treat
rustdoc warnings as errors in documentation builds. Lints detect missing
text/links, not semantic completeness; review the generated documentation from a
first-time consumer's perspective.

Public examples and packaged docs must work without workspace-only files. Distinguish
API sketches in design notes from supported examples. Never advertise a design
sketch as an available function. When behavior changes, update rustdoc, examples,
tests and affected design notes together.

Do not state volatile details that carry no meaning for the reader: exact
counts of tabs, pages, widgets, crates or test cases, and enumerations of a
growing list, go stale on the next ordinary change. Describe what exists and
what it demonstrates, name an item only where the specific item matters, and
point to the code or the gallery guide for the current set. Numbers that are
part of a contract, such as documented capacities, limits, supported versions
and the MSRV, stay and must be exact.

## Component API conventions

[contracts.md](docs/contracts.md) holds the full rules and the family
exceptions (choice sources, control adornments, component redescription,
permissions, user-write callbacks and controlled controls); read the section
before changing a component family. The conventions every change keeps:

- Required state owners lead constructor arguments. Choice controls select
  option ownership at construction: `new` takes a fixed list, `bound` follows
  a dynamic list and `for_parameter` follows its specification. Do not add a
  second source through a builder.
- Ordinary property builders replace earlier values. Capability restrictions
  use `Permission` and compose with `Permission::and`; a same-key description
  replaces the description's permission rather than accumulating old
  restrictions. Availability stays separate from parameter edit authority, and
  parameter editability independent of application enablement. Keep the
  explicit family exceptions, including retained collection controllers,
  editor drafts and geometry observation, and document any new one and its
  cancellation on revocation. Do not change authority, callback timing or
  cancellation merely for spelling symmetry.
- Explicit control adornments use replaceable icon slots, independent of bound
  caption content; accumulating spans are reserved for explicit RichText
  composition.
- Validate fixed descriptions before adoption where their validity is locally
  knowable. A value a description carries from application data, such as a
  controlled control's value, is refused at adoption with the last valid
  presentation kept. Reactive data, including dynamic choice lists, is
  validated when observed during update, preserving the last valid
  presentation on failure.
- Never publish signals from reconciliation, except for the lifecycle cleanup
  contracts.md names, or discard a refused write; use the public event-phase
  notification contract when required. Composite controls document when
  descendants observe redescription. Component-owned selection/expansion
  changes report their origin after layout settles through the public
  notification contract.
- User-write callbacks report successful local writes, proposal submission or
  a controlled control's proposal, never remote acceptance. Failures do not
  undo accepted state or replay notifications.
- An addition that offers a new way to do something already possible names,
  in its change rationale, the way it makes advanced or retires, or says why
  none can be.

## Rust and data invariants

- Centralize dependency versions in root `[workspace.dependencies]`; members inherit
  them and select their own features/platform constraints. Published local libraries
  still require a version beside the workspace path; Cargo normalizes inheritance
  when packaging.
- Follow the toolchain/MSRV policy; use stable Rust and the selected edition.
  The one exception is `api/generate.sh`, which sets `RUSTC_BOOTSTRAP=1` on
  its rustdoc JSON command to extract the API listings; nothing else may use
  it.
  Use `Self` in impls, conventional conversion traits and focused public types.
- Give domain quantities and identities appropriate validated types with private
  invariants. Keep IDs distinct from indices, labels, display order and samples/
  ticks. Reordering or recreating an item must not retarget a stale event.
- Key tables of defaults or per-variant data (palettes, metrics, lookup tables)
  by their typed role or variant, never by array position. Check completeness and
  duplicates at compile time, for example with const evaluation plus an exhaustive
  match, so adding or reordering a variant cannot silently misassign values.
- Reject invalid external values, including non-finite numbers where forbidden.
  Use `TryFrom`/fallible constructors; clamp only for documented control or
  layout limits, or a documented presentation mapping that is total over its
  input (`ProgressStatus::of`); reject, never coerce, an invalid explicit text
  offset, identity or stored preference value. Keep presentation formatting
  separate from canonical values.
- Return typed errors, using `thiserror` where appropriate. Do not discard I/O,
  command, persistence or state-reconstruction failures. Avoid `unwrap`/`expect`
  in production paths. An explicit fallback must have defined semantics.
- Use `#[must_use]` for results/handles/builders whose discard is a mistake.
  Avoid global mutable state, implicit background executors and public shared
  mutexes as a default ownership model.
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

Keep inspection and diagnostic contracts independent of rendering and of any
tooling transport; MCP and other protocols belong in the application. Capture
owned snapshots on the UI thread; remote tooling must not hold live UI handles,
and hosts expose additional app state explicitly. Unknown sizes/styles stay
absent until measured. Event history is bounded and disabled by default, with
success/failure/cancellation distinguished; inspection must not schedule
polling. Do not claim a working cross-thread inspection path from headless DTOs
alone. Capture uses the actual rendered frame and stable target identity,
preserves scale, crop/clipping and frame/state correlation, and is implemented
with real backends, not placeholder image methods. GPU readback/encoding stays
outside time-critical callbacks and image bytes outside event history;
whole-display capture is a distinct OS service. See [inspection and
diagnostics](docs/contracts.md#inspection-and-diagnostics).

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
| Rustdoc/public examples | Targeted doctests, example compilation, the affected example's headless session and warning-free rustdoc. |
| Package-local ordinary implementation | Formatting, targeted Clippy and package tests, including affected doctests, and the diagnostics snapshots when a compiler message they cover changes. |
| Public API, identity, scheduling, concurrency, persistence, real-time boundary or cross-package change | Workspace checks below, the API listing gate, focused contract tests and semantic review before merge. |
| Dependencies, features, platform integration or release | Workspace checks, MSRV, supported feature configurations, the API listing gate, packaging/consumer checks and affected native platform checks. |

Workspace gate, as CI runs it (`--locked` keeps `Cargo.lock` authoritative):

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p pert-ui --features png,svg,cpu --example live_mixer
cargo run --locked -p pert-ui --features png,svg,cpu --example dynamic_documents
cargo run --locked -p pert-ui --features png,svg,cpu --example diagram_editor
cargo run --locked -p pert-ui --features png,svg,cpu --example log_explorer -- --lines 10000
cargo run --locked -p pert-ui --features png,svg,cpu --example inventory -- --items 2000
cargo test --locked --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo test --locked -p pert-ui-diagnostics -- --ignored
```

The `cargo run` lines are the packaged examples' headless sessions, with the
features the workspace build unifies so its examples are reused; a new
headless example joins them here and in the workflow. The last line compares
the compiler messages pert-ui shapes with diagnostic attributes against the
`trybuild` snapshots in `tests/diagnostics/ui/`. It is ignored in ordinary
runs, and CI's `diagnostics` job runs it with the toolchain
`rust-toolchain.toml` pins. A changed message or a toolchain update
regenerates the snapshots with `TRYBUILD=overwrite` and the same command, and
the `.stderr` diff is reviewed like the `api/` listing.
[verification.md](docs/verification.md#workspace-gate) explains what a
snapshot promises and why a `compile_fail` doctest proves a restriction but
not a message.

The public API listings under `api/` must match the code. A change that adds,
removes or changes a public item, a dependency or the toolchain runs the API
listing gate on x86_64 Linux and commits the result; CI's separate
`api-listing` job runs the same commands and fails on any difference:

```sh
cargo install --locked cargo-public-api@0.52.0  # the version api/generate.sh names
api/generate.sh
git status --porcelain -- api/                  # must print nothing once committed
```

The diff of `api/` is part of the review. The CHANGELOG stays concise, one
section per milestone: it names added and removed public items and every
breaking change with its one-line replacement, and summarizes other changes
by what a user notices; churn from a dependency or toolchain update is
described as such. It records no plan, review or evidence narrative.

A change to the live-data protocols in `pert-ui-core`'s `live` module also
runs their `loom` models: `RUSTFLAGS="--cfg loom" cargo test --locked -p
pert-ui-core --test loom --release`. The models are the memory-ordering
evidence; a stress test alone is not.

The dependency/feature/platform/release row additionally runs every step the
`headless` job in `.github/workflows/ci.yml` runs beyond the workspace gate,
as the workflow writes them: release-profile tests, the supported feature
configurations of `pert-ui` and `pert-ui-core` (Clippy, tests and, where the
workflow runs it, rustdoc for each), the headless gallery and library
packaging. Define supported feature configurations, default and relevant
non-default builds, in the workflow when features are added; do not impose an
incoherent `--all-features` combination.

CI tests the declared MSRV, which `rust-toolchain.toml` pins for local work, on
Linux and stable Rust on Linux, macOS and Windows. Run release-profile boundary
tests when debug/release behavior can differ. GUI/GPU execution needs native
evidence on the platform matrix; headless CI alone does not establish it.
`cargo run --locked -p pert-ui-native-checks` runs the opt-in native checks one
process each on an unlocked desktop session with a hardware GPU; record the
platform it ran on with its result.

Every top-level file or `main.rs` directory in a package's `tests/` is a
separate test binary that links the package's whole dependency graph once per
feature configuration. Add a facade test as a module of the area binary it
covers (`crates/pert-ui/tests/<area>/main.rs`, sharing `tests/support`), and
introduce a new area binary only for a distinct area.

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
linked design documents, or a concrete improvement, say so in your reply.
Silence is not acceptable, and neither is quietly widening the task to fix it.

- Fix findings inside the current task's scope as part of the task and mention
  them in the reply.
- Report findings outside the scope to the user and record them in
  [TODO.md](TODO.md), creating it if it is missing. Do not fix them without
  instruction: an unrelated change makes the task harder to review, and
  prioritization belongs to the user.
- Record verified findings, not hunches: a lint, `rg` or `ast-grep` hit is a
  lead until you have read the code and the governing rule. Name the file,
  item or document, the rule or design decision involved, the deviation or
  improvement and its benefit, and a suggested fix.
- Search TODO.md before adding an entry and update an existing one instead of
  duplicating. A change that resolves an entry removes it; the changelog and
  Git history record what was done.
- Mention the TODO.md entries touching the files you change. Resolving one is
  in scope only when the user includes it or the task needs it.

TODO.md is a flat list under a `# TODO` heading, one finding per item in the
form below, newest last. It is not a changelog, a feature backlog, a place for
design proposals (those belong in `docs/`) or a substitute for fixing in-scope
work. Do not leave a `TODO`/`FIXME` comment in shipped code as the only record
of a known deviation.

```markdown
- [ ] 2026-09-15 `crates/pert-ui-runtime/src/example.rs` `Example::new` — Rust
  and data invariants: clamps a non-finite input instead of rejecting it. Return
  an error from a fallible constructor and add a rejection test.
```

## Publishing and compatibility

Build for public consumption from the first working slice. Settle library
package names, license, owner/repository metadata and asset attribution before
release, and never copy a consumer's author, license or release version. The
sample app is `publish = false`; every runtime dependency of a published
library must be registry-resolvable, with a version requirement beside each
local path.

For each library package, inspect `cargo package --list`, verify packaging, and
run `cargo publish --dry-run` when its dependency versions are registry-resolvable;
never use `--no-verify` to claim success. Before the first interdependent
release, build an isolated consumer, or use a temporary registry, from the
extracted packages only, and distinguish that local proof from a crates.io dry
run. It compiles the documented examples and a custom widget without workspace
patches, sibling paths or private imports. Verify that fonts, shaders, license
files and README content are included where needed, and that docs.rs can build
without a display or GPU. Keep build scripts deterministic, without build-time
downloads or untracked generated files. Publish dependencies in order only when
authorized.

While the library is unpublished, breaking changes to public APIs and code
structure are welcome wherever they improve the API, its usability or the code:
the purpose of the component and usability reviews is to change what works
poorly, not to preserve it. Make them without compatibility aliases or a
version bump solely for unpublished compatibility; record the replacement
usage and migrate repository consumers, examples and documentation in the same
change. This does not waive separately versioned persistence or authorize
publication. An intentional breaking change must be explicit in the task or
release scope; do not silently widen an ordinary fix. Published versions follow
the [compatibility policy](docs/contracts.md#public-compatibility-policy). Keep
a changelog and review exported API changes, including defaults and errors.
Prefer hiding backend churn behind the facade.

Public release requires completed review, package verification, relevant native
checks and an explicit publication instruction. Preparing packages is not uploading
them, and a request to prepare this repository is not a request to publish it.

## Working practice

`ast-grep` is the tool for structural Rust searches and scoped rewrites
(verified locally with version 0.45.3, installed through npm). Run
`ast-grep --version` before relying on it; if it is absent, say so and fall back
to `rg` rather than presenting a text search as a structural result. Use `rg`
for file discovery and literal text. Use the full command name, not the `sg`
alias, and single-quote patterns so the shell preserves metavariables:

```sh
ast-grep run --lang rust --pattern '$VALUE.unwrap()' --files-with-matches crates apps
```

`$VALUE` matches one syntax node and `$$$ARGS` zero or more; `--json=compact`
gives programmatic output and `--debug-query=ast` explains an ambiguous
pattern. Preview a rewrite with `--rewrite` before applying it with
`--update-all`, inspect the result and run the checks the code change needs.
AST matching is syntactic: it does not establish Rust type resolution,
macro-expanded behavior or safety. Introduce reusable `scan` rules with rule
tests only when a repeated check warrants them.

Inspect existing work before edits and preserve unrelated changes. When Git
metadata is available, inspect status and staged/unstaged diffs before
committing, run `git diff --check`, and stage only intended paths. If Git
metadata is absent, report that limitation and review files directly; do not
claim a clean Git diff or invent a commit.

Keep changes small enough to review and tie tests to behavior or contracts.
No consumer-specific release scripts, evidence gates, tooling endpoints or
widget conventions apply here. Updating this file does not authorize changes
to a sibling checkout. This file is the single authoritative instruction
file in this repository; `CLAUDE.md` only contains `@AGENTS.md` so that Claude
Code imports it. Put instructions here, never there.
