ready with changes

R13’s new approach resolves the r5 correctness findings. The remaining changes concern cost claims and verification precision, not a redesign.

| Check | Status | Assessment |
|---|---|---|
| r5: validation readers | Resolved | All readers consume ordinary latest-measurement evidence. |
| r5: first measurement | Resolved | Newly mounted rows and measured descendants acquire ordinary evidence. |
| r5: change notification | Resolved | Cache hits and misses compare against the same evidence readers consume. |
| Arrangement correctness | Acceptable | Measurement does not replace the slot allocation. |
| Retention and scheduling | Acceptable | Existing arrangement skips and measurement caches remain applicable. |
| Slot record rules | Adequate | Ownership, clearing, retention, failure and unmount are specified. |
| Cost/profile claims | Changes needed | Claims exceed what the code guarantees. |

**Medium — “one cached measurement per placed row per layout” is too broad.**

The claim and test at [plans/todo-repairs.md:315](/home/per/github/pert-ui/plans/todo-repairs.md:315) and [:363](/home/per/github/pert-ui/plans/todo-repairs.md:363) conflate an additional measurement request with total measurement work:

- Estimated lists already measure mounted rows through `measure_lines`: [virtual_list.rs:1690](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:1690). Horizontal lists use **unbounded width and bounded height**, so the new request uses different constraints: [:1708](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:1708).
- A cold `TreeRow` measurement measures its content unbounded vertically; its arrangement measures that content again at bounded height: [tree.rs:755](/home/per/github/pert-ui/crates/pert-ui-runtime/src/tree.rs:755), [:773](/home/per/github/pert-ui/crates/pert-ui-runtime/src/tree.rs:773).
- Four retained entries do not guarantee a hit after invalidation or eviction: [measurements.rs:5](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/measurements.rs:5).
- Reused parent arrangements make **no new placement-time request**: [runtime/tree.rs:1957](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1957).

Suggested spec change: promise **one additional top-level measurement request per `place_fixed_slot` call**, with a hit when equal constraints remain cached. State that misses can recursively measure descendants, and retained arrangements add no request. Distinguish request, hit and component-measure counters in tests; cover fixed/estimated and vertical/horizontal lists.

**Low — Debug/release measurement parity is not guaranteed.**

[plans/todo-repairs.md:312](/home/per/github/pert-ui/plans/todo-repairs.md:312) claims identical measurement counts and cache contents across profiles. Existing `refresh_forwarded` performs measurements only under `cfg(debug_assertions)`: [layout.rs:1225](/home/per/github/pert-ui/crates/pert-ui-runtime/src/layout.rs:1225). Those requests can change descendant cache recency and contents.

Suggested spec change: say that **the new slot operation performs its ordinary measurement in both profiles**. Remove the global parity claim.

The unbounded-height request itself is sound under the public measurement contract: measurements must depend on constraints and retained inputs, and cannot write state ([runtime.rs:1941](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:1941)). `VirtualList` still allocates its existing slot; `TreeRow` still arranges content using bounded height. The extra measurement can add legitimate theme/font dependencies and affect cache recency, but dependencies remain accumulated until invalidation ([runtime/tree.rs:1853](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1853)). Measurement-change notification only queues validation bookkeeping; it does not request another layout ([runtime/tree.rs:1887](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1887)). I found no intrinsic relayout loop or violation of AGENTS.md’s retention rule.

Slot rules remain adequate. Implementation must propagate slot-only changes to validation even when child arrangement skips, and compare height in matching units: `Node::measured_height` is scaled, while the declared slot is parent-local.

After the two wording/test corrections, the spec is ready to implement slice by slice, subject to its existing maintainer/design checkpoints. This is specification readiness, not implementation verification.

I examined only R13 and its relevant history and runtime paths against HEAD `621022a`. I did not re-audit other repairs, execute Cargo/native checks, measure performance, or inspect generated API/package artifacts. No files were edited, including TODO.md.