"""Run one independent read-only Claude review with isolated configuration.

Usage: python3 claude-review.py <checkout> <run-base> <name> <prompt-file> <credential-source>
The named output directory must not exist. Credentials remain private.
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
if not name.startswith("r3-") or "/" in name or name in (".", ".."):
    raise SystemExit("Expected a single r3- review name")
prompt_path, credential_source = (Path(value).resolve() for value in sys.argv[4:6])
output = base / name
output.mkdir()
config = output / "config"
config.mkdir(mode=0o700)
credentials = config / ".credentials.json"
shutil.copyfile(credential_source, credentials)
credentials.chmod(0o600)
original_hash = hashlib.sha256(credentials.read_bytes()).hexdigest()
prompt = prompt_path.read_text()
(output / "prompt.md").write_text(prompt)
environment = dict(os.environ, CLAUDE_CONFIG_DIR=str(config), DISABLE_AUTOUPDATER="1",
    ENABLE_CLAUDEAI_MCP_SERVERS="false", CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC="1")
command = ["claude", "-p", prompt, "--tools", "Read,Glob,Grep",
    "--add-dir", str(base / "kit-r3-head"), str(base / "kit-r3-baseline"),
    "--dangerously-skip-permissions", "--disable-slash-commands", "--setting-sources", "",
    "--strict-mcp-config", "--mcp-config", '{"mcpServers":{}}',
    "--model", "claude-opus-5-5", "--effort", "high", "--output-format", "stream-json", "--verbose"]
with (output / "transcript.jsonl").open("x") as transcript, (output / "stderr.log").open("x") as stderr:
    result = subprocess.run(command, cwd=root, env=environment, stdout=transcript, stderr=stderr)
if credentials.exists():
    if hashlib.sha256(credentials.read_bytes()).hexdigest() != original_hash:
        credentials.rename(output / "credentials-after-review.json")
        print("Changed credential copy retained privately; host source unchanged.", flush=True)
    else:
        credentials.unlink()
(output / "status.txt").write_text(f"exit_status={result.returncode}\n")
for line in (output / "transcript.jsonl").read_text().splitlines():
    try:
        event = json.loads(line)
    except ValueError:
        continue
    if event.get("type") == "result":
        (output / "result.md").write_text(str(event.get("result", "")) + "\n")
print(name, "review exit", result.returncode, flush=True)
raise SystemExit(result.returncode)
