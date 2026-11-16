# Revision 52 enemy follow-ups

These mechanics run only at simulation revision 52. Species identity uses the
existing bestiary resolver, including named/imported revision-51 bodies, rather
than a newly generated actor marker. Revision 51 and earlier keep their original
combat and movement. Numerical natural-birth tuning remains in `balance52`.

## Dragon

- Full natural energy is 50. Physical takeoff costs 8 energy through
  `enemy_spend`, allowing six activations with 2 energy left before recharge.
  Existing sleep/recovery can refill energy; this is not a lifetime six-use cap.
- Takeoff owns one complete action and earns no Fast continuation. A subsequent
  movement action can use its earned Fast continuation but never pays for
  another takeoff or advances the timer. Flight lasts at most six normal turns
  and ends early upon reaching its safe landing. `flying` and `dragonFlight`
  count down together. A later longer levitation grant is preserved on landing.
- Activation requires strictly fewer normal turns after counting takeoff and
  the current Fast/Haste action stages; a full-action saving that arrives on
  the same normal turn is rejected. A first sticky struggle consumes its action
  without earning a Fast continuation. A struggle during an earned continuation
  consumes that continuation. An already-paid sticky struggle is not charged
  again in the ground comparison. True Haste uses whether this is its bonus or
  normal action, supplied by the transient scoped NPC action receipt. The route ends
  with a safe landing adjacent to the known quarry. The guaranteed aerial route
  must fit within four moves: six flight ticks minus the upkeep before the
  next ordinary movement and the existing one-normal-turn landing reserve.
  This conservative cap does not rely on future Fast or Haste movement. Safe landing excludes chasms, damaging terrain and currents.
- Every real action rechecks terrain, occupancy and landing routes. If the
  quarry moves beyond the remaining budget or is lost, the Dragon returns to
  nearest reachable safe ground. All possible portal exits must be safe for the
  route; planning neither selects a lucky exit nor draws RNG.
- Walls, diagonal corners, habitats, stairs, bulwarks and ordinary movement
  admission still apply. The same hook runs in precise offscreen simulation.
  Forced movement, paralysis or an external obstruction can still defeat an
  already-committed landing, in which case ordinary finite-flight fall rules
  apply. No free hover, teleport or immunity is added.
- The existing silence rule blocks all new energy-paid activations, including
  this physical action. Spell power and magickal cost multipliers do not apply.

## Blorb

An actual positive-damage melee hit adds one exact-scale caustic packet. This
uses ordinary corrosion resistance, gas immunity, absorption, death handling,
source attribution and combat-audit accounting. It creates no gas cloud. Misses,
zero-damage contacts, projectiles and already-terminal victims add nothing.

## Killspawn

Each actual positive melee hit adds half its final actual HP damage as a finite
wound. The budget is rounded to nearest once (halves upward) after the existing
bleeding resistance resolver. No currently authored ring reduces bleeding; this
does not invent a new resistance. Integer installments across the next three
normal upkeeps sum exactly to that budget. Small budgets can finish sooner.

Buckets are `killspawnBleed:<dueTick>:<sourceId>` and add together for overlapping
hits. `killspawnBleeding` is the maximum remaining wound deadline, not a damage
multiplier. The exact `killspawnBleed` packet avoids a second generic scaling or
resistance step, while retaining ordinary absorption, protection and lifecycle
checks. It continues after its source dies or leaves the floor. No work is tied
to animation timing, actor speed or visibility.

Normal and offscreen upkeep share the same settlement function. Buckets are
removed before dispatch, preventing duplicate damage on repeated settlement or
death rescue. Cleansing and Blood Letting remove all remaining wound buckets;
ordinary Bleed Them status behavior is unchanged. Saves, replay state and both
temporal snapshot systems retain conditions normally. UI merges both forms into
one Bleeding meter with their maximum remaining duration, without exposing keys.
Command and save/load admission reject malformed bucket names, non-integral or
nonpositive budgets, invalid source/deadline numbers and out-of-range timers.
Admission checks syntax and retained timer/budget pairing, not deadlines against
global time: lazy inactive floors and temporal copies keep their own clocks.

## Presentation and verification

Observed takeoff/landing use E29 unfold/fold marks. Successful caustic packets
use the existing E72 impact; wound application and actual bleed ticks use E71
red droplets. Inspection and the bestiary use short action/cost descriptions.

Focused tests are `revision52_enemy_followups` and `revision52_dragon_flight` in
corpse-core, plus `revision52_followup_copy_is_revision_scoped` and the combined
Bleeding meter regression in corpse-desktop. Test commands and exit receipts are
under `verification/revision52-enemy-followups`. The full workspace gate is
required after integration; focused tests alone do not establish a full pass.

## Complexity

Contact admission is fixed species lookup plus the existing damage packet cost.
For C actor conditions, wound application scans O(C), creates at most three
buckets, and draws no RNG. Due settlement is O(C + D*P) time and O(D) scratch for
D due source/tick buckets and ordinary packet/lifecycle work P. At most three
future tick buckets per recent source remain; work scales with distinct recent
attackers rather than raw hit count. Normal and offscreen actors run it once
per normal turn; haste does not run it.
Pure command/storage admission scans O(sum C) persisted actor conditions,
including carried pilots, with O(pilot nesting depth) stack space. Serialized
syntax checks reuse the existing O(serialized fields) boundary traversal.

Dragon planning is conditional on an eligible actor/action and at most 3600
floor cells V. It builds at most E=8V departure edges with K total portal exit
outcomes, then reverse minimax Dijkstra costs O((V+E+K) log V) time and
O(V+E+K+A) scratch for A occupants. Up to two route fields are built per ordinary
takeoff attempt or active movement action. A Fast takeoff comparison projects
both route graphs into two states per cell (fresh action / earned continuation)
and runs an additional reverse minimax field for each. This multiplies V, E and
K by at most two, retaining the stated asymptotic bounds, but is extra planning
work rather than a cached scalar division. The independent finite-flight safety
check still uses movement-attempt costs and does not rely on Fast. The Haste
normal-turn conversion is O(1); the scoped action tag is O(1) transient state per
synchronously nested action, cleared on return, error and unwind. No new save or
RNG state is introduced. Authoritative admission helpers add
O(V*H + E*T): H includes per-cell terrain, hazard, equipment/condition and
portal-alias queries, and T includes per-edge traversal and portal-outcome
checks. These helpers may scan authored terrain or portal arrays; the overall
claim is deliberately not plain O(V). Actor identity/plane/occupancy scans are
O(A*I), with I their existing helper cost. No persistent route cache is added;
changes are re-evaluated. Timings are unmeasured estimates until profiled on
populated, portal-heavy depth-38-to-40 floors with and without rendering.

## Fast/Haste aggregate correction

The final aggregate found an already-paid resting Dragon could lose its second
landing step because the generic Fast sleep guard ran before the landing-only
exception. Both active and exact offscreen continuations now finish that paid
landing before returning to rest, without waking, attacking or activating flight.
The prior two-mud-cell tie fixture assumed two unconditional Fast attempts: under
the requested rule it is three ground actions versus two aerial actions. Its
replacement compares actual dispatch trajectories across Fast/no-Fast,
Haste/no-Haste, bonus/normal starts and one/two sticky cells, including genuine
same-turn ties. The applied correction passed the shared-profile core library
run: 717 tests passed, four ignored, retained under
`verification/r52-aggregate-corrections/core-01.log`. No previously failed
aggregate receipt is relabeled as a pass. The subsequently added flight-budget
boundary tests are not covered by that run and await their own focused receipt.

### Takeoff-to-upkeep safety boundary

A source trace found the prior five-move admission cap contradicted the active
pursuit budget: takeoff set the timer to six, the next ordinary upkeep reduced
it to five, and active pursuit reserved one tick, leaving four moves. The Dragon
could therefore pay eight energy and then fold in place on the unchanged route.
Admission now uses `FLIGHT_TURNS - 2`; the six-tick duration, eight-energy price,
active pursuit budget and reserved landing margin are unchanged. This is one
constant comparison change, with no added graph, save state or complexity.

Two actual-dispatch regressions cover Fast and non-Fast Dragons: five required
moves incur no payment, while the maximum admitted four moves pay once and land
with exact positions and remaining timers. The old boundary was source-traced,
not executed as a failing baseline. Execution of the corrected tests and full
core suite passed 719 tests, zero failures and four ignored (verification/r52-aggregate-corrections/core-02.log); no executed failing baseline is claimed.
