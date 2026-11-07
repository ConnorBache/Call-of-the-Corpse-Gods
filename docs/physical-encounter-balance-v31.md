# Provisional physical encounter scale, revision 31

## Contract, status and scope

The designer's anchor is a floor-1 Giant Rat costing about **20% of starting player HP over the creature's whole life**, not 20% per hit. Normal creatures here use provisional bands around 20–30%; heavy specialists around 40–60%; the rare final Dragon around 80%, with substantial death risk. Those later exact bands are implementation choices within the instruction, not additional percentages specified by the designer.

This integrated, unreleased revision 31 pass has **48 numeric metadata records: 47 creature templates plus the legacy Lesser Ichor-Fish alias**. Four deliberately retained templates are Blorb, Hemogoblin, Pale Jackal and Bombardier. The earlier 21-record checkpoint was work in progress, not a supported release. This is a measured physical/mixed-physical pass, **not a declaration that all 79 bestiary entries or the whole game are balanced**. Dedicated casters, supports, summoners, rooted fungi/Samara, Pale gas/spit/scream/breather families, Greater Jackal, invulnerable Sphinx and named Godbeasts are not normalized by this pass. Their groups, auras, environmental interactions and reward opportunities require different scenarios. No new ability, spawn range, reward, blood rule or population weight is introduced here. The Administrator and collapse finale are separately implemented; only its isolated combat benchmark is reported below.

The reference player consumes every ordinary heart from preceding floors, enters at full health, keeps the default Shortsword and chooses no abilities, equipment, consumables, level grants or discretionary actions. Passive automatic effects remain active. The 10,000 joint progression profiles use unchanged ordinary-horde selection and heart rewards, dry habitat, no machines, no placement losses, no conditional spawns and no survival selection. They are controlled reference builds, not observed-player averages or a claim that obtaining every heart is realistic. The portable summary also records average starting HP for each actual cohort.

Reference depth is independent of spawn range. Rat, Blorb, Cave Bat, Shellback, Hemogoblin, Needleling and Pale Jackal use floor 1; ordinary leaders otherwise use authored midpoints. Half-floor references mix the two adjacent floor-entry cohorts equally. Conditional Ichor tiers have explicit depths in the table. Bastidion/fungal reference-only entries use floor 1 without a stat change. The bestiary's monotonic visual palette is a separate presentation policy, not a reason to move combat references or spawn bounds.

## Measurement contract

The restricted Python model starts both actors adjacent, alive, awake and hunting. The player attacks first; the normal tick regenerates a living player, including on the killing turn, then a surviving foe can respond. Slow and intrinsic lifesteal/regeneration are included. **Fast grants extra movement only, never extra adjacent attacks.** This model deliberately forces contact: Vampire retreat and Wendigo retreat/corpse growth are not represented by its completed-life figures.

The native arena uses the actual command dispatcher in a bounded 9×9 room. The player attacks when adjacent and otherwise waits, never activates an ability or chases. Real AI, cooldowns, energy, statuses, terrain, corpse behavior and passive effects execute. Aquatics start on their required habitat, normally beside a player on dry land. The primary life ends on the dispatch containing the creature's death; that dispatch can also contain regeneration or status damage. The harness then waits for outstanding player poison/burning/environmental harm to settle. Thus **combat loss** and **settled whole-life loss** are separate, and their difference is net residual cost after regeneration. Raw post-death damage packets are recorded separately and can differ from that net difference.

Death is capped at 100% HP loss and separately counted. A mean cannot substitute for survival probability. If contact breaks, a corpse revives during settlement, or harm remains at the time cap, the case is **censored**, not a completed fight. Partial means are not upper or lower bounds because regeneration may reduce them. `mean_completed_cost` describes only the selected completed subset and is not an estimate for the entire population. Primary rows below have zero censors.

## Current numeric scale

HP remains exactly 100 +6×Defense. The table uses native held-out dispatch where available, otherwise the restricted source-ordered model. N is actual trials, not distinct profiles: model cohorts are repeated with independent combat RNG. The alias reuses Lesser Ichor-Fish evidence and has constructor parity coverage, not an extra independent calibration.

| Creature | Ref | HP | Off | Acc | Dodge | DR | Mean loss | Deaths | N | Evidence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| blorb | 1 | 10 | 2 | -3 | -4 | 0 | 0.03% | 0.00% | 100,000 | restricted model |
| rat | 1 | 22 | 11 | -1 | 3 | 0 | 20.17% | 0.14% | 200,000 | restricted model |
| cave-bat | 1 | 16 | 12 | -2 | 5 | 0 | 17.08% | 0.38% | 100,000 | restricted model |
| shellback-beetle | 1 | 46 | 14 | -2 | -3 | 14 | 19.70% | 0.00% | 100,000 | restricted model |
| hemogoblin | 1 | 34 | 7 | -1 | 2 | 0 | 22.54% | 0.04% | 100,000 | restricted model |
| glim | 11.5 | 142 | 93 | 11 | 13 | 0 | 22.96% | 0.02% | 200,000 | restricted model |
| giant-beetle | 10.5 | 118 | 73 | 9 | 11 | 22 | 24.88% | 0.10% | 200,000 | restricted model |
| bear | 12 | 178 | 93 | 13 | 14 | 0 | 30.20% | 0.06% | 100,000 | restricted model |
| tiger | 16 | 136 | 148 | 33 | 29 | 0 | 25.16% | 0.07% | 100,000 | restricted model |
| horn-2 | 14.5 | 172 | 124 | 21 | 19 | 0 | 27.10% | 0.13% | 200,000 | restricted model |
| wendigo | 19.5 | 178 | 204 | 49 | 39 | 0 | 26.95% | 0.05% | 200,000 | restricted model (forced contact) |
| troll | 21.5 | 364 | 150 | 55 | 34 | 18 | 39.88% | 0.57% | 200,000 | restricted model |
| dire-bear | 25 | 502 | 204 | 56 | 40 | 17 | 40.07% | 0.07% | 100,000 | restricted model |
| killspawn | 27.5 | 940 | 309 | 59 | 40 | 28 | 49.79% | 1.66% | 200,000 | restricted model |
| horn-4 | 28.5 | 376 | 448 | 53 | 45 | 0 | 29.11% | 0.05% | 200,000 | restricted model |
| sabretooth | 29 | 286 | 541 | 60 | 55 | 0 | 29.11% | 0.01% | 100,000 | restricted model |
| vampire | 30 | 430 | 268 | 71 | 55 | 0 | 39.95% | 1.28% | 100,000 | restricted model (forced contact) |
| horn-6 | 33 | 568 | 550 | 73 | 44 | 0 | 30.23% | 0.04% | 100,000 | restricted model |
| cancer | 38 | 1210 | 264 | 103 | 44 | 121 | 50.03% | 0.55% | 100,000 | restricted model |
| ursa-minor | 39 | 1102 | 369 | 91 | 51 | 63 | 29.57% | 0.00% | 2,048 | native dispatch |
| minotaur | 39.5 | 2020 | 302 | 103 | 48 | 87 | 59.86% | 3.61% | 200,000 | restricted model |
| ursa-major | 39 | 1504 | 372 | 103 | 45 | 90 | 59.84% | 3.27% | 2,048 | native dispatch |
| pale:Jackal | 1 | 22 | 4 | -2 | 3 | 0 | 12.52% | 0.00% | 2,048 | native dispatch |
| pale:Biter | 4.5 | 88 | 13 | 5 | 0 | 0 | 24.02% | 0.00% | 1,024 | native dispatch |
| pale:Greater Biter | 20 | 322 | 88 | 54 | 31 | 0 | 33.78% | 0.00% | 1,024 | native dispatch |
| needleling | 1 | 22 | 11 | -3 | 2 | 0 | 19.80% | 0.29% | 1,024 | native dispatch |
| short-saidri | 4 | 64 | 29 | 4 | 1 | 0 | 20.88% | 0.20% | 1,024 | native dispatch |
| long-saidri | 17.5 | 220 | 93 | 42 | 28 | 0 | 25.59% | 0.00% | 1,024 | native dispatch |
| saidri-guardian | 37.5 | 1096 | 531 | 103 | 47 | 58 | 44.32% | 0.20% | 1,024 | native dispatch |
| yeti | 34.5 | 682 | 257 | 96 | 43 | 52 | 44.20% | 0.49% | 1,024 | native dispatch |
| kelpie | 18.5 | 232 | 128 | 46 | 31 | 0 | 26.72% | 0.20% | 1,024 | native dispatch |
| moonspawn | 35.5 | 1186 | 346 | 103 | 47 | 0 | 46.25% | 0.10% | 1,024 | native dispatch |
| river-drake | 25.5 | 424 | 346 | 57 | 47 | 21 | 34.99% | 0.20% | 1,024 | native dispatch |
| lesser-ichor-fish | 3.5 | 58 | 24 | 4 | 1 | 0 | 20.74% | 0.20% | 1,024 | native dispatch |
| ichor-fish-standard | 7 | 94 | 46 | 8 | 8 | 0 | 21.83% | 0.10% | 1,024 | native dispatch |
| greater-ichor-fish | 11.5 | 136 | 90 | 13 | 16 | 4 | 27.50% | 0.00% | 1,024 | native dispatch |
| lesser-ichor-shark | 16.5 | 238 | 88 | 37 | 25 | 9 | 29.43% | 0.20% | 1,024 | native dispatch |
| ichor-shark | 21.5 | 370 | 123 | 57 | 38 | 15 | 32.99% | 0.10% | 1,024 | native dispatch |
| greater-ichor-shark | 26.5 | 520 | 191 | 59 | 46 | 21 | 38.51% | 0.20% | 1,024 | native dispatch |
| lesser-ichor-leviathan | 31.5 | 1006 | 196 | 78 | 42 | 41 | 44.01% | 0.29% | 1,024 | native dispatch |
| ichor-leviathan | 36 | 1600 | 208 | 105 | 43 | 66 | 52.73% | 0.59% | 1,024 | native dispatch |
| greater-ichor-leviathan | 38.5 | 2080 | 212 | 107 | 44 | 82 | 61.19% | 6.54% | 1,024 | native dispatch |
| chimera | 38 | 1642 | 573 | 105 | 46 | 60 | 55.40% | 1.07% | 1,024 | native dispatch |
| dragon | 40 | 3082 | 465 | 110 | 48 | 99 | 82.45% | 39.55% | 1,024 | native dispatch |
| hemogoblin-mech | 15.5 | 346 | 43 | 34 | 17 | 31 | 47.84% | 1.07% | 1,024 | native dispatch |
| ichor-fish | 3.5 | 58 | 24 | 4 | 1 | 0 | 20.74% | 0.20% | 1,024 | native alias parity |
| phoenix | 34 | 940 | 350 | 100 | 55 | 0 | 45.85% | 0.10% | 1,024 | native dispatch |
| bombardier | 17 | 58 | 10 | -2 | -3 | 0 | 26.95% | 0.00% | 1,024 | native dispatch |

### Preserved identities and repaired hierarchy

- Rat changes only Offense 4→11; HP 22, Accuracy−1, Dodge 3 and DR 0 remain. Blorb remains deliberately negligible in an already-adjacent player-first trade. Pale Jackal is **not venomous**: gas immunity alone does not justify a venom model. Its unchanged floor 1 cost is 12.52%, appropriate to its pack/birth context rather than forcing every early body to 20%.
- Cave Bat remains fragile, evasive and flying. Shellback is meaningfully slow and armored: HP 46/DR 14/Offense 14. A minimum-variance landed starter-sword strike still deals 5 damage. Neither is a generic Rat clone.
- Later Dodge is restrained against the lower-collection cohort rather than uniformly matching full-collection player Accuracy. Accuracy rewards therefore retain value. HP and role-specific armor provide much of the durability; offense alone is not used to create rare lethal spikes.
- Ursa Minor was redistributed from the provisional 21-record HP 646/Offense 629 to HP 1102/Offense 369 while retaining about 30% solo cost. Awake Major is HP 1504/Offense 372/Accuracy 103/DR 90, stronger in body, offense, accuracy and armor than Minor while Minor remains more evasive. Frozen Major's modeled floor 39 cost was about 0.008%; the restored native solo cost is 59.84%. Minor's death cry wakes/chases Major; it does not grant an invented rage stat buff.
- Leviathan bodies rise from HP 1006→1600→2080 with their intrinsic 2%-max-HP hit siphon retained. Final Dragon HP 3082 exceeds Chimera HP 1642 and Minotaur HP 2020. Dragon's former native adjacent cost was 1.90%; its restored 82.45% is an explicit extreme exception, not ordinary-fighter tuning.
- Cancer keeps real armor piercing. Minotaur remains a committed-movement **Fighter**, not a Skirmisher. Its roughly 60% stationary cost removes movement counterplay and should not be called normal roaming danger. Troll regeneration and Vampire lifesteal remain active; no Fast attack doubling is assumed.
- Pale Biters retain their distinct 8-energy repeat strike and 8-energy heal paths. Yeti's adjacency slow actually adds world ticks between player attacks. Saidri/Needleling venom is measured through the residual tail. Kelpie's wet-target grasp, Moonspawn blink/everburn, River Drake jet/submerge, piloted Mech breath and Phoenix revival are not replaced by generic melee.

### Native statuses, habitat and approach risk

| Scenario | N | Combat loss | Settled loss | Death rate | Censored |
|---|---:|---:|---:|---:|---:|
| needleling: adjacent | 1,024 | 16.93% | 19.80% | 0.29% | 0 |
| short-saidri: adjacent | 1,024 | 18.81% | 20.88% | 0.20% | 0 |
| long-saidri: adjacent | 1,024 | 23.06% | 25.59% | 0.00% | 0 |
| saidri-guardian: adjacent | 1,024 | 43.69% | 44.32% | 0.20% | 0 |
| hemogoblin-mech: piloted-mech | 1,024 | 41.28% | 47.84% | 1.07% | 0 |
| kelpie: water-wet | 256 | 18.37% | 18.37% | 0.00% | 0 |
| river-drake: water-wet | 256 | 37.76% | 37.76% | 0.39% | 0 |
| moonspawn: approach | 256 | 47.46% | 47.46% | 0.00% | 0 |
| chimera: approach | 256 | 63.66% | 63.86% | 4.30% | 0 |
| dragon: approach | 256 | 87.24% | 87.27% | 68.36% | 0 |
| ursa-major: ursa-pair | 2,048 | 75.07% | 75.07% | 52.73% | 0 |
| phoenix: approach | 1,024 | 46.16% | 46.16% | 0.00% | 0 |
| bombardier: approach | 1,024 | 54.84% | 57.64% | 0.00% | 0 |
| flesh-administrator: adjacent | 2,048 | 81.81% | 81.81% | 28.32% | 0 |

**The linked Ursa pair is an extreme group encounter:52.73% deaths in 2,048 trials, not two independently safe solo fights.** Native death-alert/wake/chase behavior and movement counterplay remain intact. **Dragon kills 39.55% of 1,024 full-heart adjacent players and 68.36% of 256 approaching players.** Its large body and actual breath preserve late hierarchy but its capped 82% mean must never be presented as an 82% chance of surviving. Approximate sampling error for these death rates is about 1.5 percentage points atN 1024 near 40%, and 1.1 points atN 2048 near 53%; the smaller approach cohort is less precise. There is no claim of guaranteed survivability.

Bombardier is intentionally unchanged. Adjacent quick kills allow only 481/1,024 detonations, versus 1,024/1,024 on approach. Adjacent settled cost is 26.95% versus 57.64% on approach; residual fire adds about 2.60 and 2.79 percentage points respectively. That existing counterplay is preserved rather than flattened. Phoenix's bare-floor **first life** costs 45.85%, not a finite sum over indefinitely possible revivals. Its approach first-life cost is 46.16%. Ordinary corpses can also revive, while Phoenix has its accelerated revival path. The player never chooses to consume the corpse in this benchmark.

Executed breath uses the actual engine's exact-scale 24-damage packet plus 4-per-tick burning, with exposure and regeneration determined by the scheduler. Dragon/Chimera/Mech measurements include their real breath dispatch rather than a pure-melee substitute. Bare-floor Everburn does not necessarily spread fire to the player; grass is therefore tested separately.

Unfinished scenarios remain explicit:

| Scenario | Trials | Censored | Reason |
|---|---:|---:|---|
| Vampire, actual stationary adjacent | 256 | 255 | Retreat breaks contact; player does not chase |
| Wendigo, actual stationary adjacent | 256 | 138 | Retreat breaks contact; player does not chase |
| River Drake approach | 256 | 143 | Jet/displacement breaks contact |
| Moonspawn adjacent grass | 256 | 116 | Later revival during ongoing environmental settlement |
| Dragon adjacent grass | 256 | 55 | Later revival during ongoing environmental settlement |
| Phoenix adjacent grass | 1,024 | 719 | Revival/continuing environmental exposure |
| Phoenix approach grass | 1,024 | 675 | Revival/continuing environmental exposure |

No completed whole-encounter mean is claimed for these rows. The full 139-case native output includes additional grass and lower-collection censors, post-death packet totals and mechanic markers. For example, wet Kelpie can spend an action grasping a player who never tries to escape, so its stationary wet cost is lower than its shore cost; that does not imply the grasp is harmless in actual play.

## Lower-collection sensitivity

The requested baseline is **all ordinary hearts**, not an equal-survival promise for under-collected builds. Reducing collection jointly reduces HP, offense, Accuracy, Dodge and DR. Many late encounters consequently become near-certain losses at 50%; that severity is reported rather than concealed or silently corrected through rewards.

| Creature | 75% hearts: mean / deaths | 50% hearts: mean / deaths |
|---|---:|---:|
| rat | 20.12% / 0.18% | 20.19% / 0.14% |
| bear | 60.09% / 7.12% | 95.39% / 79.52% |
| troll | 87.95% / 51.85% | 99.97% / 99.89% |
| cancer | 97.46% / 87.41% | 100.00% / 100.00% |
| minotaur | 98.50% / 96.56% | 100.00% / 100.00% |
| ursa-minor | 85.96% / 40.62% | 99.91% / 99.80% |
| ursa-major | 98.50% / 96.68% | 100.00% / 100.00% |
| pale:Greater Biter | 74.52% / 14.45% | 99.87% / 97.66% |
| yeti | 95.23% / 78.71% | 100.00% / 100.00% |
| greater-ichor-leviathan | 98.84% / 97.66% | 100.00% / 100.00% |
| chimera | 98.08% / 90.82% | 100.00% / 100.00% |
| dragon | 99.52% / 98.83% | 100.00% / 100.00% |
| phoenix | 91.20% / 43.95% | 100.00% / 100.00% |
| bombardier | 48.83% / 0.00% | 61.92% / 0.00% |

All 138 restricted-model cases finish within 1,000 turns, and none of those sampled profiles has zero possible player strike damage. The native primary cases also finish without censors; the retained environmental/retreat sensitivities do not. Observed 100% mortality is not a mathematical proof of immunity or universal impossibility. Starter damage, collection assumptions and capped-death tails are separate questions. A broader change to collection expectations would require its own design decision rather than quietly invalidating these profiles.

## Population, construction and compatibility

`population::creature_danger` reads separate frozen `catalog.danger`, not live actor HP/offense. Authored weights, recipes, introduction ranges, heart templates and budgets are unchanged. Thus the actor-only overlay does not feed back into the ordinary-heart distribution used for the reference profiles. Danger labels retain population-policy meaning; they are not recomputed measured encounter costs.

New revision 31 actors receive the overlay once at construction. Ordinary groups, paired followers, machines, corpse templates and Birther offspring use revision-aware constructors; the separate Mercuria Ichor constructor also applies it. Legacy Pale templates lack `creatureKind`, so an explicit construction-key helper applies their stats without changing identity. Birther's authored post-construction Offense 4 override remains intact; unchanged Jackal is consistent with it. Ichor event-spawn parity is regression tested, not merely assumed from the catalog constructor.

Revision 29/30 constructors, saves and replays retain frozen templates. Loading a wounded, grown or buffed actor never reapplies tuning. Already-stored actors continue normally offscreen and in time-travel snapshots. Vampire form changes retain their actor stats. Lookup consumes no gameplay RNG. Invalid explicit revisions fail before mutation; revision 31 uses revision 30's ordinary pools, with equal recipes checked at all 40 depths in dry/wet habitat states.

## Reproduction and evidence

1. `python tools/physical-balance/validate.py` requires Python/NumPy and reproduces 138 before/after/collection cases,7,640,000 restricted-model trials. This is a source-ordered numerical port, **not the full native engine**. The current 23 modeled records include Minor, Major and Jackal; overlapping native held-out rows are additional evidence, not extra species.
2. Build the actual harness with `sh tools/cargo.sh build --release -p corpse-core --example physical_trade_arena`. Then `python tools/physical-balance/validate_native.py --binary PATH` reproduces the 139 recorded native cases and 102,400 trials. `--limit 2` is a quick smoke option. Profiles use permutation seed 310043, held-out offset 4096 and explicit per-case combat seed prefixes. The first two 1,024-trial cases reproduced exactly, including means, deaths, censors, mechanic counts and residual packets.
3. `fixtures/physical-reference-profiles-v31.npz` losslessly stores the 10,000 joint floor-entry cohorts at 50/75/100% collection, with original seeds/metrics. It is a **developer-only analysis fixture, not embedded in the shipping binary**. Fitting and held-out cases use separate indices within this fixed cohort; validation is not an independent newly generated world population.
4. `sh tools/cargo.sh test -p corpse-core --test physical_balance --example physical_trade_arena -- --nocapture` runs native constructor/reward/recipe invariants, actual command trades, habitat/slow/cadence and deterministic harness regressions. Actual fixed-profile native smoke means were Rat 19.7952%, Shellback 19.9676%, Bat 17.1638% atN 1000; Bear 27.9428%, Dire Bear 54.6855%, Cancer 64.0110% atN 256. Their independent Python expectations for the same later profiles were 28.1469%,55.1240%,63.9087%. These fixed-profile means are not the population means in the table.
5. Cancer's 256 sequences are repeated with player DR 53 versus 1,000; HP/RNG remain identical each command, verifying real armor piercing. Already-adjacent Fast comparison likewise preserves HP/RNG. Tests check 48 unique overlay rows, exact new stats, old templates, traits/hearts, reference bounds, starter Shellback damage and actual Ichor event spawning. Six arena tests cover dispatch determinism, habitat rejection, Yeti slow, linked pair ordering, grass sensitivity and preserved traits/weapon.

Portable files:
- `docs/artifacts/physical-balance-v31-summary.csv`:48-row joined current stats, frozen stats, average reference HP, primary life/death/tail results and 75/50% sensitivity.
- `docs/artifacts/physical-balance-v31-before-after.csv`: complete restricted-model rows with uncapped incoming packets, hit probabilities and confidence intervals.
- `docs/artifacts/physical-balance-v31-native-validation.json`: all 139 summarized native cases, including residual packets, censors and executed mechanic markers.
- `docs/artifacts/physical-balance-v31-native-cases.json`: exact portable scenario/candidate/seed inputs.
- `docs/artifacts/physical-balance-v31-native-summary.csv`: compact native comparison.
- `docs/artifacts/physical-balance-v31-provenance.json`: source/input/output hashes, counts and exclusions.

These controlled arenas do not simulate generated-floor navigation, full party tactics, all abilities, a full run, the collapse escape, or platform rendering. Full-workspace tests and platform/manual review are separate integration checks. No whole-game balance certification follows from passing this harness.

## Complexity

Let T=48 tuning records, K=55 ordinary leaders, P=2 paired leaders, B overlay JSON bytes and S actor/template size. Parse-once tuning costs O(B) time/memory. Construction retains its existing O(S) clone then scans O(T) and updates five scalar stats/HP in O(1). Explicit Pale/Ichor constructor paths add only that same bounded scan. Reference lookup costs O(T+K+P) time and O(1) extra space. There is no per-turn overlay work, gameplay RNG, rendering or pathfinding. Population planner asymptotics remain unchanged.

For offline model validation with N profiles and H≤1,000 turns, full-vector draws/updates cost O(NH) time and O(N) working arrays. Profile decode/storage costs O(3×10,000×40×6), with lossless 16-bit file storage and float analysis conversion. Cases run sequentially. The 7.64-million-fight model run completed in about 7 seconds here; this is a measured observation, not a platform guarantee.

Native harness observation per command costs O(A²+S+V+E) where A is scene actors, S inspected status keys, V=81 tiles and E new log events; actor reconciliation is quadratic. Storing summarized trial outcomes costs O(N), and optional traces O(N×C×(A+S+V+E)) for C commands. Cases have explicit combat/settlement caps 128 or 256. This is **additional harness work only**: real dispatch includes existing validation, snapshots, AI/pathfinding, visibility, environment, native event/log processing and possible spawns/revival. Those engine costs are not hidden inside an O(1) attack claim. Neither harness nor Python profiling assets are shipping per-turn simulation work.
