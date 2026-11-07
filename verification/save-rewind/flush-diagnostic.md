# One bounded save-flush stage check

2026-10-31, normal Linux release builds against the published baseline and staged candidate. Diagnostic copies only; production source was not edited. Full test runner and report rendering were paused during the batch and the test pause flag was removed on exit.

**Finding:** this batch's slow candidate flush spent its time inside the **encode wall-clock interval**, not `sync_all`. This does **not** establish the cause of the earlier 2.66-second sample.

## Workload and checks

- Generated 10-floor state, five retained history frames, cold imported engine per measured flush.
- Envelope: 2,247,175 bytes, verified exactly against the frozen legacy exporter.
- One bounded sequence: baseline collector-off, candidate-off, candidate-on, baseline-on.
- Four measured cold flushes per condition plus one separately labeled setup flush. Setup oracle had already been encoded and is not reported as cold.
- All byte-equality assertions passed. All 10 instrumented inclusive/stage/residual partitions were exact.
- No unfamiliar-file verification decode occurred: each repeated read matched the Autosave's verified bytes. Existing read and byte-comparison checks remained active.

## Instrumented cold flushes

| Implementation | Four root wall times (ms) | Median (ms) | Maximum (ms) |
|---|---|---:|---:|
| Baseline | 162.569, 186.679, 187.716, 169.016 | 177.847 | 187.716 |
| Candidate | 124.587, 122.866, 267.495, 146.985 | 135.786 | 267.495 |

Four samples are not a reliable p95 estimate. No additional outlier-hunting batch was run.

For the **267.495 ms candidate sample**, the disjoint partition was:

| Stage | ms |
|---|---:|
| Encode | 262.740610 |
| Read existing file | 1.631042 |
| Open temporary file | 0.044634 |
| Write all bytes | 1.773281 |
| `sync_all` | 0.002603 |
| Close | 0.003091 |
| Rename | 0.164090 |
| Residual bookkeeping/deallocation/timer overhead | 1.135886 |
| **Inclusive total** | **267.495237** |

Encoding includes history export, serialization and associated allocation/destruction. It is a wall interval, not a CPU sample: scheduling delays or other effects within that interval are not distinguished by these timers. The exact internal cause of its variation remains undetermined.

## Small off/on control

Collector-off outer flush medians were 194.357 ms baseline and 135.917 ms candidate (four samples each, conventional midpoint median). The on root excludes diagnostic report formatting/output, while the on benchmark's outer timer includes it. Every on sample's outer-minus-root difference was roughly 0.06–0.13 ms. Off uses the same diagnostic shim with stage collection disabled; it is not an independently compiled pristine executable. These small/noisy samples do not support a precise total instrumentation-overhead estimate.

The unusually short sync calls describe this cloud workspace/filesystem only. They do not guarantee Windows disk latency or weaken the existing durability protocol.

## Evidence and reproducibility

- `summary.json`: individual stage records, raw off/on flush samples and summaries.
- `stages-{baseline,candidate}.jsonl`: instrumented stage output.
- `results-*.json`: existing benchmark results with exact-byte assertions retained.
- `provenance.json`: original/copied session code hashes, core Rust-source hashes and executable hashes.
- `prepare.py`, `build.sh`, `run-bounded.sh`, `summarize.py`: diagnostic-only reproduction.
- Baseline binary: `target-baseline/release/save-flush-diagnostic-baseline`.
- Candidate binary: `target-candidate/release/save-flush-diagnostic-candidate`.

No ptrace operation was attempted or retried. No save cadence, verification rule, write, fsync, close or rename operation was removed.
