# Run from source root with pinned Rust 1.99.1 installed. Does not touch player saves.
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force timing-results | Out-Null
cargo run --release -p corpse-core --example timing_test -- --samples 30 --warmup 3 --output timing-results/core-normal.json --state-dir timing-results/states-normal
if ($LASTEXITCODE) { throw "Normal timing run failed" }
cargo run --release -p corpse-core --features runtime-timing --example timing_test -- --samples 30 --warmup 3 --output timing-results/core-profiled.json --state-dir timing-results/states-profiled --export-input timing-results/generated-input.json
if ($LASTEXITCODE) { throw "Profiled timing run failed" }
cargo run --release -p corpse-desktop --features runtime-timing -- --timing-test timing-results/desktop.json --timing-input timing-results/generated-input.json --timing-samples 30 --timing-warmup 3
if ($LASTEXITCODE) { throw "Desktop timing run failed" }
python tools/summarize-timing.py timing-results
if ($LASTEXITCODE) { throw "Timing summary/parity failed" }
