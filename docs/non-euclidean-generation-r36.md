# Revision 36: non-Euclidean generation

This implements the designer's floor-20 threshold, cellular-automaton selection,
contiguous pockets, sequential unassigned-square algorithm, and walk-through
requirement. Runtime movement/occupancy, stochastic exit choice, observation and
projectiles are handled separately; this module never consumes simulation RNG.

## Explicit initial defaults

The designer did not specify density, size or cellular-automaton parameters. The
initial conservative defaults are **one requested pocket per floor 20–40** and a
whole **6–48-cell cardinally connected component**. These are not authored depth
scaling or a guaranteed appearance on every possible floor.

A deterministic independent stream is seeded with the run seed, floor and
`non-euclidean-r36` namespace. A uniformly selected eligible center defines an
11×11 local binary cellular automaton with a fixed dead outer border and fixed
dead ineligible-cell mask. Interior eligible cells begin alive with probability
0.55. Three synchronous rounds count the eight Moore neighbors: an alive cell
survives with at least four live neighbors; a dead cell becomes alive with at
least five. The entire resulting cardinal component is selected uniformly from
components in the size range. It is never cropped to meet size requirements,
connected by carved terrain, or repaired after selection.

Eligible cells use existing floor, grass or shallow-water terrain. Stairs,
fixtures, protected setups/event rooms, connectors, existing portal endpoints,
initial actors and ground items are excluded. The floor's terrain, original
population, item identities and budgets are never rewritten by this feature.
The safety check also rejects components not originally reachable from upstairs.

## Literal assignment, not repaired permutations

Here “adjacent” follows the existing game's eight-direction movement, including
its diagonal corner rule: both intervening orthogonal cells must not be walls.
The game does not treat trees as walls for this specific corner test; generation
uses the same helper rather than inventing different adjacency. This assignment
adjacency is distinct from the deliberately conservative cardinal CA-contiguity
and safety checks below.

Within one candidate component:

1. Draw one source with probability 70%, or two with probability 30%. With only
   two unassigned cells, always use one source.
2. The first source is uniformly selected from unassigned cells for the initial
   group; thereafter it is the previously selected forced next source. A second
   source, if drawn, is uniformly selected from the other unassigned cells. No
   additional physical-adjacency restriction is imposed on the second source.
3. Remove the selected sources **before** testing destination eligibility. If a
   two-source proposal leaves no remaining cell with a remaining legal eight-way
   neighbor, restore its second source and use one source instead.
4. Choose the destination uniformly from still-unassigned cells with a
   still-unassigned legal eight-way neighbor. If that list is empty, the sole remaining
   unassigned cell may be the destination. No assigned square is a candidate.
5. Remove the destination. If cells remain, uniformly select one of the
   destination's legal eight-way adjacent unassigned cells as the **forced first source** of the
   next group.

Each completed group is serialized in this order as `sources` (one or two
indices) and `destination`. A one-source group is reciprocal. In a two-source
group both sources lead to the destination and the destination offers the two
sources; the actual fair choice belongs to committed simulation, never generation
queries, rendering, hover, previews or replay presentation.

The literal rules can strand a terminal singleton or a next-source chain. For
example, three remaining cells can require a one-source fallback and leave one
cell with no legal destination. Such a proposal is **rejected in full**. It is
not completed with a self-link, assigned destination, backtracking rewiring, or
an extra rule that changes the specified choices. At most 64 full assignment
attempts are made per CA component and at most 48 CA attempts per floor. If no
candidate passes, the floor omits the pocket. The **70/30 ratio describes the
proposal draw**, not a guaranteed final ratio after mandatory fallback and safety
rejection.

## Walk-through proof

Physical CA contiguity alone is insufficient. For each complete assignment the
generator constructs the **directed live movement graph**: choosing a neighboring
entry cell lands at its linked arrival; destination cells with two sources have
both outcomes as separate edges. Existing revision-35 portal transitions remain
present and unchanged. A four-neighbor graph is used, giving a conservative
proof for the eight-way live walker. The proof uses the floor's initial walking
terrain and excludes grates, permanent fire, pits/chasms and solid fixtures.

A forward search from upstairs and a reverse-edge search back to upstairs must
both cover every cell in the original upstairs walking component. The proof is
then repeated with **all portal endpoints forbidden together**, covering the
original upstairs walking component with those endpoints removed. This preserves
revision 35's optional-portal requirement after non-Euclidean rewiring; a pocket
that can be escaped only through a portal is rejected. Every pocket
cell and **each possible stochastic landing state** therefore has a route both
into the floor and out toward upstairs/downstairs where those belonged to the
original component. One good random branch cannot excuse a trapped branch. This
is a reachability guarantee, not a promise against an infinitely unlucky sequence
of random choices or later player-caused terrain changes. Rejection does not
alter the candidate graph.

A regression includes a physically contiguous corridor pocket in which one
stochastic source can return upstairs and the other is trapped. It must fail
safety. A separate test constructs the movement graph independently of the
production checker and verifies paths from every accepted group endpoint.

## Persistence boundaries

Floor metadata is optional `nonEuclidean`; absent metadata remains absent for
revisions 1–35. New metadata is admitted only at simulation revision 36 onward,
including dream floors. Save admission requires bounded, sorted, unique,
cardinally contiguous cells, a complete disjoint partition into one/two-source
groups with distinct destinations, and no portal-endpoint overlap. It does not
reject legitimate later terrain changes or require a controlled fixture to
recreate a generator's random history. A separate `assignment_follows_rules`
check proves ordered generation traces in tests. Save/load retains the chosen
mapping; it does not regenerate it.

## Complexity

Let `V` be floor cells, `E≤8V` the directed proof-graph edges, `K≤48` pocket
cells, `P≤7` existing portal pairs, `H` the protected/fixture footprint entries,
`A` initial actors and `I` ground entries. Let `S≤48` be CA attempts and `R≤64`
assignment attempts per component; the CA side is fixed at 11 and has three
rounds.

- Eligibility construction: `O(V+H+A+I)` expected time/scratch using membership
  sets, plus the existing protected-generation helper's footprint-expansion
  cost. No actor/pathfinding query is hidden inside the cell loop.
- One CA proposal: `O(11²)` time/scratch with constant rounds and bounded
  connected-component searches; floor-sized eligibility is reused.
- One sequential assignment: `O(K² log K)` time and `O(K)` scratch, including
  repeated ordered-set adjacency eligibility and candidate collection.
- Structural validation: `O(K log K+P)` expected time and `O(K+P)` scratch.
- Directed safety: `O(V+E+K log K+P)` time and `O(V+E+K+P)` scratch, including
  construction and both searches, repeated once with portal endpoints excluded.
  The two proofs are a constant factor. No pathfinder is nested in the searches.
- Worst-case installation: `O(V+H+A+I + S*(11² + R*(K² log K+V+E+P)))` time and
  `O(V+E+H+A+I+K+P)` scratch. Installation runs once after initial floor
  population, not per frame, move or observation. Generation RNG and stored
  metadata add no native rendering work.

These are asymptotic/bounded-work statements, not measured platform timings.
`non_euclidean_generation_v36` exercises deterministic rooms, all six domains,
legacy nonmutation, protected cells, rejected traps, full group partitions,
metadata corruption, dream-floor validation and population/RNG noninterference.
Executed results belong in the final revision verification receipt.
