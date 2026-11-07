# Final cache guard: verified Future text fast path

Final source integrates exact `Option<String>` Future comparison before removing that already-verified text from **guard-only** JSON headers. None and Some(empty) remain distinct; other Missing/Null/Value fields remain in serialized metadata. Actual inverse/save representation is unchanged. Header clones still copy Future temporarily; the improvement removes duplicate escaping/serialization, not all O(Future bytes) work.

## Verification

- Final focused temporal suite: **10 passed**, 0 failed (`../focused-final-future.log`).
- Final normal release ABBA: six samples per implementation for generated10-floor/five-history Wait, with and without a real Future anchor.
- **2/2 complete envelopes byte-identical** across builds and to the earlier full-matrix candidate. Future envelope: **15,578,695 bytes**. Every individual sample also passes the frozen-export equality assertion.
- Both attribution roots have exact exclusive partitions, five export-cache hits and one newly projected edge.
- Main and isolated `temporal.rs` are byte-identical, SHA256 `57b3f7d86383dae7d17cca4ad316aa15ff7eb05516af6e1a2f65cf786c8cba17`.

## Directional measurements, not reliable p95

| Five-history Wait | Baseline command+encode median | Final candidate median | Baseline encode median | Final encode median |
|---|---:|---:|---:|---:|
| Generated 10 floors | 24.493 ms | 14.307 ms | 13.935 ms | 5.758 ms |
| Same world / Future anchor | 75.794 ms | 35.971 ms | 51.931 ms | 22.214 ms |

These are small, noisy, separately scheduled batches. Baseline Future time changed substantially from the first diagnostic; do not present these ratios as universal causal speedups. The larger continuous command+durable-save experiment is the final latency reference. Component medians need not sum to enclosing medians.

Separate single-sample guard attribution: ordinary **0.0803 ms**; large Future **0.6565 ms**, versus **4.6321 ms** for the first candidate that escaped Future into both guard headers. This supports the narrow fast-path rationale but is not a precision overhead estimate.

No further production changes or optimization scope followed this fix. No storage cadence, validation, fsync or rename behavior was changed by the cache work.

## Evidence

`summary.json`, raw `baseline-{1,2}.json` / `candidate-{1,2}.json`, `candidate-profiled.json`, exact `states-*` files and `provenance.json`; reproduction `../../qa/run-future-final.sh`. Earlier full-matrix evidence remains in the parent report directory, with its original code/harness reconstructed and verified against recorded hashes (`first-candidate-temporal.rs`, `first-cache_opening_bench.rs`).
