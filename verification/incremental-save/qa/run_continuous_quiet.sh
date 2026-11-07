#!/usr/bin/env bash
set -euo pipefail
cd /workspace/scratch/69f53637ba6f
OUT=incremental-save/reports/continuous-quiet
date -u +%FT%TZ > "$OUT/started.txt"
export SAVE_BENCH_STEPS=128 SAVE_BENCH_CASES=10-h0-wait,10-h1-wait,10-h5-wait,10-h5-pickup,40-h5-wait,40-h5-pickup
for turn in a1 b1 b2 a2; do
 if [[ $turn == a* ]]; then BIN=incremental-save/baseline-bench-target/release/examples/incremental_save_bench; export SAVE_BENCH_IMPL_LABEL=published-v18; else BIN=incremental-save/candidate-bench-target/release/examples/incremental_save_bench; export SAVE_BENCH_IMPL_LABEL=incremental-candidate; fi
 SAVE_BENCH_EVIDENCE_DIR="$PWD/$OUT/$turn-evidence" "$BIN" > "$OUT/$turn.json" 2> "$OUT/$turn.log"
 echo "finished $turn"
done
date -u +%FT%TZ > "$OUT/finished.txt"
