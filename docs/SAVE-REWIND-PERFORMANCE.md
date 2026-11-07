# Save and rewind: first-write journal release

## What changed

1. Before a supported floor, actor-group or ground-item-group mutable access, retain its original shared block once. Actual copy-on-write costs still count.
2. At command completion, use that evidence plus immutable snapshot identities to skip unchanged blocks. Walk changed blocks to construct the same exact selective inverse leaf patches as before; include scalar/structural snapshot fallback.
3. Keep at most five exact encoded edges, keyed by immutable endpoints and archive cursors. Revert consumes the already sealed inverse only on an exact match; otherwise it recomputes safely.
4. Serialize the typed current Run directly into the save envelope, removing the old string-to-JSON reparsing pass.
5. Keep the existing full current-state save, validation, temporary write, sync, rename, recovery and save cadence. No on-disk format change.

**This is a coarse first-write block journal, not a complete per-field mutation log or an O(changes) implementation.** A dirty block can still be large; cold loaded snapshots often lack sharing. Cached-entry count is bounded, but byte size is not constant.

## Normal-release measurements

Linux native, published source v17 versus this candidate, normal release without runtime timing instrumentation. A/B/B/A order, 20 samples per implementation per case; median uses the benchmark upper-middle order statistic for even N (not the arithmetic mean of two middle values), and p95 is nearest rank. Component times include the command and immediate save encoding, not durable flush, rendering or full Windows input-to-display latency.

| Case | Median before → after (ms) | p95 before → after (ms) |
|---|---:|---:|
| generated_10_floors-history-5-wait | 146.778 → 19.966 | 177.578 → 30.166 |
| generated_10_floors-history-5-pickup | 150.138 → 13.464 | 164.190 → 17.516 |
| legacy_40_floors_full_pack-history-5-wait | 565.618 → 30.651 | 781.099 → 38.542 |
| revert_completed_free | 27.342 → 17.446 | 32.936 → 19.489 |
| revert_pending_haste_free | 3.449 → 3.202 | 3.862 → 4.049 |
| future_creation | 16.794 → 8.668 | 19.659 → 11.137 |
| future_restore | 14.468 → 9.016 | 17.144 → 11.005 |

Work moved into commands: generated 10-floor/5-history Wait command median **2.560 → 8.513 ms**, while its immediate encode median **144.298 → 11.991 ms**. The inclusive combined median is **146.778 → 19.966 ms**. Do not add independently calculated medians to reconstruct the combined distribution. Pending-haste Revert p95 slightly regressed; this is disclosed rather than rounded away.

## Cold loaded history and durable flush

The first separate 10-sample cold five-frame run had encode median **167.608 → 149.545 ms**, but p95 **213.924 → 266.226 ms**. Inclusive durable flush median was **169.378 → 163.310 ms**, with p95 **397.826 → 2659.506 ms**. The original 2.66-second candidate stall is real measured evidence; its cause was not established.

A bounded supplemental normal-release A/B/B/A repeat, 20 samples each, did not reproduce that stall: cold encode median **170.447 → 138.309 ms**, p95 **220.372 → 176.427 ms**; durable flush median **171.257 → 140.201 ms**, p95 **223.477 → 167.752 ms**. This does not prove the rare tail is impossible or solved. Original and repeat samples remain in the evidence.

Loaded history followed by a new turn (setup/new-turn command excluded from this flush metric): final first-run durable flush median **186.738 → 39.784 ms**, p95 **256.358 → 45.840 ms**. A warm/fresh in-memory history result must not be represented as a cold-load result.

A syscall-tracing diagnostic was unavailable because this environment denied ptrace. No bypass was attempted; no syscall attribution can be made from that failed diagnostic.

The bounded in-process diagnostic then measured four cold samples per implementation. The slow candidate flush was **267.495 ms**, including **262.741 ms encoding**. Maximum candidate stage times in this batch were read 2.314 ms, write 1.773 ms, sync 0.0026 ms and rename 0.377 ms. This attributes only that new sample, not the old 2.66-second stall. All ten instrumented flush partitions and saved-byte assertions passed. Off/on controls were too small and noisy for a precise collector-overhead percentage. No further rare-tail chase was used to select favorable results.

## Complexity and remaining costs

See [save-rewind-export-complexity.md](save-rewind-export-complexity.md), [mutation-journal-foundation.md](mutation-journal-foundation.md), and [save-envelope-complexity.md](save-envelope-complexity.md) for named variables, transitive bounds, memory and fallback details. Supported write capture uses expected hash O(1) bookkeeping per access, plus O(block size) copy-on-write when required. Dirty-block serialization/diff and metadata traversal remain nonconstant. Revert parsing is O(encoded frame bytes), not O(1). Full current-state serialization and durable writing remain O(save bytes), plus validation and platform I/O latency.

Diagnostic generated 10-floor/5-history Wait recorded two projections, six old and six new block serializations, and 2699 intercepted mutable accesses. These are block/access counts, not scalar-write counts. Current-Run encode, dirty-block serialization and inverse diff remain significant. Collector-on/off n=3 overhead estimates are too noisy for a defensible percentage.

## Verification and limits

Full final opt1 workspace gate: **passed**, 143/143 executable targets complete. Debug assertions and overflow checks enabled; this is not a normal-release benchmark profile. **1021 tests passed, 0 failed, 7 ignored**. This is the final-source gate; ignored tests are not claimed passed.

- 22 saved-byte scenarios match published-v17 binaries exactly across all four measured runs.
- 66 diagnostic samples partition inclusive time exactly into exclusive scopes (no double counting).
- Focused tests cover free actions, pending haste, repeated Revert costs, Future, imported/rebased history, structural floor/actor/ground changes, null/missing, net-zero mutations, float spellings/typed float bits, alias-hint collisions, and immutable fallback.
- Storage tests retain corrupt/external-file preservation, failed-temp collision/retry, unchanged keys and terminal session rules.
- Workspace doctest command passed (zero doctests).
- Windows compact cross-build and package/static checks passed: ZIP 19,687,351 bytes; **Windows runtime has not been tested here**.
- Source API note: private runtime journal/cache fields prevent external Rust struct-literal construction of Snapshot/History. No in-project caller relied on that construction; the crate is not a published library API.
- Broad literally-every-function complexity audit remains separate/paused; this change documents its new and changed subsystem functions.

## Reproduction

The full source includes the benchmark examples and frozen previous-history oracle. Evidence contains raw normal samples, exact saved-byte hashes, instrumented scopes, test logs and code/build provenance. Use the pinned Rust toolchain and the recorded opt1 test profile; do not conflate normal-release benchmarks with instrumented/debug-check test timings. The reading-only code ZIP deliberately omits art and fixtures and is not buildable.
