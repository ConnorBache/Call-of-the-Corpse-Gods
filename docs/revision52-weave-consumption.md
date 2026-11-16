# R52 Weave of Time arrival consumption

The 2026-11-07 designer clarification supersedes R41's source-only collision rule:
"if you step on a weave, it gets destroyed. If you step on it from the wrong angle,
so does everything behind it."

## Exact current rule

- Weave of Time is passive. An ordinary successful adjacent voluntary step A to B
  leaves a directed teal segment at A pointing toward B.
- If you subsequently move from B back to A, that reverse edge is free: no action
  or world turn, resource tick, status tick, or enemy phase. On successful arrival,
  the segment at A is consumed immediately. No replacement segment is laid at B.
  You can continue backing through older segments, consuming them one at a time.
- Entering A from any other direction is an ordinary charged movement. Arrival
  consumes A's segment and all chronologically older segments. Newer segments
  survive. The ordinary step still leaves its new segment on the departed tile.
- The movement kernel still enforces terrain, actors, doors, forced movement,
  confusion, ice, and sticky ground. A bump attack is not free and consumes no
  segment. Failed movement consumes no segment. Free movement can open a door or
  trigger ordinary arrival effects; it is not invulnerability or phasing.
- The current eligible edges remain same-floor, same-plane ordinary adjacent
  steps. Stairs, pits/chasm transitions, portals, forced displacement, and
  nonlocal topology transitions do not create or qualify as a free weave edge.
  No new forced/teleport transition deletion rule is inferred.
- Waking and Dreamwalking segments retain distinct identities. Serialized trails,
  explicit future snapshots, and clone-on-write state preserve exact consumption.
- R51 and earlier recorded rules remain unchanged. R52 uses the new arrival rule.

Wrong-angle "behind" means older in the trail's canonical chronological order,
not geometric direction or proximity. A used middle segment removes only itself;
older and newer entries keep their order. If an older saved state already has an arrow under the player, new source
placement replaces only that source arrow. It does not apply a second
departure-prefix deletion after the new arrival rule.

## Complexity

Let L be the number of live segments, H retained hash/deque capacity, k the prefix
removed, R owned Run metadata and event bytes, and F floor-lookup work. Entry
classification uses expected O(1) indexed lookup after ordinary movement's own
transitive costs. Correct-angle removal at either chronological endpoint is
expected O(1); a middle removal is O(L) search/shift. Wrong-angle removal is
expected O(1+k); adversarial hash behavior can cost O((k+1)L). Extra scratch is
O(1), retained storage O(H). Shared trail mutation detaches O(H+L). A potential
free step already clones O(R) owned candidate state and its ordinary movement/
arrival COW changes; consumption does not remove those existing costs. There is
no per-frame simulation, new RNG draw, pathfinding, or new wire field.

## Verification

`revision52_weave_tests` covers whole-chain unrolling and continuation, exact
middle removal, immediate wrong-angle prefix deletion, blocked/attacking entry,
haste/Pursuit, Dreamwalking identity, save roundtrip, and snapshot isolation.
Existing R41 movement and primitive tests remain compatibility coverage.
