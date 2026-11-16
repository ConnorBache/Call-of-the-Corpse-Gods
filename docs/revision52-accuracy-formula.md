# R52 ratio accuracy and approved starting statistics

> Historical Accuracy Baselines checkpoint. The later [stat follow-up](revision52-stat-followups.md) supersedes player starting Dodge (now 15), floor-varying creature Dodge (now first-appearance values), innate player damage, Jackal hearts, and heart capacity. The later [minimum starting Dodge amendment](revision52-birth-dodge-minimum.md) also floors fresh real creature bodies at ten. The shared ratio formula remains current.

## Designer instructions

Ordinary hit chance is based on `Accuracy / (2 * Dodge)`. The designer clarified:
“Within this calculation, dodge is always min-capped at 1, starting player
accuracy is 15, starting creature dodge is 10”. On 2026-11-08 the designer also
confirmed player starting Dodge 10 and provisionally approved creature starting
Accuracy 15 with existing species modifiers on top.

These combined baselines resolve the earlier negative-enemy-Accuracy blocker.
No individual species/floor offset, damage stat, reward or encounter target was
retuned. The provisional creature Accuracy baseline is explicit, not a hidden
combat bonus.

## Shared calculation

Actual attacks and native strike/Bow previews call
`combat::hit_chance_for_run`. R52 uses:

`clamp(Accuracy / (2 * max(Dodge, 1)), 0, 1) * 0.9^range`

The result is clamped again to a probability. Division precedes multiplication
by one half so a large finite Dodge cannot overflow the denominator. Dodge's
minimum of one applies only inside this calculation, not to stored or displayed
stats. Accuracy 15 versus Dodge 10 gives 75% before range. Accuracy 1 versus
Dodge 0, negative Dodge or Dodge 0.5 gives 50%. Nonpositive effective Accuracy
gives zero. Invalid nonfinite inputs or negative physical range return zero
without changing saved-state admission rules. Valid range loss is unchanged.

Effective equipment, temporary buffs, fractional Sustain relief and Shine's
actual target-light Accuracy contribution retain their existing order. Surprise,
Blood Weapon, The Number of Blood, luck, distraction, paralysis, protection,
Legacy Missed Me and Bow miss-reduction remain outside the base formula. The
separately authorized R52 [Hanged replacement](revision52-hanged.md) removes
Missed Me retreat at that revision; it does not change hit probability.
Guaranteed strikes and Bow's miss-reduction can therefore still hit when
ordinary base chance is zero. Damage, DR, weapon multipliers and range are not
retuned.

## Actual birth statistics

New R52 normal and DEBUG players start with actual Accuracy 15 and Dodge 10.
Earned progression remains additive. The initial-player snapshot used by Mini-Me
contains these actual starting values; no birth bonus is reapplied when copied.

New non-player R52 bodies use actual Accuracy 15 plus their existing authored
species/floor Accuracy offset and Dodge 10 plus the existing Dodge offset.
Historical and R52 catalog entries remain unchanged and are treated as offsets
at the R52 birth boundary. Generic and contextual assignments replace the whole
raw birth stat plus baseline rather than incrementing a live actor repeatedly.
Special constructors and Bestiary reconstruction use the same helpers.

Ordinary actors, catalog fungi, contextual newborns, new corpses, Sphinx,
Zillennia, the Administrator, Ichor fish, Ahvamitt and Phantasmii Nightmares are
covered. Copies, Mini-Me, Half Here, Clay Soldier, resurrection, growth, loaded
actors and temporal restore do not receive another baseline. Waking Nightmares'
special statless hallucinations retain their established illusion behavior;
they are not new creature species and gain no unrequested evasion buff.

## Calculated impact and compatibility

The 550 current birth profiles contain Accuracy offsets -3 to 185. After the
approved baseline, their actual Accuracy is 12 to 200 and none is nonpositive.
Dodge offsets are -14 to 108, becoming actual Dodge -4 to 118. Thirty-eight rows
remain below the probability denominator floor of one; stored stats are not
silently changed by the formula.

Fresh-player reciprocal ordinary melee probabilities for floor-1 examples:

| Creature | Actual Accuracy / Dodge | Player hits creature | Creature hits player |
| --- | --- | --- | --- |
| Rat | 14 / 7 | 100% | 70% |
| Cave Bat | 13 / 10 | 75% | 65% |
| Blorb | 12 / -1 | 100% | 60% |
| Shellback Beetle | 13 / -4 | 100% | 65% |

These are base probabilities before range or explicit overrides, not simulated
encounter-loss guarantees. All eight floor-1 profile types have ordinary return
chances of 60–70% against a fresh Dodge-10 player. Accuracy 20 reaches 100%
against that starting player; Accuracy 2 gives 10%, not a guaranteed hit.

Pre-R52 runs retain the frozen exponential formula and their old births,
including legacy analysis tools and differential fixtures. Existing R52 saves
use the new formula but retain their saved stats; there is no automatic stat
migration. Older R52 saves and recorded replays can therefore behave differently.
No saved field, RNG primitive, normal/haste schedule or temporal ownership rule
was added. Normal, haste and offscreen attacks share the engine calculation.

Previously recorded R52 Fighter/all-role encounter fits used the exponential
formula. They do not certify this amendment; no new encounter fit or campaign-
safety conclusion is claimed.

## Verification scope

The preceding accuracy checkpoint passed 2,590 tests with zero failures and 42
ignored across 206 workspace targets, plus Windows x64 release cross-compilation
(`verification/r52-accuracy-final-03/`). Those receipts predate the final player
Dodge and creature Accuracy approvals and are historical for this source.
Fresh successful matching source/test/build receipts under the later accuracy
verification gate are required before packaging. Windows cross-compilation is
not Windows runtime, physical GPU or monitor/DPI verification.

Coverage includes numeric boundaries, actual melee/projectile rolls, native
previews, birth constructors and contextual overrides, whole-world generation
differences, legacy preservation, normal/DEBUG provenance, save/replay and
Snapshot/Future behavior. No user computer is accessed.

## Complexity

For fixed numeric inputs and a revision, the probability helper uses O(1) time
and extra memory, including the existing range exponentiation. Player starting-
stat assignment and creature birth Accuracy/Dodge conversion add O(1) work per
existing raw-stat assignment, with no new traversal, allocation, cache, RNG call
or saved field. Existing catalog scans, constructor work, effective-stat
traversal, illumination, attack lifecycle and native rendering keep their
existing bounds. Contextual replacement does not accumulate baselines. These
are analytical bounds, not measured performance or portability guarantees.


## 2026-11-09: Enemy melee label and Dodge-28 diagnosis

The creature drawer and map-hover heading now says **Enemy melee**. This is a
label-only correction: the projection already describes the enemy's ordinary
range-zero strike against the player's current effective Dodge. The reciprocal
**Your attacks** line uses player Accuracy and enemy Dodge. Neither number is a
universal probability for all abilities. Existing blocked/protected, Hanged,
offhand and other special-case preview sentences remain unchanged.

No numerical defect or stale shared 48% value was reproduced. An enemy with
Accuracy 27 against Dodge 28 has a current R52 melee chance of 27/56 = 48.214%,
rounded to 48% for display. The same actor gives 90% at Dodge 15 and 24% at Dodge
56. A saved pre-R52 revision retains the old formula in both display and actual
attacks: the same raw stats give 100%, 59% and 3% respectively. Running a new
executable does not silently convert a save's simulation revision or its stats.

Independent native diagnostic harnesses ran against released Avatar World
source and the pending source. Each checked 6,144 real melee and 1,536 real
ranged outcomes against identical deterministic RNG thresholds, with zero
mismatches. In each R52 sample of 1,024 seeds, Accuracy 27 landed 919, 485 and 232
melee hits at Dodge 15, 28 and 56. Those counts are controlled samples, not new
balance targets. JSON roundtrip and native fixture loading retained revision
behavior. A base Dodge 15 plus an actual +13 item bonus produced effective Dodge
28 and the same 48% incoming preview.

One observed run contained a Giant Rat, Bear, Hemogoblin Bloodmancer and Unicorn
with distinct Accuracy 14, 21, 27 and 56. Ten repeated native map/sidebar hover,
selection, detail close/reopen and fresh-painter comparisons retained the right
actor, including staircase row offsets, and displayed 25%, 38%, 48% and 100% at
Dodge 28. Reusing the same actor and terrain observation while changing player
Dodge 15 → 28 → 56 → 28 refreshed the preview to 90% → 48% → 24% → 48%.
Inspection did not mutate Run state or RNG. The later minimum creature
birth Dodge changed their defensive stats only, not those incoming percentages.
This exercises production presentation helpers and rasterization, not a live
SDL event-loop or Windows interaction session.

Bloodmancer/Witch status spells, poison ticks, Banshee shrieks and explosion
packets have separate resolution paths without ordinary Accuracy/Dodge rolls.
Actual Bloodmancer Hemophilia casts succeeded without consuming an RNG roll at
Dodge 15, 28 and 56, and actual poison ticks dealt 2 damage at all three values.
Hornlight and Moonbeam instead use ordinary ranged rolls with distance loss.
The label correction does not change any special action, damage, retrigger,
turn scheduling, birth stat, saved stat or RNG behavior. The user's specific
incident remains unverified without its creature/floor, visible conditions and
save revision; the controlled cases are not a reconstruction of that run.

Focused source regression:
`inspection_ui::hover_tests::revision52_enemy_melee_heading_is_shared_without_changing_special_previews`.
Existing accuracy, range, override and retreat preview tests remain applicable.
Changing two uses to one constant does not change the existing text wrapping,
effective-stat or rendering complexity bounds.
