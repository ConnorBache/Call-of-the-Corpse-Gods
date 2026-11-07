#!/bin/sh
# O(two cases * bounded samples); only run during parent-granted shared quiet window.
set -eu
ROOT=$(pwd)
OUT="$ROOT/incremental-save/cache-work/reports/future-final"
export SAVE_BENCH_IMPL=candidate SAVE_BENCH_SAMPLES=3 SAVE_BENCH_COMMANDS=wait
export CACHE_BENCH_FILTER=generated_10_floors-history-5-wait,generated_10_floors_future_anchor-history-5-wait
for run in baseline-1 candidate-1 candidate-2 baseline-2; do
 implementation=${run%-*}; binary=baseline-filtered; if [ "$implementation" = candidate ]; then binary=candidate-final-normal; fi
 if [ "${run##*-}" = 1 ]; then export SAVE_BENCH_EVIDENCE_DIR="$OUT/states-$implementation"; else unset SAVE_BENCH_EVIDENCE_DIR; fi
 "$ROOT/incremental-save/cache-work/bin/$binary" >"$OUT/$run.json" 2>"$OUT/$run.log"
done
unset SAVE_BENCH_EVIDENCE_DIR
export SAVE_BENCH_SAMPLES=1 SAVE_BENCH_PROFILE=1
"$ROOT/incremental-save/cache-work/bin/candidate-final-profiled" >"$OUT/candidate-profiled.json" 2>"$OUT/candidate-profiled.log"
