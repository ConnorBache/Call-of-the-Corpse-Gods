# Revision 51 display units

The unit conversion is presentation-only: internal offense/defense keys, serialized
saves, bonus coefficients, authored catalogs and deterministic RNG are unchanged.
The later [fixed-base reduction](revision51-player-base-damage.md) is a separate
revision51 tuning change, and does not scale Damage bonuses.

Player Damage bonuses use nominal unmitigated melee units. One internal offense
point contributes one Damage under the current (31.5 + 2 * offense) * 0.5 formula
(revision50 retains 45 + 2 * offense before the half scale).
Variance, mitigation and revision-aware final rounding still apply. This is not a guarantee of
one extra HP on every hit, and not a ranged bonus: bow coefficients and thrown
attacks differ. Existing weapon previews retain their exact contextual ranges.

An internal defense bonus becomes six HP. Apply each mechanic's integer rounding
before conversion (including separately rounded base and enchantment portions).
Separate maxHp bonuses already use HP units. Never Surrender adds one sixth of
the displayed defense-derived HP bonus to DR; it does not add total maximum HP.

Bestiary retains authoritative maximum HP and drops the redundant defense row.
Damage there is nominal ordinary melee damage, before variance, DR, ability
bonuses and final rounding: (weaponDamage default 10 + offense * coefficient default 1)
/ 2, displayed to the nearest whole number. It is not spell damage or a statement that a passive creature attacks.
Spells and special attacks retain their own descriptions. Player-heart rewards
use player bonus units, not their source monster's attack coefficient.

Authored prose converted separately includes Slaying, Never Surrender, bleeding,
ally attack boosts and Corpse Feast (increased Damage and +30 HP).
Historical fixtures/catalog keys and ordinary English “defense mechanism” remain.

Validation: targeted unit tests cover signed bonus conversion, enemy coefficients,
heart/equipment text and independent Bestiary birth-template arithmetic. Frozen
pack comparison applies explicit numeric unit changes to its old-copy oracle.
No build was run by this worker; integration owns the final cargo workspace run.

Condition-label follow-up: revision 50+ observation presentation converts signed
buffs without changing simulation state. Defense buffs become HP ×6; attack buffs
become Damage ×1 for players/their shared bodies and Mini-Me, or ×0.5 times the
monster's actual weapon coefficient. Historical observation fixtures remain raw.
Meters, hover and inspection therefore receive contextual numeric units.

Revision 51 rounds displayed nominal Damage bonuses to whole numbers after
conversion. Their underlying signed coefficients remain unchanged. See
[damage rounding](revision51-damage-rounding.md) for final packet arithmetic and
legacy compatibility.
