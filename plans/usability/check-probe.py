"""Check a tester probe's shell-isolation results from its transcript.

Usage: python3 -I check-probe.py claude <transcript.jsonl> <disallowed tool>...
       python3 -I check-probe.py codex <transcript.jsonl>

Pairs every shell command the tester session ran with its completed result,
from Claude's stream-json (tool_use and tool_result) or Codex's --json
(command_execution items), so prose that merely quotes an expected output
counts for nothing. Each check matches the exact command the probe prompt
asks for: the curl to the model host must have failed before any HTTP
response, with the reason, and no other command naming that host may have
succeeded; the `touch` outside the writable directories must have been
refused by the filesystem for that file. A check whose command never ran
fails. For Claude, the session's init event must be present and list no
MCP server and none of the disallowed tools given after the transcript
(run.sh passes its CLAUDE_DISALLOWED). Prints one line per check and exits
non-zero when any fails.
"""

import json
import re
import sys

tester, transcript, disallowed = sys.argv[1], sys.argv[2], sys.argv[3:]
if tester not in ("claude", "codex") or (tester == "claude") != bool(disallowed):
    sys.exit("usage: check-probe.py claude <transcript> <disallowed tool>... | codex <transcript>")

runs = []  # (command, output, failed)
pending = {}
init = None
for line in open(transcript, errors="replace"):
    try:
        e = json.loads(line)
    except ValueError:
        continue
    if not isinstance(e, dict):
        continue
    if e.get("type") == "system" and e.get("subtype") == "init" and init is None:
        init = e
    if e.get("type") == "item.completed":
        item = e.get("item") or {}
        if item.get("type") == "command_execution":
            runs.append((item.get("command", ""), item.get("aggregated_output", ""),
                         item.get("exit_code") != 0))
        continue
    content = (e.get("message") or {}).get("content")
    if not isinstance(content, list):
        continue
    for c in content:
        if not isinstance(c, dict):
            continue
        if c.get("type") == "tool_use" and c.get("name") == "Bash":
            pending[c.get("id")] = (c.get("input") or {}).get("command", "")
        elif c.get("type") == "tool_result" and c.get("tool_use_id") in pending:
            out = c.get("content")
            if isinstance(out, list):
                out = "\n".join(p.get("text", "") for p in out if isinstance(p, dict))
            runs.append((pending.pop(c["tool_use_id"]), str(out or ""), bool(c.get("is_error"))))

failures = 0


def check(name, matches, ok, describe):
    global failures
    if not matches:
        print(f"FAIL: {name}: the tester session never ran it")
        failures += 1
    for command, output, failed in matches:
        summary = " ".join(output.split())[:200]
        if ok(output, failed):
            print(f"ok: {name}: {describe} ({summary})")
        else:
            print(f"FAIL: {name}: `{command}` -> {summary}")
            failures += 1


CURL = "curl -sS -o /dev/null -w net=%{http_code} https://api.anthropic.com/"
TOUCH = "touch /home/tester/sandbox-write-probe"


def plain(command):
    """The command as asked for: Codex records it wrapped in a shell call."""
    m = re.fullmatch(r"(?:/usr)?(?:/bin/)?(?:ba)?sh -l?c (['\"])(.*)\1", command.strip(), re.S)
    return (m[2] if m else command).strip()


NO_RESPONSE = re.compile(r"tunnel failed|not permitted|failed to connect|couldn't connect|"
                         r"could not resolve|connection refused|\b403\b", re.I)
check("shell network",
      [r for r in runs if plain(r[0]) == CURL],
      lambda o, failed: failed and "net=000" in o and not re.search(r"net=[1-9]\d\d", o)
      and NO_RESPONSE.search(o),
      "no HTTP response")
# Any other command naming a model host must not have succeeded either.
for command, output, failed in runs:
    if plain(command) != CURL and "api.anthropic.com" in command and not failed:
        print(f"FAIL: shell network: `{command}` succeeded; read its output: "
              + " ".join(output.split())[:200])
        failures += 1
DENIED = re.compile(r"sandbox-write-probe.*(read-only file system|permission denied|not permitted)", re.I)
check("shell write outside the writable directories",
      [r for r in runs if plain(r[0]) == TOUCH],
      lambda o, failed: failed and DENIED.search(o),
      "refused")
# Claude's init event: no MCP server (the account's claude.ai connectors
# included) and none of the disallowed tools.
if tester == "claude":
    if init is None or not isinstance(init.get("tools"), list) or not isinstance(init.get("mcp_servers"), list):
        print("FAIL: session tools: no init event with its tool and MCP server lists")
        failures += 1
    else:
        present = sorted(set(disallowed) & set(init["tools"]))
        servers = [m.get("name") if isinstance(m, dict) else m for m in init["mcp_servers"]]
        if present or servers:
            print(f"FAIL: session tools: disallowed tools {present}, MCP servers {servers}")
            failures += 1
        else:
            print("ok: session tools: no MCP server and none of the disallowed tools")
sys.exit(1 if failures else 0)
