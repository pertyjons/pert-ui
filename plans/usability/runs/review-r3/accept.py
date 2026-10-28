"""Run a fresh Claude acceptance context for one terminal blind submission."""
import os
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

name=sys.argv[1]
root=Path(sys.argv[2]).resolve()
base=Path(sys.argv[3]).resolve()
assert name.startswith('r3-') and '/' not in name
run=base/name
env=dict(line.split('=',1) for line in (run/'conditions.env').read_text().splitlines() if '=' in line)
if not env.get('ended') or not env.get('outcome'):
    raise SystemExit('Submission is not terminal; refusing to evaluate a changing run.')
version='baseline' if '-baseline-' in name else 'head'
kit=base/f'kit-r3-{version}'
kit_env=dict(line.split('=',1) for line in (kit/'kit.env').read_text().splitlines() if '=' in line)
for key in ('commit','version','toolchain'):
    if env.get(key)!=kit_env.get(key):
        raise SystemExit(f'Kit mismatch in {key}; refusing to grade.')
prelaunch=json.loads((root/'plans/usability/runs/review-r3/prelaunch.json').read_text())
if (kit/'kit.env').read_text()!=prelaunch[version]:
    raise SystemExit('Kit preparation metadata differs from the frozen snapshot.')
procedure=root/'plans/usability/acceptance.md'
if hashlib.sha256(procedure.read_bytes()).hexdigest()!=prelaunch['acceptance_sha256']:
    raise SystemExit('Frozen acceptance has changed; inspect before grading.')

def source_manifest(work):
    records={}
    for parent,dirs,files in os.walk(work,followlinks=False):
        dirs[:]=[d for d in dirs if d not in {'target','.git','.claude','.codex'}]
        for name in dirs+files:
            path=Path(parent)/name
            relative=str(path.relative_to(work))
            if path.is_symlink():
                if not path.resolve().is_relative_to(work.resolve()):
                    raise SystemExit(f'Escaping symlink: {relative}')
                records[relative]='symlink:'+os.readlink(path)
            elif path.is_file():
                records[relative]=hashlib.sha256(path.read_bytes()).hexdigest()
    return records
original=source_manifest(run/'work')
original_records={str(p.relative_to(run)):hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in [run/'conditions.env', *sorted((run/'log').rglob('*'))] if p.is_file()}
acceptance=base/f'acceptance-{name}'
if acceptance.exists():
    raise SystemExit(f'Refusing to overwrite existing acceptance {acceptance}')
subprocess.run([str(root/'plans/usability/run.sh'),'exec',str(kit),str(acceptance),'true'],
               cwd=root,check=True)
task_number=int(name.split('-')[1])
tasks={1:'tasks/1-settings-editor.md',2:'tasks/2-task-list.md',
       3:'tasks/3-custom-control.md',4:'tasks/4-testing-an-app.md',
       5:'reserved/5-job-monitor.md',6:'reserved/6-notes-editor.md'}
prompt=(root/'plans/usability/runs/review-r3/acceptance-prompt.md').read_text().format(
    run_dir=run,acceptance_dir=acceptance,kit=kit,task_number=task_number,
    task=root/'plans/usability'/tasks[task_number],procedure=procedure,
    round_rules=root/'plans/usability/round-3-plan.md',harness=root/'plans/usability/run.sh')
(acceptance/'verifier-prompt.md').write_text(prompt)
(acceptance/'original-source-manifest.json').write_text(json.dumps(original,indent=2)+'\n')
(acceptance/'original-record-manifest.json').write_text(json.dumps(original_records,indent=2)+'\n')
config=acceptance/'verifier-config'
config.mkdir(mode=0o700)
credentials=config/'.credentials.json'
credential_source=Path(os.environ.get('PERT_UI_USABILITY_CLAUDE_CREDENTIALS',str(Path.home()/'.claude/.credentials.json')))
shutil.copyfile(credential_source,credentials)
credentials.chmod(0o600)
credential_hash=hashlib.sha256(credentials.read_bytes()).hexdigest()
child_env=dict(os.environ,CLAUDE_CONFIG_DIR=str(config),DISABLE_AUTOUPDATER='1',
               ENABLE_CLAUDEAI_MCP_SERVERS='false',CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC='1')
cmd=['claude' ,'-p',prompt,'--tools','Read,Write,Edit,Bash,Glob,Grep',
     '--disable-slash-commands','--setting-sources','',
     '--strict-mcp-config','--mcp-config','{"mcpServers":{}}',
     '--dangerously-skip-permissions','--model','claude-opus-5-5',
     '--effort','high','--output-format','stream-json','--verbose']
with (acceptance/'verifier-transcript.jsonl').open('x') as output, (acceptance/'verifier-stderr.log').open('x') as stderr:
    result=subprocess.run(cmd,cwd=acceptance,env=child_env,stdout=output,stderr=stderr)
if credentials.exists():
    if hashlib.sha256(credentials.read_bytes()).hexdigest()!=credential_hash:
        credentials.rename(acceptance/'credentials-after-verifier.json')
        print('Verifier changed credential copy; retained privately, never copied to host.',flush=True)
    else:
        credentials.unlink()
records_unchanged=all(p.is_file() and hashlib.sha256(p.read_bytes()).hexdigest()==digest
                      for relative,digest in original_records.items() for p in [run/relative])
unchanged=source_manifest(run/'work')==original and records_unchanged
(acceptance/'verifier-status.txt').write_text(f'exit_status={result.returncode}\noriginal_source_unchanged={unchanged}\n')
if not unchanged:
    raise SystemExit('PROTOCOL DEVIATION: original source changed during acceptance.')
print(name, 'verifier exit',result.returncode,flush=True)
if result.returncode:
    raise SystemExit(result.returncode)
if not (acceptance/'acceptance.md').exists() or not (acceptance/'acceptance.env').exists():
    raise SystemExit('Verifier left missing reports; inspect session before rerunning.')
print((acceptance/'acceptance.env').read_text(),flush=True)
