# Revision 51 damage rounding

Designer direction: round damage to the nearest whole number in all cases.
Revision 51 rounds nonnegative damage to nearest, with exact halves upward:
2.4 becomes 2, 2.5 becomes 3, and 0.4 becomes 0. There is no minimum-one-damage
rule. Negative results after mitigation remain zero.

Ordinary attacks round after variance, attack multipliers, DR and attack tuning.
Direct and environmental packets round once after their damage tuning and all
applicable resistance. Gas vulnerability is applied before resistance and that
final rounding. Gas and hemophilia no longer pre-ceil their inputs. Explosions
round each victim's half-maximum-HP amount separately. Bonded damage is a new
packet and rounds the half of its originating damage separately. Wall strikes,
fall damage, status ticks, Blood Weapon HP payments and offscreen processing use
the same revision-aware helpers. No additional RNG is consumed.

Damage events, chronicle text, inventory attack ranges, target attack previews,
venom status potency, gas exposure and Blood Weapon costs use whole-number
values. The nominal Bestiary Damage is displayed to nearest; signed nominal
Damage bonus labels preserve their sign and round halves away from zero. The
underlying bonuses and attack coefficients are not rounded early or rebalanced.

Healing, regeneration, current HP, status duration, resources, stat normalization,
spatial calculations and item-generation arithmetic keep their existing rules.
An ordinary integer damage packet can remove a fractional remaining amount of
health when lethal; damage accounting and healing based on actual HP lost remain
capped at that remaining health. Instant-kill strikes likewise consume the exact
remaining HP to preserve guaranteed death, while their damage text/events show
the rounded number. A victim at 0.4 HP still dies to an instant kill even though
its displayed damage is 0. This is health accounting, not fractional ordinary
packet damage.

Revision 50 and older saves/replays keep their original ceiling and resistance
order, including fractional resistant damage. Their simulation revision is not
upgraded on load. Revision 51 has its own incremental replay/header identity;
startup, generation, actor tuning, storage and replay admission all accept 51.
The original frozen source is unchanged.

## Complexity and ownership

Nearest/scaled/strike rounding is O(1) time and extra memory per packet/preview,
with no cache, RNG or serialization. The packet helper retains the existing
resistance lookup cost: O(A + I + C) worst case for actor-owner lookup among A
current-floor actors and I carried items or C innate conditions; registry lookup
can avoid the A scan. No additional scans or allocations are introduced by
rounding. Existing attack, visibility, AoE, offscreen, copy-on-write and death
costs remain additive and are not claimed O(1). An AoE applies this bounded
arithmetic once per admitted victim. Explicit revision admission remains
constant-count membership. Save/replay validation retains existing world-sized
costs; the new header identity adds only bounded string comparisons. Timings
are unmeasured; profile generated floors/large inventories separately.

## Validation

`crates/corpse-core/tests/revision51_damage_rounding.rs` exercises boundaries,
all packet kinds and exact-scale paths, resistance order, poison/hemophilia,
gas vulnerability, explosions and immunity, bonded packets, attack RNG parity,
unchanged healing/durations, and save/snapshot/replay preservation. Desktop
coverage checks current and historical status projections and journal/header
admission. The integration owner runs the final optimized workspace suite.
