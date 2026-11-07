# Save/rewind: staged write-time COW journal

**Scope:** this is a coarse first-write journal plus selective dirty-block diff, not a complete field-write journal. It captures mutable access to physical/dream floors, per-floor actor vectors and per-floor ground vectors. Direct player/scalar/inventory metadata, structural replacement, imported histories and temporal rebasing still require explicitly counted fallback work. The original immutable snapshots remain the safety reference; no untracked mutation is silently considered unchanged.

No save cadence, envelope version, simulation revision, disk validation, archive rule, fsync or rename behavior is changed. The current full Run is still serialized into every actual save. This does not make durable saving O(number of writes).

## What now happens

1. Before ordinary command execution, capture the existing opening snapshot and begin a thread-local journal scope.
2. Before an intercepted `Arc::make_mut` returns a mutable reference, retain its first old Arc. Actual field writes happen afterward. Repeated accesses coalesce using nonauthoritative allocation hints; the old original Arc is the only authoritative lookup key. No JSON is produced here.
3. At command completion, merge the evidence into the correct completed/free/pending-haste interval. Retain only old roots actually belonging to its opening snapshot; discard incidental probe/transaction-clone roots and allocation hints.
4. Seal the completed edge **during dispatch**. Compare shared block handles, skip immutable identical blocks, serialize changed old/new blocks and the ordinary small metadata header, then emit the original selective leaf patches. The retained journal supplies an old block only when its Arc exactly matches the opening snapshot. Otherwise use that snapshot and count `history.journal_snapshot_fallback`.
5. Store the exact encoded inverse frame in the bounded runtime edge cache. Later saves reuse it where both endpoint identities and archive cursors match. Serialize the current Run directly inside the unchanged typed desktop envelope.

Free actions extend the last completed edge; pending haste keeps a separate opening and accumulated evidence. The oldest opening has no retained incoming edge and drops that evidence. Revert/Future/rebase retain their original selective semantics; journal misses after replacement/import safely take the snapshot fallback. DEBUG loadout synchronization still uses its existing revision rules. No runtime coverage error is raised after mutating a command, because that would require a separately proven rollback contract.

## Why leaf patches still matter

Capturing an old whole actor/floor is safe storage; restoring the whole object is not equivalent. Existing Revert deliberately restores only the paths changed by that transition so retained costs/resource telemetry on untouched paths survive. The original recursive walker remains responsible for net-zero pruning, ordered paths, Missing versus Null, array lengths, scalar-array compactness, visible log/logKinds and archive prefix lengths. This stage does not claim to remove that within-dirty-block work.

## Complexity parameters

H≤5 completed frames; C≤5 cached strings; F physical/dream floor handles; G actor/ground groups; M metadata bytes including player/inventory/Future anchor strings; D bytes in changed or unshared COW blocks; L maximum nesting/path depth; P emitted patch bytes; J compactness-candidate serialization bytes; E final envelope bytes; K captured journal roots.

| Function/boundary | Time | Extra memory / caveat |
|---|---|---|
| `mutation_journal::make_mut` | Expected O(1) journal lookup plus ordinary COW O(block size) when shared | First old Arc retained; first capture may cause a COW clone. A mutable borrow that makes no actual change is a false positive, pruned later. |
| `Journal::merge`, `retain_opening` | Expected O(K + F + G), plus destructor work for discarded uniquely owned payloads | O(K + F + G) root bookkeeping; payloads share immutable Arcs. |
| `inverse_header` | O(M + F + G) | O(M + F + G); dream-plane creation/removal retains complete layer data for structural patches. A large Future anchor is **not** small metadata. |
| `projected_array`, `projected_map` | Expected O(F + G + D) | O(F + G + D); shared blocks use equal null sentinels. Only structured floor arrays are masked, never scalar arrays with compactness selection. |
| `journal_old_value`, `journal_new_value` | O(block bytes), old lookup expected O(1) | O(block JSON). Missing evidence is an explicit snapshot fallback, not an omitted delta. |
| `inverse_pair_values` | O(M + F + G + D) | O(M + F + G + D) for projected endpoints. |
| `inverse_values` / existing walker | Conservative O((M+D)×L + P + J) | O(P+L) beyond endpoint Values. Nested equality can revisit descendants; paths and compaction serialization count. |
| `History::seal_current` | Above projection/diff plus encoded frame bytes, or O(C) exact cache hit | Per-command cost, included in command timing; not hidden in a save-only benchmark. |
| `History::export` | O(Run clone + H×C + history bytes), plus missing-edge projection/diff | At most C encoded strings retained. Entry count is bounded, **not bytes**; large patches/anchors remain large. |
| Desktop `encode` | O(export + E) | O(export scratch + E), without the old text→Value→text round trip. |
| Durable flush | Existing read/validation + encode + O(E) write + sync/rename latency | File I/O latency is not a CPU big-O bound. No checks or durability steps removed. |

Worst case D can be the entire world: every block may change, or imported snapshots may have equal values but independently allocated Arcs. This stage is not an O(changed scalar fields) guarantee. Capturing original COW blocks and serializing only dirty blocks is the bounded foundation; finer typed field writers would be a later, separately audited step.

## Safety and evidence

- Exact retained original Arc identity prevents allocator address reuse from returning a wrong old value. Alias hints can only suppress a capture; missing authoritative evidence falls back to the opening snapshot. An injected collision test proves this behavior.
- Snapshots and edge-cache entries are runtime-only. Valid legacy save loading and original JSON frame spelling/order remain unchanged.
- A frozen independent inverse implementation is used in structural and real-command tests, including multiple Reverts. Existing temporal traces cover free actions, pending haste, Future, retained costs and resumption.
- Benchmarks report command, immediate save and **command+save** separately, with byte/state checks outside timing. Ordinary wall time is collected with timing disabled; instrumentation reports counts/exclusive costs separately.
- Full suite, Windows build and normal quiet benchmarks are separate completion gates; focused tests alone are not a release claim.

See [mutation-journal-foundation.md](mutation-journal-foundation.md) and [save-envelope-complexity.md](save-envelope-complexity.md).

## Recorded inverse consumption (second bounded step)

`History::recorded_inverse` lets Revert consume an already sealed edge only when both immutable endpoint identities and both archive cursors match. It parses that frame's single net-patch group, O(encoded frame bytes) time/memory, instead of recomputing its dirty-block difference. A missing/mismatched entry takes the same selective `inverse` fallback. The parser may still scan a large embedded Future string; it is not a constant-time lookup-to-undo guarantee. No new persistent patch representation is retained alongside the encoded cache strings.

Hit-versus-forced-miss tests cover every existing temporal command trace, reloads, free/pending-haste boundaries and repeated Revert costs. A number-spelling regression compares exact typed-state output/float bits after cached versus freshly computed patch application (including signed zero, a nextafter-like fractional value and large/small finite values). The first journal-only benchmark remains preserved as stage1; results after this Revert reuse must be labeled stage2.
