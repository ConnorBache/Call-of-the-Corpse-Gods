# Monster rebalance checkpoint — revision 32

## Result and scope

This is the frozen role-first rebalance checkpoint, not a claim that every scenario
or every mechanic is balanced. It precedes any later Witch/summon additions.
New games use revision32; revisions29/30/31 retain their original mechanics and
population. Source art, RGBA assets and authored Natural History are unchanged.

**Percentages are rough design guidelines.** At earliest reachable appearances,
the held-out128-trial primary measurements show Fighters/hybrids/boss variants
(excluding deliberately negligible Blorb) at **15.6–49.2%** incoming lifetime damage,
Skirmishers at **14.2–31.5%**, and ordinary Mage/Artillery at **13.8–21.9%**. Bombardier
is a separate36.9% burst exception under the unchanged50%-maxHP explosion rule.
These role ranges mix species and reference floors; they are not assertions that
every matched-depth ratio is exact or that groups add linearly.

The all-roster primary artifact contains176 observation rows:86 metadata entries
plus two isolated brood scenarios, each at r31 and r32. Non-killable, unspawned,
event and player-dependent entries remain explicit N/A, not invented measurements.
The primary cases execute20,224 real native trial fights across both revisions.
All current primary finite-combat cases had zero combat-horizon censors except the
unmounted Inventor (126/128), whose solo fleeing case is not a meaningful balanced
Fighter. Mech and group scenarios are reported separately.

| Creature / scenario | Entry floor | r31 gross | r32 gross | Attacks to kill* | Combat deaths |
|---|---:|---:|---:|---:|---:|
| blorb (adjacent) | 1 | 0.1% | 0.1% | 1.01 | 0/128 |
| rat (adjacent) | 1 | 21.9% | 21.9% | 4.30 | 0/128 |
| pale:Spitter (chase) | 4 | 43.2% | 16.7% | 1.95 | 0/128 |
| witch (chase) | 27 | 1.9% | 4.5% | 5.20 | 0/128 |
| pale:Birther (three-cast-children) | 6 | 0.6% | 22.4% | 8.55 | 0/128 |
| pale:Greater Birther (three-cast-children) | 23 | 56.0% | 46.9% | 5.70 | 3/128 |
| minotaur (adjacent) | 33 | 2.0% | 44.3% | 7.79 | 0/128 |
| dragon (adjacent) | 38 | 8.8% | 49.2% | 7.71 | 0/128 |
| flesh-administrator (adjacent) | 40 | 0.5% | 48.5% | 7.70 | 0/128 |

*Actual attack attempts, including misses, conditional on a completed kill; death
rates and incomplete fights are reported rather than silently treated as kills.
Both columns use the **same corrected r32 joint player cohorts at the same new
introduction floor**, isolating numerical/cooldown changes. They do not compare
r31's original placement against r32's new placement.

## Reference player and actual combat

The primary reference consumes each finite, naturally generated prior-floor heart
once, includes real Unblessed seeded+5 Offense/Defense level grants, starts healed,
and uses the starter Shortsword without optional loot or selected abilities.
The exact128 attempted worlds yielded127 paired valid worlds; one existing world
fails generation in both revisions. Its failure and the resulting exclusion are
recorded in the cohort receipt, not concealed. Joint profiles are retained: the
harness does not invent a player by combining independent mean stats.

Accuracy, Dodge, integer stat rounding, weapon bases, damage variance, DR, range,
regeneration, resistances, native special attacks, energy, statuses and movement
all execute in the real engine. The ordinary hit primitive is
`clamp(0.65 × 0.9^(Dodge−Accuracy),0,1) × 0.9^range`; the player's starter strike base
is`30+Offense`, with uniform0.75–1.25 variance, mitigation before half-scale upward
rounding. Enemy attacks use their actual weapon/special paths, not that one formula
indiscriminately. Fast gives extra movement, not doubled adjacent attacks.
Automatic regeneration and intrinsic healing are not stripped.

Primary gross damage ends at the dispatch containing the creature's death, or the
last child's death for a brood. A packet observer records already-mitigated damage
before lethal HP clamping, including attacks deliberately absent from text logs.
Actual capped HP damage, net loss, attached-status aftermath, legal post-kill escape,
and stationary gas/fire stress are separate columns/scenarios. Mortality before a
kill is separately reported; a capped100% net-loss mean never replaces raw damage.

## Preserved identities and explicit limits

- Blorb remains10HP/Offense2 negligible refuse-eating trash, below the general
  Fighter guide. Rat and Hemogoblin retain their modest opening values.
- Witch moves to floors27–36 and retains its actual slow/hemophilia/poison cycle.
  No summon behavior is implied by this r32 checkpoint. Controllers/supports are
  not forced into direct-damage bands by inflating their melee attacks.
- Existing escalation remains explicit: venom tiers1/2/4, Greater Screamer's doubled
  bonuses/penalties, and later richer curse/control kits. No new universal
  depth-scaling debuff rule was invented.
- Breather gas volumes remain authored. Their primary costs are about40.9%/32.3%
  for normal/Greater, not the nominal10%/14% utility suggestions. Lingering gas can
  kill a stationary player after the monster dies; some bounded arenas have no
  legal clear escape cell. Such aftermath is reported as danger/trapping, not
  declared balanced or subtracted from the evidence.
- Spitter first-life damage was a real outlier even at zero physical Offense.
  The normal form retains its fragile34HP body; existing gas-shot injection is
  reduced100→25 and the Greater form200→50. Global gas damage, diffusion, terrain
  and resistance behavior are unchanged.
- Three casts spend the real50-energy pool: twins30, then one10, then one10.
  The ten-world-turn cooldown is shared by both cast forms and spent only when
  children are actually created. Haste does not shorten it; missing save state is
  immediately eligible. Energy regeneration is unchanged: a continually engaged
  parent is finite, while losing contact and recovery sleeping can refill it.
- Heartless parents create heartless children, including a revived parent and the
  finite Greater Birther→normal Birther→Jackal descendants. Heart-bearing parents'
  ordinary minion rewards remain unchanged.
- Six fixed parent-specific birth profiles preserve species/abilities/rewards and
  serialize the actual resulting stats. Natural Jackals do not receive these
  brood stats. Four immediate children are produced by the three-cast benchmark;
  descendant births are included. The measured46.9% Greater payload is deliberately
  not forced to exactly38%. Its3/128 combat deaths and all aftermath remain visible.
- First/mid/last-floor sensitivities intentionally show old species becoming weaker
  as the player grows. Fixed species are not secretly rescaled by depth.
- Standardized adjacent/chase rooms and matched support tests do not cover all
  terrain, pursuit, surprise, groups, optional equipment or emergent full-floor play.

## Reproduce and inspect

See[the native audit contract](monster-balance-native-audit.md). Final input and
binary/source hashes are in`tools/monster-balance/final/`. Full primary, layer,
support and hazard tables are JSON and CSV in that directory; the original
candidate fitting files are not final evidence. Shipping tuning is centralized in
`crates/corpse-core/src/catalog/physical-balance-v32.json` and
`summoned-balance-v32.json`.

```sh
sh tools/cargo.sh test --workspace --locked
sh tools/cargo.sh build -p corpse-core --example physical_trade_arena --locked
python tools/monster-balance/compare_native_roster.py \
  --binary target/debug/examples/physical_trade_arena \
  --profiles tools/monster-balance/relocated-player-progressions-r32-joint-profiles.json \
  --cases tools/monster-balance/approved-native-introduction-cases.json \
  --trials 128 --output /tmp/reproduced-primary.json
```

Focused constructor/reward/serialization, cooldown/haste/temporal/heartless and
native-harness tests are included. The final aggregate verification record is the
authority for the workspace suite; no Windows build or runtime verification is
claimed for this balance-only checkpoint.
