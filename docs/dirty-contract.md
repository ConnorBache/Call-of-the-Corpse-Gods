# Dirty-derived-state contract (r39 profiling baseline)

Read-only trace completed 2026-10-30. Implementing owner-player health cache is separately authorized by scheduling coordinator. No blanket claim that public direct writes are automatically tracked.

## Player maximum health dependencies

`equipment::sync_health` at equipment.rs46–68 first runs Blood and Fury unlock, clones player, derives all stats, then computes `max(1, base_max_hp(default100) + 6*effective.defense + effective.max_hp(default0))`. It initializes missing base_max_hp even when max HP does not change. Positive max gain heals the gain; negative gain clamps current HP except an equipment change subtracts the full loss. This distinction must survive caching.

For the current implementation, defense/maxHp derive only from owner base `stats.defense`, `stats.max_hp`, `conditions["buff:defense"]`, `conditions["buff:maxHp"]`, equipped item bonuses/base bonuses (each component separately rounded) and all equipped enhancement bonuses. Player ability modifiers currently affect offense/dodge/DR/perception/energy, not these two health inputs. A generalized stats cache has MORE dependencies and is not interchangeable.

`divine_combat::unlock_blood_and_fury` checks owned Fight Long + Blood Weapon and one-time achievements and logs before the early max-unchanged return. Skipping this side effect just because maxHP is clean is wrong. Track ability ownership dirtiness separately, or leave the unlock check until ownership indexing is available.

## Minimal health API

Runtime-only `Run.health_dirty: HealthDirty` with `#[serde(skip)]`; Default = invalid. O(1) scalar input stamp includes player id, base_max_hp Option/float bits, stats.defense/max_hp, buff:defense/maxHp, current maxHP output. Explicit invalidation for inventory health-affecting edits. `sync_health` remains a forced mutation-boundary settlement and records clean stamp after performing original behavior; `flush_health` uses O(1) clean check and otherwise calls forced sync. Do not hash/clone entire inventory or actor each flush. No global actor_mut invalidation: damage/regen/movement would cause needless recalculation.

Fresh deserialize/new state = dirty. Run clone retains valid stamp only while dependencies unchanged. Whole-player replacement, offscreen/destination proxy creation, explicit debug changes, time-state restore/import invalidate. For public/test direct inventory edits, explicit invalidation or forced sync is required; wrapping all public inventory writes would be a larger separate API migration. An O(1) scalar stamp handles scalar direct writes but cannot detect arbitrary in-place item bonus edits.

## Existing forced settlement mutation boundaries

- equipment.rs: equip/unequip mutate inventory, then `settle` -> reconcile -> sync_health(true). `settle_removed` used by drop/throw/tinker. Keep equipment=true delta behavior.
- inventory.rs: heart consumption adds bonus to player.stats -> sync_health(false); equip/unequip branches redundantly force a second sync (can later flush after original forced settle).
- potions.rs: defense and other buffs target the resource owner then sync_actor_health; healing/full-rest clears effects then same owner settlement. HP scalar fallback detects defense/maxHP buff changes, but explicit invalidation is clearer.
- engine.rs: early upkeep removes `buff:*` when buffTicks expires, then sync_health(false) for player/shared endpoint. Late unconditional maxHP recomputation can become flush only after other dependency writers covered.
- progression.rs: eclipsed-defense offer adds5 to stats.defense; all chosen offers call sync_health(false). Acquisition also requires Blood/Fury unlock handling.
- crafting.rs: adds enhancement bonus and consumes heart, then sync_health(false), even when the crafted item is equipped.
- tinker.rs: grants equipment ability and renames item, then equipment::settle_removed. Stat bonuses unchanged, but ownership/achievement dirtiness changes.
- abilities.rs Remember copies passive traits -> sync_health(false); Blood Letting clears statuses/buffs/Remember ownership -> sync_health(false).
- debug_floor.rs: add/remove/finish setup call forced health settlement. playtest.rs finishes loadout with forced sync.
- royal_honey.rs: force health sync before refill; preserve order.
- migration_pipeline.rs/migrations.rs: normalize old saves with dedicated legacy formula, then native dirty cache starts invalid; do NOT replace legacy calculations with native forced sync.
- travel.rs: falls/transfers may replace player with a clone; scalar identity may be unchanged, but mark cache invalid for safety at the transfer boundary if moved state could differ.
- offscreen.rs proxy and travel.rs depth_view clone Run then replace player identity/conditions/abilities/inventory; invalidate cache there. Their caches must never leak back as authoritative player cache through field merges.
- temporal.rs: Engine load/new, snapshot restore/Future rewind/rebase use canonical serialized state. Runtime cache skipped from JSON must be rebuilt/invalid; explicit snapshot_clone.rs must carry invalid or safe cloned cache field after model addition.

## Equipment ownership contract

`equipment_abilities::reconcile` derives base, grants, ability order and active retained powers from inventory and player abilities/equipment_abilities/ability_order/remembered deadline conditions. It can remove maintained links, dismiss Mini Me across floors, trigger Phase Through crushing, initialize newly granted ability state, adjust Waking Nightmare clock after re-equip and clamp reactions. `sync_ownership` additionally validates every inventory item, captures flight/capacity state, syncs shared bodies and settles flight-loss deaths. A single dirty flag must cover all those dependencies and cannot simply skip safety settlement when a flight/status event occurred.

Relevant mutation sites: equipment equip/unequip/drop/throw; crafting enhancement changes; Tinker grants; Remember acquisition/expiry; choose_offer/promote_learned; generated/debug loadouts; inventory removal/dilution/drift and item corruption if equipped; lost powers through status clearing; state import/restore. Inventory pick-up of non-equipment does not alter ownership, but existing validation policy still validates all items. Preserve unsupported-state rejection; don't eliminate validation by treating cached ownership as proof every newly added item is supported.

## Shared-body contract

`half_here::sync` first invokes `dreamwalking::sync`, then validates Half Here ability+active flag+clone id+owner+alive/status, possibly dismissing across all floors. It copies these player fields to Half Here: abilities/active/ability_order/equipment_abilities (minus Half Here), stats/base_max_hp/hp/max_hp/energy/blood/damage_bank/nightmares/reactions/adaptive_hearts; clears adaptive_dr. Body position, local conditions, state and plane identity remain separate. Dream body copy is narrower: hp/maxhp/energy/stats/abilities/base_maxhp; it clears body active list.

Thus shared-body dirty invalidation must cover ALL resource writes, ability/counter changes, clone lifecycle/owner id, floor/plane transition and terminal status. There are many direct HP/energy/counter assignments (combat, upkeep, regeneration, potions, time travel, feeding, enemy targeted powers), not all routed through actor_mut. Do not mark projection clean merely because maxHP is clean. A dedicated shared-owner resource representation is a larger structural refactor; initial O(1) absent-endpoint index plus explicit resource-change dirty is a safer bounded migration.

## Required tests

1. Quiet tick clean health: no effective-stats call from flush; identical canonical state.
2. Heart defense/maxHP bonus; potion defense renewal/expiry; buff clear; negative fractional bonus rounding.
3. Equipment true/false loss semantics; two-handed slot displacement; craft equipped item; Tinker ability unlock.
4. Blood/Fury acquisition while maximum HP unchanged logs exactly once and at original phase.
5. Direct scalar mutation fallback; explicitly invalidated inventory mutation; newly deserialized old fixture.
6. Revert/Future restore across buff/equipment change; save/load clean cache must not omit settlement.
7. Half Here + dream body local condition preservation; full/partial HP/energy changes; death/dismiss/floor transfer.
8. Offscreen proxy cannot poison owner cache.

## Implemented conservative HP-only boundary (resumed 2026-10-30)

Only the repeated unconditional world-tick health settlements are eligible for `flush_health`. Forced equipment, craft, potion, heart, progression and other mutation-boundary settlements retain original behavior. `engine::dispatch` invalidates once on admission to cover arbitrary public `Run.inventory` edits between commands, including same-length replacement or in-place bonus writes. Therefore this version removes redundant within-command derivations, **not** all health derivations on quiet commands. Direct callers of `flush_health` after arbitrary public inventory bonus edits must call `health_dirty::invalidate`; this is a documented internal API contract, not an automatic mutation observer.

The fixed-size stamp contains player identity, base-HP field tag/value, defense, maxHP-stat field tag/value, two health buff scalars, output maxHP, two version scalars and inventory length. Expected O(1) time/space; no collection hashing. Inventory length changes conservatively invalidate on pickup/removal even when health cannot change. In-tree same-length equipped changes already force settlement. Clone retains a safe scalar stamp; deserialization defaults dirty; changed proxy identity/inventory length fails stamp comparison. Runtime cache equality ignores warmth and serialization skips it.

A clean flush still invokes the original Blood and Fury unlock predicate: O(B+H) worst case for abilities B and achievements H, plus log allocation only on actual unlock. The stamp check alone is O(1); the whole clean flush is **not** falsely described as O(1). No equipment ownership or shared-body synchronization is optimized by this subset. HP-only damage and regeneration do not invalidate maximum-HP derivation.

### Second-pass mutation review

`inventory::take_one` now invalidates health in O(1) if the selected entry has an equipped field. This closes a same-length consume-and-replace edge case: currents can invoke player movement into Borge interaction during a tick, consuming a heart and appending a shard between the early and final flushes. Normal hearts are not equipped, but the existing stats function accepts any equipped entry; the optimization must not assume stronger validation than the original engine. A regression checks equipped heart removal followed by same-length replacement against original forced settlement.

Other reviewed same-length writes: equipment slot changes force settlement; crafting bonus insertion forces settlement; Tinker calls `settle_removed`; Fury craft active toggles do not change the unconditional enhancement bonus contribution; item-magic reveal and potion identification/dilution only change presentation/state unrelated to defense/maxHP; Epidite blessing affects ground items, not carried items. Equipment-generation bonus mutation occurs while constructing items, not on live equipped inventory. Save migration operates on reconstructed canonical state. Blood and Fury still runs once at each original settlement site, before any dirty-path derivation, and preserves the achievement/log/archive behavior on the clean path.
