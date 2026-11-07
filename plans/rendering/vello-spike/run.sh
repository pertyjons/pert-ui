#!/bin/sh
# Builds the spike and runs the workload measurement RUNS times (default 3),
# pinned to CPUs 2-7, writing results/raw-workloads-<n>.md with the load
# average before and after each run. A window opens briefly per run.
#
#   plans/rendering/vello-spike/run.sh [RUNS]
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
cd "$here"
cargo build --release --target-dir "$repo/target/vello-spike"
mkdir -p results
for n in $(seq 1 "${1:-3}"); do
  out=results/raw-workloads-$n.md
  echo "<!-- load average before: $(cut -d' ' -f1-3 /proc/loadavg) -->" > "$out"
  SPIKE_OUT=results taskset -c 2-7 "$repo/target/vello-spike/release/vello-spike" >> "$out"
  echo "<!-- load average after: $(cut -d' ' -f1-3 /proc/loadavg) -->" >> "$out"
done
