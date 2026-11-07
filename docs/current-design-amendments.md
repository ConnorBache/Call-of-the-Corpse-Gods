# Current native design amendments

## 2026-11-07 — sight, awareness and Perception hearts (simulation revision 51)

New players start at 0 Perception. Ordinary tile range is
`20 + illumination/5 + Perception/5`; light at least +1 removes the distance
limit, not optical blockers. Creatures use twice their functional distance,
including additive Stealth. Eligible creatures are visible to the player;
wandering enemies acquire them at 25% per normal turn, sleeping enemies at 10%
only within 10 tiles. Hunters retain the last known location until reaching it
without a plausible target. The designer confirmed literal formula behavior
at -100 light and approved Cave Bat +1, Tiger +2, Sabretooth Tiger +3 and Dragon
+4 Perception hearts. Existing rewards and pre-51 rules remain intact. See the
[exact sight and awareness contract](revision51-perception.md) and
[knowledge-safe hover contour](revision51-perception-ui.md).

## 2026-11-07 — confirmed ability restorations (simulation revision 51)

I Came For You has no turn limit and retains its restored 10-energy activation.
Sparkle has no combustible-terrain targeting limit; its fire/light immediately
vanish on water contact. All-Seeing Eyes lasts until the next time-consuming
action, preserving bonus free moves. Active Shine suppresses its entire emitted
source. Retained revisions keep their old rules. See [execution, compatibility
and the pending Ichor-spread interpretation](revision51-ability-restorations.md).

## 2026-11-07 — player fixed attack base reduced 30% (simulation revision 51)

Reduce only the fixed base component of player melee, bow and one-handed weapon
throws to 70%. Melee's 45 becomes 31.5; Offense coefficients and enchantments
stay unchanged. Independent nonweapon/two-handed throws have no fixed component.
Shared bodies use their owner's revised melee baseline, new Mini-Me copies
snapshot it, and Clay Soldiers use the current live baseline. Existing stored
copy baselines are not rewritten. Enemies and independent ability packets are
not scaled. See [the exact formulas and copy rules](revision51-player-base-damage.md).

## 2026-11-07 — nearest-whole damage (simulation revision 51)

All ordinary damage packets round to the nearest whole number after mitigation,
scaling and resistance, with nonnegative halves rounded upward. Healing and
non-damage calculations are unchanged. Prior-revision saves/replays preserve
their rules. See [the damage rounding contract](revision51-damage-rounding.md)
for status/AoE/preview coverage and lethal-health accounting.


## 2026-11-07 — Immediate-benefit activation payment

Paid immediate-benefit toggles charge their first maintained turn on activation,
using the existing reserve-first payment. The next normal upkeep is prepaid, not
charged twice; free off/on and haste cannot reset coverage. The designer subsequently restored Dream-See and Mind Read to 1 energy/turn,
Bonded By Unseen Threads to 15 activation plus 8/turn (23 initially), and I Came
For You to 10 activation. Implant Thought stays at its R50 cost of 15. See [payment details](revision50-ability-energy.md).

## 2026-11-06 — Revision 50 carried jewelry and five pack sections

Latest designer correction supersedes shared jewelry capacity and manual wearing:
Pack has trinkets, amulets, rings, potions and hearts. Rings, amulets, potions and
hearts each start at five carried units; trinkets have no capacity limit. Existing
blessing capacity bonuses remain additive to their own category. Item.kind keeps
its action/quest identity separately from the carrying category.

All carried rings and amulets are automatically active and still count toward
their category. Old equipped flags neither disable powers nor exempt capacity.
There are no R50 Wear/Take off controls. Drop ends carried effects and is subject
to the existing non-destructive rule against removing needed carrying capacity.
Named duplicate powers remain nonstacking except additive Light. Moving items,
dropping and reacquiring never refills their energy or existing blessing reserves.
Below revision 50, the original equipment slots, capacity and commands remain.
See [inventory integration](revision50-carried-inventory.md) for paths and checks.


## 2026-10-26 — role-first monster rebalance (simulation revision 32)

The latest explicit designer instruction is the authority for this pass. New runs
use revision 32. Existing revision 29, 30 and 31 saves/replays retain their original
population, actor construction and summon behavior; saved actors are never retuned
when loaded.

Choose creature depths and roles before tuning. The reference is an awake,
full-health, 1v1 opponent against a full-health player who has eaten every finite,
naturally generated prior-floor heart once. The primary Unblessed reference includes
its actual seeded Offense/Defense level grants, starter Shortsword, no optional
loot or selected divine abilities. Heart-only alternatives are reported separately.

Targets are **total incoming HP damage over the creature's lifetime**, not damage
per hit or per turn: Fighters 10–50% of the reference player's starting maxHP,
increasing with depth and exploitable downsides; Skirmishers 60–80% of a comparable
Fighter; Mage/Artillery about half a comparable Fighter. Also report net HP loss,
actual expected attacks/turns to kill, death risk and incomplete/censored fights.
Controllers and Support preserve their authored effects, generally one dominant
function, with severity expressed through the depth at which those effects appear.
Do not invent replacement mechanics or normalize all species to identical ratios.

Pale Birthers and Greater Pale Birthers are the existing creature summoners. In
revision 32 a successful birth sets `monsterSummonReadyAt = current world tick + 10`.
Missing cooldown state means immediately eligible. A failed birth (no room or no
energy) does not spend a cooldown. Haste does not advance the world clock; offscreen
time and temporal restore use the same serialized timestamp. One cast may produce
twins under the existing energy/space rules. Stone Seal creates a wall, not a
creature, and is not subject to this creature-summoning cooldown.

Three casts should supply approximately one comparable Fighter's child value.
Reports distinguish immediate children from subsequent recursive births. The designer further specified that heartless creatures summon heartless minions.
Revision 32 children therefore inherit a heartless parent's heartless state, including
recursive descendants and births after parent revival. A heart-bearing parent's
children otherwise retain their existing reward behavior. Actual energy costs are
preserved: a 50-energy Birther can produce twins (30), one child (10), then one (10)
without resting. Existing recovery sleep can restore energy only after it loses
contact; a continuous combat sequence cannot cast forever. The cooldown limits
cast rate rather than replacing the energy budget. Global sleep regeneration is
unchanged.

The prior universal explosion rule, `ceil(target maxHP / 2)`, remains unchanged.
Bombardier burst is reported as a special rather than pretending it is an ordinary
half-Fighter damage packet. Godbeasts, invulnerable/conditional creatures, group
support, terrain, approach and recursive summon effects require separately labeled
scenarios; the reference duel is not an emergent-fight guarantee.

Numeric records and the repeatable analysis live in `physical-balance-v32.json`
and `tools/monster-balance/`; the final results document records measured coverage
and remaining limitations. The frozen reference design remains authoritative for
unrelated mechanics. Original art, alpha assets and authored lore are not rewritten.

### Measurement and fixed-birth clarifications

The final fitting anchor is the **earliest reachable natural or conditional
appearance**, not the center of a wide spawn band. Midpoint and last-floor rows
are sensitivity checks; older species are expected to become weaker as the player
grows. Fixed stats never rescale secretly with the floor. Primary damage stops at
the command containing the creature's death (or the last child for an isolated
three-cast brood). Already-applied poison/bleeding and persistent environmental
fields are separately reported. Standing in caustic gas until an entire room is
clear is a stationary stress case, not the primary lifetime target. A legal
post-kill escape case is also measured where space exists.

The designer authorized fixed, parent-specific offspring numerical profiles at
the existing birth-construction boundary. They preserve the actual child species,
abilities, rewards, energy and inherited heartlessness; natural floor-one Jackals
are not retuned by this separate brood table. Greater Birthers make normal Pale
roles, including normal Birthers, whose children are Jackals. This genealogy is
finite, not unbounded recursive branching.

Enemy-specific gas emission volumes may be tuned numerically to meet the caster
budget. Values are stored on newly created actors and default to the old constants
when absent. Global gas damage, diffusion, resistance, terrain behavior, spell
energy costs and abilities are unchanged. Old revisions and loaded actors are
never silently assigned these parameters.

### Designer clarification: guidelines, not precise constraints

The designer explicitly clarified that the percentages are guidelines. Preserve
species identity and fix clear outliers instead of chasing exact means. Blorb
therefore retains its original 10 HP / Offense 2 negligible trash identity, despite
falling below the Fighter guide; the opening Rat and Hemogoblin retain their
already modest values. Breather cloud volumes remain authored, and their area
control is reported as a hazard, not forced below a numerical controller cap.
The Greater Birther brood may remain roughly 45% rather than an exact 38% target.
Spitters were an actual outlier even before death; their existing gas-shot volumes
are reduced from 100/200 to 25/50, while Lesser Spitter retains its fragile 34 HP body.
No further exact-fit pass is intended unless held-out validation reveals a real
failure or clearly excessive result.

## Revision 33 — creature hunger and blood food (designer instruction 8)

Add Bloody walls. NPC creatures begin Hungering after 100 normal world turns
and seek reachable food while otherwise wandering. They remain still for ten
normal turns while Eating from an adjacent Bloody wall or Drinking from blood
water. Completed feeding restarts the 100-turn interval; damage, threats or
interruption reset a partial meal. Combat/fleeing and ally orders take priority.
No autonomous player hunger, starvation damage, stat growth, healing, energy
refill or food depletion is implied. Existing actual river-landmark wet cells
are persistent mild blood sources in r33; lakes and ordinary water are not
blanket seeded. Bloody walls remain solid walls. Full implemented defaults and
verification: [native-creature-feeding-r33.md](native-creature-feeding-r33.md).

## Revision 33 — deeper Zillennia clear loose ground contents (instruction 14)

The latest designer clarification limits ground erasure to things on the floor,
including hearts and grass, **not corpses and not water**. It does not authorize
living-creature deletion or damage, structural destruction, or an area aura.
Implemented default: existing Greater-and-above qualities (authored depth-quality
21+) clear only their occupied tile on arrival and initial placement. Lower forms
and revisions29–32 remain unchanged. Ordinary land-cover allowlist, preservation
rules and verification are in [native-zillennia-ground-r33.md](native-zillennia-ground-r33.md).

## 2026-10-26 — Borge and fixed non-final Godbeast milestones (revision33)

The designer confirmed seven non-final Godbeast encounters at floors 5, 10, 15,
20, 25, 30 and 35, followed by the Administrator on floor40. Only four designs
are currently defined: Mercuria, Zillennia, the existing God-Tree **Verdania**
(legacy internal `Veridian` ID), and **the Borge**. Reuse these four candidates;
do not invent the other three designs. Do not add a Godbeast on floor40 or a
second God-Tree. Revisions29–32 keep their original encounter scheduling.

The Borge is a huge central multi-cell dark obelisk. Its first attack is an
interaction saying exactly “The Borge demands a sacrifice...”, selecting the
biggest/toughest eligible living heart-bearing creature already on that floor.
Only that creature's exact heart transforms at the adjacent altar into one
**Shard of the Borge**. Offering consumes the heart without eating its stats.
The one-use weapon instantly kills any valid adjacent creature, disintegrates,
and prevents revival, including Phoenix. No accuracy roll or invented boss
immunity applies. Invalid use does not consume it. Exact description and the
implemented interaction/provenance contract are in [Borge event](borge-event-r33.md).

### Revision 33 — Nest, Royal Honey, and localized habitats

Designer-authorized additions are implemented as localized connected biome regions, not portals or otherworld travel. One Saidribee Nest appears on a seeded non-Godbeast floor28–34 (excluding30), with three linked existing rooms, a Queen chamber and reachable **Royal Honey**. The Queen summons existing workers/Guardians with paid energy and the established10-world-turn successful-cast cooldown; heartlessness is inherited. Royal Honey is eaten once to fill current health and energy to effective maxima, with no other buff or revival. Deeper permafrost uses existing snow, flower-rich pockets are decorative, and appropriate existing ordinary hordes can be relocated without splitting their composition. Details, provisional numbers, boundaries and tests: [Nest and pocket biomes](nest-and-pocket-biomes-r33.md). The god tree remains Verdania; no new progenitor, portal, ecology, trap or additional Godbeast is implied.

## October 2026 — three remaining Godbeasts (native revision 34)

Designer-authorized Phantasmii, Epidite and Ahvamitt are specified and implemented
as recorded in [release-r34.md](release-r34.md). Latest clarifications override
older assumptions: Nightmares drop nothing; all three stacked shards are on the
Phantasmii floor; sleeping-body damage only notifies (does not wake); active powers
follow the projection; Half Here remains on its creation plane; Blessed is a
binary heart property, not repeated doubling; Ahvamitt disappears after giving
his single Mask. Seven Godbeast floors occur every five levels through 35, with
the Administrator on 40. Numerical Nightmare baselines are provisional ordinary
body snapshots, not newly authored inherited monster powers.

## 2026-10-28: same-floor linked portals (revision 35)

Portals begin at floor 10, occur in linked pairs, and must always be optional to walk through. Both ends are on the same floor. The glyph is italic ASCII `0`; pair colors are blue, orange, red, yellow, green, indigo and violet. Linked tiles share occupancy: an enemy at the far endpoint is attacked when entering the near one. Sight projects through the connection and thrown objects/ranged attacks can traverse it. See `portals-r35.md` for explicit initial density and bounded geometry choices. No environmental propagation extension is implied.

## 2026-11-01: Weave of Time replaces Revert for new revision41 rules

The designer replaces Revert with passive **Weave of Time**. Exact description:
“You can freely travel along your time weave, which you leave behind as you move.”
Moving A→B leaves a teal weave at A pointing toward B; the designer explicitly
confirmed this direction. Actual voluntary movement B→A is free (no action/world
turn), but attacks are not. Following an existing weave creates no new weave.
A new weave placed on an occupied source cell destroys that old weave and all
chronologically older segments, retaining its newer continuation. Wrong-side
entry alone is not a new placement or collision.

Revision41 removes the completed-turn and pending-haste Revert history path and
its checkpoint export. Explicit See The Future anchors and ordinary transactional
rollback remain. Revisions29–40 retain their authored behavior for old saves and
replays, rather than reinterpreting their recorded commands.

Forced displacement, stairs and teleport/portal trail rules remain awaiting the
designer's clarification; no rule about deleting existing trails on those
transitions is inferred. See [timeweave-revision41.md](timeweave-revision41.md) for
implemented coverage, verification status and the current release limitations.


## 2026-11-06: Revision 50 carried inventory and visual follow-up

The latest designer instruction replaces R50's earlier shared jewelry capacity
and manual wearing model. Pack has five sections: trinkets, amulets, rings,
potions and hearts. Rings, amulets, potions and hearts each start at five units;
trinkets are unlimited. Existing blessing-specific capacity bonuses remain.
All rings and amulets in the bag are automatically active and counted. There is
no R50 Wear/Take off action or equipped-item capacity exemption. See
[carried inventory](revision50-carried-inventory.md) for acquisition, loss,
energy, save and temporal boundaries. Earlier revisions retain their rules.

Corpse X marks and authored blood-colored floor dots from the retained UI draft
remain enabled and are verified in native captures. The shared modal dialog
scale is reduced from 0.88 to 0.84; gameplay, Chronicle and the separately bounded
Pack layout keep their scale. See [visual follow-up](revision50-visual-follow-up.md)
for scope, interaction and pixel checks. Current execution evidence is recorded
in [native verification](revision50-native-verification.md).

## 2026-11-07 — Fungus feasts and forward roots (simulation revision 51)

A generated Corpse Fungus begins with exactly two heartless corpse meals: one
eligible on the current floor and one whose normal first eligible floor is two
or three floors deeper. No Godbeasts or event bosses are used. When either valid
victim pool or two rootable placement cells are unavailable, omit that fungus
and both victims, retaining room geometry. The current roster therefore produces
no setup fungi on floors 37–40; do not invent deeper species or substitute a
same-depth meal. Dead ordinary paired, aquatic species and Banshees remain eligible;
paired followers use their leader's actual authored depth bounds. A dead
Inventor or Mech needs no living companion and qualifies as a deeper meal on
floors 11–12; a dead Kelpie supplies floor 13's +3 meal. Released aquatic
survivors keep existing stranded behavior. Authored setup 24 domain/depth
metadata is unchanged, with effective revision-51 fungus population limited to
floors 1–36 by deeper-meal eligibility. Existing old saves are not repopulated.

Active fungal roots prevent corpse revival on active and inactive floors.
Corpse Fungus itself never revives, even after its roots are gone. Root glyphs
follow each outgoing path segment; the terminal cell continues the last segment.
Hidden cells keep their last observed direction until seen again. Earlier
simulation revisions retain historical generation and observation behavior.
See [fungus mechanics and verification](revision51-fungus-feasts.md).


## 2026-11-07: Quiet HUD and persistent Bestiary

The designer asks for observed monster profiles to stay on the left and for
hovering either a monster or its profile to highlight both. Bottom controls are
only Pack, Bestiary and Menu, evenly spaced. Remove unsolicited instructional
text, sensed-distance lines, condition explanations and redundant jewelry
carried/duplicate/exclusion/cooldown prose. Preserve positive effects, essential
costs, durations and actionable errors. Divine metadata is right-aligned using
available space; reserves are shown only for powers that can spend energy.

Keep ordinary terrain and light under the player, excluding its own light ring.
Corpse crosses must be small, no more than twice heart-ring size. Vampire uses V.
Native automatic resolution uses actual occupied drawable pixels, including DPI,
without multiplying pixel density twice; explicit manual preferences remain.

Bestiary records seen and unread species across saves in a local profile-side
file. Unknown pages show the book background and question mark. A newly seen
species adds purple “Bestiary Pages Added!” to Chronicle; unread entries show a
yellow exclamation. Only visible apparent identities count, not hidden/sensed or
hallucinated true species. Persistence failures must not silently overwrite data.

Show Damage, HP, DR, Accuracy and Dodge under resources. Damage replaces internal
Offense in nominal player melee-bonus units; HP replaces Defense at six HP per
point, separately from total HP. Internal fields remain compatible. Item/creature
contexts use their actual coefficients, and revision51 damage display uses whole
numbers. See the stat-display and damage-rounding contracts.

Hallucination identity changes occur every 800 ms instead of 400 ms. Water tilde
glyphs ripple geometrically; reduced motion and zero animation speed stay static.

## 2026-11-07 — First-four-floor primary loot (simulation revision 51)

The first four floors contain two potion-primary floors, one ring-primary floor
and one amulet-primary floor in seeded shuffled order. Each floor's ordinary drops
are exactly three primary items plus one or two of each other category, replacing
its old ordinary slots. Subtype repeats remain allowed, with distinct item IDs.
Separate arrows, carried loot and special/quest rewards remain separate. Floors
5+ resume normal allocation without repaying the early increase, and still share
the single-Composite-bow dungeon budget. See [primary-loot scope and checks](revision51-primary-loot.md).

## 2026-11-07 — Direct fungal-root lifecycle and authored color (revision 51)

Roots retain one shortest direct line per target and use the existing authored
olive material, independent of hallucinogenic gas color. Remove orphaned material
from severed, duplicate or invalid endpoint links while preserving shared live
cells and saved severed-pair history. Do not reroute severed links or impose a
two-corpse acquisition cap; two remains the initial generated feast requirement.
Hidden remembered roots remain unchanged until observed again. See
[reproduction, behavior and verification](revision51-root-cleanup.md).
