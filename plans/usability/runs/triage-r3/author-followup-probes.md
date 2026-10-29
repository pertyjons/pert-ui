# Author review of P7–P11 frozen-kit verification

2026-10-28, Codex. Read source, harness wrapper, actual observations, final
outputs and captures from the fresh Claude verification. The archived sources
and logs are in [followup-probes](followup-probes/README.md); its shell wrapper
is .sh.txt because it records exact local paths and is not a supported
checkout-independent build script. This is author-side reproduction, not
another blind run, acceptance change or native/human result.

The passing probe tests assert that input reached the intended scope and
print the behavior under investigation. A pass does not mean the defect is
absent. In particular, actual P8 callback logs establish wrong commands, and
P9 prints the same retained error's changing text.

| Lead | Author disposition |
| --- | --- |
| P8 / notes N6 | Verified: F10 reaches File, Down opens the expected list, and Enter runs Gamma/Beta after a prior pointer location with no new host move. Long-menu Down stalls after each scroll. Pointer Leave/no cached pointer controls reach the expected command/row. The source's unconditional refreshed-Move highlight assignment explains both. Repair under the menu-keyboard/diagnostics checkpoint, preserving pointer-driven refresh. |
| P9 / notes N8 | Verified low-severity diagnostic defect: refusal kind and serial never change, while nested WidgetId Debug changes mounted after teardown. Fix historical RevealRefusal Debug locally; preserve WidgetId's useful live Debug elsewhere. The missing ancestor key/path remains an optional enhancement, not the changing-state defect. |
| P7 / notes N4 | Existing documented behavior: Button content handles the press, arrows bubble, and Enter goes to the still-focused Button. set_active alone is insufficient. The tested composition requests list focus as well when continued list keyboard navigation is desired. Add a complete public recipe; applications choosing to focus the opened editor make that explicit instead. No on_activate contract expansion. |
| P10 / notes N7 | Public frozen-kit CPU evidence confirms the tab box on both versions while canonical text is retained. The exact glyph-0 identification still comes from the private 0.3.89 probe; do not mislabel that as a frozen-kit glyph API. Repair is the separately approved text checkpoint. |
| P11 / notes N5 | Dynamic and direct redescription retain selection on addition; removing the selected/focused tab chooses the next with Redescription origin and focuses the strip. This follows the contract. Add an explicit tested application policy to focus the replacement editor only when appropriate; avoid stealing focus from elsewhere. |

P10's same B position after tab or space is expected for TextArea's default
monospace typography, not evidence of a layout failure. The earlier Body-font
probe's tab-versus-space advances are not the metrics of this TextArea fixture.
The probe author reported that uncertainty candidly; it does not reopen the
chosen original-text/advance-preserving coverage policy.

Iteration 1's short viewport clipped the third text box; only the verifier's
fixture was enlarged. Both iterations and the final source are retained.
Baseline kit.env dirty=1 is the declared brief overlay, not a source repair.
The source of the probe was shared across kits with only exact dependency
version changing; final observation lines/captures are identical between them.
No original submission or kit was modified.

Two additional scope precisions for the final matrix: a read-only selectable
notes list does not exercise F14's editable/filterable-list recipe, so use —
for group N instead of a qualified P. Ctrl+S/N/W alone also does not exercise
F22's Alt+Up/Down/Super editing-precedence issue; group N is — there.
Undocumented children-order reliance in baseline BC2/BX1/BX2 does not prove
R2-6's documentation gap passed, so conservatively use —, retaining the source
observation. Keep the explicit baseline R2-1/4/28 code-inferred recurrences
marked as such; do not invent timing or logged friction for them.

The full matrix, completed recipes/runtime repairs, final gates and independent
whole-assessment review remain owed.
