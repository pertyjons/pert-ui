**Verdict: needs changes.** The metrics records can be approved as they are. The recurrence matrices and the new-findings ledger need the corrections below. I used read-only tools only: no edits, builds, execution or delegation. I'm not approving any implementation or the round's closure.

**Review checkpoint:** `main` at 8b2476a plus the uncommitted working tree from the start of this session (0.3.91 diagnostics work, modified `author-extra-input-probes.md`, untracked design and review files). The TODO and source statuses may change after this checkpoint.

## Required changes

**1. Task 6 R2-13 row is miscoded** (`recurrence-matrices.md:165`, json `groups.N["R2-13"]`). Four of the P cells come from helpers that turn `None` and `Some("")` into the same `""`, so they never test the empty-label rule:
- **HX1 P → —:** `label()` uses `unwrap_or_default` (`main.rs:414-419`), then asserts `""` at `:485` and `:653`.
- **BC1 P → —:** `notes.rs:83`, asserting `""` at `:187`.
- **BC2 P → —:** `acceptance.rs:73`, asserting `""` at `:156` and `:280`.
- **BX1 P → —:** `main.rs:456-463`, asserting `""` at `:613`.
- **BX2 — → P:** it renders `Label::new("")` (`main.rs:304`), reads the raw name (`:475-476`) and asserts `None` at `:524` and `:638`, with no friction.
  - Baseline 0.3.75 already documents that an empty name becomes absent (`diagnostics.rs:481-487`, `:581`), so this is not the undocumented-reliance case the R2-6 correction excludes.
  - If you still prefer —, state why explicitly.
- HC1 (`editor.rs:244`, `:279`) and HC2 (`app.rs:214`) assert `None` directly, so their P stands.
- The reason line at `initial-notes-paired.md:245` should be corrected in the matrix's correction section, not in the raw report.

**2. Two P cells cite only acceptance evidence, though timed evidence exists.** Both cells stand, but their stated basis must change.
- **H/C5 R2-26:** the initial report tags it `[A]` and cites "fraction kept" from acceptance.
  - The timed basis is `Job::status` (`lib.rs:306-313`), which runs through `four_states()` in `cpu_capture_of_jobs_in_several_states`.
  - The run's own `captures/capture.png` shows the Cancelled bar at 30% and the Failed bar at 60%.
- **H/C3 R2-15:** the cited evidence is "C3 acceptance acc2/acc5". The tester's own tests cover it: `tests/picker.rs:90-141` (Escape, then focus loss running `cancel_input`) and `:257-290` (focus ring).
- **H/X3 R2-15:** keep P, but note that `cancel_input` was never executed. That is the same reason X3 F12 is —.

**3. H/C3 F20 P uses a looser rule than group N.**
- Its basis is reading the `ColorRole::FocusRing` and `ColorPicker` contracts (`FRICTION.md:81`). It never adopted an item-page example with facade imports.
- Group N's rule is "reading pages alone is not counted".
- Make C3 —, and apply one written F20 rule to both groups. C1 (`FRICTION.md:52-53`) and C5 (`:37`) are borderline under either rule.

**4. Baseline BC2 R2-4 P should be —.**
- Every case is `cx.defer(..).map(|_| ())` as a callback's tail expression (`lib.rs:266`, `:317`, `:350`, `:371`). That conversion to `Result<()>` is needed anyway, so BC2 never met the `#[must_use]` friction.
- `TaskReceipt` is still `#[must_use]` at baseline. By the R2-6 principle, the cell is unreached, not passed.

**5. Ledger P9 and P14 status is stale at this checkpoint** (`new-findings.md:36-37`).
- The P9 row says "final workspace/API/review pending". The records show more:
  - all eleven commands in `reveal-diagnostics/full-gates/outcomes.json` exited 0 (per the review);
  - API generation is clean;
  - implementation review r1 returned **needs changes**: `resume_content_jump` lacked the Hidden/retry sentence;
  - `review-repair/author-adoption.md` discharges that condition, and the sentence is now at `content_search.rs:398`;
  - an integrated review is still owed.
- The P14 row should mention the r1 gap and its repair.

**6. The notes/N5 command-action sub-point is claimed fixed without consumer-facing evidence** (`new-findings.md:30`, "explain action/queued navigation use. Fixed 0.3.89").
- The point (BC2 #7, `initial-notes-paired.md:278`) is that docs don't say an action may call operations that flush.
- Neither `register_command` (`commands.rs:572-612`) nor `invoke_command` (`:1051-1093`) says so, and no guide chapter does.
- Only the design note `docs/contracts.md:1172` states it. Cite where consumers can read this, or keep the sub-point open or explicitly declined.

## Checks that passed

- **Run mapping:** 18 runs, identical between md and json. Every run has `acceptance.env`, its summary says "Acceptance: met", and the adoption index lists all 18 as met. The six `-retry1` runs match the six authentication failures, each with zero model tokens, tool calls and cargo commands.
- **Metrics:** every field of all 18 rows matches its summary. The task 4 line definition matches the summaries, and its type count includes the fixture (application types 16 with 0 application lines). Medians are 8.05/12.25 (Claude) and 5.50/6.65 (Codex). The frozen definitions in `summarize.py` read the same as the copy at `target/p7-captures/base-src`.
- **Hashes:** all 18 `summary_sha256` values equal each run's `snapshot.json`. I couldn't recompute SHA-256 with read-only tools. The initial reports' `source_sha256` was not recomputed either; Git reported those files unmodified at session start.
- **md vs json:** every cell agrees across both groups.
- **Corrections you asked me to check:**
  - **Applied correctly:** C2/F30 is —. F14 and F22 are — throughout group N. BC2, BX1 and BX2 R2-6 are —, and BC1 R2-6 R is backed by `FRICTION.md` 17:03. X1 F4/F6 and X3 F12 stay —, with the later acceptance exercise noted separately.
  - **Qualifiers kept:** X5 is not counted under R2-26, and the R* marks on BC2 R2-1, BC1 R2-4 and the baseline R2-28 cells are preserved.
- **Conversion errors don't inflate findings:** F11 counts only C3's real conversion diagnostic. The E0271/E0308 mismatches in X2, X3, X5 and HX1, and the borrow/cast errors, are classed as tester or N5 items.
- **Recurrence:** only F3 recurs at head (C3, X3, C5; HC1, HX1, HX2).
- **Source reading:** only BC1 logged a source read (`FRICTION.md:75-82`), and it is disclosed. C1, C4 and C5 state they read none.
- **Claims and wording:**
  - None of the three records claims human usability or that all 18 meeting acceptance means anything was universally fixed.
  - Repairs after 0.3.88 are described as regression-tested and reviewed, not blind-tested.
  - The initial reports are retained, with their historical pending statuses. Their citation errors and the ListDropPosition correction are made outside them (`author-acceptance-2.md:22-30`, `author-head-tasks1-5.md`).
- **Ledger cross-checks:**
  - **Status lines:** the menu design (r3 approved), tab ink (Codex r1 needs changes) and MultiSelect (`TODO.md:243-252`) rows match their records.
  - **TODO entries:** the AlertDialog (`:217-226`), item-reporters (`:170-180`), dropdown-paint (`:228-235`) and Button-paint (`:236-241`) entries the ledger points to exist.
  - **Inspection sections:** Tabs, MenuBar and Dialog now have "Inspection and tests" sections (`tabs.rs:278`, `menu_bar.rs:625`, `dialog.rs:177`, `:508`).
  - **Declined leads:** these agree with the author records.

Apart from item 6, I found no verified new issue or wrongly declined lead missing from the ledger.

One minor note outside this scope: `TODO.md` has blank lines inside its flat list (lines 227 and 242). I didn't record it, since this review is read-only.
