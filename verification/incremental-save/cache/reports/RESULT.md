# Cache-only opening reuse: bounded directional check

2026-10-31. This compares delivered v18 against the isolated exact-guard/cache-eviction candidate, not against the pre-journal implementation. Both use the existing direct envelope encoder and coarse journal. Production incremental storage is a separate workstream.

## Correctness

- Final focused temporal suite: **10 passed**, 0 failed. Includes exact legacy export/load, recorded-vs-uncached Revert across temporal traces, external edits, DEBUG, cursors, concurrent clones and two new guard/cache tests.
- Normal release ABBA: **24/24 full canonical envelope files byte-identical** across builds; every sample also matches frozen legacy export.
- Candidate attribution: **24/24 exact exclusive partitions**. On five-history Wait, all five export edges hit and only the newly completed edge is projected. Pickup does not execute the metadata guard.
- Metadata/root guard is conservative; arbitrary public edits cannot be inferred away. No save schema, gameplay, Revert patch semantics or durability operations changed.

## Timing scope and limitations

Quiet-agent ABBA, three samples/run, six per implementation/case. One instrumented sample/case separately. These are **directional** measurements, not reliable p95 estimates; the main incremental-save continuous-command benchmark is the intended final latency gate. Noise and occasional stalls are substantial; even cases with no guard show variation. Do not attribute every whole-build difference specifically to identity reuse. All medians are conventional midpoint medians; component medians need not sum to the enclosing median.

| Case (five starting history frames, Wait) | Baseline command+encode median | Candidate median | Baseline encode median | Candidate encode median |
|---|---:|---:|---:|---:|
| Generated 10 floors | 20.249 ms | 14.051 ms | 11.385 ms | 5.478 ms |
| Legacy 40 floors/full pack | 40.201 ms | 24.415 ms | 27.472 ms | 16.713 ms |
| Generated 10 floors/Future anchor | 43.917 ms | 45.980 ms | 30.087 ms | 23.384 ms |

The large Future case is **not a demonstrated win**. Its full envelope is 15,578,695 bytes. Command median rose from 14.643 to 21.873 ms despite lower encode time. The separate guard attribution was 0.0455 ms ordinary versus 4.6321 ms Future. This confirms that exact metadata serialization can itself be material; it must not be called O(1).

A possible next narrow optimization is to compare the Future `Option<String>` directly for exact equality and omit that already-verified string from guard serialization only. It must not be omitted from the actual inverse or save. This proposal is not included in these measurements.

## Files

- `summary.json`: all raw samples, midpoint medians, nearest-rank quantiles (not reliable p95 at n=6), full-state hashes and scoped counts.
- `baseline-{1,2}.json`, `candidate-{1,2}.json`: normal run output.
- `candidate-profiled.json`: separate attribution.
- `states-baseline/`, `states-candidate/`: exact full envelope evidence.
- `focused-final.log`: final ten-test result.
- `provenance.json`: source/harness/executable hashes.
- `../qa/run-normal.sh`, `run-profiled.sh`, `summarize.py`: reproduction.

The quiet batch ended at 18:41:50 UTC; no further measurement runs were made for this comparison.

## Superseded guard serialization detail

The final approved Future direct-comparison fast path and its bounded regression are documented in [future-final/RESULT.md](future-final/RESULT.md). The measurements above remain intact as the **first candidate**, not the final release candidate. Both phases produce exactly the same envelopes in the final repeated cases.
