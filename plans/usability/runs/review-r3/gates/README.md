# First repair checkpoint gate evidence

Commands ran on the working 0.3.89 source on 2026-10-28. This is evidence for
that checkpoint, not completion of usability or a release/platform matrix.
The exact temporary orchestration script is archived as runner.py.txt for
provenance; use the repository's documented commands for a new checkout.

- r3-workspace-tests-attempt3.log is the final successful `cargo test --locked
  --workspace` output. Earlier attempts exposed a stale negative Tabs fixture
  and the author custom-example placement/import assumption; see attempts/.
- workspace-and-serde/outcomes.json records every remaining command, its
  environment, timestamps and exit status. All commands in that run exited 0:
  format, workspace Clippy, packaged headless examples, workspace doctests,
  warning-free rustdoc, diagnostics snapshots and relevant serde configurations.
- r3-api-generate-attempt2.log records successful API generation with the
  pinned cargo-public-api 0.52.0. Its first attempt caught an unqualified new
  rustdoc link, repaired before the successful run. The committed-clean API
  status check still follows the implementation commit.
- r3-native-pointer-checks.log records the initial scenario; its tab selection
  claim was rejected by independent review because the tab was already selected.
  review-recheck/r3-native-pointer-checks-review2.log supersedes that claim
  with a click on an unselected category and passes both backends. Both runs
  record only the selected gallery interaction
  checks, both passed. It explicitly lists every skipped check. The native
  metadata and limits are in docs/verification.md; the selected GPU adapter
  was not printed. No human input or Windows/macOS evidence is claimed.
- Other logs record focused part, pointer, import and tab-lifecycle checks.
  Earlier author example compile failures are preserved rather than concealed.

The literal-tab rendering proposal is a separate unfinished checkpoint. No
result here claims that defect fixed. A later fix must run its relevant gates.

The review-recheck directory holds the focused checks after review repairs:
facade/gallery Clippy, facade/runtime doctests, strict workspace rustdoc,
exposed-part tests, pointer tests and the corrected native scenario. All final
commands passed. The narrow-strip extent probe intentionally failed: the first
fixture was allocated at Tabs' 96-pixel minimum, not the requested 8 pixels.
The final pointer fixture asserts its actual 96-pixel strip and uses 64-pixel
controls so the two arrows exceed it; no title is clickable.

negative-controls contains two intentional temporary source mutations. Restored
byte hashes equal the original hashes, and both mutated tests failed with the
expected assertion: parent layout invalidation on a part-name-only change, and
omitting the native scenario's tab click. No negative-control mutation remains.
These controls establish that the repaired tests detect the regressions review
identified; they are not failing production tests.

Three large gate logs contain the previously tracked development-validator
pairwise-overlap diagnostic flood from runtime.rs's static-component deadline
fixture. The .log files now explicitly label readable excerpts: only repeated
UnexpectedOverlap lines after the first three are abbreviated; every other
line is retained. Each sibling .full.log.gz preserves the complete original
stdout/stderr losslessly. compressed-output-provenance.json records raw,
compressed and excerpt hashes and exact omitted-line counts; decompression was
checked byte-for-byte. No command result, failure or ordinary output is removed
from the readable excerpt. This storage change is not a new test result.
