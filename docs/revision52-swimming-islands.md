# R52 Swimming rings on isolated islands

## Authored rule and runtime

An isolated island receives a Ring of Swimming only if it has no ordinary
swimming escape to another meaningful dry-land component. The ring grants haste
while its carrier is in shallow or deep water, not while flying above it. Ichor covering the tile does not count as water, consistently with the
existing water/current contact checks.

Swimming grants true Haste: two complete actions per world turn while active,
not the separate movement-only Fast trait and not a stored timed condition.
Eligibility is evaluated from the current body position at action start. Entering
water on a normal action leaves the following Haste opportunity available.
An earned bonus action completes without advancing the world or currents even
if it leaves the water. Wait and attacks can consume that same bonus action.
The next ordinary action advances the world normally. Slowing still suppresses
Haste; duplicate rings and other Haste sources do not multiply its actions.

All carried rings are active under existing R50 inventory rules. Swimming uses a
normal ring slot. A full ring category prevents pickup until the player drops a
ring; unequipping cannot bypass the category limit. Existing capacity-grant
restrictions still apply to dropping such a ring. Carried jewelry is already
exempt from water's random item loss, so the Swimming ring cannot float away.
Owner/shared-equipment and stats-only copy exclusions match other fixed rings.

## Placement and reachability

Generation checks the final ordinary walking map, not the visual landmark label.
Land components use the existing eight-direction movement and wall-corner rule.
Shallow water is a traversable land bridge; deep water separates components.
Openable doors remain traversable. Grates, walls, trees, chasms, pits, automatic
stairs and solid fixtures are not ordinary land bridges. Portal/spatial/speed-up
terrain remains part of physical island discovery, but is excluded from ordinary
mobility and placement. It cannot make an island silently disappear. If special
transport splits an island's ordinary ground into disconnected fragments, or a
special square has no finite land-only route to the connected ordinary fragment,
generation rejects it rather than putting the only ring somewhere unreachable.
The proof uses actual portal/spatial/speed-up movement outcomes. An action is
admitted only when every possible outcome has already been proved safe, remains
on the same physical land component, and avoids contact hazards. This allows
multi-step pocket interiors without sampling RNG, accepting a lucky branch, or
counting a teleport to another landmass as swimming.
There is no mainland or entrance-shore exemption. Every component containing
real dry terrain and bordering deep water is considered when the floor contains
at least two meaningful dry-land components. A single connected shore surrounding
a lake is not a trapped island. Shallow-only specks do not count as separate land
or escape destinations. A sealed dry pocket without a water boundary is not an
island. After the special-fragment safety check, a component with any ordinary
walking/swimming escape to different meaningful dry land receives no ring.

One identified Swimming ring is placed on a safe floor/grass tile of each trapped
component. Placement excludes existing ground items, live actors, fixtures,
permanent fire, gases, ichor, roots, mud, slime, razorleaves, ice and bulwarks.
No ordinary item is overwritten, discarded, relocated or deducted from the
ordinary loot budget. The ordinary randomized fourteen-ring pool is unchanged.
The safeguard consumes no random draws. Only successful new item births advance
the shared item ID allocator, so later R52 generated IDs may differ.

Two directed graphs compare ordinary swimming with ring-assisted movement.
Ordinary movement always takes one current displacement per move or wait;
ring-assisted movement additionally has water-origin bonus Move or Wait actions
with no current tick. Both proof graphs share the same baseline transition
function. The proof deliberately assumes no innate Fast or other status effects;
Swimming itself does not grant a Fast movement continuation. Shared reverse searches propagate up to two distinct reachable dry-land
component labels per state. This proves whether another meaningful landmass is
reachable without choosing an arbitrary mainland. Only an ordinarily trapped
component qualifies for a ring, and the chosen ring square must have an assisted
escape in both phases. Haste alone is never assumed to guarantee escape.
An additional four-state-per-cell graph tracks whether the player has actually acquired a ring and the normal/haste phase. It starts without
one, checks the actual floor-one entrance landing or all legal first departures
from deeper entrance stairs, and verifies access to ring squares and the
downstairs/final-pit approach. Stairs never become through-route bridges.
Acquisition on a safe dry square represents normal Drops if needed, the existing
free Pickup, and an optional normal Wait to reach the modeled phase. Current
auto-pickup after movement or Drop is unchanged. No extra capacity or free water
haste is assumed. Dynamic enemies, combat, timed terrain changes, illnesses,
slowing and other hazards are not an invulnerability guarantee.

Geometry candidates and fully decorated/populated candidates both run the check.
Missing safe placement or return routes reject the candidate within the existing
32-geometry-candidate and eight-complete-floor-attempt limits. Failure cannot
publish a partially generated floor. No new terrain repair or clearing is added.

## Compatibility and presentation

The item uses the existing `kind=ring`, `effect=swimming` wire fields and canonical
name/description helpers. No new save fields, duration counter or migration are
introduced. The power is gated to revision 52; earlier revisions keep their old
equipment and haste behavior. Existing floors and saved currents are not changed
on load. Snapshot and Future state retain the ring and tile state, so conditional
haste is recomputed on restoration instead of preserving a stale status flag.

## Complexity

Let V be floor cells (at most 3,600), A live actors, G ground entries and M the
per-cell fixture/topology metadata scan cost, including grate lists, solid
fixtures and at most eight random plus eight speed-up 48-cell spatial pockets. Ring runtime queries cost
O(I+C+B) for owner inventory I, actor conditions C and existing flight/ability
queries B, with O(1) scratch. They draw no RNG and write no state.

Placement precomputes physical/ordinary traversability and safety and labels
both physical land components and their ordinary walking fragments. The
special-land return proof has at most 8V actions and 16V possible outcomes;
worklist counters process each outcome once, including the bounded metadata
lookup in actual walking_exits. It does not repeat a whole-floor scan per proof
rank. The ordinary and ring-assisted movement graphs each have at most 2V states and 18V
edges. Each reverse search propagates at most two destination labels per state,
so every edge is processed at most twice across all islands together. The
ownership/phase graph has at most 4V states and 36V edges; one forward and one reverse BFS check
entrance acquisition and downstairs progression. There is no per-island
pathfinding loop. Ground-item and actor occupancy are indexed once. Total is
O(V*M + V + A + G) time and O(V + A + G) upper-bound scratch/output; fixed gas/layer
counts are included in the per-cell pass. Generation repeats this within the
existing bounded retry limits. No rendering or native library work occurs here.

## Verification

Deterministic tests cover shallow/deep entry and exit, flight and ichor exclusion,
legacy revision behavior, boolean stacking and slowing, current-free bonus Move
and Wait, ordinary Wait drift, every baseline Move/Wait transition against the
real scheduler across ring ownership, phase and water entry/exit, ring capacity and drop-to-pick-up, jewelry flotation,
shared/copy ownership, save/load, Future and Timeweave restoration, bridge/diagonal
connectivity, sealed pockets, occupied/hazardous placement, portal/pocket/speed-up
coverage and fragmentation, ordinary-swimming exemptions, single-shore lakes,
starting-island safeguards, unfixable repelling current necks, no-preowned-ring
progression, finite multi-step spatial returns, rejected stochastic self-loop
shortcuts, and directed escape routes replayed through the real engine.
The default boundary suite also checks a reduced, source-hashed capture of the
actual previously rejected floor 37 candidate from swimming-survey-b, attempt 4,
under fixtures/revision52-swimming-floor37-pocket.json.

### Earlier measured final-policy survey

These receipts predate the true-Haste action correction. Updated focused checks
and the same 80-floor survey must be rerun before treating them as final evidence.

The final focused gate passed 19 Swimming unit tests and eight boundary tests.
The explicit two-seed survey generated all 80 floors successfully in 23.66 seconds
of test runtime, including its validation and current reconstruction. This is a
test receipt, not an isolated generation or gameplay frame-time benchmark.

- 0-1-2:swimming-survey-a: two rings on two of 40 floors, at most one per floor;
  35 water-separated land components, at most three per floor; whole-floor retry
  index sum eight; accepted_attempt_index_sum 352.
- 0-1-2:swimming-survey-b: two rings on two of 40 floors, at most one per floor;
  29 water-separated land components, at most three per floor; whole-floor retry
  index sum four; accepted_attempt_index_sum 279.

Thus four safeguard rings appeared across 80 floors. Component counts include
both sides of a water separation, not just ring-eligible trapped islands. Retry
figures include every existing generation rejection reason and do not attribute
all retries to this safeguard. The accepted_attempt_index_sum metric sums
absolute geometry-candidate indices, including the complete-floor attempt times
32; it is not a count of all geometry work performed.
The numbers of rejected candidates within the final accepted geometry windows
sum to 96 and 151 respectively; work in earlier discarded windows is not measured
here.

Both worlds retained exactly seven Godbeasts in the planned domains and floors,
unchanged gameplay RNG during generation, exact deterministic current
reconstruction, and idempotent safeguard placement on every accepted floor.
Current overlays totaled 1,199/925 squares, 11,488/7,040 covered cells, and
74,064/52,392 square-cell visits for the two seeds, with maximum floor visits
7,530/6,778. These observed values remain separate from the asymptotic bound.

Receipts: verification/revision52-square-currents/swimming-unit-08.log,
swimming-boundary-08.log and swimming-survey-08.log. The earlier failed survey
and original rejected floor captures remain preserved separately. The correction
proves real multi-step spatial returns; it does not disable the safeguard or
change runtime movement.


### Baseline escape proof with optional Fast movement

The baseline proof deliberately has no innate Fast trait or other dynamic status.
Its route remains executable by a Fast swimmer: after each first Fast move,
the player may use Wait for the earned continuation. Fast(Haste) plus Wait
reaches the normal action without current drift, while Fast(Normal) plus Wait
produces the same single world/current advance at the destination as the
baseline move. Thus Fast does not require another static reachability state;
it can add one bounded extra input per proof move without removing the proved
route. This does not extend the proof to Slowed, paralysis, stranding or other
excluded dynamic hazards.


The updated full-source test and explicit 80-floor survey receipts are stored in
verification/r52-avatar-world-final/. They are the final integration authority;
the earlier measurements above remain labeled historical and are not silently
reused after the Haste transition changed.
