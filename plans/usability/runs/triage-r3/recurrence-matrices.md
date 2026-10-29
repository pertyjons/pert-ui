# Round 3 recurrence matrices

Status: independently approved at the 2026-10-29 records checkpoint; overall
round assessment and runtime integration remain separate. Runtime/recipe repairs
and their final assessment remain open. R = recurred; P = exercised successfully
in the timed tester session; — = not reached/proven in that session. R* is a
source-inferred recurrence, explicitly not a logged difficulty or timed event.
No passing cell means the finding is universally solved.

The exact finding definitions are in [the original plan](../../../archive/developer-usability.md).
Reasons and source/test/diagnostic references for every R/P are retained in the
initial reports linked below, with [author group-H corrections](author-head-tasks1-5.md)
and [follow-up dispositions](author-followup-probes.md). A later acceptance test
does not turn an unreached blind case into a blind P.

## Run mapping

| ID | Submission |
| --- | --- |
| C1 | [r3-1-head-claude](../r3-1-head-claude/summary.md) |
| C2 | [r3-2-head-claude-retry1](../r3-2-head-claude-retry1/summary.md) |
| C3 | [r3-3-head-claude-retry1](../r3-3-head-claude-retry1/summary.md) |
| C4 | [r3-4-head-claude-retry1](../r3-4-head-claude-retry1/summary.md) |
| C5 | [r3-5-head-claude-retry1](../r3-5-head-claude-retry1/summary.md) |
| X1 | [r3-1-head-codex](../r3-1-head-codex/summary.md) |
| X2 | [r3-2-head-codex](../r3-2-head-codex/summary.md) |
| X3 | [r3-3-head-codex](../r3-3-head-codex/summary.md) |
| X4 | [r3-4-head-codex](../r3-4-head-codex/summary.md) |
| X5 | [r3-5-head-codex](../r3-5-head-codex/summary.md) |
| HC1 | [r3-6-head-claude-1](../r3-6-head-claude-1/summary.md) |
| HC2 | [r3-6-head-claude-2-retry1](../r3-6-head-claude-2-retry1/summary.md) |
| HX1 | [r3-6-head-codex-1](../r3-6-head-codex-1/summary.md) |
| HX2 | [r3-6-head-codex-2](../r3-6-head-codex-2/summary.md) |
| BC1 | [r3-6-baseline-claude-1](../r3-6-baseline-claude-1/summary.md) |
| BC2 | [r3-6-baseline-claude-2-retry1](../r3-6-baseline-claude-2-retry1/summary.md) |
| BX1 | [r3-6-baseline-codex-1](../r3-6-baseline-codex-1/summary.md) |
| BX2 | [r3-6-baseline-codex-2](../r3-6-baseline-codex-2/summary.md) |

## Head-only tasks 1–5

Cell evidence: [initial-head-tasks1-5.md](initial-head-tasks1-5.md). Each original report names the run-specific
source, test or diagnostic for its R/P cells. Corrections below take precedence.

| Finding | C1 | X1 | C2 | X2 | C3 | X3 | C4 | X4 | C5 | X5 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| F1 | P | P | P | P | P | P | — | — | P | P |
| F2 | P | P | P | P | P | P | P | P | P | P |
| F3 | P | P | P | P | R | R | P | P | R | P |
| F4 | P | — | — | — | — | — | — | — | — | — |
| F5 | P | P | — | — | — | — | — | — | — | — |
| F6 | P | — | — | — | — | — | — | — | — | — |
| F7a | — | — | P | P | — | — | — | — | — | — |
| F7b | — | — | P | P | — | — | — | — | — | — |
| F7c | — | — | — | — | — | — | — | — | — | — |
| F8 | P | P | P | P | — | — | P | P | P | P |
| F9 | — | — | — | — | — | — | P | P | — | — |
| F10 | P | — | P | — | P | P | P | P | P | P |
| F11 | — | — | — | — | P | — | — | — | — | — |
| F12 | — | — | — | — | P | — | — | — | — | — |
| F13 | P | P | — | — | — | — | — | — | P | P |
| F14 | — | — | P | P | — | — | — | — | — | — |
| F15 | — | — | P | P | — | — | — | — | — | — |
| F16 | — | — | P | P | — | — | — | — | P | P |
| F17 | — | — | — | — | — | — | — | — | — | — |
| F18 | — | — | — | — | P | P | — | — | — | — |
| F19 | — | — | P | P | — | — | — | — | — | — |
| F20 | — | — | — | — | — | — | — | — | — | — |
| F21 | P | P | — | — | — | — | — | — | — | — |
| F22 | — | — | — | — | — | — | — | — | — | — |
| F23 | — | — | — | — | — | — | — | — | — | — |
| F24 | — | — | — | — | P | P | — | — | — | — |
| F25 | — | — | — | — | — | — | — | — | — | — |
| F26 | — | — | P | P | — | — | — | — | — | — |
| F27 | — | — | P | P | — | — | — | — | — | — |
| F28 | P | P | P | P | — | — | — | — | — | — |
| F29 | — | — | — | — | — | — | — | — | — | — |
| F30 | — | — | — | — | — | — | — | — | — | — |
| F31 | — | — | — | — | — | — | — | — | — | — |
| F32 | — | — | — | — | — | — | — | — | — | — |
| F33 | — | — | — | — | — | P | — | — | — | — |
| F34 | — | — | P | P | P | P | P | P | — | — |
| R2-1 | P | P | P | P | — | P | — | — | P | — |
| R2-2 | P | P | — | — | — | — | — | — | — | — |
| R2-3 | P | P | P | P | — | — | — | — | — | — |
| R2-4 | P | P | P | P | — | — | — | — | — | — |
| R2-5 | P | P | — | — | P | P | — | — | — | — |
| R2-6 | — | — | P | P | — | — | P | P | P | P |
| R2-7 | — | — | P | — | — | — | — | — | — | — |
| R2-8 | — | — | P | P | — | — | — | — | — | — |
| R2-9 | — | — | — | — | — | — | — | — | — | — |
| R2-10 | — | — | P | P | — | — | — | — | — | — |
| R2-11 | P | — | — | — | P | — | P | P | — | P |
| R2-12 | P | — | P | — | P | P | P | P | P | P |
| R2-13 | — | — | — | — | — | — | P | P | — | — |
| R2-14 | — | — | — | — | P | P | — | — | — | — |
| R2-15 | — | — | — | — | P | P | — | — | — | — |
| R2-16 | — | — | — | — | — | P | — | — | — | — |
| R2-17 | — | P | — | — | — | — | — | — | — | — |
| R2-18 | — | — | — | — | — | — | — | — | — | — |
| R2-19 | — | — | — | — | — | P | — | — | — | — |
| R2-20 | — | — | — | — | — | — | — | — | P | — |
| R2-21 | — | — | — | — | — | — | — | — | P | P |
| R2-22 | — | — | — | — | — | — | — | — | P | P |
| R2-23 | — | — | — | — | — | — | — | — | P | P |
| R2-24 | — | — | — | — | — | — | — | — | P | P |
| R2-25 | — | — | — | — | — | — | — | — | — | — |
| R2-26 | — | — | — | — | — | — | — | — | P | — |
| R2-27 | — | — | — | — | — | — | — | — | P | P |
| R2-28 | P | — | P | — | P | P | P | P | P | P |

## Paired task 6

Cell evidence: [initial-notes-paired.md](initial-notes-paired.md). Each original report names the run-specific
source, test or diagnostic for its R/P cells. Corrections below take precedence.

| Finding | HC1 | HC2 | HX1 | HX2 | BC1 | BC2 | BX1 | BX2 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| F1 | P | P | P | P | P | P | P | P |
| F2 | P | P | P | P | P | P | P | P |
| F3 | R | P | R | R | R | P | R | R |
| F4 | — | — | — | — | — | — | — | — |
| F5 | — | — | — | — | — | — | — | — |
| F6 | — | — | — | — | — | — | — | — |
| F7a | — | — | — | — | — | — | — | — |
| F7b | — | — | — | — | — | — | — | — |
| F7c | — | — | — | — | — | — | — | — |
| F8 | P | P | P | P | P | P | P | P |
| F9 | — | — | — | — | — | — | — | — |
| F10 | P | P | P | P | P | P | P | P |
| F11 | — | — | — | — | — | — | — | — |
| F12 | — | — | — | — | — | — | — | — |
| F13 | — | — | — | — | — | — | — | — |
| F14 | — | — | — | — | — | — | — | — |
| F15 | — | — | — | — | — | — | — | — |
| F16 | — | — | — | — | — | — | — | — |
| F17 | — | — | — | — | — | — | — | — |
| F18 | — | — | — | — | — | — | — | — |
| F19 | — | — | — | — | — | — | — | — |
| F20 | — | — | P | — | P | P | — | — |
| F21 | — | — | — | — | — | — | — | — |
| F22 | — | — | — | — | — | — | — | — |
| F23 | — | — | — | — | — | P | — | P |
| F24 | — | — | — | — | — | — | — | — |
| F25 | — | — | — | — | — | — | — | — |
| F26 | — | — | — | — | — | — | — | — |
| F27 | — | — | — | — | — | — | — | — |
| F28 | P | P | P | — | — | P | P | — |
| F29 | — | — | — | — | — | — | — | — |
| F30 | — | — | — | — | — | — | — | — |
| F31 | — | — | — | — | — | — | — | — |
| F32 | — | — | — | — | — | — | — | — |
| F33 | — | — | — | — | — | — | — | — |
| F34 | P | P | P | P | P | P | P | P |
| R2-1 | P | P | P | P | R | R* | — | — |
| R2-2 | — | — | — | — | — | — | — | — |
| R2-3 | — | — | — | — | — | — | — | — |
| R2-4 | P | P | P | P | R* | — | R | R |
| R2-5 | — | — | — | — | — | — | — | — |
| R2-6 | P | P | P | P | R | — | — | — |
| R2-7 | — | — | — | — | — | — | — | — |
| R2-8 | — | — | — | — | — | — | — | — |
| R2-9 | — | — | — | — | — | — | — | — |
| R2-10 | — | — | P | P | — | — | — | — |
| R2-11 | P | P | — | — | — | — | — | — |
| R2-12 | P | P | P | P | P | P | P | P |
| R2-13 | P | P | — | — | — | — | — | P |
| R2-14 | — | — | — | — | — | — | — | — |
| R2-15 | — | — | — | — | — | — | — | — |
| R2-16 | — | — | — | — | — | — | — | — |
| R2-17 | — | — | — | — | — | — | — | — |
| R2-18 | — | — | — | — | — | — | — | — |
| R2-19 | — | — | — | — | — | — | — | — |
| R2-20 | P | P | — | — | R | R | R | R |
| R2-21 | — | — | — | — | — | — | — | — |
| R2-22 | — | — | — | — | — | — | — | — |
| R2-23 | — | — | — | — | — | — | — | — |
| R2-24 | — | — | — | — | — | — | — | — |
| R2-25 | — | — | — | — | — | — | — | — |
| R2-26 | — | — | — | — | — | — | — | — |
| R2-27 | — | — | — | — | — | — | — | — |
| R2-28 | P | P | P | P | R* | R* | R* | R* |

## Author scope corrections and later acceptance exercise

- H/C2 F30 is —: a boxed error conversion helper compiled but never executed.
- Review 1 corrects N/R2-13: HX1/BC1/BC2/BX1 collapse absent and empty names
  in helpers and are —; BX2 directly asserts raw None and is P, as are HC1/HC2.
- N/BC2 R2-4 is —: tail-expression map(|_| ()) is needed for callback return
  conversion, so it never exercises the must-use friction.
- F20 uses one rule across both groups: actually adopt an item-page example
  with facade imports and exercise it; reading or citing pages alone is not P.
  H/C1/C3/C5 are conservatively —; C1 cited Select placeholder discovery and
  C5 cited keep_alive imports, without establishing actual example adoption.
- H/C5 R2-26 P rests on timed Job::status/four_states and its CPU capture, not
  acceptance-only evidence. H/C3 R2-15 P rests on timed picker Escape, actual
  focus-loss cancel_input and focus-ring tests; X3 P does not establish cancel_input.
- N/F14 is — throughout: the notes sidebar is selectable but does not exercise
  the editable/filterable-record recipe.
- N/F22 is — throughout: working Ctrl+S/N/W does not exercise the finding about
  Alt+Up/Down/Super editing precedence.
- N/BC2, BX1 and BX2 R2-6 are —: silent reliance on undocumented children order
  does not prove the documentation finding passed. The source observation stays
  visible; BC1 actually logged the gap and is R.
- H/X1 F4 and F6 and H/X3 F12 received additional successful acceptance exercise:
  unconfirmed draft close admission, Field status after rejection, and actual
  focus-loss cancel_input respectively. Their blind cells remain as originally
  reached; see author-acceptance-2/4 and the run-specific added test sources.
- Registry F1 positives rest on actual kit-backed builds, not just manifests.
  F11 positives require the relevant failed-conversion diagnostic, not a
  successful question-mark or an initializer signature mismatch.
- Baseline R2-1/4/28 stars mark code-inferred workarounds/forbidden UI wait use.
  No head-only or post-repair run is substituted for the frozen comparison.

Only F3 recurs at the frozen head in these matrices, across both task groups.
No R2 head recurrence was verified; many R2 cases were not reached. Baseline-only
R2 findings remain distinct from new findings such as menu/tab geometry and
the newly reproduced keyboard-hover defect. This is descriptive AI evidence,
not a score, causal effect estimate or proof of human usability.

[Independent records recheck](independent-records-review-r2.md) approves this
record after the stated author corrections; it does not approve runtime fixes
or overall round closure.
