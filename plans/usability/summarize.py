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

The sources are <run-dir>/work, or <run-dir>/submission for a kept record.
The code-size section's definitions were frozen for round 2 on 2026-10-27,
before any round 2 run, and apply unchanged to round 1's kept records:

- Code lines: lines of the tester's `*.rs` files (outside `target/`) that
  hold something other than whitespace once comments, doc comments included,
  are removed. A string literal is code.
- Test code: every file with a `tests` or `benches` directory in its path,
  and each item under `#[cfg(test)]` elsewhere. The rest, a headless mode
  included, is application code.
- Task 4: lines are counted only where they differ from the fixture at the
  kit's commit (inserted or replaced lines of a line diff of each file's
  code lines), and imports only when the fixture lacks them.
- Distinct pert-ui types: identifiers in code, outside string literals, that
  name a public struct, enum, trait, type alias or union (by its path, not
  an associated type) of `pert-ui`,
  `pert-ui-runtime`, `pert-ui-core` or `pert-ui-cpu` in the API listings at
  the kit's commit. A candidate list: a tester's own type of the same name
  counts too, so the author checks it.
- Import routes: each distinct `pert_ui::` path written in the source is
  `prelude` (it starts with `prelude::`), `module` (another lowercase first
  segment, named in the report) or `root` (anything else, as `pert_ui::Button`,
  `pert_ui::App::new` or the glob `pert_ui::*`). Glob imports are also listed.
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


source_root = run / "work" if (run / "work").exists() else run / "submission"
sources = sorted(src for src in source_root.rglob("*.rs")
                 if "target" not in src.relative_to(source_root).parts)

imports = set()
for src in sources:
    text = strip_comments_and_strings(src.read_text(errors="replace"))
    for m in re.finditer(r"\buse\s+pert_ui::([^;]+);", text):
        imports.update(p.removesuffix("::self") for p in expand(m[1], ""))
    for m in re.finditer(r"(?<!use )\bpert_ui::((?:\w+::)*\w+)", text):
        imports.add(m[1])

def lex(text):
    """Returns (code, bare): `code` is the text with every comment replaced by
    spaces, `bare` additionally blanks string and character literal contents.
    Both keep every newline, so lines stay aligned with the file."""
    code, bare = list(text), list(text)
    n, i = len(text), 0

    def blank(start, end, *targets):
        for t in targets:
            for k in range(start, end):
                if t[k] != "\n":
                    t[k] = " "

    while i < n:
        c = text[i]
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            blank(i, end, code, bare)
            i = end
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j, code, bare)
            i = j
        elif (m := re.match(r'(?:b|c)?r(#*)"', text[i:])) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            close = '"' + m[1]
            end = text.find(close, i + len(m[0]))
            end = n if end < 0 else end + len(close)
            blank(i + len(m[0]), end - len(close), bare)
            i = end
        elif c == '"' or (c in "bc" and text.startswith('"', i + 1) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_"))):
            j = i + (1 if c == '"' else 2)
            start = j
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            blank(start, min(j, n), bare)
            i = j + 1
        elif c == "'" and (m := re.match(r"'(\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'\n])'", text[i:])):
            blank(i + 1, i + len(m[0]) - 1, bare)
            i += len(m[0])
        else:
            i += 1
    return "".join(code), "".join(bare)


def cfg_test_lines(bare):
    """Line indexes (0-based) covered by items under `#[cfg(test)]`."""
    covered = set()
    for m in re.finditer(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", bare):
        depth, j, opened = 0, m.end(), False
        while j < len(bare):
            ch = bare[j]
            if ch in "([{":
                depth += 1
                opened = opened or ch == "{"
            elif ch in ")]}":
                depth -= 1
                if depth < 0 or (depth == 0 and ch == "}" and opened):
                    break
            elif ch in ";," and depth == 0:
                break
            j += 1
        first = bare.count("\n", 0, m.start())
        last = bare.count("\n", 0, min(j, len(bare) - 1))
        covered.update(range(first, last + 1))
    return covered


def git_show(commit, path):
    """A file at the kit's commit, or None. The checkout holds the history."""
    import subprocess
    result = subprocess.run(["git", "-C", str(pathlib.Path(__file__).resolve().parent),
                             "show", f"{commit}:{path}"], capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else None


def code_lines(text):
    """(index, stripped text) of each code line."""
    code, _ = lex(text)
    return [(k, line.strip()) for k, line in enumerate(code.split("\n")) if line.strip()]


commit = cond.get("commit", "")
task4 = cond.get("task", "").startswith("4-")
project = source_root
if task4 and (source_root / "bookmarks").is_dir():
    project = source_root / "bookmarks"

counts = {"application": 0, "test": 0}
type_names = set()
for package in ("pert-ui", "pert-ui-runtime", "pert-ui-core", "pert-ui-cpu"):
    for config in ("default", "cpu"):
        listing = git_show(commit, f"api/{package}/{config}.txt") or ""
        for m in re.finditer(r"^pub (?:struct|enum|trait|type|union) (pert_ui\w*::(?:\w+::)*)(\w+)", listing, re.M):
            # `Type::Assoc` is an associated type, not a type of its own.
            if not m[1].rstrip(":").split("::")[-1][:1].isupper():
                type_names.add(m[2])
types_used = {"application": set(), "test": set()}
fixture_imports = set()

for src in sources:
    text = src.read_text(errors="replace")
    _, bare = lex(text)
    rel = src.relative_to(project) if src.is_relative_to(project) else src.relative_to(source_root)
    whole_test = bool({"tests", "benches"} & set(rel.parts[:-1]))
    in_cfg_test = cfg_test_lines(bare)
    lines = code_lines(text)
    counted = {k for k, _ in lines}
    if task4:
        fixture = git_show(commit, f"plans/usability/task4-app/{rel.as_posix()}")
        if fixture is not None:
            fixture_text = strip_comments_and_strings(fixture)
            for m in re.finditer(r"\buse\s+pert_ui::([^;]+);", fixture_text):
                fixture_imports.update(p.removesuffix("::self") for p in expand(m[1], ""))
            for m in re.finditer(r"(?<!use )\bpert_ui::((?:\w+::)*\w+)", fixture_text):
                fixture_imports.add(m[1])
            import difflib
            old = [t for _, t in code_lines(fixture)]
            new = [t for _, t in lines]
            counted = set()
            for tag, _, _, j1, j2 in difflib.SequenceMatcher(None, old, new, autojunk=False).get_opcodes():
                if tag in ("insert", "replace"):
                    counted.update(lines[j][0] for j in range(j1, j2))
    for k in counted:
        counts["test" if whole_test or k in in_cfg_test else "application"] += 1
    for k, line in enumerate(bare.split("\n")):
        for name in set(re.findall(r"\b[A-Za-z_]\w*\b", line)) & type_names:
            types_used["test" if whole_test or k in in_cfg_test else "application"].add(name)

routes = collections.Counter()
modules = collections.Counter()
globs = sorted(p for p in imports - fixture_imports if p.endswith("*"))
for path in imports - fixture_imports:
    first = path.split("::")[0]
    if first == "prelude":
        routes["prelude"] += 1
    elif first[:1].islower():
        routes["module"] += 1
        modules[first] += 1
    else:
        routes["root"] += 1
all_types = types_used["application"] | types_used["test"]
code_size_lines = [
    "## Code size and API surface (definitions in summarize.py, frozen for round 2)",
    "",
    f"- Sources: `{source_root.name}`"
    + (", task 4 counted against the fixture at the kit's commit" if task4 else ""),
    f"- Code lines{' added or changed' if task4 else ''}: application {counts['application']}, "
    f"test {counts['test']}, total {counts['application'] + counts['test']}",
    f"- Distinct pert-ui types (candidates): {len(all_types)} "
    f"(application {len(types_used['application'])}, test {len(types_used['test'])})",
    f"- Import routes{' (paths the fixture lacks)' if task4 else ''}: "
    f"prelude {routes['prelude']}, root {routes['root']}, module {routes['module']}"
    + (" (" + ", ".join(f"`{k}` {v}" for k, v in sorted(modules.items())) + ")" if modules else "")
    + "; glob imports: " + (", ".join(f"`pert_ui::{g}`" for g in globs) or "none"),
    "",
    ", ".join(f"`{t}`" for t in sorted(all_types)) or "No pert-ui types.",
    "",
]

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
    *code_size_lines,
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
