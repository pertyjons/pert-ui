**Not ready yet.** I found one concrete blocker, and it's in the task 6 notification grading. Everything else from the previous review is resolved, so the plan can go ahead once that sentence is fixed and the prelaunch checks are recorded.

## Blocker

**`plans/usability/round-3-plan.md:66-75`: the "error role" condition can't be met on either commit.** Line 68 says "A `ToastStack` count increase with an error role establishes existence on either commit." Toasts have no error role or severity:

- `Toast` holds only `message` and `action` (`crates/pert-ui-runtime/src/notifications.rs:148-151`).
- The stack reports `SemanticRole::Notifications` with the count as its value (`notifications.rs:942-951`). Its items carry name, actions, selection and bounds, and nothing else.
- The 0.3.75 inspection output recorded in round 2 shows the same role, a count and no items (`runs/r2-5-head-claude-1/FRICTION.md:45`). The 359122e output is the same (`runs/triage-r2/group-4.md:25`).

Read literally, no toast-based submission passes on either commit. Read loosely, each acceptance agent will interpret it their own way. Either way the paired grading is no longer consistent.

**Fix:** drop "with an error role." Existence then rests on the count going up after the failed save. Whether it shows the actual failure stays with source inspection plus a capture or the component description, as the paragraph already says.

Fix two wording problems in the same paragraph:

- **"Outside-row" (line 72)** comes from task 5. Task 6 has no rows, so say what qualifies, for example: "a visible failure message in another component (such as a `Banner`, `Alert` role, or a label), provided inspection exposes it."
- **The acceptance agents need this interpretation.** Line 76 says they "apply the same procedure," but the sealed acceptance paragraph only says "appears as a notification in inspection." On head, an agent working from that alone could demand the toast's message text, which baseline can't provide. Say that the agents receive this paragraph alongside `acceptance.md`.

## Rechecked and resolved

- **Notification grading across versions:** the rule is the same for both commits, message text isn't required, discoverability is recorded separately, and the tab must still stay modified. This holds once the error-role sentence is fixed.
- **Scope and roles:** in-scope findings, delegated design choices with written checkpoints, and the Codex-authors / Claude-reviews split are all stated. That split keeps different model families in review, as AGENTS.md prefers. The proposal's "fresh verifying agents and a Codex review" is covered by Claude's triage, Codex's check and Claude's recheck.
- **Plain output losing links:** `pandoc -f html -t gfm-raw_html` keeps link targets, and the instruction is the same in both kits (`tester-brief.md:17-20`). One caveat, not a blocker: links inside rustdoc's code blocks for signatures come out as plain code.
- **Model alias:** Claude is pinned to `claude-opus-5-5`. For Codex, the configured and effective identity are recorded separately, and an unobserved effective model isn't claimed.
- **Order bias:** each tester's queue alternates commits and the two testers use opposite orders. Within one tester, head always comes first in each pair. Since sessions are fresh, that's acceptable and balanced across the two testers.
- **Frozen commit:** baseline is 26e6b5a and head is 86a196f plus the recorded preparation commits, with no kit changes during runs. You said the exact commit gets recorded before launch.
- **Matches the proposal otherwise:** task 6 is sealed at a81449f, task 5's notification rule is now in `acceptance.md:59-65`, the harness, brief and template are the same on both commits with the overlay recorded, there's no shared cache, the 90-minute budget is unchanged, `summarize.py`'s counting is unchanged, and paired and head-only results are reported separately.

## Prelaunch checks (outside this review)

Commit hashes, the isolation probes for the new CLI versions, the no-tester probe on each kit, and confirming `/usr/bin/pandoc` and its version in both sandboxes.

Once the error-role sentence is fixed and the wording above is updated, it's ready to proceed with those checks.
