> Current inventory correction: all R50 carried rings/amulets are active and counted. No Wear/Take off control is shown. See revision50-carried-inventory.md.

# Revision 50 item knowledge and copy

R50 item knowledge is a read-only presentation rule. Carried items and observed
or previously remembered ground items show canonical names and effect text
without drinking, throwing or equipping them first. Twenty ordinary potions and
special Verdania Sap use shared authored names and descriptions. Serialized
`Veridian Sap` remains the special item's identity. Diluted copies show the
existing half-efficacy numbers, with rounded-up durations and unchanged fixed
areas. No use-identification message, known-potion entry or identification flag
is created by R50 potion use. Legacy discovery behavior below R50 is unchanged.

The Pack row hover shows the item's description before selection. Jewelry shows
its canonical name, plain Ring or Amulet type, and a concise positive effect.
Amulets also show current energy, activation cost, and essential ranges, targeting
or durations. Carried-state labels, duplicate-copy rules, generic energy funding
explanations and exclusion paragraphs are omitted. Optional bonuses and Tinker
grants remain visible. R50 carried jewelry stays active automatically; there is
no Wear button. Existing dimensions, colors, target commands and dismissal
behavior are retained. This is a presentation-only cleanup; gameplay, save data
and historical equipment behavior are unchanged.

Ground inspection admits items through the existing current/remembered/detected
observation paths, then describes the admitted item snapshot. Knowing an item's
properties never reveals its live hidden location, altered contents, nearby
occupants or terrain. Map labels and new pickup/drop/equip/throw messages use the
same canonical R50 names without rewriting saved items or historical logs.

The descriptions match the implemented approved rules, including Reaping and
lane-wise Floods. Global R50 storage/replay admission is integrated. Description
coverage alone is not release certification; current native and platform results
are recorded in revision50-native-verification.md.

## Complexity

Let I be carried physical item entries, T the resulting text bytes, G display
groups, E identity/effect bytes and P previously admitted observed ground items.
Canonical name/description lookup has a fixed 21-potion/26-jewelry catalog and
costs O(E + T) time and O(T) output memory, with constant scratch. It does not
query actors, visibility, pathfinding, RNG or hidden live item records.

Pack rebuilding retains its existing O(I² + I*A + G log G + T) grouping and
ability-catalog work; the new name/copy work adds O(E + T) per item. A pointer
change scans up to G rows and copies the matched text, O(G + T), without a Run
clone or simulation mutation. Tooltip wrapping/rasterization retains the
existing font measurement and pixel costs; it is not claimed constant-time.
Ground observation retains all prior visibility/portal costs, then adds
O(sum(E + T)) for P admitted item snapshots. Action log naming adds O(E + T)
only after the pre-existing action admission. The old stored discovery lists
remain serialized for compatibility but are not scanned by R50 item knowledge.

These are source estimates, not measured timings. Focused tests cover every
potion, diluted values, raw save identity, repeated drink/throw, snapshot restore,
canonical jewelry labels, exact concise copy for all 26 jewelry powers and the
Transference alias, hovering before selection, repeated hover, cancellation,
reopening, scroll dismissal, item refresh, worn actions, visible ground items,
remembered/hidden isolation and legacy behavior. Test execution and visual QA
must be recorded separately; authoring tests is not an execution claim.

## Authored short jewelry copy

### Rings
- Stealth: Negative illumination on your tile is multiplied by 1.2.
- Light: Adds 10 illumination to surrounding tiles in your line of sight.
- Regeneration: Doubles your natural HP regeneration.
- Reaping: Restores 1 energy to each amulet for every 20 direct damage you deal to enemies.
- Vampirism: Melee attacks heal you for 5% of the damage dealt.
- Flight: Grants flight.
- Fire Resistance: Halves fire and burning damage.
- Poison Resistance: Halves poison and venom damage.
- Sustain Strength: Halves all temporary reductions to your stats.
- Willpower: Halves the duration of confusion, hallucination, and enemy curses.
- Protection: Grants 4 DR.
- Slaying: Grants +4 Damage.
- Ice Immunity: Prevents cold damage, cold-induced slowing, and slipping on ice.
- Corrosion Resistance: Halves acid, caustic gas, and ichor damage, and corrosion penalties to DR.

### Amulets
- Lightning: Fires a piercing bolt for 50 lightning damage.
- Firebolt: Fires a bolt for 80 fire damage, igniting creatures and flammable terrain.
- Poison: Fires a projectile that poisons its target for 6 damage per turn for 20 turns.
- Iceblast: Deals 30 cold damage and slows creatures for 3 turns in a 3×3 area, leaving ice.
- Acid: Fires a projectile that deals 40 damage and reduces the target’s DR by 10 for 6 turns.
- Blinking: Teleports you to a visible, accessible square within 10 tiles.

### Control amulets
- Beckoning: Pulls a targeted creature next to you.
- Discord: Makes a creature and all other creatures hostile to each other for 6 turns.
- Obstruction: Creates a three-tile wall perpendicular to your aim for 8 turns.
- Roots: Roots a creature in place for 4 turns, dealing 3 damage each turn.
- Teleportation: Teleports another creature to a random valid square on the current floor.
- Floods: Sends a 3×5 wave that deals 20 damage, pushes creatures up to 2 tiles, extinguishes fire, and leaves shallow water for 12 turns.

## Actual map-hover rendering

The observation report carries a detached, R50-only item-hover payload, including
stale-memory warnings when applicable. The shared map-hover painter renders it
using the existing tooltip geometry and font. Both retained and tile damage
trackers admit that payload, so opening, changing and dismissing it restore the
same background as a full repaint. An observed creature on the same square also
shows the admitted item's description; sensing a creature does not independently
admit hidden ground items. Empty/legacy reports keep the old no-item-hover path.
This adds O(T) detached text storage and font/wrapping plus clipped-pixel work,
with no new live-world reads from either painter or damage tracker.
