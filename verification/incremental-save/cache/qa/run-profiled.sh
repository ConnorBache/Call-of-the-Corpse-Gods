#!/bin/sh
# O(cases * (setup + instrumented command + encode)); attribution separate from normal timing.
set -eu
ROOT=$(pwd)
export SAVE_BENCH_IMPL=candidate SAVE_BENCH_SAMPLES=1 SAVE_BENCH_COMMANDS=wait,pickup SAVE_BENCH_PROFILE=1
unset SAVE_BENCH_EVIDENCE_DIR
"$ROOT/incremental-save/cache-work/bin/candidate-profiled" >"$ROOT/incremental-save/cache-work/reports/candidate-profiled.json" 2>"$ROOT/incremental-save/cache-work/reports/candidate-profiled.log"
