#!/bin/sh
# Usage: run.sh <consumer dir> <out dir>
set -u
consumer=$1; out=$2
export CARGO_TARGET_DIR=<repo>/target/diag-survey/target
mkdir -p "$out"
cd "$consumer" || exit 1
for f in src/bin/*.rs; do
  name=$(basename "$f" .rs)
  cargo check --offline --quiet --color never --bin "$name" > "$out/$name.txt" 2>&1
  echo "$name exit=$?"
done
