#!/bin/sh
# Builds and runs the blend-variant benchmark (bench/bench.rs) in a scratch
# crate holding a copy of the gallery's modules and the vello spike's
# translate.rs, compare.rs, workloads.rs and pertui.rs, as
# plans/rendering/vello-spike/gallery.sh does. CPU only, no window.
#
#   plans/rendering/cpu-blend-variants/bench.sh > OUTPUT
#
# The scratch crate goes to target/cpu-blend-variants/scratch in this
# checkout; the build shares target/vello-spike. BENCH_FRAMES sets the
# timed renders per scene (default 30); BENCH_BUILD_ONLY=1 only builds.
# Pin it yourself (taskset -c 2-7), or use run.sh.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
spike=$here/../vello-spike
root=$repo/target/cpu-blend-variants/scratch
out=$root/apps/gallery
rm -rf "$out/src"
mkdir -p "$out/src"
ln -sfn "$repo/assets" "$root/assets"
ln -sfn "$repo/apps/gallery/assets" "$out/assets"
cp -r "$repo/apps/gallery/src/." "$out/src/"
rm -rf "$out/src/tests" "$out/src/tests.rs"
cp "$here/bench/bench.rs" "$spike/src/translate.rs" "$spike/src/compare.rs" \
  "$spike/src/workloads.rs" "$spike/src/pertui.rs" "$out/src/"
{
  grep -E '^mod [a-z_]+;' "$repo/apps/gallery/src/main.rs" | grep -v '^mod tests;'
  echo 'mod bench;'
  echo 'mod compare;'
  echo 'mod pertui;'
  echo 'mod translate;'
  echo 'mod workloads;'
  echo 'fn main() -> Result<(), Box<dyn std::error::Error>> { bench::run() }'
} > "$out/src/main.rs"
cat > "$out/Cargo.toml" <<TOML
[package]
name = "cpu-blend-variants-bench"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
pert-ui = { path = "$repo/crates/pert-ui", features = ["png", "svg", "cpu"] }
pert-ui-cpu-variants = { path = "$here" }
vello_cpu = { version = "0.3.0", default-features = false, features = ["std", "u8_pipeline", "multithreading"] }
png = "0.18.1"

[profile.release]
debug = false

[workspace]
TOML
cp "$spike/Cargo.lock" "$out/Cargo.lock"
cd "$out"
cargo build --release --target-dir "$repo/target/vello-spike" >&2
[ -n "${BENCH_BUILD_ONLY:-}" ] && exit 0
"$repo/target/vello-spike/release/cpu-blend-variants-bench"
