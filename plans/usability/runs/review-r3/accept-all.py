"""Apply the frozen acceptance procedure in fresh independent contexts.

Usage: python3 accept-all.py <checkout> <run-base> <credential-source>
Starts only after all planned successful blind submissions are terminal.
"""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import json
import os
import subprocess
import sys
import threading

root, base, credentials = (Path(value).resolve() for value in sys.argv[1:4])
replacements = {entry["attempt"]: entry["replacement"] for entry in
    json.loads((root / "plans/usability/runs/review-r3/auth-start-failures.json").read_text())}
expected = []
for tester in ("claude", "codex"):
    for variant in ("head", "baseline"):
        for repeat in (1, 2):
            expected.append(f"r3-6-{variant}-{tester}-{repeat}")
    for task in range(1, 6):
        expected.append(f"r3-{task}-head-{tester}")
expected = [replacements.get(name, name) for name in expected]
for name in expected:
    conditions = dict(line.split("=", 1) for line in (base / name / "conditions.env").read_text().splitlines() if "=" in line)
    if not conditions.get("ended") or conditions.get("outcome") != "ended":
        raise SystemExit(f"Blind submission not terminal/successful: {name}")
    if (base / ("acceptance-" + name)).exists():
        raise SystemExit(f"Acceptance already exists; inspect its live handle/state before resuming: {name}")
environment = dict(os.environ, PERT_UI_USABILITY_CLAUDE_CREDENTIALS=str(credentials))
stop = threading.Event()

def queue(names):
    outcomes = []
    for name in names:
        if stop.is_set():
            break
        print("ACCEPT START", name, flush=True)
        with (base / ("acceptance-" + name + ".launcher.log")).open("x") as output:
            result = subprocess.run(["python3", str(root / "plans/usability/runs/review-r3/accept.py"),
                name, str(root), str(base)], cwd=root, env=environment,
                stdout=output, stderr=subprocess.STDOUT)
        outcomes.append({"run": name, "verifier_exit": result.returncode})
        print("ACCEPT END", name, result.returncode, flush=True)
        if result.returncode:
            stop.set()
            break
    return outcomes

with ThreadPoolExecutor(max_workers=2) as workers:
    futures = [workers.submit(queue, expected[offset::2]) for offset in range(2)]
    outcomes = [outcome for future in futures for outcome in future.result()]
(base / "r3-acceptance-verifier-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
if stop.is_set():
    raise SystemExit("Acceptance queue stopped for inspection; do not replay existing verifiers.")
print("ACCEPTANCE VERIFIER QUEUES COMPLETE; author verification/copy still required", flush=True)
