"""Continue the stopped acceptance queues without replaying completed work.

Usage: python3 -I accept-resume-all.py <checkout> <base> <credentials>
Run only after the documented service reset. Resumes the two interrupted
verifiers in their own contexts; new submissions get fresh acceptance contexts.
"""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import json
import os
import subprocess
import sys
import threading

root, base, credentials = (Path(value).resolve() for value in sys.argv[1:4])
replacement = {entry['attempt']: entry['replacement'] for entry in
               json.loads((root / 'plans/usability/runs/review-r3/auth-start-failures.json').read_text())}
names = []
for tester in ('claude', 'codex'):
    for version in ('head', 'baseline'):
        for repeat in (1, 2): names.append(f'r3-6-{version}-{tester}-{repeat}')
    for task in range(1, 6): names.append(f'r3-{task}-head-{tester}')
names = [replacement.get(name, name) for name in names]
work = []
for name in names:
    directory = base / ('acceptance-' + name)
    if not directory.exists():
        work.append((name, False)); continue
    status = dict(line.split('=', 1) for line in (directory / 'verifier-status.txt').read_text().splitlines())
    assert status['original_source_unchanged'] == 'True', name
    if status['exit_status'] == '0':
        assert (directory / 'acceptance.md').exists() and (directory / 'acceptance.env').exists(), name
        print('SKIP completed independent verifier', name, flush=True)
        continue
    assert name in ('r3-1-head-claude', 'r3-4-head-claude-retry1'), name
    work.append((name, True))
assert len(work) == 13, 'Unexpected work list; inspect state before continuing'
stop = threading.Event()
environment = dict(os.environ, PERT_UI_USABILITY_CLAUDE_CREDENTIALS=str(credentials))

def queue(items):
    records = []
    for name, resume in items:
        if stop.is_set(): break
        print('ACCEPT RESUME' if resume else 'ACCEPT START', name, flush=True)
        command = (['python3', '-I', str(root / 'plans/usability/runs/review-r3/resume-acceptance.py'), str(root), str(base), name, str(credentials)]
                   if resume else ['python3', '-I', str(root / 'plans/usability/runs/review-r3/accept.py'), name, str(root), str(base)])
        log = base / ('acceptance-' + name + ('.resume-launcher.log' if resume else '.launcher.log'))
        with log.open('x') as output:
            result = subprocess.run(command, cwd=root, env=environment, stdout=output, stderr=subprocess.STDOUT)
        records.append({'run': name, 'resumed': resume, 'verifier_exit': result.returncode})
        print('ACCEPT END', name, result.returncode, flush=True)
        if result.returncode:
            stop.set(); break
    return records

with ThreadPoolExecutor(max_workers=2) as workers:
    futures = [workers.submit(queue, work[offset::2]) for offset in range(2)]
    records = [record for future in futures for record in future.result()]
path = base / 'r3-acceptance-verifier-resumed-outcomes.json'
with path.open('x') as output: output.write(json.dumps(records, indent=2) + '\n')
if stop.is_set(): raise SystemExit('Stopped after a verifier failure; inspect before any continuation')
print('ACCEPTANCE VERIFIERS COMPLETE; independent author review/adoption still required', flush=True)
