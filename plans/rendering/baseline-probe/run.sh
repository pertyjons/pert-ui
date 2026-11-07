#!/bin/sh
# Builds and runs the phase 0 frame-cost baseline (probe.rs) against this
# checkout. It needs an unlocked desktop session with a hardware GPU: a window
# opens for the wgpu renderer, which then renders offscreen.
#
#   plans/rendering/baseline-probe/run.sh [OUT_DIR]
#
# The scratch crate goes below OUT_DIR (default: a new temporary directory). It
# copies the gallery's modules beside probe.rs, depends on this checkout's
# crates by path and reuses its Cargo.lock; its target directory is
# target/rendering-baseline in this checkout. PROBE_CHANGE_PAGE runs only
# the change scenarios, on that page; by default they run after the page
# tables on the page with the most mask bytes.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
root=${1:-$(mktemp -d)}
# The gallery embeds assets by relative path, so the crate keeps the
# gallery's place below a root that links the asset folders.
out=$root/apps/gallery
mkdir -p "$out/src"
ln -sfn "$repo/assets" "$root/assets"
ln -sfn "$repo/apps/gallery/assets" "$out/assets"
cp -r "$repo/apps/gallery/src/." "$out/src/"
rm -rf "$out/src/tests" "$out/src/tests.rs"
cp "$here/probe.rs" "$out/src/probe.rs"
{
  grep -E '^mod [a-z_]+;' "$repo/apps/gallery/src/main.rs" | grep -v '^mod tests;'
  echo 'mod probe;'
  echo 'fn main() -> Result<(), Box<dyn std::error::Error>> { probe::run() }'
} > "$out/src/main.rs"
cat > "$out/Cargo.toml" <<EOF
[package]
name = "rendering-baseline-probe"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
pert-ui = { path = "$repo/crates/pert-ui", features = ["png", "svg", "cpu"] }
pert-ui-wgpu = { path = "$repo/crates/pert-ui-wgpu" }
winit = { version = "0.30.13", default-features = false, features = ["rwh_06", "wayland", "wayland-dlopen", "wayland-csd-adwaita", "x11"] }

[profile.release]
debug = false

[workspace]
EOF
cp "$repo/Cargo.lock" "$out/Cargo.lock"
cd "$out"
cargo run --release --target-dir "$repo/target/rendering-baseline"
