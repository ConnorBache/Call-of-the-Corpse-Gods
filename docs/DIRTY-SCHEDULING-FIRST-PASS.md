# Dirty-state and scheduling cleanup: first bounded implementation

This document describes the first implementation batch on top of the r39 menu
and opt-in timing-harness baseline. It is **not** completion of the proposed
whole-turn scheduling migration or the paused every-function annotation audit.
Core dispatch measurements and exact canonical-state matches for eight tested
scenario/command combinations are recorded in the [timing report](DIRTY-SCHEDULING-TIMING-REPORT.md).
Final corrected core, isolated desktop, all-seeing and RGBA comparison evidence
is now recorded. Full workspace regression passed (142 targets; 974 passed,
0 failed, 1 ignored), and the standard feature-OFF Windows cross-build passed.
Packaging/publication remains separate; Windows runtime is not verified.

## Preserved behavior and boundaries

The aim is to avoid redundant derivation and lookup while preserving action
ordering, RNG consumption, world/haste/slow timing, save/replay compatibility,
physical/dream behavior, and offscreen catch-up. Existing actor AI, Minotaur
charge direction selection, damage formulas, environmental diffusion, and
source artwork/menu controls are not redesigned. The runtime timing feature
and its pre-optimization measurements remain available.

Derived caches are omitted from canonical serialization and ignored by gameplay
state equality. Their presence is not evidence that arbitrary mutations are
safe: the precise invalidation and fallback contracts below are part of each
optimization. Final verification must compare canonical state, not cache warmth.

## Implemented in this batch

### 1. Projected tile-lighting cache

Full terrain-visibility/observation passes reuse illumination fields based on
exact dependency projections. The two-plane bounded cache tracks opacity, fire,
ambient/static lighting, and relevant emitting/darkness actors. It preserves
source order and floating-point results; sparse actor lighting remains the
reference path, including Shine self-exclusion.

This is not an O(1) dirty flag. Each pass still captures/compares dependencies,
including V cells and relevant actors/sources. It removes repeated lighting
scans and ray work *per output tile*, not all repeated observation work.

- Validation: O(V + A*B + L), potentially two planes.
- Dirty build: O(V + sum of positive-source footprint*ray-length work +
  darkness-sources*V*ray-length); memory O(V + source count).
- Valid projected field read: O(1) per tile.
- A = actors, B = per-actor ability entries, L = fixed sources, V = floor cells.
- See [LIGHTING-CACHE-CONTRACT.md](LIGHTING-CACHE-CONTRACT.md).

### 2. Lazy path masks

The existing phase-local path cache previously built passability and hazard
masks before checking for a cached field. Masks are now built only for a new
field or the existing hazard-triggered rebuild. A hit still validates the
selected step against current hazards; the path key and existing rebuild rules
are preserved.

- Hit: original O(A) linear actor lookup + actor-copy cost + O(B log B) ability-key sorting
  and string construction + next-step checks.
- Miss/rebuild: additionally O(V*(allowed-cell helper + hazard-cell helper)),
  plus existing BFS/Dijkstra and spatial-link construction costs.
- Temporary field/mask storage: O(V + links).
- Cache lifetime remains one phase. This is not persistent global path caching
  and does not eliminate ability-key preparation.

### 3. Conservative maximum-HP dirty settlement

Unconditional early/final world-tick maximum-HP recomputations become flushes.
A fixed-size input stamp detects relevant scalar changes; known equipment,
crafting, potion and progression mutation boundaries still force settlement.
Admission invalidates once per command to cover arbitrary public inventory edits
between commands. Equipped item consumption also invalidates explicitly.

- Stamp comparison: O(1) time/space; no whole-inventory hash.
- Clean flush still checks the original Blood and Fury unlock side effect:
  O(B + H) worst case for B abilities and H achievements, plus logging only if
  actually unlocked. The *whole flush is not O(1)*.
- Dirty/forced settlement retains original effective-stat, equipment scan,
  HP-gain/clamp/loss behavior and its inclusive helper costs.
- This removes redundant **within-command** derivations, not every derivation
  on quiet commands. General equipment ownership and shared-body sync are not
  optimized by this HP-only subset.
- See [dirty-contract.md](dirty-contract.md) for mutation and side-effect detail.

### 4. Phase-aware Schedule kernel

The new deterministic generic kernel is keyed by deadline, phase, explicit
caller order and generation. Logical effect keys allow replacement/cancellation
without searching all queued entries. A stale generation handle cannot cancel
its replacement. Cancelled entries are physically removed, avoiding unbounded
stale-entry growth. Exact-deadline dispatch and explicit overdue dispatch are
separate APIs; caller phase semantics are not silently altered.

- N = live events, D = deadline buckets, K = events in a selected bucket.
- Schedule/cancel/pop: O(log N + log D + log K), plus payload costs.
- Absent exact-deadline lookup: O(log D), not an actor/event scan.
- Storage: O(N + D), plus payloads.
- Serialization is a deterministic ordered record list; deserialization rebuilds
  and validates indices in O(N log N + payload decoding).
- Independent floor clocks require separate queues; a key's scope is identity,
  not a dispatch-time clock filter.

### 5. Bulwark deadline-map migration

`Floor.bulwarks` retains its existing serialized ordered-map shape but gains a
strictly derived Schedule index. Every mutable map borrow invalidates before
returning the reference, including direct `get_mut`, entry, index mutation,
retention, renewal and removal. Clone preserves a warm valid schedule; load
rebuilds lazily. Expiration updates the queue directly and preserves key order.

- Warm unchanged `has_due`: O(1), apart from uncontended mutex access.
- First check after mutation: O(B log B + Z), B barriers and Z total key bytes.
- Due expiration: O(K log B + B + dropped-key bytes), K due events. One ordered
  retain avoids K independent vector shifts. No-due finite map: O(1).
- Storage: O(B + Z). A mutex maintains thread-safety; scheduling contention is
  not claimed to have constant wall time.
- Finite/fractional/signed-zero deadlines retain exact comparison semantics.
  NaN guard versus unconditional-retention behavior is also preserved for
  direct internal input, without relaxing existing save validation.
- Active/global and offscreen expiry paths use this cache. The outer **O(F)
  stored-floor loop still exists**; this is not yet a world-wide due-floor index.
- Offscreen admission checks that ask whether a barrier is still active are
  distinct and are not falsely counted as removed by expiry caching.

## Inclusive costs at changed aggregate boundaries

The following records cover changed callers as well as the new helpers. They
are additive source-derived contracts, not declarations that a large existing
function has become O(1). Internal branches can terminate early. A triggered
chain is charged for every event actually resolved; its callees are not hidden
inside a constant.

Additional symbols:

- U: bytes of non-shared owned Run/snapshot metadata actually cloned; F: stored
  floor/collection handles cloned or visited; COW: bytes detached on first writes.
- `T_validate`, `T_action`, `T_haste`, `T_finish`, `T_history`: the inclusive
  costs of the existing command validation/action/haste/finishing/history paths.
  Validation visits actor, condition, ability, item and offscreen support data;
  finishing includes pickup/doors, map visibility and resource-history scans.
  These are separately measured timing stages, not unknown constants assigned 1.
- `T_clocks`, `T_upkeep`, `T_AI`, `T_late`, `T_environment`, `T_offscreen`,
  `T_settle`: the inclusive existing tick phases. Upkeep still contains actor
  searches and condition/ability loops; AI still contains detection/path/action
  work; environment still has grid/actor/item scans and triggered event chains.
- `T_health_force`: owner cloning + Blood/Fury unlock hook + effective-stat and
  equipment derivation + resulting resource/achievement/log hooks.
- `T_health_flush`: O(1) stamp check + unlock hook when clean, or
  `T_health_force` when dirty. Unlock costs include B ability/H achievement
  scans and any actual log/archive append work, not merely one boolean check.
- `T_deadline(f)`: O(1) warm due test, plus O(B_f log B_f + Z_f) only when that
  floor's map is dirty, plus O(K_f log B_f + B_f + dropped key bytes) when it
  expires entries. B_f is that floor's barriers and K_f those due now.
- `T_light_check/build`: exact dependency projection and optional field build,
  defined above. `T_optical` includes portal-aware ray/route work, not just one
  arithmetic distance. `T_sense`, `T_metadata` and `T_disclosed_sources` include
  their actual actor/telepathy/hazard/topology scans and source-footprint output.

### `engine::dispatch`

`T_dispatch = T_validate + O(U + F) + T_action + h*T_haste + q*T_tick +
T_finish + T_history + COW`, along the selected command branch. Normal paid
commands typically use q=1; slowed paid commands may use q=2; free/haste-only
branches can use q=0 while still paying command-boundary work. The added health
invalidation is O(1) on the candidate, not a replacement for validation or the
transaction clone. Memory includes existing candidate/snapshots and copy-on-write
payload, plus O(1) health bookkeeping. Temporal wrapper costs remain outside
core dispatch where the existing call boundary puts them.

### `engine::turn`

`T_tick = T_clocks + T_upkeep + T_AI + T_late + T_environment + T_offscreen +
T_settle + sum_f T_deadline(f) + two T_health_flush + COW`, with the original
early terminal exits and event callback ordering. Here the named phase costs
exclude the newly separated final deadline and two player-health settlement
sites to avoid double counting them. Other mutation-boundary health calls inside
abilities/combat/status helpers remain part of those helpers. The final outer
floor loop is O(F) even if no barriers expire. No whole-tick asymptotic reduction
is claimed merely because two health derivations may be skipped. Existing actor
working lists, path fields, event/visibility/environment scratch remain.

### `engine::actor` and `engine::actor_mut`

Player lookup is O(1). NPC lookup remains O(A) first-match search. Mutable NPC
lookup additionally incurs the active actor block's COW clone if it is shared.
The attempted runtime actor index is **not integrated**. Comments documenting
these restored semantics do not imply a lookup optimization.

### `engine::path_step_cached` and its mask-building closure

`T_hit = O(A + J + ability-key sort/string bytes) + floor/flight/phase checks +
next-step/hazard checks`. Sorting B arbitrary strings includes their comparison
lengths; shorthand O(B log B) assumes bounded authored identifiers. A miss or
existing hazard disagreement additionally invokes `build_masks`:
`O(V*(allowed-cell cost + hazard-cell cost))`, O(V) masks, then existing field
construction (`O(V+E)` BFS or `O((V+E) log V)` weighted search, plus link
preparation and helper costs). E is effective graph edges, including links.
The closure is called only on those miss/rebuild branches. Existing cache key
hash/string allocation and actor cloning are not eliminated.

### `equipment::sync_health`, `equipment::flush_health`

Forced sync retains `T_health_force`, ordinarily including O(A+B+C+I+equipment
enhancements+owner copied bytes), plus the exact hook costs. Recording the clean
stamp adds O(1). Early unchanged-max returns still run the original unlock hook
and derivation on forced calls. Clean flush avoids derivation but keeps unlock
side effects; dirty flush delegates to the forced operation. Both use O(1)
additional cache metadata beyond existing derivation/clone/log temporary data.

### `inventory::take_one`

Health invalidation is O(1) when the selected item is marked equipped. Splitting
a stack clones the item's owned payload; removing the last unit shifts O(I)
remaining vector entries. Total is O(I + cloned item payload) as an upper bound,
not O(1) merely because invalidation is cheap. Consuming an equipped item then
appending a replacement at the same inventory length still marks health dirty.

### `clone_snapshot_state` and derived Run/Floor cloning

Snapshot clone retains O(U + F + other owned collection metadata) time and
corresponding owned-copy memory, with shared payloads cloned only when mutated
later. Added lighting-cache default is O(1) and starts cold; added health stamp
clone is O(1). A Floor COW clone already copies its authoritative barrier map;
cloning a warm DeadlineMap additionally copies its queue/index in O(B_f+Z_f)
time/space so repeated queries do not unnecessarily rebuild a cold schedule.
Derived cache contents do not enter canonical save equality or serialized bytes.

### `offscreen::catch_up`

Cost remains admission/dynamic checks plus the sum over elapsed catch-up steps:
`proxy clone + actor phase + currents/horns/Epidite/environment + feeding/doors +
world merge/living-ID rebuild + links/falls/transfers + dynamic checks +
T_deadline(f) + COW`. Each precise step may include O(W) all-world actor work.
Static approximation branches retain their own existing actor/stat/corpse costs.
The one changed deadline settlement is O(1) when warm and nothing is due;
mutation/due work follows `T_deadline`. The existing unconditional floor mutable
access at this catch-up location is preserved, so pre-existing COW work can still
occur. No claim that one cache removes all offscreen scans or clone work.

### `offscreen::tick_inactive_floors`

`T = all-floor admission/validation + sum_relevant T_catch_up +
transfers/followers + O(F) + sum_inactive T_deadline(f) + COW`.
Even when no elapsed ticks need simulation, admission and final floor iteration
still run. The final expiry path queries immutably, only taking a mutable COW
floor when a deadline is due. It preserves the old guarded-retain distinction
from unconditional catch-up retention, including edge-case NaN behavior.

### Changed vision projections

- `tile_chance_lit`: existing optical-route/probability operations; sparse local
  storage aside from route-helper allocation. Its two route calls are not merged.
- `tile_chance_projected`: plane/floor checks + O(1) projected light read +
  `tile_chance_lit`. It does not cache detection or sight.
- `tile_chance`: retains sparse cross-plane light computation where applicable,
  then the same `tile_chance_lit`; caller-specific exclusion behavior is preserved.
- `terrain_visibility` / `native_terrain_visibility`: viewpoint/sense setup +
  `T_light_check/build` + up to V cells times their sense checks and P optical
  tests. Worst-case negative visibility may test every viewpoint. Output O(V).
- `terrain_visibility_chance`: same setup/cache work, then every relevant
  viewpoint's chance calculation per cell, rather than short-circuiting at the
  first visible viewpoint. Output O(V).
- `observe_terrain`: same cache work plus V cells of P optical tests and
  hazard/topology/knowledge metadata, and disclosed-source footprint construction.
  Output/scratch includes cells and source footprints. Observation remains a
  separate pass and is not reduced to O(V) by illumination caching alone.

Every new helper/test in `turn_schedule.rs`, `deadline_map.rs`, `health_dirty.rs`
and `lighting_cache.rs` has its own local contract. Test bounds include fixture
construction, reference computations and serialization; a fixed fixture is not
evidence of O(1) gameplay. This changed-boundary documentation does **not** turn
the paused universal function audit into a completed ledger.

## Deferred, not implemented by this batch

These are follow-on parts of the requested cleanup, not silent changes or claims
of completion:

- Actor ID indexing. The attempted cache was removed before release: public
  same-length vector edits can introduce an earlier duplicate ID while leaving
  a cached matching position valid. The original first-match lookup semantics
  therefore require a mutation-tracked actor container before a safe O(1) index.
  Current actor lookup remains O(A); no general lookup speedup is claimed.
- Hunger thresholds and wake-state events.
- Remember and I Came For You expiry migrations.
- Equipment-granted ability ownership dirty tracking.
- Shared-body resource/projection dirty propagation.
- Epidite and Banshee periodic scheduling.
- Horn raised-wall scheduling and event-driven silence-aura membership.
- General early/late status, poison/bleeding, regeneration and power-upkeep
  scheduling/active registries.
- Constant-time active Riptide/Unspeak membership and cached upkeep price.
- Temporary-alliance and timed-flight/phase-loss event migration.
- Current-affected actor lists and feeding invalidation lists.
- Global due-floor/barrier index and collapse polling removal.
- NPC order/perception redesign, Minotaur targeting simplification, and changes
  to offscreen simulation semantics or environmental active-region algorithms.

Any later migration must preserve phase order, renew/cancel/death/revival
behavior, offscreen clocks, save/load and Revert/Future restoration. A due event
still pays its actual effect cost; callbacks are not declared O(1) merely because
a queue found them without scanning actors.

## Verification and measurement status

Focused module evidence includes scheduler cancellation/order/serde tests,
map mutation and exact legacy expiry comparisons,
maximum-HP mutation-boundary checks, and lighting reference bit equality with
direct same-tick changes and clone/load cases. These are narrow checks, not a
whole-game correctness proof.

Core measurement evidence now recorded: three alternating paired batches, 90
feature-OFF latency samples per version/scenario/command, and exact canonical
state matches for all eight combinations in every batch. See the
[timing report](DIRTY-SCHEDULING-TIMING-REPORT.md) for medians, p95s, batch ranges,
and exclusions. Integration status:

1. Final combined full-workspace tests passed: 142 targets, 974 passed,
   zero failed, one ignored; process exit 0. See final-validation.json and the
   full-test.log included with the timing evidence bundle.
2. Final measured state proof records 48 core and two all-seeing exact byte
   comparisons. The full suite adds regression coverage; this is not a proof of
   every possible replay/time-power/offscreen sequence.
3. Final core, isolated desktop, all-seeing and RGBA checks are recorded in
   the timing report. Final per-phase profiling attribution is not established;
   do not mix feature-OFF timings with instrumented counters or overlapping spans.
4. Standard feature-OFF Windows cross-build passed. Source/archive preservation
   and package/publication checks are separate delivery steps. Windows runtime
   verification remains unavailable; cross-compilation is not a runtime test.

Measured improvement percentages apply only to the timing report’s tested core
workloads. Neither a finished audit nor a broad event-scheduler migration is
established. Update remaining gates with verified evidence before describing
the release as fully validated.
