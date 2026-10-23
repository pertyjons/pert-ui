# API audit fix plan

Slices 5 and 6 of [the developer-usability plan](../../developer-usability.md),
proposed 2026-10-22 after the joint checkpoint. The maintainer chose to fix
every finding of [the API audit](findings.md), approved every [policy
decision](decisions.md) as recommended, and extended the scope to every
finding of round 1 (the plan's "Round 1 findings" table, F1 to F34),
including those whose destination was app-author slice 9 or the follow-ups.
The [lint survey](lints/survey.md) and the [diagnostics
survey](diagnostics/survey.md) recommendations are adopted.

The plan closes 127 audit findings, 53 decisions and 36 round 1 findings,
and two diagnostics-survey cases that no finding carries (`S:m06`,
`S:l06`). Each appears exactly once, as closed by one part; the [coverage
table](#coverage) at the end lists them sorted by ID. A decision is written
`D:` followed by the IDs of its heading in decisions.md; `S:` names a case
of the diagnostics survey. A finding a decision
answers follows the decision; a decision that is "not adopted" closes its
finding with no change (G4-4). An F-number an audit finding already covers is
closed in the same part, and its line says so.

Path prefixes: `R/` = `crates/pert-ui-runtime/src/`, `C/` (or `core/`) =
`crates/pert-ui-core/src/`, `F/` = `crates/pert-ui/src/`, `W/` =
`crates/pert-ui-wgpu/src/`. Other source paths without a prefix are under
`R/`. Line numbers are those the audit and the triage cite; `crates/` has not
changed since the audited commit d4ba108.

## Progress

One entry per part when it closes: its commits, the checks run, the review
and anything left to a later part.

- **5a (0.3.53), done 2026-10-22:** 5ee6eb6 and its review fix.
  - **Checks run:** `git diff --check`; links in contracts.md, AGENTS.md and
    manual.md; `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace
    --no-deps` and `-p pert-ui --features serde`; the targeted runtime and
    core doctests of every item whose rustdoc changed; `api/generate.sh`,
    with no difference.
  - **Review:** Codex found the no-argument rule too broad
    (`ParameterSpec::read_only()`), the scoped-access rule missing
    `SeriesSource::with_window`'s `Result`, and no outcome record. All three
    were fixed and the recheck approved them.
  - **Left to a later part:** the clamp rule's two known departures,
    `TextSelection::new` (G1-11) and the table width restore (V3-2), are
    fixed in 6d2.
- **5b (0.3.54), done 2026-10-22:** 9942634 and its review fix.
  - **Checks run:** `cargo fmt --all --check`; `cargo clippy --locked -p
    pert-ui-runtime --all-targets -- -D warnings`; every doctest of
    pert-ui-core, pert-ui-runtime, pert-ui and pert-ui-wgpu (including
    `MultiSelect::filter`'s new one); `RUSTDOCFLAGS="-D warnings" cargo doc
    --locked --workspace --no-deps` and `-p pert-ui --features serde`;
    `api/generate.sh`, with no difference; `git diff --check`.
  - **F32's probe:** rerun in a throwaway crate built into
    `target/triage-probes`. A signal write inside `cancel_input` returned
    `Ok` for `set_visible(false)`, a `View::visible_signal` hide inside a
    flush, `unmount`, `Show` removal inside a flush and `shutdown`;
    `ColorPicker::cancel_input`'s comment was corrected to match.
  - **Beyond the plan's lines:**
    - the same category-only `# Errors` wording, found during the part on
      `ContextMenuOpener::open`, `ToolWindowController::{expand,
      set_names, is_expanded}` and `RemoteHost::retry_submission`, was fixed
      here;
    - `Component::focus_lost`'s Escape sentence was corrected with F18;
    - `Runtime::set_viewport` also names `Geometry(NonFinite)`.
  - **Review:** Codex found five points:
    - Escape's routing was overstated: a drag, shown help, editor text and
      surfaces take it before the context cleanup;
    - `Runtime::focus` omitted forwarded `focus_within_changed` and
      `reveal` failures;
    - the prelude pages made a general compiler claim;
    - no outcome record;
    - a parameter-rename count in the CHANGELOG.

    All five were fixed.
  - **Recheck:** the first recheck found that the Escape text now
    overpromised the opposite way. A cancelled drag releases its source's
    capture, and a closed surface hides what it showed, calling
    `cancel_input`. The text now says an earlier consumer skips only the
    focused component's ordinary cleanup, while its own lifecycle cleans up
    as usual. The second recheck approved 5b.
  - **Left to a later part:** tracing `Runtime::focus` found a dropped error,
    added to 6d2 as `N:focus-reveal`.
- **5c (0.3.55), done 2026-10-22:** c03e6eb and its review fix.
  - **Checks run, after the review fix:**
    - `cargo fmt --all --check`;
    - `cargo clippy --locked -p pert-ui --all-targets -- -D warnings`;
    - every facade doctest, plus the guide's with `--features
      png,svg,cpu` for the CPU screenshot example;
    - runtime doctests of virtual_list, signal and table;
    - `cargo test --locked -p pert-ui --test foundation readme`, with the
      new tests for the getting-started requirement, the Rust version and
      every guide dependency line;
    - `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps`
      and `-p pert-ui --features serde`;
    - the `inventory` and `log_explorer` headless sessions;
    - `git diff --check`.

    `api/generate.sh` showed no difference before the review fix, which
    changed only doc comments and Markdown.
  - **Beyond the plan's lines:**
    - the appearance and diagnostics guides' `version = "…"` placeholders,
      the same defect as F1, were fixed here;
    - the recipe states `InlineText`'s current press behavior, which 6f2's
      list-row mode (F27) must update.
  - **Review:** Codex found that the per-row binding cost omitted scrolling.
    - The list rebuilds every mounted row whenever the mounted rows or their
      positions change (`virtual_list.rs:1839`).
    - A probe confirmed that a reveal discarded a per-row draft.
    - `ListItems::build`, `Signal::sibling`, `TableItems::cell`, the recipe
      and the CHANGELOG were corrected.

    The recheck approved 5c.
- **5d (0.3.56), done 2026-10-22:** 3c2e63a and its record fix. The item examples of pert-ui-runtime, pert-ui-core and
  pert-ui-cpu import `pert_ui::…` through hidden alias lines, so the
  facade's rendered pages show the facade's paths.
  - **Checks run after the last edit:**
    - `cargo fmt --all --check`;
    - `cargo test --locked --workspace --doc`;
    - `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps`;
    - the twelve `compile_fail` blocks, switched temporarily to `no_run`,
      which failed only for their intended reason;
    - a script over every rendered example on the facade's pages, which
      found no `pert_ui_runtime::`, `pert_ui_core::` or `pert_ui_cpu::` and
      no rendered hidden line;
    - `api/generate.sh`, with no difference;
    - `git diff --check`.
  - **Beyond the plan's lines:**
    - the three `CpuRenderer` examples, the same defect, use a hidden
      module that mirrors the facade's `cpu` path;
    - the three crate READMEs say item examples are written with the
      facade's paths and give the direct spelling.
  - **Review:** Codex found no implementation defect. Reversing the alias
    and path substitutions reproduces the previous sources exactly. It found
    three record defects:
    - this entry had swallowed 5b's deferred item;
    - the dropped `Runtime::focus` error had no TODO.md entry;
    - the commit and review were unrecorded.

    All three were fixed, and the recheck approved 5d.
- **6a1 (0.3.57), done 2026-10-22:** f66a143. Constructor order, builder and callback shapes, limit
  constants, `Color::from_linear`, `SplitRule::Ratio(NormalizedValue)`,
  `bind_<property>` and `SizeLimits::exact`. Every repository consumer was
  migrated.
  - **Checks run after the last edit (the public-API row):**
    - `cargo fmt --all --check`;
    - `cargo clippy --locked --workspace --all-targets -- -D warnings`;
    - `cargo test --locked --workspace`;
    - the five headless example sessions;
    - `cargo test --locked --workspace --doc`;
    - `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps`;
    - `api/generate.sh`, whose diff is the plan's expected list;
    - `git diff --check`.
  - **Beyond the plan's lines:** `WorkspaceError::InvalidName`'s rustdoc
    names the empty map key it also reports.
  - **Left to a later part:** the new `ExpansionChange` is at the crate root,
    and 6e adds it to the prelude beside `SelectionChange`.
  - **Review:** Codex found no implementation defect across the part's
    eleven audit findings, F33 and six decisions, and only this record
    incomplete, which this entry completes.
- **6a2 (0.3.58), done 2026-10-22:** 010e475 and its review fix. Role names (`ContextMenuController`, `ToastController`,
  `FormFields`, `ListDropPosition`), the owned-versus-reactive read rule and
  its renames, unit-bearing names and types (`PhysicalExtent` for the wgpu
  renderer, `Point` origins, `*_byte` offsets, `Length` widths), enums for
  boolean flags (`FocusDirection`, `SelectionExtension`, `Expansion`,
  `WindowCloseAction` in `NativeCloseAction::Defer`), `TaskStatus::Succeeded`
  and `PendingJump::receipt`. Every repository consumer was migrated.
  - **Checks run after the last edit (the dependency, feature, platform and
    release row):**
    - `cargo fmt --all --check`;
    - `cargo clippy --locked --workspace --all-targets -- -D warnings`;
    - `cargo test --locked --workspace` and `--release`;
    - the five headless example sessions and the headless gallery;
    - `cargo test --locked --workspace --doc`;
    - warning-free workspace rustdoc;
    - Clippy, tests and rustdoc for every `pert-ui` feature configuration
      CI checks, and `pert-ui-core --features serde`;
    - every run on the pinned toolchain, which is the MSRV;
    - `api/generate.sh`, whose diff is the plan's expected list;
    - the native checks on Linux (Fedora 44, GNOME/Wayland, Intel UHD 770
      with Mesa 26.2.4 on Vulkan), without the clipboard check, recorded in
      `docs/verification.md`. Windows and macOS evidence is missing;
    - `git diff --check`;
    - library packaging after the commit;
    - the packaged-consumer check, which had five parts:
      - extract the five 0.3.58 archives outside the checkout and patch them
        in for each other;
      - check each `cargo package --list` against its archive, including
        fonts, shaders, licenses and guides;
      - run the extracted facade's doctests, including the getting-started
        parameter, persistence and custom-widget examples;
      - build its native `getting_started` example;
      - build a copy of the round 2 fixture `task4-app` and a consumer with
        a custom widget, whose two tests pass.

      It is local packaged-consumer evidence, not a crates.io dry run.
  - **Beyond the plan's lines:** `ContextMenu::opener()` became
    `controller()`, as `FloatingWindow::controller` is named, and
    `ToastHost::new`'s parameter is `controller`.
  - **Left to a later part:** 6e considers `FocusDirection`, which a custom
    `tab_navigation` names, for the prelude.
  - **Review:** Codex found three points, all fixed:
    - the length rule overstated the code, and was reworded to separate
      property inputs, validated geometry constructors and reported
      geometry;
    - the `opener()` accessor rename lay beyond the plan's lines, and was
      recorded under G3-13 for the maintainer;
    - the packaged-consumer check and this record were missing.

    The clipboard native check stays an explicit gap: it takes over the
    session clipboard.
    The recheck approved 6a2.
- **6a3 (0.3.59), done 2026-10-22:** fec105d and its review fix. `Hash`, `Eq`, `Ord`, `Display`, `From` and `AsRef`
  impls, `#[non_exhaustive]` on the twelve runtime-built types, and `!Send`
  for `Divider` and `Svg`.
  - **Checks run after the last edit (the public-API row plus loom):**
    - `cargo fmt --all --check`;
    - `cargo clippy --locked --workspace --all-targets -- -D warnings`;
    - `cargo test --locked --workspace`, including the new
      `foundation::standard_traits` contract tests and the serialization
      tests with `--features serde`;
    - the five headless example sessions;
    - `cargo test --locked --workspace --doc`, with both new `compile_fail`
      doctests checked for their intended error;
    - warning-free workspace rustdoc;
    - the loom models;
    - `api/generate.sh`, whose diff is the plan's expected list;
    - `git diff --check`.
  - **Deviations:**
    - the contracts' `From` sentence covers validated quantities only;
      numeric identities and counters have no raw-number `From`;
    - `PartialOrd` is written by hand where `Ord` is added, so the two
      agree.
  - **Review:** Codex asked to narrow the identity sentence, which wrongly
    implied identities read only through `get()` (`ChoiceId` has `as_str()`
    and `AsRef<str>`), and to complete this record. Both were done; the
    implementation otherwise matched 6a3.
    The recheck approved 6a3.
- **6b1 (0.3.60), done 2026-10-22:** 9f755b6 and its review fix. `missing_panics_doc`, `return_self_not_must_use` and
  `fn_params_excessive_bools` are denied, with one `#[expect]` on
  `ThemeMetrics::for_density`. `#[must_use]` now follows the receipt rule:
  - `CommandId`, `HeldInputId` and `ToastReceipt` lose it;
  - owners whose drop tears down state gain it with reasons;
  - the signal creators gain it;
  - the remaining receipts carry reasons.
  - **Checks run after the last edit:**
    - `cargo fmt --all --check`;
    - workspace Clippy and Clippy in every `pert-ui` feature configuration
      CI checks, plus `pert-ui-core --features serde`;
    - `cargo test --locked --workspace`;
    - the five headless example sessions (diagram_editor rerun after the
      follow-up);
    - `cargo test --locked --workspace --doc`;
    - warning-free workspace rustdoc;
    - `api/generate.sh`, with no difference;
    - `git diff --check`.
  - **Beyond the plan's lines:**
    - the `let _ =` workarounds the old `#[must_use]` forced, about fifty
      across the examples, gallery and tests, are plain statements;
    - `CommandReceipt`, `PresentationReceipt` and `RevealReceipt` carry
      reasons that follow the receipt rule.
  - **Left to a later part:** the round 2 fixture's `let _ =
    ui.register_command` (`plans/usability/task4-app`) waits for the kit
    fixes before round 2, which also update its version.
  - **Review:** Codex found one CHANGELOG error: a toast action's failure
    propagates from the input call that activated it, not from a flush. It
    was corrected; the implementation matched 6b1. The correction is
    verified in 6b2's review.
- **6b2 (0.3.61), done 2026-10-22:** 8af8628 and its review fix. hand-written `Debug`, ending in `finish_non_exhaustive`,
  except `FilterTarget`, which derives it, for the state owners, handles,
  contexts and backends. None reads a signal, borrows shared state, formats
  a callback or bounds application values by `Debug`.
  - **Checks run after the last edit:**
    - `cargo fmt --all --check`;
    - workspace Clippy, and Clippy in every CI feature configuration;
    - `cargo test --locked --workspace`, including the new
      `foundation::standard_traits` Debug tests and wgpu's
      `renderer_implements_debug`;
    - the five headless example sessions;
    - `cargo test --locked --workspace --doc`;
    - warning-free workspace rustdoc;
    - `api/generate.sh`, whose diff is the plan's expected `Debug` lines;
    - `git diff --check`.
  - **Review:** Codex found no implementation defect. Four wordings
    claimed more than the code: the derived `FilterTarget`, the selection
    mode of `ListController` and `ListFilter`, a query `ListFilter` cannot
    return, and overrides `IconManager` cannot read. All four were
    corrected; the correction is verified in 6b3's review. The review also
    verified 6b1's correction.
- **6b3 (0.3.62), done 2026-10-22:** 94d661b and its review fix. `Debug` for the remaining 87 descriptions (86 runtime
  types and `pert_ui::Svg`), completing G4-1's 113 with 6b2's.
  `missing_debug_implementations` is denied and listed in AGENTS.md.
  - A crate-private `description_debug!` macro writes the
    `finish_non_exhaustive` form.
  - Keyed items show their key. `Element` shows key, fixed label and
    children, and `Svg` shows its sizes, tint and description.
  - No impl reads a signal, borrows shared state, formats a callback or
    bounds a parameter by `Debug`.
  - **Checks run after the last edit:**
    - `cargo fmt --all --check`;
    - workspace Clippy, and Clippy in every CI feature configuration;
    - `cargo test --locked --workspace`, including the new
      `standard_traits` description tests and the `svg` Debug test;
    - the five headless example sessions;
    - `cargo test --locked --workspace --doc`;
    - warning-free workspace rustdoc;
    - `api/generate.sh`, whose diff is the plan's expected `Debug` lines;
    - `git diff --check`.
  - **Review:** Codex found no implementation defect. It found one
    wrong sentence: the shared rustdoc said every description becomes an
    `Element`, which `Toast` and `DragPreview` do not. The sentence was
    qualified and rustdoc rebuilt warning-free; the fix is verified in
    6c1's review. The review verified 6b2's four corrections.
- **6c1 (0.3.63), done 2026-10-23:** 68dd6d2 and its review fix. Error values name what was rejected: keys, IDs,
  characters, positions, byte offsets, paths and integer limits.
  - **Maintainer decision (2026-10-23):** variants that validate a float
    stay unit in every error enum, extending decision G4-15's rule, so every
    error keeps `Eq` (recorded in contracts.md "Error policy").
  - **`Copy`:** `LayoutError` and `ParameterError` lose it, as G4-15
    accepted. `EditError`, `BindingError` and `EditOutcome<T>` lose it
    because they hold a `ParameterError`, and `FontError` because of
    `UnknownIcon(String)`.
  - **Causes and limits:**
    - workspace causes print once in a chain;
    - one `LimitError::FrameSize` reports every frame limit, with
      `CaptureError::Limit` and `CpuRenderError::Limit`, and
      `CpuRenderError::AllocationFailed` for the allocation failure.
  - **Author decision:** `DrawError::InvalidImage`, which reported three
    failures, is split into:
    - `InvalidImage { width, height, bytes }`;
    - the unit `CoverageTooLarge`;
    - `MissingVector`, `MissingMask` and `MissingImage`, each with its ID.
  - **Checks run after the last edit (the public-API row plus loom):**
    - `cargo fmt --all --check`;
    - workspace Clippy;
    - `cargo test --locked --workspace`, including the new message,
      chain and path tests;
    - the five headless example sessions;
    - `cargo test --locked --workspace --doc`;
    - warning-free workspace rustdoc;
    - the loom models;
    - `api/generate.sh`, which removes `Eq` from no type;
    - `git diff --check`.
  - **Beyond the plan's lines:** `DuplicateKey` from `FormFields::field`,
    table columns and reparenting carries the key too.
  - **Review:** Codex found three points:
    - the missing-resource messages omitted their IDs, and now print the
      index, with message assertions;
    - `CoverageTooLarge` promised clipping that only vector coverage has,
      and its rustdoc now says so;
    - the record claimed the serde-gated serialized-form test ran in the
      workspace tests.
  - **After the fix:** the serde-gated test was run and the configuration
    checks the review expected were added:
    - `cargo test --locked -p pert-ui --features serde --test foundation`;
    - Clippy and tests for `pert-ui` with `serde`, with
      `--no-default-features --features serde`, with
      `--no-default-features --features desktop-cpu` and with `cpu`;
    - serde rustdoc;
    - pert-ui-runtime's tests and doctests;
    - warning-free workspace rustdoc;
    - `api/generate.sh`, with no further difference.

    The fixes are verified in 6c2's review.
- **6c2 (0.3.64), done 2026-10-23:** 12aea82 and its review fix. Compiler diagnostics.
  - `#[diagnostic::on_unimplemented]` on the consumer traits, `StateOwner`
    and its seal, renamed `StateOwnerSeal`.
  - `do_not_recommend` on `AppError`'s blanket `From` (F11), with the
    accepted errors listed in its rustdoc.
  - A `compile_fail` doctest for sealing `StateOwner`, and `Modifiers`
    field aliases and example (`S:m06`).
  - A `publish = false` `pert-ui-diagnostics` member with `trybuild`
    snapshots, run by its own CI job on the pinned toolchain and excluded
    from packaging; AGENTS.md, CI, `docs/verification.md`, the README and
    the kit's packaging command are kept in step.
  - **Checks run (the dependency row):**
    - the workspace gate;
    - `cargo test --locked -p pert-ui-diagnostics -- --ignored`;
    - release-profile tests;
    - Clippy, tests and rustdoc in every CI feature configuration;
    - the headless gallery;
    - the loom models (core's seals changed);
    - `api/generate.sh`;
    - `git diff --check`.

    The feature configurations, release tests, example sessions and gallery
    ran before the follow-up below, which changed only seal names, rustdoc
    and two snapshot cases, and is covered by the workspace gate, loom and
    the diagnostics suite rerun after it. After the commit, library
    packaging (excluding `pert-ui-diagnostics`) and the 0.3.64
    packaged-consumer check passed.
  - **Beyond the plan's lines:**
    - the same similar-name defect in pert-ui-core's seals, renamed
      `ParameterValueSeal` and `LiveItemSeal` with their own messages and
      snapshot cases `p01` and `v01`;
    - `StateOwner`'s summary names `DescribeContext`;
    - the root README keeps the packaging command in step;
    - `Cargo.lock` moved `toml_datetime` and `toml_parser`, which
      `trybuild` shares.
  - **Recorded in TODO.md:** `AppError`'s accepted-error list is unchecked,
    and the `Component` snapshots list rustc's first implementors.
  - **Review:** Codex found that `AppError`'s fallback advice did not hold
    for boxed or non-`Send` errors. It now gives the boxed route and says a
    non-`Send` error's message is wrapped. The review also asked for the
    packaging results here, and verified 6c1's fix. Doctests and rustdoc
    were rerun after the fix, which 6d1's review verifies.
- **6d1 (0.3.65):** the collections adopt a redescription's permissions
  and callbacks, including removal.
  - A fixed denial cancels a gesture in the update the redescription
    schedules, so nothing is published from `reconcile`.
  - A removed selection callback discards undelivered changes.
  - `RegionLayout` forwards resizes only while it has a callback.
  - The radio-style choices keep their old selection on a rebind until the
    new list validates.
  - `ContextMenu`'s enabled state is a private cell.
  - **G2-13's test came first:** a custom component and a mapped label
    reading the cancelled snapshot both settle in the reconciling flush, for
    a number input and a slider, on rebind and on fixed denial. So the
    lifecycle exception is documented, with no scheduling change.
  - **Maintainer decision (2026-10-23):** AGENTS.md's rule becomes "Never
    publish signals from reconciliation, except for the lifecycle cleanup
    contracts.md names, …".
  - **Checks run after the last edit:**
    - `cargo fmt --all --check`;
    - workspace Clippy;
    - `cargo test --locked --workspace`, with every new behavior test
      checked to fail with the runtime changes stashed;
    - the five headless example sessions;
    - `cargo test --locked --workspace --doc`;
    - warning-free workspace rustdoc;
    - `api/generate.sh`;
    - `git diff --check`.

    The `api/` diff, which the plan expected to be empty, shows only the
    four overridden `reconcile` methods losing the default's
    `where Self: Sized`.
  - **Deviation:** a `FileBrowser` mounted without `folder_filter` cannot
    gain the field later. Its tree holds the documented child key, and
    replacing or removing the filter follows the description.
  - **Beyond the plan's lines:** `AppError`'s non-carriable errors are
    `Send + Sync`, which tightens 6c2's fix.
  - **Left to a later part:** `N:publish-revision`, found here, joins 6d2
    and TODO.md.

## Rules for every part

- **One review unit per part.** Each part is one commit (plus review-fix
  commits) with an independent Codex review (`codex exec -s read-only`)
  before it is called done, as the plan's working rules require. A part
  builds on the previous one in the order below.
- **Versions.** Every part changes a packaged file, so every part, slice
  5's included, takes its own workspace patch version and its own CHANGELOG
  entry. This refines the developer-usability plan's per-slice version rule
  to one version per reviewed part. A bump updates the workspace version,
  the internal version requirements and `Cargo.lock`.
- **CHANGELOG.** Every entry names each item the part's `api/` diff adds,
  removes or changes. A breaking change is a `Breaking:` entry with its
  replacement usage, as the plan's working rules require; while the library
  is unpublished, the compatibility policy allows these in patch versions.
- **Text describes the code of its own commit.** Slice 5 documents the API
  as it is. A rule sentence that names or depends on a slice 6 change lands
  in that part, not in slice 5 (G1-3, G1-9, G2-6, G3-1, G3-7, G3-19, G4-6,
  G4-8/G4-9, G4-15, G4-30, G5-11, G5-25), so no published text advertises an
  item that does not exist yet.
- **Consumers migrate in the same part.** A part that renames or reorders
  updates every repository consumer: the packaged examples and their
  sessions, the gallery, the guide, `docs/manual.md`, the READMEs, the
  tests, and the round 2 fixture `plans/usability/task4-app`, which builds
  against the packages.
- **Tests.** New facade tests join the area binary they cover
  (`crates/pert-ui/tests/<area>/main.rs`). Each behavior change has a
  regression or contract test.
- **Persisted formats are unchanged.** No part changes
  `Workspace::SCHEMA_VERSION` or `InspectionSnapshot::SCHEMA_VERSION`.
  `#[non_exhaustive]` (6a3) does not affect serde. V3-2 (6d2) rejects a
  stored column width the library never writes. F26 (6d2) adds an address
  segment only to the search field, which has no workspace state; the
  collection's address is unchanged. G4-13 (6c1) changes `CaptureError`'s
  serde form, a transport form outside any versioned schema; the CHANGELOG
  says so.
- **Not in this plan:** the lint survey's `clippy::must_use_candidate`,
  which the survey leaves to the maintainer as a getter policy; it stays off
  unless chosen (see [open points](#open-points)).

## Order and constraints

| Part | Content | Version | Breaking |
|---|---|---|---|
| 5a | Rules: contracts.md and AGENTS.md | 0.3.53 | no |
| 5b | Item rustdoc | 0.3.54 | no |
| 5c | Guide and recipes | 0.3.55 | no |
| 5d | Facade spelling in examples | 0.3.56 | no |
| 6a1 | Construction and builders | 0.3.57 | yes |
| 6a2 | Names and roles | 0.3.58 | yes |
| 6a3 | Types and traits | 0.3.59 | yes (`#[non_exhaustive]`, `!Send`) |
| 6b1 | Lints and `#[must_use]` | 0.3.60 | no |
| 6b2 | `Debug` for state owners, handles, contexts and backends | 0.3.61 | no |
| 6b3 | `Debug` for descriptions; the lint denied | 0.3.62 | no |
| 6c1 | Error values | 0.3.63 | yes |
| 6c2 | Compiler diagnostics | 0.3.64 | no |
| 6d1 | Redescription and reconciliation | 0.3.65 | yes (behavior) |
| 6d2 | Editors, data and defects | 0.3.66 | yes |
| 6e | Prelude and `component` module | 0.3.67 | no |
| 6f1 | Testing and window close | 0.3.68 | yes (behavior, F4) |
| 6f2 | Editing and collection capabilities | 0.3.69 | no |
| 6f3 | Controlled controls | 0.3.70 | no |

Constraints:
- **Renames before the text and re-exports that name them.** 6a renames
  before 6b2 writes `Debug` on final names, before 6e re-exports them
  (`ListDropPosition`, `ContextMenuController`, `ToastController`, G3-14's
  name) and before 6f's guide chapter names them.
- **G1-2 and G1-3** are one migration (6a1); G1-15's parameter names (5b)
  feed G1-3's sentence.
- **G4-11 before G4-13:** 6a3 derives `Eq` on `CpuRenderError`, which 6c1
  then reshapes.
- **G4-15 gates G4-14:** both in 6c1, the `Copy` rule first.
- **6b2 and 6b3 before the later parts:** once 6b3 denies
  `missing_debug_implementations`, every new type of 6c to 6f carries
  `Debug`.
- **G4-14 after 6a3 and before 6d2:** 6a3's `Eq` on `CpuRenderError` and
  6d2's use of `TextEditError::InvalidSelection` (G1-11) meet the variant
  shapes 6c1 sets.
- **6c2 after 6a and 6c1:** the seal rename and the trait names are final
  before the trybuild snapshots are taken. 6e and 6f regenerate a snapshot
  only if a new re-export changes a message's path, and the diff is reviewed
  as part of that part.
- **G2-13's test gates its text:** 6d1 writes the test first. If a
  component and a `Computed` both settle in the reconciling flush, the
  lifecycle exception is documented; otherwise 6d1 fixes the settle loop
  and decision G2-13's option B is reconsidered against that fix before the
  part proceeds. G2-2 and G2-14 come first in 6d1, because a fixed denial in
  a collection must cancel without publishing or fall under the same
  exception.
- **6d2 after 6a2:** G1-11 changes `TextSelection::new` after G3-10's
  renames; G2-20 adds `StringRole` variants after G3-3's `UiStrings`
  accessors.
- **6e after 6a3:** the `component` module re-exports the authoring records
  that 6a3 keeps exhaustive and constructible from `Default`.
- **6f after 6e:** the `testing` module sits beside the prelude and the
  `component` module, and G5-6's prelude-page list moves the capture names
  to it. F4's regression test needs F5's headless close (both 6f1). F34
  needs F2's helpers. 6f2's `CollectionFilter` policy follows 6d1's
  collection redescription, and `InlineText::session` follows 6d2's editor
  changes. 6f3's controlled `InlineText` builds on 6f2's session and
  list-row mode.
- **Round 2** (slice 7) waits until every part, 6f3 included, is closed,
  and runs on that branch head.

## Slice 5: documentation

### 5a Rules: contracts.md and AGENTS.md

The naming rules, permissions, notifications and error policy of
`docs/contracts.md`, and AGENTS.md's clamp sentence (G1-12, worded as the
decision approved). It writes only rules the code already follows.

- **Gate:** the Markdown-only row for contracts.md and AGENTS.md (author
  review, links, whitespace, consistency with existing decisions); the
  rustdoc row for the rustdoc lines it adds (targeted doctests,
  warning-free rustdoc).
- **`api/` diff:** none.
- **Breaking:** no.
- **CHANGELOG:** its own "Documentation" entry.
- **Version:** 0.3.53.

Closes 21 audit findings, 0 round 1 findings and 19 decisions.

Audit findings:

- **G1-5:** Naming rules (`docs/contracts.md:1151`): a validated string borrows its text with `as_str()`.
- **G1-6:** Naming rules: `get()` covers single-value types; a compound value has component accessors and a whole-value accessor named for its content (`channels`, `values`, `entries`).
- **G1-10:** Naming rules: limits the library chooses are constants; intrinsic bounds (nonzero, nonnegative, unit interval) are stated on the constructor and its `# Errors`.
- **G1-12:** AGENTS.md "Rust and data invariants" clamp sentence replaced by the decided wording; the naming rules link it; `ProgressStatus::of` and `TableColumn::width`/`set_column_width` name their clamp kind.
- **G1-13:** Naming rules: the three meanings of `with_` by receiver; a getter, where one exists, takes the bare name; component descriptions use bare builder names.
- **G1-16:** Contracts "Choice sources": collections whose rows are content and keyed containers are outside the `new`/`bound`/`for_parameter` vocabulary.
- **G2-1:** Naming rules: a no-argument builder moves one fixed option away from its default (`disabled()`, `read_only()`, `frameless()`, `nonmodal()`); an option decided from data takes a `bool`.
- **G2-3:** Contracts "Permissions and availability": title-bar parts and `App::resizable` are fixed `bool` part choices; a rustdoc line on `FloatingWindow::closable`/`minimizable` (`windows.rs:327`, `:333`).
- **G2-4:** Clause in G2-1's sentence: data descriptions take `bool`; `SemanticItem::disabled`/`Semantics::disabled` say `disabled(false)` does not restore withdrawn actions (`diagnostics.rs:392-397`, `:579-585`).
- **G2-5:** Naming rules: element builders append, whole-list and single-slot builders replace; rustdoc fixes on `Semantics::actions` and `Toast::action` (`notifications.rs:156-163`).
- **G2-17:** Contracts "Component notifications": `on_select` reports each settled tab change, `on_selection_change` a coalesced bound selection; `VirtualList` rustdoc (`virtual_list.rs:834-836`) rephrased.
- **G3-2:** Contracts "Permissions and availability" (`:296-298`): the adjective restricts a built-in capability by `Permission`, or creates a callback- or flag-supplied one whose restriction is `*_permission`.
- **G3-16:** Naming rules: `*State` names a value its owner holds directly, never a shared handle (`ViewState`, `ParameterState`).
- **G3-17:** Naming rules: `*Snapshot`, `*Inspection` and `*Report` suffixes, and descriptive nouns for other passive data.
- **G3-18:** Naming rules: `*Outcome` is the non-error result of an interaction or operation; results may be named for what they deliver and carry a continuation.
- **G3-21:** Naming rules: a cross-thread reply consumed once (`Pending<T>`) is not a receipt.
- **G4-10:** Contracts "Error policy": a kind-only conversion may return its input as the error, as std's array `TryFrom` does.
- **G4-19:** Contracts "Error policy": a `TryFrom`/`FromStr` impl states its failures in one line linking the documenting constructor.
- **G4-20:** Contracts "Error policy": serde impls are documented in the type's `# Serialization`; sections added to `PowerExponent` (`core/parameter.rs:61`) and `FrameImage` (`capture.rs:315`).
- **G4-23:** Contracts "Error policy": carriers of application or OS text (`InputRejection`, `ListingError`) display it verbatim.
- **G4-24:** Callback-boundary rationale for `From<WindowError> for RuntimeError` corrected in `docs/contracts.md:1125`, `R/runtime.rs:158` and `R/error.rs:15`.

Decisions:

- **D:G1-5:** Applied: string exception `as_str()` (G1-5).
- **D:G1-6:** Applied: `get()` for single-value types only (G1-6).
- **D:G1-10:** Applied: only chosen limits get constants (G1-10).
- **D:G1-12:** Applied: the AGENTS.md clamp wording and its link (G1-12).
- **D:G1-13:** Applied: three documented meanings of `with_` (G1-13).
- **D:G1-16:** Applied: one "Choice sources" sentence (G1-16).
- **D:G2-1:** Applied: the widened no-argument rule (G2-1).
- **D:G2-3:** Applied: fixed window parts documented (G2-3).
- **D:G2-4:** Applied: data descriptions take `bool` (G2-4).
- **D:G2-5:** Applied: appending builders documented (G2-5).
- **D:G2-17:** Applied: both names kept, difference documented (G2-17).
- **D:G3-2:** Applied: the rule the code follows (G3-2).
- **D:G3-16:** Applied: `*State` by ownership (G3-16).
- **D:G3-17:** Applied: the suffixes named in the rule (G3-17).
- **D:G3-18:** Applied: the one-directional `*Outcome` rule (G3-18).
- **D:G3-21:** Applied: a cross-thread reply is not a receipt (G3-21).
- **D:G4-10:** Applied: std-idiom exception in "Error policy" (G4-10).
- **D:G4-19,G4-20:** Applied: one-line forwarding and type-level `# Serialization` (G4-19, G4-20).
- **D:G4-23:** Applied: verbatim-text exception (G4-23).

### 5b Item rustdoc

Rustdoc of items that keep their names: `# Errors` sections, forwarding
lines, cross-links, the prelude pages and the interim narrowing of G5-25's
three sentences (closed in 6e). Also, from the surveys and the audit's notes
(no ID): the `'static` requirement on `Component`, `ListItems`,
`TreeItems`, `TableItems`, `ContentRows` and `Clipboard`, explained at
`Component::content_rows` (diagnostics survey, r04), and the values of
`Workspace::MAX_DOCUMENT_BYTES`, `VectorRasterizer::MAX_COVERAGE_WORK` and
`Renderer::READBACK_TIMEOUT` (claude-C4-v3-3). Parameter renames (G1-15,
G3-8, V3-3) change no listing.

- **Gate:** the rustdoc row (targeted doctests, warning-free rustdoc), plus
  formatting and targeted Clippy for the packages whose parameter names
  change, and `api/generate.sh` showing no difference.
- **`api/` diff:** none.
- **Breaking:** no.
- **CHANGELOG:** its own "Documentation" entry.
- **Version:** 0.3.54.

Closes 16 audit findings, 8 round 1 findings and 5 decisions.

Audit findings:

- **G1-15:** Parameters renamed `runtime` to `ui` in `ListController::new` (`virtual_list.rs:275`) and `ZoomController::new` (`zoom_view.rs:140`), `state` to `controller` in `Dialog::new`/`AlertDialog::new` (`dialog.rs:195`, `:506`), with their prose.
- **G2-18:** Doctest on `MultiSelect::filter` (`multi_select.rs:1382`) reusing `FilterMatcher` presets; it says the query arrives untrimmed.
- **G3-8:** `AxisMapping` parameters renamed `extent_pixels` and `anchor_pixel` (`axis_mapping.rs:88`, `:101`, `:146`) with their doc links.
- **G4-7:** `RichText` and `TextSpan` rustdoc: UI-thread values because a span can name a `FontId`; a sendable font key added to `plans/follow-ups.md`.
- **G4-16:** `# Errors` sections of every item in the row list the reachable phase errors or link `Signal::set`'s list, per the author's resolutions D2-D4.
- **G4-17:** `# Errors` sections of every item in the row name their variants, with a shared binding-family paragraph where many share them.
- **G4-18:** One-line forwarding `# Errors` on `ChoiceId`'s two `TryFrom` impls and the `try_from` of `Magnification`, `UiZoom` and `ViewScale`.
- **G4-21:** `Runtime::paint` rustdoc and a guide/diagnostics.md line: a helper that also paints returns `Result<_, DrawError>`, or `AppError` with `native`.
- **G4-22:** `ApplicationError::new` and `RuntimeError::Application` rustdoc point to `ApplicationError::from` for boxed errors.
- **G5-4:** `DraftEditor`'s owner contract (`text_fields/draft.rs:437-`) links `Component::retain_initial_children` (with F17).
- **G5-5:** `Runtime::text_command`'s summary reads "a text command (clipboard or selection)".
- **G5-6:** The prelude pages (`R/lib.rs`, `F/lib.rs`) list the root names applications most often add, since rustc offers no import; 6e and 6f1 keep the list current.
- **G5-29:** The facade prelude's error sentence names `AppShutdownError` (`F/lib.rs:58-60`).
- **G5-31:** The facade prelude says its native members come with `native`, enabled by `desktop` and `desktop-cpu` (`F/lib.rs:50-51`, `:59`).
- **V3-3:** `TextField`/`TextArea::{min_length, max_length}` take a parameter named `graphemes` and say the unit first (`text_fields.rs:450-458`).
- **V3-4:** `Constraints` and `Constraints::{new, width, height}` rustdoc say logical pixels.

Round 1 findings:

- **F7c:** `FilterMatcher`, `ListFilter::filter`, `TableController::filter` and `TreeController::filter` say a non-blank query is trimmed before the matcher sees it, and point to `ListFilter::items()`.
- **F8:** `Semantics::value`, `SemanticValue::Text` and `Label` say static text is the name and the value is an editable or stateful control's content.
- **F12:** `Component::cancel_input` states that it runs outside evaluation phases, may restore the widget's own bindings, must not activate actions, and that a failed restore is recorded and returned through a `Notifier`; guide "Who may write" lists it.
- **F17:** Owner-contract bullets: an owner described again returns `true` from `retain_initial_children` or reconciles controller and child together; `begin` runs the owner's `cancel_input`, so a draft ends in `focus_within_changed(false)`.
- **F18:** guide/custom-widgets.md:86-89 and guide/controls.md:11 corrected to what Escape does (the focused component's capture, timer and edit sessions end, then the key arrives; no `cancel_input`); `Runtime::key_input` and `EventContext::set_timer` say so.
- **F24:** `SemanticValue` and guide/custom-widgets.md "Inspection": a multi-component value is `Text` plus one `InspectValue::Number` property per component, as `ColorPicker` does.
- **F30:** Closed by G4-22's rustdoc pointers; the source loss was already resolved.
- **F32:** The hide and removal paths are probed again and `ColorPicker::cancel_input`'s comment (`color_picker.rs:1164-1167`) corrected.

Decisions:

- **D:G1-15:** Applied: four parameters renamed, no API listing change (G1-15).
- **D:G2-18:** Applied: `filter` kept with a `FilterMatcher` doctest (G2-18).
- **D:G4-7:** Applied: UI-thread restriction documented, sendable key to follow-ups (G4-7).
- **D:G4-21:** Applied: exclusion kept, `DrawError` route documented (G4-21).
- **D:G4-22:** Applied: no new `From`, doc pointers (G4-22).

### 5c Guide and recipes

Guide chapters (`F/guide/`): getting started, the worker example, the
editable-list recipe, large collections, screenshots and the state chapter,
with the one `VirtualList::new` rustdoc line `S:l06` needs.

- **Gate:** the rustdoc row: the guide's doctests, warning-free rustdoc,
  and the headless sessions of examples the recipe draws on.
- **`api/` diff:** none.
- **Breaking:** no.
- **CHANGELOG:** its own "Documentation" entry; it notes F1's version
  test.
- **Version:** 0.3.55.

Closes 2 audit findings, 6 round 1 findings, 0 decisions and 1 survey case.

Audit findings:

- **G5-2:** `EventContext::defer` and `Runtime::spawn` link each other; guide/state.md "Worker results and live data" gains a complete defer-then-spawn example.
- **G5-3:** `Signal::sibling`, `ListItems::build` and `TableItems::cell` describe per-row bindings and their cost; F14's recipe shows the per-row and the retained-row form.

Round 1 findings:

- **F1:** guide/getting-started.md:9-19 leads with `pert-ui = { version = "…" }` kept in step by a test (as `tests/foundation/readme.rs` does), the checkout as the alternative, the MSRV from status.md, and that examples need a checkout.
- **F7a:** `CollectionFilter` "Selection and focus" and F14's recipe show how to tell that the selected row is hidden (`ListFilter::items()`, `ListItems::index_of`, `ListSelection::active`).
- **F9:** `Each`'s example adds a lookup through the default `items` segment (`described.rs:245-266`).
- **F10:** guide/diagnostics.md "Screenshots" shows `CaptureTarget::window(&ui.primary_window())`.
- **F14:** A guide recipe for an editable, selectable, filterable list of records (`Vec` to `ListSource`, `ListFilter`, `VirtualList` with `ListSelection`, `on_activate`, `reorderable`, per-row controls), linked from "Children from values".
- **F28:** guide/state.md says signal values need `PartialEq` to be written and `Clone` to be read.

Survey cases:

- **S:l06:** `VirtualList::new` rustdoc and the guide's "Large collections" show the existing route: wrap the `ListItems` value in `ListSource::new`; no new API.

### 5d Facade spelling in examples

A mechanical sweep of the runtime's and core's example blocks (276 of 310
on the facade's root pages). The part first proves the hidden alias
(`# use pert_ui_runtime as pert_ui;`, or the core crate) on one item page
and one doctest, then applies it.

- **Gate:** the rustdoc row at full scale: `cargo test --locked --workspace
  --doc`, warning-free rustdoc for every crate, and a check of the rendered
  facade pages for remaining `pert_ui_runtime::` imports.
- **`api/` diff:** none.
- **Breaking:** no.
- **CHANGELOG:** its own "Documentation" entry.
- **Version:** 0.3.56.

Closes 1 audit finding, 1 round 1 finding and 0 decisions.

Audit findings:

- **G5-1:** Every runtime and core example block shows `use pert_ui::…` through a hidden crate-alias line, so inlined facade pages show the facade spelling and the doctests still build; `F/root.md:42-47` updated.

Round 1 findings:

- **F20:** Closed by G5-1's sweep.

## Slice 6: API

Every slice 6 part runs the public-API row of the AGENTS.md table: the
workspace gate, the API listing gate with its committed result, focused
contract tests and semantic review. The rows below add what differs.

### 6a1 Construction and builders

Constructor order, construction names, limit constants and builder
signatures. Every reorder is type-checked: the swapped arguments have
different types, so an old call fails to compile rather than misbinding.

- **Gate:** the public-API row.
- **`api/` diff:** reordered `Checkbox::new`, `Toggle::new`,
  `TableController::new`, `TreeController::new`,
  `FileBrowserController::new`, `MenuItem::toggle`, `MenuItem::radio`;
  `Color::rgba` replaced by `Color::from_linear`; `SplitRule::Ratio`'s
  payload and `SplitSizing::rule` without `const`; `PowerExponent::MIN`/`MAX`
  typed `Self`; the three `bind_*` renames; the command-button callback
  signatures; `Accordion::on_change` and the new `ExpansionChange`;
  `set_wake_delivery`'s parameter; the new constants and
  `SizeLimits::exact`.
- **Breaking:** yes.
- **CHANGELOG:** `Breaking:` entries with replacement usage; "Added" for
  the constants and `SizeLimits::exact`.
- **Version:** 0.3.57.

Closes 11 audit findings, 1 round 1 finding and 6 decisions.

Audit findings:

- **G1-2:** Bound state first after `ui`: `Checkbox::new`/`Toggle::new(value, label)` (macro `choice_controls.rs:525`), `TableController::new(ui, selection, columns, source)`, `TreeController::new(ui, selection, source)`, `FileBrowserController::new(ui, selection, roots, request)`.
- **G1-3:** `MenuItem::toggle(key, value, label)` and `MenuItem::radio(key, selection, value, label)` (`menu.rs:299`, `:333`); the naming-rule sentence on parameter order.
- **G1-7:** `PresentationRate::MIN`/`MAX` typed `Self` and `ViewState::MAX_NAME_BYTES` (`workspace.rs:235`), linked from the errors and `# Errors`.
- **G1-8:** `VectorRasterizer::MAX_FLATTENED_POINTS` and `MAX_STROKE_EDGES` (`vector_scene.rs:103-107`), backed by the private `MAX_EDGES` (`geometry/raster.rs:11`).
- **G1-9:** Placement rule for validation limits and rendering caps in the naming rules; reasons on `NumberInputError::MAX_PRECISION`, `TextError::MAX_FONT_PIXELS` and `TextLimit::MAX_RASTER_SIDE`; `PowerExponent::MIN`/`MAX` typed `Self`.
- **G1-14:** `Color::rgba` (`drawing.rs:258`) renamed `Color::from_linear` (about 90 call lines); `Color::channels` says linear.
- **G1-18:** `SplitRule::Ratio(NormalizedValue)` (`split.rs:141-145`); `SplitSizing::rule` stops being `const`, its private storage stays `f32`.
- **G2-6:** `Collapsible::bound` to `bind_expanded`, `ToolWindowLayout::names` to `bind_show_names`, `PropertyGrid::shared_label_width` to `bind_label_width`; the `bind_<property>` naming rule.
- **G2-15:** `Button::command`, `Button::command_icon` and `MenuItem::command` take `FnMut(&mut EventContext, CommandReceipt)` (`controls.rs:552`, `:592`; `menu.rs:404`).
- **G2-16:** `Accordion::on_change` takes `FnMut(&mut EventContext, &ExpansionChange)`, a new struct carrying the expanded keys and the `ChangeOrigin` (`collapsible.rs:1049`).
- **G2-19:** `Runtime::set_wake_delivery` (`runtime.rs:4183`) takes `impl Fn() -> bool + Send + Sync + 'static`; internal callers drop `Box::new`.

Round 1 findings:

- **F33:** `SizeLimits::exact(Size)`, a `const`, infallible constructor for a fixed size (`layout.rs`).

Decisions:

- **D:G1-3:** Applied: owner `ui`, key, bound state, then content (G1-3, with G1-2).
- **D:G1-9:** Applied: placement rule, reasons and `PowerExponent` constants as `Self` (G1-9).
- **D:G1-14:** Applied: rename to `Color::from_linear` (G1-14).
- **D:G1-18:** Applied: `NormalizedValue` payload, `rule` no longer `const` (G1-18).
- **D:G2-6:** Applied: `bind_<property>` for the three two-way builders (G2-6).
- **D:G2-19:** Applied: `impl Fn` signature (G2-19).

### 6a2 Names and roles

Renames for reads, roles, units and families, the typed flags of G3-11, and
the wgpu renderer's extent.

- **Gate:** the dependency, feature, platform and release row, because
  G3-6 changes the renderer interop the native path uses: the workspace
  gate, the release-profile tests, the MSRV, every supported feature
  configuration AGENTS.md lists (no default features, the image, SVG,
  serde, CPU and CPU-only desktop configurations, the headless gallery),
  the API listing gate, library packaging and the packaged-consumer check,
  and the native checks (`cargo run --locked -p pert-ui-native-checks`) on
  Linux with the platform recorded. Windows and macOS evidence, not
  available here, is recorded as missing in the native evidence record.
- **`api/` diff:** every renamed item; `FocusDirection` and the two other
  new flag enums; the changed `tab_navigation`, `focus_next`,
  `request_reveal`, `TextSelection::{moved, at_point}`,
  `ToolWindowChange::new` and `EditorTrigger::Tab`; `Renderer`'s three
  signatures; `NativeCloseAction::Defer`'s payload; the `Length` widths and
  constants; `PendingJump::receipt`.
- **Breaking:** yes.
- **CHANGELOG:** `Breaking:` entries with replacement usage.
- **Version:** 0.3.58.

Closes 18 audit findings, 0 round 1 findings and 9 decisions.

Audit findings:

- **G3-1:** The seven `UiSettings`/`ThemeManager` `*_signal` getters renamed `*_binding`, `DeliveryMonitor::observe` renamed `snapshot_binding`; the owned/reactive naming rule; `Parameter::snapshot` (live) and `AnyParameter::snapshot` (owned) documented.
- **G3-3:** `ThemeMetrics::get` to `metric`, `UiStrings::get`/`with` to `text`/`with_text` (`runtime/theme.rs:279`; `strings.rs:625`, `:601`).
- **G3-4:** `DragPayload::get::<T>` renamed `downcast_ref::<T>` (`drag.rs:53`).
- **G3-5:** `EditorSession::is_open` renamed `is_active` (`editor_session.rs:188`).
- **G3-6:** `pert_ui_wgpu::Renderer::{new, new_surface, resize}` take `PhysicalExtent` (`W/lib.rs:391`, `:1271`, `:485`); the facade's platform code migrates.
- **G3-7:** `QuickPick::width` and `CommandPalette::width` take `Length` (`quick_pick.rs:975`, `command_palette.rs:611`); the naming-rule sentence on logical lengths.
- **G3-9:** `TextLayout::{paint, paint_turned}` take a `Point` (`text.rs:262`, `:284`); `fitted_text`, `ListDropPoint::across` and `VirtualAxis` conform to G3-7's sentence.
- **G3-10:** `ResourceKind::scene_budget` to `scene_budget_bytes`, `TextSelection::{anchor, focus, range}` to `{anchor_byte, focus_byte, byte_range}`, `TextPreedit::cursor` to `cursor_byte_range`; byte-named parameters on their constructors and `RichText::highlighted`.
- **G3-11:** New `FocusDirection` for `Runtime::focus_next`, `Component::tab_navigation(cx, direction, transition: KeyTransition)` and `EditorTrigger::Tab`; `EventContext::request_reveal_and_focus`; enums for `TextSelection::{moved, at_point}`'s extension and `ToolWindowChange::new`'s expansion.
- **G3-12:** `DropPosition` renamed `ListDropPosition` (`list_drag.rs:47`); `PickStatus` stays.
- **G3-13:** `ContextMenuOpener` renamed `ContextMenuController`, `ToastQueue` renamed `ToastController`; `FieldHandle` stays under G3-19. The accessor follows the type, `ContextMenu::opener()` becoming `controller()` as `FloatingWindow::controller` is named (added during 6a2; the maintainer is told).
- **G3-14:** `FormStatus` (a cloneable handle, `field_status.rs:214`) renamed to a non-controller `Form…` name, proposed `FormFields`.
- **G3-15:** `PendingJump` keeps its name as a continuation; its forwarders become `receipt(&self) -> &ListRevealReceipt` (`content_search.rs:416-428`).
- **G3-19:** Naming rules: application-owned coordinators (`ListSelection`, the renamed `FormStatus` and its `FieldHandle`s) are named for what they hold; `*Controller` means widget-owned state.
- **G3-22:** `QuickPick`/`CommandPalette::DEFAULT_WIDTH` typed `Length`; the other read-back `f32` lengths conform to G3-7's sentence and say logical pixels.
- **G3-23:** `UiStrings::MAX_LENGTH` renamed `MAX_TEXT_BYTES` (`strings.rs:576`).
- **G3-24:** `TaskStatus::Completed` renamed `Succeeded`.
- **G3-25:** `NativeCloseAction::Defer(ReadSignal<Option<WindowCloseAction>>)` (`F/app.rs:207-210`); `diagram_editor/app.rs:891`, `:910-911` lose their comment.

Decisions:

- **D:G3-1:** Applied: option C, eight renames, residual risk documented (G3-1).
- **D:G3-3:** Applied: noun accessor pairs (G3-3).
- **D:G3-4:** Applied: `downcast_ref` (G3-4).
- **D:G3-5:** Applied: `EditorSession::is_active` (G3-5).
- **D:G3-7:** Applied: logical-length convention, two width builders (G3-7).
- **D:G3-12:** Applied: `ListDropPosition`, `PickStatus` kept (G3-12).
- **D:G3-19:** Applied: coordinator sentence, landing with G3-14's rename (G3-19).
- **D:G3-23:** Applied: `MAX_TEXT_BYTES` (G3-23).
- **D:G3-24:** Applied: `TaskStatus::Succeeded` (G3-24).

### 6a3 Types and traits

Standard traits, conversions, `#[non_exhaustive]` and the `!Send` marker.
`Capacity` lives in `pert-ui-core`'s `live` module; its derives and `From`
do not touch the protocols, but the part runs the loom models anyway.

- **Gate:** the public-API row, plus the loom models.
- **`api/` diff:** new `Hash`, `Eq`, `Ord`, `PartialOrd`, `Display`,
  `From` and `AsRef` impls; `#[non_exhaustive]` on twelve types; the `Send`
  impls of `Divider` and `pert_ui::Svg` removed.
- **Breaking:** yes, for struct literals and exhaustive matches of the
  twelve types and for code sending `Divider` or `Svg`.
- **CHANGELOG:** "Added" for the impls; `Breaking:` for
  `#[non_exhaustive]` and `!Send`.
- **Version:** 0.3.59.

Closes 12 audit findings, 0 round 1 findings and 4 decisions.

Audit findings:

- **G4-2:** `Hash` on the identities, keys and tokens listed, consistent with their `Eq` (`RequestTicket<K>` when `K: Hash`); no `Ord` added.
- **G4-3:** The missing `Eq`/`Ord`/`PartialOrd`/`Hash` on `Capacity`, `PowerExponent`, `Magnification`, `UiZoom`, `ViewScale`, `PresentationRate`, `MaskImage` and `RasterImage`, as `NormalizedValue` does.
- **G4-4:** No change: not adopted (D:G4-4).
- **G4-5:** `impl Display for Color` writing `to_hex()`'s text, documented as quantized sRGB.
- **G4-6:** Private `PhantomData<Rc<()>>` marker in `Divider` and `pert_ui::Svg`, a `compile_fail` doctest each, and the contracts sentence that descriptions are not `Send`.
- **G4-8:** `#[non_exhaustive]` on `InspectionSnapshot`, `ComponentInspection` and `ValidationCode`, with the contracts compatibility sentence.
- **G4-9:** `#[non_exhaustive]` on the other runtime-built records and `EventKind`; the rest stay closed; `serialization.rs:479-516` literals become JSON fixtures; authoring records say to build from `Default`.
- **G4-11:** `Eq` on `pert_ui_cpu::CpuRenderError`, `Clone` and `PartialEq` on `RedescriptionError` (`conformance.rs:12`).
- **G4-27:** `impl AsRef<str> for ChoiceId`.
- **G4-28:** `From<Vec<u8>>` and `From<&'static [u8]>` for `FontData`, beside the named constructors.
- **G4-29:** `From<Tab>`, `From<Section>`, `From<Property>` and `From<FloatingWindow>` for `Element`.
- **G4-30:** `From<Self>` into the raw type for `NormalizedValue`, `Capacity`, `Length`, `Magnification`, `ViewScale` and `PresentationRate`; the `get()` rule mentions it.

Decisions:

- **D:G4-4:** Applied: not adopted; no `Eq` sweep, no TODO entry (G4-4).
- **D:G4-6:** Applied: `!Send` marker on every description now (G4-6).
- **D:G4-8,G4-9:** Applied: runtime-built records only (G4-8, G4-9).
- **D:G4-30:** Applied: six `From` impls (G4-30).

### 6b1 Lints and `#[must_use]`

Adds `clippy::return_self_not_must_use`, `clippy::fn_params_excessive_bools`
and `clippy::missing_panics_doc` to `[workspace.lints.clippy]`, with
`#[expect(clippy::missing_panics_doc, reason = …)]` on
`ThemeMetrics::for_density` (`runtime/theme.rs:255`), and lists them in
AGENTS.md's lint sentence. The `#[must_use]` findings use the receipt rule
of G2-8. `unreachable_pub` and `clippy::struct_excessive_bools` stay off, as
the survey recommends.

- **Gate:** the public-API row, with Clippy in every CI feature
  configuration, since a lint can fire under one feature only.
- **`api/` diff:** none expected; the listing does not show `#[must_use]`.
- **Breaking:** no; consumers may see new `unused_must_use` warnings.
- **CHANGELOG:** "Changed".
- **Version:** 0.3.60.

Closes 5 audit findings, 1 round 1 finding and 1 decision.

Audit findings:

- **G2-7:** Type-level `#[must_use]` removed from `CommandId` (`runtime/commands.rs:379`) and `HeldInputId` (`runtime/held.rs:7`); the guide/diagnostics.md:36-40 example compiles without a warning.
- **G2-8:** `ToastReceipt` (`notifications.rs:195`) loses `#[must_use]`; `TaskReceipt`'s reason names the hidden `Cancelled`; receipt rule in the naming rules; `EventContext::defer` line; guide/surfaces.md:314 and gallery `surfaces.rs:383`, `:393` become plain statements.
- **G2-9:** `#[must_use = …]` with reasons on `ParameterHost`, `AnyParameterHost`, `SeriesSource` (the last clone closes the bridge) and `Runtime::series_source`.
- **G2-10:** Function-level `#[must_use]` on `Runtime::signal`, `StateOwner::signal`, `EventContext::signal`, `UpdateContext::signal` and `Signal::sibling`.
- **G2-11:** Reasons on `PresentationRequest` (`native_windows.rs:1014`) and `PreparedDetach` (`:244`, dropping leaves the movement uncommitted).

Round 1 findings:

- **F19:** Closed by G2-7.

Decisions:

- **D:G2-8:** Applied: `#[must_use]` only where dropping hides an otherwise unreported outcome (G2-8).

### 6b2 `Debug` for state owners, handles, contexts and backends

The first half of G4-1, the group the audit rated medium: the state owners
and handles an application stores (`ListSelection`, `ListController`,
`ListFilter`, `TableController`, `TreeController`, `FieldHandle`,
`FormStatus` under its 6a2 name, `ContextMenuController`, `ThemeManager`,
`FontManager`, `IconManager`, `UiSettings`, `NumberFormat<T>`,
`FilterTarget`, `ChildInfo`, `Runtime`), the callback contexts
(`EventContext`, `UpdateContext`, `MeasureContext`, `ArrangeContext`,
`DescribeContext`, `CellEdit`, `Painter`) and the backends
(`VectorRasterizer`, `pert_ui_wgpu::Renderer`, `pert_ui::App`). Each is a
hand-written `finish_non_exhaustive` form that names callbacks without
formatting them and puts no `Debug` bound on application values. G4-1
closes in 6b3.

- **Gate:** the public-API row, with Clippy in every CI feature
  configuration.
- **`api/` diff:** an `impl core::fmt::Debug for …` line per type in every
  listing that has the type.
- **Breaking:** no.
- **CHANGELOG:** "Added".
- **Version:** 0.3.61.

Closes no ID.

### 6b3 `Debug` for descriptions; the lint denied

The rest of G4-1, then `missing_debug_implementations = "deny"` in
`[workspace.lints.rust]` and in AGENTS.md's lint sentence.

- **Gate:** the public-API row, with Clippy in every CI feature
  configuration.
- **`api/` diff:** an `impl core::fmt::Debug for …` line per type in every
  listing that has the type.
- **Breaking:** no.
- **CHANGELOG:** "Added".
- **Version:** 0.3.62.

Closes 1 audit finding, 0 round 1 findings and 0 decisions.

Audit findings:

- **G4-1:** Hand-written `finish_non_exhaustive` `Debug` on the remaining types (widget and container descriptions, `Dynamic`, `Each`, `Element`, `pert_ui::Svg`), completing the 113 with 6b2; `missing_debug_implementations` denied.

### 6c1 Error values

Error payloads, sources and variant consolidation. G4-15's decision keeps
only the argument variants it names unit (`GeometryError`'s,
`LayoutError::{InvalidWeight, InvalidRatio}` and
`ParameterError::{InvalidRange, OutOfRange, OffLattice}`); every other
variant in G4-14's row carries the rejected value or identity. A
floating-point payload removes `Eq` from its enum and from any enum that
holds it, which the error policy permits ("where their data permits"); the
CHANGELOG names each lost `Eq` and `Copy`. `LiveError` lives in
`pert-ui-core`'s `live` module; its payload does not touch the protocols,
but the part runs the loom models.

- **Gate:** the public-API row, plus the loom models.
- **`api/` diff:** variant fields of the listed errors; `Copy` impls of
  `LayoutError` and `ParameterError` removed, and any `Eq` impl a float
  payload removes; `LimitError::CaptureSize` removed; `Limit` variants on
  `CpuRenderError` and `CaptureError`.
- **Breaking:** yes.
- **CHANGELOG:** `Breaking:` entries with the new match patterns; the
  changed `Display` texts and `CaptureError`'s serde form noted.
- **Version:** 0.3.63.

Closes 4 audit findings, 1 round 1 finding and 1 decision.

Audit findings:

- **G4-12:** `WorkspaceError::Participant` drops `{source}` from its message (`workspace.rs:643`); `RestoreRejection::{State, Runtime}` gain `#[source]` and drop `: {0}` (`:702`, `:712`).
- **G4-13:** One variant for `FrameImage::MAX_BYTES`: `LimitError::FrameSize` absorbs `CaptureSize`; `CpuRenderError::TooLarge` and `CaptureError::TooLarge` become `Limit(LimitError)`; the CPU reservation failure gets its own variant; `LimitError`'s rustdoc corrected.
- **G4-14:** Identity payloads named in `Display`: `RuntimeError::{AmbiguousKey, DuplicateKey, ChildTypeChanged, ShortcutConflict}`, `LayoutError`'s key variants, `FontError::UnknownIcon`, `ParameterError::{DuplicateOption, UnknownOption}`, `ChoiceError::DuplicateValue`, `ImageError::Io`/`ListingError` paths; rejected values in `LiveError::CapacityExceeded`, `NormalizedValueError::OutOfRange`, `NumberInputError::{InvalidRange, InvalidStep, InvalidPrecision, OutOfRange, OffStep}`, `DrawError::InvalidImage`, `PathError::InvalidParameter`, `FontError::MissingGlyph`, `NumberLocaleError::SameSeparators`, `TextEditError::{InvalidComposition, InvalidSelection}`; only the argument variants D:G4-15 names stay unit.
- **G4-15:** `LayoutError` and `ParameterError` lose `Copy`, `GeometryError` and `ChoiceError` keep it; "Error policy" states the payload rule.

Round 1 findings:

- **F23:** Closed by G4-14's key in `RuntimeError::AmbiguousKey`.

Decisions:

- **D:G4-15:** Applied: `Copy` lost only for identity payloads (G4-15).

### 6c2 Compiler diagnostics

The diagnostics survey's recommendations, which carry no finding ID:
- `#[diagnostic::on_unimplemented]` on `Component`, `ListItems`,
  `TreeItems`, `TableItems`, `ContentRows`, `Clipboard` and `StateOwner`,
  and on the private seal, with notes true for every `Self`; none on
  `IntoElement`, where it never fires.
- The private seal `signal::sealed::Sealed` (`R/signal.rs:86-87`) renamed
  `StateOwnerSeal`, which removes the similar-name note.
- `#[diagnostic::do_not_recommend]` on `impl<E> From<E> for AppError`
  (`F/app.rs:167`), with the accepted error types listed in `AppError`'s
  rustdoc (F11).
- A `compile_fail` doctest proving `StateOwner` cannot be implemented.
- Field aliases on `Modifiers` and its rustdoc example (`S:m06`), with no
  alias on `PRIMARY`.
- A new `publish = false` workspace member, `tests/diagnostics` (package
  `pert-ui-diagnostics`, `[lints] workspace = true`), whose only
  dev-dependency is `trybuild`. It holds one test function with the
  snapshot cases whose message pert-ui shapes: c01, c06, i03, l01, l05, t01,
  b01, r01, k01, s01, s02 and m03/m04, each a `ui/*.rs` file with a
  committed `.stderr`.
- **How ordinary runs exclude it:** the test function is
  `#[ignore = "compiler wording; run by the diagnostics job on the pinned
  toolchain"]`. `cargo test --locked --workspace`, the release-profile
  `cargo test --locked --release --workspace`, the stable matrix on Linux,
  macOS and Windows and the MSRV job therefore compile it and skip it.
  Clippy with `--all-targets` still lints it.
- **The dedicated job:** a separate CI job on Linux with the toolchain
  `rust-toolchain.toml` pins runs
  `cargo test --locked -p pert-ui-diagnostics -- --ignored`. A toolchain
  bump regenerates the snapshots with `TRYBUILD=overwrite` and the same
  command, and the `.stderr` diff is reviewed like the `api/` listing.
- **Kept in step in this commit:** `.github/workflows/ci.yml` (the new job,
  and `--exclude pert-ui-diagnostics` in the packaging step); AGENTS.md's
  gate table, the workspace-gate commands (the new command) and the
  packaging command `cargo package --locked --workspace --exclude
  pert-ui-gallery --exclude pert-ui-native-checks --exclude
  pert-ui-diagnostics`; the kit's packaging command in
  `plans/usability/run.sh:81`, which gains the same `--exclude`; and
  `docs/verification.md`, which also says that a `compile_fail` doctest
  does not check the error code on stable.

Records for the part:

- **Gate:** the dependency row (a new dev-dependency, a workspace member
  and a CI job): the workspace gate, the release-profile tests, the MSRV,
  every supported feature configuration AGENTS.md lists, the API listing
  gate, library packaging with the new exclusion and the packaged-consumer
  check, and the dedicated diagnostics command run locally on the pinned
  toolchain.
  `trybuild` is not in the local registry cache, so fetching it needs
  network access.
- **`api/` diff:** `StateOwner`'s supertrait path changes to the renamed
  seal; the attributes do not show.
- **Breaking:** no; the seal is private.
- **CHANGELOG:** "Changed" for the messages; the new check described under
  tooling.
- **Version:** 0.3.64.

Closes 0 audit findings, 1 round 1 finding, 0 decisions and 1 survey case.

Round 1 findings:

- **F11:** Closed by `#[diagnostic::do_not_recommend]` on `impl<E> From<E> for AppError` (`F/app.rs:167`), the accepted error types listed in `AppError`'s rustdoc, and G4-21's `DrawError` route.

Survey cases:

- **S:m06:** no alias on `Modifiers::PRIMARY`, because routing `CONTROL` to it would mislead on macOS, where it means Command (survey.md:463); rustdoc-search aliases (`CONTROL`/`CTRL`, `ALT`/`OPTION`, `SUPER`/`META`/`WINDOWS`) on the matching fields; `Modifiers`' rustdoc says there are no per-key constants, shows `Modifiers { control: true, ..Modifiers::default() }`, and says `PRIMARY` is Control on Windows and Linux and Command on macOS; no trybuild case, since rustc's message does not change.

### 6d1 Redescription and reconciliation

Behavior of retained components under redescription.

- **Gate:** the public-API row (scheduling and identity), with contract
  tests for replace, remove and revocation, and G2-13's dependency test.
  If G2-13 needs a scheduling fix, release-profile tests run too.
- **`api/` diff:** none expected; a scheduling fix would be private.
- **Breaking:** in behavior: redescribed collections follow new permissions
  and callbacks.
- **CHANGELOG:** `Breaking:` behavior entries saying what now follows a
  redescription; AGENTS.md's reconciliation sentence changes with G2-13.
- **Version:** 0.3.65.

Closes 5 audit findings, 0 round 1 findings and 2 decisions.

Audit findings:

- **G1-1:** `reconcile` keeps the `name`/`selected` caches (`choice_controls.rs:939-943`) until the new binding's list validates in `update` (`:1010`), still cancelling input aimed at the old binding; tests for the three wrappers.
- **G2-2:** `VirtualList`, `Table`, `Tree` and `FileBrowser` adopt a compatible redescription's permissions and `folder_filter`, including removal; `audit_collection_permissions.rs:685` flipped.
- **G2-12:** `RegionLayout` installs the forwarding `SplitPane::on_resize` (`region_layout.rs:363-372`) only while it has a callback; a remove-then-reinstall-before-flush test.
- **G2-13:** A test first: a component and a `Computed` reading the snapshot settle in the reconciling flush; then the lifecycle exception in contracts.md:166-167, `number_inputs.rs:1260-1261` and AGENTS.md, or a settle-loop fix; `ContextMenu`'s `enabled_signal` (`menu.rs:1472`) becomes private non-signal state.
- **G2-14:** The collections adopt a redescription's callbacks, including removal, as `Tabs` does; a removed `reorderable` or a fixed denial cancels an active reorder; replace and remove tests.

Decisions:

- **D:G2-2,G2-14:** Applied: permissions and callbacks follow; controller, source and structure stay (G2-2, G2-14).
- **D:G2-13:** Applied: the dependency test decides doc-only exception or scheduling fix; `ContextMenu` private state (G2-13).

### 6d2 Editors, data and defects

Behavior fixes with regression tests, and the signature changes they need.

- **Gate:** the public-API row (persistence for G1-4 and V3-2, identity for
  F26 and F31), with each regression test.
- **`api/` diff:** `Workspace::set_settings` returns `Result` and is no
  longer `const`; `TextSelection::new` returns `Result`; `CaptureJob`'s
  fields private with accessors; new `UiMessage` and `StringRole`
  variants.
- **Breaking:** yes: those signatures, and behavior (number inputs call
  `on_rejection` in new cases, F21's descriptions, F22's chords, F29's
  empty label height, F26's field address).
- **CHANGELOG:** `Breaking:` entries; "Fixed" for the defects.
- **Version:** 0.3.66.

Closes 7 audit findings, 7 round 1 findings and 1 decision.

Found during 5b, not from the audit or round 1:

- **N:focus-reveal:** `Runtime::focus_with_reveal` (`runtime.rs`) returns a
  failing `reveal_focus` with `?` while a `focus_lost` or
  `focus_within_changed` error may be held in `focus_result`, and so drops
  that error, neither returned nor queued in `focus_errors`. Queue the held
  error before returning the reveal error, update `Runtime::focus`'s
  `# Errors` ("in place of"), and add a regression test.
- **N:publish-revision** (found during 6d1): `Signal::publish_trusted`
  publishes without advancing `revision`, so `Dynamic`, `Show` and `Each` over
  a library-published signal never redescribe. Advance `revision` there and
  add a regression test with a `Dynamic` over `Parameter::snapshot()` during a
  gesture.

Audit findings:

- **G1-4:** `Workspace::set_settings` (`workspace.rs:552`) returns `Result<(), WorkspaceError>` with `TooLarge`, keeping the previous settings; boundary round-trip test.
- **G1-11:** `TextSelection::new` (`text_selection.rs:43`) returns `Result<Self, TextEditError>`, rejecting offsets past the text or inside a cluster as `InvalidSelection`; backward selections stay valid.
- **G2-20:** `IntegerInput`/`DecimalInput` report parse and range refusals through `on_rejection` with localized `InputRejection`s (new `UiMessage`/`StringRole` variants); the one coverage rule in contracts; `number_inputs.rs:2086-2095` fixed.
- **G4-25:** `describe_component` (`runtime/inspect.rs:145-161`) clears a protected `value`, sorts and deduplicates actions and clears them when disabled, for `Semantics` and `SemanticItem`.
- **G4-26:** `CaptureJob`'s fields private with accessors (`capture.rs:531-539`); `capture_scene` looks the request up by id and rejects stale or foreign jobs (`runtime/capture.rs:296-344`).
- **V3-1:** `TableColumn::MIN_WIDTH` states the saturated-coordinate exception, `table_columns.rs:577` asserts it, the comment at `table.rs:332` corrected.
- **V3-2:** Workspace restore rejects a stored column width below `MIN_WIDTH` (`table.rs:2028`, `:2075`) with the restore path's out-of-range error; rejection test.

Round 1 findings:

- **F6:** Closed by G2-20's `on_rejection` route.
- **F21:** A `Field` with an application error or warning gives its message to the control's description even when the control describes itself (`TextModified`, `text_fields.rs:3520-3523`); test.
- **F22:** A single-line `TextField` leaves Alt/Super+Up/Down to commands off macOS (`text_fields.rs:1078-1104`, `:3179`); `TextField`/`TextArea` rustdoc list the claimed chords per platform; test.
- **F25:** `check_redescription` (`conformance.rs:217`) lays the reference out where it cannot overlap; a viewport test asserts no validation output; its prose points to `Redescribed::component`.
- **F26:** The `CollectionFilter` field wrapper becomes `.slot("filter-field")` (`collection_filter.rs:571-584`), so the documented address holds; workspace persistence checked.
- **F29:** An empty plain `Label` measures zero height, as an empty `Field` block does (`controls.rs:282-315`); rustdoc and test.
- **F31:** Fixed by F26's slot; a test that a collection keyed `filter` and the field have distinct addresses (`tests/collections/collection_filter.rs:769-791`).

Decisions:

- **D:G2-20:** Applied: contracts sentence plus F6's recommended `on_rejection` route (G2-20).

### 6e Prelude and `component` module

Prelude membership by the rule as decided, and a component-authoring
module. Before committing, check that no example, gallery page, guide
chapter, test or the round 2 fixture defines a name the new globs would
make ambiguous. G5-6's prelude-page list drops the names that now join.

- **Gate:** the public-API row.
- **`api/` diff:** `pub mod pert_ui_runtime::component` and
  `pub mod pert_ui::component` with their re-exports; new `pub use` lines
  in both preludes.
- **Breaking:** no.
- **CHANGELOG:** "Added", listing every name.
- **Version:** 0.3.67.

Closes 24 audit findings, 1 round 1 finding and 5 decisions.

Audit findings:

- **G5-7:** Runtime prelude gains `Modifiers`, `KeyChord`, `CommandKey`, `CommandScope`, `CommandId`, `CommandReceipt`, `CommandStatus`, `CommandSource`, `CommandContext`, `CommandEntry`, `CommandOutcome`.
- **G5-8:** Prelude gains `HeldInputId` and `HeldCancellation`.
- **G5-9:** Prelude gains `ListDrop`, `ListDropPosition`, `ListDropPoint`, `ListDropPlacement`, `ListDropMark`.
- **G5-10:** Prelude gains `LevelMeter`, `Trace`, `MeterAxis`.
- **G5-11:** No change to membership: the exclusion sentence keeps a live widget's `SeriesSource` and its bridge at the root.
- **G5-12:** Prelude gains `PowerExponent`, `EditOutcome`, `PreviewOutcome`, `ParameterSnapshot`, `EditToken`, `ParameterValue`, `Revision`, `ParameterFamily`.
- **G5-13:** Prelude gains `AnyParameter`, `AnyParameterHost`, `AnySpec`, `AnyValue`.
- **G5-14:** No change: `EditSource` stays at the root under G5-26's rule.
- **G5-15:** Prelude gains `DeliveryStatus` and `AuthorityStatus`.
- **G5-16:** Prelude gains `DeliveryMonitor`, `DeliverySnapshot`, `TerminalEvent`, `PreviewEvent`, `SinkError`, `SinkErrorKind`, `DeliveryResolution`; the "hosts" sentence reworded.
- **G5-17:** Prelude gains `ViewScale`, `QuarterTurn`, `OpaqueColor`.
- **G5-18:** Prelude gains `MaskImage`, `ImageId`, `MaskId`, `VectorId`, `VectorGeometry`, `TextLayout`.
- **G5-19:** Prelude gains `FontManager`, `IconManager`, `IconGlyph`.
- **G5-20:** Prelude gains `FilterCounts`, `FilterOutcome`, `FilterTarget`.
- **G5-21:** Prelude gains `EditorSession`, `EditorStart`.
- **G5-22:** Prelude gains `ListingErrorKind`, `LinkTarget`.
- **G5-23:** Prelude gains `WorkspaceSettings`, `WorkspaceValue`, `IgnoredEntry`.
- **G5-24:** Prelude gains `WindowPlacement`.
- **G5-25:** The three guarantee sentences (`R/lib.rs:324-326`, `F/lib.rs:53-55`, `F/root.md:39-40`) point to `component`; 5b narrowed them meanwhile.
- **G5-26:** New `pert_ui_runtime::component` and `pert_ui::component` re-export modules: `Component`, the prelude's component names and the decision's 26 names, plus `EditorSession` and `EditorStart`.
- **G5-27:** Its one-hop names join `component` by the rule (receipts, `TaskStatus`, `NativeWindowOptions`, `NativeWindowId`, `ShortcutBinding`, `ShortcutScope`); `FittedText` not required; `ProfileSnapshot` stays out.
- **G5-28:** `NativeWindowId` in the facade prelude under `native` (`F/lib.rs:71`).
- **G5-30:** `WindowError` in the facade prelude under `native`; the sentence becomes "`App` and everything it reports, errors included".
- **G5-32:** `ApplicationError` joins the runtime prelude under the constructed-error sentence; testing names wait for 6f1's module.

Round 1 findings:

- **F3:** Closed by G5-7 and G5-9, with G5-6's pointer.

Decisions:

- **D:G5-11:** Applied: `SeriesSource` stays at the root, one exclusion sentence (G5-11).
- **D:G5-16:** Applied: seven host names join the prelude (G5-16).
- **D:G5-26,G5-27,G5-14:** Applied: option D, the `component` module; `IgnoredEntry` in the prelude (G5-26, G5-27, G5-14).
- **D:G5-30:** Applied: `WindowError` under `native` (G5-30).
- **D:G5-32:** Applied: no testing module on import grounds; `ApplicationError` joins the prelude (G5-32).

### 6f1 Testing and window close

New public API for headless tests, and the close-path defect. A design
checkpoint comes first, reviewed by Codex and approved by the maintainer:
the `testing` module's types and methods (no window or GPU dependency, so
it lives in the runtime with a facade re-export, and G5-32's capture names
join it), and the headless close entry for `App`. The guide's testing
chapter starts from `Runtime::query` with `Selector::id`, and covers
click, typing, replacing text, chords, settling, list rows, background work
and a CPU capture. F2's doc sub-items are the `inspection_id` example
(switched to `query`), `text_input("")` being a no-op, the caret position
after `focus`, when input is settled, and a `pointer_input` example.

- **Gate:** the dependency, feature, platform and release row, because F4
  changes the native close path: the workspace gate, the release-profile
  tests, the MSRV, every supported feature configuration AGENTS.md lists,
  the API listing gate, library packaging and the packaged-consumer check,
  the packaged examples' headless sessions (F34 changes them), and the
  native checks plus a hand check of closing a window with a focused draft
  on Linux, with the platform recorded. Windows and macOS evidence, not
  available here, is recorded as missing in the native evidence record.
- **`api/` diff:** the `testing` module's items in the runtime and facade
  listings; `App`'s headless close entry.
- **Breaking:** in behavior: a close request commits valid focused drafts
  before `App::on_close` runs.
- **CHANGELOG:** "Added" for the module and entry; `Breaking:` for F4's
  behavior.
- **Version:** 0.3.68.

Closes 0 audit findings, 4 round 1 findings and 0 decisions.

Round 1 findings:

- **F2:** A `testing` module (runtime and facade) with a test driver: click a component, replace a field's text, key chord, semantics of a component through `Runtime::query` with `Selector::id`, settle, wait for background work; a "Testing an application" guide chapter; the five doc sub-items.
- **F4:** The native close path (`F/platform/mod.rs:670-672`, `platform/windows.rs:252-254`) clears focus before `App::on_close`, committing valid drafts as deactivation does; documented in `App::on_close` and contracts; tested through F5.
- **F5:** A headless entry that runs `App`'s `on_close` policy for a window and returns its decision; `Runtime::request_close` and `take_presentation_requests` say how a non-`App` host completes a close.
- **F34:** The packaged examples' sessions use F2's chord and settle helpers (`inventory/session.rs:54-93`, `log_explorer/session.rs:50-88`), leaving one recipe; the three-flush settle (lead U2) is checked first.

### 6f2 Editing and collection capabilities

Capabilities the round 1 testers had to build themselves. A design
checkpoint comes first, as in 6f1. `CollectionFilter`'s default keeps both
documented branches (`collection_filter.rs:438-443`): with focus in the
collection a hidden active item moves to the nearest remaining row, and
without focus it stays hidden. The new policy applies only to the unfocused
branch.

- **Gate:** the public-API row.
- **`api/` diff:** `InlineText::session` and the list-row mode builder;
  `CollectionFilter`'s policy builder and its enum; `EventContext::spawn`.
- **Breaking:** no.
- **CHANGELOG:** "Added".
- **Version:** 0.3.69.

Closes 0 audit findings, 4 round 1 findings and 0 decisions.

Round 1 findings:

- **F7b:** `CollectionFilter` gains an opt-in policy for a filter change that hides the active item while the collection lacks focus (default keeps today's rule: it stays hidden; options move it to `nearest_active` or clear it); the focused branch (`collection_filter.rs:438-443`) is unchanged; tests for focused and unfocused filtering under every policy.
- **F13:** `EventContext::spawn`, starting background work from a callback as `Runtime::spawn` does; 5c's guide example switches to it.
- **F15:** `InlineText::session(&EditorSession)` lets code begin a row's rename; `Runtime::key_input` says a synthesized press of a held key is a repeat.
- **F27:** An `InlineText` list-row mode: a single press reaches the row, a press on the selected row or a double-click edits; the default press behavior documented.

### 6f3 Controlled controls

F16's controlled `Checkbox` and `InlineText`. A design checkpoint comes
first, reviewed by Codex and approved by the maintainer: contracts.md
specifies refusal, optimistic presentation and callback timing before any
code, and the controlled forms take a separate exclusive constructor (for
example `controlled`). The existing `Checkbox::new` and `InlineText::new`,
which take bindings, stay as they are, so the part is additive; the
`new`/`bound`/`for_parameter` vocabulary stays limited to choice-option
ownership.

- **Gate:** the public-API row.
- **`api/` diff:** the controlled constructors and callbacks of `Checkbox`
  and `InlineText`.
- **Breaking:** no.
- **CHANGELOG:** "Added".
- **Version:** 0.3.70.

Closes 0 audit findings, 1 round 1 finding and 0 decisions.

Round 1 findings:

- **F16:** Controlled forms of `Checkbox` and `InlineText` (a value plus a change callback) on their own constructor path, with refusal, optimistic presentation and callback timing specified in contracts.md first; `ListItems::build` points to them.

## Plans and records this plan changes

- **App-author slice 9** (`plans/app-author-improvements.md`, slice 9 and
  its inputs). Each part that fixes an input removes it there:
  - F1 (5c), F9 (5c), F14 (5c), F20 (5d), F3 (6e), F2 (6f1), F13 (6f2) and
    F16 (6f3) leave the input list.
  - Slice 9's bullet "a chapter on headless testing and CPU captures" is
    met by 6f1.
  - The mixer-probe names: `CommandScope`, `KeyChord`, `CommandSource`,
    `LevelMeter`, `Trace`, `MeterAxis` and `EditOutcome` join the prelude in
    6e; `series_bridge`, `SeriesConfig`, `SeriesSource`, `Delivery` and
    `Timestamp` stay at the root by G5-11, which leaves a thematic
    live-data module to slice 9.
  - Slice 9's "smaller application prelude": 6e grows the prelude by about
    seventy names. Slice 9 keeps the decision to shrink it on its own
    breaking branch.
- **Follow-ups** (`plans/follow-ups.md`):
  - "Sessions for more editors": 6f2 removes `InlineText` and the F15
    evidence; the other editors stay.
  - "Controlled controls": 6f3 narrows it to the controls F16 does not
    cover, or removes it if none remain.
  - 5b adds a sendable font key for styled text (G4-7).
  - Destinations that never became entries need nothing: F2's helper type,
    F5's driver, F7b's policy, F13's convenience, F27's row mode and F24's
    variant.
- **TODO.md:** no entry is resolved by this plan. Entries touching files the
  parts change, to mention in each part's report:
  - TEXT-01 (`text_fields.rs`, the shared `MAX_TEXT_BYTES`) in 5b, 6a2 and
    6d2;
  - TOOL-01 (`tool_windows/mod.rs`) in 6a1 and 6a2;
  - the `docs/verification.md` gate-block entry in 6c2, which changes the
    gate;
  - NATIVE-01 to NATIVE-05, beside which 6a2 and 6f1 record their missing
    Windows and macOS evidence;
  - the `FrameImage::encode_png` entry in 6c1 and 6d2 (`capture.rs`).

  G4-22 asks to update F30's TODO.md entry, but none exists, so nothing is
  updated.
- **The developer-usability plan:** slices 5 and 6 follow this plan. The
  findings' "If not fixed" and "Destination" columns no longer apply once a
  part closes them. Slice 7 measures the result.

## Open points

Choices this plan makes where a finding offered alternatives and no decision
chose, for the maintainer to confirm or change before the part starts:
- **G1-4:** a fallible `set_settings`, not reserved room in `new`/`decode`,
  which would reject documents that load today.
- **G1-11:** `TextSelection::new` becomes fallible rather than gaining a
  checked twin.
- **G1-18:** `SplitSizing::rule` loses `const`, rather than adding a public
  const constructor to `NormalizedValue`.
- **G3-10:** the getters are renamed too, not only the parameters.
- **G3-14:** the new name for `FormStatus`; `FormFields` is a placeholder.
- **G3-15:** `PendingJump` keeps its name and exposes its receipt.
- **G4-13:** a `Limit(LimitError)` payload rather than a policy exception.
- **G4-25:** normalizing in `describe_component` rather than private
  fields.
- **F18:** the documentation follows the runtime; Escape keeps not calling
  `cancel_input`.
- **F21, F22, F29:** behavior changes rather than documentation only.
- **F7b:** the policy applies only when the collection lacks focus; the
  focused branch keeps moving the active item.
- **F26 and F31:** the wrapper becomes a slot, rather than reserving the
  key `filter`.

Not placed, because no decision or finding asks for them:
- `clippy::must_use_candidate` (79 getters): a getter policy the survey
  leaves to the maintainer.
- New `Modifiers::SHIFT`/`CONTROL`/`ALT`/`SUPER` constants, which triage
  suggested for follow-ups: `S:m06` answers those guesses with field
  aliases and an example instead.
- Proposals inside rows the plan does not adopt: `Borrow<str>` for
  `ChoiceId` (G4-27), a `previous` field in `ExpansionChange` (G2-16), and a
  two-dimensional `SemanticValue` variant (F24), which would raise
  `InspectionSnapshot::SCHEMA_VERSION`.
- Round 1 entries the triage classified as kit issues or tester errors are
  outside the F table and keep their records in the runs' README.

## Coverage

All 218 IDs, sorted by ID. `D:` rows are decisions; `S:` rows are
diagnostics-survey cases.

| ID | Part | Action |
|---|---|---|
| D:G1-3 | 6a1 | Applied: owner `ui`, key, bound state, then content (G1-3, with G1-2). |
| D:G1-5 | 5a | Applied: string exception `as_str()` (G1-5). |
| D:G1-6 | 5a | Applied: `get()` for single-value types only (G1-6). |
| D:G1-9 | 6a1 | Applied: placement rule, reasons and `PowerExponent` constants as `Self` (G1-9). |
| D:G1-10 | 5a | Applied: only chosen limits get constants (G1-10). |
| D:G1-12 | 5a | Applied: the AGENTS.md clamp wording and its link (G1-12). |
| D:G1-13 | 5a | Applied: three documented meanings of `with_` (G1-13). |
| D:G1-14 | 6a1 | Applied: rename to `Color::from_linear` (G1-14). |
| D:G1-15 | 5b | Applied: four parameters renamed, no API listing change (G1-15). |
| D:G1-16 | 5a | Applied: one "Choice sources" sentence (G1-16). |
| D:G1-18 | 6a1 | Applied: `NormalizedValue` payload, `rule` no longer `const` (G1-18). |
| D:G2-1 | 5a | Applied: the widened no-argument rule (G2-1). |
| D:G2-2,G2-14 | 6d1 | Applied: permissions and callbacks follow; controller, source and structure stay (G2-2, G2-14). |
| D:G2-3 | 5a | Applied: fixed window parts documented (G2-3). |
| D:G2-4 | 5a | Applied: data descriptions take `bool` (G2-4). |
| D:G2-5 | 5a | Applied: appending builders documented (G2-5). |
| D:G2-6 | 6a1 | Applied: `bind_<property>` for the three two-way builders (G2-6). |
| D:G2-8 | 6b1 | Applied: `#[must_use]` only where dropping hides an otherwise unreported outcome (G2-8). |
| D:G2-13 | 6d1 | Applied: the dependency test decides doc-only exception or scheduling fix; `ContextMenu` private state (G2-13). |
| D:G2-17 | 5a | Applied: both names kept, difference documented (G2-17). |
| D:G2-18 | 5b | Applied: `filter` kept with a `FilterMatcher` doctest (G2-18). |
| D:G2-19 | 6a1 | Applied: `impl Fn` signature (G2-19). |
| D:G2-20 | 6d2 | Applied: contracts sentence plus F6's recommended `on_rejection` route (G2-20). |
| D:G3-1 | 6a2 | Applied: option C, eight renames, residual risk documented (G3-1). |
| D:G3-2 | 5a | Applied: the rule the code follows (G3-2). |
| D:G3-3 | 6a2 | Applied: noun accessor pairs (G3-3). |
| D:G3-4 | 6a2 | Applied: `downcast_ref` (G3-4). |
| D:G3-5 | 6a2 | Applied: `EditorSession::is_active` (G3-5). |
| D:G3-7 | 6a2 | Applied: logical-length convention, two width builders (G3-7). |
| D:G3-12 | 6a2 | Applied: `ListDropPosition`, `PickStatus` kept (G3-12). |
| D:G3-16 | 5a | Applied: `*State` by ownership (G3-16). |
| D:G3-17 | 5a | Applied: the suffixes named in the rule (G3-17). |
| D:G3-18 | 5a | Applied: the one-directional `*Outcome` rule (G3-18). |
| D:G3-19 | 6a2 | Applied: coordinator sentence, landing with G3-14's rename (G3-19). |
| D:G3-21 | 5a | Applied: a cross-thread reply is not a receipt (G3-21). |
| D:G3-23 | 6a2 | Applied: `MAX_TEXT_BYTES` (G3-23). |
| D:G3-24 | 6a2 | Applied: `TaskStatus::Succeeded` (G3-24). |
| D:G4-4 | 6a3 | Applied: not adopted; no `Eq` sweep, no TODO entry (G4-4). |
| D:G4-6 | 6a3 | Applied: `!Send` marker on every description now (G4-6). |
| D:G4-7 | 5b | Applied: UI-thread restriction documented, sendable key to follow-ups (G4-7). |
| D:G4-8,G4-9 | 6a3 | Applied: runtime-built records only (G4-8, G4-9). |
| D:G4-10 | 5a | Applied: std-idiom exception in "Error policy" (G4-10). |
| D:G4-15 | 6c1 | Applied: `Copy` lost only for identity payloads (G4-15). |
| D:G4-19,G4-20 | 5a | Applied: one-line forwarding and type-level `# Serialization` (G4-19, G4-20). |
| D:G4-21 | 5b | Applied: exclusion kept, `DrawError` route documented (G4-21). |
| D:G4-22 | 5b | Applied: no new `From`, doc pointers (G4-22). |
| D:G4-23 | 5a | Applied: verbatim-text exception (G4-23). |
| D:G4-30 | 6a3 | Applied: six `From` impls (G4-30). |
| D:G5-11 | 6e | Applied: `SeriesSource` stays at the root, one exclusion sentence (G5-11). |
| D:G5-16 | 6e | Applied: seven host names join the prelude (G5-16). |
| D:G5-26,G5-27,G5-14 | 6e | Applied: option D, the `component` module; `IgnoredEntry` in the prelude (G5-26, G5-27, G5-14). |
| D:G5-30 | 6e | Applied: `WindowError` under `native` (G5-30). |
| D:G5-32 | 6e | Applied: no testing module on import grounds; `ApplicationError` joins the prelude (G5-32). |
| F1 | 5c | guide/getting-started.md:9-19 leads with `pert-ui = { version = "…" }` kept in step by a test (as `tests/foundation/readme.rs` does), the checkout as the alternative, the MSRV from status.md, and that examples need a checkout. |
| F2 | 6f1 | A `testing` module (runtime and facade) with a test driver: click a component, replace a field's text, key chord, semantics of a component through `Runtime::query` with `Selector::id`, settle, wait for background work; a "Testing an application" guide chapter; the five doc sub-items. |
| F3 | 6e | Closed by G5-7 and G5-9, with G5-6's pointer. |
| F4 | 6f1 | The native close path (`F/platform/mod.rs:670-672`, `platform/windows.rs:252-254`) clears focus before `App::on_close`, committing valid drafts as deactivation does; documented in `App::on_close` and contracts; tested through F5. |
| F5 | 6f1 | A headless entry that runs `App`'s `on_close` policy for a window and returns its decision; `Runtime::request_close` and `take_presentation_requests` say how a non-`App` host completes a close. |
| F6 | 6d2 | Closed by G2-20's `on_rejection` route. |
| F7a | 5c | `CollectionFilter` "Selection and focus" and F14's recipe show how to tell that the selected row is hidden (`ListFilter::items()`, `ListItems::index_of`, `ListSelection::active`). |
| F7b | 6f2 | `CollectionFilter` gains an opt-in policy for a filter change that hides the active item while the collection lacks focus (default keeps today's rule: it stays hidden; options move it to `nearest_active` or clear it); the focused branch (`collection_filter.rs:438-443`) is unchanged; tests for focused and unfocused filtering under every policy. |
| F7c | 5b | `FilterMatcher`, `ListFilter::filter`, `TableController::filter` and `TreeController::filter` say a non-blank query is trimmed before the matcher sees it, and point to `ListFilter::items()`. |
| F8 | 5b | `Semantics::value`, `SemanticValue::Text` and `Label` say static text is the name and the value is an editable or stateful control's content. |
| F9 | 5c | `Each`'s example adds a lookup through the default `items` segment (`described.rs:245-266`). |
| F10 | 5c | guide/diagnostics.md "Screenshots" shows `CaptureTarget::window(&ui.primary_window())`. |
| F11 | 6c2 | Closed by `#[diagnostic::do_not_recommend]` on `impl<E> From<E> for AppError` (`F/app.rs:167`), the accepted error types listed in `AppError`'s rustdoc, and G4-21's `DrawError` route. |
| F12 | 5b | `Component::cancel_input` states that it runs outside evaluation phases, may restore the widget's own bindings, must not activate actions, and that a failed restore is recorded and returned through a `Notifier`; guide "Who may write" lists it. |
| F13 | 6f2 | `EventContext::spawn`, starting background work from a callback as `Runtime::spawn` does; 5c's guide example switches to it. |
| F14 | 5c | A guide recipe for an editable, selectable, filterable list of records (`Vec` to `ListSource`, `ListFilter`, `VirtualList` with `ListSelection`, `on_activate`, `reorderable`, per-row controls), linked from "Children from values". |
| F15 | 6f2 | `InlineText::session(&EditorSession)` lets code begin a row's rename; `Runtime::key_input` says a synthesized press of a held key is a repeat. |
| F16 | 6f3 | Controlled forms of `Checkbox` and `InlineText` (a value plus a change callback) on their own constructor path, with refusal, optimistic presentation and callback timing specified in contracts.md first; `ListItems::build` points to them. |
| F17 | 5b | Owner-contract bullets: an owner described again returns `true` from `retain_initial_children` or reconciles controller and child together; `begin` runs the owner's `cancel_input`, so a draft ends in `focus_within_changed(false)`. |
| F18 | 5b | guide/custom-widgets.md:86-89 and guide/controls.md:11 corrected to what Escape does (the focused component's capture, timer and edit sessions end, then the key arrives; no `cancel_input`); `Runtime::key_input` and `EventContext::set_timer` say so. |
| F19 | 6b1 | Closed by G2-7. |
| F20 | 5d | Closed by G5-1's sweep. |
| F21 | 6d2 | A `Field` with an application error or warning gives its message to the control's description even when the control describes itself (`TextModified`, `text_fields.rs:3520-3523`); test. |
| F22 | 6d2 | A single-line `TextField` leaves Alt/Super+Up/Down to commands off macOS (`text_fields.rs:1078-1104`, `:3179`); `TextField`/`TextArea` rustdoc list the claimed chords per platform; test. |
| F23 | 6c1 | Closed by G4-14's key in `RuntimeError::AmbiguousKey`. |
| F24 | 5b | `SemanticValue` and guide/custom-widgets.md "Inspection": a multi-component value is `Text` plus one `InspectValue::Number` property per component, as `ColorPicker` does. |
| F25 | 6d2 | `check_redescription` (`conformance.rs:217`) lays the reference out where it cannot overlap; a viewport test asserts no validation output; its prose points to `Redescribed::component`. |
| F26 | 6d2 | The `CollectionFilter` field wrapper becomes `.slot("filter-field")` (`collection_filter.rs:571-584`), so the documented address holds; workspace persistence checked. |
| F27 | 6f2 | An `InlineText` list-row mode: a single press reaches the row, a press on the selected row or a double-click edits; the default press behavior documented. |
| F28 | 5c | guide/state.md says signal values need `PartialEq` to be written and `Clone` to be read. |
| F29 | 6d2 | An empty plain `Label` measures zero height, as an empty `Field` block does (`controls.rs:282-315`); rustdoc and test. |
| F30 | 5b | Closed by G4-22's rustdoc pointers; the source loss was already resolved. |
| F31 | 6d2 | Fixed by F26's slot; a test that a collection keyed `filter` and the field have distinct addresses (`tests/collections/collection_filter.rs:769-791`). |
| F32 | 5b | The hide and removal paths are probed again and `ColorPicker::cancel_input`'s comment (`color_picker.rs:1164-1167`) corrected. |
| F33 | 6a1 | `SizeLimits::exact(Size)`, a `const`, infallible constructor for a fixed size (`layout.rs`). |
| F34 | 6f1 | The packaged examples' sessions use F2's chord and settle helpers (`inventory/session.rs:54-93`, `log_explorer/session.rs:50-88`), leaving one recipe; the three-flush settle (lead U2) is checked first. |
| G1-1 | 6d1 | `reconcile` keeps the `name`/`selected` caches (`choice_controls.rs:939-943`) until the new binding's list validates in `update` (`:1010`), still cancelling input aimed at the old binding; tests for the three wrappers. |
| G1-2 | 6a1 | Bound state first after `ui`: `Checkbox::new`/`Toggle::new(value, label)` (macro `choice_controls.rs:525`), `TableController::new(ui, selection, columns, source)`, `TreeController::new(ui, selection, source)`, `FileBrowserController::new(ui, selection, roots, request)`. |
| G1-3 | 6a1 | `MenuItem::toggle(key, value, label)` and `MenuItem::radio(key, selection, value, label)` (`menu.rs:299`, `:333`); the naming-rule sentence on parameter order. |
| G1-4 | 6d2 | `Workspace::set_settings` (`workspace.rs:552`) returns `Result<(), WorkspaceError>` with `TooLarge`, keeping the previous settings; boundary round-trip test. |
| G1-5 | 5a | Naming rules (`docs/contracts.md:1151`): a validated string borrows its text with `as_str()`. |
| G1-6 | 5a | Naming rules: `get()` covers single-value types; a compound value has component accessors and a whole-value accessor named for its content (`channels`, `values`, `entries`). |
| G1-7 | 6a1 | `PresentationRate::MIN`/`MAX` typed `Self` and `ViewState::MAX_NAME_BYTES` (`workspace.rs:235`), linked from the errors and `# Errors`. |
| G1-8 | 6a1 | `VectorRasterizer::MAX_FLATTENED_POINTS` and `MAX_STROKE_EDGES` (`vector_scene.rs:103-107`), backed by the private `MAX_EDGES` (`geometry/raster.rs:11`). |
| G1-9 | 6a1 | Placement rule for validation limits and rendering caps in the naming rules; reasons on `NumberInputError::MAX_PRECISION`, `TextError::MAX_FONT_PIXELS` and `TextLimit::MAX_RASTER_SIDE`; `PowerExponent::MIN`/`MAX` typed `Self`. |
| G1-10 | 5a | Naming rules: limits the library chooses are constants; intrinsic bounds (nonzero, nonnegative, unit interval) are stated on the constructor and its `# Errors`. |
| G1-11 | 6d2 | `TextSelection::new` (`text_selection.rs:43`) returns `Result<Self, TextEditError>`, rejecting offsets past the text or inside a cluster as `InvalidSelection`; backward selections stay valid. |
| G1-12 | 5a | AGENTS.md "Rust and data invariants" clamp sentence replaced by the decided wording; the naming rules link it; `ProgressStatus::of` and `TableColumn::width`/`set_column_width` name their clamp kind. |
| G1-13 | 5a | Naming rules: the three meanings of `with_` by receiver; a getter, where one exists, takes the bare name; component descriptions use bare builder names. |
| G1-14 | 6a1 | `Color::rgba` (`drawing.rs:258`) renamed `Color::from_linear` (about 90 call lines); `Color::channels` says linear. |
| G1-15 | 5b | Parameters renamed `runtime` to `ui` in `ListController::new` (`virtual_list.rs:275`) and `ZoomController::new` (`zoom_view.rs:140`), `state` to `controller` in `Dialog::new`/`AlertDialog::new` (`dialog.rs:195`, `:506`), with their prose. |
| G1-16 | 5a | Contracts "Choice sources": collections whose rows are content and keyed containers are outside the `new`/`bound`/`for_parameter` vocabulary. |
| G1-18 | 6a1 | `SplitRule::Ratio(NormalizedValue)` (`split.rs:141-145`); `SplitSizing::rule` stops being `const`, its private storage stays `f32`. |
| G2-1 | 5a | Naming rules: a no-argument builder moves one fixed option away from its default (`disabled()`, `read_only()`, `frameless()`, `nonmodal()`); an option decided from data takes a `bool`. |
| G2-2 | 6d1 | `VirtualList`, `Table`, `Tree` and `FileBrowser` adopt a compatible redescription's permissions and `folder_filter`, including removal; `audit_collection_permissions.rs:685` flipped. |
| G2-3 | 5a | Contracts "Permissions and availability": title-bar parts and `App::resizable` are fixed `bool` part choices; a rustdoc line on `FloatingWindow::closable`/`minimizable` (`windows.rs:327`, `:333`). |
| G2-4 | 5a | Clause in G2-1's sentence: data descriptions take `bool`; `SemanticItem::disabled`/`Semantics::disabled` say `disabled(false)` does not restore withdrawn actions (`diagnostics.rs:392-397`, `:579-585`). |
| G2-5 | 5a | Naming rules: element builders append, whole-list and single-slot builders replace; rustdoc fixes on `Semantics::actions` and `Toast::action` (`notifications.rs:156-163`). |
| G2-6 | 6a1 | `Collapsible::bound` to `bind_expanded`, `ToolWindowLayout::names` to `bind_show_names`, `PropertyGrid::shared_label_width` to `bind_label_width`; the `bind_<property>` naming rule. |
| G2-7 | 6b1 | Type-level `#[must_use]` removed from `CommandId` (`runtime/commands.rs:379`) and `HeldInputId` (`runtime/held.rs:7`); the guide/diagnostics.md:36-40 example compiles without a warning. |
| G2-8 | 6b1 | `ToastReceipt` (`notifications.rs:195`) loses `#[must_use]`; `TaskReceipt`'s reason names the hidden `Cancelled`; receipt rule in the naming rules; `EventContext::defer` line; guide/surfaces.md:314 and gallery `surfaces.rs:383`, `:393` become plain statements. |
| G2-9 | 6b1 | `#[must_use = …]` with reasons on `ParameterHost`, `AnyParameterHost`, `SeriesSource` (the last clone closes the bridge) and `Runtime::series_source`. |
| G2-10 | 6b1 | Function-level `#[must_use]` on `Runtime::signal`, `StateOwner::signal`, `EventContext::signal`, `UpdateContext::signal` and `Signal::sibling`. |
| G2-11 | 6b1 | Reasons on `PresentationRequest` (`native_windows.rs:1014`) and `PreparedDetach` (`:244`, dropping leaves the movement uncommitted). |
| G2-12 | 6d1 | `RegionLayout` installs the forwarding `SplitPane::on_resize` (`region_layout.rs:363-372`) only while it has a callback; a remove-then-reinstall-before-flush test. |
| G2-13 | 6d1 | A test first: a component and a `Computed` reading the snapshot settle in the reconciling flush; then the lifecycle exception in contracts.md:166-167, `number_inputs.rs:1260-1261` and AGENTS.md, or a settle-loop fix; `ContextMenu`'s `enabled_signal` (`menu.rs:1472`) becomes private non-signal state. |
| G2-14 | 6d1 | The collections adopt a redescription's callbacks, including removal, as `Tabs` does; a removed `reorderable` or a fixed denial cancels an active reorder; replace and remove tests. |
| G2-15 | 6a1 | `Button::command`, `Button::command_icon` and `MenuItem::command` take `FnMut(&mut EventContext, CommandReceipt)` (`controls.rs:552`, `:592`; `menu.rs:404`). |
| G2-16 | 6a1 | `Accordion::on_change` takes `FnMut(&mut EventContext, &ExpansionChange)`, a new struct carrying the expanded keys and the `ChangeOrigin` (`collapsible.rs:1049`). |
| G2-17 | 5a | Contracts "Component notifications": `on_select` reports each settled tab change, `on_selection_change` a coalesced bound selection; `VirtualList` rustdoc (`virtual_list.rs:834-836`) rephrased. |
| G2-18 | 5b | Doctest on `MultiSelect::filter` (`multi_select.rs:1382`) reusing `FilterMatcher` presets; it says the query arrives untrimmed. |
| G2-19 | 6a1 | `Runtime::set_wake_delivery` (`runtime.rs:4183`) takes `impl Fn() -> bool + Send + Sync + 'static`; internal callers drop `Box::new`. |
| G2-20 | 6d2 | `IntegerInput`/`DecimalInput` report parse and range refusals through `on_rejection` with localized `InputRejection`s (new `UiMessage`/`StringRole` variants); the one coverage rule in contracts; `number_inputs.rs:2086-2095` fixed. |
| G3-1 | 6a2 | The seven `UiSettings`/`ThemeManager` `*_signal` getters renamed `*_binding`, `DeliveryMonitor::observe` renamed `snapshot_binding`; the owned/reactive naming rule; `Parameter::snapshot` (live) and `AnyParameter::snapshot` (owned) documented. |
| G3-2 | 5a | Contracts "Permissions and availability" (`:296-298`): the adjective restricts a built-in capability by `Permission`, or creates a callback- or flag-supplied one whose restriction is `*_permission`. |
| G3-3 | 6a2 | `ThemeMetrics::get` to `metric`, `UiStrings::get`/`with` to `text`/`with_text` (`runtime/theme.rs:279`; `strings.rs:625`, `:601`). |
| G3-4 | 6a2 | `DragPayload::get::<T>` renamed `downcast_ref::<T>` (`drag.rs:53`). |
| G3-5 | 6a2 | `EditorSession::is_open` renamed `is_active` (`editor_session.rs:188`). |
| G3-6 | 6a2 | `pert_ui_wgpu::Renderer::{new, new_surface, resize}` take `PhysicalExtent` (`W/lib.rs:391`, `:1271`, `:485`); the facade's platform code migrates. |
| G3-7 | 6a2 | `QuickPick::width` and `CommandPalette::width` take `Length` (`quick_pick.rs:975`, `command_palette.rs:611`); the naming-rule sentence on logical lengths. |
| G3-8 | 5b | `AxisMapping` parameters renamed `extent_pixels` and `anchor_pixel` (`axis_mapping.rs:88`, `:101`, `:146`) with their doc links. |
| G3-9 | 6a2 | `TextLayout::{paint, paint_turned}` take a `Point` (`text.rs:262`, `:284`); `fitted_text`, `ListDropPoint::across` and `VirtualAxis` conform to G3-7's sentence. |
| G3-10 | 6a2 | `ResourceKind::scene_budget` to `scene_budget_bytes`, `TextSelection::{anchor, focus, range}` to `{anchor_byte, focus_byte, byte_range}`, `TextPreedit::cursor` to `cursor_byte_range`; byte-named parameters on their constructors and `RichText::highlighted`. |
| G3-11 | 6a2 | New `FocusDirection` for `Runtime::focus_next`, `Component::tab_navigation(cx, direction, transition: KeyTransition)` and `EditorTrigger::Tab`; `EventContext::request_reveal_and_focus`; enums for `TextSelection::{moved, at_point}`'s extension and `ToolWindowChange::new`'s expansion. |
| G3-12 | 6a2 | `DropPosition` renamed `ListDropPosition` (`list_drag.rs:47`); `PickStatus` stays. |
| G3-13 | 6a2 | `ContextMenuOpener` renamed `ContextMenuController`, `ToastQueue` renamed `ToastController`; `FieldHandle` stays under G3-19. The accessor follows the type, `ContextMenu::opener()` becoming `controller()` as `FloatingWindow::controller` is named (added during 6a2; the maintainer is told). |
| G3-14 | 6a2 | `FormStatus` (a cloneable handle, `field_status.rs:214`) renamed to a non-controller `Form…` name, proposed `FormFields`. |
| G3-15 | 6a2 | `PendingJump` keeps its name as a continuation; its forwarders become `receipt(&self) -> &ListRevealReceipt` (`content_search.rs:416-428`). |
| G3-16 | 5a | Naming rules: `*State` names a value its owner holds directly, never a shared handle (`ViewState`, `ParameterState`). |
| G3-17 | 5a | Naming rules: `*Snapshot`, `*Inspection` and `*Report` suffixes, and descriptive nouns for other passive data. |
| G3-18 | 5a | Naming rules: `*Outcome` is the non-error result of an interaction or operation; results may be named for what they deliver and carry a continuation. |
| G3-19 | 6a2 | Naming rules: application-owned coordinators (`ListSelection`, the renamed `FormStatus` and its `FieldHandle`s) are named for what they hold; `*Controller` means widget-owned state. |
| G3-21 | 5a | Naming rules: a cross-thread reply consumed once (`Pending<T>`) is not a receipt. |
| G3-22 | 6a2 | `QuickPick`/`CommandPalette::DEFAULT_WIDTH` typed `Length`; the other read-back `f32` lengths conform to G3-7's sentence and say logical pixels. |
| G3-23 | 6a2 | `UiStrings::MAX_LENGTH` renamed `MAX_TEXT_BYTES` (`strings.rs:576`). |
| G3-24 | 6a2 | `TaskStatus::Completed` renamed `Succeeded`. |
| G3-25 | 6a2 | `NativeCloseAction::Defer(ReadSignal<Option<WindowCloseAction>>)` (`F/app.rs:207-210`); `diagram_editor/app.rs:891`, `:910-911` lose their comment. |
| G4-1 | 6b3 | Hand-written `finish_non_exhaustive` `Debug` on the remaining types (widget and container descriptions, `Dynamic`, `Each`, `Element`, `pert_ui::Svg`), completing the 113 with 6b2; `missing_debug_implementations` denied. |
| G4-2 | 6a3 | `Hash` on the identities, keys and tokens listed, consistent with their `Eq` (`RequestTicket<K>` when `K: Hash`); no `Ord` added. |
| G4-3 | 6a3 | The missing `Eq`/`Ord`/`PartialOrd`/`Hash` on `Capacity`, `PowerExponent`, `Magnification`, `UiZoom`, `ViewScale`, `PresentationRate`, `MaskImage` and `RasterImage`, as `NormalizedValue` does. |
| G4-4 | 6a3 | No change: not adopted (D:G4-4). |
| G4-5 | 6a3 | `impl Display for Color` writing `to_hex()`'s text, documented as quantized sRGB. |
| G4-6 | 6a3 | Private `PhantomData<Rc<()>>` marker in `Divider` and `pert_ui::Svg`, a `compile_fail` doctest each, and the contracts sentence that descriptions are not `Send`. |
| G4-7 | 5b | `RichText` and `TextSpan` rustdoc: UI-thread values because a span can name a `FontId`; a sendable font key added to `plans/follow-ups.md`. |
| G4-8 | 6a3 | `#[non_exhaustive]` on `InspectionSnapshot`, `ComponentInspection` and `ValidationCode`, with the contracts compatibility sentence. |
| G4-9 | 6a3 | `#[non_exhaustive]` on the other runtime-built records and `EventKind`; the rest stay closed; `serialization.rs:479-516` literals become JSON fixtures; authoring records say to build from `Default`. |
| G4-10 | 5a | Contracts "Error policy": a kind-only conversion may return its input as the error, as std's array `TryFrom` does. |
| G4-11 | 6a3 | `Eq` on `pert_ui_cpu::CpuRenderError`, `Clone` and `PartialEq` on `RedescriptionError` (`conformance.rs:12`). |
| G4-12 | 6c1 | `WorkspaceError::Participant` drops `{source}` from its message (`workspace.rs:643`); `RestoreRejection::{State, Runtime}` gain `#[source]` and drop `: {0}` (`:702`, `:712`). |
| G4-13 | 6c1 | One variant for `FrameImage::MAX_BYTES`: `LimitError::FrameSize` absorbs `CaptureSize`; `CpuRenderError::TooLarge` and `CaptureError::TooLarge` become `Limit(LimitError)`; the CPU reservation failure gets its own variant; `LimitError`'s rustdoc corrected. |
| G4-14 | 6c1 | Identity payloads named in `Display`: `RuntimeError::{AmbiguousKey, DuplicateKey, ChildTypeChanged, ShortcutConflict}`, `LayoutError`'s key variants, `FontError::UnknownIcon`, `ParameterError::{DuplicateOption, UnknownOption}`, `ChoiceError::DuplicateValue`, `ImageError::Io`/`ListingError` paths; rejected values in `LiveError::CapacityExceeded`, `NormalizedValueError::OutOfRange`, `NumberInputError::{InvalidRange, InvalidStep, InvalidPrecision, OutOfRange, OffStep}`, `DrawError::InvalidImage`, `PathError::InvalidParameter`, `FontError::MissingGlyph`, `NumberLocaleError::SameSeparators`, `TextEditError::{InvalidComposition, InvalidSelection}`; only the argument variants D:G4-15 names stay unit. |
| G4-15 | 6c1 | `LayoutError` and `ParameterError` lose `Copy`, `GeometryError` and `ChoiceError` keep it; "Error policy" states the payload rule. |
| G4-16 | 5b | `# Errors` sections of every item in the row list the reachable phase errors or link `Signal::set`'s list, per the author's resolutions D2-D4. |
| G4-17 | 5b | `# Errors` sections of every item in the row name their variants, with a shared binding-family paragraph where many share them. |
| G4-18 | 5b | One-line forwarding `# Errors` on `ChoiceId`'s two `TryFrom` impls and the `try_from` of `Magnification`, `UiZoom` and `ViewScale`. |
| G4-19 | 5a | Contracts "Error policy": a `TryFrom`/`FromStr` impl states its failures in one line linking the documenting constructor. |
| G4-20 | 5a | Contracts "Error policy": serde impls are documented in the type's `# Serialization`; sections added to `PowerExponent` (`core/parameter.rs:61`) and `FrameImage` (`capture.rs:315`). |
| G4-21 | 5b | `Runtime::paint` rustdoc and a guide/diagnostics.md line: a helper that also paints returns `Result<_, DrawError>`, or `AppError` with `native`. |
| G4-22 | 5b | `ApplicationError::new` and `RuntimeError::Application` rustdoc point to `ApplicationError::from` for boxed errors. |
| G4-23 | 5a | Contracts "Error policy": carriers of application or OS text (`InputRejection`, `ListingError`) display it verbatim. |
| G4-24 | 5a | Callback-boundary rationale for `From<WindowError> for RuntimeError` corrected in `docs/contracts.md:1125`, `R/runtime.rs:158` and `R/error.rs:15`. |
| G4-25 | 6d2 | `describe_component` (`runtime/inspect.rs:145-161`) clears a protected `value`, sorts and deduplicates actions and clears them when disabled, for `Semantics` and `SemanticItem`. |
| G4-26 | 6d2 | `CaptureJob`'s fields private with accessors (`capture.rs:531-539`); `capture_scene` looks the request up by id and rejects stale or foreign jobs (`runtime/capture.rs:296-344`). |
| G4-27 | 6a3 | `impl AsRef<str> for ChoiceId`. |
| G4-28 | 6a3 | `From<Vec<u8>>` and `From<&'static [u8]>` for `FontData`, beside the named constructors. |
| G4-29 | 6a3 | `From<Tab>`, `From<Section>`, `From<Property>` and `From<FloatingWindow>` for `Element`. |
| G4-30 | 6a3 | `From<Self>` into the raw type for `NormalizedValue`, `Capacity`, `Length`, `Magnification`, `ViewScale` and `PresentationRate`; the `get()` rule mentions it. |
| G5-1 | 5d | Every runtime and core example block shows `use pert_ui::…` through a hidden crate-alias line, so inlined facade pages show the facade spelling and the doctests still build; `F/root.md:42-47` updated. |
| G5-2 | 5c | `EventContext::defer` and `Runtime::spawn` link each other; guide/state.md "Worker results and live data" gains a complete defer-then-spawn example. |
| G5-3 | 5c | `Signal::sibling`, `ListItems::build` and `TableItems::cell` describe per-row bindings and their cost; F14's recipe shows the per-row and the retained-row form. |
| G5-4 | 5b | `DraftEditor`'s owner contract (`text_fields/draft.rs:437-`) links `Component::retain_initial_children` (with F17). |
| G5-5 | 5b | `Runtime::text_command`'s summary reads "a text command (clipboard or selection)". |
| G5-6 | 5b | The prelude pages (`R/lib.rs`, `F/lib.rs`) list the root names applications most often add, since rustc offers no import; 6e and 6f1 keep the list current. |
| G5-7 | 6e | Runtime prelude gains `Modifiers`, `KeyChord`, `CommandKey`, `CommandScope`, `CommandId`, `CommandReceipt`, `CommandStatus`, `CommandSource`, `CommandContext`, `CommandEntry`, `CommandOutcome`. |
| G5-8 | 6e | Prelude gains `HeldInputId` and `HeldCancellation`. |
| G5-9 | 6e | Prelude gains `ListDrop`, `ListDropPosition`, `ListDropPoint`, `ListDropPlacement`, `ListDropMark`. |
| G5-10 | 6e | Prelude gains `LevelMeter`, `Trace`, `MeterAxis`. |
| G5-11 | 6e | No change to membership: the exclusion sentence keeps a live widget's `SeriesSource` and its bridge at the root. |
| G5-12 | 6e | Prelude gains `PowerExponent`, `EditOutcome`, `PreviewOutcome`, `ParameterSnapshot`, `EditToken`, `ParameterValue`, `Revision`, `ParameterFamily`. |
| G5-13 | 6e | Prelude gains `AnyParameter`, `AnyParameterHost`, `AnySpec`, `AnyValue`. |
| G5-14 | 6e | No change: `EditSource` stays at the root under G5-26's rule. |
| G5-15 | 6e | Prelude gains `DeliveryStatus` and `AuthorityStatus`. |
| G5-16 | 6e | Prelude gains `DeliveryMonitor`, `DeliverySnapshot`, `TerminalEvent`, `PreviewEvent`, `SinkError`, `SinkErrorKind`, `DeliveryResolution`; the "hosts" sentence reworded. |
| G5-17 | 6e | Prelude gains `ViewScale`, `QuarterTurn`, `OpaqueColor`. |
| G5-18 | 6e | Prelude gains `MaskImage`, `ImageId`, `MaskId`, `VectorId`, `VectorGeometry`, `TextLayout`. |
| G5-19 | 6e | Prelude gains `FontManager`, `IconManager`, `IconGlyph`. |
| G5-20 | 6e | Prelude gains `FilterCounts`, `FilterOutcome`, `FilterTarget`. |
| G5-21 | 6e | Prelude gains `EditorSession`, `EditorStart`. |
| G5-22 | 6e | Prelude gains `ListingErrorKind`, `LinkTarget`. |
| G5-23 | 6e | Prelude gains `WorkspaceSettings`, `WorkspaceValue`, `IgnoredEntry`. |
| G5-24 | 6e | Prelude gains `WindowPlacement`. |
| G5-25 | 6e | The three guarantee sentences (`R/lib.rs:324-326`, `F/lib.rs:53-55`, `F/root.md:39-40`) point to `component`; 5b narrowed them meanwhile. |
| G5-26 | 6e | New `pert_ui_runtime::component` and `pert_ui::component` re-export modules: `Component`, the prelude's component names and the decision's 26 names, plus `EditorSession` and `EditorStart`. |
| G5-27 | 6e | Its one-hop names join `component` by the rule (receipts, `TaskStatus`, `NativeWindowOptions`, `NativeWindowId`, `ShortcutBinding`, `ShortcutScope`); `FittedText` not required; `ProfileSnapshot` stays out. |
| G5-28 | 6e | `NativeWindowId` in the facade prelude under `native` (`F/lib.rs:71`). |
| G5-29 | 5b | The facade prelude's error sentence names `AppShutdownError` (`F/lib.rs:58-60`). |
| G5-30 | 6e | `WindowError` in the facade prelude under `native`; the sentence becomes "`App` and everything it reports, errors included". |
| G5-31 | 5b | The facade prelude says its native members come with `native`, enabled by `desktop` and `desktop-cpu` (`F/lib.rs:50-51`, `:59`). |
| G5-32 | 6e | `ApplicationError` joins the runtime prelude under the constructed-error sentence; testing names wait for 6f1's module. |
| S:l06 | 5c | `VirtualList::new` rustdoc and the guide's "Large collections" show the existing route: wrap the `ListItems` value in `ListSource::new`; no new API. |
| S:m06 | 6c2 | no alias on `Modifiers::PRIMARY`, because routing `CONTROL` to it would mislead on macOS, where it means Command (survey.md:463); rustdoc-search aliases (`CONTROL`/`CTRL`, `ALT`/`OPTION`, `SUPER`/`META`/`WINDOWS`) on the matching fields; `Modifiers`' rustdoc says there are no per-key constants, shows `Modifiers { control: true, ..Modifiers::default() }`, and says `PRIMARY` is Control on Windows and Linux and Command on macOS; no trybuild case, since rustc's message does not change. |
| V3-1 | 6d2 | `TableColumn::MIN_WIDTH` states the saturated-coordinate exception, `table_columns.rs:577` asserts it, the comment at `table.rs:332` corrected. |
| V3-2 | 6d2 | Workspace restore rejects a stored column width below `MIN_WIDTH` (`table.rs:2028`, `:2075`) with the restore path's out-of-range error; rejection test. |
| V3-3 | 5b | `TextField`/`TextArea::{min_length, max_length}` take a parameter named `graphemes` and say the unit first (`text_fields.rs:450-458`). |
| V3-4 | 5b | `Constraints` and `Constraints::{new, width, height}` rustdoc say logical pixels. |
