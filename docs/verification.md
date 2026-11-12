# Verification and evidence

This record states what the current checks establish, the native evidence per
platform and what is still missing, and local measurements with their limits.
The commands follow [AGENTS.md](../AGENTS.md) and `.github/workflows/ci.yml`;
that workflow is the authoritative current gate. It is not a history: what
each release changed is in the [changelog](../CHANGELOG.md), and earlier
versions of this record, with the per-plan evidence of completed plans, are in
Git history (this file at commit f5fde76).

## Workspace gate

Run the smallest gate covering the change. The verification table and the
workspace gate commands are in [AGENTS.md](../AGENTS.md) under
"Verification and review", and `.github/workflows/ci.yml` is authoritative:
it runs formatting, Clippy, the workspace tests, the packaged examples'
headless sessions, the doctests, strict rustdoc, the diagnostics snapshots and
the API listing gate. This page explains those checks rather than repeating
the command list, so the list is kept in step in one place beside the
workflow.

The `pert-ui-diagnostics` package (`tests/diagnostics`, never published)
holds `trybuild` snapshots of the compiler messages pert-ui shapes with
`#[diagnostic::on_unimplemented]` on its extension traits and the seals of
`StateOwner`, `ParameterValue` and `LiveItem`, and with
`#[diagnostic::do_not_recommend]` on `AppError`'s blanket conversion. rustc's wording changes between releases, so the
snapshot test is ignored in the runs above, which compile it and skip it;
CI's `diagnostics` job runs it on Linux with the toolchain
`rust-toolchain.toml` pins:

```sh
cargo test --locked -p pert-ui-diagnostics -- --ignored
```

A changed message or a toolchain update regenerates the snapshots with
`TRYBUILD=overwrite` and the same command, and the `.stderr` diff is reviewed
like the `api/` listing. The contract in a snapshot is what pert-ui writes:
the `message`, `label` and `note` texts of its diagnostic attributes. The rest
of rustc's output around them is reviewed as evidence but is not a promise.
In particular, several snapshots (the `Component` cases `c01` and `c06` among
them) include rustc's list of the first implementors of a trait. A new
built-in type that sorts early changes that list without any change to a
pert-ui message. That churn is expected: regenerate the snapshots and check
that the diff touches only the implementor list. A `compile_fail` doctest is different evidence: on
stable Rust, rustdoc does not check the error code a `compile_fail,E0XXX`
block names, so the doctest passes for any compile error, a typo included. It
proves a type-level restriction that must hold on every toolchain, such as
`StateOwner` being sealed, not the message.

The same package holds the `testing` module's closure check
(`tests/testing_closure.rs`). It reads the committed API listings under
`api/` for the runtime and for the facade's default, `cpu`, no-default and
`desktop-cpu` configurations, starts from the types `testing` exports and the
`Runtime` methods its names exist for, follows their methods, associated
constants, fields and variant payloads across the pert-ui packages, and fails
for every reached type that is neither in `testing`, nor in the prelude, nor
on its reviewed exclusion list; it also fails on a listing line it does not
recognize and on an unresolved or ambiguous type name. It is not ignored, so
it runs in `cargo test --locked --workspace`. It does not run
cargo-public-api: the API listing gate keeps its input current, so a change
that forgets to regenerate the listings fails CI's `api-listing` job, not
this test.

Changes to the core live-data protocol additionally run the release-profile
Loom models named in AGENTS.md. Dependency, feature, platform and release
changes also need the release tests, feature configurations, headless gallery
and packaging steps of the workflow's `headless` job, and the MSRV and
isolated-consumer checks AGENTS.md names. A passing headless
gate is evidence for contracts that do not require a native session; it is not
evidence for GPU output, focus routing or platform integration.
CI runs `cargo clean` between the base, image/SVG, serde/CPU and packaging
groups to reclaim disk space before the next group, and builds the dev and test
profiles without debuginfo or incremental caches; it retains every check. The
facade's integration tests are grouped into a few area binaries under
`crates/pert-ui/tests/<area>/main.rs`, so each feature configuration links a
few test binaries instead of one per test file.

### Text engine and toolchain

The toolchain and declared minimum Rust version are 1.99.1 since 0.3.52; that
point release needed no source change. Parley is at 0.12.0. Its
[release](https://github.com/linebender/parley/releases/tag/v0.12.0)
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
(`Semantics::protected`), must report no value. A disabled component offers no actions, and neither does a disabled item.
A QuickPick or CommandPalette result reported disabled still delivers a
click or Enter to the choose callback, because choosing it is where the
application explains why it is unavailable (see `PickItem`); its item offers
no semantic action.

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

The latest full run passed **all 19 checks** of `cargo run --locked -p
pert-ui-native-checks` on 2026-11-05 at 9655e56 (0.3.101): Fedora 44, kernel
7.2.15-200.fc44.x86_64, GNOME Shell and Mutter 50.6 on Wayland, Intel UHD
Graphics 770 (ADL-S GT1) with Mesa 26.2.6 on Vulkan, rustc 1.99.1, winit
0.30.13 and wgpu 30.0.1, the clipboard check and the workspace restart
included. Earlier runs also used an Intel Iris Xe (RPL-P) and an Intel Arc
A370M on Fedora 44; their environments were recorded incompletely. Every row
below is Fedora 44 GNOME/Wayland/Vulkan only; Windows and macOS have no
evidence for any of them.

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
| Tool window floating frames and own native windows (Layout › Tool windows: float, drag the header and the corner grip, open in a native window and close it back to the stripe, dock from the options menu) | Steps of the gallery interaction check since 2026-10-05; window creation and closing were the real host's. |
| Gallery interaction check (`pert-ui-gallery::interaction_check` and its CPU twin) | Passed in every full native run from 2026-10-02 to 2026-11-11 (39 steps). On 2026-11-12 (0.3.108, where built-in widgets record `DrawShape`) the CPU twin passed its 39 steps and the GPU check first failed with a clock error, as it did at 0.3.107 the same day: five steps clicked through `testing::Driver`, which advances the runtime's clock one test frame per action, and a window loop that came back sooner than that had its real-time flush refused. The steps now click without moving the clock, and the GPU check passed its 39 steps in three runs and the CPU twin in one, each run by itself; the rest of that day's native run passed, the GPU readback and parity checks included. The steps are listed in `apps/gallery/src/interaction_check.rs`. Input enters at the runtime's entry points, so it is not evidence for the adapter's OS event translation. |
| Software presentation (`App::software_rendering`): capture, resize, minimize/restore, input, recovery, floating windows, detach/redock and settled idle | Passed 2026-10-05 with softbuffer over Wayland shared memory and no GPU device created. The maintainer then checked the `--cpu` gallery by hand: real pointer, keyboard and text input, the clipboard, and display scales from 75 % to 200 %. |
| `desktop-cpu` build (no wgpu) | Passed 2026-10-18: each ignored window check of `pert-ui` built with `--no-default-features --features desktop-cpu` ran in its own process (startup, resize and idle, capture, drag, three software surfaces with redock). The pacing check needs `gpu`; CPU frames have no interactive budget. The native runner builds only the default configuration. |
| Native window retitle (`PresentationAction::Retitle`) | Passed 2026-10-07 in GPU and software presentation as a step of the three-surface checks. winit cannot read a title back on X11 or Wayland, so the shown title bar is not asserted. |
| Native close request with a focused draft (`a_native_close_request_saves_the_focused_draft_before_the_policy`) | Passed since 2026-10-23: `WindowEvent::CloseRequested` for the real main window, delivered through the runner's event handler, saves the draft before `App::on_close`, and focus returns after `KeepOpen`. No close button is pressed. |
| Application state from the initializer, kept behind the native runner's erased owner | Passed 2026-10-28: the window, three-surface and close-request checks read `on_error`, `on_window_error` and `on_close` state, and the probes that overlap on purpose fail on an unexpected overlap. |
| Hand checks at 37a8c9e and 83da271, 2026-10-28 | Passed by the maintainer: partial terminal progress fills (Controls › Progress); rows that shrink and wrap and a narrowed Progress cell; sortable table titles, a partly cut title and its grip, and the Note editor at all three densities; `diagram_editor --window` closing with a moved node and with a refused Height draft (Cancel keeps it, Escape then closes); Revert of a refused draft in the gallery's sign-in form. |
| Round 3 repairs (0.3.92 to 0.3.94): parked menu ownership, MultiSelect press focus and Escape, empty-raster bounds and the wrapped caret | Selected GPU and CPU gallery interaction checks and the text-mode probe passed 2026-10-29. Runtime-injected input only; no OS-adapter or hand claim. |
| Hand checks at 9655e56 (0.3.101), 2026-11-05 | Passed by the maintainer, GPU presentation: one centered label after renaming a node in `diagram_editor --window`; a tool window described with `ToolWindowMode::Window` opens in its own native window at start, and disabled `Select` and `MultiSelect` anchors match a disabled `Button` (a temporary program outside the repository, since no shipped application has either); the `diagram_editor`, `log_explorer --lines 10000` and `inventory` windows' main tasks, including double-click and F2 cell editing and Tab between editable cells; `SelectableText` selection copied into another application. IME composition into a cell editor was **not run** ([follow-ups](../plans/follow-ups.md)). |
| Minimize during a gallery probe (NATIVE-04) | **Failed** 2026-11-05: a window minimized with Super+H is never reported hidden. The GPU interaction check failed on an unrelated step, the CPU one timed out and `workspace_restart` passed unaware. winit 0.30.13 delivers no `WindowEvent::Occluded` on Wayland, and the surface did not report `Frame::Occluded`. |
| CPU/GPU parity (`pert-ui-wgpu::tests::native_cpu_gpu_parity`) | Passed: the exact tier matched byte for byte and the tolerance tier differed by at most one per channel. No other adapter, and no CI software-driver comparison, has run. |
| Analytic shape parity (the shape fixtures of the same check) | Passed 2026-11-11, run by itself: over rounded and cut corners, borders, arcs, triangles and clips at sub-pixel phases and scales 1, 1.25 and 2, and geometry up to 65 000 pixels from its span, decided pixels kept their tier and edge pixels differed by at most one per channel, which set the edge tier. The gallery's GPU interaction check passed the same day (39 steps); built-in widgets do not record `DrawShape` yet, so it is evidence that the pipeline's presence disturbs nothing, not for shapes on screen. No other adapter has run. |

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

## Rendering checks

### Thin outlines at fractional zoom

A 75% scale regression renders a one-logical-pixel rounded outline at a
fractional origin and checks a pixel on each straight side. The CPU oracle and
the native CPU/GPU parity case passed on Fedora 44 Wayland with Intel UHD
Graphics 770 (Vulkan); the parity case differed by at most one 8-bit channel.
This checks stroke visibility and backend agreement for that fixture, not every
control geometry or zoom phase.

### Text rendering modes

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

Five release runs on 2026-09-30, on the Fedora 44 GNOME/Wayland, Intel UHD
Graphics 770 and Mesa Vulkan 26.2.3 environment above, at 960×640 and scale 1
with an 800×400 plot and the direct stroke coverage of 0.2.27, gave these
per-frame medians (ranges across runs):

| Workload | Rasterize | Prepare | GPU pass | Total |
|---|---|---|---|---|
| Columns, 4096 samples | 7–8 µs | 0.49–0.51 ms | 0.53–0.57 ms | 1.12–1.22 ms |
| Polyline, 800 points | 1.49–2.54 ms | 60–102 µs | 0.70–0.85 ms | 2.30–3.55 ms |
| Polyline, 4096 points | 2.26–3.85 ms | 55–105 µs | 0.80–0.87 ms | 3.31–5.06 ms |
| Waterfall, 512×256 | < 1 µs | 28–63 µs | 0.64–0.77 ms | 1.16–1.66 ms |

Before that coverage, a changing polyline was rasterized by scan conversion
and took 9.9–10.1 ms in total at 800 points and about 16.3 ms at 4096 points,
the whole 60 Hz frame period on this machine. Rebuilding the
content-identified waterfall image (sRGB premultiplication and content
hashing, inside composition) took 0.45–0.79 ms; creating and uploading its
texture was not a significant cost. These are local measurements on one
device, not cross-platform results or performance targets.

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

## Example applications

The packaged examples' headless sessions run in the workspace gate; what each
session drives is in its source. Local release-profile measurements on Fedora
44 x86_64 (24 threads):

- `log_explorer` over 50 000 generated lines (0.3.31): loading about 36 ms, a
  wheel step 4.13 ms, sorting 25 ms, content search 7 ms and an inspection
  snapshot from another thread 3 ms, with at most about 600 components
  mounted. Debug-profile development validation reported no unbounded
  collection.
- `inventory` over 10 000 and 100 000 generated items (0.3.44): 752 mounted
  components at both sizes; Enter opened an editor in about 2 ms, a save took
  5 to 9 ms, and a save that re-sorts 100 000 rows and moves the edit about
  66 ms.

On 2026-10-20 (0.3.51), with temporary instrumentation that is not kept, the
`log_explorer` wheel step was split; each value is the median of three runs:

| Lines | Wheel step | Text preparation | Shapings | Shaping | Rasterizations | Rasterization |
|---|---|---|---|---|---|---|
| 50 000 | 4.28 ms | 2.12 ms | 49.4 | 0.92 ms | 19.9 | 0.96 ms |
| 200 000 | 4.28 ms | 2.12 ms | 49.3 | 0.92 ms | 19.8 | 0.96 ms |

Every step prepares the newly mounted rows' cells, so neither the layout
cache nor raster reuse answered any of them. The step does not grow with the
log's length, since only the mounted rows prepare text.

## Usability studies

Three rounds of blind tests had AI testers (Claude Code and Codex CLI) build
small applications from the packaged documentation in a sandbox with no
network beyond the model hosts and no access to the repository: round 1 on
2026-10-21 against 0.3.52, round 2 on 2026-10-27 against 0.3.75, and round 3
against a frozen 0.3.88, closed on 2026-10-29 after its repairs through
0.3.94. Every run finished well inside its 90-minute budget with its
acceptance met or partly met, and each round's findings were repaired before
the next. With two testers and one run per task, the evidence is
exploratory: the times and counts describe how each tester worked, not a
usability score. The harness is described in
[plans/usability/README.md](../plans/usability/README.md); the run records
are in Git history.

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

The component API review completed on 2026-10-05 with an individual
assessment and explicit API exceptions for every public component. Local
packaged-consumer checks, building documented examples from extracted
packages outside the checkout without workspace patches, have passed at
several releases; they are not a crates.io dry run. Core's crates.io publish
dry run passed at that review; the interdependent packages' dry runs need
registry-resolvable versions.
