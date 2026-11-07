#!/bin/sh
# Runs a command only once the machine is quiet: waits (polling every
# minute, at most 30 minutes) until the 1-minute load average is below 2.0,
# then writes the load and the processes with the most CPU (a lifetime
# average, as `ps` reports it) before and after the run, and the highest
# 1-minute load sampled every 10 seconds during it, as HTML comments to the
# output file, with the command's stdout between.
#
#   plans/rendering/tools/quiet-run.sh OUTPUT COMMAND [ARGS...]
set -eu
out=$1
shift
waited=0
while [ "$(awk '{print ($1 < 2.0)}' /proc/loadavg)" != 1 ]; do
  if [ "$waited" -ge 30 ]; then
    echo "machine not quiet after 30 minutes: $(cat /proc/loadavg)" >&2
    exit 3
  fi
  sleep 60
  waited=$((waited + 1))
done
{
  echo "<!-- start $(date -Iseconds), waited ${waited} min, load $(cut -d' ' -f1-3 /proc/loadavg)"
  ps -eo pcpu,comm --sort=-pcpu | head -6
  echo "-->"
} > "$out"
samples=$(mktemp)
(while :; do cut -d' ' -f1 /proc/loadavg >> "$samples"; sleep 10; done) &
sampler=$!
status=0
"$@" >> "$out" || status=$?
kill "$sampler"
{
  echo "<!-- end $(date -Iseconds), load $(cut -d' ' -f1-3 /proc/loadavg), highest during the run $(sort -g "$samples" | tail -1)"
  ps -eo pcpu,comm --sort=-pcpu | head -6
  echo "-->"
} >> "$out"
rm -f "$samples"
exit "$status"
