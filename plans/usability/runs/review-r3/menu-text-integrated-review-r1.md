**Verdict: I approve the source and integration, with no code blockers.** Two things must happen before final adoption: API generation has to finish and be recorded, and one small `TODO.md` record is missing. Everything else checks out.

## Integration correctness (verified)

- **Library source is unchanged from what Codex approved.** The final-gate source manifest gives these hashes:
  - menu side: `select.rs 7ee35013…`, `menu.rs ff9d6701…`, `menu_bar.rs a9241464…`, `activation.rs 29d28116…` and `menu_pointer_ownership.rs ad3d618c…`. All equal `menu-pointer/repair2/final/hashes.txt`.
  - text side: `text.rs 2417e148…`, `text/tabs.rs bafd5784…`, `text_fields.rs e07a31f6…`, `text_selection.rs a0cbc2e3…`, the fixture assets and `tab_whitespace.rs`. All equal repair3's owned-file hashes.
  - On main, `key_armed()` is crate-private (`activation.rs:31`), and the `pointer_owns`/`last_pointer` transitions in `select.rs:556-763` and `menu_bar.rs:386-404` match the approved diff.
- **Copying the worktree files erased nothing from .89, .90 or .91.**
  - The menu worktree was based on 0.3.90. The .91 manifest still carries the 0.3.90 hashes for all four menu files, so .91 never touched them.
  - The text worktree was based on 0.3.89. Its base files (`text.rs 202bd99b…`, `text_selection.rs e7bab670…`, `text_fields.rs 91a21ddd…`) equal the .90 and .91 states.
  - The `61f00dc9…` hash for `text_fields.rs` in the 0.3.89 gate manifest looks like a mismatch, but that manifest predates a doc-only edit (`later-doc-only-differences.json`).
  - .89's parts survive: `expose_part` for trigger, items, submenus, titles, control and options, plus row `local_bounds` in `select.rs:894`. .90's inactive-frame trigger doc is at `menu.rs:675`.
  - `surfaces/main.rs` keeps .91's `tab_focus_recipe` and adds `menu_pointer_ownership`. `editing/main.rs` keeps its existing modules and adds `tab_whitespace`.
- **No API, dependency or schema changes visible in the manifests.** `Cargo.toml`, `Cargo.lock` and the task4 fixture changed version only (0.3.92), and the dependency table is unchanged. The fixture font is loaded only by tests (`tabs.rs:102-120`), with provenance and license recorded.
- **The native scenario is meaningful.** Neither step starts from an already-selected state.
  - Title step: F10 then Down opens File while the pointer rests on View. The old gate would switch to View, whose first row toggles the grid and fails both the `expanded` check and the "Created sketch" check.
  - Row step: the pointer is parked on row 1, which is the "Open recent" submenu. Under the old gate, Enter opens that submenu and changes no status, so both assertions fail.
  - The step works in window coordinates: it reads `SemanticItem.bounds` directly, and its `center` helper uses `Runtime::bounds`.
  - Negative-control logs show each restored old gate failing on its exact assertion text. Hashes were restored identically, and the restored run recompiled `pert-ui-runtime` and passed.
  - Native run1 passed both new steps in GPU and CPU modes on Fedora 44/Wayland. The adapter was not reported and other checks were skipped by the name filter, as you stated.
  - The native run (10:34:46–10:36:34) falls inside the workspace gate's unchanged-source window, so it ran on the manifest's exact bytes.

## Gates

- **Workspace gate: finished, all 12 commands exited 0, source unchanged.** It ran format, Clippy, workspace tests, five examples, doctests, strict rustdoc and diagnostics. The in-repo copy `gates/final-0.3.92` shows `the_interaction_steps_pass_headlessly`, the `tab_whitespace::*` tests and the menu-ownership tests running with no failures.
- **API generation: pending.** `r3-menu-text-api/generation.log` has a start record and file list but no exit status, and there is no committed-clean check yet. Root must finish it and confirm the `api/` diff is empty before adoption, since the CHANGELOG says no public signature changed.

## Required before adoption (records, not code)

1. **`TODO.md` has no entries for the two verified follow-ups.** One is empty-raster pen-point bounds in `TextService::rasterize_glyphs`; the other is the `Editor::position` / wheel horizontal clamp that leaves hanging carets unrevealed.
   - AGENTS.md requires verified out-of-scope findings to be recorded in `TODO.md`. The tab design (revision 4 and its clarification) also asks for this.
   - `docs/contracts.md:1251` already says the limitation "is recorded for follow-up", which is currently untrue, and MultiSelect P13 sets the precedent of a TODO entry alongside its design.
   - Fix: add two entries pointing to `round-3-design-hanging-whitespace.md` and remove them when that checkpoint lands.

## Nonblocking notes

- `crates/pert-ui/src/guide/controls.md:78` still says unsupported characters use the missing-glyph mark without the tab exception. Its runtime README twin (`README.md:506`) was updated; `controls.md:375` covers tabs, but the two paragraphs should match.
- The native negative-control record (`outcomes.json`) doesn't archive the exact mutation diffs or the gallery source hash at control time. The title mutation can be inferred from the unused-`moved` warning; the row mutation can't. Archive both alongside the in-repo copy. The positive run on final bytes is still covered by the workspace gate.
- `plans/usability/runs/README.md:315-317` still lists historical diagnostics and focus recipes as "remain tracked", although .91 committed them, and it doesn't mention .92 yet. Leave this for the whole-round records recheck.

I made no edits and ran no builds or programs. This approval covers integration and documentation only. It doesn't claim whole-round closure, OS-adapter or human input, tab-ink GPU evidence, or the empty-raster and hanging-caret repairs.
