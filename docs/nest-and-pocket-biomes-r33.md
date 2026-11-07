# Revision 33: Saidribee Nest and localized pocket biomes

This implements the designer's Nest/Queen/Royal Honey, deeper monster habitats, and decorative flowers. It does **not** implement portals, otherworld travel, new traps, additional Godbeasts, pollination, or an ecology simulation. The existing god tree remains Verdania.

## Nest

Exactly one seed-selected floor from **28, 29, 31, 32, 33, 34** receives the Nest, preferring a non-Archipelago candidate when the window has one. Godbeast floors are never selected. The generator chooses three existing dry rooms joined by traversable paths; it does not carve walls, close passages, or move stairs. In watery domains, at most 96 existing water/shallow-water link cells become ordinary floor bridges using the existing bridge presentation. In a rare all-water window (for example seed155), a compact part of each of three existing room footprints is raised with its links: at most160 converted water cells total, without changing the floor domain. Ordinary plans keep chamber occupants and Honey on original dry land; fallback plans place them on the raised dry platforms. These platforms and links only add ground connectivity; original landmark/topology quality checks remain in force. Safe room cells and links become an amber-toned, connected Nest. The first room is the Queen chamber. Royal Honey is placed on a distinct safe cell in that chamber, reachable from the entrance. Six inhabitants are distributed across the network: four existing Short-legged Saidribees and two existing Saidribee Guardians. These conditional inhabitants are explicit exceptions to ordinary depth windows; no new worker species or ordinary-roster Queen entry is added.

Nest cells reserve space against ordinary monsters, loot, machine placement, and Sphinx room selection. The Queen and reward are populated once as part of the normal atomic floor-generation transaction. Initial Nest residents add seven creatures and **13 heart blessings** above the ordinary population budget (Queen3 + four workers×1 + two Guardians×3). This is a revision33 conditional-event reference shift; revision32 reports and budget invariants remain frozen and unchanged.

## Queen

The **Saidribee Queen** is a first-class, Nest-only creature with its own catalog identity, manual entry, heart and artwork. Approved provisional stats: 430 health, 100 energy, offense 230, defense 48, accuracy 145, dodge 27, DR 0, perception 55. It uses existing dedicated-caster movement, not a new rooted or special guard AI.

Each successful summon costs **15 energy**, creates **one** existing Saidribee, and sets an absolute **10-world-turn** cooldown. Haste opportunities do not shorten this wait. Worker/Guardian probabilities are **2:1**. Blocked placement, insufficient energy, silence, paralysis, and an unelapsed cooldown produce no creature and no successful-cast timer. Uncast uses existing spell cancellation semantics. Children inherit heartlessness and allegiance. Existing species stats are not silently scaled. The Queen heart follows the existing Guardian pattern: 3 blessings, +1 DR, +3 Defense, 30 energy.

## Royal Honey

Exact display name: **Royal Honey**. Food glyph: `%`; inventory action: **Eat**. It fully restores current health and energy to their current effective maxima, then is consumed once. It is also usable by its ID from directly underfoot. It grants no other bonus, buff, ability, blessing, or revival. Full resources do not prevent consumption. Normal blocked-action rules apply before use; time-travel restores the item and resources as ordinary serialized state, while retaining the existing time-travel costs.

## Other pockets

Non-Godbeast floors from depth 6 can contain flower-rich groves. Floors 17–36 can also contain permafrost. Each applicable theme's independent probability is `min(0.80, 0.10 + depth * 0.018)`. Patches use one existing room before depth 24 and two linked rooms thereafter; the Nest plus two other themes is the maximum of three biomes. Permafrost reuses the existing snow layer and rendering. Flower groves use existing grass over safe ordinary ground; new fuel is excluded from vent/fire/furnace gas components and the existing final fire/vent safety pass still runs afterward. Flowers themselves are immutable decorative metadata, rendered as colored `*` only where observed or remembered terrain remains grass; no new terrain or fire rules are introduced. Nest and grove ground colors preserve explicit appearance customization.

Where matching creatures already exist, up to three are relocated to their appropriate pocket without changing the ordinary population/reward budget: Wendigo at 17–26, Yeti at 27–36; existing Dryads, Bears, Tigers and Saidribees in flower groves. No unmatched monster is added to make a decorative pocket populated. Existing special/machine creatures and complete authored hordes are preserved (see implementation tests).

Biome/flower information is shown only for visible or remembered terrain; unseen terrain gains no new information. Decorative metadata is immutable across ground-cover erasure, but a visible grass-to-floor change hides flowers immediately. Unseen remembered grass retains its last-observed flowers until revisited. Seeds use a separate explicit generation stream; no runtime or presentation RNG is consumed by biome planning.

## Verification

`tests/nest_pockets_v33.rs` covers seeded network/reward reachability, depth exclusions, deterministic generation, theme occurrence, Queen cost/cooldown/blocked casts/heartlessness, Honey inventory and underfoot consumption, full resources, blocked actions, no revival, save/load, Revert, See the Future, and replay. Desktop `pocket_biome_observation_tests` checks amber/flowers/snow rendering and fog filtering. These are targeted regressions, not a claim of exhaustive gameplay balance or all-platform verification.
