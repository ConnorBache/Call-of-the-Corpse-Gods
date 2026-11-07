# Exact optical/actor visibility batching (October 31, 2026)

This implements intrinsic **blocks line of sight** tile masks and shared exact
rays, not a new shadowcasting visibility rule. Wall, tree and closed-door opacity
is captured once per distinct floor in each immutable bulk pass. Destination
opacity is ignored for direct sight, as before; portal transfer endpoints must
both be open. Rays are directional: no reverse-ray symmetry is assumed.

## Order and costs

N = floor cells; K = portal endpoints (at most 14); R = maximum ray length;
P = viewpoints; G = distinct viewed floors (at most two); A = floor actors;
B = ability entries; L = placed light sources; F = existing floor-selection cost.

1. Collect viewpoints, preserving player/clone/perfect-ally order: O(A B).
2. Capture exact opacity and portal-pair dependencies: O(G(N+K)) time/space.
   Public same-tick edits are supported. This is real guard work, not free.
3. On geometry change only, compute directed endpoint-to-tile clear distances:
   O(G K N R) time, O(G K N) memory. No all-tile graph is constructed.
4. For a fresh observer origin, Dijkstra over portal endpoints costs O(K²+KR).
   Combine its endpoint distances with exact origin-to-tile rays and cached
   endpoint-to-tile distances: O(N(R+K)), O(N) output. No RNG is used.
5. Retain up to eight origin fields per geometry. Same-origin reuse scans at most
   eight keys, O(1) under this explicit bound. A teleport to an uncached origin
   pays step 4 but not step 3 when geometry is unchanged. Opacity/portal edits
   invalidate the geometry and its origins. At most two geometries are retained.
6. Per target/viewer probability uses cached distance and light, expected O(F+B)
   including existing plane/floor selection. Viewer hash worst-case adds O(P).
   For a bulk map, this is O(N P (F+B)) after setup, plus unchanged telepathy
   checks/classification. The prior per-target O(K² R) portal searches are gone.
7. Actor batches use a borrowed VisibilityProjection. Lighting/geometry initialize
   lazily on the first optical query, so reveal/telepathy-only and empty batches
   do not build fields. Each lazy check still costs O(1). Viewpoint collection is
   not free. Excluded own Shine uses the exact top-two source field documented in
   [ACTOR-LIGHT-CACHE.md](ACTOR-LIGHT-CACHE.md).

## What remains scalar

One-off AI/event queries still use sparse light computation and a scalar portal
route. Their duplicate distance/existence route search is removed, and no-portal
queries bypass graph allocation. A persistent tick-only cache would be wrong:
actors, sources, doors and planes can change multiple times within a turn. Bulk
resource history and desktop actor observations use one immutable projection per
batch instead. No effective-stat snapshot is reused across gameplay mutations.

## Exactness boundaries

Portal distance is the minimum **complete route length**, summing all ordinary
segments and zero-cost transfers. It is not the minimum of its two legs. Multiple
portal chains, blocked endpoints, directed raster ties and opaque destination
visibility are retained. Distance-only fields do not replace public route point
sequences, whose tie ordering stays on the original route function.

A removed wall can affect many target rays, not merely one. This implementation
rebuilds the affected geometry field conservatively; it does not claim per-ray
incremental invalidation or outward shadowcasting. Such changes need a separate
semantic equivalence proof (ordinary shadowcasting differs at corners/ties).
Caches are serde-skipped, ignored in Run equality and reset on clone/snapshot,
load and temporal restore; no save schema, IDs, menu assets or RNG rule changes.

## Verification

Focused lighting tests compare exact floating-point bits against the original
sparse implementation, including active/inactive own Shine, multiple owners,
darkness order, planes and direct same-tick edits. Geometry tests compare every
origin/target pair on deterministic blocker maps with 0/1/3/7 portal pairs against
the original directed route solver. Projected actor visibility is compared to
scalar visibility over 16 maps, 33 actors and nine mutation phases. Full-suite
and measured performance results belong in the release report; no millisecond
claim follows merely from these asymptotic bounds.

### Scalar mid-turn follow-through

Scalar illumination still scans A actors, but now rejects distant Shine sources
by integer-coordinate Chebyshev range before inspecting abilities/body/plane;
darkness uses its original fractional Manhattan range before those gates. Cost
is O(A + A_near B + nearby emitting ray work), not O(1). Source and darkness
subtraction order remain unchanged. A same-call actor sight sample also reuses
LOS and target light in detection's zero-chance fallback; cross-context-plane
light remains distinct. No cache spans a mutable AI phase, and no RNG draw or
engine loop ordering changed. Frozen-v16 scalar-light and detection oracles are
included in scalar_light_culling_tests.rs; their ignored microbenchmark separates
mostly distant actors from the all-near worst case.
