# Author adoption: extra input probes P12–P14

2026-10-29. These are scoped follow-up probes, not additional blind sessions or
acceptance tests. Read the source and printed command/value observations;
passing scope assertions alone do not establish absence of the defects.

P12 confirms the MenuTitle lead: parking over View before F10/Down opens View
and Enter runs Zoom; never/Leave/over-File controls run Save. A genuine Move
after opening switches correctly. Leave followed by a same-coordinate host
Move still switches, so rejecting only refreshed events is insufficient.
Adopt into the Menu/Select checkpoint with explicit position-change gating.

P13 confirms a separate MultiSelect defect, including callback consequences.
The four-row popup stays on Gamma despite Down; the long popup stays on O03
after twelve Downs and Enter toggles O03. Never/Leave/outside controls behave
as documented. Source inspection confirms Row::pointer_input writes highlight
on every inside Move. Recorded in TODO.md for its separate row-family scope;
no claim that fixing shared OptionList repairs MultiSelect.

P14 does not establish a legitimate wrong Hidden fallback. The one reproduced
fallback requires application code to claim unknown native window visibility,
outside HostIntegration's contract. Decline that proposed defect. The genuine
hidden-window case returns RuntimeError::Hidden from reveal and focus/click;
adopt the missing-error documentation and misleading-comment correction.
Deferred scroll work runs after reveal, so it does not prove an in-reveal race.

Archive integrity: every file hash matches archive-manifest.json; all three
head/baseline observation pairs are byte-identical. The .sh suffix is changed
to .sh.txt for archival use; bytes are unchanged. Original external directories,
frozen kits and submissions remain unchanged. The first scroll fixture did not
overflow; its unsuccessful evidence remains with the corrected later runs.
No native, GPU, OS input, human testing or repaired-head result follows.
