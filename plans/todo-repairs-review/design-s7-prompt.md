You are an independent design reviewer for the pert-ui repository (a Rust
GUI library). Read AGENTS.md first: it is the governing instruction file.
Read-only: do NOT edit files; you cannot run Cargo, so your evidence is
static.

Review `plans/document-editor.md`, the design plan for large-document text
editing (TODO.md entry TEXT-01; slice 7 of plans/todo-repairs.md). The
maintainer decided that TEXT-01 gets only this design plan now; nothing is
implemented. Its benchmark source is in plans/document-editor-bench/.

Evaluate:
1. Accuracy of every claim about the current code (file:line references,
   how TextArea/Editor/TextLayout/TextSelection/IME/limits work today, what
   shares MAX_TEXT_BYTES), against crates/pert-ui-runtime/src.
2. The measurement method and whether the reported numbers are presented
   honestly (what was and was not measured, machine, load).
3. The decision (a separate DocumentEditor over a TextDocument buffer vs
   extending TextArea): sound? Conformance with AGENTS.md (no privileged
   paths for built-ins, public API documented, separation of state and
   application data, undo ownership — the library does not own the
   application's undo history, retained layout, no polling, IME rules,
   inspection rules).
4. The per-paragraph design: correctness risks in selection/caret/IME/hit
   testing across paragraphs, bidi, shaping across paragraph boundaries,
   long paragraphs, scale changes, retained rasters, memory bounds.
5. Slices, risks and open questions: missing items, wrong order,
   unverifiable claims.
6. Its "Findings outside this plan's scope" section: are the findings
   accurate?

Output: verdict line first ("ready for the maintainer's checkpoint",
"ready with changes" or "needs rework"), then findings by severity
(blocker / should-fix / nit) with file:line evidence and the suggested
change, then what you did not examine.
