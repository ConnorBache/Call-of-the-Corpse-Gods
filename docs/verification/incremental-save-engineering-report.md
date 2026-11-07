# Incremental save engineering findings

31 October 2026

**Status: executable and static gates passed.** All 164 Linux workspace test harnesses passed, with 1,306 passed test executions, 0 failed and 13 ignored. Continuous and supplemental recovery measurements are complete. The Windows cross-build and static checks passed; final archive assembly is recorded separately. Native Windows runtime and hardware power-loss behavior remain unverified.

## Finding and scope

Compared with published v18, this candidate lowers median and p95 **command plus durable save** time in all six measured continuous traces, with periodic full checkpoints included. It writes substantially fewer logical bytes. It does not eliminate checkpoint spikes, existing-file scanning, or costly loads; some maxima and load observations are worse.

The candidate combines command-based disk persistence with exact opening-identity/cache reuse in the in-memory history subsystem. These are whole-candidate comparisons, not isolated attribution of every improvement to the disk format.

## Continuous performance

Normal Linux release, collector off; A/B/B/A order, with A = published v18 and B = candidate. Each run executes **128 consecutive commands without resetting the Engine**. Combining two runs per implementation gives **256 samples per case**. H is history retained at setup, not a guarantee about every subsequent state. Fixtures use 10 or 40 floors, remove NPCs and boost player HP/energy to survive the trace; the 40-floor fixture also has a synthetic 20-item pack. These are controlled persistence tests, not combat or input-to-display measurements.

The measured interval includes dispatch, save validation and durable flush, including `sync_all` and the periodic checkpoint path. Each candidate case contains **248 routine appends and 8 full rollover checkpoints**. Two initial checkpoints per case are timed separately, excluded from the 256-command summary and byte totals. Across all cases, observed initial-checkpoint times span 4.812–28.185 ms candidate and 4.272–92.400 ms baseline.

Setup, assertions, evidence output and selected read/load/five-Revert checks are outside the command/save timers. Those checks run at steps 31, 32, 64, 96 and 128 and can perturb caches or allocator state for later samples. Both implementations perform the same validation protocol, but replay work differs. The filesystem cache is not forced cold. These sequential samples are not independent repetitions; no confidence interval or population-tail guarantee is claimed. Median means the upper-middle order statistic; p95 is nearest-rank.

### Command plus durable save

All values are milliseconds; all periodic checkpoints are included.

| Case | v18 median | Candidate median | v18 p95 | Candidate p95 |
|---|---:|---:|---:|---:|
| 10-h0-wait | 27.227 | 12.123 | 60.438 | 29.131 |
| 10-h1-wait | 28.728 | 13.171 | 63.234 | 37.237 |
| 10-h5-wait | 28.188 | 12.976 | 58.196 | 22.779 |
| 10-h5-pickup | 13.346 | 8.310 | 27.611 | 14.144 |
| 40-h5-wait | 39.785 | 12.668 | 83.448 | 24.347 |
| 40-h5-pickup | 25.399 | 8.812 | 51.766 | 19.990 |

**Retained regressions:** in `10-h1-wait`, the maximum combined sample rose from 86.119 to 163.413 ms, and a routine append reached 102.685 ms. In `10-h5-pickup`, command-only median rose from 6.972 to 7.229 ms. Better medians/p95 do not mean every command or worst observation improved.

### Candidate flush components

These are inclusive flush times, not command times or isolated disk-only costs. Each row has 248 append samples and 8 periodic-checkpoint samples. With only 8 checkpoints, nearest-rank p95 equals the maximum; the table reports that maximum rather than implying a stable tail estimate.

| Case | Append median | Append p95 | Append max | Checkpoint median | Checkpoint max |
|---|---:|---:|---:|---:|---:|
| 10-h0-wait | 0.880 | 2.129 | 6.085 | 8.981 | 10.013 |
| 10-h1-wait | 0.974 | 3.416 | 102.685 | 9.631 | 38.821 |
| 10-h5-wait | 0.927 | 1.598 | 6.578 | 8.875 | 13.893 |
| 10-h5-pickup | 0.985 | 1.955 | 4.534 | 8.385 | 15.036 |
| 40-h5-wait | 2.019 | 3.362 | 7.373 | 23.087 | 29.775 |
| 40-h5-pickup | 2.275 | 6.010 | 17.530 | 32.770 | 54.903 |

Checkpoint work is not gone. For example, `40-h5-pickup` checkpoint median is 32.770 ms versus v18's per-command full-flush median of 19.113 ms; the candidate wins the continuous distribution because most saves append instead.

### Logical bytes written

Totals cover the 256 command-triggered saves, including all 8 candidate periodic checkpoints, but exclude initial checkpoints. The harness counts full file length at a checkpoint and file-length growth at an append. These are logical save bytes, **not measured physical storage traffic**, filesystem metadata writes or device write amplification. Reductions range from 96.78% to 96.87%; exact existing-file reads remain.

| Case | v18 bytes | Candidate bytes |
|---|---:|---:|
| 10-h0-wait | 561,316,216 | 17,734,048 |
| 10-h1-wait | 561,541,190 | 17,829,238 |
| 10-h5-wait | 562,397,848 | 18,102,610 |
| 10-h5-pickup | 547,326,780 | 17,179,204 |
| 40-h5-wait | 1,248,767,272 | 39,480,150 |
| 40-h5-pickup | 1,235,770,940 | 38,693,084 |

## Load and recovery tradeoff

Load reconstructs a full checkpoint and then executes its bounded command suffix. This shifts work to loading rather than making it free. The continuous run supplies only **two observations per implementation at step 31**, plus eight at checkpoint boundaries per case. The baseline always loads a full snapshot; only the candidate step-31 file has a 31-command tail. Selected observed ranges below are milliseconds, not medians or p95 estimates.

| Case | v18 load at step 31, n=2 | Candidate 31-command-tail load, n=2 | Candidate checkpoint-boundary load, n=8 |
|---|---:|---:|---:|
| 10-h0-wait | 601.3–630.5 | 352.5–372.7 | 553.2–1112.1 |
| 10-h5-wait | 658.9–1229.7 | 824.3–1676.9 | 575.2–746.9 |
| 10-h5-pickup | 600.3–1080.3 | 1286.3–2239.4 | 518.4–720.6 |
| 40-h5-wait | 2401.1–2484.6 | 2413.4–2707.4 | 2039.6–2426.0 |
| 40-h5-pickup | 2011.4–2404.1 | 4151.5–4702.6 | 1918.5–4294.3 |

The `40-h5-pickup` tail loads reached 4.15–4.70 seconds; this regression is not hidden by the faster save path. The original baseline also had a 14.398-second checkpoint-boundary load outlier in `40-h5-wait`; all observations remain in the raw data. Causes are not established by wall timers. The supplemental recovery measurements below are additional evidence, not a replacement for this first batch.

## Supplemental recovery timing

This is a **within-candidate** comparison using the same final normal-release build, not a second v18 A/B test. Each mode has 10 samples, alternating forward/reverse mode order, with a fresh Engine and bounded file read in the timer. Setup and exact-state assertions are outside timing; the filesystem cache is not forced cold. These are **Wait traces with five starting history frames**, not the earlier pickup workload that reached 4.70 seconds.

The full-checkpoint mode reads a standalone full legacy envelope representing the current state. The valid-tail mode loads an earlier checkpoint and replays 31 commands. The torn-tail mode adds an incomplete, uncommitted 32nd command and must recover the same state as the valid tail. Exact valid-versus-torn canonical-save assertions passed. Values below are milliseconds; ranges are observed minimum–maximum. With n=10, nearest-rank p95 equals the maximum and is not a stable population-tail estimate.

| Floors | Mode | Median | Observed range | File bytes |
|---|---|---:|---:|---:|
| 10 | Full checkpoint | 860.374 | 684.614–1206.389 | 2,274,434 |
| 10 | Valid 31-command tail | 1088.071 | 964.185–1390.835 | 2,145,886 |
| 10 | Torn uncommitted 32nd | 1004.914 | 925.546–1259.613 | 2,146,029 |
| 40 | Full checkpoint | 2347.369 | 1970.054–3454.112 | 4,930,942 |
| 40 | Valid 31-command tail | 2698.230 | 2235.736–3569.673 | 4,835,121 |
| 40 | Torn uncommitted 32nd | 2740.316 | 2392.572–3580.975 | 4,835,264 |

The measured replay-tail medians exceed full-checkpoint medians for both world sizes. Torn-tail recovery preserves the committed prefix, but does not make loading cheap. Small valid-versus-torn timing differences do not establish an intrinsic recovery speed difference or explain earlier outliers.

## What is preserved and what changes

- The tracked desktop Engine records successful commands plus a small sanity stamp, not a whole-world checksum. Mutable escapes, errors/panics, imports, cloned sessions and receipt overflow require full checkpoints. Session/epoch and persisted-base tokens prevent appending another branch's suffix or acknowledging unsaved changes prematurely.
- The new persistent container holds an unchanged legacy envelope checkpoint plus length/CRC-framed command batches. The header CRC is verified before trusting length; sequences, previous-checksum links, strict commands and replay compatibility are checked. CRC is not authentication. Valid incomplete final records, including torn UTF8, recover the last complete prefix; complete bad records reject. Loading does not rewrite the file. Retry checkpoints live state rather than blindly duplicating uncertain appends.
- Ordinary changed saves still sync the file before acknowledgement. Checkpoints still use temporary file, write, sync and rename. At 32 accumulated commands or 1 MiB tail data the save checkpoints; oversized/untracked cases also checkpoint. No fsync was skipped, made asynchronous or replaced with delayed acknowledgement. Existing exact-byte file validation, session isolation, unchanged-key skip and terminal policies remain.
- The persistent `.lock` sidecar uses a cooperating nonblocking OS lock. Contention returns a recoverable error; process exit releases the kernel lock. Do not delete/recreate the sidecar while sessions use it. Old builds, editors/exporters that ignore the protocol, and alternate symlink/hardlink names are not excluded. The empty sidecar is coordination, not a second persistent-state transaction.
- Only simulation revision 39 with replay compatibility `corpse-r39-incremental-v1` uses journals. Revisions 29–38 remain full snapshots without silently changing rules. Old snapshots still load, but older executables cannot read the new journal container. Export a compatible full snapshot before rollback; preserve provenance restrictions. Replay semantic changes require a new compatibility ID.

## Complexity remains bounded rather than purely incremental

Let E be existing file bytes, D new receipt bytes, and Wᵢ the full simulation cost of replayed command i. Routine changed saves are **O(E + D)** time and transient memory, plus lock/I/O/sync latency: exact old-file read/comparison is still linear in E. Routine appends do not serialize or diff the current world, but checkpoints still pay full canonical history export and current Run serialization. Initial cache capacity reserves bounded tail room to avoid first-append whole-buffer reallocation.

Load is O(E + checkpoint import/history reconstruction + ΣWᵢ), with a storage replay budget of at most 32 commands. Command count does not bound command execution cost or payload bytes by itself. Inputs are streaming-read-bounded to 32 MiB; the codec also caps headers, batches and records. Exact opening-identity reuse adds a nonconstant metadata/root comparison on eligible completed turns; large Future anchors remain large. This work neither supplies a complete per-field mutation journal nor finishes the paused every-function complexity audit.

## Correctness evidence and verification status

**Exact cross-binary evidence:** all 30 selected canonical live-state exports match byte-for-byte across A1/B1/B2/A2: six cases × steps 31, 32, 64, 96 and 128. These comparisons include serialized history. They are not a claim that the new journal bytes equal the old full-file format.

**Load/rewind oracle:** each selected loaded state is compared with the live source with only the opaque `history` field removed, then both undergo up to five actual Reverts, checking exact remaining serialized state and matching Revert availability after each. This is an executed semantic-history oracle, not merely omission of history from a comparison.

A separate **pre-existing v18 reload spelling issue** was reproduced by the baseline smoke run: one inverse group reordered four deletion-only patches on distinct `floors[0].vents[1].pulse` map keys. The stored mismatch pair differs only in `history`; the patch multiset is unchanged. It is not treated as an incremental-save regression, nor is byte-identical post-load history spelling claimed. The five-Revert checks preserve a behavioral requirement despite that ordering difference.

| Gate | Verified status |
|---|---|
| Focused session-storage suite | **45 passed, 0 failed, 1 helper ignored**, 266 filtered out; not the full workspace suite |
| Real separate-process probes within that suite | Passed: kill during partial append; kill after real sync before acknowledgement; nonblocking contention and lock release after killing owner |
| Other focused coverage | Framing/corruption/truncation, strict command fields, replay stamps, mutable-escape and retry behavior, free/haste/Revert/Future/resume, rollover, legacy revisions, session/terminal isolation |
| Continuous Linux benchmark | Completed A/B/B/A; 30 exact cross-binary states and selected semantic rewind checks passed |
| Full Linux workspace test gate | **164/164 executable harnesses passed: 1,306 passed test executions, 0 failed, 13 ignored, 0 measured, 0 filtered**; includes repeated module tests across all-target harnesses, not 1,306 unique cases |
| Doctest command | Exit 0; 0 passed, 0 failed, 0 ignored |
| Windows cross-build and static checks | **Passed**: unsigned PE32+ x64; system DLLs plus bundled SDL2 only; 84 creature and 4 UI assets, fonts and exact DPI resource checked; archive CRC/member-byte checks passed. Final archive receipt is separate |
| Supplemental recovery timing | Completed: 10 samples for each of 6 modes; exact valid-versus-torn assertions passed |
| Native Windows runtime/filesystem behavior | **Not tested** |
| Hardware power loss or stronger directory-fsync durability | **Not claimed** |

The focused suite's ignored helper is intentionally launched by supervised process tests; it is not an independently skipped acceptance scenario. The 45-pass count is the whole focused storage suite, **not 45 process-kill tests**. The full workspace gate used opt-level 1, debug info 0, debug assertions enabled and overflow checks enabled; it is distinct from the normal-release performance build and compact Windows release build. Passing these gates does not establish native Windows runtime or hardware power-loss behavior.

## Evidence inventory for the final bundle

Paths below are relative to the `incremental-save` working area. Keep the first-batch data even when adding supplemental measurements.

- `reports/continuous-quiet/summary.json`, `summary.txt`: merged samples and 30 exact-state hash results.
- `reports/continuous-quiet/a1.json`, `b1.json`, `b2.json`, `a2.json`: raw per-run samples, bytes, initial saves and selected loads; matching `.log` files plus `runner.log`, `started.txt`, `finished.txt` establish completion/order. Window: 19:03:29–19:08:54 UTC on 31 October 2026.
- `reports/continuous-quiet/{a1,b1,b2,a2}-evidence/*.json`: 120 files representing the same 30 named states across four runs.
- `qa/run_continuous_quiet.sh`, `qa/summarize_continuous.py`: run and order-statistic aggregation methods.
- `reports/continuous-binary-hashes.txt`, `reports/pre-bench-source.json`: executable and source provenance.
- `source/crates/corpse-desktop/examples/incremental_save_bench.rs`: fixture construction, timed boundaries and semantic five-Revert oracle.
- `reports/session-store-process-kill-tests.log`: final focused 45/0/1 result and supervised child probes; `reports/session-store-tests-final.log`, `session-store-tests-lock-final.log`, `main-dispatch-tests.log`, `main-pack-tests.log` retain earlier focused evidence.
- `reports/baseline-smoke.log`, `baseline-smoke-not-timing.json`, `checkpoint-mismatch-left.json`, `checkpoint-mismatch-right.json`: reproduced baseline history-spelling discrepancy. These smoke attempts are **not timing results**.
- `reports/continuous-smoke*.log`, `continuous-smoke*-not-timing.json`: harness-development evidence, excluded from performance claims.
- `source/docs/incremental-save-journal.md`, `source/docs/snapshot-opening-cache-reuse.md`: format, compatibility, coordination, ownership and transitive complexity contracts.
- `reports/recovery-quiet.json`, `recovery-quiet.log`, `recovery-started.txt`, `recovery-finished.txt`: supplemental raw recovery timings and completion; `source/crates/corpse-desktop/examples/incremental_recovery_bench.rs` defines modes/assertions. `reports/recovery-bench-build.log` is compilation evidence only.
- `reports/full-test-summary.json`, `reports/full-tests/status.json` and per-harness logs: final 164-harness execution gate; `reports/full-test-build.log`, `reports/full-test-artifacts.jsonl` and `reports/doctests.log`: build, target inventory and doctest evidence.
- `reports/windows-final-gate.json`, `reports/windows-assets-static.json`, `reports/windows-build-profile.json`, `reports/windows-build-retry.log` and `reports/windows-package-final.log`: Windows cross-build/static checks and provenance; final archive identity is recorded in the packaging receipt, not frozen in this report.
