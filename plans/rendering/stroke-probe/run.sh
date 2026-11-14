#!/bin/sh
# Builds the slice 0 stroke prototype (plans/rendering/stroke-spec.md)
# against this checkout and runs its three modes into RESULTS_DIR:
#
#   plans/rendering/stroke-probe/run.sh RESULTS_DIR
#
# check.md and compare.md are deterministic and run once. The timing runs
# RUNS times (default 5), each through plans/rendering/tools/quiet-run.sh
# (QUIET=0 runs without waiting for a quiet machine, for a smoke run only),
# pinned with `taskset` to the core PIN names (default 2, as in phase 1's
# pinned runs), into time-N.md. PROBE_PATH (tiles or quads, default tiles)
# selects the GPU path for check and time, PROBE_TILE (default 16) the tile
# size and PROBE_QUICK=1 shortens every mode, for a smoke run only.
#
# It needs a hardware GPU with Vulkan but no window. The baseline's window
# present is measured separately by the real-time probe (verification.md).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
[ $# -eq 1 ] || { sed -n '2,/^set -eu/p' "$0" | sed '$d; s/^# \{0,1\}//' >&2; exit 2; }
results=$(mkdir -p "$1" && cd "$1" && pwd)
export CARGO_TARGET_DIR="$repo/target/stroke-probe"
(cd "$here" && cargo build --offline --release)
probe=$CARGO_TARGET_DIR/release/stroke-probe
"$probe" check > "$results/check.md"
"$probe" compare > "$results/compare.md"
runs=${RUNS:-5}
pin=${PIN:-2}
n=1
while [ "$n" -le "$runs" ]; do
  if [ "${QUIET:-1}" = 0 ]; then
    taskset -c "$pin" "$probe" time > "$results/time-$n.md"
  else
    "$repo/plans/rendering/tools/quiet-run.sh" "$results/time-$n.md" taskset -c "$pin" "$probe" time
  fi
  n=$((n + 1))
done
