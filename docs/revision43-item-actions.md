# Revision43 requested item/action changes

This independent playtest preserves revisions29–42 mechanics and recording identities. New desktop games select43. The designer chose at least one special property for generated jewelry and continuously changing distorted creature identities marked with a question mark. The latter remains cosmetic-only; canonical logs and simulation RNG stay deterministic.

### Revision43 Chronicle wording
The central log ingress performs three fixed exact-string comparisons for revision43, O(M) in message bytes, and substitutes requested wording before the existing archive append. Older revisions return their original messages. Tinker success formatting is O(M). Witch summon wording performs the existing summon-side visibility check plus one newborn visibility check and displayed-name projection; those helpers retain their existing vision/cache/disguise costs, not O(1). Formatting adds O(name bytes), with no extra RNG, actor mutation, world scan, or checkpoint. Hidden newborns remain unnamed and disguised newborns use their displayed identity. Permanent-death provenance and prevention bypass are unchanged; only the user-facing death sentence changes.


### Revision43 fixed Honey restoration and deliberate chasm cost

Royal Honey changes only a constant-size numeric branch after the existing
maximum-resource settlement. Revision43 adds9999 to health and energy then
clamps to the existing effective maxima; ≤42 retains the original full restore.
Its total time is still the inherited effective-stat/equipment/ability and
Half Here synchronization cost, not O(1) for the whole action; the added
calculation is O(1) time and memory, with no new RNG, lookup or persistent state.

`travel::jump_chasm` is an explicit player-command entrypoint. Its source tile
check and revision-selected packet scale are O(1), excluding the inherited
floor lookup. The unchanged travel pipeline includes floor generation/catchup,
actor and ally departure, safe landing selection and arrival hooks; this change
adds no search, snapshots or new event pass. Only the player's deliberate chasm
packet bypasses the old fall×0.25 scale at revision43. Deep-water immunity and
forced/equipment/potion/cohort fall packets are unchanged. The input command
route uses the existing optional command-move trace; its small scalar writes
are O(1), while the already-defined optional edge classification includes
floor/portal lookups O(F+P). No extra candidate Run clone is introduced.

Borge's one-handed melee weapon behavior already existed from revision39.
Revision43 reuses it unchanged: hit/accuracy and target protection remain,
landed hits permanently kill and consume the unique shard, misses do not.
The new regression adds no production work. Player-facing description edits
are in the desktop presentation layer, not a rewrite of saved Item.effect.


### Revision43 fixed Fury packet calibration

`crafting::fury_damage_per_craft` is O(1) per helper call, zero allocation/RNG. The existing preview/payment passes remain O(I+E) over inventory and equipped enhancements, with unchanged payment-entry storage O(E). One funded craft adds114 raw first-main-strike damage in43; ≤42 adds10. Ten-energy payment, funding order after Unyielding Assault, inactive/unfunded behavior, and one-attack/on-hit/RNG semantics are unchanged. The fixed constant is not a turn-time cohort computation and does not scale with floor depth.

Offline calibration:127 existing generated floor15-entry all-initial-heart/actual-Unblessed-reward profiles, standard starter Shortsword retained by the original census (no gear-loot simulation);1,992 naturally occurring ordinary physical target bodies from7species. O(P*T*B) analytic evaluations for P profiles/target occurrences and B tested fixed bonuses; no runtime profile retained. Exact uniform-variance/ceil/DR integration with hit probability yields56.821 expected normal packet and113.763 with114 Fury, versus113.642 for two ordinary attacks. This excludes HP-overkill clipping and special defense/ability/terrain effects. No claim of actual player average or universal2× damage: Greatsword sensitivity1.531×, Bastard sword1.921×; bow-base sensitivity without ranged hit modifiers3.043×. Native tests must verify actual packet arithmetic and unchanged action counts/energy/RNG. No timing claim follows from this offline numerical calculation.


### Revision43 concise inventory and hazard presentation

String substitutions retain their previous output-length cost and add no gameplay state writes, RNG calls, disk writes, or canonical-message changes. Honey/Borge/chasm descriptions with revision-dependent semantics keep <=42 wording and use the requested43 copy. The other changes are display-only.

Inventory damage previews reuse the exact core attack scaling/rounding and pure extractions of existing weapon/throw-base arithmetic. Let I be inventory items, C actor conditions, A actor abilities and E equipment enhancements; C_stats is the existing effective_stats cost, including owner resolution and inventory/condition/ability/equipment helper work. Each display_groups rebuild derives effective Offense ONCE, then shares its numeric value across all item descriptions. Individual describe_item callers derive it once for a weapon. Extra base arithmetic/variance-endpoint work is O(1) per weapon plus output text. Active funded Fury previews call the shared funding scan O(I+E) per applicable weapon, preserving Assault-before-Fury energy ordering; across W weapon groups this is O(W*(I+E)+C_stats+output text), not O(W) while hiding stat scans. Pack refresh still rebuilds groups as before; cosmetic hover does neither. No new Stamp fields, world clones, RNG, or hypothetical equipment mutation.

Unequipped damage explicitly uses the current loadout, not a predicted post-equip state. Ordinary displayed ranges exclude target defenses and ability bonuses. A separate funded Fury first-main-strike/shot/throw range includes the shared calibrated bonus but still excludes other abilities and target defenses. It is not a claim about unknown enemy HP loss. Offhand does not get a first-strike Fury preview. Unfunded/disabled enhancements add none.

The core helper lifts retain original floating arithmetic order and do not change actual attack or throw cost. Endpoints use ordinary_strike_damage variance0.75/1.25, zero targetDR and no surprise/paralysis modifiers. Tests compare throw arithmetic to the prior inline source, exact numeric output/zero/infinity, pure UI state/RNG retention, requested descriptions, legacy chasm/known-water guards, and refresh after free stat/Assault changes. Refresh always rebuilds groups; no new Stamp cache guard is required. Runtime tests remain lead-controlled; no timings claimed by this worker.

Ability description overrides are O(1) ID/revision selection plus existing string-copy/output cost. They operate only in desktop projection: canonical catalog descriptions remain unchanged because core offer tagging reads them. Blessing offers already use the same AbilityView projection as ability rows; no main-loop/renderer changes are needed. Inspection funding uses the shared pure O(I+E) forecast instead of duplicating raw10 arithmetic.


### Revision43 generated jewelry guarantee
Ordinary floor loot enters `generate_floor_loot_revision` → `create_equipment_revision`.
Only otherwise plain newly generated rings/amulets receive one existing
`modify_equipment` call after all old rolls. Ability-grant branches already
return a real catalog property; their output and RNG are untouched. Godbeast
permanent rewards already always add a nonzero stat, and do not need a new path.
Authored `make_equipment`, admin/inventory, saved items and load are unchanged.

For C equipment entries, A ability catalog entries, O owned ability IDs, and L
loot slots (bounded20), existing creation scans C and may scan A with O(O)
linear owned membership: O(C+A*O+item payload). The guarantee adds an allocation-
free scan of exactly8 stats and at most one existing modifier call: constant
catalog width, at most6 RNG draws, small fixed stat-choice vectors/name suffix.
No rerolls/retry loops, whole-Run scan, new per-turn work or persistence change.
Old≤42 take no additional RNG draws and retain byte-identical creation output.
Tests compare every29–42 revision with frozen pre-change algorithm and compare
already-successful42/43 creations including RNG; many seeds/depths cover empty
ability pools, zero/negative bonus semantics and Godbeast rewards. Actual43
floor generation/seed replay/load coverage is included. The focused native gate passed224 core unit tests and the actual generated-floor/save/replay integration test; final combined gates are recorded separately.
