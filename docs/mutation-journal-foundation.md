# First-write COW journal foundation (staged, incomplete)

`mutation_journal::make_mut` intercepts mutable access to `Arc<Floor>`, `Arc<Vec<Actor>>`, and `Arc<Vec<GroundItem>>`. On the first covered access it retains the old typed Arc before COW occurs. It does not serialize JSON or reconstruct a world diff. Other concrete Arc payload types pass through without capture; derived generic creature tries deliberately retain ordinary `Arc::make_mut` and are outside canonical journal coverage.

This is **first-write block evidence**, not yet a complete scalar mutation log. Direct scalar/player/inventory assignments and whole-block replacements are not intercepted by this module. Selective changes within a captured block still require the consumer's typed/dirty-component finalization. The immutable opening snapshot remains an explicit fallback for uncaptured roots.

## API and scope

- `begin() -> Scope` creates a thread-local nested transaction scope.
- `Scope::finish() -> Journal` returns evidence and drops post-COW address hints.
- Dropping a scope without finishing discards evidence, including on errors/unwind. Scope is thread-bound and cannot be sent between threads.
- `Journal::old_floor`, `old_actors`, and `old_ground` return a borrowed retained Arc **only** when its identity exactly matches the requested opening Arc.
- `merge` unions original keys; it does not substitute intermediate values for an earlier opening.
- `retain_opening(&Run)` prunes everything except physical/dream floor roots and actor/ground groups belonging to that exact opening. Consumers must call this after merging free/haste evidence to avoid accumulating intermediate/probe payloads.
- `counters()` reports historical supported mutable accesses, capture events and coalesced accesses. These are not counts of scalar writes or proof of complete coverage. `retained_blocks()` reports current evidence roots.

Post-COW addresses are nonauthoritative coalescing hints. They own neither strong nor Weak references, avoiding an artificial COW clone/allocation on every repeated access. An allocator-reused hint can suppress a capture. That can cause only **missing evidence**, never an incorrect old value: retained original allocations are protected by strong Arcs and exact pointer checks. The consumer MUST use the opening snapshot and increment an explicit fallback metric when exact evidence is absent; it must never interpret absence as unchanged. An injected hint-collision test verifies this contract.

No RefCell borrow is held during `Arc::make_mut`/payload cloning. Thread-local scopes isolate different execution threads. Nested scopes own separate evidence; callers merge committed children explicitly. Discarded probe writes can produce conservative capture events within a scope, but opening-root pruning removes irrelevant retained allocations.

## Complexity

Let W be supported mutable accesses, K captured roots, A address hints, R opening root handles, and C the size of a payload cloned on a COW miss. Capture/coalescing is expected O(1) per access and O(K+A) metadata, plus retained old payload memory. The first capture creates a strong old owner and may cause O(C) COW cloning; subsequent access does not add an old owner. Existing real snapshot/branch sharing may still force additional legitimate COW clones.

Scope creation is amortized O(1). Finish is O(nesting depth + A); normal LIFO removal is O(1) before hint cleanup. Merge is expected O(K+A) for incoming entries. Opening pruning is expected O(R+K), O(R) scratch, **plus destruction of uniquely owned discarded payloads**. Exact old-root lookup is expected O(1). These are allocation/hash bounds, not guarantees about allocator latency. The journal is bounded by actual transaction accesses/root captures, not by a fixed global limit. Frame-level retention requires the explicit prune step.

## Tests supplied

First-old capture, repeated-access pointer stability, net-zero writes retaining correct evidence, inactive/unsupported no-forced-COW, nested scope/drop/merge, cloned branches and alias chains, injected alias collision with correct immutable fallback, and pruning transient probes while keeping historical capture counts. Tests require execution in the integrated branch; compilation alone is not a behavioral pass.
