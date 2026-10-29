# Public focus-continuity recipes

The isolated Claude author patch received [fresh Codex approval](../focus-recipes-codex-review-r1.md).
Its exact author evidence is in author/, with archive-provenance.json naming
unchanged bytes and archived-script suffixes. Root integrated only guide/layout,
the two new area test modules and their mod lines, preserving reveal_diagnostics.

The selected-tab recipe intentionally covers an already-focused strip. Deferred
work performs a supported nested flush before checking current observed selection
and focus, so raw key_input changes since queuing cannot briefly focus an old
editor. Missing editors leave focus alone. All observation writes occur in the
event phase. The Button-row recipe bootstraps an empty source, then publishes
rows with the mounted list identity; it sets active and requests list focus.

Author review corrected the guide's phrase "opens on a pointer press" to the
actual matching primary press/release activation contract. Root also linked
VirtualList::on_activate to the recipe and stated the focus consequence. The
independent triage review found command actions' may-flush statement only in
design notes, so register_command and invoke_command now document it for
consumers, including recursive BorrowConflict. No runtime behavior/API change.

Integrated checks in integrated/ record actual commands/timestamps/statuses:
row and tab focus contracts, layout doctests, relevant runtime docs, all-targets
facade Clippy, strict runtime/facade rustdoc and format, all passed on 0.3.91.
The on_activate doc filter selected zero doctests (that method has no example);
its new prose/link is covered by strict rustdoc, not falsely counted as an
executed example. Author evidence also covers no-default-features recipes.
Final-source-hashes.json matches every integrated source listed. Full workspace
evidence belongs to the runtime checkpoints; no native/human or overall round
closure follows from these public examples. Final integrated review still owed.
