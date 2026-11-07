# Creature hunger and bloody walls — revision 33

## Authority and scope

Designer instruction 8 adds **Bloody walls**, creatures **Hungering after 100
turns**, and **Eating / Drinking for 10 turns** from walls / the blood river.
The implementation is NPC ecology, not a player survival meter. It does **not**
add starvation damage, healing, energy restoration, stat growth, consumable food,
or source depletion. Royal Honey is a separate explicitly requested consumable.

## Executed defaults

- Every living NPC accumulates one hunger turn per **normal simulated
  world turn**, independent of haste actions or Slow movement. At 100 it remains
  Hungering until a meal completes. Spawned/revived actors and r33 actors missing
  ecology fields start from zero. Corpses, the player, Half Here / Mini Me player
  copies do not accumulate this ecology clock. Dormant Mechs remain neutral and
  cannot autonomously leave their station to feed.
- An idle wandering creature chooses the nearest reachable safe feeding position
  using the existing deterministic grid distance rules. Walls are eaten **from
  an adjacent tile**, never entered. Water can be drunk from a safe bank or the
  creature's own legal water tile. The normal corner, flight, habitat, grate,
  bulwark, occupancy, hazard and rooted movement rules remain authoritative.
  Inaccessible food produces ordinary idle behavior, not teleportation or a
  retry loop. Food seeking itself makes no gameplay RNG draw.
- A meal requires ten consecutive stationary normal world turns at the same
  valid source. `Eating` or `Drinking` appears during the first nine completed
  feeding turns; on completion the hunger counter becomes zero. The next meal
  is not due until another 100 normal turns. Haste does not advance a meal;
  Slow still permits stationary feeding each normal turn.
- Hunting, stalking, fleeing, ranged recovery, paralysis, existing neutral
  guardian behavior, combat and player rally/follow instructions retain
  precedence. Autonomous allies may feed only when no enemy or player-follow
  need is present, never while obeying a positional rally order. Hunger can wake
  a fully rested sleeping creature; incomplete energy recovery still takes
  precedence. A meal is canceled by actual positive HP loss, threat/priority
  changes, displacement, loss of its source, or a missed feeding turn. A
  subsequent attempt begins at one, never resumes an interrupted partial meal.
- Food never changes HP, energy, Offense, Defense or any other resource. Existing
  regeneration/status effects still run normally; they are not meal rewards.

## Sources and visual knowledge

`Floor.bloodyWalls` is an optional boolean **overlay on `wall` terrain**, not a
new walkable tile. Generation deterministically marks approximately one in twelve
accessible interior wall cells, with one guaranteed when candidates exist. This
provisional density uses a separate cell/seed hash and consumes no generation or
simulation RNG. Existing wall collision, LOS, pathfinding, fire and destruction
semantics are unchanged. If the underlying wall is destroyed its mark is inert.

For r33 generation, existing **`river` landmark wet cells** are explicitly seeded
as persistent, mild dissolved-blood sources of concentration **3.0**, using the
water subsystem's distinct `bloodRiverSources` list. This does not seed lakes or
ordinary water, replace currents, or change water physics. Kelpie wet spawn sites
also retain their separate sources, and actual HP damage may contaminate water.
Drinking requires water/shallow-water without Ichor and actual concentration at
least the water system's **0.05 cutoff**. Cosmetic crimson water from r31, ground
blood, dry tiles and Ichor are not food. See
[native-environment-hooks-r33.md](native-environment-hooks-r33.md) for transport,
source persistence and water rendering.

Bloody-wall tint and the **Bloody wall** tile-inspector title use the existing
observed/remembered hazard boundary (`bloodyWall` in `HazardSnapshot`). Hidden
changes do not update old memories. Creature inspection shows **Hungering**,
**Eating**, or **Drinking** only for a directly visible, unmasked real actor;
it does not reveal unseen food or bypass Skin Walker / hallucination projections.

## Persistence and offscreen simulation

Nine numeric actor condition keys retain age, clock, partial-meal source/type,
position, progress and interruption timing. They are in ordinary serialized actor
state and COW snapshots, so save/load, Revert and future restoration restore the
same progress, with no wall clock, external cache or RNG reconstruction.

Revision 29–32 behavior is gated out and old saves do not acquire ecology keys.
An involved inactive r33 floor with living actors uses exact existing offscreen
turn processing rather than stationary catch-up, so food seeking and meal
completion are not skipped. Unvisited/uninvolved floors retain the existing
unsimulated-floor rule; hunger does not invent retroactive pre-activation turns.
Offscreen proxy perception does not create an imaginary player threat, and
allegiance, travel-follow and habitat restrictions still apply. An actor clock
stamp prevents a transferred body from aging twice at the same world tick.

## Verification and supported status

Implemented and executed in `creature_feeding.rs`, normal/haste AI, offscreen AI,
shared actual-HP-loss boundary, generation, observation and validated storage.
`tests/creature_feeding_v33.rs` covers exact 100/10 thresholds, repeat meals,
no buffs/depletion/player hunger, safe adjacent walls, blood-only bank drinking,
interruption/reset, Slow, actual haste-only commands, wake-from-sleep, autonomous
ally versus follow/rally/combat priorities, rooted/aquatic/unreachable cases,
legacy gating, validated round-trip, snapshot/Revert/future restoration and
exact offscreen completion. Desktop `feeding_observation_tests` covers the map,
inspector and stale fog memory. These are native deterministic tests, not claims
of exhaustive combat balance or Windows GUI/manual platform verification.
`verification/feeding-r33/` additionally contains inspected native-software
render captures for Eating, Drinking and Bloody wall panels. Their saved input
is an explicitly authored observer fixture, not a claim of naturally reached
gameplay; the command-path behavior is covered by the regressions above.

## Complexity

Let **N** be floor cells (existing native pathfinder limit 3600), **A** floor
actors, **W** hungry sleepers needing stat checks, **F** involved floors,
**T** elapsed normal turns, **C** living actors with active partial meals,
**S** the existing effective-stat helper cost, and **H** the existing per-cell
hazard/passability helper cost. Existing helpers may scan actor/terrain/trait
collections; legacy gas-channel materialization may be O(N). H is not assumed
constant and these costs are included below.

- Each floor clock pass is O(A + W*S), O(A) scratch and constant added data per
  eligible actor. Actor blocks detach at most once per mutated COW snapshot,
  adding O(A * actor-state-size) existing clone work when shared.
- A hungry idle travel query builds at most **two** existing hazard-free BFS
  distance fields. Safe-cell mask creation costs O(N*H), indexed occupancy
  O(A), source-neighbor checks O(N) with at most eight checks/cell, and both
  BFS fields O(N) because grid degree is bounded. Total **O(N*H + A + N)**
  time/O(N) transient storage per seeking actor/turn; no per-food path search,
  unbounded search or sorting. Legacy helper cache misses can make the mask
  O(N²), as can the existing pursuit code; this is an explicit bound, not a
  measured claim of linear whole-turn performance.
- Active feeding uses bounded source-neighbor work plus existing safety/priority
  checks and actor lookup. End-turn cleanup is O(C*(A + H + priority-cost))
  with O(C) IDs; C <= A. Allied priority checks include existing actor/LOS work.
- Exact involved offscreen simulation retains these costs per turn and floor:
  O(T * sum_F(existing actor/environment cost + ecology cost)). There is no
  elapsed-time shortcut for meals. The loop is bounded by requested serialized
  elapsed turns, not an arbitrary search budget. Uninvolved floors remain frozen.
- Bloody-wall generation is O(N) time/O(N) retained optional bits. The separate
  river-source initialization/transport bounds are documented by the water
  subsystem. The overlay uses one constant-cost observed-cell check in rendering
  and inspection, within existing terrain/lighting costs. Hazard snapshots and
  save export include O(N) optional source data and O(A) numeric ecology data;
  temporal state remains within existing O(saved-state-size) materialization.

These are asymptotic bounds. No timing claim is made. Reproduce focused tests
with `sh tools/cargo.sh test -p corpse-core --test creature_feeding_v33` and
`sh tools/cargo.sh test -p corpse-desktop feeding_observation_tests`; profile
representative full floors with hungry actors and legacy gas channels before
changing the helper or offscreen policies for performance.
