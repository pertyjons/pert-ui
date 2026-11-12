#!/bin/sh
# Builds the phase 1 probe (probe.rs) against one or more checkouts and runs
# them in alternating rounds. Slice 0 runs the checkout before phase 1, and
# slice 6 runs the same unchanged probe against the checkouts before and
# after (plans/rendering/phase1-plan.md). It needs an unlocked desktop
# session with a hardware GPU: a window opens for the wgpu renderer, which
# then renders offscreen. PROBE_GPU=0 leaves the GPU out and needs no window.
#
#   plans/rendering/phase1-probe/run.sh RESULTS_DIR LABEL=REV [LABEL=REV ...]
#
# Each REV is exported with `git archive` into
# target/phase1-probe/checkouts/LABEL. A scratch crate per checkout takes the
# gallery's sources (with an adapter appended to its transforms page, for
# the zoom workload's private cards), `live_mixer.rs` as the module `mixer` and the diagram
# editor as the module `diagram` from that checkout, each example with its
# `main` removed and an adapter from this folder appended; the diagram
# editor's `crate::` paths become `crate::diagram::`. The harness and the
# adapters are this folder's files, the same for every checkout. The crate
# depends on the checkout by path with the `png`, `svg` and `cpu` features,
# uses the checkout's Cargo.lock and has its own target directory.
#
# All checkouts are built first. Then RUNS rounds (default 5) run every
# checkout once each, in the order given, every run through
# plans/rendering/tools/quiet-run.sh (QUIET=0 runs without waiting for a
# quiet machine, for a smoke run only), pinned with `taskset` to the core
# PIN names (default 2, a performance core on the measuring machine, as in
# phase 0's pinned runs). Run N of LABEL writes
# RESULTS_DIR/LABEL/run-N.md (the report, with the load before and after)
# and run-N.tsv. The first run of each label also writes CPU captures of
# every gallery page to target/phase1-probe/captures/LABEL, which are not
# committed.
#
# The J1 page is fixed by the first run: it records the page with the most
# mask bytes in RESULTS_DIR/j1-page.txt, unless that file exists, and every
# later run reuses it through PROBE_J1_PAGE. PROBE_QUICK=1 (fewer
# repetitions) is passed through, for a smoke run only.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
[ $# -ge 2 ] || { sed -n '2,/^set -eu/p' "$0" | sed '$d; s/^# \{0,1\}//' >&2; exit 2; }
results=$(mkdir -p "$1" && cd "$1" && pwd)
shift
work=$repo/target/phase1-probe
runs=${RUNS:-5}
mkdir -p "$work/tmp"

# Prints FILE without its top-level `fn main`, which runs to the next line
# that is a lone closing brace.
strip_main() {
  awk '/^fn main\(\)/ { skip = 1 } !skip { print } skip && /^}$/ { skip = 0 }' "$1"
}

build() {
  label=$1
  rev=$2
  co=$work/checkouts/$label
  root=$work/crates/$label
  rm -rf "$co" "$root"
  mkdir -p "$co"
  git -C "$repo" archive "$rev" | tar -x -C "$co"
  # The gallery embeds assets by relative path, so the crate keeps the
  # gallery's place below a root that links the asset folders.
  out=$root/apps/gallery
  mkdir -p "$out/src/diagram"
  ln -sfn "$co/assets" "$root/assets"
  ln -sfn "$co/apps/gallery/assets" "$out/assets"
  cp -r "$co/apps/gallery/src/." "$out/src/"
  rm -rf "$out/src/tests" "$out/src/tests.rs"
  cp "$here/probe.rs" "$out/src/probe.rs"
  cat "$here/gallery_adapter.rs" >> "$out/src/pages/layout/transforms.rs"
  echo 'pub(crate) use transforms::probe_zoom_canvas;' >> "$out/src/pages/layout.rs"
  examples=$co/crates/pert-ui/examples
  { strip_main "$examples/live_mixer.rs"; cat "$here/mixer_adapter.rs"; } > "$out/src/mixer.rs"
  for file in app canvas editor model session; do
    sed 's/crate::/crate::diagram::/g' "$examples/diagram_editor/$file.rs" > "$out/src/diagram/$file.rs"
  done
  {
    strip_main "$examples/diagram_editor/main.rs" | sed 's/crate::/crate::diagram::/g'
    cat "$here/diagram_adapter.rs"
  } > "$out/src/diagram/mod.rs"
  {
    grep -E '^mod [a-z_]+;' "$co/apps/gallery/src/main.rs" | grep -v '^mod tests;'
    echo '#[allow(dead_code)]'
    echo 'mod mixer;'
    echo '#[allow(dead_code)]'
    echo 'mod diagram;'
    echo 'mod probe;'
    echo 'fn main() -> Result<(), Box<dyn std::error::Error>> { probe::run() }'
  } > "$out/src/main.rs"
  cat > "$out/Cargo.toml" <<EOF
[package]
name = "phase1-probe"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
pert-ui = { path = "$co/crates/pert-ui", features = ["png", "svg", "cpu"] }
pert-ui-wgpu = { path = "$co/crates/pert-ui-wgpu" }
winit = { version = "0.30.13", default-features = false, features = ["rwh_06", "wayland", "wayland-dlopen", "wayland-csd-adwaita", "x11"] }

[profile.release]
debug = false

[lints.rust]
# The examples gate their window code on the facade's features, which this
# crate does not declare; those items are left out.
unexpected_cfgs = "allow"

[workspace]
EOF
  cp "$co/Cargo.lock" "$out/Cargo.lock"
  (cd "$out" && cargo build --release --target-dir "$work/target-$label")
  echo "$label=$rev ($(git -C "$repo" rev-parse --short "$rev"))" >> "$results/checkouts.txt"
}

: > "$results/checkouts.txt"
for spec in "$@"; do
  build "${spec%%=*}" "${spec#*=}"
done

echo "runs pinned to core ${PIN:-2}" >> "$results/checkouts.txt"

round=1
while [ "$round" -le "$runs" ]; do
  for spec in "$@"; do
    label=${spec%%=*}
    mkdir -p "$results/$label"
    report=$results/$label/run-$round.md
    captures=
    [ "$round" = 1 ] && captures=$work/captures/$label
    j1=
    [ -f "$results/j1-page.txt" ] && j1=$(cat "$results/j1-page.txt")
    command="env PROBE_TSV=$results/$label/run-$round.tsv PROBE_TMP=$work/tmp"
    [ -n "$captures" ] && command="$command PROBE_CAPTURES=$captures"
    [ -n "$j1" ] && command="$command PROBE_J1_PAGE=$j1"
    command="$command taskset -c ${PIN:-2} $work/target-$label/release/phase1-probe"
    echo "round $round, $label" >&2
    if [ "${QUIET:-1}" = 0 ]; then
      $command > "$report"
    else
      "$repo/plans/rendering/tools/quiet-run.sh" "$report" $command
    fi
    if [ ! -f "$results/j1-page.txt" ]; then
      sed -n 's/^j1-page: //p' "$report" > "$results/j1-page.txt"
    fi
  done
  round=$((round + 1))
done
