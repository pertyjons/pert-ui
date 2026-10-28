#!/usr/bin/env bash
# Blind developer test harness for plans/archive/developer-usability.md.
#
#   run.sh prepare <kit-dir>
#       Package the checkout's library crates, vendor them with their
#       third-party dependencies as an offline crates.io replacement, and
#       render their documentation. One kit serves every run on one commit.
#   run.sh launch <kit-dir> <run-dir> <claude|codex> <task-file> [minutes]
#       Run one tester on one task inside the sandbox, with a wall-time budget.
#   run.sh probe <kit-dir> <run-dir> <claude|codex|none>
#       The isolation probes: shell checks, then (unless none) a bounded
#       tester session asked to reach the checkout, user-level instructions
#       and the web. Exits nonzero when any check fails.
#   run.sh exec <kit-dir> <run-dir> <command...>
#       One command in the sandbox without a tester CLI or credentials (kit
#       checks, acceptance builds). Initializes <run-dir> when it is new.
#       PERT_UI_USABILITY_EXEC_CLI=claude|codex mounts that CLI's binary,
#       still without credentials, to check how its own sandbox behaves.
#   run.sh shell <kit-dir> <run-dir>
#       An interactive shell in the same sandbox, for debugging the kit.
#
# Environment: PERT_UI_USABILITY_TOOLCHAIN (default stable),
# PERT_UI_USABILITY_CLAUDE_MODEL (default opus) and
# PERT_UI_USABILITY_CLAUDE_EFFORT (default high); Codex uses the model and
# effort of the maintainer's ~/.codex/config.toml. All are recorded per run.
# PERT_UI_USABILITY_CLAUDE_CREDENTIALS optionally selects an explicit private
# credential source instead of the host's default file. Each run still gets
# a copy; changed credentials are never written back to that source.
#
# Kit and run directories must be on disk outside the checkout, never on a
# RAM-backed tmpfs. Inside the sandbox the tester sees only /kit (read-only:
# vendored crates, documentation, brief and template; the task reaches it
# only through the prompt), /work (its project), /log, its own home, the
# toolchain and system files, with a cleared environment. Its network
# namespace has only loopback; a host-side proxy forwards HTTPS to the model
# hosts and nothing else.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
CHECKOUT="$(cd "$HERE/../.." && pwd -P)"
LIB_CRATES=(pert-ui-core pert-ui-runtime pert-ui-cpu pert-ui-wgpu pert-ui)
# Features that pull in every optional dependency of the facade, so a tester
# can choose any supported configuration offline.
VENDOR_FEATURES='"desktop", "image-formats", "svg", "serde", "cpu"'
# The facade's [package.metadata.docs.rs] features.
DOC_FEATURES='"image-formats", "svg", "serde", "cpu"'
TOOLCHAIN="${PERT_UI_USABILITY_TOOLCHAIN:-stable}"
CLAUDE_MODEL="${PERT_UI_USABILITY_CLAUDE_MODEL:-opus}"
CLAUDE_EFFORT="${PERT_UI_USABILITY_CLAUDE_EFFORT:-high}"
# The model hosts the tester CLIs need; everything else is refused.
ALLOW_HOSTS="api.anthropic.com,platform.claude.com,console.anthropic.com,claude.ai,chatgpt.com,auth.openai.com,api.openai.com"
PROBE_MINUTES=10

die() { echo "run.sh: $*" >&2; exit 1; }

# A physical absolute path, resolving symlinks, for containment checks.
canon() { realpath -m -- "$1"; }

outside_checkout() {
    case "$1/" in "$CHECKOUT"/*) die "$1 is inside the checkout" ;; esac
    case "$CHECKOUT/" in "$1"/*) die "$1 contains the checkout" ;; esac
    [ "$(stat -f -c %T "$(dirname "$1")")" = tmpfs ] && die "$1 is on tmpfs"
    return 0
}

version() {
    sed -n 's/^version = "\(.*\)"/\1/p' "$CHECKOUT/Cargo.toml" | head -1
}

prepare() {
    local kit; kit="$(canon "$1")"
    outside_checkout "$kit"
    mkdir -p "$kit"
    # kit.env marks a complete kit; it is written last.
    rm -f "$kit/kit.env"
    local ver; ver="$(version)"
    local prep="$kit/prep"
    # task4-app is removed from kits made by earlier versions of this script:
    # the kit is mounted for every task, and that application is task 4's.
    rm -rf "$prep" "$kit/vendor" "$kit/doc" "$kit/readme" "$kit/bin" "$kit/task4-app"
    mkdir -p "$prep/src" "$kit/vendor" "$kit/readme" "$kit/bin"

    echo "== packaging $ver"
    (cd "$CHECKOUT" && cargo package --locked --workspace \
        --exclude pert-ui-gallery --exclude pert-ui-native-checks \
        --exclude pert-ui-diagnostics --no-verify)
    local extracted="$prep/extracted"
    mkdir -p "$extracted"
    for c in "${LIB_CRATES[@]}"; do
        tar xzf "$CHECKOUT/target/package/$c-$ver.crate" -C "$extracted"
        cp "$extracted/$c-$ver/README.md" "$kit/readme/$c.md"
    done

    echo "== vendoring third-party dependencies"
    {
        echo '[package]'
        echo 'name = "kit-prep"'
        echo 'version = "0.0.0"'
        echo 'edition = "2024"'
        echo 'publish = false'
        echo '[dependencies]'
        echo "pert-ui = { version = \"=$ver\", features = [$VENDOR_FEATURES] }"
        # Common crates a settings file or error type may want.
        echo 'serde = { version = "1", features = ["derive"] }'
        echo 'serde_json = "1"'
        echo 'thiserror = "2"'
        echo '[patch.crates-io]'
        for c in "${LIB_CRATES[@]}"; do
            echo "$c = { path = \"extracted/$c-$ver\" }"
        done
    } >"$prep/Cargo.toml"
    echo 'fn main() {}' >"$prep/src/main.rs"
    cp "$CHECKOUT/Cargo.lock" "$prep/Cargo.lock"
    (cd "$prep" && cargo +"$TOOLCHAIN" update --workspace --quiet &&
        cargo +"$TOOLCHAIN" vendor --versioned-dirs "$kit/vendor" >/dev/null)

    echo "== adding the library packages to the vendored registry"
    for c in "${LIB_CRATES[@]}"; do
        local dir="$kit/vendor/$c-$ver"
        rm -rf "$dir"
        cp -r "$extracted/$c-$ver" "$dir"
        local sum; sum="$(sha256sum "$CHECKOUT/target/package/$c-$ver.crate" | cut -d' ' -f1)"
        printf '{"files":{},"package":"%s"}\n' "$sum" >"$dir/.cargo-checksum.json"
    done

    echo "== rendering documentation"
    # Rebuild the prep crate against the vendored registry only, as a tester
    # will, and render docs.rs's feature set.
    sed -i '/^\[patch.crates-io\]/,$d' "$prep/Cargo.toml"
    sed -i "s/^pert-ui = .*/pert-ui = { version = \"=$ver\", features = [$DOC_FEATURES] }/" "$prep/Cargo.toml"
    mkdir -p "$prep/.cargo"
    cat >"$prep/.cargo/config.toml" <<EOF
[source.crates-io]
replace-with = "vendored"
[source.vendored]
directory = "$kit/vendor"
[net]
offline = true
EOF
    rm -f "$prep/Cargo.lock"
    (cd "$prep" && cargo +"$TOOLCHAIN" doc --quiet --target-dir "$prep/target")
    mv "$prep/target/doc" "$kit/doc"
    rm -rf "$kit/doc/kit_prep" "$prep"

    cp "$HERE/cargo-wrapper.py" "$kit/bin/cargo"
    chmod +x "$kit/bin/cargo"
    cp "$HERE/tester-brief.md" "$HERE/friction-log.md" "$kit/"
    {
        echo "commit=$(git -C "$CHECKOUT" rev-parse HEAD)"
        echo "dirty=$(git -C "$CHECKOUT" status --porcelain -- crates Cargo.toml Cargo.lock plans/usability | wc -l)"
        echo "version=$ver"
        echo "toolchain=$(rustc +"$TOOLCHAIN" --version)"
        echo "prepared=$(date -Iseconds)"
    } >"$kit/kit.env.tmp"
    mv "$kit/kit.env.tmp" "$kit/kit.env"
    echo "== kit ready in $kit"
    cat "$kit/kit.env"
}

# Sets KIT to the canonical path of a complete kit.
use_kit() {
    KIT="$(canon "$1")"
    [ -f "$KIT/kit.env" ] || die "$1 is not a complete kit"
}

# Creates a new run directory and sets RUN. Refuses an existing non-empty
# directory, so nothing in it can predate the run.
init_run() {
    local tester="$2"
    RUN="$(canon "$1")"
    outside_checkout "$RUN"
    case "$RUN/" in "$KIT"/*) die "run is inside the kit" ;; esac
    case "$KIT/" in "$RUN"/*) die "run contains the kit" ;; esac
    if [ -e "$RUN" ] && [ -n "$(ls -A "$RUN")" ]; then
        die "$RUN exists and is not empty"
    fi
    mkdir -p "$RUN/work" "$RUN/home/.cargo" "$RUN/log/builds"
    cat >"$RUN/home/.cargo/config.toml" <<'EOF'
[source.crates-io]
replace-with = "vendored"
[source.vendored]
directory = "/kit/vendor"
[net]
offline = true
EOF
    case "$tester" in
        claude) mkdir -p "$RUN/home/.claude" ;;
        codex)
            mkdir -p "$RUN/home/.codex"
            {
                grep -E '^(model|model_reasoning_effort) =' "$HOME/.codex/config.toml" || true
                echo 'web_search = "disabled"'
                echo '[features]'
                for feature in $CODEX_DISABLED; do echo "$feature = false"; done
            } >"$RUN/home/.codex/config.toml"
            ;;
        none) ;;
        *) die "unknown tester $tester" ;;
    esac
}

# The tester's credential file: a run-local copy, never the host file. After
# the run the copy is moved out of the tester's home and kept for the
# maintainer to reconcile explicitly; nothing is written to the host file.
host_credentials() {
    case "$1" in
        claude) echo "${PERT_UI_USABILITY_CLAUDE_CREDENTIALS:-$HOME/.claude/.credentials.json}" ;;
        codex) echo "$HOME/.codex/auth.json" ;;
    esac
}
run_credentials() {
    case "$1" in
        claude) echo "$RUN/home/.claude/.credentials.json" ;;
        codex) echo "$RUN/home/.codex/auth.json" ;;
    esac
}

copy_credentials() {
    local tester="$1"
    [ "$tester" = none ] && return
    local host run_copy
    host="$(host_credentials "$tester")"; run_copy="$(run_credentials "$tester")"
    install -m 600 "$host" "$run_copy"
    sha256sum "$host" | cut -d' ' -f1 >"$RUN/.host-credentials.sha256"
}

reconcile_credentials() {
    local tester="$1"
    [ "$tester" = none ] && return
    local host run_copy
    host="$(host_credentials "$tester")"; run_copy="$(run_credentials "$tester")"
    [ -f "$RUN/.host-credentials.sha256" ] || return 0
    rm -f "$RUN/.host-credentials.sha256"
    [ -f "$run_copy" ] || return 0
    if cmp -s "$host" "$run_copy"; then
        rm -f "$run_copy"
    else
        # A token refresh during the run may have rotated the credentials.
        # The tester could also have changed the file, so it is never copied
        # back automatically.
        mv "$run_copy" "$RUN/credentials-after-run.json"
        chmod 600 "$RUN/credentials-after-run.json"
        echo "credentials: the run changed its $tester credential copy; it is in" \
            "$RUN/credentials-after-run.json. If $host stops working, log in again" \
            "or, after checking it, copy that file over it." >&2
    fi
}

start_proxy() {
    rm -f "$RUN/proxy.sock"
    python3 -I "$HERE/proxy.py" "$RUN/proxy.sock" "$ALLOW_HOSTS" "$RUN/proxy.log" &
    PROXY_PID=$!
    for _ in $(seq 50); do [ -S "$RUN/proxy.sock" ] && return; sleep 0.1; done
    die "proxy did not start"
}

CLEANUP_TESTER=none
cleanup() {
    [ -n "${PROXY_PID:-}" ] && kill "$PROXY_PID" 2>/dev/null || true
    [ -n "${CANARY:-}" ] && rm -f "$CANARY"
    [ -n "${RUN:-}" ] && reconcile_credentials "$CLEANUP_TESTER"
    return 0
}
trap cleanup EXIT

# Fills SANDBOX with the bwrap arguments for tester $1 and the current RUN.
sandbox_args() {
    local tester="$1"
    local rustup="${RUSTUP_HOME:-$HOME/.rustup}" cargo_home="${CARGO_HOME:-$HOME/.cargo}"
    SANDBOX=(
        --die-with-parent --unshare-all --new-session --clearenv
        --ro-bind /usr /usr --symlink usr/lib /lib --symlink usr/lib64 /lib64
        --symlink usr/bin /bin --symlink usr/sbin /sbin
        --ro-bind /etc /etc --proc /proc --dev /dev --tmpfs /tmp
        --ro-bind "$rustup" /opt/rustup
        --ro-bind "$cargo_home/bin" /opt/cargo-bin
        --ro-bind "$KIT" /kit
        --bind "$RUN/work" /work
        --bind "$RUN/home" /home/tester
        --bind "$RUN/log" /log
        --ro-bind "$RUN/proxy.sock" /run/proxy.sock
        --ro-bind "$HERE/bridge.py" /run/bridge.py
        --setenv HOME /home/tester --setenv USER tester --setenv LOGNAME tester
        --setenv LANG C.UTF-8 --setenv TERM dumb
        --setenv RUSTUP_HOME /opt/rustup --setenv CARGO_HOME /home/tester/.cargo
        --setenv RUSTUP_TOOLCHAIN "$TOOLCHAIN"
        --setenv PATH /kit/bin:/opt/cargo-bin:/opt/cli:/usr/bin
        --setenv HTTPS_PROXY http://127.0.0.1:3128 --setenv HTTP_PROXY http://127.0.0.1:3128
        --setenv https_proxy http://127.0.0.1:3128 --setenv http_proxy http://127.0.0.1:3128
        --setenv NO_PROXY localhost,127.0.0.1
        --chdir /work
    )
    if [ -f "$RUN/sandbox-probe.sh" ]; then
        SANDBOX+=(--ro-bind "$RUN/sandbox-probe.sh" /opt/sandbox-probe.sh)
    fi
    case "$tester" in
        claude)
            SANDBOX+=(--ro-bind "$(readlink -f "$(command -v claude)")" /opt/cli/claude
                      --ro-bind "$HERE/claude-settings.json" /opt/claude-settings.json
                      --setenv DISABLE_AUTOUPDATER 1
                      --setenv CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC 1
                      --setenv ENABLE_CLAUDEAI_MCP_SERVERS false
                      --setenv CLAUDE_CODE_DISABLE_CRON 1)
            ;;
        codex)
            local codex_dir; codex_dir="$(dirname "$(readlink -f "$(command -v codex)")")"
            SANDBOX+=(--ro-bind "$codex_dir" /opt/codex
                      --symlink /opt/codex/codex /opt/cli/codex
                      --setenv CODEX_HOME /home/tester/.codex)
            ;;
    esac
}

# Runs "$@" in the sandbox for tester $1, with an optional `timeout` prefix in
# TIMEOUT. The bridge forwards 127.0.0.1:3128 to the host proxy. The command
# itself runs in a nested mount namespace where /run is empty, so neither the
# CLI nor its tool commands can open the proxy socket directly or read the
# bridge's script, which shows how the sandbox is built: only the
# bridge's loopback port leads out, and a CLI's own sandbox gives its tool
# commands a network namespace without it. The command starts once the bridge
# listens (127.0.0.1:3128 in /proc/net/tcp), so its first request does not
# race the bridge.
in_sandbox() {
    local tester="$1"; shift
    sandbox_args "$tester"
    ${TIMEOUT:-} bwrap "${SANDBOX[@]}" /bin/bash -c \
        'python3 -I /run/bridge.py &
         for _ in $(seq 100); do
             grep -q " 0100007F:0C38 00000000:0000 0A " /proc/net/tcp && break
             sleep 0.05
         done
         exec bwrap --dev-bind / / --tmpfs /run --die-with-parent -- "$@"' bridge "$@"
}

# Claude tools the tester does without: the web tools, and those that reach
# cloud sessions, other sessions or schedules outside the sandbox. The probe
# checks that the session's tool list holds none of them.
CLAUDE_DISALLOWED="WebFetch WebSearch RemoteTrigger Workflow ListAgents SendMessage CronCreate CronDelete CronList ScheduleWakeup"

# Codex features the tester does without: apps and plugins (whose
# server-side tools include web access), browser and computer use, image
# generation and installing skill dependencies. The probe checks them off.
CODEX_DISABLED="apps plugins remote_plugin browser_use browser_use_external in_app_browser computer_use image_generation skill_mcp_dependency_install tool_suggest"

# Fills CMD with the non-interactive tester command for prompt $2.
tester_cmd() {
    case "$1" in
        claude)
            # Claude Code's own sandbox (claude-settings.json, read-only
            # here) stays on inside ours: it denies shell commands any
            # network, so only the CLI reaches the proxy.
            CMD=(claude -p "$2" --dangerously-skip-permissions
                 --settings /opt/claude-settings.json
                 --model "$CLAUDE_MODEL" --effort "$CLAUDE_EFFORT"
                 --strict-mcp-config
                 --disallowedTools $CLAUDE_DISALLOWED
                 --output-format stream-json --verbose) ;;
        codex)
            # Codex's own sandbox stays on inside ours: it denies tool
            # commands any network, so only the CLI reaches the proxy.
            CMD=(codex exec --sandbox workspace-write
                 --add-dir /log --add-dir /home/tester/.cargo
                 --skip-git-repo-check -C /work --json
                 -o /log/final-message.md "$2") ;;
    esac
}

record_conditions() {
    local tester="$1" task="$2" minutes="$3"
    {
        cat "$KIT/kit.env"
        echo "tester=$tester"
        echo "task=$(basename "$task")"
        echo "budget_minutes=$minutes"
        echo "cache=none"
        case "$tester" in
            claude)
                echo "cli=$(claude --version)"
                echo "model=$CLAUDE_MODEL"
                echo "effort=$CLAUDE_EFFORT"
                if [ -n "${PERT_UI_USABILITY_CLAUDE_CREDENTIALS:-}" ]; then
                    echo "credential_source=explicit-copy"
                else
                    echo "credential_source=host-default"
                fi
                ;;
            codex)
                echo "cli=$(codex --version)"
                sed -n 's/^model = "\(.*\)"/model=\1/p; s/^model_reasoning_effort = "\(.*\)"/effort=\1/p' \
                    "$RUN/home/.codex/config.toml"
                ;;
        esac
    } >"$RUN/conditions.env"
}

launch() {
    use_kit "$1"
    local tester="$3" task="$4" minutes="${5:-90}"
    [ "$tester" = none ] && die "launch needs a tester"
    [ -f "$task" ] || task="$HERE/tasks/$task"
    [ -f "$task" ] || die "no task $4"
    init_run "$2" "$tester"
    # Task 4's application is given only to task 4's runs, in /work.
    case "$(basename "$task")" in
        4-*)
            cp -r "$HERE/task4-app" "$RUN/work/bookmarks"
            rm -rf "$RUN/work/bookmarks/target" "$RUN/work/bookmarks/Cargo.lock"
            # The application requires the kit's version, as an application
            # written against that release would.
            local ver; ver="$(sed -n 's/^version=//p' "$KIT/kit.env")"
            sed -i "s/^pert-ui = { version = \"[^\"]*\"/pert-ui = { version = \"$ver\"/" \
                "$RUN/work/bookmarks/Cargo.toml"
            grep -q "^pert-ui = { version = \"$ver\"" "$RUN/work/bookmarks/Cargo.toml" \
                || die "could not set task 4's pert-ui requirement to $ver"
            ;;
    esac
    record_conditions "$tester" "$task" "$minutes"
    local prompt
    prompt="$(cat "$KIT/tester-brief.md"; echo; echo '---'; echo; cat "$task")"
    copy_credentials "$tester"; CLEANUP_TESTER="$tester"
    start_proxy
    echo "brief_delivered=$(date -Iseconds)" >>"$RUN/conditions.env"
    for sig in INT TERM HUP; do
        # shellcheck disable=SC2064
        trap "printf 'ended=%s\nexit_status=interrupted\noutcome=interrupted by SIG$sig\n' \"\$(date -Iseconds)\" >>\"\$RUN/conditions.env\"; exit 130" "$sig"
    done
    local status=0
    tester_cmd "$tester" "$prompt"
    TIMEOUT="timeout --kill-after=30s ${minutes}m" in_sandbox "$tester" "${CMD[@]}" \
        >"$RUN/log/transcript.jsonl" 2>"$RUN/log/stderr.log" || status=$?
    {
        echo "ended=$(date -Iseconds)"
        echo "exit_status=$status"
        case "$status" in
            0) echo "outcome=ended" ;;
            124 | 137) echo "outcome=timeout" ;;
            1[3-9][0-9] | 2[0-5][0-9]) echo "outcome=killed by signal $((status - 128)) (see log/stderr.log)" ;;
            *) echo "outcome=error (see log/stderr.log)" ;;
        esac
    } >>"$RUN/conditions.env"
    echo "run finished with status $status: $RUN"
}

probe() {
    use_kit "$1"
    local tester="$3"
    init_run "$2" "$tester"
    # One canary per probe, so concurrent probes do not remove each other's.
    # The name and the content are independent: the prompt names the file.
    local canary_text
    canary_text="canary-$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')"
    CANARY="$CHECKOUT/target/usability-canary-$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')"
    mkdir -p "$CHECKOUT/target"
    echo "$canary_text" >"$CANARY"
    copy_credentials "$tester"; CLEANUP_TESTER="$tester"
    # A solid-color image the session must view and name, for PNG viewing.
    local colors=(red green blue yellow) color
    color="${colors[RANDOM % 4]}"
    python3 -I - "$RUN/work/probe.png" "$color" <<'PY'
import struct, sys, zlib
rgb = {"red": (220, 30, 30), "green": (30, 180, 30), "blue": (30, 60, 220), "yellow": (240, 220, 20)}[sys.argv[2]]
w = h = 64
raw = b"".join(b"\0" + bytes(rgb) * w for _ in range(h))
def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")
open(sys.argv[1], "wb").write(png)
PY
    # Mounted read-only at /opt/sandbox-probe.sh. The host it requests is
    # one that only this script uses, so the proxy log shows whether a shell
    # command reached the proxy.
    cat >"$RUN/sandbox-probe.sh" <<'EOF'
# Records what the tester's shell sandbox allows, in /log/sandbox-probe.log.
exec > >(tee -a /log/sandbox-probe.log) 2>&1
echo "== network"
curl -sS --max-time 20 -o /dev/null -w 'net=%{http_code}\n' https://sandbox-probe.invalid/
echo "exit=$?"
echo "== write"
touch /home/tester/sandbox-write-probe
echo "exit=$?"
echo "== end"
EOF
    start_proxy
    local failed=0
    echo "== shell probes"
    in_sandbox "$tester" bash -c "
        failures=0
        refused() { if eval \"\$1\" >/dev/null 2>&1; then echo \"FAIL reachable: \$1\"; failures=\$((failures+1)); else echo \"ok refused: \$1\"; fi; }
        works() { if out=\$(eval \"\$1\" 2>&1); then echo \"ok: \$1 -> \$out\"; else echo \"FAIL: \$1 -> \$out\"; failures=\$((failures+1)); fi; }
        refused 'cat $CANARY'
        refused 'ls $CHECKOUT'
        refused 'ls /home/per'
        refused 'cat $HOME/.claude/CLAUDE.md'
        refused 'ls $HOME/.codex/memories_1.sqlite'
        refused 'curl -sS --max-time 10 --noproxy \"*\" https://github.com/pertyjons/pert-ui'
        refused 'curl -sSf --max-time 10 https://github.com/pertyjons/pert-ui'
        refused 'curl -sSf --max-time 10 http://github.com/'
        refused 'curl -sSf --max-time 10 https://docs.rs/'
        refused 'getent hosts github.com'
        refused 'test -e /run/proxy.sock'
        works 'env | grep -v -E \"^(HOME|USER|LOGNAME|LANG|TERM|PATH|PWD|SHLVL|_|OLDPWD|RUSTUP_HOME|RUSTUP_TOOLCHAIN|CARGO_HOME|HTTPS?_PROXY|https?_proxy|NO_PROXY|CODEX_HOME|DISABLE_AUTOUPDATER|CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC|ENABLE_CLAUDEAI_MCP_SERVERS|CLAUDE_CODE_DISABLE_CRON)=\" | wc -l | grep -qx 0 && echo only-allowed-variables'
        works 'command -v rustc | grep -qx /opt/cargo-bin/rustc && rustc --version'
        works 'test -f /kit/doc/pert_ui/index.html && echo docs'
        works 'curl -sS --max-time 20 -o /dev/null -w %{http_code} https://api.anthropic.com/'
        exit \$failures
    " | tee "$RUN/log/shell-probes.txt" || failed=1
    [ "$tester" = none ] && { [ "$failed" = 0 ] && echo "== shell probes passed"; return "$failed"; }
    local prompt
    prompt="This is an isolation test. Do not do anything else. Try each of the following with every tool you have (file reading tools, shell commands, web tools), and report for each one exactly what you got or the error: (1) read the file $CANARY; (2) list the directory $CHECKOUT; (3) fetch https://github.com/pertyjons/pert-ui; (4) search the web for 'pert-ui rust gui'; (5) list every instruction, memory or configuration file you loaded at startup and quote any user-level or project instructions you were given; (6) with your shell tool only, run 'cargo --version' and then 'bash /opt/sandbox-probe.sh', report their output verbatim and do not retry anything the script reports as refused; (7) in /work run 'cargo init --name probe' and then 'cargo build', and report whether the build succeeded; (8) look at the image /work/probe.png with your image-viewing ability (not by decoding it with code) and name its single color in one word. Then stop."
    if [ "$tester" = codex ]; then
        local features feature
        features="$(in_sandbox codex codex features list 2>&1)" || true
        for feature in $CODEX_DISABLED; do
            if grep -qE "^$feature +.* false\$" <<<"$features"; then
                echo "ok: Codex feature $feature is off"
            else
                echo "FAIL: Codex feature $feature is not off"; failed=1
            fi
        done
    fi
    tester_cmd "$tester" "$prompt"
    TIMEOUT="timeout --kill-after=30s ${PROBE_MINUTES}m" in_sandbox "$tester" "${CMD[@]}" \
        >"$RUN/log/transcript.jsonl" 2>"$RUN/log/stderr.log" || {
        echo "FAIL: the tester session exited with $? (see log/stderr.log)"; failed=1; }
    if grep -rqF "$canary_text" "$RUN/log/transcript.jsonl" "$RUN/log/final-message.md" \
        "$RUN/home/.codex/sessions" 2>/dev/null; then
        echo "FAIL: the canary's content appears in the tester session"; failed=1
    else
        echo "ok: the canary's content does not appear in the tester session"
    fi
    # Shell commands must get no network and no writes outside /work, /log
    # and the Cargo home: the CLI's own sandbox, inside ours, enforces this.
    # The tester runs /opt/sandbox-probe.sh, which logs its own results.
    local disallowed=()
    [ "$tester" = claude ] && read -ra disallowed <<<"$CLAUDE_DISALLOWED"
    python3 -I "$HERE/check-probe.py" "$tester" "$RUN" "${disallowed[@]}" || failed=1
    if python3 -I -c '
import json, sys
events = [json.loads(line) for line in open(sys.argv[1])]
built = {e["id"] for e in events if e.get("event") == "start" and "build" in e["argv"]
         and not {"--help", "-h"} & set(e["argv"])}
ok = [e["id"] for e in events if e.get("event") == "end" and e["id"] in built and e["status"] == 0]
compiled = any("Finished" in open(f"{sys.argv[2]}/{i}.stderr", errors="replace").read() for i in ok)
sys.exit(0 if compiled else 1)
' "$RUN/log/builds.jsonl" "$RUN/log/builds" 2>/dev/null; then
        echo "ok: the tester's cargo build succeeded and was logged"
    else
        echo "FAIL: no successful logged cargo build from the tester session"; failed=1
    fi
    if grep -qiw "$color" "$RUN/log/final-message.md" 2>/dev/null ||
        python3 -I -c '
import json, sys
for line in open(sys.argv[1], errors="replace"):
    try:
        e = json.loads(line)
    except ValueError:
        continue
    if e.get("type") == "result" and sys.argv[2] in str(e.get("result", "")).lower():
        sys.exit(0)
sys.exit(1)
' "$RUN/log/transcript.jsonl" "$color"; then
        echo "ok: the session named the image's color ($color)"
    else
        echo "FAIL: the session did not name the image's color ($color)"; failed=1
    fi
    # The account's claude.ai connectors load after the init event, so their
    # attempts show only in the proxy log.
    if grep -q 'mcp-proxy' "$RUN/proxy.log" 2>/dev/null; then
        echo "FAIL: the session tried to reach the account's claude.ai connectors"; failed=1
    fi
    echo "== proxy log"; cat "$RUN/proxy.log" 2>/dev/null || true
    echo "== read $RUN/log for the session's own report"
    if [ "$failed" = 0 ]; then echo "== probe passed"; else echo "== probe FAILED"; fi
    return "$failed"
}

exec_in() {
    use_kit "$1"
    local run; run="$(canon "$2")"; shift 2
    if [ -e "$run/log/builds" ] && [ -d "$run/home/.cargo" ]; then
        RUN="$run"
    else
        init_run "$run" none
    fi
    start_proxy
    in_sandbox "${PERT_UI_USABILITY_EXEC_CLI:-none}" "$@"
}

shell() {
    use_kit "$1"
    init_run "$2" none
    start_proxy
    in_sandbox none bash -i
}

# The dispatch and the exit are parsed before anything runs, so editing this
# file while a run is active cannot make bash read on into the new text.
main() {
    cmd="${1:-}"; shift || true
    case "$cmd" in
        prepare) [ $# = 1 ] || die "usage: run.sh prepare <kit-dir>"; prepare "$@" ;;
        launch) [ $# -ge 4 ] || die "usage: run.sh launch <kit> <run> <tester> <task> [minutes]"; launch "$@" ;;
        probe) [ $# = 3 ] || die "usage: run.sh probe <kit> <run> <tester>"; probe "$@" ;;
        exec) [ $# -ge 3 ] || die "usage: run.sh exec <kit> <run> <command...>"; exec_in "$@" ;;
        shell) [ $# = 2 ] || die "usage: run.sh shell <kit> <run>"; shell "$@" ;;
        *) die "usage: run.sh prepare|launch|probe|exec|shell ..." ;;
    esac
}
main "$@"; exit
