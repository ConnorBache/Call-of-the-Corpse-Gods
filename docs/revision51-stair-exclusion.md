# Revision 51: creature and Mercuria stair exclusion

## Authored rule and compatibility

In revision 51, non-player, non-allied creatures cannot occupy either `up` or `down` tiles. Independent neutral creatures are included: otherwise a neutral could become hostile while already on stairs. Player movement, permanent allies and shared player bodies retain their prior stair behavior. Temporarily commanded creatures are excluded even while friendly, so expiration cannot strand a newly hostile creature on a stair; command duration, allegiance and displacement rules are unchanged. Revision 50 and older runs retain their prior movement, arrival ordering, flood spread and RNG behavior; no save revision is silently upgraded.

Stair followers finish their ordinary approach on an accessible neighboring source tile. Their final stair approach action still consumes its original turn, but no creature is placed on the source stair. At the destination, the eight adjacent candidates retain the existing deterministic hazard-first, direction-order preference. Passability, habitat, diagonal corners, grates, bulwarks, terrain modifications and portal alias occupancy still apply. The stair center is never a fallback. If no neighbor is legal, the creature remains at its source approach with its existing `chaseDepth`, exit and landing conditions, and retries on a later eligible turn. No parallel queue, new field, ID or RNG draw is introduced.

Mercuria's revision-aware flood treats stair tiles as barriers: it neither occupies nor propagates through them. The finite frontier/visited fields remain the existing serialized authority. Opening a wall/door or restoring terrain uses the same revision-aware admission. The old floor-only entry points are retained for frozen reference callers. Fish-spawn admission independently excludes stairs, even if supplied a stale or malformed ichor layer.

## Implementation audit

- Engine and travel actor passability enforce the same exclusion for ordinary, haste, confused, forced, flying, phasing and portal-outcome moves. Active and offscreen path masks avoid stair cells, including authored stairs temporarily covered by restorable terrain. This prevents a later terrain restoration from producing a stair underneath a phasing creature. The blocked-pursuit helper treats an enemy's known stair goal as a request to approach a legal adjacent square, not permission to enter stairs.
- Destination admission also covers short-range Moonstep/Beckon, amulet teleportation, Ahvamitt teleportation, combat pushes/Unstoppable Force and environmental/current displacement through their existing shared predicates. Forced falls have an explicit destination-cell guard.
- Ordinary and machine population construction rejects a stair spawn before IDs or generation RNG are spent. Existing generation samplers already choose legal floor/grass/water cells. Nest, special Godbeast and nightmare generation protect fixtures/stairs or select floor cells. No scripted stationary actor is removed or relocated.
- Witch and Pale Birther candidates explicitly exclude enemy stair births; allied offspring retain their friendly rules. Queen summons already allow only floor/grass. Enemy hallucination creation also rejects stairs before payment.
- Corpse revival is deferred on stairs in active, precise offscreen and aggregate offscreen schedules. A revived former ally becomes hostile, so the revival gate is independent of its prior allegiance. Corpses may remain where existing corpse-placement rules put them.
- Failed/deferred arrivals retain the existing serialized chase metadata. COW temporal snapshots and Future anchors retain the same floor frontier and actor metadata. Neither ability needs a second stair-specific state representation.

## Complexity

Let V be floor cells (bounded by 3,600), A living destination creatures, F stored floors, C actor condition count, P portal alias-query cost, M saved terrain modifications (each K≤9 cells), T temporary terrain records and H existing hazard/passability helper cost. Non-overlapping saved modification cells and temporary edits are bounded by V.

- Ordinary tile classification is O(1). Occupancy classification additionally scans saved restorable terrain metadata in O(M*K+T) worst case and uses O(log V) temporary key bytes for an altered-chasm map lookup. Unmodified floors take O(1). Run-floor lookup is O(F) in existing `floor()` access; the path-mask form receives its Floor and avoids a new lookup. Actor restriction reads fixed fields and a fixed number of condition-map keys.
- Existing path-mask construction gains O(V*(1+M*K+T)) worst-case work and no new retained memory. The movement field and blocked-pursuit graph retain their existing transitive costs; see `revision51-enemy-reroute.md`.
- Each transfer keeps O(A) destination occupancy setup and at most eight candidates, each with its existing O(H+P) admission. Sorting is bounded by eight elements. The source-edge test has the same O(M*K+T+H) standing-cell admission cost; the existing Run proxy/COW and actor-block mutations keep their prior clone/admission costs. Deferred arrival adds no queue or snapshot memory beyond existing O(C) chase fields.
- Flood initialization scans V cells and allocates existing O(V) visited/ichor layers. Subsequent spread is O(Q), Q the current frontier, with at most four neighbors each and O(Q) next-frontier memory. Stair rejection adds constant work per tested cell. Opening admission is O(1) beyond existing callers. Birth and revival gates use the same restorable-terrain scan as occupancy.
- No rendering/native-library changes. Complexity estimates are static analysis, not measured timings.

## Verification scope

Focused regressions live in `revision51_stairs_tests.rs` and `revision51_stair_arrival_tests.rs`. They cover both stair directions, ordinary/phase/flight/forced and neutral restrictions, preserved player/ally/legacy admission, path detours, constrained teleport and spawn cases, Mercuria barriers and blocked fish births, crowded/fully blocked/retried arrivals, source final-action timing, corner/stair rejection, offscreen revival, deterministic RNG and serialized/temporal state restoration. Reroute integration adds its own adjacent-goal/corner/habitat tests.

At authoring time these tests are written but not yet executed; parent integration owns the serialized Cargo test run. Full workspace and native Windows verification remain separate checks.
