# MultiSelect and popup-focus integration

Working0.3.93, 2026-10-29. The [isolated MultiSelect patch](../multiselect/README.md)
received [independent Codex approval](../multiselect-codex-review-r1.md), as did
[the generic popup repair](../popup/independent-review-r1.md).
The [integrated Claude review](../multiselect-integrated-review-r1.md) approves
runtime/docs integration and requests stronger gallery Escape evidence and
record corrections. Those are addressed below; [focused independent recheck](../multiselect-integrated-review-r2.md)
approves the integration and discharges all review1 conditions. Final whole-goal/text integration remains separate.

## Complete evidence

- ../gates/final-0.3.93/: all eleven workspace commands exit0. Exact commands,
  environment, timestamps and source hashes are retained; source-unchanged
  confirms no captured bytes changed during that complete run.
- api/: generation exits0 and git status --porcelain -- api/ is empty. Review1's
  API-pending statement is historical. No public signature/schema changes.
- native/: original selected GPU and CPU/softbuffer checks exit0 on Fedora44,
  kernel7.2.12, GNOME Shell50.6/Wayland, Rust1.99.1, winit0.30.13/wgpu30.0.1.
  Adapter not reported; every other native check explicitly skipped. The
  original native run has no separate source manifest: correlation comes from
  running11:45:54–11:47:37 inside the unchanged full-gate window11:44:21–11:51:38.
  final-source-correlation.json records the same bytes still matched afterwards.
- initial-correlation.json and the before-popup log preserve the headless short-
  popup failure after successful long-list continuation; gallery-after-popup-
  attempt1.log passes. The initial source record is retrospective, explicitly
  marked. Native size does not assert page overflow. Claim the actual escaping
  outside-ScrollArea path from headless before/after and focused public tests;
  native evidence proves actual click/value/search continuation at its size.

## Review1 repairs

The original two-Escape step asserted only final closure, so its logs do not
prove the intermediate clear. The corrected step flushes after first Escape,
asserts open=true and empty search semantics by its exposed part's inspection
id, then asserts closed and unchanged value after second Escape. Root's new
row helper also walks public children and matches inspection ids, avoiding
chip/row duplicate-key ambiguity. The popup module registration was alphabetized.

review-repair1/: corrected gallery headless run passes; formatting, gallery
Clippy and focused foundation popup tests exit0. Exact reconstructed pre-review
source hashes equal the original full-gate manifest. Reconstruction is honestly
retrospective. The scoped diffs show only gallery test/helper changes and module
registration order; all production bytes still equal the full-gate source.
source-before/after records correlate the logged supplemental/native jobs; the before
record was made just after job launch with no intervening source edit.

native-review-repair1/: both selected GPU and CPU interaction checks exit0 with
the stronger Escape assertions. Source stayed unchanged through these jobs.
After them, only the0.3.93 evidence row was added to docs/verification.md; links
and whitespace receive author review. No production/public API change follows
from these supplemental test/record fixes, so unrelated expensive checks were
not repeated. API listing still matches. The committed-clean check follows the
checkpoint commit.

The controls/manual/contracts describe actual pointer-highlight ownership,
completed-click focus before callbacks, callback failure retention and already-
inside admitted-surface focus preservation. Root also corrected the old fixture
comment and renamed the existing short-list test without changing its behavior.
Raw isolated evidence/review hashes remain untouched.

No repaired-head blind result, OS-adapter, human, Windows/macOS or GPU tab-pixel
claim follows. The native short-popup case is not independent proof of overflow.

The separate gallery-headless.log has no embedded command/timestamps/status;
its command was cargo test --locked -p pert-ui-gallery --bin pert-ui-gallery
the_interaction_steps_pass_headlessly, recorded here retrospectively. Its output
reports success, but it is not claimed inside the later source-manifest window.
The corrected native checks execute the same steps within that recorded window.
Formatting/Clippy completed before source-before was written; their correlation
also relies on the stated absence of intervening edits and reconstruction.

Production/docs whitespace checks pass. The complete staged diff check reports
only archived-log whitespace (16 paths); original log bytes remain preserved,
not silently rewritten. This is an evidence-format exception, not shipped-code
whitespace. The independent reviews are read-only and do not claim to rerun gates.
