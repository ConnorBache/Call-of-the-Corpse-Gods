# Revision 52: all-role floor progression

## Numeric selection and compatibility

The R52 birth catalog now contains **550 exact species/context/floor profiles**. Its SHA-256 is `15c968d8361c7198ead29c76f763997cdbdae77b40af616bf572a0dc4c5c725f`. All 150 previously verified Fighter rows retain the same values. This pass expands the floor-aware model to ordinary Skirmishers, Mages, supports/controllers, hybrids, natural unclassified creatures, summoner parents, offspring contexts and the fixed Saidribee Nest.

Only explicit new-birth boundaries apply profiles, after historical overlays. Existing creatures are not retuned on loading, travel, copying, revival, or temporal restoration. The gate is exact revision 52; earlier revisions, their generic constructors and legacy overlays remain unchanged. Traits, spells, rewards, cast costs, cooldowns and ordinary pack recipes remain intact. Every selected profile stat is integral and new Defense-derived capacity is `100 + 6*Defense`.

The original Fighter-only evidence remains in `revision52-fighter-balance.md`. The integrated 550-profile source passed 646 focused tests (624 core unit, 13 birth integration, 6 admission and 3 coefficient tests), with no failures and two ignored tests. The release owner records the final whole-workspace gate separately. The paired native scenario receipts are not campaign-survival certification.

## Reference and targets

The designer's example is interpreted as **percentage points**, not multiplicative growth: Fighter reference `20 + 2*floor(depth/5)` percent of starting player maxHP. The 20% opening anchor is the working example, not a guarantee for each seed or species.

| Reference | Floor 1 | Floor 10 | Floor 20 | Floor 30 | Floor 40 |
|---|---:|---:|---:|---:|---:|
| Fighter |20%|24%|28%|32%|36%|
| Skirmisher midpoint |14%|16.8%|19.6%|22.4%|25.2%|
| Ordinary Mage |10%|12%|14%|16%|18%|
| Three successful summon casts |20%|24%|28%|32%|36%|

These are role guidelines. A row at a floor without a natural appearance is not an invented encounter. The nearest authored row for the **same exact key** handles out-of-window births; it does not fall back between natural, Nest, or parent-specific child identities.

- Mandatory fixed Fighter groups are sequential, with no player reset between members. Ursa Minor and Major remain one Fighter unit.
- Mech measurements include its exact Inventor, native mount/destruction/release and carried potion payloads as one encounter.
- Hybrids retain distinct first-appearance reference threat, scaled by Fighter-reference growth, rather than receiving a universal hybrid percentage or a free second full damage budget. Water, pilot, venom, fire, slow, and flight behavior stays native.
- Controllers/supports are evaluated by their actual healing, buffs, control/status opportunities and gas exposure. They are not fitted to an invented HP-percent utility conversion.
- The Nest is deliberately harder. Its fixed Queen, four workers and two Guardians, plus actual descendants, are one sequential group without resets. Selection is around **two Fighter references**, 60–64% over its observed floor 28–34 window. Fresh-world group means were 61.8–74.2%, with 21 deaths in 768 trials and no censors. It is not seven full independent Fighter budgets.
- A Greater Birther's three-cast value includes its recursive Jackals. Those grandchildren are not separately granted another full Fighter budget. Native twins, genealogy, finite energy and the 10-turn successful-cast cooldown are unchanged.

## What changed

- Ordinary Skirmishers and offense-scaled Mages receive floor-specific stat growth while remaining comparatively fragile. Evasion is constrained where it produced disproportionate low-Accuracy tails; caster and Vampire growth does not depend on added Dodge.
- Valta's charged Moonbeam uses a saved local damage coefficient. Its HP, charge sequence, range and energy rules are retained. Banshee's extreme early evasiveness is reduced; its 30-turn shriek clock remains unchanged.
- Dryad retains 112 HP and heals 25–69 HP across its natural window, with the same range, energy and cooldown. Extra HP did not improve its support contribution in the paired native fixture, so it was not added.
- Bloodmancer's body restores opportunities to use its existing effects. Hemophilia remains victim-maxHP-scaled with the same duration and episode rules.
- Screamers recover useful opportunities without increasing their 5/10 Accuracy penalties or control timers. Maulchild remains literal zero direct damage; its selected body/Dodge curve avoids the rejected long miss-chain tail while retaining the 3-turn grab.
- Natural Night Crawler and Skin Walker remain explicitly unclassified in the role map. Their numeric evasiveness was reviewed separately; no new designer role is asserted.
- Ordinary Breather's authored gas is unchanged. Its existing early lethality and residual deaths are reported, not hidden inside a controller target. Greater Breather retains hallucination identity, gas quantities and timers; trapped bounded-arena aftermath remains a disclosed censor.
- Blorb remains negligible trash with minor caustic. Bombardier retains its half-victim-maxHP burst exception. Neutrals, event bosses, finale and other special exceptions are not silently normalized into ordinary roles.

## Persisted local coefficients

Optional `heal_amount` and `special_damage` are copied at birth into R52-only actor conditions. Runtime reads those saved numbers with the original constant as fallback; it does not consult current depth. The supported fixed-damage reads are Dryad healing, Valta Moonbeam, River Drake jet, direct breath impact and Banshee shriek. The selected catalog uses only the required local overrides. Global fire/gas physics, statuses, damage channels, costs and cooldowns do not change.

The new keys accept nonnegative finite integers only in R52. Zero is a real value. Clone, save, travel, temporal restore and permitted revival preserve them. The selected 550-row catalog contains **no non-null gas overrides**. An integration guard blocks new gas overrides until their revival semantics are deliberately addressed; historical inherited gas retains its previous behavior.

## Measurement and uncertainty

Players consume all eligible finite initial natural hearts before entering each next floor, through native Unblessed progression. The model includes initial living natural heart-bearing actors and eligible initial Zillennia; it excludes dream entities, initial corpses, conditional/dynamic summons, gated rewards and repeated farming. Equipment and optional chosen abilities are absent. Rewards/pack recipes are unchanged by this pass, so these joint player vectors remain applicable under that stated collection protocol.

Calibration uses worlds 0–191. Worlds 192–255 were used for adaptive guardrails and are labeled adaptive validation. Worlds 256–319 are the final post-selection cohort. Species with few source worlds, absent natural cases, repeated trial seeds, deaths and censors remain explicit. 128 trials do not imply 128 independent worlds.

Primary damage means authoritative gross lifetime damage, including bounded postdeath settlement. Net loss, healing, maximum packets, death and censor counts are separate. Class summaries use population-weighted species means/medians, not the median random fight. Support-first counterplay is not represented by a fixture that explicitly targets the Fighter first; its paired support contribution is conditional on that policy. Hallucination, disguise and invisibility information cost is not measured by true-state chase. Real-room terrain, routing and large mixed packs remain outside the bounded open-arena model.

Fresh ordinary-role validation completed 36,608 paired records across 143 cases with zero errors/censors and 18 after-change deaths. Fresh utility validation completed 22,080 records across 69 cases, but its different arm counts must not be pooled into an all-utility before/after death rate. Detailed per-case receipts retain the remaining sparse Horn cohorts, unchanged Breather deaths and trapped-gas censors.

## Implementation cost

Let R=550 catalog rows, A actors, K actor condition entries, and J serialized save bytes. One-time catalog parse is O(catalog bytes) time/space. Each explicit birth performs an O(R) exact-key/nearest-floor scan and O(1) stat/optional-coefficient writes, with no RNG consumption. Runtime coefficient reads are expected O(1), with no floor lookup. Added validation is O(K) per actor within the existing validation traversal; save-schema recursion remains O(J). Optional condition storage is O(1) per profiled actor. These are structural bounds, not a frame-time benchmark.

The audit's native dispatch, visibility, pathfinding, gas and actor simulation dominate scenario cost. Its sample counts are measurements of verification coverage, not a claim that O(R) birth lookup is the runtime bottleneck. Final merged birth tests exercise every selected profile through its real construction route, strict full-world parity, unchanged rewards, saved coefficients, travel and temporal behavior.
