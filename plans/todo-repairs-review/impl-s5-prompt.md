You are an independent code reviewer for the pert-ui repository (a Rust GUI
library). Read AGENTS.md first: it is the governing instruction file. Do NOT
edit any file; read-only review. You cannot run Cargo, so your evidence is
static; say where a claim needs a run.

Scope: commit 47292a9 on branch todo-repairs ("Build collection rows without
keys and open window-mode tool windows at mount (0.3.99)"), slice 5 of
plans/todo-repairs.md (R18, R19). Review with `git show 47292a9` and the
surrounding code; the spec's slice 5 section states the requirements.

Look hardest at:
- R18 RowItem (virtual_list.rs, tree.rs, lib.rs exports): the From impls
  (coherence, the placeholder key, element metadata preserved), into_element,
  object safety, docs and the "Breaking" CHANGELOG replacement usage; the
  mechanical migration of ~96 implementations (any behavior change, e.g. a
  build that used to return None now returning Some, or a dropped key check
  in gallery data.rs), rustdoc/guide examples, the l05 diagnostics case.
- R19 (tool_windows/panel.rs): requesting the layout delivery from the
  panel's update when expanded in window mode and not detached/detaching —
  update-phase rules (no signal publication from update/reconciliation),
  convergence (revision bump -> panel update -> request loop?), behavior
  when the layout's notifier is not yet set, after a refused detach, after
  the user closes the native window, workspace restore of a window-mode
  tool window, and the tests.
- CHANGELOG, api/ listing, TODO.md removals.

Output: verdict line first ("ready", "ready with fixes" or "needs fixes"),
findings by severity (blocker / should-fix / nit) with file:line, the
problem, a concrete failure scenario and the suggested fix, then what you did
not examine.
