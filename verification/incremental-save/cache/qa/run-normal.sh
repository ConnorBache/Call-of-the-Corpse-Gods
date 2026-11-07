#!/bin/sh
# O(cases * samples * (setup + command + encode)); execute only in agreed quiet window.
set -eu
ROOT=$(pwd)
export SAVE_BENCH_IMPL=candidate SAVE_BENCH_SAMPLES=3 SAVE_BENCH_COMMANDS=wait,pickup
for run in baseline-1 candidate-1 candidate-2 baseline-2; do
 implementation=${run%-*}
 if [ "${run##*-}" = 1 ]; then export SAVE_BENCH_EVIDENCE_DIR="$ROOT/incremental-save/cache-work/reports/states-$implementation"; else unset SAVE_BENCH_EVIDENCE_DIR; fi
 "$ROOT/incremental-save/cache-work/bin/$implementation-normal" >"$ROOT/incremental-save/cache-work/reports/$run.json" 2>"$ROOT/incremental-save/cache-work/reports/$run.log"
done
