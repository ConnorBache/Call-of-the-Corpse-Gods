# Revision 45 Nova energy cost

The designer requested Nova to spend **40 energy**. Revision 45 therefore funds
and debits each Nova attack explosion at 40 energy. Revisions 29–44, including
missing-revision legacy fixtures, retain the exact 30-energy implementation.
`abilities::nova_attack_cost` is the single revision-aware numeric source.
Canonical ability catalog prose and passive/toggle classification remain frozen;
revision-aware presentation supplies the current cost separately.

## Runtime boundary and call chain

`engine::attack_base_with_fury` calls `nova_after_attack` once after its existing
resolved hit or miss path. `nova_after_attack` resolves the same shared owner,
checks the same active power, and passes the revision-specific cost to the
unchanged `spend_shared`. A fully funded blast records that exact energy loss and
runs the existing captured-impact/source explosion. Insufficient energy does not
partially pay or explode; it removes Nova from the owner's active list. Having
exactly enough energy leaves Nova enabled until a later unfunded attack, as
before. Blocked attacks and projectiles without an actor impact never enter this
post-attack path. Passive explosion immunity remains unchanged even when the
toggle is off or cannot be funded.

The shared route covers:

- Ordinary player main-hand and offhand strikes, including each Unyielding Assault
  repeat. Assault's separate 10-energy charge retains its original priority.
- `attack` and `attack_ranged`/`attack_ranged_resolved`, including NPC attack and AI
  Hornlight/Pale-spit routes. Their own spell/repeat costs are still paid first.
- Bow and thrown-weapon inventory commands, including revision39 Shard hits.
- Recursive deflection, Never Without a Fight counterattacks, Mini Me offhand,
  and Pale repeat bites, with one Nova funding attempt per resolved attack.
- Half Here counterattacks, which continue to draw from the shared player owner,
  and Dreamwalking's existing plane/physical-body activation rules.

Independent NPC divine toggles are still rejected by the ordinary full-run
supported-mechanics admission boundary. Tests of these lower-level attack and AI
routes do not claim that this update enables independent Nova NPC runs. No
admission rules were broadened. Offscreen simulation and haste phases retain
their existing admission and attack scheduling; no additional simulation loop,
automatic activation, or upkeep is added.

Nova activation/deactivation is still free, including when current energy is
below the per-blast cost, and Nova remains absent from turn-maintenance costs.
Unspeak's `highest_energy_cost` uses the runtime Nova cost only at revision45+
when considering that ability, rather than parsing the immutable historical
30-energy catalog description. Other costs, maximum selection, and old revision
parsing remain identical. `enemy_traits::maximum_cost` and the AI wrapper cover
intrinsic enemy-trait costs only, not divine Nova, and need no new rule.

Desktop creature ability descriptions and the offhand intervening-effect funding
forecast use the same helper. The player ability panel's revision45 display
updates are maintained with the other wording changes. These adapters remain
read-only and retain the old cost for old runs.

## Persistence and validation

No schema, resource field, RNG draw, event order, explosion damage rule, actor
profile, or world-generation rule changes. The already recorded simulation
revision chooses the numeric cost after load, replay, transactional snapshot
restore, and temporal travel. Revision admission/header/default plumbing is a
separate change. Future/Weave mechanics are not changed by Nova.

`crates/corpse-core/tests/nova_revision45.rs` adds deterministic controlled cases
for legacy cost30 and revision45 cost40; 39/40 funding; melee/ranged hit and miss;
free toggles/upkeep; actual bow/throw commands; lower-level NPC/AI attacks;
Assault/offhand/deflection/repeat costs; Half Here owner funding; blockers;
Unspeak's maximum; and save/load, snapshot restore and command replay. Desktop
unit tests check legacy/current creature cost prose and the 39/40 offhand forecast
boundary without mutating the source run. Existing `nova_commands` differential
traces and `shard_weapon_independent_r39` preserve legacy executed behavior.
The implementation worker did not run builds; execution results are supplied by
the lead verification gate, rather than implied by this coverage list.

## Complexity

Let A be current-floor actors, K the relevant active/ability count, C the fixed
catalog size, D the examined description bytes, and P the number of resolved
attacks (including existing recursive attacks). The new cost lookup is O(1)
time and O(1) extra memory per existing Nova funding attempt. It adds no RNG,
allocation, traversal, cache or serialized field. The original active-power scan,
owner lookup, resource-history/visibility work, actor copy-on-write mutation,
explosion traversal/propagation, damage, death, and shared-body synchronization
remain transitive costs of the attack; this does not make a whole attack O(1).
Across P existing attack resolutions, the added dispatch work is O(P).

Unspeak still initializes intrinsic metadata and scans the target's K powers,
with a linear search in the C-entry fixed catalog for each power and a description
scan of up to D bytes for non-overridden powers: O(K * (C + D)), plus unchanged
actor lookup, intrinsic metadata/condition work and any actor-block copy-on-write.
The Nova override replaces its old description scan with constant comparison and
arithmetic, using O(1) additional memory. No shared cache is added.

The offhand forecast adds one O(1) helper lookup to its existing detached combat
projection. Creature prose formats one bounded two-digit cost with the existing
string allocation; added work and memory are O(1). Catalog lookup, output text
construction and native text layout/rasterization retain their original costs.
These are source-derived complexity bounds, not measured timing claims. Run the
focused tests plus the workspace gate and profile representative combat/inspection
scenes for timing evidence.
