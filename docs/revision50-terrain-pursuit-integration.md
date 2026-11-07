# Revision 50 terrain and pursuit implementation boundary

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

Cogworks pit, shared-HP wall and free removal have runtime command integration.
Clay Army has runtime movement, placement and end-turn attack APIs, with engine
scheduling, collision guards and phase-aware path masks wired by the integration layer. Pursuit includes runtime activation, route
planning, persistent condition state and the movement-credit hook; engine
scheduling and sight-display rendering are caller-owned. Native tests were not run: toolchain installation is
pending approval. Legacy revisions are rejected before mutation.

## Terrain hooks

`Floor.revision50_terrain` persists `Vec<Modification>` per floor, including
offscreen/temporal snapshots, and defaults to missing for legacy saves. The schema
generator preserves this native extension. `cogworks_preview` is the authoritative pit/wall footprint, including
full-boundary and creature occupancy checks. Both preview and actual floor paint
should use the existing Kative palette, not newly invented colors. Mode 3 uses
`remove`, costs zero, and restores the selected modification's previous tiles.

`activate_cogworks` checks ownership, current creature occupancy and ability-reserve energy before
committing a pit or wall. `preview_for_run` is knowledge-limited: hidden creatures cannot
change the preview. Command::Ability with target=1/2/3 and missing x/y selects
`r50CogworksMode` without time or energy; x/y coordinates activate the stored mode.
Default mode is 1. Mode 3 removes selected Cogworks terrain for zero energy;
Clay Soldiers are not eligible for Cogworks removal.
Register `MODE_KEY` as a persistent condition. Failed validation does not mutate
terrain or spend energy. The dispatcher already enters the player's plane.

Standalone `commit` charges exactly 20 and stores previous terrain and a
single 200-HP pool for all five wall cells. It does not itself run turn scheduling,
hazard contact or combat. Runtime uses `Run::floor_mut`, which records
the mutation journal's COW evidence. Optical and lighting caches compare exact
terrain dependencies on the next query, including same-tick changes; no manual
cache-clear function is required.
The authored wall is one modification. Any wall-cell strike subtracts from its
shared HP pool; at zero the complete footprint restores its exact previous
terrain and removes the record. Every restored cell enqueues the existing ichor
opening update. Shattering that intersects one wall cell destroys that whole
object, while ordinary obstacles retain Shattering's original radius and Clay
stays unbreakable. Restored underlying terrain is not shattered again in the
same impact. Mode 3 free removal is unchanged.

A hostile enemy's ordinary movement attempt into a wall becomes one automatic
hit: the existing enemy weapon/offense base, ordinary 0.75..1.25 damage variance
and half-damage/ceiling tuning, no wall defense, dodge or creature-only triggers.
It consumes one serialized RNG draw and the action, without moving the attacker.
Player, friendly and neutral bumps are unchanged (Discord-hostile creatures may
strike). Forced movement and movement-only haste cannot attack. Phase-through
retains ordinary wall traversal. Ordinary pursuit routes remain preferred; only
when there is no route does a second field treat Cogworks walls as breachable.
That fallback retains other terrain, habitat, hazard, portal, Clay and obstruction
rules. The separate inactive-floor normal pursuit path uses the same fallback
and engine strike; it cannot skip or duplicate catch-up damage.

Boundary validation checks physical and dream records for correct footprint,
unique cells, legal prior tiles, matching live terrain and a finite shared HP
pool in (0,200]. Invalid data is rejected, never repaired. Temporary amulet terrain,
Black Pit restoration and horn-created expiring walls cannot be overwritten by
persistent Cogworks placement. Full footprints and HP survive ordinary save and
temporal snapshot restoration. Visible wall tooltips show current shared HP;
remembered terrain never queries hidden current HP.

Clay Army's `activate_clay(r,x,y)` validates an ordinary one-tile move through
an internal full-occupancy plan, then uses the real movement pipeline. Public
`clay_preview_for_run` ignores hidden occupants, as does the Cogworks UI preview.
A COW opening
snapshot plus nested `events::Probe` makes movement, departure wall, presentation
events and 20-energy payment atomic if a
movement side effect changes the outcome. The player must reach the selected
adjacent tile; random portal relocation is not silently treated as one tile.
After movement, recheck the departure tile for creatures dragged by Grappler or
other effects. Occupancy rejects the whole transaction rather than entombing or
displacing a creature. Black Pit's altered-chasm records are also an overlapping
terrain owner and reject placement, preventing its later regeneration from
overwriting an unbreakable soldier. The nested presentation probe restores the
outer event batch on rejection/error and commits only after placement/payment.

`clay_blocks_run(r,actor,x,y)` must be checked BEFORE phase-through in collision
and path masks. `is_soldier(actor)` identifies temporary attack sources; block
movement, incoming attacks and direct/environment damage to these sources.
`clay_end_turn` runs once per full world turn across physical/dream floors. Each
soldier attacks adjacent living creatures, including the player/allies, in stable
floor/modification/actor order. It refreshes the current player's effective stats
and the shared greatsword base before every attack. NO abilities, active powers,
equipment grants or attack-trigger conditions are copied. Each attack uses the
ordinary engine attack/defense/reaction/death pipeline. Temporary sources receive
registry-admitted fresh monotonically increasing IDs and are removed from their
original floor on success or error, even when a reaction changes player depth.
Old-floor loops stop on depth or terminal-status changes. Terrain
metadata, not those temporary actors, is the persisted soldier state.

Paid Cogworks, Clay and pursuit activation use revision50_ability_energy. A
missing authored reserve blocks activation; no reserve capacity is invented.
Geometry-only commit helpers receive a cost surrogate, avoiding a second gate on
the player's shared energy after reserve admission. Free removal needs no pool.

Unsupported placements do not spend energy. Arbitrary angled throws have no
integer perpendicular rounding rule, so only cardinal/exact diagonal throws are
admitted. Occupied wall placements and overlapping own modifications are rejected
rather than guessing displacement/restoration mechanics.

## Pursuit hooks

Export `revision50_pursuit` from core. Runtime `activate` is dispatched from
`revision50::activate`. It checks a living visible hostile target in the player's
plane and energy-silencing, spends 10 shared energy only after admission, and
stores `r50PursuitTarget` / `r50PursuitDepth` in numeric conditions. Register
`condition_key` in the condition support registry so these fields do not expire.
`r50PursuitBonusTick` preserves the once-per-turn budget across retargeting.

`route_for_run(&Run) -> Vec<(f64,f64)>` resolves the target's current identity and
coordinates, builds passability/occupancy masks, and uses the core's existing
ordered/stochastic portal-aware distance field. Terrain checks include player
passability/event solids, ichor, bulwark, dream-plane stair restrictions and
current flight/phase state. Friendly/neutral occupancy is blocked. Other enemies
are blocked in the entire first search; only if no route exists are they allowed.

Render the returned route in the sight display. It includes source, movement
entry cells and deterministic portal arrivals, and stops at random forks rather
than presenting an arbitrary exit as certain. Movement then replans from the
actual randomly chosen exit. No pathfinding call draws RNG.

Call `grant_for_move(before:&Run, after:&mut Run, dx,dy)->bool` ONLY for successful
explicit player Move commands before advancing the world tick. It requires the
command direction to match the current first route edge and actual resulting
position to be one of that entry's legal portal outcomes. Failed movement,
attacks without movement, forced movement, floor changes, death and unrelated
movement cannot earn credit. True means schedule ONE movement-only credit, not a
free attack/action. The bonus move must share the original world turn budget;
re-entering this hook with the same tick cannot earn another credit. The function
does not clone the Run; reuse existing command snapshots where available.

`settle(&mut Run)` clears invalid target/depth after death, plane change or travel.
Call it from turn/travel cleanup. Save/load and time travel preserve these fields
through the ordinary serialized actor conditions. Actual enemy passage remains
ordinary collision/combat; the planned fallback does not teleport through actors.

## Complexity ledger entry

Let V be movement graph cells, E legal movement edges, M terrain modifications,
K cells in a modification (authored K <= 9). Pursuit route rebuild: two BFS passes, O(V+E) time and
O(V) extra memory, per movement/preview when needed. Graph construction is
caller-owned and must add the full cost of terrain/actor/portal lookups; no total
O(V) runtime claim is made for that step. `claim_bonus` is O(1). Terrain geometry
is O(K), plus K occupancy queries whose costs belong to the caller. Commit/remove
scan modifications: O(M*K+K) time with bounded K, O(K) scratch; Vec removal also
shifts O(M) modification records. `clay_blocks` is O(M*K) per collision query,
not a constant-time layer. No timings or
platform verification are claimed. Unit tests cover footprints, rejection,
payment atomicity, free restoration, live ability-free attacks, route avoidance
and fallback, target movement, legacy gating and once-per-turn credits.

Runtime route construction adds O(V*(A*P+C)+field(V,L)) per route/credit check,
where A is living actors, P portal alias lookup, C passability/contact-query cost
and L spatial links. It allocates O(V+L) masks/field scratch. Preview reconstruction
is bounded to V steps and includes repeated next-step/spatial-exit query costs.
Pursuit introduces no new cache or full-Run clone. Six additional runtime regression
tests cover target movement/death, atomic rejection, legacy gating, enemy detours
once-per-tick credits, deterministic portal arrivals and forced enemy-route
fallback; all remain authored but unexecuted.

Cogworks run activation clones only the modification vector before validation:
O(M*K) temporary memory/time, with bounded K<=9; floor COW may detach the full
floor payload on its first shared write. Occupancy checks cost O(K*A*P) for A
actors and P spatial-alias lookup; preview additionally pays direct visibility
cost for tested actors. Cache queries retain the exact-dependency and cold-build
costs documented in optical_cache/lighting_cache. Four further runtime tests
cover mode selection, save/load plus free removal, occupied-wall atomic
rejection, and hidden-occupant preview privacy. Native execution remains pending.

Clay end-turn work is O(S*A + H*(Tstats + Tcombat + A)), for S soldiers, A current
actors, H adjacent attacks, live-stat lookup Tstats and full engine attack cost
Tcombat (including reaction chains and visibility). Temporary target IDs use
O(A) memory; actor COW may detach O(A) storage once per shared block. No terrain
actor is retained between attacks. Activation additionally clones the Run's
ordinary non-Arc metadata/inventory and Arc roots for rollback; first writes may
detach full floor/actor payloads. This is not claimed to be O(1). Added tests cover
live stat updates without abilities, placement, reserve-first payment, atomic
rejection and attacks against player/allies with temporary-source cleanup.
Each transient ID checks the shared creature registry: O(d) clean or the registry's
documented dirty reconciliation cost; neither retained identity collisions nor
numeric allocator exhaustion is bypassed. Regression tests cover collision
rejection, terminal-state suppression, original-floor cleanup after a simulated
reaction error/depth change, and forbidden Cogworks removal of Clay Soldiers.

Inactive floors with physical or dream Clay Soldiers force the precise offscreen
turn path. Every elapsed world tick runs Clay after that floor's environment and
door settlement, never on the stationary approximation. The active floor is
excluded by normal inactive-floor admission, and last_tick prevents duplicate
catch-up attacks. The proxy receives the actual player's current effective stats
explicitly because its own inventory/powers are stripped; it never targets the
dead/off-map proxy player. Registry admission temporarily reserves the real
player identity, and projected dream-floor edits are preserved without replacing
the actual player's dream-state flags. Historical catch-up uses current observed
player stats, as the world has no per-past-tick player-stat history.

Offscreen admission adds O(F+M) per dynamic/involved query for F physical/dream
floors and M matching modifications. Existing elapsed-tick simulation cost gains
the Clay attack cost above once per exact tick. Tests cover forced precise
catch-up, no duplicate same-tick attacks, no active-player damage, use of supplied
live stats rather than proxy stats, and hidden-occupant Clay preview privacy.
Presentation rollback adds O(E+B) event-buffer cloning for E<=128 queued ordinary
events and B pending deduplicated explosion entries. Transaction regressions
cover riptide rejection with an existing outer event batch, Grappler moving a
creature into the departure cell, and Black Pit departure terrain overlap.

## Final approved wall integration complexity

For V floor cells, M modifications, K<=9 footprint cells and L spatial links,
wall damage validates/clones O(M*K) records and restores at most five cells;
first shared floor mutation can detach the entire floor payload. Save/command
validation across F physical/dream floors costs O(F+M*K) time and O(M*K) scratch.
A wall strike additionally pays effective-stat derivation, actor lookup and
visibility/logging costs, then one RNG draw. No dummy actor or creature-combat
reaction tree is created. The no-route fallback builds a fresh O(V+L) field and
masks only when needed: O(V*(M*K+P+B+H)+field(V,L)), with portal lookup P,
ability/obstruction/bulwark work B and hazard query H. This is not a constant-time
path query and is not cached. Shattering captures affected objects before edits:
O(M*K*C + M*K*W) work for C<=25 examined impact cells and W affected wall cells,
plus existing floor/ichor changes. No timing or release-readiness claim is made.

Added regression sources cover live placement/payment, occupied and underfunded
atomic rejection, shared damage from different cells, complete restoration,
ordinary hostile attack RNG, friendly/haste/forced/Clay exclusions, no-route AI,
physical/dream independence, same-tick sight caching, Shattering, save/temporal
state, validation rejection, expiring-terrain overlap and offscreen catch-up.
These additions await the final integrated Cargo run.
