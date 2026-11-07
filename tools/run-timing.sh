#!/bin/sh
# Portable from project root after source integration; requires the pinned Rust toolchain.
# Use OUTPUT_DIR to retain machine-readable reports and exact final-state files.
set -eu
OUT=${OUTPUT_DIR:-timing-results}
mkdir -p "$OUT"
cargo build --release -p corpse-core --example timing_test
cargo run --release -p corpse-core --example timing_test -- --samples 30 --warmup 3 --output "$OUT/core-normal.json" --state-dir "$OUT/states-normal"
cargo run --release -p corpse-core --features runtime-timing --example timing_test -- --samples 30 --warmup 3 --output "$OUT/core-profiled.json" --state-dir "$OUT/states-profiled" --export-input "$OUT/generated-input.json"
cargo run --release -p corpse-desktop --features runtime-timing -- --timing-test "$OUT/desktop.json" --timing-input "$OUT/generated-input.json" --timing-samples 30 --timing-warmup 3
python3 tools/summarize-timing.py "$OUT"
