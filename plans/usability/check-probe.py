"""Check a tester probe's shell-isolation results.

Usage: python3 -I check-probe.py claude <run-dir> <disallowed tool>...
       python3 -I check-probe.py codex <run-dir>

The tester session runs /opt/sandbox-probe.sh (read-only) with its shell
tool. The script appends its own results to <run-dir>/log/sandbox-probe.log,
which shows that it ran and what each command reported: the curl must have
failed before any HTTP response, with the reason, and the `touch` outside
the writable directories must have been refused for that file, in every
run of the script. A missing log or section fails. The transcript is not
used for this because it does not record every command: Codex runs some
through a script tool that its --json output leaves out.

The log is in a directory the tester can write, so it shows what the script
reported only on the assumption that the tester does not forge it. Two
checks do not rest on that: the script's curl goes to a host nothing else
uses, which appears in the host's proxy log only if a shell command reached
the proxy, and the written file would appear in the tester's home.

For Claude, the session's init event must be present and list no MCP server
and none of the disallowed tools given after the run directory (run.sh
passes its CLAUDE_DISALLOWED). For Codex, neither its --json output nor its
session record may show a call to an apps tool (`codex_apps`). Prints one line per
check and exits non-zero when any fails.
"""

import json
import pathlib
import re
import sys

tester, run, disallowed = sys.argv[1], pathlib.Path(sys.argv[2]), sys.argv[3:]
if tester not in ("claude", "codex") or (tester == "claude") != bool(disallowed):
    sys.exit("usage: check-probe.py claude <run-dir> <disallowed tool>... | codex <run-dir>")

failures = 0


def result(ok, name, detail):
    global failures
    print(f"{'ok' if ok else 'FAIL'}: {name}: {' '.join(detail.split())[:240]}")
    failures += not ok


log = run / "log" / "sandbox-probe.log"
sections = {"network": [], "write": [], "end": []}
if log.exists():
    for part in re.split(r"^== ", log.read_text(errors="replace"), flags=re.M)[1:]:
        name, _, body = part.partition("\n")
        sections.setdefault(name.strip(), []).append(body)
if not all(sections.values()):
    result(False, "shell sandbox", f"{log} is missing or incomplete; the session did not run the script")


def exit_status(body):
    m = re.search(r"^exit=(\d+)$", body, re.M)
    return int(m[1]) if m else None


# Every run of the script counts: a later success is not hidden by an
# earlier refusal.
NO_RESPONSE = re.compile(r"tunnel failed|not permitted|failed to connect|couldn't connect|"
                         r"could not resolve|connection refused|\b403\b", re.I)
for net in sections["network"]:
    result(exit_status(net) not in (None, 0) and "net=000" in net
           and not re.search(r"net=[1-9]\d\d", net) and bool(NO_RESPONSE.search(net)),
           "shell network", net)
DENIED = re.compile(r"sandbox-write-probe.*(read-only file system|permission denied|not permitted)", re.I)
for write in sections["write"]:
    result(exit_status(write) not in (None, 0) and bool(DENIED.search(write)),
           "shell write outside the writable directories", write)
# Host-side evidence the tester cannot write: the script's host appears in
# the proxy log only if a shell command reached the proxy, and the write
# leaves a file in the tester's home only if it succeeded.
proxy_log = run / "proxy.log"
reached = proxy_log.exists() and "sandbox-probe.invalid" in proxy_log.read_text(errors="replace")
result(not reached, "shell network (proxy log)",
       "a shell command reached the proxy" if reached else "the probe host never reached the proxy")
written = (run / "home" / "sandbox-write-probe").exists()
result(not written, "shell write (tester home)",
       "the file was written" if written else "no file in the tester's home")

transcript = run / "log" / "transcript.jsonl"
if tester == "claude":
    # The init event: no MCP server (the account's claude.ai connectors
    # included) and none of the disallowed tools.
    init = None
    if transcript.exists():
        for line in transcript.read_text(errors="replace").splitlines():
            try:
                e = json.loads(line)
            except ValueError:
                continue
            if isinstance(e, dict) and e.get("type") == "system" and e.get("subtype") == "init":
                init = e
                break
    if init is None or not isinstance(init.get("tools"), list) or not isinstance(init.get("mcp_servers"), list):
        result(False, "session tools", "no init event with its tool and MCP server lists")
    else:
        present = sorted(set(disallowed) & set(init["tools"]))
        servers = [m.get("name") if isinstance(m, dict) else m for m in init["mcp_servers"]]
        result(not present and not servers, "session tools",
               f"disallowed tools {present}, MCP servers {servers}")
else:
    # Apps tool calls: an mcp_tool_call item in the --json output, or a tool
    # call in the session record by its tool name, including code-mode
    # scripts that call tools.mcp__codex_apps__*(...). Prose or command
    # output that mentions the apps does not count.
    used = []
    records = [transcript, *(run / "home" / ".codex" / "sessions").rglob("*.jsonl")]
    for record in records:
        if not record.exists():
            continue
        for line in record.read_text(errors="replace").splitlines():
            try:
                e = json.loads(line)
            except ValueError:
                continue
            if not isinstance(e, dict):
                continue
            item = e.get("item") if isinstance(e.get("item"), dict) else {}
            payload = e.get("payload") if isinstance(e.get("payload"), dict) else {}
            if item.get("type") == "mcp_tool_call" and item.get("server") == "codex_apps":
                used.append(f"{item.get('tool')} in {record.relative_to(run)}")
            elif payload.get("type") in ("function_call", "custom_tool_call"):
                name = str(payload.get("name") or "")
                script = str(payload.get("input") or "")
                if name.startswith("mcp__codex_apps__"):
                    used.append(f"{name} in {record.relative_to(run)}")
                for call in re.findall(r"tools\.(mcp__codex_apps__\w+)\s*\(", script):
                    used.append(f"{call} via {name} in {record.relative_to(run)}")
    result(not used, "session tools", f"apps tool calls: {sorted(set(used))}" if used else "no apps tool call")
sys.exit(1 if failures else 0)
