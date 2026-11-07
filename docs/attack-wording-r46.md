# Evidence-backed Chronicle attack wording

Revision46+ changes only newly recorded attack sentences. Older revisions keep
their exact wording for differential replay tests. Attack rolls, damage, armor,
resource spending, effects, death, movement, projectile paths, and RNG are unchanged.
Existing saved messages are not retrospectively reclassified.

## Sources and mapping

Sources are the canonical equipment-v48 catalog, authored descriptions and natural
history in the desktop manual catalog, and current projectile implementation.

- Shortsword, Bastard sword, Greatsword and Hatchet melee strikes: slash. These
  authored blades support cutting; recognized magic/cursed suffixes retain identity.
  Each hand uses its actual item. Unarmed, Shard and unknown/custom weapons: strike.
- Giant Rat, Bear, Dire Bear, Tiger, Sabretooth Tiger, Pale Biter/Greater Biter,
  Pale Jackal and Pale Screamer/Greater Screamer retain their established bite action.
- Ichor piranha/fish and sharks: bite, matching the authored predator/teeth descriptions.
- Night Crawler: skewer, quoted directly in its authored natural history.
- Needleling: pierce, matching its large needle nose.
- Short/Long Saidribee and Guardian: sting, matching bee anatomy/stingers and venom.
- Bicorn, Quadricorn and Hexacorn: gore, explicitly stated in natural history.
- Cancer: crush, matching Crushing Claws and its explicit crushing description.
- Killspawn: slash, matching its body made of blades.
- Ursa Major: claw, matching the explicit claw description.
- Ranged spit, River Drake water jet, Valta moonbeam and odd-horn magical bolt retain
  their actual projectile type, rather than being reported as generic melee strikes.
  Player bows shoot at; thrown objects throw at. Ranged identity requires the live
  matching ability, so an erased power cannot silently supply an action label.

Unknown or underspecified creature styles retain neutral strike. No new attacks
are granted to Maulchild, fungi, Samara or any other nonattacking creature.

## Designer choices still open

The examples maul, pummel, bludgeon, scratch and erase do not specify particular
actors or weapons. No weapon in the current canonical equipment catalog is a
club/maul. Bear/Tiger retain their prior biting action until a different preferred
attack is chosen. Troll/Yeti/humanoid methods and Zillennia erasure wording remain
neutral rather than inventing a specific anatomy, impact style or terminal erasure
mechanic. Additional styles can be assigned once the designer chooses their users.

## Grammar and verification

Player verbs use the base form; other actors use third person. Misses say
"tries to skewer ... but misses" rather than claiming a successful skewering.
Zero damage remains a landed hit. Weapon/projectile wording is carried through
miss deflection, which otherwise resets range for the existing combat rule.
Common-creature articles recognize the expanded vocabulary and new species names.

Deterministic tests compare revision45 and46 attacks after excluding only log text,
log provenance/archive and the revision number. All remaining Run fields and emitted
presentation events must be identical. Coverage includes both hands, implicit
main-hand attacks, hit/miss grammar, player naming, projectiles, zero-damage hits,
nonattacking creatures, explicit identity mapping, unknown identities, and articles.
