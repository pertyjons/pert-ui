**Verdict: needs changes. One small rustdoc fix is needed; nothing else blocks.**

I only used Read/Glob/Grep. I made no edits, ran no builds or code, and delegated nothing. Menu/Select and the focus recipes were not reviewed.

## Required change

**1. `Runtime::resume_content_jump` doesn't document `RuntimeError::Hidden`** (`crates/pert-ui-runtime/src/runtime/content_search.rs:396-397`).
- The plan's revision 3 requires documenting Hidden on content-search jump forwarding.
- `resume_content_jump` is the public way to finish `ContentJump::Pending`, and it calls `reveal_and_focus(&row)` (line 403). So a host-hidden window returns `Err(Hidden)` through it.
- Its `# Errors` still has the 0.3.88 wording unchanged: "The errors of `Self::reveal_and_focus` for a stale or foreign row." That restricts the forwarded set, so a reader won't expect Hidden.
- **Why it matters:** this is the route most likely to hit a hidden window, because a pending row jump is finished on a later frame, when the window may have been minimised. A consumer following the rustdoc would treat Hidden as an unexpected failure instead of retrying once the window is visible.
- The CHANGELOG ("content-search jumps document…") and `contracts.md:1039-1040` ("including through … content jumps") claim more than the code documents.
- **Fix:** give it the same Hidden-and-retry sentence `jump_to_content` has (lines 358-359). That is a rustdoc-only change: targeted doctests plus warning-free rustdoc cover it, and the full gate doesn't need to run again.

## What I confirmed
- **Formatter reads only immutable identity.**
  - `HistoricalWidgetId(u64)` is private (`reveal.rs:62`).
  - The manual `RevealRefusal` Debug reads `WidgetId::incarnation()`, which is just `self.0.serial` (`runtime.rs:127-129`). `Node.serial` is a plain `u64`, not a `Cell` (`runtime.rs:864`).
  - There is no `mounted` read, `Shared` access or runtime lookup.
- **Output shape is preserved.**
  - `debug_struct` uses the same variant and field names, `.finish()` on the variant, and `finish_non_exhaustive` gives `WidgetId { serial: N, .. }`. Pretty output follows the same structure.
  - `ModalBlocked` uses `write_str`, which is identical to the derived output.
  - Clone/Eq/PartialEq are still derived.
- **WidgetId's own Debug is unchanged** (`runtime.rs:143-150`), and the test checks it reports `mounted: true` and then `mounted: false`.
- **Each refusal kind comes from a real refusal:**
  - a closed Dialog gives `AncestorRefused`;
  - a `View::visible_signal(false)` gives `ApplicationControlled`;
  - `set_visible(false)` gives `Hidden`.
  - A mismatched kind panics rather than being accepted. Each case is checked after opening, after unmount, after Runtime drop and after function return. The serial matches `inspection_id`.
- **Receipt, status and boxed errors are covered.**
  - The terminal `RevealStatus` comes from a real `request_reveal`, and `RevealReceipt` Debug is checked after drop.
  - The `Box<dyn Error>` is real.
  - `Cleanup` is built by the test. That's fine: a driver can't produce Cleanup around NotRevealed, because NotRevealed returns before any input. It still exercises the nested formatting.
- **Host-hidden errors are covered on these routes:** reveal, reveal_and_focus, `jump_to_content(Widget)`, `Driver::click`, `Driver::click_item`, and both queued variants ending in `Finished(Err(Hidden))`.
  - After the window is shown, retrying reveal_and_focus, click and click_item succeeds.
  - The misleading internal comment is fixed (`reveal.rs:662-663`, compared with the 0.3.88 text).
  - The README is explicit that this is headless host-visibility evidence, not native minimisation.
- **The other docs match:** the Hidden variant, `RevealOutcome`/`RevealStatus`/`RevealReceipt` rustdoc, click/click_item `# Errors`, `diagnostics.md:83-89`, `contracts.md:1035-1041`. The test module is registered (`collections/main.rs:28`).
- **Manifests are at 0.3.91:** the workspace, every `pert-ui*` crate in the lockfile, and the task4-app dependency.

## Evidence as recorded
- **CI gate:** `full-gates/outcomes.json` shows all eleven CI base commands exiting 0, and the workspace test log compiles at v0.3.91. Both reveal_diagnostics tests pass there and in `final-focused.log`.
- **Source guard:** the extra before/after source guard **exited 1**. `source-doc-only-difference.json` shows only TODO.md changed (`Row::pointer_input` became `ChoiceRow::pointer_input`), and the README keeps that failure on record. `ChoiceRow` exists at `multi_select.rs:507` with `pointer_input` at :583, so the correction is right, and the hashes of `reveal.rs` and the test file match the targeted-check manifest. I'm not asking for any rerun because of it.
- **API listing:** `api/generate.sh` exited 0 and `git status --porcelain -- api/` printed nothing.
- **Full test log:** I didn't decompress the gzip. I'm relying on the author's round-trip record for it.

## Non-blocking notes
- **Negative control is from an earlier test revision.** The log panics at `reveal_diagnostics.rs:122:5`, but the final assertion starts at line 124. Also, `negative-control-restoration.json` doesn't name the file it mutated; its hash matches `reveal.rs`. This doesn't matter in practice: derived Debug would print `mounted: true`, which the final assertion rejects.
- **Shape isn't pinned by the tests.** They check byte stability, that "mounted" is absent, and that Display contains `serial: N`, but not the exact `WidgetId { serial: N, .. }` text. One exact-string assertion would catch shape drift.
- **The Hidden-window test doesn't cover the `ContentTarget::Row` jump route or retrying a jump or queued reveal after showing the window.** These are documented, and the code reads correctly.
- **`TestError::NotRevealed` rustdoc says nothing about historical formatting.** The guide and the `RevealRefusal` docs cover it.

I make no claim here about native, OS or human input, or about overall usability.
