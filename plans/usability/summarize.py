"""Summarize one blind-test run.

Usage: python3 -I summarize.py <run-dir>

Reads <run-dir>/conditions.env, the optional <run-dir>/acceptance.env
(`accepted=<ISO time>` and `acceptance=met|partly|not-met`, written by the
author), <run-dir>/log/builds.jsonl with the per-build output files, the
transcript and the tester's sources under <run-dir>/work. Writes
<run-dir>/summary.md and <run-dir>/diagnostics.md (the full text of every
distinct error and warning the summary counts).

Counts are descriptive observations of how the tester worked, not a usability
score: they depend on how often the tester chose to build. The imported-items
list is a candidate inventory for the author to verify.
"""

import collections
import datetime
import json
import pathlib
import re
import sys

run = pathlib.Path(sys.argv[1])


def read_env(path):
    values = {}
    if path.exists():
        for line in path.read_text().splitlines():
            key, sep, value = line.partition("=")
            if sep:
                values.setdefault(key, value)
    return values


cond = read_env(run / "conditions.env")
acceptance = read_env(run / "acceptance.env")


def ts(text):
    return datetime.datetime.fromisoformat(text).timestamp() if text else None


def minutes(seconds):
    return "n/a" if seconds is None else f"{seconds / 60:.1f} min"


# Global cargo options that take a value, so the subcommand is found after them.
VALUE_OPTIONS = {"--config", "-C", "-Z", "--color", "--explain"}
COMPILING = {"build", "b", "check", "c", "run", "r", "test", "t", "clippy", "doc", "d", "bench", "rustc"}


def subcommand(argv):
    args = iter(argv)
    for arg in args:
        if arg.startswith("+"):
            continue
        if arg in VALUE_OPTIONS:
            next(args, None)
            continue
        if arg.startswith("-"):
            continue
        return arg
    return None


def executes_session(argv):
    """A run or test invocation that may execute the tester's code: not
    --no-run, help, a version query or a test listing. A filter matching no
    test still passes, so the result is a candidate for the author to check
    against the run's output."""
    sub = subcommand(argv)
    if sub not in {"run", "r", "test", "t"}:
        return False
    before = argv[:argv.index("--")] if "--" in argv else argv
    if {"--no-run", "--help", "-h", "--version", "-V"} & set(before):
        return False
    if "--" in argv and "--list" in argv[argv.index("--"):]:
        return False
    return True


builds = {}
index = run / "log" / "builds.jsonl"
if index.exists():
    for line in index.read_text().splitlines():
        try:
            e = json.loads(line)
        except json.JSONDecodeError:
            continue
        b = builds.setdefault(e["id"], {"id": e["id"]})
        if e.get("event") == "start":
            b.update(start=e["time"], argv=e["argv"])
        elif e.get("event") == "end":
            b.update(end=e["time"], status=e["status"])
builds = sorted((b for b in builds.values() if "start" in b), key=lambda b: b["start"])
compiling = [b for b in builds if subcommand(b["argv"]) in COMPILING]
incomplete = [b for b in compiling if "end" not in b]


def output(b, stream):
    path = run / "log" / "builds" / f"{b['id']}.{stream}"
    return path.read_text(errors="replace") if path.exists() else ""


DIAG_START = re.compile(r"^(error|warning)(\[(\w+)\])?: (.*)")
NOT_DIAGNOSTIC = re.compile(r"^(aborting due|could not compile|build failed|test failed|unused manifest key|`.*` \(.*\) generated|\d+ warnings? emitted)")
errors, warnings = collections.Counter(), collections.Counter()
diagnostics = collections.OrderedDict()


occurrences = []


def record(level, code, message, text):
    """Keeps every occurrence; they are counted after all builds are read."""
    occurrences.append((level, code, message, text))


for b in compiling:
    # Human-readable diagnostics: a block runs from its header to the next
    # header or blank line followed by a non-indented line.
    lines = output(b, "stderr").splitlines()
    i = 0
    while i < len(lines):
        m = DIAG_START.match(lines[i])
        if not m or NOT_DIAGNOSTIC.match(m[4]):
            i += 1
            continue
        block = [lines[i]]
        i += 1
        while i < len(lines) and not DIAG_START.match(lines[i]):
            if lines[i] == "" and (i + 1 >= len(lines) or not lines[i + 1].startswith(" ")):
                break
            block.append(lines[i])
            i += 1
        record(m[1], m[3], m[4], "\n".join(block).rstrip())
    # JSON diagnostics from --message-format=json.
    for line in output(b, "stdout").splitlines():
        if not line.startswith("{"):
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-message":
            continue
        d = msg["message"]
        if d.get("level") not in ("error", "warning") or NOT_DIAGNOSTIC.match(d.get("message", "")):
            continue
        record(d["level"], (d.get("code") or {}).get("code"), d["message"],
               (d.get("rendered") or d["message"]).rstrip())

# rustc names a warning's lint (`#[warn(...)]`) only at its first occurrence
# in a build, so a message seen with its lint once is counted under it.
lint_of = {}
for level, code, message, text in occurrences:
    lint = re.search(r"#\[warn\(([\w:]+)\)\]", text)
    if lint:
        lint_of.setdefault(message, lint[1])
for level, code, message, text in occurrences:
    if level == "error":
        errors[code or message[:60]] += 1
    else:
        warnings[code or lint_of.get(message) or message[:60]] += 1
    location = re.search(r"--> (\S+)", text)
    key = (level, code, message, location[1] if location else "")
    if key not in diagnostics:
        diagnostics[key] = text

sessions = [b for b in compiling if b.get("status") == 0 and executes_session(b["argv"])]
first_session = min(sessions, key=lambda b: b["end"]) if sessions else None
if first_session is not None:
    stdout = output(first_session, "stdout") + output(first_session, "stderr")
    # One result line per test binary and doctest run; their sum is the count.
    passed = [int(n) for n in re.findall(r"test result: ok\. (\d+) passed", stdout)]
    first_session_note = (f"`cargo {' '.join(first_session['argv'])}`"
                          + (f", {sum(passed)} tests passed" if passed else ""))


def strip_comments_and_strings(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    text = re.sub(r"//[^\n]*", "", text)
    return re.sub(r'"(\\.|[^"\\])*"', '""', text)


def expand(tree, prefix):
    """Expands a use tree like `a::{b, c::{d as e}}` into full paths."""
    tree = tree.strip()
    depth, split = 0, None
    for pos, ch in enumerate(tree):
        if ch == "{":
            if depth == 0 and split is None:
                split = pos
            depth += 1
        elif ch == "}":
            depth -= 1
    if split is None:
        path = re.sub(r"\s+as\s+\w+$", "", tree).strip()
        return [prefix + path] if path else []
    head = tree[:split].strip()
    body = tree[split + 1:tree.rfind("}")]
    parts, depth, start = [], 0, 0
    for pos, ch in enumerate(body):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif ch == "," and depth == 0:
            parts.append(body[start:pos])
            start = pos + 1
    parts.append(body[start:])
    result = []
    for part in parts:
        result += expand(part, prefix + head)
    return result


imports = set()
for src in (run / "work").rglob("*.rs"):
    if "target" in src.parts:
        continue
    text = strip_comments_and_strings(src.read_text(errors="replace"))
    for m in re.finditer(r"\buse\s+pert_ui::([^;]+);", text):
        imports.update(p.removesuffix("::self") for p in expand(m[1], ""))
    for m in re.finditer(r"(?<!use )\bpert_ui::((?:\w+::)*\w+)", text):
        imports.add(m[1])

# Mentions of the model hosts or the proxy in the tester's tool use: any is a
# lead for the author to read, since only the CLI itself should use them.
audit = collections.Counter()
transcript = run / "log" / "transcript.jsonl"
if transcript.exists():
    for line in transcript.read_text(errors="replace").splitlines():
        for needle in ("127.0.0.1:3128", "proxy.sock", "api.anthropic.com", "chatgpt.com", "api.openai.com", "HTTPS_PROXY"):
            if needle in line:
                audit[needle] += 1

# The model the session actually used, when its transcript names one (Claude's
# stream-json init event does); otherwise only the requested model is known.
effective_model = None
if transcript.exists():
    for line in transcript.read_text(errors="replace").splitlines():
        try:
            e = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(e, dict) and isinstance(e.get("model"), str):
            effective_model = e["model"]
            break

brief, ended, accepted = ts(cond.get("brief_delivered")), ts(cond.get("ended")), ts(acceptance.get("accepted"))
lines = [
    f"# Run summary: {run.name}",
    "",
    "Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.",
    "",
    f"- Tester: {cond.get('tester')} ({cond.get('cli', '')}), requested model {cond.get('model', 'CLI default')}, "
    f"effective model {effective_model or 'not in the transcript'}, effort {cond.get('effort', 'CLI default')}",
    f"- Task: {cond.get('task')}; budget {cond.get('budget_minutes')} min; session outcome {cond.get('outcome', 'unknown')} (exit {cond.get('exit_status')})",
    f"- Kit: commit {cond.get('commit', '')[:12]}, version {cond.get('version')}, {cond.get('toolchain')}, third-party cache {cond.get('cache')}",
    f"- Brief to end of session: {minutes(ended - brief if brief and ended else None)}",
    f"- Brief to accepted completion: "
    + (minutes(accepted - brief) if brief and accepted else f"n/a (acceptance {acceptance.get('acceptance', 'not recorded')})"),
    f"- Acceptance: {acceptance.get('acceptance', 'not recorded')}",
    f"- Time to first build: {minutes(compiling[0]['start'] - brief if compiling and brief else None)}",
    f"- Builds: {len(compiling)}; failed {sum(b.get('status', 0) != 0 for b in compiling if 'end' in b)}; "
    f"killed or unfinished {len(incomplete)}; total build time {minutes(sum(b['end'] - b['start'] for b in compiling if 'end' in b))}",
    "- First successful run or test (candidate; the author checks it exercised the app): "
    + ("none" if first_session is None else
       f"{first_session_note}, build {compiling.index(first_session) + 1}, "
       f"{minutes(first_session['end'] - brief) if brief else 'n/a'} after the brief"),
    "",
    "## Errors by code (occurrences across builds)",
    "",
    *(f"- `{k}`: {v}" for k, v in errors.most_common()),
    "",
    "## Warnings by code or lint (occurrences across builds)",
    "",
    *(f"- `{k}`: {v}" for k, v in warnings.most_common()),
    "",
    "## pert_ui paths used (candidate inventory, verify against the source)",
    "",
    ", ".join(f"`{i}`" for i in sorted(imports)) or "none",
    "",
    "## Proxy and model-host mentions in the transcript (read each)",
    "",
    *(f"- `{k}`: {v}" for k, v in audit.most_common()),
    "",
]
(run / "summary.md").write_text("\n".join(lines).rstrip("\n") + "\n")
(run / "diagnostics.md").write_text(
    ("# Distinct diagnostics\n\n"
     + ("\n\n".join(f"```text\n{t}\n```" for t in diagnostics.values()) or "None.")).rstrip("\n") + "\n")
print((run / "summary.md").read_text())
