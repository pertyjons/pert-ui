# Integrated empty-raster bounds and wrapped caret presentation

Working0.3.94, 2026-10-29. Isolated source received
[independent approval](../independent-review-r3.md) after the
[recorded prose repairs](../review-repair1/README.md) and final module-comment
correction. [Design revision4](../../../../round-3-design-hanging-whitespace.md)
explicitly adopts fractional rounding, transparent phase-raster and mask-edge
sampling consequences. Full workspace/API and available native checks pass; the fresh integrated/
whole-assessment review remains pending.

source-integration.json shows all six approved baseline hashes matched root
before copying the approved source. Root preserved existing editing modules and
placed hanging_whitespace alphabetically. The new root guide/assets/contracts/
rendering/changelog/version changes are separate from the isolated source
review. Menu, popup, Button and historical-diagnostic source is untouched.
The root targeted CPU hanging-whitespace run reports12 passed, no failures.

## Execution and limits

- api/: generation exits0 with empty git status --porcelain -- api/.
- gates-attempt1/: failed workspace attempt is preserved. Root incorrectly ran
  API generation concurrently with tests. api/generate.sh line37 removes old
  listing files before regeneration, while testing_closure reads them; four
  tests fail with missing api/pert-ui/default.txt. No text assertion failed.
  The orchestration record states this is not an unchanged all-files interval.
- The retry waits for API completion and runs the entire required gate on the
  same final bytes. All eleven commands exit0 in ../../gates/final-0.3.94, with an unchanged
  source window. Production/test/guide/manifest/API bytes still match; only
  the Markdown native-evidence row was added afterwards, with exact diff and
  author links/whitespace review.
- native/: actual selected text-rendering probe and both gallery interaction
  checks exit0. The text probe reports Intel UHD Graphics 770 (ADL-S GT1), Vulkan;
  the gallery checks do not print their adapter. Platform metadata records
  Fedora44, kernel7.2.12, GNOME Shell50.6/Wayland, Rust1.99.1; workspace uses
  winit0.30.13/wgpu30.0.1. Other native checks are skipped by the filters.
- Native ran before the final retry window. Matching outer source manifests
  and root's no-code-edit interval correlate executed production bytes, but
  API listings were temporarily absent. source-correlation.json states this
  limit explicitly rather than claiming the retry's continuous guard covers it.
- Raw PNG bytes are preserved with archive hashes. Descriptive comparison of
  existing text-mode probe samples finds CPU/GPU channel differences at most1.
  This is not a new tolerance gate, performance claim, tab-specific GPU proof
  or hanging-caret assertion. The focused public CPU tests establish the caret,
  IME and clipping policy; the native probe exercises general real text masks
  and GPU readback. Runtime-injected gallery input does not prove OS-adapter,
  human or Windows/macOS behavior.

The original author evidence remains immutable, including its incorrect
default/phase heading and overbroad icon-pixel/baseline-width summaries; repair1
explicitly supersedes those claims with actual raw data. All prior request-
changes verdicts remain alongside the final isolated approval. No claim of
repaired-head blind timing or universal usability follows.

The preliminary root-targeted log has no embedded command/timestamps. Its
command was cargo test --locked -p pert-ui --features cpu --test editing
hanging_whitespace, stated retrospectively; its output reports success. The
complete retry gate is the auditable source/command record for these same
public tests, rather than an inferred preliminary unchanged-source window.
