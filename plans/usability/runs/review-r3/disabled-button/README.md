# Disabled Button checkpoint evidence

Focused checks ran on working 0.3.90 source on 2026-10-28. Full workspace
checks and independent implementation reviews passed; see the
[final approval](../disabled-button-records-review-r3.md). No overall round closure.

- cargo clippy --locked -p pert-ui-runtime -p pert-ui --features pert-ui/cpu --all-targets -- -D warnings: passed.
- cargo test --locked -p pert-ui --features cpu --test presentation disabled_button: final passed.
- cargo test --locked -p pert-ui --test controls component_availability: passed.
- cargo test --locked -p pert-ui --test presentation theme: passed. The filter
  also runs matching names in other modules, as the log shows.

The first new-test attempt painted before flushing input and returned
PendingUpdates. The second parked the pointer inside the root Button's actual
allocated bounds and therefore saw ButtonHover, not Button. Those were author
fixture errors: flush after press, and use a genuinely outside pointer event.
No runtime fix was made for those errors. Attempt 3 and the versioned final
focused run pass. Earlier outputs remain intact.

The exact CPU captures show enabled/disabled Button beside an empty placeholder
TextField and a disabled TextField in both palettes, plus a disabled icon button.
Author viewed both. The neutral frame is deliberately shared; centered captions
and the new surface/boundary distinguish the disabled action from its enabled
counterpart. Tests check real fill/boundary pixels and retain palette tests for
role contrast. No native, GPU, Windows/macOS or human-input claim is made.

The command descriptions above were recorded retrospectively; the full-gate
runner records command headers, timestamps, status and source hashes directly.

## Final evidence after review tightening

The initial and final complete workspace runs both passed every required base
command. Their actual command headers, timestamps, terminal exit statuses and
source manifests are in full-gates-initial/ and full-gates-final/. Only the
latter covers the final lazy frame-role reads, additional Menu rustdoc and
tightened tests. final-source-check.json records a fresh comparison: every
captured current source byte matches that final manifest. The repeated-overlap
diagnostic flood is abbreviated in readable excerpts; sibling full.log.gz files
preserve exact original bytes with checked decompression and hashes.

r3-disabled-button-review-focused.log is the final focused CPU run, including
the separate frameless scene, immediate idle assertions, shared-field pixels
and actual zero/3px/shape-filling border widths. Command:
cargo test --locked -p pert-ui --features cpu --test presentation disabled_button.
r3-disabled-button-review-clippy.log records:
cargo clippy --locked -p pert-ui-runtime -p pert-ui --features pert-ui/cpu --all-targets -- -D warnings.
Both passed. These focused command strings are retrospective; the full-gate
headers and outcomes are recorded directly by their archived runner.

captures-final/ supersedes the earlier comparison images and adds the disabled
frameless No frame button. Its provenance.json records exact copied PNG hashes;
no image transformation was performed. Actual test pixels cover the disabled
field and frameless control as well as the action buttons. Both images were
viewed by author and independent reviewer.

API generation completed successfully and leaves api/ unchanged, as
api-generated-clean.json records. Initial implementation review and recheck
approved source/tests; the recheck's pending-gates concern is now discharged
by full-gates-final/ and its source check. This record update receives the
final checkpoint review. No new native/GPU/human evidence is claimed.

The full-gates-initial/ and full-gates-final/ archives are copies of the private
runner directories named in their runner scripts. Capture copies were made
after the final workspace run and the focused run; both write the same paths
with unchanged deterministic test source. The exact producing invocation was
not retained for those copies, so the provenance does not attribute it to one.
The final reviewer could inspect readable excerpts but not decompress full
logs with its read-only tool set; author verified exact gzip round trips.
Optional focused enabled-frameless focus-ring coverage remains source-reviewed,
not a newly executed test.
