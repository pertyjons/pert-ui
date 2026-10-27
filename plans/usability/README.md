# Blind-test kit

The harness for [the developer-usability plan](../developer-usability.md).
It is unpackaged and lives only in the repository. Testers never see this
directory; they get a prepared kit, the brief and one task.

| File | Purpose |
|---|---|
| `run.sh` | `prepare` a kit from the checkout, `launch` a run, `probe` the isolation, `exec` a command in the sandbox without credentials, `shell` for debugging |
| `proxy.py`, `bridge.py` | The only network path out of the sandbox: a host-side CONNECT proxy for the model hosts and the in-sandbox bridge to it |
| `cargo-wrapper.py` | The tester's `cargo`, logging every command with timing, status and its stdout and stderr separately |
| `summarize.py` | A run's summary and distinct diagnostics |
| `claude-settings.json` | Claude Code's own sandbox settings for the Claude tester |
| `check-probe.py` | The tester probe's shell network and write checks, read from the log of the script the session runs, and its tool checks |
| `tester-brief.md`, `friction-log.md` | Given to every tester |
| `tasks/` | Tasks 1 to 4, given one per run through the prompt |
| `reserved/` | Task 5, unused until round 2 |
| `task4-app/` | The application task 4 tests, copied into `/work/bookmarks` of task 4's runs only |
| `acceptance.md` | The author's acceptance checks, never shown to testers |
| `author-notes.md` | Author observations made before any run, checked in slice 3's triage |

## Use

```sh
plans/usability/run.sh prepare ~/pert-ui-usability/kit-<commit>
plans/usability/run.sh probe   ~/pert-ui-usability/kit-<commit> ~/pert-ui-usability/probe-<tester>-<n> <claude|codex>
plans/usability/run.sh launch  ~/pert-ui-usability/kit-<commit> ~/pert-ui-usability/r1-<task>-<tester> <claude|codex> 2-task-list.md 90
python3 -I plans/usability/summarize.py ~/pert-ui-usability/r1-<task>-<tester>
```

Kit and run directories must be on disk outside the checkout; `run.sh`
refuses the checkout and tmpfs. A kit is about 1.6 GB (vendored sources and
rendered documentation). Delete a run's `work/target` after its records are
copied.

## Checkpoint decisions (slice 0)

Recorded 2026-10-21 and revised after the checkpoint's Codex review; the
review and the maintainer's approval are recorded in the plan's review
record when they exist.

**Consumer directory.** `prepare` packages the library crates with `cargo
package --no-verify` (the archives only; packaging verification remains the
release gate's job and is not claimed here), vendors their third-party
dependencies at the workspace lockfile's versions with `cargo vendor`, adds
the extracted library packages to the same directory source with their
archive checksums, and resolves the tester's project through it as a
crates.io replacement with `net.offline`. The tester writes `pert-ui =
"<version>"` as a crates.io consumer would. The vendored set covers the
facade's `desktop`, `image-formats`, `svg`, `serde` and `cpu` features, plus
`serde` with `derive`, `serde_json` and `thiserror` for a tester's own file
format and errors. Documentation is rendered from that directory source with
the facade's docs.rs features (`image-formats`, `svg`, `serde`, `cpu`) and
includes every dependency, so cross-crate links resolve offline. `kit.env`
is written last and marks a complete kit.

**Enforced isolation.** Every run is a `bwrap` sandbox with new user, mount,
PID, IPC, UTS, cgroup and network namespaces and a cleared environment that
sets only the variables the run needs. The filesystem holds `/usr`, `/etc`,
the rustup toolchain and Cargo proxies (read-only), the kit at `/kit`
(read-only), the run's `/work`, `/log` and `/home/tester`, and the tester
CLI binary. `/home/per`, the checkout, and the maintainer's `~/.claude` and
`~/.codex` do not exist in it. A run directory must be new or empty, and
kit and run paths are resolved physically before they are checked against
the checkout, so a symlink cannot alias it. The network namespace has only
loopback: `bridge.py` forwards `127.0.0.1:3128` to a Unix socket where
`proxy.py` admits `CONNECT` to the model and authentication hosts on port
443 only, and logs every request with its decision. The tester's command
runs in a nested mount namespace with an empty `/run`, so only the bridge
holds the socket and nothing the tester starts can open it directly. The
bridge's script is mounted at `/run/bridge.py` from the checkout, so the same
empty `/run` hides it from the tester; it is not part of the kit.

**What the proxy does not cover.** The proxy restricts destinations, not
processes, so each CLI's own sandbox stays on inside ours and keeps tool
commands off the network; only the CLI itself reaches the proxy. For Codex
that is its `workspace-write` sandbox, which denies tool commands any
socket; `/log` and the Cargo home are added as writable directories. For
Claude it is Claude Code's Linux sandbox (bubblewrap and `socat`), set by
[`claude-settings.json`](claude-settings.json), which is mounted read-only
and passed with `--settings`: sandboxing on and failing closed if it cannot
start, no unsandboxed retries, strict mode with an empty domain allowlist,
and `/log` and the Cargo home writable beside `/work`. Claude Code's
sandbox covers shell commands only; its file tools run outside it and are
confined by the outer sandbox, which has nothing of the maintainer's to
read. The tester probe has the session run a read-only
`/opt/sandbox-probe.sh`, which requests a host nothing else uses and
touches a file in the tester's home. The host's proxy log must not show
that host and the file must not exist; both are outside what the tester
can write. The script's own log in `/log` shows that it ran and what each
command reported, on the assumption that the tester does not forge it.
(Codex runs some commands through a script tool that its `--json` output
leaves out, so the transcript is not a complete record of commands.)
Claude runs with `--strict-mcp-config` and the account's claude.ai
connectors turned off, and without the web tools or the tools that reach
cloud sessions, other sessions or schedules (`CLAUDE_DISALLOWED` in
`run.sh`); the probe checks the session's tool list and fails if the proxy
log shows a connector attempt. Codex runs with `web_search = "disabled"`
and with its apps and plugins, browser and computer use, image generation
and skill dependency installation turned off (`CODEX_DISABLED` in `run.sh`):
its apps server offers a web tool that runs on OpenAI's side, which no
local sandbox stops. The probe checks those features off with `codex
features list` and fails on any apps tool call in the session record.
`summarize.py` lists every mention of the proxy
or a model host in the transcript for the author to read, and a tester
command that uses them is a protocol violation recorded with the run.

**Clean context.** The tester's home is new for every run and holds only the
Cargo source configuration, the CLI's credential copy and, for Codex, a
config with the maintainer's model, reasoning effort, `web_search =
"disabled"` and the features above turned off. The plan proposed canary instructions in the maintainer's
user-level files; that would change files the maintainer's own sessions
load. Instead the probe shows that those files and Codex's memory database
do not exist inside the sandbox and that the environment holds only the
allowed variables, and the tester session is asked to list what it loaded.

**Credentials.** The tester gets a run-local copy of the CLI's credential
file, never a mount of the host file, and nothing is written back to the
host file automatically: the tester could have changed its copy. When the
copy changed during the run (a token refresh may rotate the credentials),
the launcher moves it to the run's `credentials-after-run.json` and says so;
if the maintainer's own CLI then stops authenticating, the maintainer logs
in again or checks that file and copies it over. Dedicated credentials for
testers would remove this; the maintainer has not provided any. The tester
can read its copy.

**Measurement and conditions.** `conditions.env` records the kit commit,
version, toolchain (`stable`, rustc 1.99.0 at preparation), tester, CLI
version, model and effort (Claude: `opus`, `high` unless overridden; Codex:
the maintainer's config), task, budget, the brief's delivery time, the end
time, exit status and outcome (`timeout` for status 124 or 137). The
wrapper writes a start event before each cargo command and an end event
after it, so a build killed by the timeout stays visible as unfinished.
Completion is the author's acceptance (`acceptance.env`), not the end of
the session. No third-party build cache: every run compiles its
dependencies, so the first build's time includes that. The budget is 90
minutes unless a task says otherwise. Tasks are passed in the prompt and
are not in the kit, so a tester never sees other tasks or the reserved
one, and task 4's application is copied only into task 4's runs. An
interrupted launcher records which signal interrupted it, a CLI killed by a
signal records that signal, and any other nonzero exit is `outcome=error`
with the CLI's stderr in `log/stderr.log`. `summarize.py` reports the
effective model when the transcript names it (Claude's does); for Codex
only the configured model and effort are known, and whether Codex used
them is not independently checked.

## Checkpoint evidence

2026-10-21, Fedora 44, kit from ac69e3b (0.3.51) with the uncommitted
harness, bubblewrap sandbox:

- `prepare` built the kit in about 6 minutes. The vendored registry holds the
  library packages and their dependencies, and the rendered documentation
  includes the library crates and the guide.
- `probe … none` passed: reading a canary file in the checkout, listing the
  checkout and `/home/per`, reading `~/.claude/CLAUDE.md` and Codex's memory
  database, `curl` to GitHub directly, through the proxy and over plain
  HTTP, `curl` to docs.rs and DNS for github.com were all refused; the
  environment held only the allowed variables; `rustc` resolved to the
  rustup toolchain; and a request to `api.anthropic.com` through the proxy
  got an HTTP response.
- `exec` in a fresh project depending on `pert-ui = "0.3.51"` with `cpu`
  resolved offline from the kit, compiled and ran. A failing build, a
  `cargo test --no-run` and a successful `cargo run` were logged and
  summarized as expected (the `--no-run` not counted as an executed
  session), and `cargo metadata` output stayed clean on stdout.
- Codex's sandbox (`codex sandbox -P :workspace`, without credentials)
  works inside ours: a tool command got `Operation not permitted` for a
  socket, and `/kit` was read-only.
- Task 4's application builds offline in the kit with Clippy clean, and a
  throwaway headless test drove all its listed behaviors and a CPU capture
  through the public API.
- After `socat` was installed: bubblewrap nests inside ours, and `claude
  --settings /opt/claude-settings.json sandbox status` inside ours (without
  credentials) reports the sandbox supported and enabled in strict mode;
  without the settings it reports it off. With the proxy socket hidden by
  the nested namespace, a direct connection to it fails, the bridge still
  reaches the model host, and Codex's sandbox (one level deeper) still
  denies a tool command its socket. Before that change a plain process
  could open the socket and get `200 Connection established`; Codex's
  sandbox already refused it.

- The maintainer's first Claude tester probe (2026-10-21) passed every
  check: the canary and checkout absent for the Read tool and the shell,
  GitHub and a search engine refused by Claude's sandbox (`host is not on
  the allow list`), the model host refused to a shell command, the write to
  the tester's home refused (`Read-only file system`), no instruction or
  memory file loaded, the build logged, and the image's color named. It
  also showed the account's claude.ai connectors being tried through the
  proxy (refused) and the cloud and scheduling tools present, which are now
  turned off and checked; and the probe printed no final verdict line,
  which it now does.
- The maintainer's first Codex tester probe (2026-10-21) found a leak:
  with `web_search = "disabled"`, Codex still had its apps server's web
  tool and used it to open the project's GitHub page and search the web.
  The shell refused the network (`failed to open socket`) and the write
  (`Read-only file system`), no instruction file was loaded, the build was
  logged and the image's color named. The curl and touch ran through
  Codex's script tool and were missing from the `--json` output, so the
  transcript-based check reported them as never run; and a concurrent
  Claude probe removed the shared canary file, so the canary check matched
  an empty string. The apps and the other features are now off and
  checked, the shell checks rest on the proxy log and the tester's home
  with the probe script's log as evidence that it ran, every probe has
  its own canary, and the sandboxed command now waits until the bridge
  listens (a command started at once could find no listener).
- `run.sh` was edited while the maintainer's Claude pilot was running.
  Bash reads a script as it goes, so after that run finished it read on
  into the edited file at its old offset. The run's records are written
  before that point; the dispatch is now one function whose call and
  `exit` are parsed before anything runs, so later edits cannot do this.

- The Claude pilot on task 2 (2026-10-21, harness 39d483f) ran end to end:
  the session ended after 8.4 minutes, its submission builds and its nine
  headless tests pass in a fresh acceptance directory, and the author
  recorded every acceptance item as met (`acceptance.md` and
  `acceptance.env` in the run directory). The tester read no library
  source and kept a detailed friction log, for slice 3. `summarize.py`
  worked; it counted `test failed` as an error and split one warning
  between its lint and its message, which are fixed. The task took far
  less than its 90-minute budget; whether the tasks or the budget change is
  a kit decision for after the Codex pilot.

- Both tester probes passed again on the hardened probe (2026-10-21,
  harness 3f2577c): the shell checks from the probe script and the host,
  no apps or disallowed tool, no MCP server, and the Codex session reported
  no web tools.
- The Codex pilot on task 2 (2026-10-21, harness 3f2577c, apps and the
  other features off) ended after 5.1 minutes; its submission builds
  without warnings, its single headless session test passes, and the author
  recorded every acceptance item as met. It built its own row component
  around `DraftEditor` instead of the library's list, selection and filter
  components; that and its friction log go to slice 3. Its friction log's
  times were invented, because the template did not say where to take
  them; the template now asks for `date +%H:%M`, which needs a newly
  prepared kit.

Both pilots finished task 2 in a small part of its 90-minute budget. The
maintainer kept the tasks and the budget (2026-10-21): the budget is a
ceiling, the time to accepted completion is still measured, and the
friction logs were informative. Round 1 prepares a new kit from the final
harness.
Starting a tester session from the author's Claude Code session was refused
by its auto-mode permission classifier, also after the maintainer asked for
it, so the maintainer starts those runs.
