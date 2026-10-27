# Blind-test records

Round 1 of [the developer-usability plan](../../developer-usability.md),
run on 2026-10-21. Exploratory evidence from AI testers: the times and
counts describe how each tester worked, not a usability score.

## Conditions

- **Kit:** prepared from commit 359122e on branch `developer-usability`
  (0.3.52, Rust 1.99.1). The plan says the round runs on `main`; the
  library at 359122e equals `main`'s apart from the move to Rust 1.99.1 in
  0.3.52, which changed no source.
- **Testers:**
  - Claude Code 2.1.306 with model `claude-opus-5-5`, effort high.
  - Codex CLI 0.163.0 with model `gpt-6.1-sol`, effort medium.
- **Isolation:** each tester ran in the [kit's sandbox](../README.md) with a
  budget of 90 minutes. Claude and Codex ran in parallel, one run each at a
  time, each run with its own target directory.
- **Acceptance:** for each run, a fresh Claude agent applied
  [the acceptance procedure](../acceptance.md). It built and tested a copy
  of the submission in the sandbox and added its own headless tests where a
  tester's check was weak; those tests are in its report, not kept here.
  The author spot-checked each report against the source, tightened one
  verdict (r1-1-codex) and confirmed one the agent had marked as borderline
  (r1-2-claude). Codex's review of these records then led to a third
  decision (r1-1-claude, confirmed by an author probe) and a recorded
  reading for moves under a filter (r1-2-codex). Each run's `acceptance.md`
  records those decisions.

## Runs

| Run | Task | Session (min) | First build (min) | Builds (failed) | Friction entries | Acceptance |
|---|---|---|---|---|---|---|
| [r1-1-claude](r1-1-claude) | 1 settings editor | 5.5 | 0.3 | 8 (1) | 7 | partly: an uncommitted typed change does not block closing |
| [r1-1-codex](r1-1-codex) | 1 settings editor | 6.3 | 2.6 | 6 (2) | 6 | partly: native close refusal untested, in-view window added for testing |
| [r1-2-claude](r1-2-claude) | 2 task list | 7.6 | 2.6 | 11 (3) | 8 | partly: actions inert for a shown selection when the query has a trailing space |
| [r1-2-codex](r1-2-codex) | 2 task list | 6.9 | 1.0 | 10 (4) | 4 | met |
| [r1-3-claude](r1-3-claude) | 3 custom control | 8.0 | 1.8 | 21 (8) | 7 | met |
| [r1-3-codex](r1-3-codex) | 3 custom control | 3.8 | 1.6 | 4 (1) | 2 | met |
| [r1-4-claude](r1-4-claude) | 4 testing an app | 4.5 | 1.0 | 8 (3) | 9 | met |
| [r1-4-codex](r1-4-codex) | 4 testing an app | 4.2 | 2.3 | 3 (1) | 5 | met |

No tester read library source. The `/kit/vendor` paths in the
transcripts are compiler and runtime diagnostic locations, or listings of
the directory. The proxy logs show only the model hosts and Codex's denied
analytics host.

## What each directory holds

- **Run records:** `conditions.env`, `summary.md` and `diagnostics.md` (from
  `summarize.py`), `acceptance.md` and `acceptance.env`, and the tester's
  `FRICTION.md`.
- **`submission/`:** the consumer source as submitted, without `Cargo.lock`
  (the kit's vendored registry fixes the versions), build output, logs, and
  the testers' own helpers for reading the rendered documentation as text.
- **`captures/`:** the PNGs that tasks 3 and 4 require. They were
  re-encoded losslessly, pixel-identical, because the library's PNG encoder
  wrote them about a hundred times larger.

Not kept: target directories, transcripts and the rest of the raw build
logs.

## Kit fixes found in round 1

Before round 2, each with what was done (2026-10-27):

- **Task 4's app README:** it gives row paths as `list/<id>/…`, but `Each`
  puts its rows under `list/items/<id>/…`. *Done by the library:* since
  layering part L3 (0.3.74) `Each`'s default container adds no address
  segment, so the README's `list/<id>/…` is the real path. The fixture's
  `.key("row")`, which `Each` replaces with the item's key, stays: `build`
  must return an `Element`, and a key is the only way to make one (TODO.md,
  `Each::new`).
- **Task 4's app `Cargo.toml`:** it requires 0.3.51, which led one tester to
  edit it; the requirement could name the kit's version. *Done:* `run.sh
  launch` sets the copied application's requirement to the kit's version,
  and the fixture itself requires the branch's version.
- **The bridge script:** `bridge.py` sits in `/kit/bin`, where a tester read
  it, and shows how the sandbox is built. It could be mounted outside
  `/kit`. *Done:* it is mounted at `/run/bridge.py`, which the tester's
  nested namespace hides with the proxy socket, and is no longer copied
  into the kit.
- **The acceptance procedure:** it should say plainly that the author's own
  check can establish an item the tester's check covers only partly. These
  acceptances read it that way. *Done:* `acceptance.md` step 3 says so.
- **Friction-log times:** several testers first wrote estimated times and
  corrected them from `date`. Entries were often written in batches, so
  their times are write times rather than event times. *Done:*
  `friction-log.md` asks for the time at the moment of the event and marks
  an entry written later with `(written later)`; round 2's comparison
  treats unmarked times as event times and marked ones as write times.
- **Task 4's fixture, `let _ =`:** an application-lifetime command needed
  `let _ = ui.register_command(…)` while `CommandId` was `#[must_use]`
  (author notes; fix-plan part 6b1 left it for the kit fixes). *Done:* the
  fixture registers the command with a plain statement.
