#!/bin/sh
# Follow-up 1: presentation without read-back, RUNS times (default 3), each
# started only on a quiet machine (../tools/quiet-run.sh), pinned to CPUs
# 2-7. Windows open on the desktop; keep them visible. Writes
# results/raw-present-<n>.md.
#
#   plans/rendering/vello-spike/present.sh [RUNS]
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
cd "$here"
cargo build --release --target-dir "$repo/target/vello-spike"
for n in $(seq 1 "${1:-3}"); do
  ../tools/quiet-run.sh "results/raw-present-$n.md" \
    taskset -c 2-7 "$repo/target/vello-spike/release/vello-spike" present
done
