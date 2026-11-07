#!/bin/sh
# Builds the benchmark (bench.sh), then runs it RUNS times (default 1) on a
# quiet machine (../tools/quiet-run.sh), pinned to CPUs 2-7, writing
# results-raw-<n>.md here.
#
#   plans/rendering/cpu-blend-variants/run.sh [RUNS]
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
cd "$here"
BENCH_BUILD_ONLY=1 ./bench.sh
bin=$repo/target/vello-spike/release/cpu-blend-variants-bench
scratch=$repo/target/cpu-blend-variants/scratch/apps/gallery
for n in $(seq 1 "${1:-1}"); do
  (cd "$scratch" && "$here/../tools/quiet-run.sh" "$here/results-raw-$n.md" taskset -c 2-7 "$bin")
done
