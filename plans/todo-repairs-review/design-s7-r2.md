needs rework

Revision 2 substantially improves the plan, but its revision identity, raster admission and layout contracts still contain correctness gaps. These need design changes before the maintainer’s checkpoint; implementation is not required to resolve them.

| # | Revision-1 finding | Status | Recheck |
|---|---|---|---|
| 1 | Paragraph parity | Resolved | Unconditional parity is withdrawn; intended behavior, characterization tests and the `TextArea` decision are explicit. |
| 2 | Raster limits | Partly resolved | Two-dimensional physical tiling is specified, but cumulative raster admission and visible-memory bounds remain unsound. |
| 3 | Privileged public editor | Resolved | Prototype stays internal; public extension points precede adoption; facade-only reproduction is required. |
| 4 | Long-paragraph chunking | Resolved | First version chooses refusal and defers the missing chunking algorithm. |
| 5 | IME prepared unit | Partly resolved | Paragraph projection replaces the large unit, but direction dependencies and refused-display geometry remain incomplete. |
| 6 | Edit/history contract | Partly resolved | Ranges, payload access and publication ordering improve; revision identity, no-op publication and undo outcomes remain undefined. |
| 7 | Boundary representation | Partly resolved | Separator ownership and deletion are clear; affinity is incorrectly restricted to soft wraps, and one example miscounts bytes. |
| 8 | Cache/index invalidation | Partly resolved | Generations and anchors are described, but inherited direction is absent from dependencies and settling has no exhaustion outcome. |
| 9 | Measurement checkpoint | Resolved | An internal end-to-end prototype now precedes the checkpoint; provisional targets precede future results. Target conditions still need tightening. |
| 10 | Measurement conclusions | Resolved | Attribution is qualified, allocator overhead is disclosed, and round-2 tables agree with preserved output. |
| 11 | Current-code summaries | Resolved | Draft/deferred writes and per-selection-rectangle painting are correctly qualified. |
| 12 | Outside-scope disposition | Resolved | Completed recording is distinguished from the remaining current-contract repair. |

**Blockers**

1. **Revisions do not uniquely identify document states.** At [plans/document-editor.md:438](/home/per/github/pert-ui/plans/document-editor.md:438), every replacement increments the source counter within the same lineage. Two replacements from the same persistent document therefore produce different contents with the same revision. `TextChange` also names only its base, despite claiming interpretation against named revisions at [line 454](/home/per/github/pert-ui/plans/document-editor.md:454).

   Content-only equality creates another inconsistency: a same-text replacement can produce a new revision, but `Signal::set` suppresses equal values ([signal.rs:554](/home/per/github/pert-ui/crates/pert-ui-runtime/src/signal.rs:554)). The editor could record history and notify with a result revision that the binding never adopted.

   **Suggested change:** define branch-safe state identity, revision exhaustion, and no-op behavior. Give changes an explicit result identity or an equally unambiguous result association. Specify how equality, binding adoption, external-write detection and callbacks agree. Test sibling branches, same-text replacement, undo/redo and a different-lineage equal-content write.

2. **A bounded output tile does not bound its input rasters.** [Line 564](/home/per/github/pert-ui/plans/document-editor.md:564) applies today’s cumulative temporary budget per tile, while [line 571](/home/per/github/pert-ui/plans/document-editor.md:571) handles only an individually excessive glyph. Many individually admissible overlapping glyphs can exceed that cumulative budget in one tile—for example, a combining-mark sequence. The document can remain within its paragraph limit.

   Moreover, today’s path renders the glyph before checking its allocated coverage ([text.rs:1396](/home/per/github/pert-ui/crates/pert-ui-runtime/src/text.rs:1396), [text.rs:1437](/home/per/github/pert-ui/crates/pert-ui-runtime/src/text.rs:1437)). Reusing that order cannot establish a bound before allocation.

   **Suggested change:** specify bounded streaming/compositing or another admission algorithm, including aggregate exhaustion and preallocation checks. Define conservative candidate bounds for fallback fonts, variable outlines and raster phase. Withdraw the renewed “within capacity” guarantees at [line 872](/home/per/github/pert-ui/plans/document-editor.md:872) and [line 971](/home/per/github/pert-ui/plans/document-editor.md:971) until these cases have a defined outcome.

**Major findings**

3. **Inherited direction invalidates the content-only cache and touched-paragraph cost claim.** Neutral paragraphs inherit preceding strong direction at [line 350](/home/per/github/pert-ui/plans/document-editor.md:350), but shaped keys contain only content and `G` at [line 592](/home/per/github/pert-ui/plans/document-editor.md:592). Identical neutral text following LTR and RTL paragraphs needs different layouts. Editing one strong paragraph can also change an arbitrarily long following neutral run, contradicting [line 526](/home/per/github/pert-ui/plans/document-editor.md:526). IME projection cannot assume all paragraphs outside `R` remain unchanged either ([line 713](/home/per/github/pert-ui/plans/document-editor.md:713)).

   **Suggested change:** include resolved direction in layout identity and specify indexed direction-context lookup and lazy invalidation through the next strong paragraph. Qualify edit complexity. Apply the same dependency rules to committed edits and temporary projections.

4. **Affinity also matters at bidi boundaries within one line.** [Line 401](/home/per/github/pert-ui/plans/document-editor.md:401) says it matters only at soft wraps. Parley’s cursor uses affinity to choose the adjacent logical cluster even without a wrap; see its local `editing/cursor.rs:80–85,331`. Discarding that distinction prevents faithful visual movement and hit testing in mixed-direction lines.

   The boundary-crossing rule at [line 665](/home/per/github/pert-ui/plans/document-editor.md:665) also needs to distinguish logical endpoints from visual edges: an LTR paragraph ending in an RTL run need not place its logical end at its rightmost edge.

   **Suggested change:** define affinity at bidi boundaries, explicit conversion at hard paragraph endpoints, and paragraph crossing after visual movement reaches an edge. Add same-line mixed-direction cases alongside the cross-paragraph cases.

5. **Visible-range settling promises convergence without an exhaustion policy.** [Line 612](/home/per/github/pert-ui/plans/document-editor.md:612) repeats until stable, capped at three passes. Large changes from estimated to measured heights can expose another unmeasured range on each pass. Nothing defines what happens when the third pass is insufficient, yet hit testing requires current-generation measurements.

   **Suggested change:** define a convergent visible-fill algorithm or an explicit bounded continuation policy covering paint, input and scheduling. After reflow or edits, also specify how an offset that is no longer a line start resolves to an anchor line, and how deletion-created grapheme joins are handled.

6. **The visible-memory bound omits paragraph-grid fragmentation.** Tiles are full 1024² allocations anchored separately for each paragraph ([line 538](/home/per/github/pert-ui/plans/document-editor.md:538)). Many short visible paragraphs can each require a mostly empty 1 MiB tile. Their aggregate allocation is not bounded by viewport area alone as claimed at [line 564](/home/per/github/pert-ui/plans/document-editor.md:564).

   **Suggested change:** account for tile rounding per paragraph, overscan, overlapping overhangs, temporary glyphs and simultaneous old/new display lists. Alternatively, use cropped edge tiles or a shared tiling strategy. State whether area is logical or physical so scale is counted once.

7. **Refused IME display has no defined caret geometry.** The plan retains composition when projection is refused ([line 728](/home/per/github/pert-ui/plans/document-editor.md:728)), but always obtains the IME rectangle from the projected layout ([line 741](/home/per/github/pert-ui/plans/document-editor.md:741)). That layout does not exist in the refusal case. Mapping also leaves committed positions inside the replacement range undefined and ambiguously includes the suffix boundary in “preedit.”

   **Suggested change:** specify half-open mapping ranges and endpoint ownership, reverse mapping where input needs it, a committed-layout caret fallback when display is refused, and anchor behavior when projection appears or disappears. Include direction changes beyond `R` from finding 3.

8. **Invisible-glyph fallback needs an explicit adoption decision.** [Line 571](/home/per/github/pert-ui/plans/document-editor.md:571) accepts text while omitting excessive glyph ink and reports it only through inspection. Through slice 6, that becomes a new `TextArea` behavior; the compatibility section and maintainer questions do not clearly present that choice.

   **Suggested change:** explicitly choose omission, a bounded replacement marker or refusal, including how users and hosts learn about it. Define `raster_refused` lifetime, phase dependence and reset behavior. Record the chosen `TextArea` behavior in the checkpoint decision and migration notes.

**Further changes**

9. **Undo boundaries need executable semantics.** [Line 488](/home/per/github/pert-ui/plans/document-editor.md:488) supplies limits but does not define an individual edit exceeding the byte budget, redo accounting, restored selection, or whether undo/redo publish fresh revisions and `on_change`.

   **Suggested change:** define those outcomes and their notification ordering. Host-owned undo can remain explicitly deferred.

10. **Targets need reproducible conditions.** [Line 882](/home/per/github/pert-ui/plans/document-editor.md:882) fixes machine and timing endpoints but leaves viewport, scale, rendering mode, cache warmth and history occupancy unspecified. These materially affect mandatory visible-tile memory and the steady/peak heap targets. The 8 MiB peak target also needs a stated operation domain.

    **Suggested change:** freeze those conditions before prototype results, define steady state and included allocations, and distinguish display-list recording targets from backend presentation latency.

11. **Finish publication dependency ordering.** The internal-until-public rule is repaired at [line 976](/home/per/github/pert-ui/plans/document-editor.md:976). However, slice 7 publishes document selection/index primitives while `TextDocument` and its errors wait until slice 8 ([line 978](/home/per/github/pert-ui/plans/document-editor.md:978)).

    **Suggested change:** state whether slice-7 APIs operate independently of `TextDocument`; otherwise move their required public data types into slice 7. Each slice must have independently usable examples.

12. **Correct the normative example.** At [line 407](/home/per/github/pert-ui/plans/document-editor.md:407), `"a\n\nb"` contains four bytes. Selection `0..4` selects all four, not five.

I did not run Cargo, benchmarks, native IME, renderers or memory instrumentation. I checked the preserved aggregate output against the tables, not individual timing samples or machine conditions. I did not perform a complete Unicode/Parley conformance audit, validate future performance feasibility, or review unrelated working-tree changes. No files were edited, and no findings were added to TODO.md under the read-only instruction.