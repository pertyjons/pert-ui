"""Run fresh replacements for the authenticated-start failures, in frozen order.

Usage: python3 resume-claude.py <checkout> <run-base> <credential-source>
This does not overwrite an attempt, kit, source credential or global config.
"""
from pathlib import Path
import json
import os
import subprocess
import sys

root, base, credentials = (Path(value).resolve() for value in sys.argv[1:4])
jobs = [
    ("r3-2-head-claude", "tasks/2-task-list.md"),
    ("r3-6-head-claude-2", "reserved/6-notes-editor.md"),
    ("r3-6-baseline-claude-2", "reserved/6-notes-editor.md"),
    ("r3-3-head-claude", "tasks/3-custom-control.md"),
    ("r3-4-head-claude", "tasks/4-testing-an-app.md"),
    ("r3-5-head-claude", "reserved/5-job-monitor.md"),
]
# Every original must be demonstrably terminal and must have failed before
# model execution, not merely look unfinished after an observation timeout.
for name, _ in jobs:
    run = base / name
    conditions = dict(line.split("=", 1) for line in (run / "conditions.env").read_text().splitlines() if "=" in line)
    if not conditions.get("ended") or conditions.get("exit_status") != "1":
        raise SystemExit(f"Original is not a terminal start failure: {name}")
    results = [json.loads(line) for line in (run / "log/transcript.jsonl").read_text().splitlines()]
    if not any(event.get("type") == "result" and event.get("is_error") and
               "Failed to authenticate" in str(event.get("result", "")) for event in results):
        raise SystemExit(f"Original is not the documented auth failure: {name}")
    if (run / "log/builds.jsonl").exists():
        raise SystemExit(f"Original started cargo; inspect before replacing it: {name}")
    if (base / (name + "-retry1")).exists():
        raise SystemExit(f"Replacement exists; inspect its live handle, never replay: {name}")

environment = dict(os.environ,
    PERT_UI_USABILITY_CLAUDE_CREDENTIALS=str(credentials),
    PERT_UI_USABILITY_CLAUDE_MODEL="claude-opus-5-5",
    PERT_UI_USABILITY_CLAUDE_EFFORT="high")
for name, task in jobs:
    replacement = name + "-retry1"
    variant = "baseline" if "-baseline-" in name else "head"
    print("START", replacement, flush=True)
    with (base / (replacement + ".launcher.log")).open("x") as output:
        result = subprocess.run([str(root / "plans/usability/run.sh"), "launch",
            str(base / ("kit-r3-" + variant)), str(base / replacement), "claude",
            str(root / "plans/usability" / task), "90"], cwd=root, env=environment,
            stdout=output, stderr=subprocess.STDOUT)
    if result.returncode:
        raise SystemExit(f"Launcher failed: {replacement}: {result.returncode}")
    conditions = dict(line.split("=", 1) for line in (base / replacement / "conditions.env").read_text().splitlines() if "=" in line)
    subprocess.run(["python3", "-I", str(root / "plans/usability/summarize.py"), str(base / replacement)], cwd=root, check=True)
    if conditions.get("outcome") != "ended":
        raise SystemExit(f"Stopped for inspection after non-successful session: {replacement}")
    print("END", replacement, flush=True)
print("REPLACEMENT QUEUE COMPLETE", flush=True)
