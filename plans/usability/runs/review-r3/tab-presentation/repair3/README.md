# Round 3 literal tabs: implementation 3 (remaining P2 repairs)

Implementation author's evidence for the two remaining P2 findings of
`/home/per/pert-ui-usability/r3-text-codex-review2/result.md`, in the
uncommitted worktree `/home/per/pert-ui-usability/r3-text-worktree` (HEAD
`3bbfb813a8c02c417d68386b0c5547b4b8139580`, nothing staged or committed). It is
not an independent review. The production algorithm is unchanged; this repair
edits tests and rustdoc only. No native, GPU, workspace-wide, API-listing or
human-input check was run.

## Layout

- `pre-edit/`: state before any edit. `state.log` shows the owned files equal
  implementation 2's final hashes; `*.orig` are copies of the four edited files
  as found; `previous-evidence-files.sha256` hashes all 142 files of
  `r3-text-implementation-evidence/` and `r3-text-implementation2-evidence/`;
  `evidence1-vs-impl2-record.log` shows the first set still matched its
  original 69-file record.
- `negative-controls/`: `run.py`, per-control `.diff` and `.log`,
  `post-restore-rebuild.log`, `summary.txt`, and
  `other-sources-at-control-time.sha256`. `attempt1/` is the first run on an
  earlier `tabs.rs` (before the single-direction guard and the literal check at
  both scales); same outcome, but its summary extractor missed the panic text.
- `checks/` and `run-checks.out`: final targeted checks (`run-checks.py`), each
  with its command, exit status and full output.
- `production-bytes.py`: check 16, proving no production line changed.
- `final-state/`: final state and hashes, the full tracked diff, the untracked
  `tabs.rs` as a diff, and the repair-only diffs against `pre-edit/`.

## Modified paths

- `crates/pert-ui-runtime/src/text/tabs.rs`: test module only (check 16: the
  4,229 bytes before `#[cfg(test)] mod tests` are unchanged).
- `crates/pert-ui-runtime/src/text.rs`: `TextLayout` and `TextLayout::size`
  rustdoc only.
- `crates/pert-ui-runtime/src/text_fields.rs`: `TextArea` rustdoc only.
- `crates/pert-ui-runtime/src/text_selection.rs`: one `caret` rustdoc sentence;
  non-doc lines equal HEAD (checks 15, 16).

Unchanged: fixture assets, `tab_whitespace.rs`, `editing/main.rs`, manifests,
lockfile, versions, CHANGELOG, guides, API listings, main and other worktrees.

## P2-1: fixed-coordinate hit and word assertions

In `hanging_tabs_shrink_the_extent_and_translate_right_to_left_public_coordinates`
(all eight cases, scales 1 and 1.25), at every point, the test now asserts the
following.

1. **Reference at the identical public point.** `at_point` focus and
   `word_at_point` byte range, anchor and focus (offset and affinity) equal
   expectations computed from the reference's own clusters. They use the
   reference's own origin, computed independently: `expected_canvas(…, true)`
   is the same raster-bounds calculation with tab ink counted. The test also
   asserts that canvas equals the reference's public canvas. No origin delta
   is subtracted.
2. **Repaired layout at that point.** The hit as before, plus the word with an
   independent byte range and both endpoint affinities.
3. **Reference at the point moved by the origin change.** The hit and the full
   word equal the repaired expectation.

The independent helpers are:

- `cluster_at`: the visual-cluster walk.
- `expected_hit`: the side rule.
- `word_starts`: UAX #29 rules for the test texts.
- `expected_word`: anchor upstream (downstream at 0), focus upstream. It
  asserts single-direction text, because Parley gives a focus before a word
  of the other direction downstream affinity. Negative control N11 showed this
  in the mixed Hebrew/Latin text of another test.

Per scale, the test asserts:

- all four point categories occur: (a) glyph, (b) origin band, (c) hanging tab,
  (d) outside/above/below;
- at least one point differs between the identical-point reference and the
  repaired layout (the log shows 10 at each scale);
- the review's literal point.

**Literal point** (checks/03 line 63, scale 1; line 174, scale 1.25). The
"RTL wrapped, hanging tab on a middle line" case at (6.499997, 10) and
(6.5, 10):

| Layout and point | Hit | Word |
|---|---|---|
| Repaired, same point | (8, Upstream) | 0..8: (0, Downstream)–(8, Upstream) |
| Reference, same point | (8, Downstream) | 8..9: (8, Upstream)–(9, Upstream) |
| Reference, translated point (9.5 / 9.7, 10) | (8, Upstream) | 0..8 |

Other discriminating points, all asserted:

- origin band (1.5, 10): repaired (8, Downstream), word 8..9; the reference
  at the same point gives (9, Upstream);
- above/below: repaired (6, Upstream) / (20, Upstream); the reference gives
  (6, Downstream) / (20, Downstream).

The full table is in checks/03 (debug) and checks/04 (release), 154 hit lines
each.

## P2-1 negative controls (`negative-controls/summary.txt`)

| Control | Mutation in `text_selection.rs` | First failure in the hanging test |
|---|---|---|
| N10 | `at_point`: `let x = point.x() * layout.scale;` (origin translation removed) | `tabs.rs:1246`, reference at the same point (6.499997, 10): got (8, Upstream), expected (8, Downstream) |
| N11 | `word_at_point`: `point.x() * layout.scale` | `tabs.rs:733` (`assert_word`), same point: got 0..8 (0,D)–(8,U), expected 8..9 (8,U)–(9,U) |

Both also fail `tabs_keep_the_reference_geometry_and_unicode_analysis_without_their_ink`.

**Restoration:** the original bytes were written back (not `cp -p`), the file
was touched, and its sha256 `a0cbc2e3…5d28` was checked as identical after
each control. A post-restore focused run recompiled `pert-ui-runtime` ("Compiling
pert-ui-runtime" in `post-restore-rebuild.log`) and passed. `tabs.rs`, `text.rs`
and `text_fields.rs` were not edited after the controls: their hashes in
`other-sources-at-control-time.sha256` still match. The final checks ran after
the restoration.

## P2-2: owned public documentation

- **`TextArea`:**
  - "scrolls internally to reveal the caret" now names the exception.
  - A new paragraph states that horizontal scrolling stays within
    `TextLayout::size` plus the caret width. That size leaves out hanging
    whitespace (`Editor::position` clamp).
  - The verified clipping cases, for spaces and tabs alike: LTR End or typing
    the hanging space/tab, the RTL hanging line end, and the wrapped-RTL
    text-start caret. All are from the base and repaired runs in
    `r3-text-implementation2-evidence/facade-probe/final/`, for example
    `base.out:75` and `repaired.out`, typed-space and typed-tab.
  - The RTL space/tab difference, caused by the empty space raster.
  - The scrollbar paragraph's "reveals the caret again" now refers to the same
    limit.
  - The mixed-ligature tab exception stays.
  - No scrolling code changed.
- **`TextLayout`:**
  - It no longer claims an exact drawn-ink union. The size is the layout box
    joined with the drawn glyphs' raster bounds.
  - An empty raster, such as a bundled-font space, extends the size to its pen
    position; a hidden tab adds nothing.
  - In RTL, a hanging space can therefore extend the size and move the origin
    where a tab of the same advance does not, so the two give different size,
    origin and caret coordinates. This is the existing caveat; no twin
    equivalence is claimed.
  - The mixed-ligature exception paragraph is unchanged.
- **`TextLayout::size`:** the same qualification, linking the type
  documentation.
- **`TextSelection::caret`:** one sentence saying RTL space and tab coordinates
  can differ.

No empty-raster geometry or scrolling repair was made.

## Checks (`run-checks.out`, all exit 0)

| # | Check | Result |
|---|---|---|
| 01 | fmt | exit 0 |
| 02 | Clippy runtime, all targets, `-D warnings` | exit 0 |
| 03 | `text::tabs` debug | 11 passed |
| 04 | `text::tabs` release | 11 passed; only the pre-existing `measurements.rs` `is_empty` dead-code warning, also in implementation 2's log |
| 05 | Exact hanging test | 1 passed |
| 06 | Runtime `--lib text` (text, text_fields, text_selection) | 53 passed, 1 ignored |
| 07 | Runtime doctests | 392 merged + 23 standalone passed |
| 08 | Facade editing | 322 passed |
| 09 | Facade cpu `tab_whitespace` | 8 passed |
| 10–11 | Runtime and facade rustdoc, `-D warnings` | exit 0 |
| 12 | `git diff --check` | exit 0 |
| 13 | status | exit 0 |
| 14 | Owned hashes | recorded |
| 15 | `text_selection.rs` rustdoc-only | exit 0 |
| 16 | Production bytes | unchanged |
| 17 | Previous evidence | all 142 files unchanged; the first set also matches its original 69-file record |

## Limits

- `word_starts` models only the UAX #29 cases its texts use: letters, space
  runs, tabs and line breaks. `expected_word` refuses mixed-direction text.
- Point coverage is the existing generated set: per line, (a) one glyph, (b)
  the origin band and (c) every hanging tab where they exist, (d) far left/right
  of each line plus above and below. Category (b) exists only in the RTL
  overhang cases.
- The accepted impossible-case clarifications are unchanged: mixed atoms never
  hang in Parley, and ellipsis candidates cannot end in a tab. The RTL
  space/tab twin difference is the existing empty-raster behavior, now
  documented.
- The documentation's reveal statements rest on implementation 2's recorded
  facade runs (base `3bbfb81` extraction and repaired source). They were not
  re-executed here, because no runtime or facade code changed.
- Root TODO, design, guide, contracts, CHANGELOG, workspace and API gates, and
  the independent recheck remain with the integrator.
