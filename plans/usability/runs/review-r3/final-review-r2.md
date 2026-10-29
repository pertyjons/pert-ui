I can't approve closure yet. There is one concrete blocker, and it's small. Once it's fixed as described below, the local round-3 goal can close without another blind experiment, full gate rerun or new review round.

## Blocker: stale limitation sentence in the `pert-ui-runtime` crate docs

`crates/pert-ui-runtime/README.md:474-475` still says:

> Existing empty non-tab rasters can still contribute pen positions to bounds. TextArea documents its current hanging-caret reveal limit.

0.3.94 removed both behaviors, and `text.rs`, `text_fields.rs`, the guides, the assets README, `contracts.md` and `rendering-platform.md` all describe the new policy correctly. This README is the package readme in `Cargo.toml`, and `src/lib.rs:1` includes it as the crate-level rustdoc. So the public docs contradict the delivered repair, and the 0.3.94 changelog's claim that the docs were updated is only partly true. AGENTS.md requires rustdoc to change with behavior, and design revision 4 lists the runtime README among the required updates.

**Fix:** replace those two sentences with plain prose (it renders on crates.io, so avoid intra-doc links). For example: "Zero-area glyph rasters, such as bundled-font spaces, add no placement or bounds. Wrapping TextArea pins its displayed caret inside the text box; its rustdoc describes the limits." Then run, per the rustdoc row in AGENTS.md:
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pert-ui-runtime --no-deps`
- `cargo test --locked -p pert-ui-runtime --doc`
- `git diff --check`

Author review of that diff is enough; neither a full workspace rerun nor another review is needed.

## What I verified and accept

- **Isolated vs integrated source:** the six base hashes in `base/baseline.json` equal root's pre-copy hashes in `source-integration.json`. The copied hashes equal the gate's `source-manifest.json` for `text.rs`, `tabs.rs`, `text_fields.rs`, `text_selection.rs`, `tab_whitespace.rs` and `hanging_whitespace.rs`. Root's `editing/main.rs` is textually identical to the isolated worktree's. Because root matched the isolated base before the copy, 0.3.93 had not touched these files, so no earlier repair could have been erased through them.
- **Runtime logic matches revision 4:**
  - `text.rs:1447` skips a raster only on zero width or height, after cache admission and byte accounting and before alignment and bounds. There is no alpha scan.
  - In `text_fields.rs`, `Horizontal` keeps the pin for wrapping editors only and a `max_scroll` floored at zero. It is stored in `Prepared` by both arrange (`:2665`) and paint (`:3139`).
  - The wheel (`:3578`) reuses that stored range. Paint, reveal and input-method anchoring (`:3158`) use the pinned caret. Selection fill, the composition underline and hit testing stay unpinned.
  - There is no negative scroll and no new signature, dependency or schema; the API check is clean.
- **Gates:**
  - **Retry:** all eleven commands exit 0 and the source was unchanged throughout. The test summaries show no failures, and all 12 `hanging_whitespace` and 9 `tab_whitespace` tests ran.
  - **First attempt:** it failed with exit 101 only because API generation overlapped the tests, as the orchestration record says.
  - **API:** generation is clean, and `api-committed-clean.json` is empty at ddfdb20.
  - **Afterwards:** only the one `docs/verification.md` row changed, and the diff is preserved.
- **Native:** the text probe passed and printed Intel UHD 770 on Vulkan. Both gallery runs passed 39 steps each, and the remaining checks were skipped by the name filter. The CPU/GPU difference of at most 1 is stated only as a description of this sample, which is the correct scope.
- **Ledger and TODO:** every verified finding has a fix or a reasoned disposition. The temporary whitespace and MultiSelect TODO entries are gone. TEXT-01, the overlap diagnostic flood, `Measurements::is_empty` and the other unrelated items are still recorded and were not repaired. I found no claim of universal usability, zero warnings or support on all platforms.

## Nonblocking, for the status-wording adoption

1. `new-findings.md` row 42 ends with "current limitation is qualified until fixed", and rows 41/42 say "awaits service reset". Both are stale.
2. The `round-3-design-hanging-whitespace.md` status line still says "Independent repair recheck remains required", although review r3 approved it.
3. The README says native ran "before the final retry window". By the timestamps it ended at 12:05:44.9, about 3.6 s after the retry's format step started (12:05:41.3). The overlap doesn't change any conclusion, but "before" is slightly imprecise.
4. **Out of scope, should go in TODO.md:** the native-checks summary (`apps/native-checks/src/main.rs:85` and `:796`) only recognizes a `Hardware: ` line. It therefore printed "not reported; no check that prints its adapter ran" even though the text probe printed `Text mode adapter: …` (`run1.log:21` vs `:118`). Recognizing that line, or saying "no `Hardware:` line", would make the summary accurate.

## Decision

After the README correction and its targeted doc checks are committed, the declared local round-3 goal can close. The limits that remain are the existing ones: Linux/Wayland only, with selected checks and input injected through the runtime; no Windows or macOS evidence; no human or causal-speed claim; no blind timing of the repaired head; and publication, push and merge are not authorized.

This was read-only, so I didn't recompute any hashes. The matches above come from comparing the recorded manifests with each other and reading the files directly, not from running git or sha256.
