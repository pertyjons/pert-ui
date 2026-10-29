# Round 3 new findings and repair ledger

Status: independently approved records checkpoint, 2026-10-29. Later repair
statuses continue to be updated; final whole-assessment review remains pending. The frozen head is 0.3.88, baseline 0.3.75. Repairs below are subsequent
source changes with regression/review evidence, never repaired-head blind runs.

The source reports retain every per-run observation and positive note:
[head tasks 1–5](initial-head-tasks1-5.md), [paired notes](initial-notes-paired.md)
and [early notes triage](initial-head-task6.md). Author corrections and declined
leads are in [author decisions](author-decisions.md),
[group H](author-head-tasks1-5.md), [P7–P11](author-followup-probes.md) and
[P12–P14](author-extra-input-probes.md). The table deduplicates repairs; it does
not replace those per-run records. F/R2 recurrence is in the separate
[matrices](recurrence-matrices.md), including unreached and acceptance-only cases.

## Verified findings

| Finding / original IDs | Verified conclusion and disposition | Current evidence/status |
| --- | --- | --- |
| Component/import discovery: H/F3 | component omitted ApplicationError despite its authoring promise. Re-export that existing facade-owned error; keep Viewport, drawing and live-data types in their intentional modules and improve guide pointers. | Fixed 0.3.89, committed 3bbfb813; independent re-review and workspace/serde/API gates passed. |
| Testing imports: H/N2 | Guide introduction overpromised the testing import seed for paint-output inspection. Name component imports and move its custom widget example to the appropriate chapter. | Fixed 0.3.89. |
| Composite discovery: H/N3, notes/N2–N3 | Select anchor, menu titles/rows and editor search/clipboard controls need public scoped discovery without private-key traversal. Add exposed parts independently of addressing, semantics and persisted workspace paths; document actual Dialog paths. | Fixed 0.3.89, public custom/built-in scope, metadata-idle, lifecycle, window and serialization tests. AlertDialog confirmation-button parts remain a separate existing TODO finding. |
| Pointer item geometry: notes/N1–N2 | Tab titles and shared OptionList rows lack public item rectangles. Supply live clipped paint/hit geometry, update click_item's supported examples and test actual pointer actions. | Fixed 0.3.89, headless plus selected Linux wgpu/CPU native interaction checks. No OS-input/other-platform claim. Other item reporters remain separately tracked. |
| Disabled Button: H/N4 | Enabled and disabled framed buttons used the same fill. Adopt Surface/Muted/Outline with inward BorderWidth; document shared field-frame roles and frameless exception. | Fixed 0.3.90, committed b9e4c6c. CPU pixels in both palettes, cancellation/idleness/theme tests; full workspace/API and final independent review passed. Separate Select/MultiSelect paint paths and missing Button paint-inspection metadata remain TODO entries. |
| Named App initializer: H/N5 | Show RuntimeError builder wrapping in App::new's public example. No new generic initializer error API. | Fixed 0.3.89, runnable headless example/doctest. |
| Message conversion discovery: H/N6 | Existing From<String>/From<&str> suffice; link them from ApplicationError::new. | Fixed 0.3.89; duplicate constructors declined. |
| Empty editor semantics: H/N7 | Empty nonprotected editor reports Some(Text(String::new())), distinct from absence. Document precise Semantics/TextField/TextArea inspection behavior. | Fixed 0.3.89 with supplemental strict documentation evidence; the text patch also corrects malformed rustdoc markers found during its authoring. |
| Mailbox recipe: H/N8 | Complete guide ownership/import example, receiver lifetime, independent signal handler, Full retry and Closed termination. | Fixed 0.3.89; ordinary UI sender contract stays separate from callback-safe producer endpoints. |
| Table source recipe: H/N9 | Explain event-phase cell callback use, next-update observation, per-mounted-row work and BorrowConflict; no callback timing change. | Fixed 0.3.89. |
| Command routing link: H/N10, notes/N5 | Routing link fixed 0.3.89. Review identified that actions-may-flush was still only in design notes; register_command/invoke_command now state it and recursive BorrowConflict explicitly. | Consumer rustdoc correction committed 17c6c3d on 0.3.91; affected doctests/strict docs passed in focus-recipes/integrated. No new global shortcut route. |
| Retained tab lifecycle: notes/N5 | Dynamic/reconcile already retain keys and reveal flushes before key lookup. Add lifecycle recipe and explicit application refocus policy for selected-tab removal. | Core recipe/precision fixed 0.3.89; guarded editor-refocus recipe independently approved and integrated, with focused tests/doctests/Clippy/strict docs passed. Nested settlement before rechecks is explicit. |
| Clickable row continuity: notes/N4 / P7 | Button row pointer opening is supported; set_active alone leaves focus on the Button, so subsequent Enter can reopen it. Document explicit list-focus composition; keep on_activate Enter-only. | Frozen probes verify existing route; complete public recipe and positive/negative tests independently approved and integrated. Public on_activate rustdoc gains the cross-link. |
| Menu keyboard highlight: notes/N6 / P8 | Stationary refreshed pointer retargets active rows and stalls scrolling keyboard navigation. | Verified on both kits; isolated implementation and required test/doc repairs independently approved, committed dae748d (0.3.92); workspace/API and selected Linux GPU/CPU native checks passed, independent integrated approval conditions discharged. Whole-round review remains separate. Keep pointer-driven data refresh. |
| MenuBar title switching: P12 | F10/Down can open View instead of File under a parked pointer. Duplicate native positions also count incorrectly as movement. | Verified on both kits; approved per-title tracking committed dae748d (0.3.92), with genuine title-to-title motion, targeted mutation proof and selected native checks preserved. |
| Literal tab ink: notes/N7 / P10 | Canonical U+0009 is retained but drawn as a missing-glyph box. Preserve original Unicode analysis and advance; hide tab-only coverage with mixed custom-font atom exception. | Approved revision 4; independent Codex reviews required and verified raw public geometry, rich-font/icon and fixed-point word/hit repairs. Final isolated approval received; committed dae748d (0.3.92), full workspace/API and independent integrated source review passed. Native runs do not establish tab-ink GPU pixels. |
| Historical refusal diagnostics: notes/N8 / P9 | Live WidgetId Debug changes a retained refusal's text after teardown. Missing ancestor key/path is optional, not the defect. | Implemented identity-only RevealRefusal Debug with actual Dialog/View/explicit-hide lifecycle tests, receipts/boxed/cleanup formatting, negative control and host-hidden retry tests. All eleven workspace commands and API generation passed. Review r1 required resume_content_jump error docs; repair plus targeted documentation/tests/Clippy passed, author adopted that condition and committed d479516. Final integrated/whole-round review still owed. |
| Hidden-window error docs: P14 | Legitimate host-hidden window returns operational RuntimeError::Hidden through reveal, clicks/items, jump and queued status. | Actual public tests confirm; reviewer r1 found missing resume_content_jump wording, now repaired and checked with the diagnostics patch at d479516. No refusal-model change. |
| MultiSelect keyboard highlight: P13 | Separate ChoiceRow overwrites highlight under a stationary pointer: Down cannot leave Gamma/O03 and Enter toggles it. | Verified both kits with actual callbacks; tracked in TODO until separate proposed checkpoint is reviewed, implemented and tested. Shared OptionList repair alone does not fix it. |
| MultiSelect long-row click focus: current-source follow-up | A short popup keeps search focus after a click; an overflowing popup focuses its list, leaving Enter/text input away from search even after pointer Leave. | Actual 0.3.91 public probe and callback/search-text observations confirm; approved separate MultiSelect design includes admitted-click focus ordering before app callbacks. No baseline/native claim for this extra lead. |
| Popup press-focus escape: MultiSelect follow-up | A short popup in an overflowing ScrollArea loses focus to its outside ancestor and closes before the pressed row handles input; no value/callback is written. | Actual unmodified-source run06 verified; generic admitted-surface focus-boundary checkpoint proposed and tracked temporarily in TODO. No claim the isolated MultiSelect patch repairs pre-dispatch routing. |
| Empty-raster bounds: text follow-up | A zero-area space raster's pen point expands RTL canvas despite no coverage, producing different extent/origin from hidden tabs. | Actual base/repaired evidence and source verified; separate hanging-whitespace design proposed within this goal. No fix claimed from the tab-ink patch. |
| Hanging-caret reveal: text follow-up | Typing/LineEnd at hanging whitespace and some RTL edge positions clips the caret because horizontal scrolling excludes its actual rectangle. | Actual base/repaired public input/captures establish the defect. Separate reviewed geometry step follows, including wheel consistency and negative offsets; current limitation is qualified until fixed. |

## Declined leads and bounded suggestions

- Dynamic writes are observed before reveal's key lookup; the claimed missing
  timing capability is contradicted by frozen execution. Tabs choosing the next
  record and focusing their strip after selected-content removal is documented
  behavior; editor refocus is an explicit application policy.
- P14's purported wrong Hidden fallback was not reached by a legitimate path.
  Its one reproduction falsified known host visibility. Record the operational
  error documentation instead, without claiming a newly proven runtime defect.
- ScrollAxes::Both retains width bounds and scrollbars overlay by documented
  policy. No width/layout rewrite or gutter API follows from a covered border.
- ListDropPosition already owns its key and implements Clone; its borrowed
  accessor is not proof of missing ownership. Canonical typed quantities,
  choice Eq requirements, App size errors and borrow/cast diagnostics are
  documented behavior, with targeted guide pointers where useful.
- Existing constructor/composition routes cover the proposed message/find,
  padding, semantic-text/capture/drag helpers, chord spelling and subtree
  address conveniences. They were not introduced solely as alternate spelling.
  An infallible App::size, new bound-Tabs source and a full app-author tutorial
  are not required by a verified missing capability.
- Long pandoc output is a reading-cost observation, not evidence of a broken
  kit. Source reading was allowed if logged; baseline Claude's toast source
  check is disclosed. Isolation is not inferred merely from passing builds.
- Lexical Job-name sorting and one failure per five at a random point satisfy
  the frozen task wording; neither establishes natural sorting or randomized
  failure selection as an extra acceptance requirement. Determinate bars on
  terminal rows do not exercise the terminal ProgressStatus contract.

No passing acceptance or matrix cell is a universal usability result. Existing
unrelated TODO entries and absent Windows/macOS/human evidence remain explicit;
publication is outside this goal. Final closure still requires completed repairs,
relevant gates, independently reviewed matrices/assessment and a final decision.
