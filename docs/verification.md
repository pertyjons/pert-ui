# Verification and evidence

This record distinguishes implemented contracts from native evidence and
outstanding checks. The commands below follow [AGENTS.md](../AGENTS.md) and
`.github/workflows/ci.yml`; that workflow is the authoritative current gate.
The [manual](manual.md) contains runnable application examples.

## Mixer-probe plan

The slices of [the mixer-probe plan](../plans/archive/mixer-probe-improvements.md)
(0.3.9 to 0.3.17) each passed the workspace gate on Fedora 44 Linux with Rust
1.99.0; slice 5 also passed the three serde configurations. Each slice had an
independent Codex implementation review whose findings were fixed. On Fedora
44 GNOME/Wayland, with temporary instrumentation that is not kept, the native
runner supplied its viewport before its first flush for a minimal
`PropertyGrid` application and for the probe's mixer; this is the evidence for
slice 1 leaving `App` startup unchanged.

The `live_mixer` example's headless session, extended after its review to a
cancelled audition, undo during a drag, a full engine queue with an undo and
Retry, a stopped engine and a partial layout restore, passed thirty
consecutive runs; an earlier version of
the full-queue step raced with the engine and was made deterministic by
having the engine confirm that it is holding before the queue is filled. The
0.3.15 window (`--window`, default features) ran on the same system and closed
with no error or validation output; a maintainer used it interactively. Library
packaging was verified with `cargo package --locked --workspace --exclude
pert-ui-gallery --exclude pert-ui-native-checks --allow-dirty`, admitting the
uncommitted slice without disabling verification. From the extracted 0.3.17
packages, outside the checkout and with their internal dependencies patched to
those extracted packages, the example compiled and its headless session passed
with `--no-default-features`, and it compiled with
`--no-default-features --features desktop-cpu`. This is local packaged-consumer
evidence, not a crates.io dry run; no other platform was exercised.

## Diagram-probe plan

The slices of [the diagram-probe plan](../plans/archive/diagram-probe-improvements.md)
(0.3.18 to 0.3.23) each passed the workspace gate on Fedora 44 Linux with Rust
1.99.0, and the pointer-refresh tests also passed in the release profile.
Slices 1, 2 and 3 had a Codex design checkpoint before implementation, and
every slice an independent Codex implementation review whose findings were
fixed with regression tests. The `diagram_editor` example's headless session,
extended after its review (0.3.23) to the outline selecting on the canvas, a
draft across a selection change, close with a draft, an overtaken open,
rename refusal and hiding, and label escaping, passed twenty consecutive runs with `--no-default-features --features cpu`
and passed without the `cpu` feature; `cargo package --list` includes its
sources. Its window has not yet been recorded as used on a native platform;
the probe it was reduced from ran natively on Fedora 44 GNOME/Wayland, where
the first version's drag failed because of the repeated pointer move that
slice 1 documents.

## Log-explorer plan

The slices of [the log-explorer plan](../plans/archive/log-explorer-improvements.md)
(0.3.24 to 0.3.31) each passed the workspace gate on Fedora 44 Linux with
Rust 1.99.0; the shrink, collection-diagnostic, selectable-text and text
tests also passed in the release profile, the serde configuration ran the
schema test and the CPU configuration the capture tests after the text
change. Slices 1, 3 and 4 had a Codex design checkpoint before
implementation (each first rejected, then approved after revision), and
slices 1 to 5 an independent Codex implementation review whose findings
were fixed with regression tests (0.3.26, 0.3.28 and 0.3.31). Slice 4's
unit tests count shapings: a fitting ellipsis text is shaped once cold and
not at all warm, a truncated one measures its whole string once; CPU
captures of 56 ellipsis cases rendered by a temporary program against
0.3.28 and 0.3.31 were pixel-identical. The `log_explorer`
example's headless session over 50 000 generated lines passed in the debug
profile, where development validation reported no unbounded collection,
and in the release profile, where loading took about 36 ms, a wheel step
4.13 ms (4.78 ms before slice 4), sorting 25 ms, content search 7 ms and an
inspection snapshot from another thread 3 ms, with at most about 600
components mounted. `SelectableText` and the example's window have not been
recorded as used on a native platform; the probe the example was reduced
from ran natively on Fedora 44 GNOME/Wayland.

On 2026-10-20 (0.3.51), with temporary instrumentation that is not kept, the
release-profile wheel step was split. Each value is the median of three runs
on Fedora 44 x86_64 with 24 threads.

| Lines | Wheel step | Text preparation | Shapings | Shaping | Rasterizations | Rasterization |
|---|---|---|---|---|---|---|
| 50 000 | 4.28 ms | 2.12 ms | 49.4 | 0.92 ms | 19.9 | 0.96 ms |
| 200 000 | 4.28 ms | 2.12 ms | 49.3 | 0.92 ms | 19.8 | 0.96 ms |

Every step prepares the newly mounted rows' cells, so neither the layout
cache nor raster reuse answered any of them. The step does not grow with the
log's length, since only the mounted rows prepare text. The "before" of the
slice 4 comparison (0.3.28) cannot be rebuilt: the plan's branch was
squash-merged and deleted, so only the 4.78 ms recorded then remains.

## Table-editing plan

The slices of [the table-editing plan](../plans/archive/table-editing-improvements.md)
(0.3.32 to 0.3.44) each had an independent Codex implementation review
whose findings were fixed (0.3.33, 0.3.36, 0.3.38, 0.3.40, 0.3.42 and
0.3.44), and
slices 4, 5 and 6 a Codex design checkpoint before implementation. These are
claimed by the plan and the squash commit; the per-slice gate runs were not
recorded. On 2026-10-20 the merged result (5a6e451) passed the workspace gate
on Fedora 44 Linux (kernel 7.2.11) with Rust 1.99.0.

The `inventory` example's headless session over 10 000 and 100 000
generated items passed in the release profile on Fedora 44 x86_64. It
mounted 752 components at both sizes. Enter opened an editor in about 2 ms
and a save took 5 to 9 ms. A save that re-sorts 100 000 rows and moves the
edit took about 66 ms. The example's window, the gallery's editable Note
column and Tab traversal scopes have no native evidence (TODO.md NATIVE-05).

## Plan-audit repairs

The slices of [the plan-audit repairs](../plans/plan-audit-repairs.md)
(0.3.45 onward) each passed the workspace gate on Fedora 44 Linux (kernel
7.2.11) with Rust 1.99.0. From slice 3 that gate includes the examples'
headless sessions. Slice 2 also passed its tests in the release profile, and
slice 4 its serialization test with `serde`. Each slice had an independent
Codex review whose findings were fixed before the commit or in a follow-up
commit; slices 1 and 2 also had a design checkpoint.

The gallery demonstrations of slice 8 have headless gallery tests. On
2026-10-20, after they were added, `cargo run --locked -p
pert-ui-native-checks -- interaction` passed both gallery interaction checks
on Fedora 44 Workstation, GNOME on Wayland: `interaction_check` and
`interaction_check_cpu`, 30 steps each. Input entered at the runtime's native
entry points, and these checks do not step through the new demonstrations
themselves. No hand check of them is claimed.

## Developer usability

[The developer-usability plan](../plans/developer-usability.md) runs blind
tests: AI testers build small applications from the packaged documentation
in a sandbox with no network beyond the model hosts and no access to the
repository. The kit holds the packaged crates, whose source a tester may
read if it logs it; none did.

The evidence is exploratory. There are two testers, one run per task each,
so the times and counts describe how each tester worked, not a usability
score. Who did what:

- **Testing:** Claude Code with Opus 5.5 at high effort, and Codex CLI with
  `gpt-6.1-sol` requested at medium effort (Codex's effective model is not
  in its transcript).
- **Acceptance and triage:** fresh Claude agents, followed by the author's
  checks.
- **Review:** Codex. It also reviewed the acceptance and triage of the runs
  it produced itself.

Round 1 ran on 2026-10-21 against 0.3.52 (commit 359122e, Rust 1.99.1).
Each run's records are in
[`plans/usability/runs/`](../plans/usability/runs/README.md):

| Task | Claude: session, acceptance | Codex: session, acceptance |
|---|---|---|
| 1 settings editor | 5.5 min, partly met | 6.3 min, partly met |
| 2 task list | 7.6 min, partly met | 6.9 min, met |
| 3 custom control | 8.0 min, met | 3.8 min, met |
| 4 testing an existing app | 4.5 min, met | 4.2 min, met |

**Results:**
- Every run finished well inside its 90-minute budget.
- The three partly met runs missed one behavior each:
  - closing with an uncommitted typed change;
  - a close refusal tested only on a window added for testing;
  - list actions that did nothing for a shown selection when the query
    ended in a space.

**Triage:** the 64 friction entries of round 1 and the two task 2 pilots,
with the acceptance notes and what verification turned up, resolve into 36
findings (one of them, the getting-started dependency, a kit issue today),
ranked in
[the plan's findings table](../plans/developer-usability.md#round-1-findings).
The most frequent are:
- the getting-started dependency instructions, an artifact of the
  unpublished library;
- the missing testing chapter and helpers;
- prelude gaps for keyboard and command types.

Defects found:
- a focused editor's draft is not committed before a window's close policy
  runs;
- `check_redescription` reports a false overlap with a viewport set;
- `CollectionFilter`'s documented `filter` key collides with its search
  field (found while verifying, not by a tester).

## Thin outlines at fractional zoom

A 75% scale regression renders a one-logical-pixel rounded outline at a
fractional origin and checks a pixel on each straight side. The CPU oracle and
the native CPU/GPU parity case passed on Fedora 44 Wayland with Intel UHD
Graphics 770 (Vulkan); the parity case differed by at most one 8-bit channel.
This checks stroke visibility and backend agreement for that fixture, not every
control geometry or zoom phase.

## Text rendering modes

The gallery switches live widget text between `LineMask` and `PhaseLine`.
Headless tests cover switching, one mask command per text run, vertical ink
alignment at 75%, 90%, 100%, 125% and 150% zoom, and repaint during fractional
scrolling. A rich-text pixel test checks four inline icons across x/y
quarter-pixel positions at those zoom levels, comparing absolute ink positions
and icon-to-text alignment with `LineMask`. A retention test checks that
whole-pixel scrolling reuses local drawing commands while fractional scrolling
repaints them. A boundary test checks the longest repeated-glyph line admitted
by `LineMask` at a fractional origin. Native CPU/GPU parity checks cover the
phase mode at 75%, 90%, 100% and 125%, including inline icons at fractional
positions and 75%, 90% and 125% zoom. On Fedora 44 Wayland with Intel UHD Graphics 770
(Vulkan), the new icon cases passed with at most one 8-bit channel difference.
A native probe can capture moving and scrolling text and report
input/preparation, CPU rendering and GPU offscreen rendering plus readback:

```sh
PERT_UI_TEXT_MODE_REPORT_DIR=/tmp/pert-ui-text-modes cargo test --locked --release -p pert-ui-wgpu native_text_rendering_probe -- --ignored --nocapture
```

On Fedora 44 Wayland, Intel Core i9-12900 and UHD Graphics 770 (Vulkan),
three release runs of 20 visible rows over 32 quarter-pixel scroll steps gave
these ranges of run medians:

| Mode | Mask commands | Input + preparation | CPU render | GPU + readback |
|---|---:|---:|---:|---:|
| LineMask | 20 | 55–73 µs | 3500–6635 µs | 1910–2789 µs |
| PhaseLine | 20 | 79–102 µs | 3067–5345 µs | 1979–2409 µs |

These are local comparisons, not frame-time guarantees or pure GPU draw times.
Both modes carried 111260 scene mask bytes in this fixture. `PhaseLine` keeps
separate glyph and run caches, each admitting up to 16 MiB of coverage data;
cache keys and retained scenes add memory beyond those budgets. The original
text cache separately admits up to 16 MiB. PhaseLine was chosen over per-glyph
drawing because visual quality appeared similar while the per-glyph mode made
603 commands versus 20 and took more preparation and GPU plus readback time
in the earlier comparison. That discarded mode used smaller scene masks and
was faster on CPU in one run, so software rendering and upload volume remain
tradeoffs to measure on target workloads. The user reported sharper phase text;
this does not establish a visual improvement for every font, scale or display.

An opt-in release-profile cache probe prepares 20, 60 and 100 unique lines at
75%, 90% and 125% scale, then repeats four vertical or all sixteen two-axis
quarter-pixel phases. It times only phase-mask lookup/composition, including
probe bookkeeping, and reports cache hits, evictions and retained coverage:

```sh
cargo test --locked --release -p pert-ui-runtime --lib phase_cache_scroll_profile -- --ignored --nocapture
```

With the former 128-entry line limit, the 60- and 100-row vertical cases and
even the 20-row two-axis case had zero hits after the first phase cycle. The
100-row vertical cases took 1.6–2.7 ms per frame in one release run. Raising
the internal entry limit to 2048 while retaining the 16 MiB coverage limit
gave full hits and no evictions in all these warmed cases. Across two later
runs, the 100-row frames took 0.14–0.15 ms median. At 100 rows, retained line
coverage was 1.4–3.7 MiB for vertical motion and 5.4–14.9 MiB across both
axes; glyph coverage stayed below 50 KiB. A regular regression test preserves
reuse for 60 rows over four vertical phases. The timings are local CPU
comparisons, not whole-frame or GPU measurements. The coverage figures exclude
cache keys, metadata, prepared text and scene ownership, and longer text can
still exhaust the byte budget.

An earlier native GPU interaction run failed at the command-palette Go to step,
while CPU and a direct GPU retry passed. The check now observes the filtered
query and its single result before pressing Enter. The headless run and direct
native GPU and CPU runs passed on Fedora 44 GNOME/Wayland after that change,
including both selected `pert-ui-native-checks interaction` runs on kernel
7.2.10-200.fc44.x86_64 with Rust 1.99.0. The original intermittent failure
was not reproduced or assigned a library cause. The native restart check also
passed its save and restore processes through
`pert-ui-native-checks workspace_restart` after its pacing moved from
an input timer to an update deadline. These runs do not exercise a native
minimize or focus loss during either check.

CPU captures of the Input & parameters gallery page at scale 2 in the built-in
dark and light modes showed the slider thumb clearly marking the boundary
between the `Primary` fill and `Outline` remainder. No color or geometry
change was needed for that marker. This was a visual review of the default
value and themes, not a contrast guarantee for custom palettes.

## Workspace gate

Run the smallest gate covering the change. The workspace gate is:

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo test --locked --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
```

Changes to the core live-data protocol additionally run the release-profile
Loom models named in AGENTS.md. Dependency, feature, platform and release
changes also need the feature configurations, release tests, MSRV, headless
gallery, package and isolated-consumer checks listed there. A passing headless
gate is evidence for contracts that do not require a native session; it is not
evidence for GPU output, focus routing or platform integration.
CI runs `cargo clean` between the base, image/SVG, serde/CPU and packaging
groups to reclaim disk space before the next group, and builds the dev and test
profiles without debuginfo or incremental caches; it retains every check. The
facade's integration tests are grouped into a few area binaries under
`crates/pert-ui/tests/<area>/main.rs`, so each feature configuration links a
few test binaries instead of one per test file.

### Parley 0.12 review

The [0.12.0 release](https://github.com/linebender/parley/releases/tag/v0.12.0)
adds grapheme-sized shaped clusters, CSS-like line-box and whitespace behavior,
vertical alignment for spans and inline boxes, CJK line-break control and the
lower-level `parley_engine`. The runtime keeps Parley's paragraph layout because
its current text controls need Parley's line breaking, alignment and editing
geometry. It uses the new vertical alignment for icon spans while continuing to
center icon ink on neighboring text; current plain and rich text preserve their
spaces, so the new `TreeBuilder` whitespace-collapse modes are not enabled.
Explicit CJK line-break policy and lower-level line breaking need a demonstrated
consumer requirement. Parley's grapheme, shaping and line-break fixes apply
through the existing text service; no new application API is required.

The dependency migration passed the workspace and CI feature gates, release
tests, the headless gallery, package verification and a separate consumer build
using only extracted library packages. On Fedora 44 Wayland, GNOME Shell 50.5,
with Rust 1.99.0,
the native GPU/CPU parity check passed its text case, and both GPU and software
gallery interaction checks passed all 29 steps, including typing, focus and IME
requests. These runs did not resize the gallery during the interaction check;
the separately recorded resize-sensitive history failure remains open. Native
text and font output on macOS and Windows still need execution evidence.

### Rust 1.99 upgrade review

The toolchain and declared minimum Rust version are 1.99.1 since 0.3.52;
the 1.99.0 runs recorded in this file predate that. The 1.99.1 point
release (2026-10-15) fixes a missing `wcslen` symbol on UEFI and embedded
targets, a miscompilation of FFI `bool` return values on x86_64 and the
`libLLVM` rpath of rust-lld on macOS; it needs no source change. On
2026-10-21 on Fedora 44 with `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, every
step of the CI workflow passed locally (formatting, Clippy, debug and
release tests, the examples' headless sessions, doctests, rustdoc, the
feature configurations, the release Loom models and the headless gallery),
and the five library packages were packaged and verified. The native checks
were not rerun for the point release. The
[official release notes](https://doc.rust-lang.org/1.99.0/releases.html#version-1990-2026-10-01)
inform the 1.99 upgrade:

- `String::from_utf8_lossy_owned` fits the internal validation test log:
  the log already owns its byte buffer, so valid UTF-8 can reuse that allocation
  instead of copying through a borrowed lossy conversion.
- Cargo now allows edition-2024 members to override an inherited dependency's
  `default-features`. The existing workspace already disables defaults at the
  shared boundary and selects features per member, so it needs no adjustment.
- Inclusive-range iteration optimizations and rustdoc implementation filtering
  may benefit existing code without API changes. No project-specific speedup
  has been measured from this upgrade.
- The new `debug` profile currently behaves like `dev`; the existing profiles
  remain suitable. CI now disables incremental compilation by default.
- C variadic definitions, raw-pointer layout operations and allocation-parts
  APIs do not demonstrate a need for a new unsafe boundary in this library.
  `VecDeque::retain_back` is available, but no existing queue requires changing
  its traversal order as part of this upgrade.

Direct dependency versions were checked against crates.io on 2026-10-03.
`thiserror`, `serde` and `serde_json` needed manifest updates; the other direct
dependencies already used the latest non-yanked stable releases. The lockfile
also refreshes compatible transitive releases. Native and packaged-consumer
verification is required because those releases include platform code.

The upgrade passed the workspace gate on Fedora 44 on 2026-10-03 with
`rustc 1.99.0 (b940084d7 2026-09-28)`: debug and release tests, doctests,
warning-free Clippy and rustdoc, the CI feature configurations, release Loom
models and the headless gallery. Library package listings were inspected for
fonts, shaders and licenses, and `cargo package --locked --workspace --exclude
pert-ui-gallery --exclude pert-ui-native-checks --allow-dirty` verified the
packages. `--allow-dirty` admitted the uncommitted upgrade; verification was
not disabled.

An isolated consumer outside the checkout used only extracted library packages,
with their internal dependencies wired to those extracted packages rather than
workspace patches. The getting-started guide's parameter, persistence and
custom-widget examples passed as doctests, and the packaged native
`getting_started` example compiled. This is local packaged-consumer evidence,
not a crates.io publication or dry run. A fresh-context review found no
actionable bugs in the upgrade. Windows and macOS were not exercised locally.

## Component API review completed

The component API review's final closure recorded completion on 2026-10-05 on
Fedora 44 Linux with Rust 1.99.0. Every public component has an individual
assessment and explicit API exceptions. Final formatting, workspace
Clippy/tests/doctests and warning-free rustdoc passed after all repairs,
including cross-component notification ordering and menu-title cancellation.
Focused SVG tests and the headless gallery also passed.

Library package listings and verified workspace packaging passed with
`--allow-dirty`, admitting this review's uncommitted changes without disabling
verification. Fonts/icons, attribution/licenses, shaders, guides and examples were
checked in the archives. A consumer outside the checkout used only extracted
packages, without workspace patches; form, parameter, dynamic-choice, custom
widget and callback tests passed. The extracted facade's getting-started example
compiled and doctests passed. Dependency versions matched the workspace lockfile.
Core's crates.io publish dry run passed without uploading. Interdependent library
dry runs remain unavailable until their versions are registry-resolvable; local
package/consumer verification is distinct from publication. Generated API docs,
manual, guides, inventory, conventions and Markdown links were reviewed together.
NATIVE-01/02/03 remain unchanged; no new native platform execution is claimed.

The following entries preserve evidence for the earlier bounded slices.

The first availability slice on `component-api-review` passed the workspace
formatting, Clippy, tests, explicit doctests and warning-free rustdoc gates on
2026-10-05. Public-facade regressions cover restrictive permissions, fixed
pre-flush state, armed-input cancellation, binding replacement and hidden-state
refresh. CollectionFilter and FileBrowser fixed-denial tests failed before their
repairs and pass afterward. Actual local Claude reviewed the slice and rechecked
its corrections. A CPU gallery screenshot supplied initial-state visual evidence;
no new native platform evidence is claimed. Its availability report recorded
the scope and executed evidence. The final closure above supersedes the original
incomplete-review handoff.

The numeric format review recorded cloned formatter layout retention and
changed-format invalidation tests. Its sequential workspace formatting, Clippy,
tests, doctests and warning-free rustdoc gate passed on 2026-10-05. This closes
that bounded finding, not the remaining numeric component review.

The table restore repair passed the same sequential workspace gate on
2026-10-05. Direct component-hook tests reproduce shutdown rejection and
exercise forbidden-phase rejection with unchanged column state. After Claude
reached its session limit, a user-approved fresh-context agent independently
reviewed the repair and found no correctness defects. Its rollback-coverage
finding was addressed with a source-triggered shutdown regression and rechecked
by the reviewer. The table-column test suite, targeted Clippy and formatting
passed afterward; no production code changed. This closes the bounded
persistence review, not the full Table API assessment.

The bound icon-button repair passed the sequential workspace gate on 2026-10-05.
DisplayList comparisons verify that icon changes and text/icon transitions match
fresh descriptions while preserving identity and the bound semantic name;
unchanged descriptions retain layout. Generated documentation and review-report
file links were checked. This adds no native execution evidence and does not
close the full API review.

The family A adornment assessment passed the full sequential workspace gate on
2026-10-05. Public drawing comparisons cover retained caption adornments,
replaced/cleared slots, binding replacement, icon-button names, icon overrides
and armed action adoption. Empty-caption tests exposed a text-shaper panic, and
independent review caught a follow-on empty-text cache regression; both were
repaired and tested. The reviewer rechecked the fixes and documentation with no
remaining findings. Claude supplied the bounded design consultation. These are
headless results; native gaps remain unchanged.

The bound choice construction slice passed formatting, workspace Clippy,
workspace tests, explicit workspace doctests and warning-free rustdoc
sequentially on 2026-10-05. Focused facade tests cover duplicate rejection,
last-valid semantics, recovery, foreign option sources and same-key source
transitions. Generated constructor documentation was inspected. Claude supplied
design input; independent read-only review findings were repaired and rechecked.
These results do not complete family B or add native evidence.

The choice permission slice passed the full sequential workspace gate on
2026-10-05: formatting, warning-denying Clippy, workspace tests, explicit
doctests and warning-free rustdoc. Generated permission documentation and
relative links were inspected. Focused regressions cover cancellation, authority
restrictions, retained dropdown highlight, source replacement, released row
capture, chips, hidden/reveal behavior and draft closing. Claude consultation
and independent read-only review informed the repairs; the final reviewer found
no actionable issues. The separately reproduced MultiSelect option-source
handoff defect is addressed in the follow-up below; this does not close family
B.

The MultiSelect source-handoff repair passed the full sequential workspace gate
on 2026-10-05, including explicit doctests and warning-free rustdoc. The
failed-before public reproduction now shows the replacement source. Focused
tests verify source/filter replacement, retained row identity and draft,
refreshed highlight subscriptions, old-source silence, invalid-source recovery,
rapid descriptions and reopen. Independent read-only review found no actionable
issues; generated documentation, links and whitespace were checked. The resolved
TODO entry was removed. Native evidence is unchanged, and the remainder of
family B stays open.

## Semantic completeness

The gallery test `every_interactive_gallery_component_has_complete_semantics`
walks every gallery page headless. On each page, every presented component
that reports itself interactive must have semantics with a role, a resolved
non-blank name and, for a role that presents a value (checkbox, switch, radio
group, segmented choice, slider, number and text inputs, combo box, readout,
progress), that value. The name may be the component's own, its element's
label (`Element::with_label`), or one a wrapping `Field` gives it; both reach
the part of a composite that represents it through the composite's internal
structure. A concealed component, one whose semantics are `protected`
(`Semantics::protected`), must report no value. A disabled component offers no actions.
An item a QuickPick or CommandPalette reports disabled is the one deliberate
exception: it keeps `Activate`, because choosing it is where the application
explains why it is unavailable (see `PickItem`).

The check's evidence is limited to what it walks: the gallery's own
compositions, on the pages as they open. Controls a page hides (an inner
unselected tab, a collapsed section, a closed popup) are checked only where a
page shows them, components that are not interactive are structure and need
no semantics, and compositions a consumer builds are not covered. A new
component is covered once a gallery page shows it.

## Native evidence

The target matrix is Fedora 44 with GNOME or KDE, Ubuntu 26.04 LTS with GNOME
or KDE, Windows 11 25H2 with DirectX 12, and macOS Tahoe 26 with Metal. Linux
uses Wayland and Vulkan. Only Fedora 44 GNOME/Wayland/Vulkan has native
execution evidence. Every other matrix platform has **no native evidence**;
headless CI does not change that status. A later run should record its OS patch,
adapter and driver rather than infer missing details.

After updating `serde_json` to 1.0.154 on 2026-10-15, the native runner's
`--list` parsed Cargo's JSON messages and its selected `interaction` run passed
both GPU and CPU gallery checks on Fedora 44, kernel 7.2.10-200.fc44.x86_64,
GNOME/Wayland with Rust 1.99.0. This selected run did not print the GPU adapter
or driver; it does not cover the other native checks or another platform.

| Contract or check | Fedora GNOME/Wayland/Vulkan evidence |
|---|---|
| Window startup, resize, zero-size restore, settled idle and GPU pixel readback | Passed. |
| Standard controls, popups, numeric inputs and native focus-loss dispatch | Passed in native smoke checks. |
| Text editing with injected IME preedit and commit | Passed; inline numeric text mode passed 2026-09-26. |
| Text-field context menu and clipboard | Passed by hand through the clipboard provider's X11 fallback. |
| Dialogs, nested menus, banners, toasts, rich text, font and icon replacement | Passed in native smoke checks. |
| Two-window startup focus transfer; three retained surfaces, recovery, suspend/resume and re-dock | Passed. |
| Live workload; workspace save and restore across two processes | Passed. |
| Native check runner, view transforms, quarter-turn pixels, window and widget capture | Passed. |
| Presentation pacing above 60 Hz | Passed on one monitor reporting 99.981 Hz. |
| `ZoomView` zoom and pan | Reported working by hand. |
| Raster-image GPU rendering | Passed on Linux/Vulkan; the original record did not name a distribution. |
| Drag session through injected native pointer events, including preview, release outside and tab detach | Passed 2026-09-26; these were injected events. |
| Drag cursor, fractional-scale preview and real pointer leaving the window | Reported working by hand 2026-09-26. |
| SVG gallery section, menu bar, native pinch | No native evidence. |
| General components of 2026-10-01 (search field, knob, chip, breadcrumb, reveal navigation, link, color picker, collapsible and accordion, multi-select, command palette, property grid, file browser) | The whole native suite passed on the branch that adds them (2026-10-01), so existing native behavior did not regress; no native check or hand check exercises the new components themselves yet. |
| Tool window floating frames and own native windows (Layout › Tool windows: Float, then drag the header and the corner grip; Open in window, then close the native window and see the tool window back on its stripe; Dock from the window's options menu) | Linux: passed 2026-10-05 on Fedora 44 GNOME/Wayland as steps of the native gallery interaction check: float through the options menu, header drag, corner-grip resize, open in a real native window and close it back to the stripe. Input enters at the runtime's native entry points; window creation and closing were the real host's. Windows and macOS have no evidence. |
| Gallery interaction check (`pert-ui-gallery::interaction_check`): menu-bar brand mark and history back/forward, command palette shortcut and Go to, chip removal, search field debounce/choose/clear, concealed password (no IME request, Alt+F8 reveal, concealed again on focus loss), content search shortcut and jump, knob pointer drag, collapsible section toggle, tool window expand and Escape, float, header drag, corner-grip resize, own native window and its close, collection filter and clear, settled idle | Passed natively 2026-10-02 (tool window float to close steps added and passed 2026-10-05) on Fedora 44, GNOME on Wayland, Intel UHD Graphics 770 with Mesa 26.2.3, as part of the whole native suite; Linux only. Input enters at the runtime's entry points, so it is not evidence for the adapter's OS event translation. |
| Native suite on the application-author branch (0.3.8, `app-author-improvements`): interval timers serviced every loop turn, software presentation, and the gallery as it then was, which showed value-driven children only through `Dynamic` on the Chips page and did not yet demonstrate flex layout or selection notifications | Passed 2026-10-18 on Fedora 44, kernel 7.2.11-200.fc44.x86_64, GNOME/Wayland, Intel UHD Graphics 770 with Mesa 26.2.4 on Vulkan, Rust 1.99.0: every check of the runner, including both gallery interaction checks and the settled-idle checks, which show the interval servicing adds no wakeup when no timer runs. `--skip-clipboard` omitted the clipboard check. No check starts an interval timer in a native window. |
| `desktop-cpu` build (no wgpu) | On the same machine and date, each ignored window check of `pert-ui` built with `--no-default-features --features desktop-cpu` ran in its own process: startup, resize and idle, capture, drag, and three software surfaces with redock passed, all presented by the CPU renderer. The pacing check is compiled only with `gpu`: in this build it measured 47.6 updates/s against a 99.981 Hz monitor, since CPU frames have no interactive budget. The native runner itself builds the default configuration and does not run this one. |
| Native suite after the framework audit repairs | Passed 2026-10-04 on Fedora GNOME/Wayland/Vulkan, including hardware readback and real-renderer capture. `--skip-clipboard` explicitly omitted the clipboard check. Existing injected/runtime-entry interaction limits still apply; no new hand check of floating tool windows or another platform is claimed. |
| Software presentation (`App::software_rendering`): `software_presentation_captures_the_window_and_a_widget`, `software_window_resize_and_idle`, `three_software_surfaces_animate_settle_and_redock` and `pert-ui-gallery::interaction_check_cpu` | Passed 2026-10-05 on Fedora 44 GNOME/Wayland with softbuffer over Wayland shared memory: capture, resize, minimize/restore, input, recovery, floating windows, detach/redock, the 29-step interaction check and settled idle, with no GPU device created. Input entered at the runtime's native entry points. The user then checked the `--cpu` gallery by hand on the same desktop on 2026-10-05: real pointer, keyboard and text input, the clipboard, and display scales from 75 % to 200 %, including fractional ones, all looked and behaved correctly. Windows and macOS only compiled. |
| Native checks after the Permission and geometry slice (0.2.88): `drag_smoke`, both multi-window smokes and the GPU and CPU gallery interaction checks | Linux: passed 2026-10-07 on Fedora 44 GNOME/Wayland after detachment became a Permission evaluated at request time and floating-window movement a Permission with `on_frame`. Input entered at the runtime's native entry points; the GPU adapter was not printed by these checks. Windows and macOS have no evidence. |
| Native window retitle (`PresentationAction::Retitle`), a step of `three_native_surfaces_animate_settle_and_redock` and `three_software_surfaces_animate_settle_and_redock` | Linux: passed 2026-10-07 on Fedora 44 GNOME/Wayland (0.2.87), GPU and software presentation: the real host applied a queued retitle of a detached window and completed it as presented. winit cannot read a title back on X11 or Wayland, so the shown title bar is not asserted and no hand check is claimed. Windows and macOS have no evidence. |
| CPU/GPU parity (`pert-ui-wgpu::tests::native_cpu_gpu_parity`) | Passed 2026-10-05 on Fedora 44 GNOME/Wayland, Intel UHD Graphics 770 (ADL-S GT1) on Vulkan: the exact tier matched byte for byte and the tolerance tier differed by at most one per channel. No other adapter, and no CI software-driver comparison, has run. |
| Native suite on the CPU renderer branch (0.2.82) | Passed 2026-10-05 on Fedora 44 GNOME/Wayland, Intel UHD Graphics 770 on Vulkan, after the capture crop and tool window title repairs: every native check including CPU/GPU parity, real-renderer capture and the 29-step gallery interaction check. `--skip-clipboard` omitted the clipboard check. |
| Native suite after the post-repair sweep (0.2.81) | Passed 2026-10-05 on Fedora 44 GNOME/Wayland, Intel UHD Graphics 770 with Mesa 26.2.3 on Vulkan: capture, drag, multi-window, pacing, resize/idle, GPU readback and damage probes, workspace restart and the 24-step gallery interaction check. `--skip-clipboard` omitted the clipboard check. No hand check of floating tool windows or another platform is claimed. |

The original evidence recorded these environments; “not recorded” means exactly
that. The table and check descriptions in this document record the available
test evidence and its limits.

| Run | Date | OS patch or desktop | Adapter | Driver |
|---|---|---|---|---|
| Window smoke and readback | 2026-09-11 | Not recorded | Intel Iris Xe (RPL-P) | Mesa 26.1.8 |
| Standard controls/readback | Not recorded | Kernel 7.2.4-200.fc44 | Readback: Intel Iris Xe (RPL-P) | Readback: Mesa 26.1.8 |
| Text fields | Not recorded | Kernel 7.2.4-200.fc44 | Not recorded | Not recorded |
| Text-field context menu by hand | Not recorded | Kernel 7.2.5-200.fc44, GNOME/Mutter | Not recorded | Not recorded |
| Numeric inputs | 2026-09-12 | Kernel 7.2.4-200.fc44 | Not recorded | Not recorded |
| Transient surfaces; fonts and icons | Not recorded | Not recorded | Not recorded | Not recorded |
| Native input probe | 2026-09-12 | Kernel 7.2.4-200.fc44, GNOME Shell/Mutter 50.4 | Not recorded | Mesa Vulkan drivers 26.1.8 |
| Three native surfaces | 2026-09-12 to 2026-09-13 | Not recorded | Not recorded | Not recorded |
| Live workload | 2026-09-16 | GNOME Shell/Mutter 50.4 | Not logged (Iris Xe and Arc A370M present) | Mesa Intel 26.1.8 |
| Workspace restart | 2026-09-20 | Kernel 7.2.5-200.fc44 | Intel UHD Graphics 770 (system inventory) | `mesa-vulkan-drivers-26.1.8-1.fc44` |
| Native check runner | 2026-09-21 | Not recorded | Intel UHD Graphics 770 | Mesa 26.2.2 |
| Capture | 2026-09-23 | Not recorded | Intel UHD 770 | Not recorded |
| Pacing above 60 Hz | 2026-09-23 | Not recorded | Intel UHD Graphics 770 (ADL-S GT1) | Mesa 26.2.3 |
| `ZoomView` by hand | 2026-09-24 | Not recorded | Not recorded | Not recorded |
| Raster image GPU check | Not recorded | Not recorded | Intel integrated graphics | Not recorded |
| Injected-pointer drag check | 2026-09-26 | Kernel 7.2.7-200.fc44, GNOME on Wayland | Intel UHD Graphics 770 (ADL-S GT1) | Mesa 26.2.3 |
| Drag and drop by hand | 2026-09-26 | Kernel 7.2.7-200.fc44, GNOME Shell 50.5 | Intel UHD Graphics 770 | Mesa 26.2.3 |
| Native suite on the general-components branch | 2026-10-01 | Kernel 7.2.8-200.fc44, GNOME on Wayland | Intel UHD Graphics 770 (ADL-S GT1) | Mesa 26.2.3 |
| Native suite with the gallery interaction check, at 0.2.79 and again at 0.2.80 | 2026-10-02 | Kernel 7.2.8-200.fc44, GNOME on Wayland | Intel UHD Graphics 770 (ADL-S GT1) | Mesa 26.2.3 |
| Whole native suite after the Rust 1.99.0 and dependency upgrade | 2026-10-03 | Fedora 44, kernel 7.2.8-200.fc44, GNOME Shell 50.5 on Wayland | Intel Iris Xe (RPL-P), PCI 0000:00:02.0 | Mesa 26.2.3, Vulkan |
| Whole native suite after the Rust 1.99.0 and dependency upgrade, with `MESA_VK_DEVICE_SELECT=8086:5693!` | 2026-10-03 | Fedora 44, kernel 7.2.8-200.fc44, GNOME Shell 50.5 on Wayland | Intel Arc A370M (DG2), PCI 0000:03:00.0 | Mesa 26.2.3, Vulkan |
| Available native suite after the framework audit repairs (`--skip-clipboard`) | 2026-10-04 | Fedora 44, kernel 7.2.8-200.fc44.x86_64, GNOME on Wayland; compositor version not recorded for this run | Intel Iris Xe (RPL-P), PCI 0000:00:02.0 | Mesa 26.2.3, Vulkan |

Both upgrade runs above passed every check the native runner selected, including
the gallery interaction and workspace restart checks. They used scale 1 and
the built-in display reporting 59.95 Hz; pacing exercised the at-most-60-Hz
fallback, not higher-refresh presentation. The renderer's logged adapter
confirmed each GPU. These debug-profile verification runs are not controlled
performance comparisons with the earlier release-profile probes.

Run `cargo run --locked -p pert-ui-native-checks` on an unlocked desktop session
with a hardware GPU to exercise the opt-in native suite. Besides the libtest
checks and the workspace restart, the suite runs the gallery's interaction
check (`pert-ui-gallery --interaction-check`): it opens the real gallery
window and drives named targets on its pages one step at a time, waiting for a
frame after each action and for one step interval so debounces elapse in real
time, then checks what the page shows, fails on an error or a validation issue,
and ends by requiring that nothing stays scheduled. Its input enters at the
runtime's native entry points the `App` calls (`command_key_input`,
`pointer_input`, `text_input`), not as window-system events, so it is evidence
for the components' behavior in a native window and frame, not for the
adapter's translation of OS events, which the drag smoke and the input probe
cover. The steps it covers are listed in
`apps/gallery/src/interaction_check.rs`; its passing run is evidence for those
steps on that platform only. Run the gallery for
manual interaction and inspect its current pages through
[apps/gallery/README.md](../apps/gallery/README.md). Record OS patch, desktop,
adapter, driver, scale and which checks actually ran. The open native-evidence
items are tracked in [TODO.md](../TODO.md).

The native runner starts each ignored GUI/GPU check in its own process because
the window event loop cannot be recreated within one test process. `--list`
shows the checks, name fragments select them, `--skip-clipboard` leaves the
session clipboard alone, and `--timeout SECONDS` bounds each process. A check
that times out is reported as such. On macOS, window tests hosted by libtest
are unsupported because the event loop must be on the process main thread;
the standalone probe below runs on that thread. Do not record an unsupported
check as passed.

For a diagnostic two-window input trace, run:

```sh
cargo run --locked -p pert-ui-native-checks --bin pert-ui-focus-probe -- focus-probe.log 60
```

The log file must not already exist; the optional duration is 1–600 seconds.
Exercise focus transfer with held keys, modifiers, repeats, text composition,
minimize/restore and scale changes. Archive the log with OS/compositor,
hardware/driver, keyboard layout, IME, scale, actions performed and resolved
window dependency version. A successful probe exit means recording finished,
not that an input contract passed.

## Presentation workload

The gallery includes a mixed static/live workload: unchanged controls stay
mounted beside a changing meter and trace. It measures component work, layout,
text preparation, scene composition and presentation separately. Clipping
reduced composition work in the tested scene; the remaining full scene-order
walk and full render pass are known costs. Those local measurements motivate
future renderer work but do not establish a cross-platform performance target.
Reproduce the headless path with
`cargo run --locked -p pert-ui-gallery -- --headless`; native timing requires a
real display and recorded hardware environment.

The opt-in `pert-ui-wgpu::tests::native_damage_area_probe` compares renderer
candidates on a hardware GPU. It paints a retained grid component and a
separately identified changing component, then checks that the grid's paint
callback ran only once. It times the existing offscreen full, scissored and
retained-grid-copy fixtures. It also compares a direct full surface pass with a
test-only persistent target: redraw the known changed rectangle there and copy
the whole target to the surface in **one submission**. Both surface paths wait
for GPU completion. Acquisition calls are timed separately; the completion
wait can still include surface-image or compositor synchronization. Present
calls and pixel readback are outside the samples. Measurement order alternates
independently of trace color. The surface must support `COPY_DST`, or the test
reports that capability is unavailable. Run it on an unlocked desktop session
with:

```sh
cargo test --locked --release -p pert-ui-wgpu native_damage_area_probe -- --ignored --nocapture
```

A local release run on 2026-09-29 used Fedora 44 GNOME 50.5/Wayland, kernel
7.2.7-200.fc44.x86_64, Intel UHD Graphics 770 and Mesa Vulkan 26.2.3. Across
120 measured frames with the earlier offscreen-only probe, scene composition
took 3.9 ms and renderer preparation 10.4 ms. The complete passes took
104.0 ms, scissored passes 30.4 ms and the retained-grid copy plus changed
drawing 67.0 ms. These are totals for this workload and device, not pass/fail
thresholds. The test checked both changing
colors and full-frame pixel equality for each candidate.

The retained-grid fixture models a layer associated with the grid's `WidgetId`;
it does not implement layer tracking. Today, `WidgetId` identifies a retained
component in the runtime, but `DisplayList` commands carry neither their owner
nor a layer identity or revision. A real layered path would need a stable owned
render identity tied to the component incarnation, ordered layer composition,
and invalidation for movement, clipping, resize, theme changes and removal. A
grid and a changing line trace could then occupy distinct layers even when they
overlap. The probe's full-texture copy is one possible composition cost, not a
prediction for every layered design. Scene composition and renderer preparation
are reported separately from the candidate passes. The CPU plus GPU completion
times are local measurements, not GPU timestamps or cross-platform results.
The fixture has an opaque grid background and a translucent trace. After timed
frames, it compares retained-target pixels when the trace moves, shrinks,
disappears and reappears, using explicitly supplied old/new damage rectangles.
This checks the fixture's pixel behavior. It also checks a full redraw on theme
change followed by a partial trace update. A separate bare scene checks that a
scissored load pass differs from a full pass for both opaque and translucent
backgrounds when no fill restores the scene clear color. These are fixture
checks, not general damage detection or scene ownership. Clipping, surface recovery and
automatic fallback remain unverified.

An earlier local-only experiment tried public scene ownership metadata and an
incremental renderer. Its results did not establish the proposed 15% gain,
and its source is not part of this branch. The retained-grid fixture in this
branch has an extra offscreen submission and wait; its cost cannot establish
performance of a different component-layer compositor.

Five release runs of the final fixture on the same Fedora/Vulkan GPU on
2026-09-30 passed retained-target pixel checks; the actual presented surface
was not read back. Both paths used a surface configured with `COPY_DST`, unlike
the production full pass, so this is not a direct production-baseline
comparison. All five reported FIFO presentation, `Auto` alpha mode,
`COPY_DST | RENDER_ATTACHMENT` usage, a two-frame latency setting and scale 1;
the probe would reject a suboptimal acquisition.
Its p95 diagnostic time (scene composition, renderer preparation and one surface
submission through GPU completion) ranged 1.187–2.121 ms for direct full
rendering and 1.127–1.994 ms for damage plus surface copy. Per-run medians
ranged 0.894–1.089 ms and 0.824–1.033 ms respectively. Median surface
acquisition took 7.98–8.69 ms and 8.00–8.48 ms respectively. The overlapping
ranges do not establish a gain under the plan's promotion criterion. The
diagnostic wait blocks the UI thread and can include surface synchronization;
these numbers do not isolate GPU work or measure normal frame pacing. Refresh
rate, missed deadlines and the other native platforms remain unmeasured.

### Real-time content cost

The opt-in `pert-ui-wgpu::tests::native_realtime_workload_probe` measures what
content that changes on every frame costs each renderer phase. It draws a
changing signal as minimum/maximum rectangles per column (the technique of the
built-in `Trace`), as a stroked antialiased polyline and as a scrolled
heatmap `RasterImage`. Each frame is timed as update, scene composition, CPU vector
rasterization, vertex encoding plus resource upload, and one full offscreen
pass through GPU completion. Presentation is not measured. The probe checks
that every frame produces a new scene and that the first and last frames differ
in pixels; it sets no timing threshold. Run it on an unlocked desktop session
with:

```sh
cargo test --locked --release -p pert-ui-wgpu native_realtime_workload_probe -- --ignored --nocapture
```

Three release runs on 2026-09-30, on the Fedora 44 GNOME/Wayland, Intel UHD
Graphics 770 and Mesa Vulkan 26.2.3 environment above, at 960×640 and scale 1
with an 800×400 plot, gave these per-frame medians (ranges across runs):

| Workload | Rasterize | Prepare | GPU pass | Total |
|---|---|---|---|---|
| Columns, 4096 samples | 7–8 µs | 0.49–0.51 ms | 0.53–0.57 ms | 1.12–1.22 ms |
| Polyline, 800 points | 8.90–8.94 ms | 91–99 µs | 0.86–0.93 ms | 9.94–10.06 ms |
| Polyline, 4096 points | 14.97–15.04 ms | 101–104 µs | 0.89–0.90 ms | 16.25–16.31 ms |
| Waterfall, 512×256 | < 1 µs | 28–63 µs | 0.64–0.77 ms | 1.16–1.66 ms |

A changing polyline is rasterized from scratch on the CPU every frame, and that
alone takes most of a 60 Hz frame; at 4096 points the total reaches the
16.7 ms frame period on this machine. The columns and the waterfall stay near
1–2 ms. Rebuilding the content-identified waterfall image (sRGB
premultiplication and content hashing, inside composition) took 0.45–0.79 ms;
creating and uploading its texture was not a significant cost. These are local
measurements on one device, not cross-platform results or performance targets.

The table above predates the direct stroke coverage of 0.2.27. Five further
release runs on the same device on 2026-09-30, with that coverage, gave these
per-frame medians (ranges across runs); the columns and waterfall workloads
did not change:

| Workload | Rasterize | Prepare | GPU pass | Total |
|---|---|---|---|---|
| Polyline, 800 points | 1.49–2.54 ms | 60–102 µs | 0.70–0.85 ms | 2.30–3.55 ms |
| Polyline, 4096 points | 2.26–3.85 ms | 55–105 µs | 0.80–0.87 ms | 3.31–5.06 ms |

Rasterization times were bimodal across runs, each run landing near one of
the two ends of its range, which suggests which processor cores the test
thread ran on rather than a change in work; the probe does not pin threads.

The opt-in `pert-ui-runtime` test `stroke_phase_profile` splits the CPU cost of
one stroke into flattening, outlining, scan conversion and direct coverage,
and reports the mean active outline edges per sub-scanline. Run it with:

```sh
cargo test --locked --release -p pert-ui-runtime --lib stroke_phase_profile -- --ignored --nocapture
```

On that device it attributed the earlier cost to scan conversion, which grew
with the active edges: a steep alternating trace kept about 1600 edges active
per sub-scanline and took about 50–60 ms, against about 10–13 ms for direct
coverage.

### CPU rendering cost

`cargo run --release -p pert-ui-cpu --example frame_cost` renders a grid of
translucent panels, mask rows and filtered images that scales with the window
and reports the median of seven renders. On Fedora 44 with a 12th Gen Intel
Core i9-12900, Rust 1.99.0 release build, single thread, on 2026-10-05 it
measured:

| Size | Commands | Median | Frame |
|---|---|---|---|
| 640×480 | 120 | 13 ms | 1.2 MiB |
| 1280×800 | 384 | 41 ms | 3.9 MiB |
| 1920×1080 | 792 | 83 ms | 7.9 MiB |
| 2560×1440 | 1440 | 144 ms | 14.1 MiB |
| 3840×2160 | 3168 | 322 ms | 31.6 MiB |

The whole run peaked at 50 MiB resident. A headless gallery screenshot of the
1280×800 window, including mounting and layout, took 0.09 s and peaked at
40 MiB. This is a reference interpreter for offline rendering and captures; no
interactive frame budget is claimed.

### Desktop build configurations

On 2026-10-18 a small notes application (menu bar, list, text editor, a
background save and a CPU-rendered capture example; a session-local program,
not in the repository) was built against pert-ui at commit `ace22d0` from an
empty target directory with `cargo build --release`, the default release
profile of a standalone package, in each desktop configuration: `pert-ui` with
`features = ["cpu"]`, and with `default-features = false, features =
["desktop-cpu"]`. The machine was a 12th Gen Intel Core i9-12900 (24 hardware
threads) on Fedora 44 with Rust 1.99.0. "Stripped" is `strip -o` of the release
binary; build time is wall-clock time of the build command:

| Configuration | Packages in the normal dependency tree of `pert-ui` | Clean release build | Binary | Stripped |
|---|---|---|---|---|
| `desktop` (with `cpu`) | 210 | 32 s | 25.2 MiB | 20.3 MiB |
| `desktop-cpu` | 170 | 34 s | 19.0 MiB | 15.6 MiB |

`desktop-cpu` leaves wgpu and its graphics stack out, which made the program
about a quarter smaller. It did not shorten the build on this machine, where
wgpu compiles in parallel with the rest; a build with fewer threads was not
measured. Package counts are from `cargo tree -e normal` for `pert-ui` alone.

## Review and publication

The workspace has undergone a file-by-file API, invariant and boundary review.
The historical Git review record was removed before the first public prerelease;
a new review should inspect current code, generated rustdoc, examples, tests and
the governing contracts rather than treating an older sign-off as current
proof. High-risk changes need an independent fresh-context review before
merge or release. Missing reviewers or platforms are recorded as missing
evidence, not silently marked as passed.

Before a public release, verify packaged library contents, assets and
licenses, run documented examples and a custom widget as an isolated consumer,
and perform relevant native checks on the supported matrix. Preparing or
packaging the libraries does not publish them. Publication needs an explicit
instruction.
