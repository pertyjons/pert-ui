needs rework

Revision 4 addresses every prior point, but several replacement specifications still do not establish their claimed bounds. These are design gaps; implementation evidence is not required to resolve them.

| R3 point | Status | Recheck |
|---|---|---|
| 1. Raster storage/work | Partly resolved | Font-derived allocations are explicitly excluded; flattening and bands help, but the charge omits work and storage capacity. |
| 2. Raster descriptor | Resolved | Bounds and rendering share the hinted, variation-instantiated outline; synthesis is explicitly excluded. |
| 3. Markers | Partly resolved | Whole-glyph admission on a paragraph-local grid fixes crop/phase instability. Positive size, order and deduplication are specified; confinement is not ensured. |
| 4. Incoming overhang | Partly resolved | The limit and halo establish discovery for admitted ink. The marker geometry can violate the premise. |
| 5. IME mapping | Resolved | Forward mapping covers replacement starts and insertions; grapheme joins have an internal presentation rule that preserves committed `R`. |
| 6. Frame memory | Partly resolved | Separate old/new terms fix unequal frames. Scratch capacity and rounding remain undercounted. |
| 7. Undo accounting | Partly resolved | Whole superseded paragraphs and variable path costs are charged, but not every retained allocation. |

**Blocker**

1. **The charge still does not establish the raster-work guarantee, and clipping lacks fill semantics.** [plan:765](/home/per/github/pert-ui/plans/document-editor.md:765) charges clipped segment crossings and area; [plan:807](/home/per/github/pert-ui/plans/document-editor.md:807) then claims linear raster work.

   Zeno’s [`HeapStorage::set`](/home/per/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zeno-0.3.3/src/raster.rs:595) searches a row’s linked list from its head on every update. Inserting successive distinct x cells can require quadratic list traversal. Fixed `T` provides a finite upper bound, but the proposed charge does not include that factor. Re-emitting the path for each band also repeats flattening and traversal of segments discarded by clipping.

   Moreover, clipping individual segments does not preserve filled contours: a contour enclosing a cell can have no edges inside it while covering the entire cell. Zeno preserves left-side winding through a sentinel cell ([raster.rs:258](/home/per/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zeno-0.3.3/src/raster.rs:258)). The plan supplies no equivalent contour-clipping rule. Clipping before applying fractional phase can also discard geometry that subsequently moves across a cell boundary.

   **Change:** specify winding-preserving clipping, contour closure and phase handling. Charge all emitted and traversed work, band replay, and a conservative bound for row searches—or choose storage with directly bounded updates. The optional whole-glyph path at [plan:820](/home/per/github/pert-ui/plans/document-editor.md:820) must use the same bounded geometry or have its own admission proof.

**Major findings**

2. **The band-storage formula bounds occupied cells, not allocated memory.** The uniqueness argument at [plan:815](/home/per/github/pert-ui/plans/document-editor.md:815) is useful: Zeno merges equal-x cells, so the proposed cell-count bound is conservative. However, [`HeapStorage`](/home/per/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zeno-0.3.3/src/raster.rs:576) uses growing vectors, calls `push`, and retains their capacity after `clear`. Capacity can exceed the maximum live length. Reuse does not make [plan:818](/home/per/github/pert-ui/plans/document-editor.md:818)’s exact byte bound true or ensure allocation happens only once.

   **Change:** account for vector capacities and growth peaks, or specify a storage implementation with controlled preallocation. Name the actual integration through Zeno’s public `Scratch`; its `HeapStorage` is internal ([scratch.rs:16](/home/per/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zeno-0.3.3/src/scratch.rs:16)). Update the frame scratch term accordingly.

3. **Marker construction does not ensure the confinement used by the halo proof.** [plan:860](/home/per/github/pert-ui/plans/document-editor.md:860) intersects bounds with the line box, then grows and recenters the rectangle. Growth can escape the box; an excessive displaced mark can have an empty intersection. A positive-width marker also cannot fit inside a zero-width line box. Nevertheless, [plan:974](/home/per/github/pert-ui/plans/document-editor.md:974) assumes markers remain inside line boxes.

   **Change:** define marker geometry independently for empty intersections and zero-advance lines. Explicitly enforce vertical confinement after minimum-size adjustment, including short line boxes, or assign markers a bounded overhang and include it in the halo. The paragraph-anchored grid and whole-glyph policy otherwise resolve the previous stability concern.

4. **Undo’s charge misses allocations retained by range deletion.** [plan:604](/home/per/github/pert-ui/plans/document-editor.md:604) charges paragraph lengths and copied tree paths. Deleting a large range can leave history retaining an entire removed subtree, including nodes that were never copied. Empty paragraphs make the omission especially clear: their text-length charge is zero, while their allocations and tree structure remain alive. `Arc` headers, revision tokens and history bookkeeping are also absent.

   **Change:** charge complete allocations retained exclusively through history, including removed subtree nodes and allocation overhead. Conservatively reserve both restoration directions when recording an entry, with an explicit policy when moving it between undo and redo. Preserve the exact-state restoration semantics.

**Further findings**

5. **The flattener’s accuracy contract is incomplete.** [plan:768](/home/per/github/pert-ui/plans/document-editor.md:768) specifies both quarter-pixel tolerance and maximum depth without saying what happens when the depth is exhausted before tolerance is met. Geometric deviation also does not directly bound coverage-byte differences. [plan:1438](/home/per/github/pert-ui/plans/document-editor.md:1438) defers the comparison tolerance.

   **Change:** define the flatness criterion and depth-exhaustion outcome, preferably terminal marker fallback. Distinguish geometric tolerance from rendered-coverage acceptance. Compare tiled and whole rendering using the same flattened path to test seams separately from differences introduced by the new curve approximation.

6. **The frame round-off allowance can undercount both edges.** [plan:896](/home/per/github/pert-ui/plans/document-editor.md:896) allows one row and column per tile. Rounding `[0.9, 1.1]` outward produces two pixels from a width of `0.2`: more than one additional pixel. Both axes can have this problem.

   **Change:** derive `R_F` from both rounded edges, including corner area, or define the formula entirely using already-rounded integer rectangles. The `C_old + C_new` correction itself is sound.

The plan is **not ready for the maintainer’s design checkpoint** while its admission and memory guarantees depend on these unresolved specifications.

I did not run Cargo, benchmarks, renderers, native IME or allocation instrumentation, and did not re-audit timing samples. This was not a complete Unicode, font-parser or public-API review. No files were edited, including TODO.md; unrelated working-tree changes were not reviewed.