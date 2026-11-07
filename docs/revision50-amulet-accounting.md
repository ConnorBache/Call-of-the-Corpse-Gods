# Revision 50 amulet energy and action accounting

The latest designer instructions replace the original cooldown scaffold. Every
amulet has 50 base item energy. There are no amulet cooldowns, no automatic or
level-up recharge, and no resource increase on equip/unequip. Appropriate reduces
an authored activation cost by 10, with a minimum cost of zero. Spellslinger keeps
one action-free activation per serialized normal turn, shared across all of one
owner's amulets and targets. Further activations consume an action.

`AmuletSpec` contains the explicit capacity (50) and activation cost. Effect
modules must supply authored costs; missing or invalid specs reject without
mutation. This module does not supply effects, targets, loot or effect defaults.
`AmuletState` is stored optionally in `Item.amuletState` and holds stable item
identity and energy. The old draft `ready_at_half_tick` property remains readable
for wire compatibility but is never checked, advanced or reduced by gameplay.
Newly initialized state omits that obsolete zero-valued property.

`initialize_item` can materialize an uninitialized amulet at 50 once, but refuses
to replace existing state. `plan_item_activation` is read-only: an otherwise
valid item with missing state projects its initial 50, and only the successful
commit stores the resulting spent state. Cancelled targeting, rejected effects
or invalid payment never create or refill a reserve. Effect/target admission and
resource commit must live in the caller's atomic command transaction.

Item energy pays first, then player energy covers the remainder. Planning
admits the entire cost before either pool or the action ledger changes. Combined
insufficiency rejects without spending anything; stale commits also preserve
both pools. Appropriate can reduce an activation to zero cost without requiring
positive item or player energy. The previous partial-funding guard is removed.
The item must be worn to activate. Pickup initializes its authored 50 energy
once, preserving existing charge after dropping/reacquiring or equip changes.

The shared serialized `Actor.amuletLedger` retains the last free normal tick,
last activation tick and monotonically increasing commit sequence. A plan retains
specification, item identity/state, ledger, player energy, tick, blessing flags
and whether item state was absent. Commit recomputes the plan before any mutation
and rejects changed inputs or repeated commits. Owner player-energy expenditure
is recorded through existing resource history and Half Here synchronization.

Raw Run/Snapshot/Future cloning carries the new optional item and actor fields.
Native field generation preserves them. R50 item state/ledger admission rejects
malformed identity, nonfinite/negative or over-capacity energy, malformed ledgers
and use of these new fields in old revisions. Full Engine50 save/replay admission
remains subject to the overall release gate. The state fields do not introduce
production amulet generation or imply that all effects are admitted.

The approved Ring of Reaping now accumulates actual direct HP damage to hostile
creatures across attacks and turns. Every complete 20 HP restores one energy to
every owned authored amulet, including backpack amulets, capped at 50 each.
The shared player `reapingLedger.damageCarry` retains only the remainder in
[0, 20). Whole awards that cannot fit are lost, even if every amulet is already
full; unequipping the ring neither accrues new damage nor erases existing carry.
Duplicate worn Reaping rings do not multiply the reward. Missing amulet state
already represents the initial full 50 and is not materialized or refilled by
Reaping. Player energy, ability reserves, activation credits and RNG are untouched.

Eligibility is resolved from pre-impact source/target snapshots. Player-owned
Half Here/dream endpoints use the player's one ledger and inventory. Allied
creatures, Mini-Me, clay soldiers and decoys cannot earn player credit, even if
carrying copied rings. Self/shared-owner, allied and neutral targets do not count;
Discord's explicit temporary hostility can make a former ally or neutral eligible.
Hostility created by provoking a neutral target does not retroactively qualify
the provoking hit. Credited HP is after mitigation, resistance, immunity and
Fated Duel admission, capped at the owner's positive pre-hit HP before death
rescue. Revival therefore cannot subtract credit, and overkill cannot add it.

Direct paths include landed melee/ranged strikes, Execute/Witch Hatred terminal
strikes, Lightning/Firebolt/Acid/Iceblast/Floods impacts, Nova's immediate blast,
and fresh Bombardier/Inventor/Ruin special-ability impacts when actually sourced
from a player-owned actor. Ruin Pulse is the cast-time attack packet emitted by
`monster_ruin::special_action` after range, cooldown, energy and LOS checks; it is
not a periodic environmental pulse. Ordinary Ruin enemies and allied copies earn
nothing for the player. Retained activation-time sources survive earlier victim
callbacks in a multi-target impact, including self-death. Poison/venom, Roots,
bleeding, persistent fire, gas explosions/propagation, terrain/falls and Bonded
secondary damage remain environmental/status paths and never accrue credit.

The Reaping field is part of Actor cloning, raw serialization, Snapshot and
Future anchors. Admission rejects null, nonfinite, negative, 20-or-more carry and
pre-R50 use. The general raw traversal also checks nested initial-player, pilot,
remembered and dream payloads. Timeweave movement carries the live ledger through
its existing candidate transaction; it does not rewind or independently re-award
damage. The overall R50 release gate is unchanged.

## Verification

Four module tests cover authored50 initialization, no reinitialization, unknown
and invalid specification rejection, exhausted fallback, split funding and
combined insufficiency, Appropriate's cost reduction and zero cost, ignored legacy
cooldowns, global free credit and stale commit atomicity.

The `revision50_amulet_energy` integration tests cover read-only planning,
equip/no-passive recharge, item/player expenditures, shared free credit across
items, stale-plan rejection, old cooldown compatibility, raw serialization,
Snapshot restoration, item identity, revision-sensitive admission, partial
item/player split, insufficient combined funds and unworn rejection. They are
awaiting the coordinating final native test run. The effect module has separate
projectile/targeting tests. No platform verification is implied.

## Complexity

Pure plan/commit and individual state/ledger validation are O(1) time and memory.
Runtime item plan/commit scan I inventory entries and B blessing IDs, adding
O(I+B) work, with O(1) plan payload. Commit additionally uses existing resource
history and Half Here synchronization, whose actor/visibility/copy costs are not
included in the scalar accounting bound. Run admission scans I player items,
A actors with J actor-local items and G ground items: O(I+A+J+G) time, O(1) scratch.
Snapshots retain existing cloning/COW costs plus O(I+J+A) scalar fields where
present. The per-owner ledger and each item state are O(1) retained data. There
is no timer scan, render-clock logic, RNG or periodic energy refill in this
module. UI inventory/tick projections already include item state and therefore
invalidate on actual energy changes. These are source-derived bounds, not
measured performance results.

## Reaping equipment and verification boundary

Both temporary worn-Reaping run and Equip guards are removed together now that
the approved effect is implemented. Equip/Unequip/Drop continue through ordinary
capacity and settlement checks. The replacement combat-ring regression covers
ring/ring1/ring2 aliases in normal and haste phases, subsequent commands and
continued run admission.

`revision50_reaping` adds 16 deterministic focused tests for exact 19+1 carry,
cross-turn and multi-target accrual, fractional resistance/overkill, duplicate
rings, all owned amulets, spent source-amulet repayment, cap overflow, missing
state, unworn rings, friendly/neutral/self/allied exclusion, Discord hostility,
DoT/environment/Bonded exclusion, DR/immunity/Fated Duel, melee/ranged/Execute,
resurrection, Half Here/Mini-Me/clay exclusion, retained source after self-death,
fresh Ruin versus allied Ruin, raw save/Snapshot/Timeweave, malformed ledgers and
legacy behavior. The Future accounting regression also restores a saved 19-HP
Reaping remainder alongside item energy and ability reserves. Tests are authored;
execution is awaiting the coordinating final native run. No platform verification
is implied.

Reaping adds O(I+C) worst-case time per eligible direct-hit callback and O(1)
scratch, for I owned inventory entries and C ownership/hostility condition keys.
Hash-backed condition lookups are expected O(1), conservatively O(C) worst case.
The initial boolean worn-ring scan and award scan both traverse I, without a
per-energy or per-20-damage loop. I includes unlimited trinket entries, so it is
not a fixed-size bound. Noneligible attacks normally short-circuit before scanning
inventory. A multi-target effect with K credited impacts adds O(K*(I+C)) to the
existing geometry, actor lookup, packet/death, COW and source-snapshot costs,
not to be mistaken for the total cost of the spell. One scalar ledger per actor
adds O(A) worst-case retained storage across A actors, although normal execution
materializes only the player ledger. Existing full-run snapshot/Future/raw-wire
traversals preserve it with their existing clone/serialization costs. Admission
adds O(A) typed checks plus O(V) raw-node traversal already required at storage
boundaries. No cache, RNG, timer or per-turn scan is introduced. These are
source-derived bounds; timings are unmeasured.
