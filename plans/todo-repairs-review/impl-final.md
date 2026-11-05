needs fixes

Static review of `a11bf83`: no new production defect established, but several requested regression checks remain incomplete.

| Report finding | Status | Assessment |
|---|---|---|
| 4b: public extension parity | Resolved | Both adoption accessors are public, documented, listed in the API snapshots and changelog; external retry doctest added. |
| 4b: disabled-result docs | Resolved | QuickPick and verification guide consistently distinguish semantic actions from delivered clicks. |
| 4b: picker search coverage | Partly | Stronger overlap assertions; complete target assertions and picker continuation/re-keying coverage remain missing. |
| 4b: `report_items` cost wording | Resolved | Now accurately excludes row construction, layout and scheduling. |
| 4/5/6: S4-1 modality on continuation | Resolved | Recheck retained. |
| S4-2 picker search policy | Resolved | Default participation and gallery opt-out retained. |
| S4-3 item/row ordering | Partly | Explicit mounted-list sequence and targets added; nested collections remain untested. |
| S4-4 reporter geometry | Resolved | Partial clipping and visible-part activation coverage retained. |
| S4-5 AlertDialog redescription | Resolved | Path comparisons and separate cancel opening retained. |
| S5 serde return-type migration | Resolved | Previous repair retained. |
| S5 gallery bounds validation | Resolved | Previous repair retained. |
| S5 typed reconciliation | Resolved | Previous coverage retained. |
| S5 R19 failure/pending coverage | Resolved | Full-queue refusal coverage retained. |
| S5 example placeholder keys | Resolved | Previous migration retained. |
| S5 review-record whitespace | Partly | Original repair retained; this commit introduces new whitespace failures. |
| S6 zero-width border metadata | Resolved | Helper and transition coverage retained. |
| S6 metadata committed after paint | Partly | Assignment ordering is correct; failure-preservation test still absent. |
| S6 R17/R20 verification | Partly | Light commands and retained-paint profiling added; anchor-specific gaps remain. |
| S6 near-limit PNG round trip | Resolved | Release-only boundary fixture and aspect-ratio tests retained. |
| S6 FrameImage documentation | Resolved | Corrected dependency and allocation wording retained. |

Remaining findings:

- **Medium — Picker coverage remains incomplete.** [popup_items.rs:517](/home/per/github/pert-ui/crates/pert-ui/tests/collections/popup_items.rs:517) checks the row’s role, text and path, but does not compare its `ContentTarget` with the mounted row’s identity or assert the complete expected hit sequence. The existing re-keying test at line 410 never holds a search continuation across the query/session change. Add those explicit assertions and the picker-specific stale-continuation case.

- **Medium — Nested collection continuation remains uncovered.** [content_search.rs:1601](/home/per/github/pert-ui/crates/pert-ui/tests/collections/content_search.rs:1601) correctly checks a mounted list’s unmounted rows followed by mounted children, including targets. Its item owner and list remain siblings, however; `Combined` at line 1543 still has no child. Add the requested nested collection fixture with an explicit sequence and complete targets.

- **Medium — Anchor presentation coverage is still partial.** [disabled_button.rs:569](/home/per/github/pert-ui/crates/pert-ui/tests/presentation/disabled_button.rs:569) uses plain captions; rich/icon anchor captions remain absent. Overrides at line 620 exercise only Surface, and aggregate color checks do not independently establish the selected chip’s treatment. Light drawing and profiling checks themselves address their earlier gaps.

- **Low — Failed-paint preservation remains untested.** [select.rs:1387](/home/per/github/pert-ui/crates/pert-ui-runtime/src/select.rs:1387) and [multi_select.rs:1282](/home/per/github/pert-ui/crates/pert-ui-runtime/src/multi_select.rs:1282) commit their records after fallible operations, as requested. The Progress entry acknowledges the missing forced-failure test; acknowledgement does not complete that regression requirement.

New review-record problems:

- **Low — Progress overstates closure.** [todo-repairs.md:1001](/home/per/github/pert-ui/plans/todo-repairs.md:1001) says all slice 4b points are fixed, and line 1015 leaves only native checks pending. Record the remaining search, nested-collection and anchor coverage alongside the acknowledged paint-failure test.
- **Low — Whitespace gate fails.** `git diff a11bf83^ a11bf83 --check` reports trailing whitespace in [impl-s4b.md:5](/home/per/github/pert-ui/plans/todo-repairs-review/impl-s4b.md:5), also lines 8, 11 and 14.

The repairs are **not ready to merge from a code-review standpoint** until the remaining coverage requests and inaccurate closure record are addressed.

I did not run Cargo, tests, doctests, Clippy, API generation, packaging, rendering or native checks, or verify recorded execution results. I did not audit unrelated repairs or backends. No files were edited.