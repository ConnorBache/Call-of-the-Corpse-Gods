# Player heart progression: full native generation audit

## Meaning of the reference build

The primary numerical reference is **Unblessed**, the guaranteed equipped Shortsword,
no optional equipment or consumables, and every initially generated living,
heart-bearing creature collected once. This is a synthetic collection census, **not a
claim that this player can win every fight or survive collecting every heart**.
Each layer has an entry snapshot (all preceding layers consumed) and an exit snapshot
(that layer consumed as well). Maximum HP is reported, not an invented surviving HP.
Maximum Energy is the capacity stat; current energy is not a damage-budget estimate.

`all_initial_unblessed` includes ordinary hordes, actual machine population and initial
Zillennia. `ordinary_unblessed` removes Zillennia. The corresponding `all_initial` and
`ordinary` tracks retain the same actual heart effects but deliberately suppress
selected level rewards. These are conservative **heart-only** references for
Blessed/Avatar, not simulations of their starter abilities or eventual chosen builds.

There is no optional loot optimization, equipment reroll, farming, survival selection,
heart destruction or navigation simulation. Every seed uses actual 40-layer native
generation; this replaces the older dry-horde-only reference that omitted machines.

## Exact mechanics used

- `startup::new_run_shell_revision` and `startup::ensure_floors` provide the starter
  inventory and all actual generated actors. Starting HP 100, maximum Energy 100,
  Perception 300; Offense/Defense/Accuracy/Dodge/DR are zero.
- `inventory::heart_for` constructs the real species/quality reward, and
  `inventory::command(Command::Use)` applies it. Stat contributions use the native
  integer rounding and health synchronization. Heart bonuses do not depend on player
  level, and eating does not replace a reward with an assumed average.
- `progression::level_up` spends `2^current_level` blessings. Starting at level 1,
  total blessings needed to reach level L are `2^L - 2`.
- Unblessed's real `choose_offer` grants +5 Offense **or** +5 Defense, 50/50.
  After k level grants, each stat's expected bonus is 2.5k and conditional standard
  deviation is 2.5√k; their allocation is negatively correlated. The audit keeps the
  actual seeded outcomes, rather than adding mean bonuses to every build.
- Native maximum health is `100 + 6×Defense + explicit maxHp bonuses`; the reference
  has no extra independent maximum-HP source. Defense is capacity, **not DR**.
- Shortsword strike base is `30 + Offense`. Ordinary physical damage is
  `max(0, ceil((strike_base×U[0.75,1.25) - target_DR)×0.5))`.
  Surprise/paralysis and special traits are separate modifiers; no surprise is
  assumed for the ordinary adjacent full-life benchmark.
- Hit chance is `clamp(0.65×0.9^(Dodge−Accuracy),0,1)×0.9^range`, clamped again.
  Accuracy/Dodge are exponential chances, not additive percentage points.
- An unmodified player attacks once per normal world tick. Haste/Fast grants extra
  **movement only**, not a second adjacent attack. Slowness adds a normal world tick.
  Living actors normally regenerate 0.2% of maximum HP per tick. The combat harness
  executes these paths and reports gross incoming damage separately from net HP loss.

## Finite heart scope and conditional alternatives

Machine corpses are generated dead and heartless and therefore contribute no heart.
The first actual death drops a heart and marks the actor heartless; revival does not
drop another. Initial Sphinx/finale heartless actors also contribute none.

Birther descendants are action-dependent and are **not** silently added to the initial
census. Their starting 50 Energy is finite. With room for twins, 30 Energy creates two
children, followed by two 10-Energy single casts: three casts, four children. If only
one adjacent cell is free each cast, five 10-Energy singles can create five children.
The revision 32 ten-world-tick cooldown is time, not a new source of energy.

Basic Birthers produce Jackals (zero stat/blessing reward). Greater Birthers choose
uniformly among five **basic** roles: Biter, Breather, Spitter, Birther, Screamer.
Thus no arbitrary recursion is assumed: a basic Birther child can only add Jackals.
For the four-child scenario, a Greater parent yields four direct children plus an
expected 3.2 Jackal grandchildren: 7.2 descendant hearts (maximum 20), with expected
+1.6 Offense/Defense/Accuracy/maximum Energy, +0.8 DR, and 4 blessings. Revision 32
heartless parents create heartless children, so those branches add no heart reward.
The conditional sensitivity exhausts each starting energy pool once, with enough
time and survival; it does not include a sleep/refill cycle.

Mercuria creates **at most ten fish per activated well**. These are not present in the
initial actor list. A separate sensitivity assumes every fish emerges and is eaten,
using the actual native eligible-species constructors and heart templates. It uses
discrete reward-distribution convolution to account for level thresholds, not the
incorrect shortcut of applying the level formula to an average blessing value.
The output includes fish-only, finite-summon-only, and their combined conditional
expectations; none should be mislabeled as an executed survival simulation.

## Reproduction and validation

From the project root, use the pinned toolchain:

```sh
sh tools/cargo.sh test -p corpse-core --example heart_progression_audit
sh tools/cargo.sh build -p corpse-core --example heart_progression_audit
target/debug/examples/heart_progression_audit 128 0 heart-audit 31 > baseline.jsonl
target/debug/examples/heart_progression_audit 128 0 heart-audit 32 > relocated.jsonl
python3 tools/monster-balance/summarize_player_progression.py baseline.jsonl baseline
python3 tools/monster-balance/summarize_player_progression.py relocated.jsonl relocated
python3 tools/monster-balance/compare_player_progression.py baseline.jsonl relocated.jsonl paired
target/debug/examples/heart_progression_audit fish-ledger 32 > fish-ledger.json
python3 tools/monster-balance/player_sensitivities.py relocated.jsonl fish-ledger.json sensitivities.json
```

The sample is fixed indices 0–127, not replaced when generation fails. Seed 71 normalizes
to `7-10-9:heart-audit-000071` and fails on layer 27: a Veridian Tree tap position was
overwritten by the down stair, so required item placement rejects it. This existing
generation issue affects both revisions. Paired reports are conditional on the same
127 successfully generated worlds and retain the failed seed/error explicitly.

Summary validation checks every entry/exit chain, actual ordinary blessing budget,
level threshold, maximum HP and the total Unblessed stat-grant sum. The example has a
real-inventory regression for six Biter hearts and both reward policies. Independent
QA also recomputes species rewards and Zillennia qualities from source templates.
Distribution files retain means, sample SD/SE, approximate 95% mean intervals,
min/p10/median/p90/max and first/second-half means; joint profiles are retained for
combat evaluation instead of fighting only an artificial mean-stat player.

## Complexity

For S seeds, D≤40 floors, H generated heart-bearing actors per world, and V cells per
floor, the wrapper performs S existing full-generation pipelines; their full geometry,
placement and feasibility costs remain those documented by the generation subsystem.
It does not substitute a cheaper horde draw and call it full generation. Collection
uses four tracks, with real heart, inventory, progression, health/event/log paths;
their helper costs are retained rather than claimed constant. The tracks share the
immutable generated `Arc` floors/actor arrays, while inventories stay at three starter
items plus one pending heart. Retained collection logs/resource records are O(H).
Serialized results are O(SD) numeric snapshots plus the actual per-floor species census.

The Python summary is O(SDTK) arithmetic and O(S log S) sorting per quantile cell,
T = 4 tracks and K = 13 metrics; raw census memory is O(SD) plus species records. Paired
comparison has O(SDTK) arithmetic and input memory. Fish sensitivity convolves at most
ten draws per well over a bounded integer reward support; with F≤80 fish and B≤400
blessings, worst-case O(SFBK) time, K≤3 eligible fish species, O(B) distribution scratch.
These are analysis tools only, not runtime gameplay or embedded shipping assets.
