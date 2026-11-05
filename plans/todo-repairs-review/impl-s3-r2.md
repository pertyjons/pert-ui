needs fixes

No new production defect established statically, but the claim that every finding is fixed overstates the regression coverage.

| Prior finding | Status | Assessment |
|---|---|---|
| Stale slot during retained scaling | Partly resolved | Refresh looks correct; regression does not prove it. |
| Measured-window guard | Resolved | Committed geometry is required; suppression/reappearance test covers it. |
| FileBrowser replacement/placeholders | Partly resolved | Both paths fixed; placeholder regression coverage missing. |
| Retention/failure evidence | Partly resolved | Several required cases remain absent; failure test misses the slot measurement. |
| Horizontal check and timings | Resolved statically | Width check implemented; timing evidence recorded as author evidence. |
| Public documentation | Mostly resolved | Axis behavior documented; measurement-parity claim remains too broad. |
| Incarnation versus tree ordering | Resolved by disclosed deviation | Acceptable deterministic ordering choice. |
| Uncommitted TODO link | Resolved | Markdown link removed. |

Remaining findings:

- **P2 — scale tests do not establish the repaired invariant.** [fixed_slots.rs:292](/home/per/github/pert-ui/crates/pert-ui/tests/foundation/fixed_slots.rs:292) changes viewport DPI, while `slot_extent` multiplies by accumulated **view scale**. The separate [zoom test:344](/home/per/github/pert-ui/crates/pert-ui/tests/foundation/fixed_slots.rs:344) admittedly passes without the fix and does not assert that the parent arrangement was retained. Add a regression that confirms retained placement reuse and fails with the refresh removed, using content close enough to the slot extent to expose stale scaling. An internal runtime test is appropriate if the public fixture cannot isolate this.

- **P2 — failure test errors before reaching `place_fixed_slot`.** [fixed_slots.rs:397](/home/per/github/pert-ui/crates/pert-ui/tests/foundation/fixed_slots.rs:397) starts with unconditional measurement failure. `Band` inherits the default `Component::measure`, which measures its child before arrangement. Consequently, the first error proves ordinary measurement propagation, not slot-measurement propagation or transactional slot adoption. Make only the slot constraints fail, and verify the previous slot survives failed replacement/clearing. The requested child-only arrangement, retained movement/scrolling, forwarding-wrapper and ordinary-cache-hit consistency cases also remain uncovered.

- **P2 — the new serialized variant has no round-trip regression.** [serialization.rs:762](/home/per/github/pert-ui/crates/pert-ui/tests/foundation/serialization.rs:762) updates envelope tests to schema 18, but no serde test exercises `ExceedsFixedWidth` in issues or markers. Add that coverage. The version constant, rejection messages, envelope tests, API listings and current documentation otherwise appear consistent; remaining schema-17 references describe historical introductions.

- **P2 — placeholder repair lacks its requested test.** [file_browser.rs:1458](/home/per/github/pert-ui/crates/pert-ui/tests/collections/file_browser.rs:1458) covers populated entry rows and replacement of `EntryFailed`. It never exercises a long loading/empty message or directory-listing-error placeholder. Removing the placeholder ellipsis could therefore escape this regression. Add narrow placeholder cases, including a string-table replacement.

- **P3 — rustdoc overclaims debug/release measurement parity.** [layout.rs:1277](/home/per/github/pert-ui/crates/pert-ui-runtime/src/layout.rs:1277) says debug and release “measure the same,” although `refresh_forwarded` still measures only in debug builds. Limit the promise to this operation making its ordinary measurement request in both profiles, as the plan already qualifies.

The child-commit refresh itself looks sound. It can temporarily read the previous slot internally during a fresh arrangement, but the parent subsequently installs and records the new placement before validation runs. Retained arrangements keep the declaration and refresh its scaled extent. I found no within-pass stale-slot report path. `measured_width` also follows the height plumbing through relocation, unmeasurement, geometry, measurement-change draining and update.

The tuple and incarnation-order deviations are acceptable. Component call counts reasonably demonstrate cache reuse, but do not establish request counts or absence of requests. Treating a non-detecting zoom test as sufficient regression evidence is not acceptable under AGENTS.md’s verification requirements.

I read AGENTS.md, `git show a2cf0a6`, the prior review, affected code, tests, API diffs and documentation. The commit whitespace check passed. I changed no files and ran no Cargo commands. I did not verify reported gates, generated listings, timings, native/visual behavior, or unrelated local changes.