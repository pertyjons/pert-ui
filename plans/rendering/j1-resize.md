# J1: UI cost of a window resize or scale change

Investigation of the phase 0 finding "Resize and scale-change UI cost"
([phase0-report.md](phase0-report.md#findings-outside-this-scope)). It
locates the cost; it does not fix it. Code references are to `afa383a`
(`rendering-phase0`), whose `crates/` equal `8254019` (0.3.102).

## Summary

Four causes explain the 19–117 ms. Two of them account for nearly all of it:

1. **The visibility walk (RESIZE-01).** After every component update,
   `refresh_child_visibility` walks the component's whole subtree, hidden
   pages included. A resize updates every visible component, so the
   ancestors of the visible page each walk nearly all 4,112 mounted nodes:
   84,477 visits per frame. This is the "Flush self time" on every page.
2. **Invalidating everything on a resize (RESIZE-02).** Any viewport change,
   including a width-only one, invalidates every node for layout. Every
   visible component updates, and every retained measurement is dropped.
3. **Text cache thrash (RESIZE-03).** The live page's 1,000 checkboxes need
   1,029 distinct text layouts per frame. The text cache holds 128 entries,
   evicts FIFO and is searched linearly. 99.2% of 3,068 text preparations per
   frame miss, and each miss reshapes and rerasterizes.
4. **Rasterizing during measure (RESIZE-04).** Text prepared in `measure` is
   rasterized at once, though only about 144 components paint. Rasterizing is
   about two thirds of each miss.

With RESIZE-01..03 removed, a resize costs 0.5–2.2 ms instead of 18–94 ms.
A scale step costs 3.5–49 ms instead of 21–114 ms. RESIZE-04 is the main
remainder on the live page.

## Outcome (0.3.103)

All four causes are fixed in 0.3.103:

- **RESIZE-01:** the visibility walk stops where visibility is unchanged.
- **RESIZE-02:** a same-scale resize invalidates only explicit readers of
  the window extent and the window-placed surfaces. The readers are
  `MeasureContext::viewport`, `UpdateContext::viewport` and
  `Painter::viewport`; `Painter::pixel_scale` reads the scale alone.
- **RESIZE-03:** the text cache is an LRU of 4,096 entries with hashed key and
  signature indexes, within the unchanged 16 MiB budget.
- **RESIZE-04:** preparation places glyphs from outline bounds and renders
  coverage on first paint. A test checks that measured placements equal
  rendered ones across fonts, scales, icons and bidi text.

A Codex review of the fixes found no blocker or major defect. It found three
minor issues and one nit, all corrected: the `paint_turned` and
`Painter::pixel_scale` rustdoc, a missing paint-path test for lazy coverage,
and timing evidence in the changelog.

Measured with the probe of this note, without the instrumentation patch, in
the same session: HEAD `b053273` ("before") against the fixed tree
("after"). Each run was pinned to CPU 2, with the profiler on as before.
The figures are medians in ms of UI work per frame; each is the mean of two
runs, which agreed within 4%. The 1-minute load average was 11–13 during
the runs, so the absolute figures may be high. The "before" column
reproduces the pristine column above within 3%.

| Scenario | before | after |
|---|---|---|
| navigation resize | 21.3 | 2.1 |
| menus resize | 18.4 | 0.76 |
| live resize | 94.0 | 2.95 |
| navigation scale step | 27.6 | 7.75 |
| menus scale step | 20.7 | 3.8 |
| live scale step | 114.5 | 22.2 |

## Method

- **Probe.** [`j1-probe/probe.rs`](j1-probe/probe.rs) is the phase 0 change
  scenario, run headless, with no wgpu. It uses the same gallery, page, 1280×800
  window, +7 px per frame for 30 frames, scale steps 1→2 and profiler. It also
  reports per-frame counters.
- **Instrumentation.** The counters come from
  [`j1-probe/instrumentation.patch`](j1-probe/instrumentation.patch), a
  throwaway thread-local timer and counter module in `pert-ui-runtime`. The
  patch also adds four switches, read from `J1_FLAGS`, that each remove one
  cause:
  - **E1:** `refresh_visibility` stops descending where visibility did not
    change.
  - **E2:** a resize that keeps the scale skips the invalidate-all loop.
  - **E3:** the text cache gets a hashed index and 4,096 entries; the byte
    budget is unchanged.
  - The switch for RESIZE-04 was not built; its gain is inferred.

  The patch was reverted; nothing is committed.
- **Pristine baseline.** HEAD sources extracted with `git archive` and the
  same probe, without the counters.
- **Pinning.** All timings in the table run pinned with `taskset -c 2`, a P-core
  thread. Unpinned runs land on E-cores and take 1.4–1.8× longer. All CPUs
  reported `cpuinfo_max_freq` = 2.401 GHz during the runs.
  - Pinned, the pristine baseline reproduces phase 0: navigation resize 21.2
    vs 21.5 ms, scale 27.0 vs 27.0 ms; live resize 92.7 vs 97.0 ms, scale
    113–114 vs 117.4 ms.
- **Repeats and load.** Two repeats agree within 1–3%
  ([j1-probe/results-pinned.txt](j1-probe/results-pinned.txt)). The 1-minute
  load average was 1.0–1.8 on 24 threads during the pinned runs.
- **Instrumentation overhead.** The instrumented build with timers off ("none")
  matches pristine within 1%. Timers on add 15–20%: the timed figures in
  "Evidence" are inflated, and their ratios and counts are what matter.

## Causes and measured share

Medians in ms per frame of UI work (`set_viewport` + `flush` + `paint`),
pinned. "none" is the instrumented build with every switch off.

| Scenario | Pristine | none | E1 | E1+E2 | E3 | E1+E3 | E1+E2+E3 |
|---|---|---|---|---|---|---|---|
| navigation resize | 21.0 | 21.3 | 3.3 | 2.2 | 20.5 | 2.5 | 1.8 |
| menus resize | 18.2 | 18.4 | 0.9 | 0.5 | 18.4 | – | 0.5 |
| live resize | 92.8 | 93.8 | 73.9 | 47.9 | 24.8 | 4.6 | 2.2 |
| navigation scale step | 27.2 | 27.6 | 9.4 | n/a | 27.9 | 9.6 | n/a |
| menus scale step | 20.5 | 20.8 | 3.5 | n/a | 20.8 | 3.5 | n/a |
| live scale step | 113.8 | 114.5 | 94.2 | n/a | 69.6 | 49.5 | n/a |

E2 applies only when the scale is unchanged, so it has no scale-step column.
Each figure is the mean of two runs' medians, except the E1+E3 resize figures,
which come from a single run. A dash marks a combination not run.

| ID | Cause | Location | Share removed (measured) |
|---|---|---|---|
| RESIZE-01 | Visibility walk over whole subtrees after every update | `runtime/tree.rs:1580-1592, :1670`, called from `runtime.rs:6379` | resize: 85% navigation, 95% menus, 21% live; scale: 66% / 83% / 18% |
| RESIZE-02 | A same-scale resize invalidates every node for layout | `runtime/settings.rs:528-538, :639-645` | On top of E1: 1.1 ms navigation, 0.4 ms menus, 26 ms live (28% of baseline). On top of E1+E3: 2.4 ms live |
| RESIZE-03 | Text cache: 128 entries, FIFO, linear search | `text.rs:586, :729, :1096, :1110, :1151` | live resize 74% alone (93.8 → 24.8 ms); live scale 39% |
| RESIZE-04 | Measured text is rasterized in full, painted or not | `text.rs:1160` via `layout.rs:912, :1013` | not built; inferred up to ~20 of the remaining 49.5 ms on a live scale step |

The shares overlap: RESIZE-02 multiplies the work RESIZE-01 and RESIZE-03
make expensive.

## Evidence per cause

### RESIZE-01: visibility walk (the Flush self time)

Measured (navigation resize, timers on):

- Flush self time is 26 ms (unpinned), of which `refresh_child_visibility`,
  called at the end of `update_component` (`runtime.rs:6379`), takes 25 ms.
  Everything else in `update_component` takes 0.4 ms across 140 updates.
- The walk makes 84,477 `refresh_visibility` calls per frame for 140 updates;
  the live page makes 95,595 for 1,085 updates. The whole tree has 4,112 nodes
  and the visible page 144.
  - `refresh_visibility` (`runtime/tree.rs:1593-1671`) always ends with
    `refresh_child_visibility(serial)` (`:1670`), even when the node's
    visibility did not change.
  - So each updated node walks its entire subtree, including every hidden
    gallery page. Every visible ancestor of the shown page (root, tabs,
    category, page strip, …) walks nearly the whole tree.
- Cost per visit is about 0.21 µs pinned: 18.0 ms / 84.5k visits. Each visit
  clones `children` (`:1581`), calls `child_visible` dynamically and makes
  several `BTreeMap` lookups.
- The total therefore scales with mounted nodes × ancestor depth, not with
  the visible page. That is why Flush self time "barely depends on the page".

Inferred:

- The walk is not specific to resizing. Any update of a container high in the
  tree, such as a root redescription or a theme change, pays a walk of its
  whole subtree, hidden pages included.

Safety evidence for the fix (E1):

- The runtime's unit tests (213) and every facade test binary (2,775 tests)
  pass with E1.
- Every writer of `local_visible` already refreshes that node itself
  (`runtime.rs:5589-5598`, `native_windows.rs:372, :645, :734`,
  `tree.rs:921`). Descendants depend only on their parent's `visible` and on
  the parent's `child_visible` answer. That answer is re-read in
  `refresh_child_visibility` for direct children after each update.

Governing rules:

- AGENTS.md "Scheduling and real-time boundaries": "no … hidden-view
  polling"; "Visibility must stop unnecessary presentation work".
- `docs/architecture.md:59-60`: hidden views suspend presentation work.

### RESIZE-02: a same-scale resize invalidates every node

Measured:

- **The trigger.**
  - `change_viewport` (`runtime/settings.rs:528-538`) sets `geometry_change`
    for any viewport change, not only a scale change.
  - `apply_geometry_change` (`:564-650`) then invalidates every node in the
    window with `Invalidation::Layout` (`:639-645`): 4,112 nodes per resize
    frame on navigation and 4,116 on live.
  - That queues every visible component for `update` and resets every
    retained `Measurements` (`runtime.rs:1056-1070`).
- **E2 skips the loop when the scale is unchanged.**
  - navigation: measures 377 → 110 and text preparations 210 → 74 per frame;
  - live: text preparations 3,083 → 2,034.
- **A side cost.** `invalidate_layout` re-walks every ancestor without an early
  exit (`runtime.rs:1067-1068`): 39,542 calls for 4,116 nodes on live, about
  1.1 ms. A fix of this cause removes it for resizes. Scale steps keep it.

Why the fix needs more than E2:

- E2 passes every test, but that shows a coverage gap, not safety. Both
  `MeasureContext::viewport` (`layout.rs:854-857`) and `UpdateContext::viewport`
  (`runtime.rs:3099`) read the window size without subscribing.
  - The rustdoc of the first relies on "Native resize and UI zoom invalidate
    layout before this value is consumed again".
  - Built-in readers include toasts (`notifications.rs:1072`), the quick pick
    (`quick_pick.rs:1338`), menus, popovers and the toolbar overflow.

Governing rules:

- AGENTS.md: "Retain layout/text/drawing data for unchanged widgets. Binding
  changes invalidate only their dependencies. No … whole-root rebuild".
- `docs/architecture.md:62-66`: "A changed binding invalidates only its
  readers … The runtime retains text, layout and drawing data that did not
  change."

### RESIZE-03: the text cache thrashes on the live page

Measured (live resize, timers on, pinned):

- **Working set.** The live page is 1,000 `Checkbox`es labelled "Control
  0001"… in a `Wrap` in a `ScrollArea` (`apps/gallery/src/live.rs:28`,
  `:430-435`). Each frame:
  - 3,068 plain text preparations, 1,029 of them distinct (text, width,
    scale): about 3 per checkbox, one per measure pass.
  - Hits: 18.8 fast and 5.9 resolved (0.8%). The cache held 128 entries at
    every miss.
- **Why it misses.** `TextService::cache` (`text.rs:586`) is a `VecDeque`.
  - Capacity: 128 entries (`:1110`) within a 16 MiB budget (`:28`).
  - Eviction is FIFO from the front, and a hit does not refresh its entry.
  - Lookups scan linearly: `cached_plain` (`:729`), the key lookup (`:1096`)
    and the signature-reuse lookup in `shape` (`:1151`).
  - The measure passes interleave over all 1,000 children, so the second and
    third preparation of a label always finds it evicted.
- **Per frame:** 3,060 misses: 64 ms in `shape`, of which Parley layout 18 ms
  (29%) and `rasterize_glyphs` + `composite` 43 ms (67%).
- **Width is not the cause.** Only 2.8 misses per frame found the same text
  cached at another width.
- **E3:** a hashed index and 4,096 entries leave Text at 0.5 ms per frame on a
  resize. On a scale step, each distinct text is prepared once instead of
  three times (2,037 fast hits).
  - With E3, the signature-reuse scan in `shape` (`:1151`) still walks about
    2,300 entries per miss. About 9 ms of a live scale step's `shape` time lies
    outside layout and rasterizing. Inferred: mostly that scan. A fix must
    index it too.

Governing rule: `docs/rendering-platform.md:160`, "Text shaping, layout and
coverage preparation are retained and bounded."

Related records:

- RENDER-01 in TODO.md records the same FIFO and linear-scan pattern in
  `ShapeCache`.
- `docs/verification.md:271-277` records that raising the PhaseLine cache from
  128 to 2,048 entries fixed the same thrash for line coverage.

### RESIZE-04: text is rasterized in measure

Measured:

- `MeasureContext::text`/`rich_text` (`layout.rs:912, :1013`) return a full
  `TextLayout`. On a miss, `shape` always rasterizes and composites the
  coverage (`text.rs:1160-1167`).
- On a live scale step with E1+E3, 1,030 misses per step spend:
  - Parley layout: 7.5 ms;
  - `rasterize_glyphs` + `composite`: 23.2 ms (timers on).
- Paint runs for 144 components per frame. The 1,000-row `Wrap` measures every
  child, but the `ScrollArea` shows only a fraction.

Inferred:

- About 85% of that rasterization is coverage that is not painted in the frame.
- Rasterizing lazily, at first paint, would save up to about 20 ms of the
  remaining 49.5 ms per scale step, and the same share of the page's first
  layout.

### Not causes, or minor

- **Coverage masks.** Phase 0 already excluded them; confirmed here: `Paint`
  totals 0.1–0.5 ms per frame.
- **Component `update` bodies.** 0.1–0.3 ms per frame, even with all 1,085
  live components updating. The cost of an update lies around it
  (RESIZE-01), not in it.
- **`Wrap` passes its full constraints to every child**
  (`containers.rs:499`). Every width change therefore re-measures all 1,000
  checkboxes, two passes each, even with E2. Once text hits (E1+E2+E3), this
  is 1.0 ms of Measure self time. It is an observation, not an entry.

## Draft TODO.md entries

```markdown
- [ ] **RESIZE-01** 2026-11-06 `crates/pert-ui-runtime/src/runtime/tree.rs:1580-1671`
  `Runtime::refresh_visibility` — Scheduling and real-time boundaries
  (hidden views do no presentation work; `docs/architecture.md:59-60`): it
  always ends with `refresh_child_visibility`, even when the node's visibility
  did not change. Every `update_component` (`runtime.rs:6379`) therefore walks
  the updated component's whole subtree, hidden pages included. A gallery
  resize makes 84,477 visits for 140 updates in a 4,112-node tree: 18 ms of
  the 21 ms frame on the navigation page and 17 of 18 ms on menus. Descend only
  into children whose visibility changed. Re-read `child_visible` for the
  direct children of the updated component as now. Keep the full walk only for
  structural callers that need it (mount, redock, native window visibility).
  Add a test that updating a parent does not call `child_visible` on the
  descendants of an unchanged hidden child. A custom component can count the
  calls through the public `Component::child_visible`.
- [ ] **RESIZE-02** 2026-11-06 `crates/pert-ui-runtime/src/runtime/settings.rs:528-538, :639-645`
  `WindowShared::change_viewport` / `Runtime::apply_geometry_change` —
  Scheduling and real-time boundaries ("Retain layout/text/drawing data for
  unchanged widgets", "no whole-root rebuild"; `docs/architecture.md:62-66`):
  a width- or height-only viewport change invalidates every node for layout.
  Every visible component updates, and every retained measurement and its
  text are dropped. On the live gallery page, keeping them saves 26 ms of 93 ms
  per resize frame, or 2.4 of 4.6 ms once RESIZE-01 and RESIZE-03 are fixed.
  The invalidation also re-walks ancestors per node (39,542
  `invalidate_layout` calls for 4,116 nodes). Track viewport reads, as
  `UpdateContext::motion` tracks its readers:
  - `MeasureContext::viewport` (`layout.rs:854-857`, whose rustdoc relies on
    the blanket invalidation) records a measurement dependency;
  - `UpdateContext::viewport` (`runtime.rs:3099`) records an update
    subscription.
  On a same-scale change, invalidate only the roots and those readers. Keep
  the gesture cancellation that `set_viewport` documents, and the full
  invalidation for scale and zoom changes. Add tests: a toast or quick pick
  re-measures on a resize; an unrelated component neither updates nor
  re-measures.
- [ ] **RESIZE-03** 2026-11-06 `crates/pert-ui-runtime/src/text.rs:586, :729, :1096, :1110, :1151`
  `TextService::cache` — Improvement (`docs/rendering-platform.md:160`, text
  preparation is retained and bounded): the cache holds 128 entries, evicts
  FIFO without refreshing hits, and is searched linearly in `cached_plain`,
  `prepare_resolved` and `shape`'s signature reuse. The live gallery page needs
  1,029 distinct layouts per frame: 99.2% of its 3,068 preparations miss,
  each reshaping and rasterizing, for 68 of the 93 ms of a resize frame.
  Index the entries by a hash of the key and of the raster signature, move a
  hit to the back, and raise the entry cap under the existing 16 MiB budget.
  A hashed 4,096-entry cache cut that frame to 25 ms, and to 2.2 ms together
  with RESIZE-01/02. Add a test that a working set above 128 entries keeps
  hitting. See RENDER-01 for the same pattern in `ShapeCache`.
- [ ] **RESIZE-04** 2026-11-06 `crates/pert-ui-runtime/src/text.rs:1160-1167`
  `TextService::shape` — Improvement: every text preparation rasterizes and
  composites its coverage, including those `measure` makes through
  `MeasureContext::text`/`rich_text` (`layout.rs:912, :1013`) only for a size.
  On a live-page scale step, 1,030 preparations spend 23 ms rasterizing
  against 7.5 ms shaping, while 144 components paint. Keep the shaped layout
  and rasterize coverage on first paint (lazy parts in `TextLayout`), or give
  measurement a shape-only path, as `plain_shape` already does for fitting.
  Expected, not measured: up to about 20 ms of the remaining 49.5 ms per scale
  step, and the same share of a page's first layout.
```

## Reproduction

The probe needs no window.

1. Apply `j1-probe/instrumentation.patch`.
2. Build a scratch crate as `baseline-probe/run.sh` does, with
   `j1-probe/probe.rs` as its `probe` module. Add `pert-ui-runtime` as a path
   dependency so the probe can reach the throwaway counters.
3. Run it pinned:

   ```sh
   J1_FLAGS=E1,E3 PROBE_J1=0 PROBE_CHANGE_PAGE=live PROBE_SCENARIO=resize taskset -c 2 <binary>
   ```

   - `PROBE_J1=0` turns the timers off.
   - `PROBE_SCENARIO` is one of `resize`, `scale`, `idle` or `toggle`.
