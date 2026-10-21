#!/usr/bin/env bash
# Lint survey of slice 4 (rubric "Surveys: Lints"). Runs the candidate lints
# as warnings in throwaway invocations (hits.py reads their
# JSON messages); nothing here changes the workspace's
# lint configuration. Writes one hits file per configuration beside this
# script: lint<TAB>file:line<TAB>target kind<TAB>message.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../../.." && pwd)"
out="$root/plans/usability/api-audit/lints/raw"
cd "$root"
mkdir -p "$out"

LINTS=(
    -W missing_debug_implementations
    -W unreachable_pub
    -W clippy::must_use_candidate
    -W clippy::return_self_not_must_use
    -W clippy::missing_panics_doc
    -W clippy::fn_params_excessive_bools
    -W clippy::struct_excessive_bools
)

# survey <name> <cargo clippy arguments...>
survey() {
    local name="$1"
    shift
    # Touching the sources forces clippy to re-lint the selected packages.
    find crates -name '*.rs' -path '*/src/*' -exec touch {} +
    cargo clippy --locked --target-dir "$root/target/lint-survey" "$@" \
        --message-format=json -- "${LINTS[@]}" 2>/dev/null |
        python3 "$root/plans/usability/api-audit/lints/hits.py" | sort >"$out/$name.tsv"
    echo "$name: $(wc -l <"$out/$name.tsv") hits"
}

survey workspace --workspace --all-targets
survey core-serde -p pert-ui-core --features serde --all-targets
survey runtime-serde -p pert-ui-runtime --features serde --all-targets
survey facade-no-default -p pert-ui --no-default-features --all-targets
survey facade-image-formats -p pert-ui --features image-formats --all-targets
survey facade-no-default-png -p pert-ui --no-default-features --features png --all-targets
survey facade-images -p pert-ui --features images --all-targets
survey facade-svg -p pert-ui --features svg --all-targets
survey facade-no-default-svg -p pert-ui --no-default-features --features svg --all-targets
survey facade-serde -p pert-ui --features serde --all-targets
survey facade-no-default-serde -p pert-ui --no-default-features --features serde --all-targets
survey facade-cpu -p pert-ui --features cpu --all-targets
survey facade-no-default-desktop-cpu -p pert-ui --no-default-features --features desktop-cpu --all-targets
