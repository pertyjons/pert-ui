"""Historical serial blind-test queue; refuses to reuse any run directory."""
import os
from pathlib import Path
import subprocess
import sys

root = Path('/home/per/github/pert-ui')
base = Path('/home/per/pert-ui-usability')
tester = sys.argv[1]
assert tester in ('claude', 'codex')
versions = ('head', 'baseline') if tester == 'claude' else ('baseline', 'head')
jobs = [(6, versions[0], 1), (6, versions[1], 1), (1, 'head', None),
        (2, 'head', None), (6, versions[0], 2), (6, versions[1], 2),
        (3, 'head', None), (4, 'head', None), (5, 'head', None)]
tasks = {1:'tasks/1-settings-editor.md', 2:'tasks/2-task-list.md',
         3:'tasks/3-custom-control.md', 4:'tasks/4-testing-an-app.md',
         5:'reserved/5-job-monitor.md', 6:'reserved/6-notes-editor.md'}
env = dict(os.environ, PERT_UI_USABILITY_CLAUDE_MODEL='claude-opus-5-5',
           PERT_UI_USABILITY_CLAUDE_EFFORT='high')
for task, version, repetition in jobs:
    name = f'r3-{task}-{version}-{tester}' + (f'-{repetition}' if repetition else '')
    run = base/name
    if run.exists():
        raise SystemExit(f'Refusing to overwrite {run}; inspect before resuming.')
    print(f'START {name}', flush=True)
    cmd = [str(root/'plans/usability/run.sh'), 'launch', str(base/f'kit-r3-{version}'),
           str(run), tester, str(root/'plans/usability'/tasks[task]), '90']
    with (base/f'{name}.launcher.log').open('x') as output:
        result = subprocess.run(cmd, cwd=root, env=env, stdout=output, stderr=subprocess.STDOUT)
    if result.returncode:
        raise SystemExit(f'Launcher failed {name}: {result.returncode}')
    subprocess.run(['python3','-I',str(root/'plans/usability/summarize.py'),str(run)],
                   cwd=root,check=True)
    print(f'END {name}', flush=True)
print('QUEUE COMPLETE', tester, flush=True)
