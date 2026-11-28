# Revision 50 potions and persistent ice

> Darkness's decrement rule below is retained for R50/R51. Revision 52 instead
> sets effective light to -100 immediately with the same footprint and duration;
> see [the current Darkness amendment](revision52-darkness.md).

Draft native support, behind the unchanged normal R50 release admission gate.

## Authored roster

The 20 ordinary effects are Healing, Energy, Incineration, Caustic Gas, Paralysis,
Confusion, Telepathy, Levitation, Speed, Invisibility, Descent, Cleansing, Fire
Immunity, Darkness, Flooding, Freezing, Repulsion, Discord, Teleportation and
Shattering. Offense, Defense, Accuracy, Dodge and Detect Magic no longer occur in
R50 ordinary drops. Earlier revision generation streams and those old effects
remain supported for historical data. Verdania Sap is still special, with its
historical serialized `Veridian Sap` identity and HP/energy/blessing semantics.

The existing dispatcher owns drink/throw landing, shared resource ownership,
consumption and Bottomless Pack/Dilute handling. Healing restores 50 HP; Energy
restores 50 energy and retains Boundless Hunger's no-cap exception. Existing
ignition, gas propagation, Paralysis's five-turn impact status, Telepathy's
30-turn floor sensing, Levitation's 20-turn flight, Speed's ten-turn haste,
Invisibility's 20-turn status and Descent's ground-collapse rules are unchanged.

New body-local effects reuse the existing normal-turn/fresh-status convention.
Fire Immunity lasts 20 turns, blocks fire damage and new burning, and does not
block explosions or remove intrinsic everburn. Cleansing removes named harmful
temporary statuses and their provenance, including poison, cold/curse slow,
paralysis, bleeding, confusion, hallucination, weakness, disorientation, venom,
corrosion, Roots, Discord, grabs, negative timed stat buffs and nonpermanent burning.
Shared defense/HP/energy-capacity penalties resolve through the resource owner,
then the existing potion tail settles derived HP/energy limits. Beneficial statuses,
equipment, ability state, allegiance, intrinsic traits and enemy cooldowns remain.
Received Silence/Riptide may be re-applied by their still-active source.

Darkness subtracts 50 illumination in exactly 5×5 cells for 20 normal turns,
including light from active fire and Shine; existing additive ring-light ordering
is retained. Overlapping doses refresh maximum magnitude and expiry, not stacks.
The serialized `revision50Darkness` field has at most one entry per floor cell.
Scalar and cached lighting include the same decrement and exact dependency.
Flooding creates shallow water on replaceable ground in 3×3 for 20 turns, preserving
structures and existing temporary edits; ordinary flames are quenched by the
existing wave helper. Expiry restores exact previous ground only if that edit
still owns the tile. Darkness and temporary water expire on visited inactive
floors and both normal/dream planes through the existing terrain clock path.

Freezing applies three turns of Slow to mutable creatures in 3×3 and freezes
water there. Ice Immunity blocks this cold Slow. Iceblast retains 30 cold damage
and three-turn Slow, and additionally places ice on valid open ground/water in
its wall-shielded 3×3. Ice is the existing persistent overlay, preserving the
underlying terrain and existing ice movement/water rules. It has no expiry.
R50 direct ignition, spread heat, standing fire/hellfire, and grounded burning
creatures melt ice. Earlier revisions' ice/snow semantics are unchanged.

Repulsion uses the existing Euclidean-radius convention: creatures within three
tiles move up to three forced steps away, stopping at physical obstacles,
occupants and portal redirection, with normal arrival hooks. Furthest-first,
actor-ID tie breaking makes collisions deterministic. The **provisional exact-center
default** chooses uniformly among directions whose first forced step is valid,
using the serialized gameplay RNG. Candidate order is north, east, south, west,
northeast, southeast, southwest, northwest. The choice is made after the other
creatures have been pushed; it uses one direction-selection draw whenever at
least one direction exists, including a single candidate. No center creature or
no valid first step consumes no direction-selection RNG. The chosen direction is
kept for up to three steps (two when diluted), with the same collision and arrival
rules as every other push. This applies both to the drinker and a directly hit
creature; it neither searches for a full-length route nor avoids ordinary ground
hazards. Immutable/wrong-plane targets remain excluded. Discord applies the
existing reversible mutual-hostility overlay for
ten turns without changing allegiance. A player drink remains player-controlled;
it does not introduce automatic player attacks. Teleportation can move the
drinker and reuses deterministic serialized-RNG selection among safe unoccupied
same-floor squares, excluding the origin. If none exists, it fizzles without RNG.

Shattering uses Euclidean radius two. It removes interior ordinary walls, trees,
doors, grates, temporary Obstruction and affected Cogworks wall cells (retaining
correct metadata for surviving cells). It never removes Claywalls. Map-boundary
walls, portal/topology cells, Borge/Epidite structures and fixed event/vent/light/
permanent-fire fixtures are preserved. It does not erase creatures, items,
corpses, pits, currents, gas, roots/razorleaf/slime hazard layers or arbitrary
unrecognized objects. This is the exact currently implemented obstacle scope.

Established dilution halves numeric magnitude/duration and rounds duration or
step count upward: Fire Immunity ten turns; Darkness −25 for ten turns; Flooding
ten turns; Freezing Slow two turns; Repulsion two steps; Discord five turns.
Footprints/radii and binary Cleansing/Teleportation/Shattering/ice remain fixed.

## Complexity

Let V be floor cells, A current-floor actors, C actor conditions, P portal/topology
entries, M terrain modifications, D darkness entries (D≤V), I equipped inventory,
and H the cost of arrival/environment checks. Fixed square footprints visit at
most 9 or 25 cells. Cleansing is O(C) named removals/retain plus actor lookup O(A).
Potion target filters may perform actor lookup and owner/trait checks per actor,
so new area collection is O(A² + A*(I+C)) worst case, not O(A).
Cleansing timed-buff discovery/removal adds O(C²) worst case from indexed ordered
map removals, plus the existing derived-stat/health settlement costs. Repulsion adds
O(A log A) sorting and at most three forced arrivals each with existing movement,
portal, collision, habitat and environment costs H. Each exact-center target adds
at most eight read-only first-step queries, O(8*Q) time and eight direction slots,
where Q includes actor/occupancy and owner scans, portal/topology lists, terrain
modifications, traits and player interaction guards. Q is not assumed constant;
existing helper scans may multiply A and P. This bounded lottery adds no world
copy, full-route search, renderer work or persistent state. Overall Repulsion
scratch remains O(A) plus existing movement-helper scratch. It runs only on potion
impact, not normal/haste clocks, inactive-floor ticks or rendering. Teleportation inherits O(V*(A+P+B+H)) destination enumeration and O(V) scratch
from the shared helper, with B actor abilities. No terrain edit bypasses arrival.

Darkness insertion scans up to D entries for each of ≤25 cells, O(D) with fixed
footprint, O(D) stored state. Scalar light adds O(D) lookup. Cached dependency
capture adds O(D) time/memory and field rebuild O(D) application; existing LOS,
light source, ring and native rendering costs remain additional, not constant.
Expiry is O(D); offscreen processing runs on the existing normal world clock.
Ice creation allocates O(V) mask on first use and checks at most nine cells with
portal and modification scans O(P+M). Heat melting scans ice and grounded burning
actors, O(V*A) worst case, O(V) scratch, per normal environment turn; direct
ignition/spread adds O(1) mask mutation. Shattering examines ≤25 candidate cells,
with portal, fixture and modification queries O(P+M+fixtures), then scans small
terrain/door/grate containers to preserve metadata. Copy-on-write floor edits
can copy all floor payloads, including V layers and D/M metadata; snapshots retain
those blocks. No animation, wall-clock randomness or new native renderer work is
introduced. Bounds are source estimates, not measured profiling results.

## Verification boundary

Deterministic tests cover roster/legacy generation, rejected old-revision effects,
benefit-preserving Cleansing, resource caps/Sap, Fire Immunity expiry, darkness
scalar/cache parity and overlap, water restoration, permanent water ice/heat,
Ice Immunity Slow guard, Repulsion collisions/radius, production drink/direct-hit
center selection in normal/haste phases and dilution, no-choice/no-center RNG,
center arrival/obstacle rules and serde/snapshot/Future-capture repeatability,
player Discord, self
Teleportation/RNG, Shattering protections, serde/snapshot/Future retention and
inactive-floor expiry. Native results are reported separately after execution.
Normal R50 save/replay/new-run admission remains closed; raw serde/snapshots do
not constitute released save or platform verification.

### Freezing and Discord retirement

Current runs choose from 15 ordinary potion effects. Freezing and Discord no longer roll; any already-generated bottles are labeled retired and cannot be drunk or thrown. This does not remove the Discord Amulet, other ice sources, or Verdania Sap. A separate command boundary preserves legacy seed loot, old journal commands, and historical use/throw behavior; it is recorded before generation in a new game and after the historical command ledger when resuming an old save. Generation still draws exactly once per potion: the static roster lookup and retired-effect match add O(1) time and memory per item with no rerolls.
