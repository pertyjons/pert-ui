needs rework

Revision 3 resolves much of revision 2, but the raster admission, visible-fill and IME contracts still need design changes before the maintainer’s checkpoint. Implementation evidence is not required to repair these specifications.

| R2 point | Status | Recheck |
|---|---|---|
| 1. State identity | Resolved | Live tokens distinguish branches; no-ops, binding adoption and result revisions agree. |
| 2. Raster admission | Partly resolved | Streaming bounds output buffers, but the stated scratch-memory and work bounds omit rasterizer internals. |
| 3. Direction dependencies | Resolved | Resolved direction enters cache identity; indexed lookup and lazy invalidation cover edits and IME. |
| 4. Affinity | Resolved | Same-line bidi boundaries, endpoints and paragraph crossing are explicit. |
| 5. Visible settling | Partly resolved | Anchor-first filling removes pass exhaustion, but does not discover all incoming glyph overhangs. |
| 6. Visible memory | Partly resolved | Cropping fixes full-cell fragmentation; scratch storage and unequal old/new frames remain unaccounted for. |
| 7. IME geometry | Partly resolved | Refused-display geometry is defined; replacement-start mapping and grapheme joins remain incomplete. |
| 8. Glyph fallback | Partly resolved | Marker adoption and lifetime are explicit; zero-advance glyphs and tile-dependent markers undermine the policy. |
| 9. Undo semantics | Resolved | Exact-state restoration, applicability, notifications, selections and oversize edits are specified. Accounting needs a separate correction below. |
| 10. Measurement conditions | Resolved | Viewport, scale, modes, warmth, history, allocation scope and operation domains are frozen. |
| 11. Publication ordering | Resolved | Slice 7 now includes the document, revisions, changes and errors required by its extension points. |
| 12. Byte-count example | Resolved | The example correctly counts four bytes. |

Pointer identity is **not inherently a defect** here. Retained `Arc` tokens prevent address reuse while a revision remains referencable. `DocumentRevision` must implement identity-based `Eq` and `Hash` consistently; any future `TextDocument::Hash` must follow its content equality. Undo’s A→B→A token recurrence correctly denotes restored states, rather than unique edit events. The explicit fresh identity on deserialization also avoids pretending these are durable identifiers. Keep revisions and changes explicitly nonportable; application persistence needs its own versioned envelope and durable identifiers if required. See [identity](/home/per/github/pert-ui/plans/document-editor.md:474), [undo](/home/per/github/pert-ui/plans/document-editor.md:571) and [serialization](/home/per/github/pert-ui/plans/document-editor.md:1184).

**Blocker**

1. **The new admission algorithm still does not establish its memory or work claims.** [plan:736](/home/per/github/pert-ui/plans/document-editor.md:736) allocates a tile and coverage scratch buffer, then [plan:747](/home/per/github/pert-ui/plans/document-editor.md:747) claims those plus one outline are the peak temporary memory, and raster work is at most `W` pixel operations plus candidate scanning.

   Zeno additionally maintains raster cells and row indices. Its adaptive storage spills cells into heap vectors ([raster.rs:630](/home/per/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zeno-0.3.3/src/raster.rs:630), [raster.rs:733](/home/per/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zeno-0.3.3/src/raster.rs:733)). Summed window area does not bound path traversal, curve subdivision or repeated edge processing: a complicated outline can occupy a tiny window.

   **Change:** account for raster-cell/index storage and outline/scaler allocations. Define a separate complexity/work admission policy with a terminal fallback, or withdraw the claimed work bound. State which font-derived allocations remain outside the raster admission guarantee.

**Major findings**

2. **One-pixel padding is conditional on using precisely the same final outline.** [plan:677](/home/per/github/pert-ui/plans/document-editor.md:677) specifies scaled outline bounds and a cache key containing font, glyph, size and variation coordinates, but does not fix hinting or synthetic transformations.

   Today’s scaler uses `.hint(true)` ([text.rs:1361](/home/per/github/pert-ui/crates/pert-ui-runtime/src/text.rs:1361)). Swash applies hinting during outline generation, and optional emboldening/transforms afterward ([scale/mod.rs:568](/home/per/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/swash-0.2.10/src/scale/mod.rs:568), [scale/mod.rs:858](/home/per/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/swash-0.2.10/src/scale/mod.rs:858)). One pixel cannot generally cover those differences.

   **Change:** define a common raster descriptor and compute bounds from the exact hinted, variation-instantiated, synthetically transformed outline subsequently rendered. Include every relevant setting in cache identity, or explicitly exclude unsupported synthesis. Variable coordinates and fallback fonts are already addressed. Color/bitmap-only glyphs are explicitly excluded at line 688, so their absence is a scope choice, not a demonstrated bounds failure.

3. **The marker can be invisible and is not stable across tile crops.** [plan:754](/home/per/github/pert-ui/plans/document-editor.md:754) makes the box as wide as the glyph advance. Combining marks commonly have zero advance—the very exhaustion case required by slice 1—so the promised visible replacement can have no width.

   Admission is per tile and uses cropped window areas ([plan:723](/home/per/github/pert-ui/plans/document-editor.md:723)). One glyph can consequently be admitted in one tile and replaced in another. Fractional scrolling changes crops and can change aggregate admission even with phase-independent bounds, contrary to [plan:769](/home/per/github/pert-ui/plans/document-editor.md:769). Marker geometry also differs from the outline geometry used to discover candidates and allocate crops.

   **Change:** give markers a positive minimum extent and define their discovery bounds, clipping, overlap order and deduplicated inspection count. Choose explicitly between whole-glyph replacement and partial tile replacement. Either stabilize admission independently of viewport crops or withdraw the no-flicker claim.

4. **The visible fill covers line boxes, not necessarily all visible ink.** [plan:854](/home/per/github/pert-ui/plans/document-editor.md:854) stops after paragraph heights cover the region. Yet glyphs may extend beyond their paragraph’s line boxes ([plan:707](/home/per/github/pert-ui/plans/document-editor.md:707)). A paragraph beyond the filled range can therefore contribute visible ink without ever being shaped or considered. Fixed half-height overscan does not establish a maximum overhang.

   **Change:** specify how incoming overhang candidates are discovered—through a conservative halo, indexed extents or another bounded policy. Include this work in the fill bound at [plan:866](/home/per/github/pert-ui/plans/document-editor.md:866). Define whether `N` and `M` in the memory formula include those paragraphs.

5. **IME forward mapping omits `c == r0` for nonempty replacements.** At [plan:1003](/home/per/github/pert-ui/plans/document-editor.md:1003), the three branches cover `c < r0`, `c >= r1` and `r0 < c < r1`. When `r0 < r1`, the replacement start has no mapping. Moving an interior anchor to `r0` ([plan:1042](/home/per/github/pert-ui/plans/document-editor.md:1042)) reaches that undefined case.

   Projection can also join graphemes across the preedit boundary. For example, inserting a combining mark after `a` makes the preedit start an interior grapheme position, conflicting with the selection validation rule at [plan:930](/home/per/github/pert-ui/plans/document-editor.md:930).

   **Change:** make endpoint mapping total, distinguishing insertion from nonempty replacement. Specify presentation handling for prefix/preedit/suffix grapheme joins while preserving the original committed replacement range.

**Further findings**

6. **The memory formula needs separate old/new frame terms.** [plan:791](/home/per/github/pert-ui/plans/document-editor.md:791) claims twice visible coverage is the worst case. That works for equal-sized regions, but a viewport or scale change can leave an old frame substantially larger than the new one.

   **Change:** use `old_frame_coverage + new_frame_coverage`, with each frame’s dimensions, overhangs and rounding, and add the rasterizer storage from finding 1.

7. **Undo’s byte accounting undercharges persistent paragraph copies.** [plan:592](/home/per/github/pert-ui/plans/document-editor.md:592) charges removed/inserted bytes plus a constant for tree paths. Storage uses whole `Arc<str>` paragraphs ([plan:463](/home/per/github/pert-ui/plans/document-editor.md:463)); a one-byte edit can retain an entire superseded paragraph. Tree-path storage also depends on tree height.

   **Change:** either describe 8 MiB as a logical history-payload budget with a separate retained-memory bound, or charge retained paragraph allocations and actual tree-node costs conservatively. Keep the exact-state undo semantics.

**Not examined**

I did not run Cargo, benchmarks, renderers, native IME or memory instrumentation, and did not re-audit the preserved timing samples. This was not a complete Unicode, font-parser or public-API conformance review. Slice 7’s dependency ordering is repaired; its eventual signatures and usability remain implementation-review work.

No files were edited, including TODO.md. Unrelated working-tree changes were not reviewed.