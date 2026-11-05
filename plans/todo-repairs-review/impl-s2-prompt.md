You are an independent code reviewer for the pert-ui repository (a Rust GUI
library). Read AGENTS.md first: it is the governing instruction file. Do NOT
edit any file; read-only review. You cannot run Cargo (read-only sandbox), so
your evidence is static; say where a claim needs a run.

Scope: commit ea3b324 on branch todo-repairs ("Center in-place drafts and
check documentation imports (0.3.96)"), which implements slice 2 of
`plans/todo-repairs.md` (items R08–R11). Review it with `git show ea3b324`
and the surrounding code. The spec's slice 2 section states what each item
must do.

Look hardest at:
- R11 `DraftAlign::Center` (text_fields.rs origin calculation, draft.rs):
  correctness of the centered origin with the caret reservation; scrolling,
  reveal, hit testing, selection painting and IME anchoring all using the
  same origin; multi-line and embedded editors; redescription; rustdoc
  accuracy; whether the tests in crates/pert-ui/tests/editing/draft_editor.rs
  would catch a regression (e.g. Center behaving like Start or End).
- R10 diagram editor (examples/diagram_editor/canvas.rs, session.rs): the
  NodeView `renaming` flag, reconcile, the canvas update's redescription
  when a rename starts or ends (including end through visibility_changed,
  end_rename, undo/document replacement), no update loop, the session's
  painted-label check and its robustness.
- R08 AppError docs and tests/foundation/app_error_conversions.rs: accuracy
  of the new rustdoc against the blanket From impl and DrawError; test
  coverage of all accepted types and variant normalization.
- R09 the `#![doc(test(attr(deny(unused_imports))))]` lint and the corrected
  examples: any example whose meaning changed, macro-generated doc
  (on_binding_refusal), the hidden allow in glob-only blocks.
- CHANGELOG accuracy, api/ listing changes, TODO.md removals.

Output: verdict line first ("ready", "ready with fixes" or "needs fixes"),
then findings by severity (blocker / should-fix / nit), each with
file:line, the problem, a concrete failure scenario and the suggested fix.
Finish with what you did not examine.
