# Blind-test kit

A harness for blind usability tests: AI testers build small applications
from the packaged documentation in a sandbox with no network beyond the
model hosts and no access to the repository. Three rounds ran from
2026-10-21 to 2026-10-29 (see [verification](../../docs/verification.md#usability-studies));
their tasks, acceptance items and run records are in Git history (this
directory at commit 8254019). It is unpackaged and lives only in the
repository. Testers never see this directory; they get a prepared kit, the
brief and one task.

| File | Purpose |
|---|---|
| `run.sh` | `prepare` a kit from the checkout, `launch` a run, `probe` the isolation, `exec` a command in the sandbox without credentials, `shell` for debugging |
| `proxy.py`, `bridge.py` | The only network path out of the sandbox: a host-side CONNECT proxy for the model hosts and the in-sandbox bridge to it |
| `cargo-wrapper.py` | The tester's `cargo`, logging every command with timing, status and its stdout and stderr separately |
| `summarize.py` | A run's summary and distinct diagnostics |
| `claude-settings.json` | Claude Code's own sandbox settings for the Claude tester |
| `check-probe.py` | The tester probe's shell network and write checks, read from the log of the script the session runs, and its tool checks |
| `tester-brief.md`, `friction-log.md` | Given to every tester |
| `acceptance.md` | The author's acceptance procedure and each task's acceptance items, never shown to testers |

## Before a new round

The earlier tasks are known and the library was repaired against them, so
a new round writes new tasks into a `tasks/` directory beside this file and
their acceptance items into `acceptance.md` before any run. Prepare a kit
from the commit under test, and run `probe` for each tester and confirm
every check passes before the first launch: the tester CLIs change between
versions, and the first probes found leaks (connector attempts, a
server-side web tool) that are now turned off and checked. The maintainer
starts tester sessions; starting one from an agent session was refused by
its permission classifier. Do not edit `run.sh` while a run is in progress.

## Use

```sh
plans/usability/run.sh prepare ~/pert-ui-usability/kit-<commit>
plans/usability/run.sh probe   ~/pert-ui-usability/kit-<commit> ~/pert-ui-usability/probe-<tester>-<n> <claude|codex>
plans/usability/run.sh launch  ~/pert-ui-usability/kit-<commit> ~/pert-ui-usability/r<round>-<task>-<tester> <claude|codex> <task>.md 90
python3 -I plans/usability/summarize.py ~/pert-ui-usability/r<round>-<task>-<tester>
```

A task is passed by path or by its name under `tasks/`. `summarize.py`
rewrites a run's `summary.md` and reads the API listings at the kit's
commit from this checkout's history.

Kit and run directories must be on disk outside the checkout; `run.sh`
refuses the checkout and tmpfs. A kit is about 1.6 GB (vendored sources and
rendered documentation). Delete a run's `work/target` after its records are
copied.

## Design

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
"disabled"` and the features above turned off. Rather than placing canary instructions in the maintainer's
user-level files, which the maintainer's own sessions load, the probe shows that those files and Codex's memory database
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
version, toolchain, tester, CLI
version, model and effort (Claude: `opus`, `high` unless overridden; Codex:
the maintainer's config), task, budget, the brief's delivery time, the end
time, exit status and outcome (`timeout` for status 124 or 137). The
wrapper writes a start event before each cargo command and an end event
after it, so a build killed by the timeout stays visible as unfinished.
Completion is the author's acceptance (`acceptance.env`), not the end of
the session. No third-party build cache: every run compiles its
dependencies, so the first build's time includes that. The budget is 90
minutes unless a task says otherwise. Tasks are passed in the prompt and
are not in the kit, so a tester never sees other tasks. An
interrupted launcher records which signal interrupted it, a CLI killed by a
signal records that signal, and any other nonzero exit is `outcome=error`
with the CLI's stderr in `log/stderr.log`. `summarize.py` reports the
effective model when the transcript names it (Claude's does); for Codex
only the configured model and effort are known, and whether Codex used
them is not independently checked.
