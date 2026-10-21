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
443 only, and logs every request with its decision.

**What the proxy does not cover.** The proxy restricts destinations, not
processes. For Codex, its own `workspace-write` sandbox stays on inside
ours and denies tool commands any socket, so only the CLI reaches the proxy;
`/log` and the Cargo home are added as writable directories. Claude Code's
own Linux sandbox needs `socat` beside bubblewrap; `socat` is not installed
here and that sandbox was not tried. Until it is, Claude's tool commands can
reach the model hosts through the proxy (with the run's copy of the
credentials). They cannot reach GitHub, docs.rs or any other host.
Claude runs with WebFetch and WebSearch disallowed and Codex with
`web_search = "disabled"`; `summarize.py` lists every mention of the proxy
or a model host in the transcript for the author to read, and a tester
command that uses them is a protocol violation recorded with the run.

**Clean context.** The tester's home is new for every run and holds only the
Cargo source configuration, the CLI's credential copy and, for Codex, a
config with the maintainer's model, reasoning effort and `web_search =
"disabled"`. The plan proposed canary instructions in the maintainer's
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

Not yet run: `probe` with the Claude and Codex testers (the sessions' own
file, shell and web tools, what they report having loaded, and Codex's
`--add-dir` for `/log` and the Cargo home), and the pilot. Starting a
tester session from the author's Claude Code session was refused by its
auto-mode permission classifier, so the maintainer starts those runs.
