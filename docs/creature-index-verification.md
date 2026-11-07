# Creature index verification and measurements

Baseline: released source v15 / standard Windows v11. Candidate adds indexed creature lookup, reusable floor-local handles, collision admission, and the separately authorized possible-exit hover early guard. Deep profiling instrumentation remains a separate diagnostic, not part of this release.

## Regression status

**994 tests passed across 142 workspace/doc-test targets; two diagnostic tests intentionally ignored.** The no-fail-fast workspace run initially reported five failures in one portal-test builder: it copied the same creature ID onto floor 10 while retaining the original on floor 1. The builder now moves that creature instead, preserving its ID and intended portal assertions. The corrected target was rerun: all 8 tests passed. The other 141 targets passed in the full run. No JSON fixture or production ID was renumbered; no uniqueness checks were weakened.

The logs retain both the original malformed-fixture failure and the successful isolated retest rather than hiding that history. Subsequent source edits only clarified complexity comments; the corrected target and final Windows build compile the current source. New tests cover 100k trie capacity, sparse expansion, duplicates/removal, snapshots, floor transfer/reuse and generation/epoch checks, public same-length mutation, nested pilots, safe-ID exhaustion, single/twin-birth collision rejection, full command/RNG rollback, and normal-command handle stability.

The 15 focused non-Euclidean/hover-related tests passed, including all three new sequencing tests. The early guard avoids terrain observation when the possible-exit overlay cannot apply; relevant cases retain the same knowledge gates. The detailed proof is in `hover-exit-sequencing-review.md`.

Standard Windows release built with optional timing instrumentation disabled. PE32+ x86-64 imports match the previous distribution; every unchanged asset and DLL was byte-verified. This is a cross-build/static packaging check, **not a Windows runtime playtest**.

## Actual command measurements

Three alternating baseline/candidate batches, 30 measured samples each after three warmups: 90 collector-disabled samples per version/scenario. Same retained feature-off build profile: Rust 1.99.1, size optimization, fat LTO, one codegen unit. All eight resulting canonical states matched exactly in every batch. Setup/generation, state cloning/admission, and final save comparison are outside the measured command interval as specified by `examples/timing_test.rs`. This core comparison predates the desktop-only hover guard and is unaffected by it.

| Scenario/action | Baseline median ms | Indexed median ms | Baseline p95 ms | Indexed p95 ms |
|---|---:|---:|---:|---:|
| generated-entry / wait | 2.899 | 2.786 | 4.012 | 4.148 |
| generated-entry / pickup | 1.744 | 1.658 | 3.621 | 2.755 |
| synthetic-no-actors / wait | 2.068 | 2.122 | 3.305 | 3.628 |
| synthetic-no-actors / pickup | 1.474 | 1.481 | 2.520 | 2.215 |
| synthetic-crowded-plus32rats / wait | 6.291 | 6.054 | 9.105 | 9.671 |
| synthetic-crowded-plus32rats / pickup | 2.594 | 2.701 | 3.272 | 5.003 |
| synthetic-three-involved / wait | 5.343 | 5.301 | 6.640 | 6.782 |
| synthetic-three-involved / pickup | 1.643 | 1.651 | 2.029 | 2.112 |

The present small-game command improvement is modest and mixed, with batch variation. Ordinary/crowded Wait medians are about 4% lower, while some no-turn/empty-actor cases are slightly slower. Do not treat these measurements as a universal speedup or evidence that lookup dominated existing turns. Other visibility, lighting, save and rendering work remains.

## Integrated index capacity, memory and lookup

Release-mode isolated synthetic worlds; no AI/turn simulation. Actor counts below are NPCs **plus one separately stored player**. The retained allocation delta excludes canonical actor vectors, real actor payload allocations, allocator bookkeeping and RSS. Lookups use deterministic scattered IDs and include equality assertions; linear lookup retains the previous floor-vector search. First-index construction is separately measured.

| NPCs | First index build ms | Additional retained index bytes | Indexed lookup ns | Linear lookup ns |
|---:|---:|---:|---:|---:|
| 32 | 0.022 | 8,836 | 33.4 | 75.7 |
| 100 | 0.028 | 19,012 | 32.8 | 85.1 |
| 1,000 | 0.232 | 148,612 | 35.1 | 431.6 |
| 10,000 | 3.305 | 1,444,612 | 55.9 | 3164.6 |
| 100,000 | 25.867 | 14,405,476 | 242.6 | 100697.3 |

`Actor` itself is 736 bytes in this build before owned strings/conditions/abilities: 100,000 default actors already require about 73.6MB of canonical vector storage. The ~14.4MB index at that capacity is additional. Real-world memory depends on actor payloads, floor-slot high-water marks and shared temporal snapshots. A 100k lookup test is not a 100k-enemy gameplay throughput guarantee.

## Why not blindly use 100-wide branches?

A separate matched persistent-trie microbenchmark (u64 values, two runs) found radix 100 faster and slightly smaller for dense IDs: dense 10k~5.4ns/163KB versus radix 10~13.3ns/196KB. Dense100k~9.2ns/1.63MB versus~13.6ns/1.96MB. However,100k sparse IDs spread through 9.7 million values used 158.3 MB at radix 100 versus 36.6 MB at radix 10, and lookup was~77ns versus~32ns. These figures are not whole-index/game measurements.

Because existing IDs share a lifetime counter with items and may be sparse, the global index keeps the smaller lazy decimal branching.100×100 remains a reasonable dense-table tradeoff, not an automatic improvement. No new hard 10k/100k cap was introduced.

## Actual radius-five explosion, cold index

Two opposite-order batches of 30 measured samples after three warmups per version/scenario; feature-off builds. The same JSON input is freshly cloned/admitted for each sample, so the indexed version pays its initial registry construction **inside** the explosion interval. Every sample asserts positive damage; all resulting canonical states match baseline. Ignition is disabled; no world turn or complete fire/gas explosion-chain closure is measured.

| Current-floor NPCs | Baseline median ms | Indexed cold median ms |
|---:|---:|---:|
| 20 | 0.237 | 0.300 |
| 52 | 0.574 | 0.634 |
| 148 | 3.069 | 3.142 |

This cold-path comparison adds roughly 0.06–0.07ms; it is **not** an explosion speedup. A warmed in-game registry avoids initial construction, but no warmed explosion timing is claimed here. Radius/victim/visibility/damage work still dominates the larger case.

## Operational limitations

* Clean failed lookups are not free: they perform numeric validation, dirty/mutex checks and allocated-prefix traversal, O(d), but no actor/floor scan. Costs assume the existing single simulation thread and uncontended cache mutex.
* Dirty structural reconciliation is O((A+P+B)d+F+V), not O(1). Repeated spawn/removal can rebuild repeatedly; this release does not implement a fully incremental actor store.
* Floor-slot generations retain high-water history. Rewind/load/independent clones require handle reacquisition; normal command commits preserve handles for creatures that remain on their floor.
* Valid generated runs retain the existing monotonic allocator. Boundary checks cover current creatures and mounted pilots, not an exhaustive historical/deleted-ID audit of manually edited saves. Low legacy cursors are not silently rewritten; actual birth collisions/unsafe exhaustion are rejected.
* Player-mounted-pilot payloads are unsupported and rejected; ordinary NPC-mounted pilots retain identity but are not exposed as active targets.
* The universal all-function complexity audit is still incomplete. Every newly introduced index algorithm/access boundary is annotated; existing project systems are not claimed comprehensively re-audited.
