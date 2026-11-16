# R52: nearby-floor generation and square currents

## Designer rule and implementation boundary

Newly generated R52 floors use overlapping square circulations for deep lakes.
No saved floor is regenerated, pruned, or migrated on load. R51 and earlier
retain their existing generation horizons, inward currents and shore repair.
The existing current wire format remains unchanged: zero means no current;
cardinal north/east/south/west are 1/3/5/7.

Normal startup generates floors 1 and 2. Successful player arrivals request at
most their destination plus one floor, capped at floor 40. DEBUG and playtest
startups retain the prior floors required for reconstruction and generate one
floor below the chosen start. Revisiting a shallower floor never deletes deeper
saved floors. Explicit generation tools still honor their requested target.
Nonplayer falls and item/corpse placement can request an actual physical
destination beyond the player's horizon; they do not add speculative batches.
A cohort fall requests an extra floor only when the player has a legal landing.
Collapse prepares its actual destination, then the successful player-fall path
extends the horizon if needed. Ordinary falling, stairs and vertical travel use
the same player-arrival horizon. The seed-based domain/Godbeast schedule is not
changed, and no Godbeast is forced to exist on an ungenerated future floor.

## What is a lake?

A lake is a cardinally connected component of current-eligible deep-water
(`water`) tiles. Ichor-covered water, closed doors and grates are excluded,
matching the runtime's existing current-surface eligibility. Shallow water,
dry ground and obstacles are never painted. The generation stream is private:
run seed, accepted geometry-attempt seed and floor depth are mixed with the
new 96000 salt. It does not consume gameplay, loot, population or starter RNG.

Lake enumeration starts at the first unvisited row-major tile; flood fill uses
north/east/south/west order. Every eligible lake tile is covered by at least one
square, even a one-tile or narrow lake. Coverage does not imply a nonzero final
arrow because the specified collision rule can cancel opposing currents.

## Square sampling and direction

The designer specified squares of size 2x2 and larger, without a maximum,
size distribution, square count or rule for three-way overlaps. The following
bounded implementation choices make those cases deterministic:

1. Pick a random tile from the lake's still-uncovered tiles.
2. Pick side length uniformly from 2 through the smaller of the floor's width,
   height, and the lake's largest bounding-box dimension (at least 2).
3. Independently pick x and y uniformly among in-bounds square origins that
   contain that tile. A square may extend over dry land or other lakes, but its
   influence is clipped to its own lake.
4. Pick clockwise or counterclockwise with equal probability. Visit the square
   row-major, resolving each covered lake cell, and remove newly covered cells
   from an indexed swap-remove set.
5. Repeat until every lake cell has been covered. When the first square covers
   an entire multi-tile lake, add one more square with an anchor from that lake.
   Thus a multi-tile lake receives at least two squares, while progress is
   bounded by L+1 squares for a lake with L tiles. No rejection loop is used.

Each tile compares Manhattan distance to the four square edges. The distance
is zero on the edge. Among tied closest edges choose uniformly using the same
private deterministic stream. Clockwise maps bottom to left, top to right,
right to down and left to up. Counterclockwise reverses each direction.

## Overlaps

Squares fold into the current layer in their generation order:

- Equal directions keep that direction.
- Distinct nonopposite directions choose the square closer to its own edge.
  Equal distance chooses either source with equal probability.
- Opposite directions from the same rotation cancel to no current.
- Opposite directions from different rotations turn according to the explicit
  designer example: clockwise up -> left -> down -> right -> up, and its
  inverse for counterclockwise. Either input gives the same resulting direction.

The surviving result retains the stronger source's edge distance and rotation
for later overlaps, randomly choosing the source when strength ties. This also
applies when the two directions were equal. A canceled pair leaves no source,
so a later square supplies a fresh current. This deterministic left fold is an
explicit multiway-overlap inference, not a claim that all possible pair orders
are equivalent.

## Integration and gameplay implications

Squares replace the final R52 inward/shore-repaired arrow layer. Authored water
setups 101–108 retain terrain, traps, lighting and creatures, but their former
straight/bent current lanes no longer override the new lake rule. Such setups
may no longer force their previous path. The old bank repair is not applied
because it would erase the requested closed circulation. Square generation
runs on finalized geometry and again after population decorators, using the
same accepted attempt and final terrain. Neither pass carves terrain.

Current displacement, collision with occupants, legal bank landings, flight,
normal/haste timing, offscreen simulation and current save data remain handled
by the existing runtime. An arrow aimed at an illegal destination does not
move an actor. Closed circulations are now permitted. Square currents alone
neither create nor guarantee natural dry paths between islands; the separately
specified [Swimming-ring safeguard](revision52-swimming-islands.md) is responsible
for isolated-island access. It adds a ring only where ordinary swimming cannot
reach different meaningful dry land, and rejects a candidate if the ring cannot
provide an escape. There is no blanket ring grant to every separate landmass.

## Complexity

Let V be floor cells, G grate entries, D total door-key bytes, and for lake i,
L_i its tiles, Q_i the number of squares and S_i the maximum square side.
Current generation precomputes eligibility in O(V + G + D), flood fills in
O(V), and visits each overlaid square's area. Total worst-case time is
O(V + G + D + sum(Q_i * S_i^2)); additional memory is O(V + max L_i).
Q_i <= L_i + 1, and S_i <= min(width,height) <= 60 for supported geometry.
There is no hidden pathfinding, sorting, native rendering or cache-miss work
inside the overlay loops. Every uncovered removal and selection is O(1).

A normal floor candidate performs one square pass; the accepted complete floor
performs another after population decorators. Existing generation bounds remain
32 geometry candidates per window and at most eight atomic complete-floor
windows. Rejected candidates may therefore repeat the bounded work. These are
source-derived bounds, not timing measurements. Generation reports expose lake,
square, covered-tile and visited-square-cell counts for deterministic tests and
profiling without adding persistent save fields.

Changing the horizon is O(1). It reduces requested speculative floor count,
without changing the existing per-floor geometry, population, identity-index
reconciliation or copy-on-write costs. Fresh startup requests two instead of
ten floors; actual physical offscreen destinations and explicit full-world
sweeps remain exceptions because they request real work, not prefetch.

## Verification

Focused tests cover both rotations and every edge, seeded corner/center ties,
all pair collision branches, strength ties, retained multiway provenance,
cancellation followed by another square, complete bounded coverage, tiny and
narrow lakes, excluded surfaces, non-current-field preservation, old revision
horizons, fresh startup, stairs, nonplayer and player cohort falls, unchanged
Godbeast planning, actual generated geometry and retained saved arrows.
Verification logs and measured profiling outcomes are recorded separately; this
document does not imply that unrun platform or full-workspace gates passed.

The independent Swimming safeguard may reject a geometry or complete-floor
candidate that previously passed, selecting a later deterministic attempt.
Any resulting layout/population delta is attributable to that new acceptance
rule rather than current painting. Current-only comparisons must hold the
candidate attempt fixed; complete-world checks must also verify ring placement
and accepted-candidate safety. R51 and older acceptance remains unchanged.

### Rejected-candidate diagnostics and admission comparison

Default tests and all production builds perform no candidate-export IO. The
ignored `startup::export_rejected_swimming_candidate` diagnostic can be enabled
with `COTCG_ISLAND_FAILURE_EXPORT_DIR` set explicitly to a fresh absolute
verification directory. It generates the recorded survey-B world and exports
failed final floor-37 placement inputs: seed, absolute geometry attempt, full
floor, actors, ground items and error. Existing files are never overwritten.
Successful corrected generation is accepted; the driver does not assert that
the old failure must continue. Failed generation remains a failing diagnostic.

For J serialized JSON bytes, each enabled export costs O(J) time and additional
memory plus O(J) native filesystem-write work and filesystem latency. At most
eight final floor candidates are attempted. No exports or serialization occur
without the explicit flag. This is test instrumentation, not gameplay, RNG,
save-format or generation-acceptance logic.

The original independent R51/R52 admission oracle found a legitimate acceptance
delta at `0-5-11:r52-admission`, floor 4: geometry attempt 0 became attempt 1,
changing 17 actors to 19. R52 independently rejects old attempt 0 because a
1031-cell land component has no safe obtainable Swimming ring and escape route.
The failed oracle and rejection evidence are retained in the verification logs.
The strict comparison now independently regenerates R51 geometry and population
at the selected R52 candidate/window indices. Only those two indices are read
from R52; no terrain, actors, loot, currents or other output is copied. Original
R51 topology acceptance and every unrelated-field comparison remain required.
Separate legacy-save and frozen-revision tests still exercise historical rules.

The accepted-layout comparison also validates the retained-item ledger before
normalizing its approved delta: ordinary count from the three-floor budget,
retained hearts from independently generated Jackal counts, unchanged carried
items, retained slots, block total/target/overshoot, fixed early drop schedule,
items-stage count, final ground count including guaranteed rings, and allocator
increment. The old default-layout aggregate (four added ordinary items for the
first seed) ceased to apply after safety rerolls. Its replacement is not a new
copied golden: the selected layouts independently require three additional
drops on floor 5, three on floor 6, and two on floor 7, plus one Swimming ring.
The original failed aggregate and per-floor diagnostic remain in
`verification/revision52-square-currents/admission-budget-diagnostic-09.log`.

### Verified native scope (2026-11-08)

With the final Swimming land-return proof and unchanged generation runtime,
the focused square tests passed 7/7, Swimming unit tests 19/19, boundary tests
8/8 (including the captured floor-37 candidate), horizon/current integration
9/9, strict admission 6/6, and the full core library 710/710 with four explicitly
ignored diagnostics/surveys. The admission test includes two independent
40-floor R51/R52 comparisons. Receipts are under
`verification/revision52-square-currents`: `square_currents-08.log`,
`swimming-unit-08.log`, `swimming-boundary-08.log`,
`revision52_generation_currents-10.log`, `revision52_admission-11.log`, and
`core-lib-11.log`.

The explicit two-seed, 80-floor survey passed in 23.30 seconds on the cloud
native test build (`swimming-survey-11.log`). Seed A/B each produced two
Swimming rings on two floors, maximum one per floor. All accepted floors passed
safeguard revalidation, exact current reconstruction, unchanged gameplay RNG,
and the complete seven-Godbeast schedule. Square counts were 1,199/925;
covered cells 11,488/7,040; square-cell visits 74,064/52,392; maximum visits for
one accepted floor 7,530/6,778. These counts describe accepted floor painting,
not discarded-candidate work or a worst-case timing guarantee.

`accepted_attempt_index_sum` was 352/279, and `population_retry_sum` was 8/4.
The former sums absolute indices of accepted geometry candidates and is not a
count of all candidate attempts. Earlier discarded windows can stop before all
32 geometry slots are examined, so total generation work cannot be inferred
from that sum alone. Measured survey time includes complete generation and
its revalidation; it is separate from the asymptotic analysis above.

These results do not certify Windows, renderer behavior, or a full-workspace
release. Those aggregate and platform gates are recorded by the release owner.
