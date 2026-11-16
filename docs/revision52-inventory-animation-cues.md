> Appearance override: the latest designer instruction replaces the historical decorative silhouettes and scene crossfades below with tile illumination or moving canonical glyphs. Trigger/geometry/privacy coverage remains applicable; old style-specific raster receipts do not certify the new look. See [Brogue-style contract](revision52-brogue-animation-style.md).

# Revision 52 inventory outcome cues

These cues report committed inventory changes. They do not change payment, action cadence, item retention, random selection or item capacities. They are detached presentation events, not saved simulation state.

- P14 Tools of the Trade: one violet item-use flick only when a successful heart/potion use consumes the item or leaves its actual diluted remainder and Tools was owned before use. Throws, missing items and unsuccessful uses do not receive it.
- P18 Spellslinger: one inventory spark only after the activation plan commits its first action-free amulet activation in that normal turn. It does not imply free energy, duplicate the spell or appear on later paid actions.
- P65 Tinker: orange gear strokes followed by the actual granted god's spark. No decorative random reroll. Rejected, unsupported, unpaid and already-tinkered attempts emit no success cue.
- P66 Bottomless Pack: split into a half outline only when the potion is actually retained diluted, then fade the remainder when that diluted potion is actually consumed. No map aura for carrying capacity.
- Amulet payments: reserve drain only for a positive debit to that specific amulet; player energy drain only for a positive debit to the player. A zero-cost activation has neither.
- Reaping: only amulets whose reserve actually increases receive upward glints. Full or missing-state/full reserves remain quiet. These inventory events contain no victim coordinates or connecting trail.
- E74 Borge: only successful altar rewards glint on the actual new shard, alongside the observed altar response. Repeated or failed interactions stay quiet.
- E82 Verdania sap: the actual acquired item receives a droplet and the consumed tap fractures. Full, distant or repeated attempts produce no inventory success cue; acquiring sap does not falsely imply healing.

Inventory row effects use the visible row when available; consumed or offscreen rows use the Pack button. Player-energy events use the energy display. Their rendering and ordering are owned by the central action-animation renderer. Actual potion/heart HP increases alone produce healing map marks in R52; energy-only gains use the energy display and full-health consumption does not falsely suggest healing. Historical event behavior remains unchanged through R51.

## Tests

Eleven focused core tests in `animation_inventory_tests.rs` exercise free consumption versus throws/missing items, retained-then-consumed potion identity, reserve/player split payment and stale-plan rejection, first-free versus later activation and zero cost, full/partial reserve Reaping with a hidden victim, actual Tinker result and failed repeat, HP-gain admission, actual Borge/Verdania reward and failure paths, and R51 silence/RNG preservation. All eleven passed in the 48-test combined animation core batch (`verification/r52-animations/core-focused-5.log`), and the subsequent 55-test batch also passed. Subsequent focused batches reached73 core and23 native passes. Selected actual retained/consumed potion, open/closed Pack and resource-HUD frames have been visually inspected, including three viewport sizes and Normal/Reduced/Instant behavior. These focused receipts do not certify the complete final workspace or every one of the170 proposal scenarios.

## Complexity

Let I be carried items and B the player's ability IDs. Tools captures one existing ability scan O(B) and verifies committed item change with O(I) search only on eligible consumption. Existing amulet planning remains O(I+B); cue emission adds constant bounded output per payment and no extra inventory search. Reaping reuses its existing O(I) traversal and collects at most I changed IDs in O(I) scratch, only allocating for actual R52 gains. Tinker adds constant-size style construction after existing successful random selection. Inventory event admission and metadata append are O(1) until the central event cap requires prefix eviction; the renderer's queue bound and drawing costs are documented separately. No new visibility query, path search, simulation RNG or persistent field is introduced by these hooks.
