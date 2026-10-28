"""Resume only a verifier interrupted by the documented service usage limit.

Usage: python3 -I resume-acceptance.py <checkout> <base> <run> <credentials>
Preserves its context, copy and first transcript; never replays a blind task.
"""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys

root, base = (Path(value).resolve() for value in sys.argv[1:3])
name = sys.argv[3]
assert name.startswith('r3-') and '/' not in name
source = Path(sys.argv[4]).resolve()
original = base / name
acceptance = base / ('acceptance-' + name)
status = dict(line.split('=', 1) for line in (acceptance / 'verifier-status.txt').read_text().splitlines())
assert status == {'exit_status': '1', 'original_source_unchanged': 'True'}
events = [json.loads(line) for line in (acceptance / 'verifier-transcript.jsonl').read_text().splitlines()]
last = [event for event in events if event.get('type') == 'result'][-1]
assert last.get('api_error_status') == 429 and last.get('api_error') == 'usage_limit_reached'
session = next(event['session_id'] for event in events if event.get('type') == 'system' and event.get('subtype') == 'init')
config = acceptance / 'verifier-config'
assert list((config / 'projects').rglob(session + '.jsonl')), 'Original session state missing'

def manifest(work):
    values = {}
    for parent, dirs, files in os.walk(work, followlinks=False):
        dirs[:] = [directory for directory in dirs if directory not in {'target', '.git', '.claude', '.codex'}]
        for entry in dirs + files:
            path = Path(parent) / entry
            relative = str(path.relative_to(work))
            if path.is_symlink():
                assert path.resolve().is_relative_to(work.resolve()), relative
                values[relative] = 'symlink:' + os.readlink(path)
            elif path.is_file():
                values[relative] = hashlib.sha256(path.read_bytes()).hexdigest()
    return values

before = json.loads((acceptance / 'original-source-manifest.json').read_text())
records = json.loads((acceptance / 'original-record-manifest.json').read_text())
def unchanged():
    return manifest(original / 'work') == before and all(
        (original / relative).is_file() and hashlib.sha256((original / relative).read_bytes()).hexdigest() == digest
        for relative, digest in records.items())
assert unchanged(), 'Original source or records changed since the verifier began'
assert not (acceptance / 'verifier-resume-transcript.jsonl').exists(), 'Resume already attempted; inspect first'
credentials = config / '.credentials.json'
assert not credentials.exists(), 'Unexpected credentials in isolated verifier config'
shutil.copyfile(source, credentials)
credentials.chmod(0o600)
digest = hashlib.sha256(credentials.read_bytes()).hexdigest()
prompt = '''Continue your own independent acceptance verification after the service's usage-limit interruption. This is the same verifier, original session context, acceptance copy, kit and frozen criteria. No blind task is being rerun. Read your existing verifier-prompt.md if needed. Preserve existing verifier tests, command outputs and the unchanged original submission. Do not recopy over or discard existing evidence. Continue the checks that remain; correct only verifier checks when their assumptions are wrong, never the application. Finish acceptance.md and acceptance.env with all required evidence and any missing checks. Every build still goes through the exact harness sandbox. Do not delegate or edit the repository, kits, other runs or original source.'''
(acceptance / 'verifier-resume-prompt.md').write_text(prompt + '\n')
environment = dict(os.environ, CLAUDE_CONFIG_DIR=str(config), DISABLE_AUTOUPDATER='1',
    ENABLE_CLAUDEAI_MCP_SERVERS='false', CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC='1')
command = ['claude', '-p', prompt, '--resume', session, '--tools', 'Read,Write,Edit,Bash,Glob,Grep',
    '--disable-slash-commands', '--setting-sources', '', '--strict-mcp-config', '--mcp-config', '{"mcpServers":{}}',
    '--dangerously-skip-permissions', '--model', 'claude-opus-5-5', '--effort', 'high', '--output-format', 'stream-json', '--verbose']
with (acceptance / 'verifier-resume-transcript.jsonl').open('x') as output, (acceptance / 'verifier-resume-stderr.log').open('x') as error:
    result = subprocess.run(command, cwd=acceptance, env=environment, stdout=output, stderr=error)
if credentials.exists():
    if hashlib.sha256(credentials.read_bytes()).hexdigest() != digest:
        credentials.rename(acceptance / 'credentials-after-resume.json')
    else:
        credentials.unlink()
safe = unchanged()
(acceptance / 'verifier-resume-status.txt').write_text(f'exit_status={result.returncode}\noriginal_source_unchanged={safe}\n')
assert safe, 'Original source or records changed during resumed verification'
if result.returncode:
    raise SystemExit(result.returncode)
assert (acceptance / 'acceptance.md').exists() and (acceptance / 'acceptance.env').exists(), 'Missing final reports'
print(name, 'resumed verifier completed; author review required', flush=True)
