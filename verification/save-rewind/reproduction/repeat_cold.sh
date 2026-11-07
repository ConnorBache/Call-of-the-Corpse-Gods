#!/usr/bin/env bash
set -euo pipefail
cd /workspace/scratch/69f53637ba6f
OUT=save-journal-r39/reports/stage2/cold-repeat
while ! python3 -c 'import json; assert json.load(open("save-journal-r39/reports/stage2/full-tests/status.json"))["state"] in ["paused_between_targets","passed"]' 2>/dev/null; do sleep 2; done
trap 'rm -f save-journal-r39/qa/PAUSE_STAGE2_TESTS' EXIT
date -u +%FT%TZ > "$OUT/started.txt"
export SAVE_BENCH_ROUNDS=5 SAVE_BENCH_FILTER=generated_10_floors/5/cold
for turn in a1 b1 b2 a2; do
 if [[ $turn == a* ]]; then BIN=save-journal-r39/baseline-target/release/examples/save_envelope_bench; else BIN=save-journal-r39/bin/save-envelope-normal; fi
 "$BIN" > "$OUT/$turn.json" 2> "$OUT/$turn.log"
 echo "finished $turn"
done
export SAVE_BENCH_ROUNDS=2
for turn in baseline candidate; do
 if [[ $turn == baseline ]]; then BIN=save-journal-r39/baseline-target/release/examples/save_envelope_bench; else BIN=save-journal-r39/bin/save-envelope-normal; fi
 strace -ttt -T -yy -e trace=openat,read,write,fsync,fdatasync,rename,renameat,renameat2,close -o "$OUT/$turn.strace" "$BIN" > "$OUT/$turn-traced.json" 2> "$OUT/$turn-traced.log"
done
date -u +%FT%TZ > "$OUT/finished.txt"
