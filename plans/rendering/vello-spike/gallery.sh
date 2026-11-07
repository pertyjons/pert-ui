#!/bin/sh
# Builds and runs part 2 on the headless gallery: gallery/gallery_probe.rs
# with ../src/translate.rs and ../src/compare.rs, in a scratch crate that
# holds a copy of the gallery's modules (as plans/rendering/baseline-probe/run.sh
# does). No window or GPU is needed.
#
#   plans/rendering/vello-spike/gallery.sh [RESULTS_DIR]
#
# The scratch crate goes to target/vello-spike/gallery-crate in this checkout
# and shares the spike's target directory and Cargo.lock. SPIKE_SAVE_PAGE=<key>
# saves that page's two frames at scale 1 as PNGs.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
results=$(cd "${1:-$here/results}" && pwd)
root=$repo/target/vello-spike/gallery-crate
out=$root/apps/gallery
rm -rf "$out/src"
mkdir -p "$out/src"
ln -sfn "$repo/assets" "$root/assets"
ln -sfn "$repo/apps/gallery/assets" "$out/assets"
cp -r "$repo/apps/gallery/src/." "$out/src/"
rm -rf "$out/src/tests" "$out/src/tests.rs"
cp "$here/gallery/gallery_probe.rs" "$here/src/translate.rs" "$here/src/compare.rs" "$out/src/"
{
  grep -E '^mod [a-z_]+;' "$repo/apps/gallery/src/main.rs" | grep -v '^mod tests;'
  echo 'mod compare;'
  echo 'mod gallery_probe;'
  echo 'mod translate;'
  echo 'fn main() -> Result<(), Box<dyn std::error::Error>> { gallery_probe::run() }'
} > "$out/src/main.rs"
cat > "$out/Cargo.toml" <<TOML
[package]
name = "vello-spike-gallery"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
pert-ui = { path = "$repo/crates/pert-ui", features = ["png", "svg", "cpu"] }
vello_cpu = { version = "0.3.0", default-features = false, features = ["std", "u8_pipeline", "multithreading"] }
png = "0.18.1"

[profile.release]
debug = false

[workspace]
TOML
cp "$here/Cargo.lock" "$out/Cargo.lock"
cd "$out"
cargo build --release --target-dir "$repo/target/vello-spike"
SPIKE_OUT="$results" "$repo/target/vello-spike/release/vello-spike-gallery"
