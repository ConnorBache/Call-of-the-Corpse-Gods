#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
OUT=save-journal-r39/reports/quiet
export SAVE_BENCH_SAMPLES=10 SAVE_BENCH_COMMANDS=wait,pickup SAVE_BENCH_BASELINE_LABEL=published-v17
trap 'rm -f save-journal-r39/qa/PAUSE_TESTS' EXIT
# Test runner was explicitly paused between targets before this script starts.
date -u +%FT%TZ > "$OUT/started.txt"
for turn in a1 b1 b2 a2; do
  if [[ $turn == a* ]]; then
    export SAVE_BENCH_IMPL=legacy
    COMMAND=save-journal-r39/baseline-target/release/examples/journal_command_bench
    TEMPORAL=save-journal-r39/baseline-target/release/examples/journal_temporal_bench
  else
    export SAVE_BENCH_IMPL=candidate
    COMMAND=save-journal-r39/bin/journal-command-normal
    TEMPORAL=save-journal-r39/bin/journal-temporal-normal
  fi
  SAVE_BENCH_EVIDENCE_DIR="$ROOT/$OUT/$turn-command-evidence" "$COMMAND" > "$OUT/$turn-command.json" 2> "$OUT/$turn-command.log"
  SAVE_BENCH_EVIDENCE_DIR="$ROOT/$OUT/$turn-temporal-evidence" "$TEMPORAL" > "$OUT/$turn-temporal.json" 2> "$OUT/$turn-temporal.log"
  echo "finished $turn" >&2
done
# Count/exclusive diagnostic uses separate collector-on and collector-off samples.
export SAVE_BENCH_IMPL=candidate SAVE_BENCH_SAMPLES=3
SAVE_BENCH_PROFILE=0 save-journal-r39/bin/journal-command-profiled > "$OUT/profile-off-command.json" 2> "$OUT/profile-off-command.log"
SAVE_BENCH_PROFILE=1 save-journal-r39/bin/journal-command-profiled > "$OUT/profile-on-command.json" 2> "$OUT/profile-on-command.log"
SAVE_BENCH_PROFILE=1 save-journal-r39/bin/journal-temporal-profiled > "$OUT/profile-on-temporal.json" 2> "$OUT/profile-on-temporal.log"
export SAVE_BENCH_ROUNDS=5
export SAVE_BENCH_FILTER='generated_10_floors/0/cold,generated_10_floors/1/cold,generated_10_floors/5/cold,generated_10_floors/5/changed_turn'
save-journal-r39/baseline-target/release/examples/save_envelope_bench > "$OUT/baseline-cold-flush.json" 2> "$OUT/baseline-cold-flush.log"
save-journal-r39/bin/save-envelope-normal > "$OUT/candidate-cold-flush.json" 2> "$OUT/candidate-cold-flush.log"
date -u +%FT%TZ > "$OUT/finished.txt"
