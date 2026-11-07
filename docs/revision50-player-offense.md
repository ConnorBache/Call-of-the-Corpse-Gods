# Revision 50 player offense and enemy baseline

Revision 51 now reduces the fixed base alone by 30%, while preserving this
revision 50 arithmetic for old saves/replays. See [current formulas](revision51-player-base-damage.md).

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

## Authored rule

The designer requested the player's offensive baseline to match a Greatsword
instead of a Shortsword, with enemy adjustments only if necessary. The existing
`equipment-v48.json` defines Greatsword damage 45 and Offense multiplier 2.
Revision 50's unequipped melee base is therefore **45 + 2 × effective Offense**,
not merely 45 + Offense and not a doubled character Offense stat. Accuracy, damage
variance, DR, the attack-channel half scale and upward packet rounding remain.
The old baseline was 30 + effective Offense. No weapon item or equipment slot is
created, and carried weapon enchantments and offhands do not enter this baseline.

`revision50_player_attack::with_offense` owns the constant arithmetic.
`revision50_player_attack::base` reads the current live player effective Offense,
including existing rounded stat contributions. It performs no attack rolls,
ability activations, damage multipliers or other attack effects. Clay's basic
attack copy can use this current value without copying the player's attack
blessings. Mini-Me copies the same 45/2 baseline and has no offhand strike.

Explicit bow and throw bases remain item-defined. In particular, both the Bow
commit path and item previews use `engine::player_weapon_strike_base*`; these
generic item helpers must not be changed to an unconditional Greatsword base.
The bow preview's `weapon_base(Some(bow), offense)` is also intentionally intact.
Melee inspection uses the shared R50 baseline and no longer forecasts equipped
main/offhand attacks or removed Fury/Assault effects. A defender with an active
Missed Me reaction gets a conditional retreat explanation instead of the old
50%-chance penalty or an invented hit forecast; trapped outcomes remain the
simulation's responsibility. Older revisions retain their old arithmetic.

## Enemy preservation, not new tuning

Several native birth and population guards previously stopped at revision 46.
An explicitly constructed R50 actor could therefore reject admission or skip
the already calibrated physical/Offense/Dodge overlays. That is not a deliberate
rebalance. Exact revision 50 now inherits the unchanged revision-46 baseline:

- Physical r31/r32 bodies, r33 Accuracy/Dodge and r42 final birth profiles.
- Natural constructors, ordinary horde selection and authored setup eligibility.
- Witch/Queen parent-specific final child profiles and Birther context overlays.
- Early Zillennia Ruin raw packets 14/18, rather than reverting to 28/36.
- Internal floor-generation entry points for explicitly constructed R50 runs.

No numerical catalog values, spawn weights, rewards, enemy abilities or saved
actors were retuned. Revisions 47–49 remain unimplemented. Startup, storage,
replay admission and debug-floor release gates remain closed to R50; these edits
are not a new-run or release enablement.

## Completed analytical sensitivity, limited scope

Run `python3 tools/monster-balance/revision50_greatsword_sensitivity.py`.
JSON and CSV results are in `docs/artifacts/revision50-greatsword-body-sensitivity.*`.
The script completed using all 127 stored r32 `all_initial_unblessed` joint entry
profiles and unchanged revision-46 enemy stats. It solves the exact discrete
damage distribution and expected attacks against a stationary, non-healing body.
It is deterministic arithmetic, **not execution of the native encounter engine**.

| Target | Reference floor | Old expected attempts | New expected attempts | Reduction |
|---|---:|---:|---:|---:|
| Blorb | 1 | 1.000 | 1.000 | 0.0% |
| Giant Rat | 1 | 2.243 | 1.533 | 31.7% |
| Cave Bat | 1 | 2.308 | 1.538 | 33.3% |
| Shellback Beetle | 1 | 5.886 | 3.338 | 43.3% |
| Hemogoblin | 1 | 2.777 | 2.000 | 28.0% |
| Bear | 6 | 5.017 | 3.127 | 37.7% |
| Minotaur | 33 | 7.714 | 3.803 | 50.7% |
| Dragon | 38 | 7.599 | 3.682 | 51.5% |

At zero Offense the unmitigated landed packet averages 15.5 → 23.0 after existing
half-scale upward rounding. Against Shellback's DR 14 it averages 8.5 → 16.0.
Base Offense 0/25/100 gives raw strike bases 30/55/130 → 45/95/245. This is not a
uniform multiplier: armor thresholds and already-one-hit targets behave
differently, and the benefit grows as Offense grows.

The diagnostic omits incoming enemy damage, death risk, regeneration (including
Troll's healing), Dragon breath, movement, terrain, status effects, summons,
optional jewelry and R50 blessings. Its old r32 progression cohort is not a
regenerated R50 campaign. It cannot estimate native lifetime damage or certify
whole-game difficulty. Earlier 20%-Rat and other r31/r32 reports also predate the
existing r42 forgiving-start tuning and must not be presented as current means.

The analytical result motivated the paired native check below. It is retained as
an independent damage-arithmetic sensitivity, not substituted for native fights.

## Completed bounded native comparison

`revision50_offense_arena.rs` adapts the existing `physical_trade_arena.rs` scenario,
chase, escape and authoritative damage-observer machinery. It constructs a
supported R46 analysis shell, explicitly selects R46 or R50 and admits normal
commands through the real dispatcher. It does **not** enable R50 startup or saved
runs. R46 keeps only the starter Shortsword; R50 has no inventory. Both have no
player blessings or optional gear. Enemy candidate overrides are explicitly
rejected. Native enemy stats, abilities, spells, movement, healing, damage,
statuses, death and environmental rules remain active.

The old harness's attack-text counter recognized only “You strike”, missing the
current Shortsword “You slash” and “You try to … but miss” forms. This separate
diagnostic recognizes both current baselines and tests hit/miss counting.

**Final bounded sample:** 32 distinct joint player profiles selected evenly from
all 127 stored r32 `all_initial_unblessed` worlds, 14 scenarios, both revisions:
**896 native fights and 4,820 actual commands**. Every paired trial verifies the
same initial player profile, combat seed and complete initial enemy records.
Different kill timing may legitimately consume different later random draws.
The final recorded execution took 10.79 seconds on this shared cloud host, not
a platform speed guarantee. All 896 primary fights ended in kills; the sample
had zero deaths, command errors,
combat censors or aftermath censors. This is not a guarantee against rare deaths.
No permanent fields were silently removed; legal escape can finish local exposure
while leaving remote gas/fire in the room.

The primary metric is the mean of each trial's native incoming gross damage,
divided by that trial's initial HP, through the end of the killing command.
Packets are captured after mitigation and before HP clamping. Net HP loss,
post-death exposure, attempted attacks, kill-conditional means and survival are
separate raw output fields. Percentages below are **total lifetime incoming
fractions, not damage per hit**.

| Scenario | Reference floor | R46 gross | R50 gross | R46 attacks | R50 attacks |
|---|---:|---:|---:|---:|---:|
| Blorb, adjacent | 1 | 0.0% | 0.0% | 1.00 | 1.00 |
| Giant Rat, adjacent | 1 | 9.2% | 3.1% | 2.31 | 1.50 |
| Cave Bat, adjacent | 1 | 5.9% | 2.0% | 2.56 | 1.59 |
| Shellback Beetle, adjacent | 1 | 10.2% | 5.8% | 6.19 | 3.56 |
| Hemogoblin, adjacent | 1 | 9.3% | 5.3% | 2.78 | 2.00 |
| Needleling, adjacent | 1 | 10.3% | 4.8% | 2.00 | 1.47 |
| Bear, adjacent | 6 | 15.0% | 8.8% | 5.53 | 3.38 |
| Troll, adjacent | 19 | 21.8% | 9.3% | 7.22 | 3.63 |
| Dire Bear, adjacent | 20 | 27.8% | 12.1% | 7.69 | 3.97 |
| Minotaur, adjacent | 33 | 23.0% | 10.3% | 7.84 | 3.81 |
| Dragon, adjacent | 38 | 24.4% | 7.6% | 7.69 | 3.78 |
| Pale Spitter, fixed chase | 4 | 8.8% | 7.4% | 2.00 | 1.25 |
| Pale Birther, isolated three-cast children | 6 | 14.3% | 7.7% | 9.00 | 5.97 |
| Linked Ursa pair, Minor then Major | 36 | 40.7% | 16.7% | 14.34 | 7.06 |

Native markers confirm more than basic damage arithmetic: Needleling applied
venom 23→11 times, Dragon used fire breath 65→32 times, and each Spitter spent
10 energy on its real attack path and left an escapable residual gas field.
All 64 Ursa-pair trials executed the death cry. All isolated Birther trials began
with four children from exactly three successful native setup casts; intervening
world turns were not simulated and the parent was then removed. This measures
that finite payload, not the summoner's full life. Troll retains its actual
regeneration trait and runs normal native upkeep; healing is not stripped.

Already-applied harm is not uniformly reduced. Mean post-death gross damage is
Needleling 2.88%→1.38% and Dragon 0.79%→0.89%. The latter small increase coexists
with much lower combat exposure. The Spitter's gas component persists despite
a faster melee kill. These differences are evidence against one universal
compensating enemy multiplier.

## Tuning decision and remaining evidence

**Keep production enemy numbers unchanged.** The requested Greatsword baseline
is a substantial improvement and the bounded sample quantifies it. It does not
establish a new target difficulty or justify automatically cancelling the upgrade
with enemy buffs. Earlier r31/r32 percentage targets also predate the current
forgiving-start profiles. This diagnostic is not a claim that campaign difficulty
is correct or that selective future adjustments will never be needed.

The no-item, no-player-blessing subset is interpretable now because it does not
invent any reserve capacities, reserve acquisition rules, jewelry tiers, amulet
effects, bow/ammunition category rules or optional starting loadouts. It directly
measures basic offense against unchanged native defenses and AI. Caster/status
rows measure their actual isolated effects; paired and summon rows are separately
labeled controlled scenarios, not an emergent floor or campaign.

Before defensible selective tuning:

1. Finalize the R50 initial item/loadout and ability-reserve policies, and identify
   representative intended god/blessing combinations and optional jewelry/amulet
   loadouts. Do not infer capacities or consumable availability for the benchmark.
2. Regenerate actual joint R50 progression profiles from matched world seeds,
   including realistic heart-collection sensitivities. The stored r32 cohorts
   here are deliberately held fixed, not presented as future R50 progression.
3. Run matched seeded encounters and multi-room/floor routes with those loadouts,
   finite energy/ammunition/consumables, expected movement and recovery behavior.
   Include armor, regeneration, ranged hazards, controlled groups, summons and
   bosses. Track lifetime gross and net damage, energy/resource use, attempts,
   survival, censoring and lingering status/terrain costs separately.
4. Compare reproducible outliers against the intended current challenge and role,
   then adjust only supported fixed species/context values. Preserve authored
   behavior, placement and rewards rather than fitting every row to one number.

## Verification, artifacts and reproduction

Cargo 1.99.1 is now installed. The earlier absent-Cargo result is superseded for
this diagnostic. Its **three example tests passed**: unchanged paired NPC/profile
fixtures and closed startup gate, current hit/miss text counting, deterministic
native trials and rejection of numerical overrides. The dedicated example build
passed with locked dependencies and two compiler jobs. A separate four-profile
Rat/Shellback smoke completed 16 fights before the full 896-fight sample.
This is not a full-workspace, release or platform-rendering certification.

Exact commands, from the repository root:

```sh
sh tools/cargo.sh test -p corpse-core --example revision50_offense_arena --locked -j2
sh tools/cargo.sh build -p corpse-core --example revision50_offense_arena --locked -j2
python3 tools/monster-balance/revision50_native_offense.py --trials 4 --kinds rat shellback-beetle --output docs/artifacts/revision50-native-offense-smoke
python3 tools/monster-balance/revision50_native_offense.py --trials 32 --output docs/artifacts/revision50-native-offense
```

Final output paths:

- `docs/artifacts/revision50-native-offense.json`: paired summary, input/source and
  executable hashes, selected profile indices, execution time and explicit scope.
- `docs/artifacts/revision50-native-offense.csv`: compact numerical comparison.
- `docs/artifacts/revision50-native-offense.raw.jsonl`: complete native summaries
  and all 896 trial outcomes, initial actors, seeds, profiles, markers and censors.
- `docs/artifacts/revision50-native-offense.requests.json`: exact executable inputs.
- `docs/artifacts/revision50-native-offense.stderr.log`: native stderr, empty on success.
- Matching `revision50-native-offense-smoke.*` files retain the separate smoke.
- `verification/revision50/offense-arena-test.log`, `offense-arena-build.log` and
  `offense-arena-run.log`: exact final command output.

The initial deterministic analytical script and its three recurrence checks also
completed. Separate implementation tests cover main/generic strike equality,
explicit ranged preservation, live effective stats, Mini-Me/preview behavior,
unchanged birth profiles, population/generation admission and legacy gating;
those checks belong to the broader integration suite rather than being implied
by this arena's three passed tests.

## Complexity

Let A be current-floor actors, K actor conditions/abilities, I equipped inventory
entries and E their enhancements. The pure baseline helper is O(1) time/space.
The live `base` getter inherits `combat::effective_stats`: actor/owner lookup,
O(K + I(1 + E)) contribution scans and existing health/ability helper costs, with
O(A) worst-case actor lookup where applicable. It must not be described as an
O(1) live-stat query. It adds no RNG, persistent fields, snapshots or per-turn
work outside callers' existing attack/projection paths.

Mini-Me still clones its actor and detached inventory; replacing two scalar
weapon-condition values and removing two offhand keys adds bounded map edits
(O(K) worst-case shift/removal). Inspection adds a constant revision branch and
uses existing effective-stat work. Exact50 admission adds bounded comparisons
to unchanged construction/generation algorithms and does not change their
asymptotic cost. Birth overlay scans/parsing retain the existing r42 ledger cost.

For the offline diagnostic, let M distinct (HP, base, DR) combinations occur,
H maximum target HP and D the count of possible nonzero damage packets. Dynamic
programming costs O(MHD) time, O(H + D) transient memory per unique combination,
and O(M) cached scalar results. Profile decoding retains O(P × F × S) memory for
P=127 worlds, F=40 floors and S=6 joint stats. Reporting costs O(TP) for T=12
targets after recurrence results are cached. This script is not shipping runtime
work and does not measure native performance.

For the native diagnostic, let T<=512 trials, C<=256 combat commands and
S<=128 settlement commands per request, V=81 cells, A actor records, K condition
fields and E log/events per dispatch. Added instrumentation is O(A²+K+V+E) per
command plus existing native validation, COW snapshots, pathfinding, visibility,
AI, effects and environment costs. Chase/escape add native traversal/helper costs
over the room graph. Summons are not capped or omitted by instrumentation. The
selected initial groups have at most four actors; storing full output costs
O(T) rows with their initial actor/condition and terminal summary payloads, plus
optional O(T(C+S)(A+K+V+E)) traces. This run disables traces and preserves all
per-trial outcomes. The Python driver retains P=127 stored worlds across F=40 floors, R request rows
with T selected profiles each, and N received trial outcomes: O(PF×6 + RT×6 +
N×output-row-size) memory, plus source digest metadata. It reads/hash-scans the
recorded source files once.
Measured elapsed time is in the receipt, not an asymptotic/native performance
guarantee. No analysis code runs in ordinary play.
