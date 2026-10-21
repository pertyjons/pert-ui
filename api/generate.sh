#!/usr/bin/env bash
# Regenerates the public API listings in this directory: one file per
# published package and feature configuration, `<package>/<configuration>.txt`.
# CI runs it and fails when the committed listings differ; CONTRIBUTING.md
# describes when to run it.
#
# Needs cargo-public-api at TOOL_VERSION (`cargo install --locked
# cargo-public-api@<version>`) and the toolchain pinned in rust-toolchain.toml.
# The listings are a view for x86_64 Linux and depend on both versions, so a
# toolchain or tool update regenerates them in the same change.
#
# rustdoc's JSON output is unstable. Instead of a nightly toolchain, the JSON is
# built with the pinned stable toolchain and RUSTC_BOOTSTRAP=1, set for that one
# command only; no build, test or package of the library uses it.
set -euo pipefail

TOOL_VERSION=0.52.0
TARGET=x86_64-unknown-linux-gnu

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

tool="$(cargo-public-api --version 2>/dev/null || true)"
if [ "$tool" != "cargo-public-api $TOOL_VERSION" ]; then
    echo "api/generate.sh: needs cargo-public-api $TOOL_VERSION, found '${tool:-none}';" \
        "run: cargo install --locked cargo-public-api@$TOOL_VERSION" >&2
    exit 1
fi
pinned="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
rustc="$(rustc --version)"
case "$rustc" in
    "rustc $pinned "*) ;;
    *) echo "api/generate.sh: needs the pinned Rust $pinned, found '$rustc'" >&2; exit 1 ;;
esac

target_dir="$root/target/api"
rm -f api/*/*.txt

# listing <package> <configuration> [feature arguments...]
listing() {
    local package="$1" configuration="$2"
    shift 2
    # Empty encoded flags take precedence over every other flag source
    # (environment, target-specific variables and Cargo configuration).
    CARGO_ENCODED_RUSTFLAGS='' CARGO_ENCODED_RUSTDOCFLAGS='' RUSTC_BOOTSTRAP=1 \
        cargo rustdoc --locked --quiet -p "$package" --target "$TARGET" \
        --target-dir "$target_dir" "$@" -- -Z unstable-options --output-format json
    mkdir -p "api/$package"
    cargo-public-api --color=never --simplified \
        --rustdoc-json "$target_dir/$TARGET/doc/${package//-/_}.json" \
        >"api/$package/$configuration.txt"
    echo "api/$package/$configuration.txt"
}

# The configurations CI checks, each package on its own.
listing pert-ui-core default
listing pert-ui-core serde --features serde
listing pert-ui-runtime default
listing pert-ui-runtime serde --features serde
listing pert-ui-cpu default
listing pert-ui-wgpu default
listing pert-ui default
listing pert-ui no-default --no-default-features
listing pert-ui images --features images
listing pert-ui image-formats --features image-formats
listing pert-ui no-default-png --no-default-features --features png
listing pert-ui svg --features svg
listing pert-ui no-default-svg --no-default-features --features svg
listing pert-ui serde --features serde
listing pert-ui no-default-serde --no-default-features --features serde
listing pert-ui cpu --features cpu
listing pert-ui no-default-desktop-cpu --no-default-features --features desktop-cpu
