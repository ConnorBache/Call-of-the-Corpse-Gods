# Revision 50 roster and implementation audit

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

Authority: [the exact designer text](revision50-request.txt). This is a **pre-integration source audit**, not a claim that adding a catalog implements its mechanics. Runtime integration work may resolve rows below; use its tests and final release notes for final support status. Existing hook means a real execution path was located, not complete semantic equivalence or newly passed tests. Historical `ability-native-coverage.md` describes the old roster and must not be read as Revision 50 completion evidence.

## Data contract and compatibility

- `crates/corpse-core/catalog/abilities-v50.json` contains exactly 72 identities, twelve contiguous god groups of six. Names and full description paragraphs, including authored Implementation paragraphs, match the request verbatim except the nine explicitly authorized follow-up amendments recorded below. `ability-description-amendments-v50.json` records their revised full descriptions. Flags describe UI intent, not executable admission.
- The immutable `abilities-v48.json` remains unchanged. `progression::catalog_for_revision` must select the new data only for revision >=50; old saves/replays keep their old identities and behavior. Do not replace `catalog()` globally.
- `ability-identities-v50.json` is a complete, ordered 72-row identity correspondence with `godId`, one-based `slot`, `legacyId`, `revision50Id`, and `name`. It is metadata for UI and an explicitly versioned migration, not authorization to silently upgrade old saves. Most entries map to themselves. `numo-revert` maps to the existing revision41 `numo-weave-of-time` identity; Sparkle keeps its stable `anavani-illuminate` ID.
- No entry has a general `craft` flag. Kative still has Cogworks, Clay Army, Appropriate, Tinker, Bottomless Pack and Mini-Me. Historical craft abilities must remain available only to historical runs.
- Runtime selection needs to cover progression offers, initial starter selection, grants/Tinker, validation, UI descriptions, copied-power admission, old-save handling and temporal snapshots. Metadata lookup by ID alone cannot choose between changed descriptions with identical IDs; pass revision where descriptions matter. Starter pools must not keep offering removed identities in new runs. This patch does not invent which replacement powers should be permitted as starters.
- `blockedSemantics` records only still-unresolved authored choices after the follow-up. Empty entries do **not** assert execution support. A runtime gate must reject unsupported actions before charging energy, changing state, consuming RNG or taking a turn.

## Audit of all 72

Paths below are relative to `crates/corpse-core/src/`; multiple paths identify cross-system checks, not guarantees of complete testing. Replacement rows describe missing semantics in the inspected baseline, before concurrent integration.

| # | Revision 50 blessing | Baseline finding | Source and scope |
|---|---|---|---|
| 1 | Rage at the Dying of the Light | Existing hook | `divine_rescues.rs; progression.rs`: Escalating rescue and level reset exist; no new combined rescue proof. |
| 2 | Execute | Replacement | `engine.rs; abilities.rs`: Old paid double-attack Assault is not Execute. Needs strict below-25% test, paid20 instant kill and insufficient-funds behavior. |
| 3 | Grappler | Unresolved replacement | `movement.rs; engine.rs`: Old I Hit Back counter is not Grappler. Multiple-adjacent-enemy movement conflict is not authored. |
| 4 | Fight Long | Existing hook | `divine_combat.rs`: Heals one fifth of resolved melee HP loss, capped at max; overkill excluded. |
| 5 | Unstoppable Force | Replacement | `divine_combat.rs; engine.rs`: Old Beat to Death debuff is not displacement/swap. Needs occupancy and movement-hazard integration. |
| 6 | Masochism | Unresolved replacement | `engine.rs`: Old Fury Crafting is removed. Healing retriggers and simultaneous-threshold stacking are not authored. |
| 7 | You Shall Never Be Rid of Me | Existing hook | `divine_rescues.rs; progression.rs`: Full-health rescue, consumed-until-level flag and deactivation exist. |
| 8 | I Came For You | Preserved mismatch | `abilities.rs; engine.rs; vision.rs`: Legacy activation costs10 and target expires after20 rounds; neither restriction appears in r50 text. Existing closing-distance/free-turn/telepathy hooks need revised lifecycle. |
| 9 | Fated Duel | Unresolved replacement | `abilities.rs; engine.rs`: Old counterattack is not Fated Duel. Exact area geometry is unresolved; do not assume 5×5. |
| 10 | Never Surrender | Existing hook | `divine_reactions.rs`: Costs8 and adds defense as DR for first damaging strike in tick. Non-attack damage packets need scrutiny against broad take-damage wording. |
| 11 | Bulwark of Mind | Existing hook | `abilities.rs; movement.rs`: Visible empty placement,20 energy, enemy-entry ward and expiry exist. |
| 12 | Relentless Pursuit | Replacement | `movement.rs; abilities.rs`: Old Zeal craft/heal is not moving-target path pursuit. Requires obstacle-first path with enemy traversal only if otherwise unreachable. |
| 13 | Missed Me | Unresolved changed rule | `engine.rs; abilities.rs`: Legacy reaction halves hit chance rather than retreats. No-legal-retreat case is unresolved. |
| 14 | Tools of the Trade | Changed rule | `inventory.rs`: Free potion/heart action behavior exists; typed +5 ring/+5 potion capacity is new. |
| 15 | Make Slippery | Existing hook | `abilities.rs; movement.rs`: Selected-target movement denial and8-energy charge exist; ordinary attacks remain possible. |
| 16 | Quick Hand | Existing hook | `divine_reactions.rs; engine.rs`: Reaction expenditure and one-turn haste exist, including landed zero-damage hits; wait refresh uses shared pool. |
| 17 | Are You Sure About That? | Existing hook | `divine_reactions.rs; engine.rs`: Miss deflection selects deterministic-RNG adjacent hostile and spends reaction; full recursive interactions not newly verified. |
| 18 | Spellslinger | Replacement | `inventory.rs; abilities.rs`: Old Guile craft/capacity is not once-per-turn free amulet activation. General amulet energy/cooldowns are unspecified. |
| 19 | Implant Thought | Changed rule | `abilities.rs; engine.rs`: Legacy30-energy temporary ally must become15-energy immediate forced attack with two selected creatures and visibility/adjacency checks. |
| 20 | Distract | Existing hook | `abilities.rs; engine.rs`: 10-energy distracted state forces next attack miss. |
| 21 | Mind Read | Preserved mismatch | `abilities.rs; vision.rs`: Telepathy radii exist; old1-energy upkeep remains in baseline although absent in r50 text. |
| 22 | Mind Slave | Existing hook | `abilities.rs`: 100-energy threshold at20% and perfect-ally state exist; retained ownership and scheduling boundaries apply. |
| 23 | Weave of Time | Existing hook | `timeweave.rs; progression.rs; temporal.rs`: Revision41+ identity and time-weave implementation already exist. Not legacy Revert; keep historical temporal behavior revision-gated. |
| 24 | Remember | Bounded existing hook | `abilities.rs`: Five-round trait copying has explicit supported-copy admission. Catalog text does not imply arbitrary enemy AI can execute as player powers. |
| 25 | Phase Through | Existing hook | `divine_environment.rs; abilities.rs`: 5-energy upkeep, wall traversal/flight and crushing on deactivation exist. |
| 26 | Rushing Torrent | Existing hook | `abilities.rs; ai.rs`: 15-energy upkeep and active haste exist. |
| 27 | Drown | Existing hook | `divine_environment.rs; engine.rs; currents.rs`: Water healing multiplier, damage multiplier and item-current interaction are existing hooks; this is health regeneration, not permission for energy regeneration. |
| 28 | Float Away | Existing hook with boundary | `engine.rs; movement.rs`: Damage-triggered movement/flight exists. Verify enemy-only source and exact until-next-turn expiry; old description used end of next turn. |
| 29 | Half Here | Bounded existing hook | `half_here.rs; vision.rs`: Stationary shared-health clone, sight and removal exist; baseline additionally shares energy/ability ranges and removes on floor change. |
| 30 | Riptide | Existing hook | `abilities.rs; divine_environment.rs`: 15 activation/10 upkeep and rally movement/immobilization exist. |
| 31 | Waking Nightmares | Replacement | `divine_reactions.rs; abilities.rs`: Old nightmare absorbs incoming damage; not a targetable decoy.100-turn resource timer exists but needs new identity/lifecycle wiring. |
| 32 | Sudden Sleep | Existing hook | `abilities.rs`: 20-energy full heal and51 pre-turn paralysis accounting implement50 subsequent turns; verify through action schedule, not raw counter alone. |
| 33 | Dream of Time Gone By | Existing hook | `abilities.rs`: 50-energy spatial anchor and validated return exist; not a full-world rewind. |
| 34 | Sleep Walk | Existing hook | `ai.rs`: Zero-energy haste exists; no energy regeneration should be introduced. |
| 35 | Dream-See | Preserved mismatch | `abilities.rs; vision.rs`: Sleeping/line-of-sight telepathy exists; old1-energy upkeep absent from r50 text remains in baseline. |
| 36 | Oblivious | Replacement | `engine.rs; vision.rs`: Old Dream Craft flight/cap bonuses are not unseen-enemy immunity. Requires damage, targeting and non-damage effects source attribution. |
| 37 | Overflowing | Existing hook | `divine_rescues.rs; progression.rs`: Blood rescue,5 initial/55 cap and50-damage accrual exist. |
| 38 | Bleed Them | Existing hook | `divine_reactions.rs`: Offense-derived bleed hook exists; revision44 damage model differs from obsolete catalog parenthesis. R50 only says proportional. |
| 39 | Blood Letting | Existing hook | `abilities.rs`: 10 self-damage and status cleanup exist; lethal/rescue ordering is inherited, not newly retested. |
| 40 | Hardened Blood | Existing hook | `divine_combat.rs`: Missing derived maximum HP divided by10, rounded up, contributes DR. |
| 41 | Blood Weapon | Existing hook | `divine_combat.rs; engine.rs`: Rounded10%-max-HP priming, hit guarantee and double damage exist; defensive effects and single priming are retained boundaries. |
| 42 | The Number of Blood | Existing hook with boundary | `engine.rs (angel_ready)`: Every fifth tick grants first resolved attack guarantee/double damage, not an autonomous attack. Wording does not specify an automatic target. |
| 43 | Step Through Shadows | Existing hook with boundary | `ai.rs (is_hasted_in)`: Uses enemy seesPlayer state plus detection chance. Existing hook, not just catalog admission; unseen-aware hunter edge cases need tests. |
| 44 | Silent Strike | Preserved mismatch | `engine.rs; combat.rs`: No Silent Strike execution check found: surprise uses sleeping/wandering state and generic3×; sixfold unseen-target bonus is missing in inspected baseline. |
| 45 | Unspeak | Existing hook | `abilities.rs`: Dynamic highest-cost suppression/upkeep exists, subject to supported enemy ability scheduler. |
| 46 | Reverse Entropy | Existing hook | `divine_environment.rs`: Healing/energy/revival and propagation suppression exist within radius; native-library/environment interactions remain inherited. |
| 47 | Shadow in the Dark | Existing hook | `vision.rs; abilities.rs`: Negative light multiplier, active -20 modifier and2-energy upkeep exist. |
| 48 | Not There | Existing hook | `vision.rs; abilities.rs; engine.rs`: 3-energy upkeep and beyond5 ranged targeting boundary exist; every special ability must still preserve this guard. |
| 49 | Boundless Hunger | Existing hook | `resources.rs; inventory.rs`: Energy gain cap exemption exists. No regeneration is implied by uncapped storage. |
| 50 | Entropy | Existing hook | `divine_environment.rs`: Caustic immunity, originating cloud and suppression toggle exist. |
| 51 | Erase Metaphysical | Existing hook | `engine.rs; abilities.rs`: Paid10 random trait/ability erasure exists; no-target-property case must remain free and deterministic. |
| 52 | Death Swift | Existing hook | `engine.rs`: Kill-triggered3-turn haste exists. |
| 53 | Black Pit | Existing hook | `divine_environment.rs; movement.rs`: 5-energy upkeep, departure chasms and recovery exist. |
| 54 | Witch-Hatred | Replacement | `engine.rs; vision.rs`: Uncast spell disruption is not passive caster telepathy/instant kill. Must use actual Magickal Caster classification, not name guesses. |
| 55 | Shine | Preserved mismatch | `vision.rs (emits_shine)`: Revision46 active mode suppresses entire source; r50 specifies no self illumination. Surrounding light and accuracy must not silently inherit whole-source suppression. |
| 56 | Ascend | Existing hook | `abilities.rs`: Passive flight and50-energy ascent through above-floor chasm exist; map annotation/display is separate UI evidence. |
| 57 | Nova | Existing hook | `abilities.rs (nova_attack_cost); engine.rs`: Revision45+ already uses40 per attack; old catalog30 is stale. Half-max-HP explosion and immunity have existing paths. |
| 58 | Sparkle | Preserved mismatch | `abilities.rs; environment.rs`: 20-energy permanent source exists but rejects walls/chasm/pit/plain water; r50 says any visible square, not combustible only. |
| 59 | Star Power | Existing hook | `divine_environment.rs`: Passive immunity/permanent burning and free suppression/resumption exist. |
| 60 | Commandment | Replacement | `engine.rs`: Old targeted paralysis Make Kneel is not passive one-tile/active20-energy maximum knockback. |
| 61 | Cogworks | Partly unresolved replacement | `abilities.rs; movement.rs`: Old enhancement-slot passive is not pit/wall construction.20-energy modes1/2,200-HP wall, perpendicular geometry and visible preview need implementation; removal energy unresolved. |
| 62 | Mini-Me | Unresolved adaptation | `mini_me.rs`: Bounded level1 starter clone exists; weapon/armor and enhancement references conflict with removed equipping. Do not silently substitute equipment semantics. |
| 63 | Clay Army | Replacement | `engine.rs; movement.rs`: Old Adaptive Armor mitigation is not a stationary unbreakable soldier copying attacks. Needs displacement, path collision and end-turn attack scheduling. |
| 64 | Appropriate | Replacement | `inventory.rs`: Old Artifice Craft is not typed jewelry capacity/cooldown modifier. +3 ring/+3 amulet and50% cooldown reduction are authored; amulet base values are not. |
| 65 | Tinker | Unresolved adaptation | `tinker.rs`: Deterministic outside-god nonduplicate item grants exist, but weapon/armor equipped-only text conflicts with removed equipping; unsupported grants require atomic rejection. |
| 66 | Bottomless Pack | Changed/replacement rule | `inventory.rs`: Dilute half-efficacy reuse exists under legacy ID; new +2 amulet/+5 potion capacity and identity needed. |
| 67 | Fortune Teller | Existing hook with boundary | `abilities.rs; engine.rs`: Fortune spending and lucky/unlucky exist. Legacy20-turn recharge and3 initial fortunes are additional preserved rules, not explicitly stated here. |
| 68 | Bonded By Unseen Threads | Preserved mismatch | `abilities.rs; engine.rs (share_bonded_packet)`: Half-damage sharing exists but legacy15 activation/8 upkeep are absent from r50 text. |
| 69 | Turning the Wheel | Existing hook | `abilities.rs; engine.rs`: 20-energy upkeep and enemy attack blocking exist; interaction with source-specific effects not newly proven. |
| 70 | Sever Fate | Existing hook | `abilities.rs; engine.rs`: 30-energy below-half threshold and neutral-until-harmed behavior exist. |
| 71 | See The Future | Existing hook | `temporal.rs`: Full-energy snapshot/rollback exists at Engine boundary; raw core activation is deliberately unsupported. |
| 72 | All-Seeing Eyes | Preserved boundary | `abilities.rs; engine.rs; vision.rs`: 30-energy reveal exists; baseline expires after next time-consuming action, while r50 says next action. Free consumable/amulet actions need explicit r50 expiry test. |

## Historical follow-up decisions

Later decisions supersede this early follow-up: Mini-Me copies stats only;
Masochism gives at most10 haste turns per hit; all blessing pools start/refill at50;
amulets start at50, have no cooldowns and use split payment; Appropriate subtracts10;
Reaping restores all owned amulets per20 direct hostile HP damage. Final rings,
amulets, potions and unlimited trinkets are implemented in the linked current
subsystem notes. No item-energy or wall-health design decision remains open.


The designer subsequently said:

> Cogworks removal cost should be free, fated duel's area is a 5x5 square. Missed Me with no escape fails and doesn't spend the reaction. For grappler a character cannot be moved with the character, they are not moved. For masochism, retriggers are fine.
>
> Tinker will change to grant a not our god ability and put it onto any ring or amulet, and the ring or amulet will have both.
>
> Mini Me should just say it's a complete copy of you.

The seven corresponding descriptions now reflect those decisions. Existing activation costs remain: Mini-Me30, Tinker100, Fated Duel3 per turn, Cogworks modes1/2 each20. Tinker preserves existing jewelry abilities and adds its new grant. Mini-Me no longer says level1 or unenchanted. No arbitrary equipment-adaptation blocker remains.

The audit rows above describe the pre-follow-up baseline; their formerly open decisions are resolved by this section, not reasons to keep those abilities globally disabled. Masochism's retrigger rule is now resolved, but simultaneous-crossing haste stacking remains unspecified. Do not infer duration stacking from permission to retrigger.

In addition, the amulet system specifies personal reserves, player-energy fallback, cooldowns and Appropriate's 50% reduction, but no authored per-amulet activation energies, reserves, cooldown durations or effect roster. Do not fabricate these values. Separate blessing energy bars remain a proposal with no final rules. Ring qualitative tiers have no authored distribution or tier definitions here.

## General-rule acceptance checklist

- No weapon/armor equip commands in revision50; bows Apply directly. Unlimited simultaneously worn rings/amulets; worn jewelry absent from backpack accounting.
- Latest approved capacities are shared jewelry5, potions5, hearts5, and unlimited trinkets. Tools adds ring-only5/potion5; Bottomless adds amulet-only2/potion5; Appropriate adds ring-only3/amulet-only3. Combined finite capacities: shared5/ring8/amulet5/potion15/heart5. Trinkets include bows, arrow stacks, tools, quest shards and special usable items, with one trinket slot per stack. Admission solves shared versus dedicated jewelry allocation independently of order. See `revision50-items-generation.md` for the subsequent source implementation; the older 7-trinket/22-total draft is superseded.
- Potions/hearts with Tools take no action, including haste/temporal accounting. No player energy regeneration, including waits, off-screen/sleep phases and indirect resource refresh. Existing energy gains from explicitly authored consumables are a separate rule.
- No hidden implementation of removed crafts through item grants, starter pools or command aliases. Old revisions retain them.

## Verification and complexity

`tests/revision50_catalog.rs` is an automatically discovered integration test: it compares every name/description against the literal request plus the authorized description amendments, checks six entries per god, unique identities, the complete legacy correspondence, preserved special identities, and removal of craft flags. It is a data test only. Runtime unit/differential/UI/save-load tests are required separately. The test was authored here but could not be compiled in the available environment; no compiler, dependency or network installation was attempted. Python extraction and JSON contract checks passed locally.

The new files are static data; no simulation algorithm is added. With A=72 records and B total UTF-8 bytes, first parse is O(B+A) time/memory; cached revision selection is O(1), metadata lookup in an unsorted vector is O(A) comparisons plus ID bytes. An explicit migration by naive ID-map lookup is O(K*A) comparisons for K referenced ability IDs, plus serialization bytes; account for every actor, item, initial profile and temporal snapshot, not only the player. The catalog regression parser is O(B), with O(B+A) extra memory; its temporary ID set adds O(A log A) comparisons. These are source-level bounds, not measured timings or platform performance claims. The integration owner should link this entry from the main complexity ledger when wiring runtime selection.

## Preserved-edge integration follow-up

The baseline mismatch for Sparkle (row 58) and action-boundary gap for All-Seeing Eyes (row 72) now have revision-gated execution changes and deterministic test cases. [The preserved-edge coverage note](revision50-preserved-edges.md) specifies no-action versus bonus-action behavior, permanent fire on noncombustible terrain, temporal scope, complexity, and test limitations. Those source gaps now have executed regressions in the historical stats-only checkpoint. Current item integration and its verification are described in the implementation-status and native-verification reports; Revision 50 release admission remains closed.

## Latest designer decisions

Later instructions resolve Masochism to 10 total haste turns even when one hit crosses multiple thresholds, and Clay Soldiers copy the player's current attacks without abilities. The description amendment file now has eight entries. The original uploaded text above remains unchanged as provenance.

Every ability now has its own energy reserve, spent before player energy; level-up refills ability reserves, not player energy. Numeric reserve capacities and partial-funding policy remain unauthored. The implementation stores only explicitly configured reserves and does not fabricate defaults. See revision50-ability-energy.md.

The player now uses authored Greatsword attack arithmetic (45 plus twice Offense) instead of Shortsword arithmetic. Enemy baseline revision gates retain the latest existing calibration; no arbitrary retuning was performed. See the new attack/balance report for source-only sensitivity and verification limits.

## Superseding Mini-Me decision, 2026-11-06

The designer subsequently said: “Mini Me doesn't get any abilities, just the stats”.
This replaces the complete-copy wording above. Mini-Me copies current numeric
stats and carries no abilities, equipment grants, reserves, active effects or
ability-linked targets. The 30-energy activation and perfect-ally identity remain.
See the current Mini-Me implementation and native verification reports rather
than the historical audit row for its support status.

## Later amulet and ring clarifications

Amulet cooldowns are removed. This scope does not remove enemy or Mask cooldowns.
Appropriate now reduces amulet energy costs by 10 (minimum zero) instead of
halving cooldowns; its backpack bonuses remain. It is the ninth description
amendment. Amulets have 50 initial/base item energy, with no level-up or passive
refill. Reaping's later energy-return rule supersedes its proposed cooldown
reduction. See current amulet/ring subsystem documents for completed hooks and
remaining payment/Reaping details; the earlier audit and uploaded text remain
historical provenance.
