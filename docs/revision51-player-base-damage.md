# Revision 51 player fixed attack damage

Designer direction: “Reduce the players base damage by 30%.” Only the fixed
component changes. One internal Offense still contributes one nominal melee
Damage after the existing attack half scale. Enchantment and all existing attack
coefficients remain intact. Revision 50 and older saves/replays retain their
original formulas and rounding. No existing actor/item catalog is rewritten.

## Exact components and rounding

All formulas below are raw strike bases, before variance, mitigation and the
attack-channel half scale. `O` is already-derived effective Offense, `D` is the
item's fixed damage, `E` its enchantment, and `C` its Offense coefficient.

- Ordinary player melee: `45 + 2O` becomes `31.5 + 2O`.
- Bow: `D + E + CO` becomes `0.7D + E + CO`. A Bow's fixed 15 becomes 10.5.
  A Composite Bow's fixed 30 becomes 21. Their coefficient remains 0.5.
- One-handed weapon throw: `D + E + CO` becomes `0.7D + E + CO`.
  Non-throwing one-handed weapons still halve the entire resulting attack base.
  A fixed 30 weapon becomes 21 before that existing throw penalty.
- Other thrown items/two-handed weapons use `O`, with no fixed base to reduce.
- Explicit ability/spell attack bases, direct damage, enemy attacks, monster
  profiles and environmental/status packets are not scaled by this change.

Reduction happens before variance, ability attack multipliers, DR, the existing
0.5 attack scale and revision 51's final nearest-whole rounding. There is no early
rounding of 31.5 to 32. Percentage arithmetic preserves authored half boundaries:
45 ×70 /100 is exactly 31.5, avoiding the slightly-low binary result of 45 ×0.7.
No RNG is added, removed or reordered.

At mean variance 1 and zero DR, using revision 51 nearest rounding on both sides
to isolate the tuning change:

- Offense 0 melee: raw 45 →31.5; nominal 22.5 →15.75; final 23 →16 HP.
- Offense 20 melee: raw 85 →71.5; nominal 42.5 →35.75; final 43 →36 HP.
- Offense 20 Bow, +3 enchantment: raw 28 →23.5; nominal 14 →11.75;
  final 14 →12 HP. At variance 0.75–1.25 its current preview is 9–15 HP.
- Offense 20 one-handed throwing weapon with damage 30, coefficient 1.5 and
  +3 enchantment: raw 63 →54; nominal 31.5 →27; final 32 →27 HP.
  A non-throwing one-handed version retains its penalty: raw 31.5 →27,
  nominal 15.75 →13.5, final 16 →14 HP.

These are contextual examples, not a 30% reduction of total damage. Added Damage
and enchantments dilute the percentage change; DR and final rounding also affect
actual HP lost. No late global 0.7 multiplier is applied.

## Ownership, copies and compatibility

Shared Half-Here/Dreamwalking bodies use their actual owner and current stats.
New Mini-Me actors snapshot 31.5 fixed damage and coefficient 2 alongside their
existing frozen effective numeric stats. Already-created Mini-Me actors retain
whatever `weaponDamage` was serialized, including 45 from an earlier revision 51
build. Their inspection preview reads that same stored baseline; load, rewind
and Future state do not rewrite arbitrary actor weapon conditions.

Clay Soldiers explicitly copy the current player's live basic melee damage.
Each temporary attacker therefore uses the revised base and current effective
stats, without copying abilities, items or attack procs. Ordinary enemies and
independent allies retain their own authored attack components.

Current main/generic player attacks, counterattacks, bow/throw commits, inventory
ranges and target inspection use the same revision-aware helpers. Legacy pure
helpers remain available for callers without a revision and old regression
oracles. No schema, startup gate, item definition or save revision is changed.

## Complexity and validation

Added arithmetic is O(1) time and O(1) extra memory per strike, preview or copied
attacker, with no allocation, cache or RNG. Run-aware wrappers retain the existing
`effective_stats` traversal: actor-owner lookup among A current-floor actors
(registry acceleration when available), I inventory entries and C condition
contributions. Those costs remain O(A + I + C) worst case, plus existing item
metadata work. Item and already-derived-stat helpers introduce no scans. Mini-Me
snapshot/copy, Clay actor insertion/removal, inspection visibility and save/rewind
retain their existing transitive costs; this tuning does not make them O(1).
Execution frequency remains one helper call per attack/preview/construction.
Wall-time and renderer timing are not measured by this change.

Deterministic regressions in `revision51_player_base_damage.rs` cover actual
main/generic melee in normal/haste states, bow commits, dedicated/improvised and
nonweapon throws, signed bonus coefficients, enchantment, post-mitigation
rounding, enemy and explicit ability preservation, live versus frozen copies,
a synthetic retained shared-avatar secondary-attack path and save/snapshot
retention. The retired Never Without a Fight blessing is not offered/acquirable
in R50+; its synthetic reserve exists only in this low-level owner-routing test.
Desktop tests cover
inventory ranges and current/stored-copy target previews. The existing revision 51
rounding regression now compares each revision against its own melee base.
The eight base-damage regressions passed against the integration owner's pinned
pre-vision core artifact using direct `rustc --test` linkage. The initial fixture
failures were repaired by retaining every floor's actor block, modeling existing
blood-splatter RNG, and initializing the synthetic compatibility reserve directly.
See `verification/r51-ui/direct-core-tests/base-damage-repair.log`. No production
changes were required by that repair. Final Cargo integration after later workers'
changes and Windows runtime verification remain separate checks.
