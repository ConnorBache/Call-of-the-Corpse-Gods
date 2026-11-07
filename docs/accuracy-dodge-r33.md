# Revision33 Accuracy / Dodge specialization

## Scope and authority

The designer requested flavor-based Accuracy/Dodge with meaningful favorable and unfavorable extremes; percentage goals are rough guidelines, not exact fit targets. The approved revision33-only overlay changes 17 natural profiles and two explicit Witch-born profiles. **No HP, Offense, Defense, DR, reward, spawn, ability, or global hit-formula change** is part of this tuning. Earlier revision29–32 catalogs and construction remain unchanged. Natural profiles apply once after the frozen body overlay; Witch profiles replace only Accuracy/Dodge after the lighter HP 244/Offense 200 birth body. Loaded/grown actors and Skin Walker appearance projection never reselect their real stats.

The existing melee formula remains `clamp(0.65 * 0.9^(Dodge - Accuracy), 0, 1)`. Ranged probability multiplies the clamped result by `0.9^range`. A five-point Accuracy lead already caps melee probability, so numerical differences in a saturated matchup need not change the result. Surprise, statuses, specials and actual range still follow native engine rules.

## Reference and measurement contract

* **Primary:** frozen r32 actual-generator `all_initial_unblessed.entry`, 127 complete joint worlds from the original 128 seed census (failed seed71 retained in provenance). All initially generated hearts, including machine actors and initial Zillennia, plus real seeded Unblessed level rewards. Every species uses its actual earliest approved depth, not the final-floor build.
* **Sensitivity:** `ordinary_unblessed.entry`, the historical non-Godbeast initial-actor track. It is not the primary baseline or an equal-survival promise. Conditional-event accounting is stated separately below.
* **Before versus after:** both execute the same current revision33 engine. Before explicitly patches only the original body statistics; after has **no candidate overrides** and verifies the shipping constructor values. This isolates the overlay from unrelated new revision33 mechanics. Identical profiles and combat seed prefixes are paired.
* Each mean row uses all 127 real joint profiles, one combat seed each. P10/P90 selects one actual whole profile by Accuracy or Dodge rank, deterministic seed-index tie break, and runs32 combat seeds; other stats stay correlated. It is not an independently spliced extreme build or an empirical confidence interval.
* The native 9×9 arena dispatches real commands, AI, energy, regen, statuses, lifesteal, summons and terrain. Ordinary bodies begin adjacent; Horns, Wendigo, Vampire and Queen use the existing four-tile chase. Limits:256 combat commands,64 escape-settlement commands. Range4 probability is an opening diagnostic, not a claim that every ranged attack occurs at that distance.
* Gross means sum actual native pre-HP-cap packets divided by each trial's own initial HP. Lifetimes are attack attempts/world turns **conditional on a kill**; dead/censored trials do not prove a completed lifespan. Full JSON/CSV retain net costs, standard errors, deaths, combat/aftermath censors, maximum individual packet fractions and source/binary hashes.
* Results cover controlled encounters and selected NPC pairs, **not all 40 floors of survival balance, optimal tactics, loot builds, or all emergent fights**.

## Approved profiles and primary results

Gross = mean primary combat damage as percent of starting HP. Attempts = kill-conditional player attack attempts. Deaths are after /127; a nonzero death rate makes the displayed lifespan conditional. Queen's gross is a partial encounter diagnostic, not a complete summoner budget.

| Species (entry depth) | Accuracy before→after | Dodge before→after | Gross % before→after | Attempts before→after | Player→enemy mean hit after | Enemy→player mean hit after | Deaths after |
|---|---:|---:|---:|---:|---:|---:|---:|
| rat (1) | -1→-1 | 3→4 | 18.9→22.9 | 3.88→4.52 | 42.6% | 58.5% | 0 |
| cave-bat (1) | -2→-2 | 5→7 | 13.6→18.5 | 3.99→4.87 | 31.1% | 52.7% | 1 |
| shellback-beetle (1) | -2→-2 | -3→-7 | 25.7→23.5 | 6.72→6.03 | 100.0% | 52.7% | 0 |
| bear (6) | 10→6 | -5→-9 | 27.0→18.4 | 5.10→5.04 | 100.0% | 42.4% | 0 |
| giant-beetle (8) | 12→8 | 16→8 | 30.1→20.6 | 5.31→5.29 | 100.0% | 39.1% | 0 |
| tiger (10) | 21→25 | 23→38 | 21.1→28.9 | 3.09→3.33 | 96.1% | 83.9% | 1 |
| horn-1 (12) | 37→41 | 43→43 | 14.8→16.3 | 4.33→4.28 | 96.6% | 87.0% | 0 |
| wendigo (17) | 96→100 | 34→50 | 24.5→29.0 | 3.13→3.18 | 99.2% | 78.5% | 0 |
| troll (19) | 117→111 | 24→16 | 39.3→31.3 | 6.91→6.95 | 100.0% | 54.5% | 0 |
| dire-bear (20) | 128→124 | 17→9 | 35.2→30.4 | 7.43→7.41 | 100.0% | 58.0% | 0 |
| horn-3 (22) | 132→136 | 75→75 | 18.3→20.1 | 4.55→4.50 | 91.5% | 76.4% | 1 |
| night-crawler (24) | 140→144 | 35→74 | 20.9→24.8 | 5.72→6.04 | 98.3% | 79.6% | 2 |
| sabretooth (25) | 138→142 | 46→78 | 26.3→30.3 | 3.19→3.26 | 97.8% | 72.1% | 1 |
| skin-walker (26) | 148→148 | 44→92 | 21.3→29.6 | 4.78→5.57 | 88.4% | 74.3% | 9 |
| vampire (30) | 154→158 | 56→92 | 31.0→36.5 | 6.90→7.33 | 97.2% | 82.7% | 4 |
| minotaur (33) | 153→147 | 37→29 | 45.9→37.8 | 7.87→7.83 | 100.0% | 52.0% | 0 |
| saidribee-queen (28) | 145→145 | 35→27 | 35.4→35.4 | 4.77→4.77 | 100.0% | 65.4% | 23 |

Witch-born Night Crawler: Accuracy 140/Dodge 64; Witch-born Skin Walker: Accuracy 144/Dodge 78. Both retain HP 244/Offense 200. Three successful native casts are an isolated payload, excluding parent and the intervening 20 turns: primary gross22.3→23.9%, all 127 payload fights completed and no deaths/censors. Ordinary-only payload gross42.3→50.0%, deaths1→10/127. Actual Witch chase still leaves descendants alive: primary after 4/127 deaths and123 aftermath censors. Do not present its27.98% gross as a complete encounter fit.

## Meaningful counters and unresolved tails

* Skin Walker Dodge 92 is genuinely evasive, not universally untouchable: full-heart mean incoming hit88.4%; its actual Accuracy-P10 opponent hits52.65%, takes66.9% gross and dies3/32; Accuracy-P90 hits100%, takes7.1% and dies0/32. Across all 127 primary profiles,9 die. Ordinary-only has53/127 deaths and69.2% mean gross; that counter-build sensitivity is intentionally visible.
* Vampire remains polarized through evasion and life drain: frozen-reference mean gross 36.5%, 4/127 deaths; ordinary-only 82.1%, 69/127 deaths. Final Dodge is 92 (eased from exploratory 96 after actual r33 measurements below). Night Crawler is deliberately less evasive (Dodge 74), rather than copying Skin Walker's role.
* Same-depth giant beetle Accuracy 8 versus Tiger Dodge 38 gives2.76% melee hit, versus Tiger's100% return hit; Tiger wins64/64 native ally-oriented trials. This is a deliberately unfavorable extreme, not an accidentally averaged-away interaction.
* Horn1 counters Tiger:89.16% opening melee-equivalent chance (58.5% at range4), Tiger's9.76% return melee chance; Horn wins64/64 native ally-oriented trials. Precision and range remain separate.
* Late native Accuracy is often high enough to counter the evasive species: Minotaur147, Horn3 136 and Vampire158 all hit Skin Walker92 at 100% in melee. Minotaur wins64/64 (mean1.91 native commands). These saturated NPC matchups are reported, not hidden or “fixed” by making Skin Walker invulnerable to ordinary players.
* Native NPC matrix: nine species pairs whose natural windows overlap, both ally orientations, plus separately labeled reciprocal hostile packet probes (normal same-side enemies do not spontaneously fight). 3,456 trials, zero errors or256-command timeouts, sealed spectator player untouched. Ally turn ordering matters, so orientation is retained. Packet probes are not an AI/world-time model.
* All12,988 player before/after trials had zero combat-horizon timeouts. Queen remains unresolved: primary23/127 deaths and104/127 aftermath censors from surviving children. All initial Queen HP 430 is preserved; a prior analysis helper erroneously derived388 from Defense 48. The corrected helper preserves authored HP unless a candidate explicitly requests otherwise.
* Rare-hit/large-packet screen: flag opening mean enemy hit below 10% together with an observed individual packet above 35% initial HP. No row was flagged. This diagnostic is not a balance rule or proof against all bursts. Largest after packet observed was 40.96% (ordinary-only Wendigo, opening mean hit 90.63%, not rare); complete packet maxima remain in the CSV/JSON.
* Additional **untouched** high-Offense sentinels were checked, not retuned: Horn6, Chimera, Dragon, Ursa Major and Administrator, each against real same-depth P10/P90 Dodge joint profiles, 32 seeds per case (320 total). No deaths or timeouts occurred in these selected trials. At P90 Dodge, Horn6/Dragon/Ursa Major/Administrator have opening melee chances 3.40%/6.40%/8.78%/9.76%, with largest observed packets 21.63%/27.93%/10.62%/12.16% HP. Dragon's P10 case costs 87.6% gross versus 8.2% at P90: strong extremes remain visible. The complete results are `r33-accuracy-untouched-heavy-sentinels.json`.

## Fresh revision33 reference: 32 generated worlds

The frozen r32 cohort is retained for a stable before/after reference, **not used to hide the actual r33 generation shift**. A fresh census generated all 40 floors for 32 fixed `heart-audit` seed identities (indices0–31), with zero failures/substitutions. This is actual generation and heart consumption, not a survival run. Its `all_initial_unblessed` entrance profiles are the primary fresh reference; each case below runs32 real profiles ×4 combat seeds =128 native trials. This is still a small cohort and only four representative species, not an all 40-floor balance certification.

Each conditional Nest contributes exactly 13 additive blessings (Queen 3 + four Short ×1 + two Guardian ×3), outside the ordinary budget. The unchanged r32 summarizer rejects that old equality; the r33 extractor checks the separate contribution explicitly. The historical ordinary track now includes these non-Godbeast Nest residents; all_initial also includes initial Zillennia. The final Vampire Dodge-only change does not change generation RNG, fixed encounter-danger metadata or heart rewards, so the generation census remains valid.

| Species | Fresh primary gross before→final | Final deaths /128 | Combat timeouts | Aftermath censors |
|---|---:|---:|---:|---:|
| night-crawler | 24.0→26.2% | 0 | 0 | 0 |
| skin-walker | 23.8→32.3% | 5 | 0 | 0 |
| vampire | 39.5→45.6% | 9 | 0 | 0 |
| saidribee-queen | 37.9→37.9% | 24 | 0 | 104 |

Paired identical 32 seed identities show the progression change: at depth30 r32→r33 mean Accuracy 116.59→110.91 and Dodge 143.00→133.41; at depth26 Accuracy 109.38→105.97 and Dodge 139.16→133.41. Data are in `r33-refresh-debug32-paired-comparison.json`. This materially increases exposure to accurate lifesteal attackers despite similar health.

The exploratory Vampire Dodge 96 produced 51.7% mean gross and 17/128 deaths on these actual r33 profiles. A narrowly approved four-point ease to **Dodge 92** gives final no-candidate 45.59% and 9/128 deaths, without changing HP, Offense or Accuracy. Candidate Dodge 90 was not adopted because it saturates the real P10-Accuracy counter at 100%, flattening that distinction unnecessarily. The observed minimum-Accuracy joint profile (Accuracy 82, actual HP/Offense/Dodge/DR retained) still hits only 22.66% and loses 31/32 final no-candidate native trials. That severe unfavorable extreme is preserved and disclosed, not replaced with an arbitrary 999-stat opponent.

An earlier independent eight-world sample exaggerated the mean danger (exploratory pre-final Vampire 73.7%, Skin 56.0%, Night 36.7%). Its raw data, matched-seed comparison and receipts remain as small-sample evidence, **not current shipping means**. The larger fixed32 result above is the actual r33 checkpoint used for the final decision. Queen and Witch parent/remaining-brood censors still prevent a complete encounter-balance claim.

## Reproduction and artifacts

```sh
sh tools/cargo.sh test -p corpse-core --test accuracy_dodge_v33 --test witch_skinwalker_v33 --test monster_balance_v32 --example physical_trade_arena --locked
sh tools/cargo.sh build --release -p corpse-core --example physical_trade_arena --example accuracy_npc_arena --example heart_progression_audit --locked
python3 tools/monster-balance/accuracy_dodge_r33.py --phase shipping
python3 tools/monster-balance/accuracy_npc_r33.py --phase shipping --specs tools/monster-balance/r33-accuracy-shipping-method.json
python3 tools/monster-balance/accuracy_witch_r33.py
```

Compact matrix: `tools/monster-balance/r33-accuracy-summary.{json,csv}`. Frozen detailed outputs: `tools/monster-balance/r33-accuracy-shipping.{json,csv}`, `r33-accuracy-shipping-method.json`, `r33-accuracy-npc-shipping.{json,csv}`, `r33-accuracy-witch-shipping.{json,csv}`. Fresh final results: `r33-refresh-debug32*`, `r33-accuracy-refresh32-shipping*`, `r33-accuracy-vampire-refresh32-final*`, and `r33-accuracy-vampire-real-low-tail-final.json`. Earlier eight-world and candidate files are exploratory, not current shipping results. Exploratory `proposal*` artifacts are not shipping assertions. `verification/accuracy-dodge-v33-tests.log` records35 passing targeted tests; no full workspace suite was run for this subtask.

## Complexity

Natural specialization scans K=17 fixed immutable rows once per new actor; Witch birth scans B=2 rows once after its body setup. O(K)/O(B) time, O(1) mutation memory after one-time O(K+B) JSON parse/cache storage; no RNG, render work, per-turn lookup, loaded-actor retune, or growing-actor rescale. The analytical driver stores O(R×C) summary rows plus O(S×40) profile input; percentile selection sorts O(S log S) per selected species/stat. Native arenas retain full dispatch/validation/pathfinding/visibility/environment cost and bounded command horizons; they are not claimed constant-time just because the room is fixed. NPC arena has81 cells, two initial species, T≤4096 trials and256 commands; corpse/revival/state behavior remains native. Extra maximum-packet instrumentation is O(1) per observed packet and no additional asymptotic memory. Timing measurements do not follow from these bounds.
