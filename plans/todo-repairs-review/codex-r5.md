needs rework

Revision 5 resolves two r4 findings. R13’s independent evidence avoids cache eviction, but its readers, initialization and change notification remain underspecified.

| r4 finding | Status | Assessment |
|---|---|---|
| R13 isolation | Partly resolved | Independent storage survives eviction; gaps below still affect validation. |
| R13 placement storage | Resolved | Explicit internal struct, parent-authored evidence stored on the child, staged adoption and retention rules. |
| R15 inspection counter | Resolved | Assertion now covers ItemScan continuation; ancestor naming inspection is explicitly excluded. |

“Resolved” means specified adequately, not implemented or tested.

**High — R13 misses two validation paths that read the latest cache entry.**

The replacement list at [plans/todo-repairs.md:329](/home/per/github/pert-ui/plans/todo-repairs.md:329) omits:

- Successful component updates populate `Validator::update` using `Node::measured_height`: [runtime.rs:6243](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:6243). That accessor currently reads `Measurements::latest`: [runtime.rs:955](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:955). An update following a probe can therefore reintroduce probe-derived evidence.
- `refresh_forwarded` obtains constraints through `forwarded_constraints`, which reads `Node::measured_constraints`, currently backed by `latest_constraints`: [layout.rs:1225](/home/per/github/pert-ui/crates/pert-ui-runtime/src/layout.rs:1225), [tree.rs:1827](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1827), [runtime.rs:961](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:961). After the guard ends, it can remeasure a descendant at probe-derived constraints as an ordinary request.

Suggested spec change: explicitly migrate **all validation measurement readers**, including update-time height and forwarded constraints. Define the accessors centrally, preserve height scaling, and specify invalidation of the independent record alongside cache clearing at [runtime.rs:1034](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:1034). Add update-after-probe and forwarded-refresh regressions.

**High — Blanket suppression leaves first-measured descendants without usable validation evidence.**

[Spec:320–328](/home/per/github/pert-ui/plans/todo-repairs.md:320) suppresses every validation-record write throughout nested probes. First measurement under a probe is a normal case: `VirtualList::measure` does not measure rows, and fixed rows are placed during arrangement: [virtual_list.rs:2121](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:2121), [virtual_list.rs:2152](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:2152).

A newly mounted forwarding row and its descendants can consequently have no ordinary record. Redirecting forwarded constraints to that record makes `refresh_forwarded` do nothing; leaving it on the cache causes the isolation leak above. With no measured height, the parent-box validator falls back to allocated height, potentially hiding compression: [development.rs:723](/home/per/github/pert-ui/crates/pert-ui-runtime/src/validation/development.rs:723).

Suggested spec change: define first-measurement semantics explicitly—for example, distinguish ordinary evidence from probe-only fallback evidence, preserving existing ordinary records while supporting newly measured wrappers and descendants. Specify how forwarded refresh uses that provenance. Add an initially unmeasured nested fixed-height wrapper regression; the current “same as without the probe” assertion does not establish this behavior.

**Medium — Changing the drain’s reader does not fix change detection.**

`measure_node` still compares against the **cache’s** latest size before calling `note_measure_change`: [tree.rs:1839](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1839), [tree.rs:1871](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1871), [tree.rs:1887](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1887).

Once commit and drain read independent evidence, a probe alone cannot change their observed height. It can nevertheless enqueue unnecessary work. More seriously, after that queue is drained, a later ordinary request matching the probe’s cached size can update ordinary evidence **without enqueueing a change**. Unchanged geometry can skip arrangement entirely: [tree.rs:1925](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1925). Validation then retains the old height.

Suggested spec change: make notification follow changes to the independent validation evidence, using its previous value as the comparison baseline. Suppress probe-only notifications. Test a completed probe pass followed by an ordinary cache hit at the probe’s constraints, with unchanged geometry.

The whole spec is **not ready to implement unchanged**. Slices 1–2 retain their previously accepted readiness; slice 3 needs these corrections. Later slices remain subject to their stated design and maintainer checkpoints.

I examined only the three r4 findings and the revised R13 measurement paths against HEAD `621022a`. I did not re-audit other repairs, run Cargo/native checks, or verify generated API listings, packaging or performance. No files were edited, including TODO.md.