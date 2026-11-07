# Creature identities and floor-local lookup

Status and measured results: see [creature-index-verification.md](creature-index-verification.md).

## Invariants and compatibility

* Stable creature IDs remain the existing JSON numbers. Player is normally `0`; the existing shared creature/item `nextId` allocator and deterministic vector/action order are unchanged. Offscreen proxy player `-1` is internal-only and is excluded from the creature trie.
* `00000` is display padding, not a different identity. Five decimal positions cover 100,000 IDs including zero. The sparse trie expands beyond five digits: existing large IDs are not truncated or renumbered. Valid wire IDs are nonnegative exact integers through 2^53−1. Capacity is not a promise to simulate 100,000 enemies quickly.
* The trie rejects duplicate insertion. Save and command admission reject duplicate, fractional, negative or unsafe creature identities, including mounted inventor pilots. A pilot reserves its ID while inside a mech but is deliberately not targetable through ordinary actor lookup.
* Low historical allocation cursors are not silently normalized and are not alone a reason to reject a unique world. Actual birth collisions and unsafe counter exhaustion are rejected; successful allocations retain original IDs and RNG order. Item identity rules are not redesigned by this creature-index change.
* `Run.actors` retains its original serialized map-of-vectors shape. It is wrapped in `ActorFloors`: public mutable map/vector access invalidates derived metadata before exposing the storage. Same-length replacements and ID edits cannot leave trusted cached locations behind. Internal property-only mutation uses a separately named, crate-private path. New code must not use that path to change IDs, pilots, vector order or length.
* Public raw mutations are editing operations, not admitted worlds. Invalid edits yield lookup failure and a clear admission error; no duplicate silently overwrites a prior trie entry.

## Floor-local IDs

`Run::creature_floor_id(global_id)` returns the floor name plus a local handle. `Run::creature_from_floor_id(floor, handle)` resolves it. The player is included. Dead actors/corpses remain while they remain in the canonical actor vector. Mounted pilots are reserved identities but not active floor occupants.

A creature staying on one floor keeps its slot. Structural reconciliation vacates departed slots before allocating arrivals. Reuse changes the slot generation; stale handles do not resolve to a new occupant. A return to a floor can receive a different slot. These slots never reorder the canonical vectors.

Handles additionally contain a process-local epoch. Load, independent clone and time-travel restore require reacquisition; ordinary atomic command commits preserve the original epoch. Offscreen proxy commits preserve the live world's slot history. Handles are not serialized or used by existing delayed gameplay effects. A future scheduler must store stable global IDs or explicitly restore/reacquire local handles; it must not persist a bare reusable slot.

## Complexity (not timings)

Variables: **A** current active stored creatures including player; **P** nested pilots; **B** prior indexed creatures; **F** floor records retained by the index; **d** decimal digits (5–16 for wire IDs); **H** sum of floor-slot high-water marks; **V** free-slot entries copied on first mutation after a shared snapshot; **C** actor payload copied by existing Arc copy-on-write.

| Operation | Time | Extra memory / caveat |
|---|---|---|
| Clean stable-ID lookup | O(d), independent of A | O(1); no floor scan or per-hit heap allocation |
| Dirty reconciliation | O((A+P+B)d + F + V) | O((A+P)d + F), persistent historical slots retained |
| Public mutation invalidation | O(1) | Next indexed access pays reconciliation |
| Mutable indexed actor access | O(d + C) | C=0 if floor vector uniquely owned; otherwise existing COW payload |
| Floor-slot lookup | O(d) plus expected O(1) floor map access | Generation and epoch checks are O(1) |
| Slot allocate/release | O(d), plus O(V) first shared free-list write | Slots retain generation history, O(Hd) retained storage |
| Actor-floor snapshot | O(F) | Arc vectors and trie roots shared; fresh handle epoch |
| Trie snapshot | O(1) | Later mutation copies only affected path |
| Validating a birth batch of k | O(kd), plus pending reconciliation | No RNG or identity is consumed by preflight |
| Save identity validation | Expected O(A+P) | O(A+P) HashSet, separate from full save validation |
| Serialize actors | Unchanged: O(total serialized actor payload) | Runtime indexes/handles omitted |
| Iterate actors / act every enemy | Still O(A) before per-actor work | This change does not make AI, visibility or history matching constant-time |

Overall retained index memory depends on current creatures **and** floor-slot high-water history, not only living enemies. Cloned temporal snapshots retain shared old index nodes. Structural churn can cause repeated rebuilds; a spawn-heavy turn can still pay repeated O(A d) reconciliation. This deliberately bounded integration does not claim fully incremental structural updates or removal of every linear search.

## Audited mutation paths

Creation, death/corpse removal, travel, dream bodies, Half Here/Mini-Me, inventor boarding/ejection, direct map replacement, and public vector mutation invalidate the index. Existing actor order, RNG order, same-tile multiple bodies, corpse lifetime and offscreen simulation behavior are retained. Property-only hot paths (feeding, silence/riptide cleanup, Half Here synchronization, corpse burning, stair-follow flags and maintained links) use audited identity-preserving access.

All new algorithms and access boundaries carry source complexity comments. `creature_trie.rs` contains allocation, pruning, persistence, generation and 100k-capacity tests. `creature_index.rs` covers public mutation, floor transfer/reuse, stale handles, save shape and nested pilots. Measured results and verification limits are in `creature-index-verification.md`.
