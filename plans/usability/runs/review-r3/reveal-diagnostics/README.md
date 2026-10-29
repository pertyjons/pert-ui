# Historical reveal diagnostics checkpoint

Author implementation on working 0.3.90, integrated version 0.3.91, 2026-10-29.
Approved menu/diagnostics revision 3 governs this diagnostics portion. Menu
implementation and public focus recipes are separate patches, not evidence
claimed here. No frozen blind source or acceptance criterion changed.

`RevealRefusal` now formats only immutable runtime-local serial identity. Its
private formatter does not read Cell or query Runtime. Standalone WidgetId Debug
remains live. Real Dialog, application-bound View and explicit set_visible
refusals are retained through opening, unmount, Runtime drop and function return.
Tests compare Display, normal/pretty Debug, RevealOutcome, terminal RevealStatus,
RevealReceipt, Cleanup and a real boxed dyn Error. ModalBlocked text is unchanged.

The hidden-window test uses the public headless host contract to report known
visibility, confirming existing Hidden errors from reveal/focus, Driver
click/item, jump, both queued reveal variants, and successful retry after showing.
It establishes no native minimization or OS/human behavior. Rustdoc/contracts and
the diagnostics guide describe these existing operational errors and identity
formatting. No signature, schema, selection, focus or reveal behavior changes.

## Commands and attempts

- attempt1.log: cargo test --locked -p pert-ui --test collections reveal_diagnostics
  passed, but author review caught fixture root overlap diagnostics. The fixtures
  now arrange siblings under a Column. This was an author test-layout issue.
- attempt2.log: same command passed with no overlap diagnostics after repair.
- negative-control-live-debug.log: restoring derived Debug made the historical
  formatting test fail as expected (exit 101). negative-control-restoration.json
  records exact byte restoration. No negative-control mutation remains.
- final-focused.log: same focused command passed after real boxed-error,
  both queued variants and content-jump coverage was added.
- checks/: actual command headers, timestamps, environment, exit statuses and
  source hashes for format, targeted all-targets Clippy, the collection reveal
  tests, runtime/facade doctests and warning-free rustdoc. All passed, hashes
  match before/after. Those targeted checks precede the version-only .91 update.
- Full .91 workspace, API generation and independent review are recorded
  separately; completion is not inferred from targeted checks.

Focused command descriptions outside checks/ were written retrospectively;
raw output preserves their actual test filter/name/version. Source-level review
also checked every changed public error/doc path. No new native, GPU,
Windows/macOS, human-input or overall usability closure claim is made.

## Complete 0.3.91 workspace gate

full-gates/ records all eleven CI base commands, each exiting zero, including
examples, strict rustdoc and diagnostic snapshots. The runner's additional
before/after source guard then exited 1 because TODO.md's source item name was
corrected from Row::pointer_input to ChoiceRow::pointer_input during the run.
source-doc-only-difference.json proves that exact prose-only difference and
that every other captured file, including compiled source and tests, matches.
This guard failure is retained, not represented as an unchanged-source run.
No Cargo check failed and no compiled source changed. Author review covers the
TODO correction; it needs no test rerun.

The archive's helper explicitly accepts that documented prose difference.
Repeated known overlap diagnostics are abbreviated only in the readable test
excerpt; the full gzip and hashes preserve exact bytes with checked round trip.
The root scheduling fixture's existing TODO remains, distinct from the new
test fixture overlap that was corrected before the final gate.

## Independent review and repair

[Implementation review r1](../reveal-diagnostics-implementation-review-r1.md)
accepted formatter/source/tests with one required missing resume_content_jump
Hidden/retry sentence. review-repair/ preserves the correction, focused tests,
content-search doctests, strict rustdoc and Clippy (all zero), exact source
correlation and explicit author adoption of that conditional result. The raw
needs-changes verdict remains intact. No new runtime behavior was added.
API generation passed and api/ stays unchanged. This checkpoint does not close
the menu/text/recipe work or the whole usability round.
