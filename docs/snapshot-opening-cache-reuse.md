# Exact opening identity reuse

This cache-only change preserves gameplay, save schema, legacy inverse bytes and durability. It does not introduce incremental storage or a complete mutation journal.

## Redundancy removed

The old dispatch captured a fresh opening `Arc<Run>` even when it exactly represented the previous closing boundary. An already sealed inverse edge therefore missed its identity key on the next export. Additionally, `seal_current` evicted cache index zero at capacity, but export stores newest-first; that could evict the immediately preceding still-retained edge.

The candidate reuses a closing identity only after an exact projected guard, and prunes sealed entries by membership in the currently retained transitions rather than assuming a vector index means oldest. Cache size remains at most five.

## Safety guard

`Engine.run` remains public. Desktop DEBUG loadout calls and `ensure_run_id` are examples of writes outside `Engine::dispatch`; tests and library callers can perform arbitrary other writes. Therefore no “dispatch was the last writer” assumption is used.

After ordinary dispatch confirms that a tick completed, compare its immutable pre-command capture and attempt reuse only when no pending-haste opening exists and the exact preceding edge is already sealed. Free commands do not pay this guard. Then require:

1. Equal archive cursors.
2. Equal ordered physical/dream floor Arc identities.
3. Equal ordered actor/ground map keys and Arc identities.
4. Exact equality of the Future `Option<String>`, including None versus Some(empty).
5. Equal serialized metadata bytes after the identity-checked blocks and already-compared Future text are removed from temporary headers.

The metadata comparison includes every other snapshot-owned field, including RNG, scalar/player/inventory state, logs, structural dream metadata and ordered map contents. Future anchors are compared directly as exact text rather than escaped into two guard JSON buffers; actual inverse/save serialization is unchanged. Comparing bytes rather than map equality retains ordering and floating spelling. Other Field Missing/Null/Value distinctions remain in the serialized header. Snapshot-excluded replay/history/run identity/archive payloads retain their existing exclusion contract; archive cursor counts are checked separately.

Any mismatch retains the fresh opening. Different COW identities conservatively miss even when their values happen to be equal. No hash or unverified alias can create a false hit. Pending haste uses its existing authoritative opening and bypasses this redundant guard; temporal branch ownership/rebase is unchanged. Imported/rebased histories with no matching cached seal likewise bypass the guard.

## Complexity (not measured timings)

Let R be physical/dream floor handles plus actor/ground map entries, K their key bytes, M all remaining snapshot metadata bytes including Future text, H retained frames and C cache entries (H,C <= 5 normally).

- `same_arc_array`: O(R) time, O(1) scratch, no payload equality.
- `same_arc_map`: O(R + K) time, O(1) scratch, order-sensitive.
- `snapshot_header`: O(R + M) handle/metadata clone work and scratch, including dropping copied handles. Large optimized COW payloads are not copied.
- `Snapshot::same_boundary`: O(R + K + M) time; two temporary metadata clones and serialized byte buffers (Future still copies in header clones before being cleared, but is not escaped/serialized there), O(R + M) scratch. M may be large: Future anchors or other metadata are **not** constant-size.
- Completed-turn dispatch adds O(C) cache eligibility checks and, when eligible, the guard above. Free/pending commands do not add the metadata guard. Existing opening snapshot capture still occurs. The guard is not O(1), and its cost must be compared with the avoided earlier-edge projection/diff/encoding.
- `History::retains_edge`: O(H) exact identity/cursor comparisons, O(1) scratch.
- `seal_current`: existing missing-edge construction plus O(H*C) retained-edge pruning; cache hits remain O(C). Shared cloned timelines may evict each other's entries without changing results.
- Export and Revert continue exact identity/cursor lookup. Actual prior-edge cache hits, guard cost, command latency and save latency need normal-build before/after measurement; moving work between phases is not a speedup claim.

## Verification

Focused tests cover root/key ordering, conservative equal-value/new-identity misses, player edits, signed zero, metadata map reordering, dream state/roots, ground changes and archive cursors. A real command test proves a prior sealed edge survives the next completed turn, free-action closing identity is reused, and a direct external player edit forces a fresh opening. Existing exact legacy-byte tests and all temporal traces remain required for haste, Future, Revert, DEBUG external edits and reload. Runtime timing labels expose guard work and successful reuse separately.
