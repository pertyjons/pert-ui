needs rework

The separate `DocumentEditor` over an application-owned `TextDocument` value is a sound direction. Most current-code claims and the references to **9aaacca** check out. The main problems are in the proposed correctness guarantees and implementation sequence.

**Blocker**

1. **Paragraph independence does not establish parity with today’s implementation.** [plans/document-editor.md:313](/home/per/github/pert-ui/plans/document-editor.md:313) asserts identical analysis, and `:396` asserts identical hard-break movement. Unicode’s paragraph independence supports the architecture, but does not prove these implementation claims. Today, `text.rs:1246` builds one Parley layout and `:1307` applies `Alignment::Start`. The pinned Parley 0.12 implementation stores one layout-wide base level (`parley/src/builder.rs:368`) and uses it for every line’s alignment (`parley/src/layout/alignment.rs:76–121`); its analyzer resolves the supplied text in one call (`parley_engine/src/analysis.rs:811–822`). Separately prepared LTR and RTL paragraphs can therefore behave differently from that single layout. [UAX #9](https://www.unicode.org/reports/tr9/) specifies independent paragraphs, but is not evidence of current Parley parity.

   **Change:** replace the unconditional equivalence claim with an intended contract. Decide whether mixed-direction differences are corrections or compatibility requirements, especially before migrating `TextArea`. Add explicit cases for LTR→RTL, RTL→LTR, neutral and empty paragraphs, LF affinity, and visual movement across their boundaries.

2. **Viewport-sized bands do not eliminate raster-limit failures.** [plans/document-editor.md:323](/home/per/github/pert-ui/plans/document-editor.md:323), `:488` and `:623` promise more than the design establishes. A logical viewport can exceed 8192 physical pixels per side; its area can exceed the coverage budget. Wrapped text can also overflow horizontally—the existing contract explicitly permits an unbreakable wider word (`docs/contracts.md:1283`). Glyph overhangs extend raster bounds beyond line boxes (`text.rs:1496–1504`), and a single glyph can exceed the temporary coverage budget (`:1437–1441`).

   **Change:** specify physical-pixel tiling in both dimensions, including wrapped overflow, overhangs, partial lines and coverage admission. Define the failure policy for an individually excessive glyph or style. Until that exists, remove “no value within its capacity fails” and “bounded only by f64 offsets and document capacity.”

3. **The proposed public experimental editor can still have privileged capabilities.** [plans/document-editor.md:457](/home/per/github/pert-ui/plans/document-editor.md:457) defers public primitives until the editor leaves experimental status; `:662` explicitly offers an internal-only public first version. `AGENTS.md:72–75` has no experimental exemption for built-ins that consumers cannot reproduce.

   **Change:** keep the prototype internal until the required pert-ui-owned layout, painting and selection extension points are documented and public. Make a custom editor using only facade imports an acceptance requirement for slice 5. Parley affinity must also stay behind a pert-ui-owned public type.

**Should-fix**

4. **Long-paragraph chunking needs an actual layout algorithm.** [plans/document-editor.md:375](/home/per/github/pert-ui/plans/document-editor.md:375) proposes grapheme/word splits and horizontal visible-range chunks. The required unbroken 64 KiB corpus has no interior word boundary; a single grapheme can itself exceed a byte cap. Horizontal visibility cannot determine logical chunks without resolving advances and bidi order. Independently wrapping chunks can additionally change line packing and heights, beyond the acknowledged shaping differences.

   **Change:** either choose refusal for the first implementation or specify fallback split rules, paragraph-wide bidi handling, continuous wrapping, coordinate mapping and worst-case work. Test Arabic joining, mixed-direction text, enormous graphemes and edits that move chunk boundaries. Qualify the `O(touched paragraph + log n)` claim at `:360` accordingly.

5. **IME display reintroduces a large prepared unit.** [plans/document-editor.md:429](/home/per/github/pert-ui/plans/document-editor.md:429) combines affected paragraphs into one temporary unit, then suppresses it above the paragraph limit. With the proposed long-paragraph support, ordinary composition in an accepted long paragraph could remain invisible. Cross-paragraph replacement also needs an explicit mapping between committed offsets, temporary display offsets, paragraph heights and composition cursor offsets.

   **Change:** use a temporary document replacement projected through the same paragraph/chunk engine, or document a deliberately restricted first version. Retain the original replacement range independently of whether preedit is displayed. Add tests for LF insertion/removal in preedit, cross-paragraph selection replacement, rejected preedit followed by commit, cancellation and session exhaustion. Today’s session contract explicitly requires disabling IME on exhaustion (`text_input.rs:396–399`).

6. **The document/edit/history contract is too incomplete for implementation.** [plans/document-editor.md:301](/home/per/github/pert-ui/plans/document-editor.md:301) proposes `TextChange { range, inserted_len }` as a saving delta, but does not specify its source revision, coordinate space or how the inserted text is obtained. The public API at `:513–520` omits invalid-range errors and the boundary rules for `replace` and `slice`. External replacement cancellation is covered, but selection transformation and undo invalidation are not.

   Local editor undo is consistent with existing `TextArea`; it does not inherently violate application undo ownership. However, automatic document undo needs a clear boundary with host history.

   **Change:** define revision association, range validation, replacement payload access, atomic publication and callback failure behavior. Specify whether external replacement resets selection and clears local undo—as today’s `sync_text` does (`text_fields.rs:2497–2505`)—and how a host can own undo without competing shortcut handling.

7. **Paragraph boundary representation and geometry need explicit rules.** [plans/document-editor.md:290](/home/per/github/pert-ui/plans/document-editor.md:290) defines paragraphs as text between LFs, but does not settle separator ownership, trailing empty paragraphs, or mapping offsets immediately before/after LF. Those choices affect caret stops, deletion, copy, hit testing and selected-LF geometry (`:385–420`). An empty prepared paragraph also needs positive line geometry even though it has no ink.

   **Change:** add a small normative mapping table for `""`, `"\n"`, `"a\n"` and `"a\n\nb"`, including both affinities, backward selections and boundary deletion. Specify selection collapse before visual movement and preferred-x reset behavior. Parity tests alone cannot define behavior where the oracle has different paragraph semantics.

8. **Cache and height-index invalidation are underspecified.** [plans/document-editor.md:331](/home/per/github/pert-ui/plans/document-editor.md:331) lists useful cache keys, but does not define stable paragraph identity through split/merge, invalidation of previously measured heights after width/font/scale changes, or scroll-anchor priority when the caret is offscreen. Retained raster behavior also depends on rendering mode and physical phase: current `TextLayout::paint_turned` selects phase coverage from physical placement (`text.rs:335–350`).

   **Change:** define generation invalidation for both caches and height measurements, settle visible-range refinement before hit testing, and specify anchor identity through edits. Budget transient shaping/raster allocations and resources retained by display lists, not just cache residency. Add scale/font/mode changes, fractional scrolling and hidden/unmount cleanup to the test plan.

9. **The measurement checkpoint precedes the machinery needed to produce its evidence.** [plans/document-editor.md:564](/home/per/github/pert-ui/plans/document-editor.md:564) requires edit, paste, copy, external replacement and IME measurements. The checkpoint at `:603` consumes these figures, but editing, clipboard and undo appear only in slice 5 (`:613–615`). Selection/IME prototypes alone cannot measure the promised complete path.

   **Change:** include an internal end-to-end editor prototype before the checkpoint, or explicitly separate pre-checkpoint primitive measurements from later editor acceptance measurements. Set provisional latency/memory targets before examining results. Also revise slice 1’s “package-local” gate (`:594`): the scheduling/rendering boundary changes fall under AGENTS.md’s broader gate categories.

10. **The benchmark disclosures are good, but several conclusions exceed the measurements.** [plans/document-editor.md:159](/home/per/github/pert-ui/plans/document-editor.md:159) attributes over 99% of time to whole-text preparation, although the harness times the entire input call, including cloning, validation, binding publication and undo (`document-editor-bench/src/main.rs:268–279`; `text_fields.rs:3187–3225` at the measured commit). Likewise, `:188` attributes the transient peak entirely to shared-cache churn without allocation attribution.

    At `:183`, a 16 MiB cache *estimate* is presented as bounding what the series retains. It bounds cache admission accounting (`text.rs:1100–1123`), not actual heap usage, undo, scratch allocations or independently retained resources. Three corpus sizes demonstrate approximately linear observations, not a general scaling law. Scheduler migration is also an unmeasured explanation for unpinned variation (`:156`).

    **Change:** distinguish measured input-call duration from inferred causes; describe cache churn and scheduler effects as hypotheses. Label retained memory as an increment over the settled state. Preserve raw per-run output and identify allocator instrumentation as part of the measured configuration. The machine, background load, corpus limitations and exclusion of backend rendering are otherwise disclosed honestly.

**Nit**

11. **Qualify two current-code summaries.** [plans/document-editor.md:33](/home/per/github/pert-ui/plans/document-editor.md:33) says every edit writes the binding, but draft/deferred editors deliberately do not (`text_fields.rs:1732–1738`). At `:50`, selected text is painted once more **per selection rectangle**, rather than necessarily exactly twice (`:3056–3063`). The measured plain `TextArea` path supports the intended argument; tighten the general wording.

12. **Refresh the outside-scope findings’ disposition.** [plans/document-editor.md:682](/home/per/github/pert-ui/plans/document-editor.md:682) is accurate against 9aaacca:
    - Binding checks omit raster feasibility, and preparation errors can escape arrangement.
    - Edit preparation occurs before binding publication, preserving the binding on raster failure.
    - The shared-limit inventory and 32,253-byte assertion ceiling are correct.
    - The old repair-plan `:104` reference indeed points to preedit validation.

    However, the current working tree already expands TEXT-01, records the raster finding in `TODO.md:174`, and corrects the repair-plan reference (`plans/todo-repairs.md:730`). Update “the author’s step” to distinguish completed recording from remaining work. The exact raster thresholds and repeat counts remain reported experimental evidence, not statically verified facts.

**What I did not examine**

I did not run Cargo, the benchmark, native input, rendering backends or memory instrumentation, so I cannot independently confirm the reported timings, heap totals, machine conditions or exact failure thresholds. I did not review unrelated repository changes or perform a complete Parley/Unicode conformance audit. No files were edited; findings were not added to TODO.md under this read-only instruction.