# Revision 50 authored amulet effects

This is draft R50 source support. It does not open the global R50/new-run gate or claim production item-generation admission. The existing serialized `use-at` command owns targeting and activation, with a shared player activation ledger and item energy state. Invalid geometry or resource admission occurs before the action/haste/resource clocks. The engine's outer candidate transaction rolls back payment, effects, RNG, logs and clocks together on execution failure.

## Authored first six effects

| Amulet | Base energy | Executed effect |
| --- | ---: | --- |
| Lightning | 25 | 50 HP direct lightning packet. Straight ray pierces creatures, including allies, until blocking terrain or map bounds. |
| Firebolt | 20 | 80 HP direct fire packet to the first creature. Burns that creature and ignites flammable layers on the path and blocking impact. |
| Poison | 15 | First creature receives 6 HP poison per normal tick for 20 ticks. Reapplication refreshes 20, never adds rates. |
| Iceblast | 30 | Visible center and surrounding 3×3 area, 30 HP cold and 3-turn slow, including player/allies. Walls and closed corners shield victims. Ice Immunity blocks both. |
| Acid | 20 | First creature within eight physical tiles receives 40 HP acid and DR −10 for six normal ticks. Reapplication refreshes, not stacks. |
| Blinking | 20 | Teleports to a visible empty walkable destination within ten Euclidean tiles without a walkable route requirement. Shared arrival hooks handle fire/contact, stairs and other destination effects. |

Lightning, Firebolt and Poison continue past the clicked tile; it selects direction, not maximum distance. Spatial rays use the existing translated-ray plan, a traversal limit of twice the number of cells, and one visit per linked group. Stochastic links consume serialized RNG only on launch and only at forks reached before collision. Lightning deduplicates creature identities so alias/cycle encounters do not repeatedly damage the same creature. No added water conductivity or chaining exists. Fated Duel is checked against the immediate enemy source before the entire attached amulet attack: blocked Acid does not reduce DR, blocked Iceblast does not slow, blocked Poison does not start delayed damage, and blocked Firebolt does not directly ignite the protected creature. Already-running poison and environmental effects retain their existing behavior.

Damage uses the explicit exact-scale direct packet entry point. In particular, Firebolt's 80 is not passed through the legacy fire quarter-scaling. The ordinary named resistance/immunity layer still applies. Poison Resistance halves the final six-HP tick. Corrosion Resistance halves both Acid damage and its applied DR penalty but not duration. Sustain Strength also applies to the remaining temporary DR reduction. Poison and corrosion state serialize in actor conditions and tick on the existing normal-clock/offscreen paths, never haste-only phases.

All amulets start with 50 item energy. There are no cooldowns. Appropriate subtracts 10 from the authored activation cost, clamped at zero. Spellslinger makes one activation action-free per normal turn across all items and targets; further activations consume an ordinary action. The accounting module owns reserve-first atomic split payment, player fallback and Reaping. No passive/equip/level-up refill is added by the effect module.

## Knowledge boundary

Runtime directional target admission never tests for a creature along the ray. A legal empty shot or hidden terrain collision still spends the activation, avoiding hidden-occupant scouting through success/failure. Iceblast and Blinking explicitly require currently visible target cells. The UI receives `target_blocker_ui` and `preview`; previews use observed/remembered terrain and door state, remove unknown portal groups, stop at unknown geometry and stochastic forks, and use visible creatures only. They never invoke the simulation trace or draw gameplay randomness.

## Verification scope

Deterministic tests cover the first-six effects, true damage scaling, friendly fire, unlimited rays, first-target behavior, fire propagation integration, six-HP poison duration/refresh, damage/DR resistance, corrosion expiry, Ice Immunity, Blinking arrival fire, invalid target and partial-payment no mutation, shared Spellslinger dispatch, saved poison/resource determinism, and cycle termination. All 20 first-six focused tests passed with the pinned native toolchain, optimized test profile, assertions and overflow checks enabled. Receipt: `verification/revision50/amulet-effects-focused.log` and matching `.exit` (0), rerun directly from the just-built test binary after the combined Cargo run. The combined run is recorded separately and includes a failing newly added Wave/Teleport fixture outside these 20 tests. This focused pass does not replace the final workspace run and is not native Windows/platform verification.

## Complexity

Let V be current floor cells (native supported maps at most 3,600), A current-floor actors, P portal/link groups, I inventory entries and B player blessings. Each admission scans O(I+B) and uses the existing visibility cost when a visible destination is required; directional admission has no ray scan or RNG. A committed ray pays the existing `portals::ray_plan` bound O(P²V + PV(P+1)) plus at most 2V directed steps. Occupancy/plane/alias checks and deduplication add O(V·A·P + A²) worst case, O(V+A) extra storage, plus the existing damage/death/retaliation/resource hooks per hit. Portal helpers can scan bounded link groups and actor-owner/plane lookups; those transitive costs are not claimed constant.

Firebolt additionally pays one FireAdapter initialization/closure over the floor, including its existing fuel promotion, reaction union, explosive-gas closure and resulting damage hooks; see propagation/environment ledger entries. Iceblast scans A actors with constant local 3×3 geometry, then pays existing packet/status hooks per victim. Blinking admission includes visibility, collision and equipment/terrain queries; commit pays existing arrival/travel hooks. Poison/corrosion add constant state and work per affected actor to existing normal/offscreen status loops.

Each UI preview pays `vision::observe_terrain`'s documented full observation cost, an O(floor payload) clone/knowledge projection, known-link filtering, and the same bounded ray-plan/trace geometry plus observable-actor queries. It retains O(V+A+floor payload) scratch, with no renderer-native work inside this core module. Timing has not been measured; exercise repeated targeting on maximal linked floors and flame/gas chains before optimizing.
