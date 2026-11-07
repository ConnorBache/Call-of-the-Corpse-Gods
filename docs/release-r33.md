# Revision 33 — creatures, world hooks and DEBUG floor testing

This is a native preview release, not an all-40-floor balance certification.
New descents use simulation revision 33. Explicit revision 29–32 runs keep their
versioned rules, and existing actors are not silently retuned on load. Save,
replay, Revert and future-branch behavior are covered by the current native
verification gate; legacy v1 playtest replays remain supported.

The illustrated Bestiary now contains **82 pages and 84 embedded RGBA plates**.
All 79 original Natural History strings, 81 original art masters and 81 original
runtime plates are retained unchanged. Night Crawler, Skin Walker and Saidribee
Queen add separate art and pages; this release does not rewrite the old lore.

## Implemented feature checklist and explicit defaults

- [x] **Witch offspring and ordinary creatures.** Witches can summon one Night
  Crawler or Skin Walker, 50/50, for 10 Energy with a 10-world-turn successful-cast
  cooldown. Failed placement does not spend Energy or start that cooldown.
  Night Crawlers naturally occur at depths 24–34 and retain life drain. Skin
  Walkers occur at 26–36 and retain one stable shallow-creature disguise; a living
  observing player body within Chebyshev distance 2 reveals the true appearance.
  Map, inspection and gameplay-event names share the appearance boundary;
  projection never changes real stats or bypasses visibility. See
  [Witch and new-creature details](witch-creatures-v33.md).
- [x] **Accuracy/Dodge roles.** A revision-33-only overlay specializes 17 natural
  profiles and two Witch-born profiles. It does not change the global hit formula,
  HP, Offense, Defense, DR, heart rewards, spawn weights or abilities. Natural
  Night Crawler is Accuracy 144/Dodge 74; Skin Walker 148/92; Queen 145/27;
  Vampire's final Dodge is 92. Witch-born Night Crawler is 140/64 and Skin Walker
  144/78. The frozen r32 reference and actual 32-world r33 refresh are separately
  identified in [the measurements and reproduction guide](accuracy-dodge-r33.md).
- [x] **Blood in water.** Actual positive HP loss deposits dissolved blood only
  into water/shallow water without Ichor, capped at concentration 100. Each normal
  environmental turn transfers 10% to each eligible wet neighbor, retains 94%
  after spread, and removes values below 0.05. Wet Kelpie spawn sites and authored
  river cells maintain a mild 3.0 floor. Sources do not follow the creature;
  ordinary lakes are not automatically blood rivers. Clean/mild/heavy water and
  remembered concentrations have distinct observation-aware presentation.
- [x] **Hallucinations, screams and roots.** Greater Breathers emit hallucinogenic
  gas instead of caustic gas. Exposure refreshes a two-normal-turn status rather
  than stacking it; player perception changes, not true stats or AI allegiance.
  Screamer Disoriented lowers Accuracy by 5, or 10 for Greater Screamers, using
  the existing five-turn duration. Successful second-struggle root escape breaks
  the binding root and its affected fungal link, not unrelated connections.
  See [environment/status defaults](native-environment-hooks-r33.md).
- [x] **Bloody walls and feeding.** Living NPCs become Hungering after 100 normal
  turns; ten uninterrupted stationary feeding turns clear hunger. They eat from
  adjacent bloody walls or drink actual bloodied water. Combat, threats and
  player-follow/rally needs take precedence. There is no player hunger meter,
  starvation, healing, stat gain or food depletion. Bloody wall density is a
  deterministic approximately 1-in-12 eligible-wall default. See
  [feeding and observation rules](native-creature-feeding-r33.md).
- [x] **Saidribee Nest, Queen and Royal Honey.** Exactly one Nest uses a floor from
  28, 29, 31, 32, 33 or 34, preferring a non-Archipelago candidate. Three connected
  existing rooms hold the Queen, four Short-legged Saidribees, two Guardians and
  one Royal Honey. The Queen has HP 430, Energy 100, Offense 230, Defense 48,
  Accuracy 145, Dodge 27, DR 0 and Perception 55. Each successful summon costs
  15 Energy, waits ten world turns and creates a worker/Guardian at 2:1 odds.
  Nest residents add 13 initial blessings outside the ordinary population budget.
  Royal Honey restores current health and Energy to their effective maxima once;
  no other reward or revival is implied.
- [x] **Localized habitats and flowers.** Flower groves may appear from depth 6
  and permafrost at 17–36 on non-Godbeast floors. Applicable theme probability is
  `min(0.80, 0.10 + depth * 0.018)`; patches expand from one room to two linked
  rooms at depth 24. At most three biomes coexist, including the Nest. Up to three
  suitable existing residents relocate without increasing the ordinary budget.
  Flowers are decorative observed/remembered grass metadata, not a new ecology
  or fire rule. See [Nest and biome details](nest-and-pocket-biomes-r33.md).
- [x] **The Borge.** Godbeast encounters occur at 5/10/15/20/25/30/35, drawing from
  Mercuria, Zillennia, Verdania and Borge; the Administrator remains at 40.
  Borge is a solid 5×5 obelisk event, not a new actor. Its demand marks one eligible
  existing creature; only that creature's provenance-tagged heart fulfills the
  offering. The one-use, one-handed Shard kills a valid adjacent living target
  without an accuracy roll and prevents revival while retaining normal death
  hooks. It cannot be thrown. Time-travel restores branch state normally. See
  [eligibility, interaction and Shard behavior](borge-event-r33.md).
- [x] **Deeper Zillennia contact.** Greater quality and above clears loose ground
  items, including hearts, on its occupied cell after successful arrival or initial
  placement. Ordinary-ground grass/snow/slime/blood cover clears as documented.
  Corpses, carried items, living actors, water and Ichor remain. There is no aura,
  stationary erase pulse or new killing attack. See
  [the exact allowlist and compatibility gate](native-zillennia-ground-r33.md).

## Try a specific floor without changing a normal run

1. Choose **Play from x'th floor** on the title menu, then floor 1–40 and vessel.
2. Keep the random seed, use F2 to reroll, or enter an exact seed. F3/F4/F8 cycle
   the distinct gods; Ctrl+C copies the full seed and Ctrl+V pastes it in setup.
3. Choose **each actual earned level-up offer**. The game reconstructs prior-floor
   initial hearts in a deterministic floor/actor order; it does not auto-pick powers
   or pretend to reconstruct your actual past route.
4. In the separate loadout stash, Tab switches stash/Pack, Enter moves an item,
   E equips and Space enters. Keeping starter equipment is valid. All recovered
   ordinary loot remains available; the normal Pack still has twenty slots.
5. F4 reopens the DEBUG stash. F7 exports a separate `.debug.json` state. DEBUG
   never writes normal autosave/Continue or high scores.

Full health/Energy at entry and collection of all eligible initial prior-floor
hearts are explicit debug assumptions, not a simulated survival route. Target-floor
loot is untouched; optional quest prizes, hypothetical summons and unknown combat
consumption are not fabricated. [The full guide](debug-floor-testing-v33.md)
includes headless commands, temporal behavior and the 32-world level checkpoint.

## Verification, artifact provenance and limits

- [Current native verification](../NATIVE-VERIFICATION.md) and
  [the machine-readable receipt](../INTEGRATED-VERIFICATION.json) identify the
  accepted integrated test results and frozen code/build inputs. Final gate logs
  and hash manifests accompany the source; focused subsystem counts are not
  substitutes for the integrated result. The QA input manifests describe the
  tested working tree, including exploratory tools/data omitted from delivery;
  `SOURCE-MANIFEST.json` separately identifies the delivered subset and historical
  documentation additions. Every delivered gameplay/build input matches the
  accepted freeze, without claiming omitted exploratory files are in this ZIP.
- The Windows ZIP is cross-compiled and statically inspected, **not Windows-runtime
  verified**. No Windows launch, display, input, audio or save behavior is claimed.
- Accepted new RGBA art preserves authored pixels. Solid-looking regions can
  retain near-opaque alpha (rather than exactly 255), with faint low-alpha fringe;
  this is not a hard binary-alpha cleanup. Historical manual-mask/edge-review debt
  remains recorded in the alpha manifest. Original artwork was not silently keyed,
  thresholded or repainted. [New-art provenance](art-provenance-r33.md) records
  the selected prompts, generation method, final paths/hashes and approved blur.
- The accuracy matrices and fresh r33 heart census are controlled measurements,
  not a full survival or universal encounter fit. Queen/Witch surviving-brood
  aftermath censors and severe low-Accuracy/lifesteal tails remain explicit.
  Earlier r32 reports and historical r33 pre-tuning checkpoints remain **historical**;
  they are not current r33 balance claims. Provisional candidate outputs are omitted.
  The final name-projection correction postdates the full numerical matrices;
  focused state/combat equivalence receipts document that presentation-only delta.
  The matrices were not relabeled as a full rerun of the later binary.
- The existing creature-initial/glyph and ability-language audits remain read-only
  findings. They do not authorize or imply a global glyph or prose rewrite.
- No portals/otherworld travel, new traps, pollination, extra future Godbeasts,
  new Dryad native/outsider behavior or other deferred design work is added.

### Source-package continuity

The complete canonical r32 source archive is the historical baseline. Its four
late-completed documents/receipts that were absent from the early r33 working copy
are restored **only in this package**: `docs/monster-balance-v32.md`,
`tools/monster-balance/final/verification-receipt.json`,
`verification/monster-balance-r32-final.txt`, and the historical r31 manifest.
The canonical r32 manifest is also preserved. These historical-only additions are
not presented as tested r33 gameplay inputs. Current `SOURCE-MANIFEST.json` covers
the release contents; build outputs, toolchains, downloads, private coordination
logs and provisional fitting candidates are excluded.
