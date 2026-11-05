ready with nits

| `impl-final-r2.md` finding | Status |
|---|---|
| Complete ordered picker result | Resolved: checks the entire sequence, fields, text, roles, paths, visibility and target relationships. |
| Fresh searches contain only current results | Partly: every hit’s text is checked; only item hits require the current target. |
| Icon identification | Resolved for this fixture; content-based comparison replaces the geometry heuristic. |
| Chip label and MultiSelect foreground | Resolved: label coverage must be nonempty and muted; both foreground reports are checked. |
| Progress-record accuracy | Partly: fault-trigger wording is corrected; the remaining fresh-search identity gap qualifies closure. |
| Fault cleanup suggestion | Resolved for its current use: the guard resets the thread-local fault during unwinding. |
| Previously resolved findings | Retained; not re-audited. |

Findings:

- **Low — stale same-text row hits remain undetected.** [popup_items.rs:460](/home/per/github/pert-ui/crates/pert-ui/tests/collections/popup_items.rs:460): the old list already contained “Alphabet”. A stale mounted-row hit for that result could accompany the current item and pass every assertion. Require non-item hits to target the current row or its descendants. Consequently, “only the current results” in [todo-repairs.md:1019](/home/per/github/pert-ui/plans/todo-repairs.md:1019) remains slightly stronger than the evidence.
- **Low — newly committed review-record whitespace.** [impl-final-r2.md:31](/home/per/github/pert-ui/plans/todo-repairs-review/impl-final-r2.md:31), also lines 34, 37, 40 and 43: commit-scoped `git diff-tree --check` reports trailing whitespace. Remove the hard-break spaces, consistent with the earlier whitespace repair.

The reference comparison does **not** pass vacuously: both differences must be nonempty. Mask equality includes dimensions and coverage, and scene composition actually interns equal content. In this fixture, chip corner coverage and the close icon remain shared despite differing label widths; default line-mask rendering avoids placement-phase differences. Missing sharing would expose non-muted frame/container masks and fail the assertions. I found no demonstrated false-pass path from differing control sizes. The helper’s explanation should be understood as fixture-specific, not a general guarantee for arbitrary controls.

The changelog otherwise matches the changes. Recorded execution and negative-control results remain unverified. No new production defect was established; the remaining issues are nonblocking review nits.

I examined only `69981d8` and supporting implementation needed for these findings. I did not run Cargo, validate recorded runs, re-audit unrelated branch repairs, or examine native checks. No files were edited.