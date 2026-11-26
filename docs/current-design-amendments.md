## November 2026: gameplay animation style

Designer instruction: “All animation should follow this rule: Either we light up a tile, or we animate moving a glyph.” Activated abilities being used or proccing, distant effects and selected ambient features receive feedback. Implement within Brogue’s glyph-and-light vocabulary. This supersedes the R52 proposal’s decorative strokes, deforming glyphs and whole-scene crossfades, without changing its actual trigger, knowledge or geometry rules. Ordinary walking and ordinary melee remain quiet. See [implementation and complexity](revision52-brogue-animation-style.md).

# Current native design amendments

## 2026-11-07 — Speed-up uses adjacent Non-Euclidean pairs (revision 52 review)

Speed-up tiles use the same destination mapping and numbering as Non-Euclidean
space, with each pair next to each other. Existing generation already builds
successive adjacent pairs, and the UI already uses the same saved-link number
for both terrain types. Preserve their own teal terrain identity and the
existing movement, shared-occupancy, projectile and local-vision rules. This
clarification changes no runtime mapping or older save admission. See
[adjacent destination mapping and regression coverage](revision51-speed-up-tiles.md).

## 2026-11-07 — Darkness sets a radius-3 circle to -100 light (revision 52)

The designer instructed “Have darkness turn the darkened area into -100 light
immediately,” then “Darkness, instead of being a 3x3 square, should be a radius 3
circle.” R52 overrides effective illumination to exactly -100 in a plane-local
Euclidean circle: cell-center offsets satisfy dx² + dy² <= 9 (29 cells in an
unclipped room). The mask retains its wall-independent area and existing projectile
impact rules; it is clipped only by floor bounds. The mask lasts 20 normal world
ticks, or 10 when diluted, and overlapping doses retain the latest expiry without
stacking. Existing, moving and newly created lights remain suppressed inside the
active mask; expiry exposes their current values. Raw ambient/source powers are
not rewritten. The literal R51 Perception formula is unchanged, including sight
at -100 with sufficient Perception. R50/R51 saves/replays retain their decrement
rule. See [Darkness implementation and checks](revision52-darkness.md).

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


## 2026-11-07: Reachable targeting and Sparkle (revision52)

Select the reachable physical line. Iceblast detonates on the first creature or
last reachable square before terrain. Blink lands on the last legal empty
square, retaining its ten-tile range. Lightning remains piercing. Sparkle lasts
until extinguished; ordinary water quenches it, while Ichor-covered water does
not. Older revisions retain their rules. See revision52-targeting-sparkle.md.


## 2026-11-07: Paid Dragon flight and melee follow-ups (revision52)

Dragon takeoff costs 8 from its natural 50-energy pool, with at most six normal
turns of flight. Takeoff is a paid normal action; use it only for a strictly
faster normal-turn route with a planned safe landing. Blorb adds one exact-scale
caustic packet after positive melee damage. Each positive Killspawn melee hit
adds a rounded half-hit budget across the next three normal turns, with
independent additive wounds and ordinary cleansing. Prior revisions retain
their original behavior. See revision52-enemy-followups.md for protection,
action-timing, forced-interruption and compatibility details.


## 2026-11-07: Immutable amulet birth tiers (revision52)

Newly born amulets use Tier 1 on floors 1–4, then one higher tier per five-floor band through Tier 9 on floor 40. Names use “Tier X Amulet of Y”. Existing items retain their original tier and energy; missing tier means base Tier 1. Higher tiers increase independently fitted damage, or the approved utility range/duration, with unchanged costs and reserves. Earlier revisions retain their exact rules. See [amulet tier mechanics and numeric profiles](revision52-amulet-tiers.md) and [native presentation](revision52-amulet-tier-ui.md). Shot-count bands are approximate and refer to each individual creature, with periodic applications reported separately.

## 2026-11-07: Floods deepens shallow water (revision 52)

Floods is consumed by preexisting shallow water, turning that terminal tile into
deep water. Each of its three lanes still stops independently; existing deep
water still stops the lane. Converted deep water uses Floods' existing 12-normal-
turn lifetime, refreshing one temporary record while preserving the original
permanent terrain. Preview and animation include the consumed shallow cell.
Earlier saves/replays retain their exact stopping rules. See
[Floods behavior, restoration and checks](revision52-floods.md).

## 2026-11-07: R52 Weave of Time unrolls on entry

Successful entry onto a weave destroys that segment. Reverse entry along its
stored edge is free and consumes only the entered segment, allowing consecutive
backtracking. Wrong-angle entry consumes it and all chronologically older
segments immediately on arrival; newer segments remain. Attacks and failed
movement do not count as entry. The ordinary local-edge scope stays unchanged.
This supersedes the R41 departure-only collision rule for R52. See
[exact rule and complexity](revision52-weave-consumption.md).

## 2026-11-07: R52 ratio accuracy and birth baselines

Ordinary hit probability uses Accuracy divided by twice Dodge, with Dodge
min-capped at one within this calculation. Preserve the existing range loss and
explicit hit/miss overrides. New players start at actual Accuracy 15. New
non-player creatures start from a ten-point Dodge baseline plus their authored
species/floor offset; do not flatten their variation or repeatedly apply the
baseline to saved, copied or revived bodies. Earlier revisions retain their
formula and births. See [the exact accuracy contract](revision52-accuracy-formula.md)
for compatibility and verification. The early-creature Accuracy issue at this
checkpoint is resolved by the later 2026-11-08 baseline approval below.


## 2026-11-08: Player also starts with Dodge 10 (R52)

The designer confirmed the player also starts with actual Dodge 10. New normal
and DEBUG R52 players therefore begin with Accuracy 15 and Dodge 10; earned
progression is additive afterward. Keep existing saved statistics and earlier
simulation revisions unchanged. The separate creature Accuracy approval follows.


## 2026-11-08: Provisional creature Accuracy baseline of 15 (R52)

The designer approved, “For now”, that creatures also start with 15 Accuracy,
with existing species modifiers on top. New non-player bodies therefore use
Accuracy 15 plus their authored species/floor offset and Dodge 10 plus its
authored offset. Preserve copied, loaded, grown and revived statistics and the
special statless illusion behavior. This resolves the prior zero-hit design
blocker; it does not authorize independent species/floor retuning. New release
gates must certify this combined source before packaging.

## 2026-11-08: R52 player, species Dodge, Jackals and heart capacity

New R52 players start with Dodge 15. Innate fixed player melee damage rises by
20%, from 31.5 to 37.8, without scaling Offense/equipment bonuses or Bow/thrown
bases. Every new monster type uses its first-appearance Dodge on all floors,
keeping exact contextual species keys and deliberate modifiers. New Jackals are
heartless. Base heart carrying capacity is 10, retaining other capacity bonuses.
See [the stat follow-up](revision52-stat-followups.md) for save compatibility and
precise scope. The preceding Accuracy Baselines receipt is historical for these
later changes; its ratio formula remains unchanged.


## 2026-11-08 — ten-energy reserves and one reaction per Wait (revision 52)

Amulet and blessing reserves are capped at 10, including existing R52 reserves.
New acquisition and blessing level-up use 10; amulet Reaping recharge caps at 10.
Player energy, activation/upkeep costs and other recharge triggers are unchanged.
Wait restores one reaction up to the existing maximum, rather than all reactions.
Pre-R52 saves/replays retain their rules. Valid R52 save/load and temporal restore
boundaries clamp old excess without refilling spent reserves. See
[reserve and Wait mechanics, compatibility and complexity](revision52-reserves-wait.md).



## 2026-11-08 — Hanged replaces Missed Me (revision 52)

Hanged is a permanent passive with no extra reaction or toggle. Damage that
would reduce HP below 1, including excess from the same hit, uses main energy
at four energy per HP. Drain available energy, then apply uncovered damage to
HP without a new rounding rule. Health costs, equipment vitality changes and
unpreventable deaths retain their existing behavior. Revisions below 52 retain
Missed Me. See [the execution and save contract](revision52-hanged.md).

## 2026-11-08: Avatar-only new games, selected gods, and player symbol

New playable normal and DEBUG runs are Avatars. The main menu selects three
distinct gods using the existing ordered seed prefix. Each choice uses its
authored god color and the designer's exact name, alias and domain. The aliases
are: SHRAIL the Godthorn, God of Fury; PELIAN the King of Desire, God of Zeal;
ISHEILAH the Trickster, God of Guile; NUMO the Pregenitor, Goddess of Memory;
ETILL the Ocean-Black, Goddess of Flow; URSA the Sleeping Mother, Goddess of
Dreams; LICHOR the Gushing One, God of Blood; DAWEL the Mouthless, Goddess of
Silence; GRISHA the Devourer, God of Ruin; ANAVANI of the Moon, Goddess of
Ascendance; KATIVE the Craftsman, God of Artifice; MORAE the Sacrifice,
Goddess of Fate. Preserve “Pregenitor” exactly as authored.

This is a new-game interface policy, not a save migration. Continue, imported
saves and replay reconstruction retain their recorded modes, gods and revisions.
The complete seed, including god order, remains the dungeon identity; selecting
different gods can change geometry and loot. Historical constructors remain for
save/replay compatibility, not as user-selectable new-game modes.

The actual player is displayed as @ with no mode superscript, including the
player's tagged sleeping body during Dreamwalk. The player glyph is fixed even
when a stored appearance profile contains an older glyph override; profile data
is retained and player color/font/material controls remain available. Other
creatures, allies, copies and their authored tier superscripts are unchanged.

See [new-game boundaries](avatar-new-run-boundary-r52.md),
[menu and god selection](revision52-avatar-menu.md), and
[player glyph presentation](player-glyph-presentation.md).

## 2026-11-08: Monster condition text and meaningful energy display

Present monster conditions as comma-separated text with measured wrapping,
retaining applicable duration counts. Player condition meters are unchanged.
Show a monster's energy only if it has an executable energy-using capability;
a generic stored energy pool alone is not sufficient. Energy users retain their
bar while empty, asleep, silenced or otherwise temporarily unable to act.
Apply the same rule to relevant live entity/inspection surfaces and public
Bestiary templates without exposing hidden live state or changing simulation.

## 2026-11-08: Gradual corpse roots and gas-entry warnings

New R52 corpse-root links, including generated ones, grow along their existing
direct path one tile per three elapsed normal turns. Acquiring a target does not
instantly fill its path. Haste-only actions and animation time do not advance
this cadence. Only the physically grown prefix is visible and interactive; a
corpse becomes rooted when that prefix reaches it, and cannot revive while
rooted. Existing saved full links remain grown rather than being erased.
Roots remain recognizable as roots under hallucination. See
[root growth and compatibility](revision52-root-growth.md).

When the player currently occupies any gas, suppress only the warning about
stepping into gas, including a different gas type. Entering gas from clean air,
waiting warnings and independent hazards keep their existing behavior. Gas
effects, route preferences and visibility/knowledge boundaries are unchanged.
See [gas-entry warnings](gas-entry-warnings.md).

## 2026-11-08: Fated Duel target termination

In R52, Fated Duel ends when its selected target dies/is removed or leaves the
existing anchored area/floor. Killing an unrelated enemy does not end it. Area,
activation/upkeep costs and owner-exit behavior are unchanged. Returning or
reviving later does not reactivate an expired Duel. Historical revisions retain
their recorded mechanics; current state is settled before subsequent protection,
upkeep and persisted/displayed boundaries.

See [Fated Duel lifecycle and compatibility](revision52-fated-duel.md).

## 2026-11-08: One-floor lookahead and overlapping lake currents

New R52 runs generate the starting floor and one floor below it. Normal player
travel ensures the arrival floor plus one, capped at floor 40, rather than
speculatively generating a five-floor block. Keep already generated saved floors.
Explicit DEBUG reconstruction, requested generation sweeps and real offscreen
fall destinations may still require their actual target floors. Preserve the
existing seeded domain/Godbeast schedule rather than requiring those future
floors to exist early.

For each deep-water lake, cover its eligible cells with seeded, axis-aligned
squares of side two or greater. Each square has a clockwise or counterclockwise
rotation. The closest edge by Manhattan distance determines the clockwise
direction: bottom leftward, top rightward, right downward, left upward; reverse
for counterclockwise. Ties are seeded random choices. Equal directions agree;
perpendicular directions choose the source nearer its edge, with random ties.
Opposite directions with equal rotations cancel. Opposite directions with
opposite rotations advance to the specified turn sequence: clockwise up to left
to down to right, and the inverse for counterclockwise.

Coverage, square sampling bounds and multi-square folding are deterministic
implementation details documented in [square currents](revision52-square-currents.md).
R52 replaces the old drainage/bank-repair and authored setup-arrow overrides,
which would otherwise erase these circulations. Other authored setup features
remain. Existing saved current layers and historical revisions are not rewritten.
This current rule does not itself carve natural land paths or prove island
access; do not describe it as doing so.

These are amendments within R52, not a new simulation revision. Reconstructing
an older R52 seed replay can therefore differ when it generates amended floors
or depends on amended mechanics. Loading a recorded world preserves its saved
floor layers; future missing floors use the current R52 generator.

## 2026-11-08: Isolated-island Ring of Swimming

The latest designer rule narrows the guarantee: new R52 generation supplies a
Ring of Swimming only on an island that has no ordinary swimming route to
another meaningful dry-land component under actual current timing. Include the
starting landmass in this test; do not assume an exempt mainland. A single
connected shore around a lake is not a trapped island merely because no second
landmass exists. Shallow-water specks alone are not escape destinations.
The ring grants ordinary, nonstacking haste while its
owner is in shallow or deep water, not while flying above it. Ichor-covered
terrain retains its existing non-water semantics. Use the existing carried-ring
slots/capacity and other ring behavior; do not add this guarantee to the ordinary
random ring pool or silently remove another item.

Place the ring on a safe, obtainable dry cell on the island. Validate static
ordinary-movement/current return routes with haste, and entrance-to-downstairs
progress without assuming a ring is already owned. The latter includes actual
ring acquisition with normal capacity and available swaps, and also applies
when no safeguard ring is needed. Reject the generation candidate through
bounded retries when placement or the required routes are impossible.
This is a terrain/current safeguard, not immunity from
enemies, dynamic hazards or disabling statuses. Keep existing saved floors
unchanged. See [Swimming and island safeguards](revision52-swimming-islands.md)
for the exact topology, phase, placement and verification limits.


## 2026-11-08: Enclosed enemy sight preview

The designer requests a complete enclosed red area showing all of the player's
known space that the selected enemy can see. Outline the full confirmed-visible
known-cell mask, including map limits and internal occluders, rather than only
visible/nonvisible interfaces that leave gaps beside unknown cells. Keep actual
perception rules and detached-knowledge privacy. Remembered geometry is not a
license to read hidden current illumination or terrain. Where sight cannot be
established from player knowledge, close the known-area contour without claiming
that the enemy cannot see beyond it. See
[enemy perception overlay](revision52-enemy-perception-overlay.md).


## 2026-11-09: Chasm base color and ordinary illumination

The designer clarifies: “Chasms shouldn't be artificially dark in illumination.”
This supersedes the earlier requirement to keep chasms dark through lighting.
Keep their authored dark Brogue base palette in current sight and memory, but
apply the ordinary visible-terrain illumination transformation, including the
existing negative-light foreground contrast treatment. Do not apply hidden live
light to remembered terrain or change actual illumination, sight, knowledge,
passability or fall behavior. See [ordinary chasm illumination](revision52-normal-chasm-illumination.md)
for the exact presentation scope and native checks. The earlier
[dark chasm memory](revision52-dark-chasm-memory.md) remains historical evidence
for preserving the dark remembered base rather than forcing it to gray.


## 2026-11-08: Larger red corpse X and retained living color

The latest designer request supersedes the earlier smaller corpse X: draw a
larger red X over each corpse, while the corpse itself retains the color it had
in life. Keep the corpse's color and marker color independent, preserve stored
individual variation, and handle older saves without inventing unavailable
individual data. No death, decay, resurrection, visibility or loot rule changes
are authorized by this visual amendment. See
[corpse colors and cross](revision52-corpse-colors-cross.md).


## 2026-11-08: Automatic paths through shallow, not deep water

Player automatic pathfinding/travel may route through shallow water at normal
cost but must not plan through known genuine deep-water contact. The existing
ichor-covered-water dry-footing exception remains. This does not prohibit deliberate manual swimming
or change the island-safeguard escape model. Keep ordinary terrain knowledge and
other automatic-travel safety boundaries. See
[shallow-water pathing](revision52-shallow-water-pathing.md).


## 2026-11-08: Distinct wall and stale-flesh glyph colors

The designer requests different colors for the exposed-wall # and stale flesh
behind the walls. Distinguish their foreground colors using the existing known
terrain/material identities. Preserve illumination grading, remembered/fog
rules, unknown-area hiding and actual terrain behavior. See
[wall and stale-flesh colors](revision52-wall-flesh-colors.md).


## 2026-11-08: Visible wall-feeding state

Show Feeding as a specific visible creature state when the creature is actually
feeding by licking the walls. Retain other relevant states and comma-separated
condition presentation. Do not infer feeding from mere hunger or wall proximity,
and do not reveal hidden or disguised behavior. See
[feeding state](revision52-feeding-state.md) for the exact mechanical signal,
presentation surfaces and verification.


## 2026-11-08: Fast movement versus true Haste

The designer distinguishes innate Fast from actual Haste. Fast can move two tiles;
after the first move it may instead attack, allowing creatures such as Cave Bats
to catch a fleeing player and hit. Do not grant an extra attack when the first
normal action was already an attack or movement failed. True Haste grants two
complete actions, rather than a movement-only extra pass. Show innate Fast as
Fast rather than Hasted in creature presentation.

The implementation composition is Fast movement on each complete Haste action,
allowing up to four moves or two move-then-attacks. This is a documented natural
composition assumption, not a separate quoted designer stacking rule. Preserve
existing Slow/Slowed suppression and unrelated movement/AI constraints. See
[Fast versus Haste](revision52-fast-versus-haste.md) for the exact scheduling,
revision compatibility and interaction tests.

## 2026-11-09: Radius-four Floods proposal deferred from release (revision 52)

Designer clarification for the pending proposal: “A circle of radius 4.” The
later release instruction retains the previously released targeted Floods wave
until its direct-damage/immediate-push decision is answered. Native Apply emits
UseAt and follows the exact released 3-by-5 wave, tier damage, two-tile push,
quenching, shallow-to-deep lane consumption, cost and duration. No radial Use
activation or current-overlay schema is included. Independent completed changes
remain included. See the [release contract](revision52-released-floods.md) and
[preserved proposal](revision52-radial-floods.md).

## 2026-11-09: Loose corpses follow currents and lose flight

“Corpses (even those of flying enemies), items, etc. should be affected by
currents. Flying corpses aren't flying and fall.” R52 loose corpses follow the
existing legal water-current displacement once per normal environment turn.
Dead bodies have no physical flight; retain their innate traits for revival.
Existing ground-item currents, corpse/item placement and immediate chasm falls
remain the shared paths. Connected fungal roots, separate occupancy layers,
living-flyer immunity and dream-world boundaries remain intact. Already-unsupported
waking corpses also settle through shared placement at command/phase boundaries,
including after a Haste/free Black Pit move, without an extra world tick.
Structural fixtures are not loose objects. See [scope, exceptions, complexity and verification](revision52-corpse-currents.md).


## 2026-11-09: R52 minimum starting creature Dodge

New real non-player bodies start at `max(10, 10 + authored Dodge offset)`,
retaining exact contextual first-appearance keys and all higher starting values.
Players remain at their existing starting Dodge 15. This does not floor live
effective stats or retune loaded, copied, grown, revived or restored bodies;
statless hallucinations remain statless. The ratio denominator guard stays one.
See [birth paths, numerical impact and checks](revision52-birth-dodge-minimum.md).


## 2026-11-09 — Never Surrender waits for DR-reducible damage (revision 52)

Never Surrender does not override damage that ignores DR. Such damage must not
spend its 8 energy or consume its first-source activation. It waits for the first
otherwise-damaging source that uses DR, including ordinary ranged attacks.
A hit completely blocked by the added DR still pays; an ordinary zero-damage hit
does not. Legacy revisions preserve their existing behavior. Its exact current
description is: “Increase your DR for the first source of damage every turn.”
See [execution, preview and verification details](revision52-never-surrender-audit.md).


## 2026-11-09: Random gods on each main-menu return

Every return to the main menu selects three random distinct gods. These remain
editable and do not reroll while staying on the menu or closing its god picker.
An ordinary initial blank/suffix-seed visit also randomizes the choices. An
explicit full CLI seed remains exact on initial menu entry; subsequent returns
replace only its ordered god prefix and preserve the suffix. This is UI state,
not a change to saved/replayed gods, modes, revisions, or gameplay randomness.
See [menu lifecycle and verification](revision52-menu-god-randomization.md).


## Fast mode (November 9, 2026)

Add an opt-in main-menu toggle named Fast mode. Each physical descent advances two
effective floors for gameplay and generation. A scheduled five-floor Godbeast
skipped by that transition appears on the destination floor, for example floor 5
on a 4→6 descent. Creature stats stay at their ordinary destination-depth values.
Double only the blessing amount and stats awarded by each consumed heart; retain
ordinary rounding, energy recovery and all other rules. Do not add other
compensation. Existing saves and replays without the flag remain Normal.

Fast begins on floor 1 and visits 1, 3, …, 39, with a final one-step descent to
the existing floor 40 finale. DEBUG accepts the same reachable depths. See
[Fast mode](revision52-fast-mode.md) for implementation and verification status.

## 2026-11-15 — Composite bow uses normal accuracy (R52)

Composite bow shots use the ordinary effective Accuracy / (2 × max(Dodge, 1))
calculation, capped at 100%, with no distance penalty and no bow miss-halving
bonus. Regular Bow accuracy is unchanged. Shared effective-stat modifiers and
lucky/unlucky/distracted/paralyzed overrides still apply. This changes accuracy
only: Composite shots remain ranged for surprise admission, collision, effects,
ammunition and damage. Existing R52 saves receive this rule; pre-R52 runs retain
the old bow accuracy. Older R52 replays may therefore produce different outcomes.
Inventory text and target forecasts use this policy. Regression coverage is in
revision52_composite_accuracy and the desktop tests of the same name.

## 2026-11-15 — Both bows use normal accuracy and minimal description

The later instruction extends normal hit calculation to regular Bow as well as
Composite bow. For R52 both use effective Accuracy / (2 × max(Dodge, 1)), capped
at 100%, with no distance penalty or bow-specific miss reduction. Normal shared
hit overrides remain, along with ranged surprise semantics, existing damage,
ammunition and physical collision. Pre-R52 runs retain their old calculations.

For either bow the entire description is exactly: “Apply to fire an arrow at a
visible target. [Count] arrows available.” Replace [Count] with the current arrow
count; do not append equipment, damage, accuracy, enhancement or other prose.
Target hit forecasts remain available and match the updated calculation.

## Twenty-floor default dungeon (2026-11-16)

New Avatar games use a single twenty-floor route. There is no Fast Mode option.
Displayed floors advance 1 through 20. The historical effective-content coordinates
remain internal to preserve authored terrain, enemy and item balance, with route
`1,3,5,7,10,11,13,15,17,20,21,23,25,27,30,31,33,35,37,40`.
The final physical floor uses full depth-40 content and the Administrator/finale;
there is no separate depth-39 floor. Godbeast milestones appear on displayed floors
3, 5, 8, 10, 13, 15, 18, 20. Heart stat rewards remain doubled. Heart blessings
are half their prior Fast Mode awards; the first level-up costs one blessing,
with later costs doubled successively: entering levels 2, 3, 4 costs 1, 2, 4
blessings respectively (corrected 2026-11-17). Sap retains its fill-to-next-level effect.

`compressedDepths` explicitly versions this route and reward contract in saves,
replays, temporal snapshots and future anchors. Existing saves/replays without
this flag keep their original route and reward rules rather than being destructively
renumbered. New normal/debug starts always use the twenty-floor contract.

## Targeting, inspection, numeric displays and Pack width (2026-11-17)

Selecting an already-targeted divine ability again cancels its targeting state and
removes its highlight without a simulation turn. This includes secondary-target
selection. Inspector content is exclusively the observed tile/entity report;
notifications and targeting-cancellation messages do not replace it.

Player-facing combat quantities, including Hanged HP/energy endpoints, bow
forecasts and combat stat labels, render in whole units. Simulation values remain
unchanged. Living fractional current HP continues to display at least one HP.

The Pack reserves the measured width of the longest Tier 4 amulet plus a legal
non-Kative Tinker ability name, its reserve bar and controls. At default typography
the maximum is “Tier 4 Amulet of Teleportation and Are You Sure About That?”.
The Pack can span the sidebars; narrow viewports or oversized fonts wrap labels
rather than hide them with ellipses.


Implementation hold (2026-11-17): Never Ending Flesh's nonlethal-only crossing and 25%-of-maximum-HP heal are provisional pending the designer's answer about lethal hits and healing basis. Do not treat these choices as settled or publish the combined release before clarification.
Approved-fixes-only export: the provisional Never Ending Flesh implementation is excluded from this source. The hold above applies to the separate combined candidate; existing Relentless Pursuit behavior is preserved here while clarification is pending.

### Persistent divine geometry — November 17, 2026
Riptide retains a blue line along the currently predictable pull route while active, stopping at observed blockers or unknown/nondeterministic continuation; it is recalculated after world changes, not left as a straight line through obstacles. Reverse Entropy retains a bright white outline of its actual radius, including in darkness and unknown space. That outline does not reveal terrain or occupants. Disabling the ability removes its overlay.

### November 17: distinct divine animation receipt geometry
Ragelight and Execute use red small novas on successful save/victim respectively;
Never Be Rid of Me uses blue. Fated Duel's existing 5×5 boundary expands through
alternating circle/square stages; Make Slippery contracts a 5×5 square on its target.
I Came For You grows a golden connection; Implant Thought holds a purple connection
through the attack; Mind Slave grows that connection then contracts a purple square.
Half Here and Waking Nightmares connect creation sites in cyan and black respectively.
These receipts do not alter gameplay ranges, costs, simulation randomness or turns.
The presentation layer controls when resulting state becomes visible.

### November 17: Space continuation prompts
Health threshold acknowledgments now require a fresh Space, not Escape. Paralysis
shows “You are paralyzed. Press space to wait until un-paralyzed”; acknowledgment
runs normal Wait actions one presentation boundary at a time, stopping when paralysis ends, for a required health/death pause, choices, or user
interruption. Damage or a newly seen threat alone does not cancel this waiting. Intentional
known chasm entry/Descend and Ascend from chasm are queued before dispatch. Forced
falls hold the prior observed scene until Space before displaying their outcome;
authoritative simulation remains atomic and may already have computed that result.

#### Death Swift ordering — November 17, 2026
The victim visibly dies before Death Swift's green nova plays on its beneficiary. The death and nova have separate presentation boundaries; gameplay simulation remains atomic and no extra turn is consumed.

## November17 nova visual correction
Novas use irregular gas-like radial lobes and a weighted cellular expansion instead of filling a perfect square. Wisps can reach beyond the former nominal square radius, without changing gameplay range or revealing hidden cells. The outward front remains brisk (small540ms, large810ms); smooth trailing state fades last660/990ms. Total animation/input lock is1200/1800ms. Fated Duel/Make Slippery squares are unchanged.

### Restored maintained-thread visuals — November 17, 2026
Bonded By Unseen Threads restores the October 15 instruction: a clear linked status and faint red thread between player and target for the active link. I Came For You uses the same thin-thread presentation in its existing gold color. The November 17 outgoing-line activation grows the thread to its target; the resulting maintained link follows actor movement. Only currently observed line cells render; hidden gaps are not connected. Existing action snapshots defer visible telepathy and linked-status results until the growing line finishes. These are presentation changes, not new visibility or telepathy mechanics.

## Amulet retirement and control revision (2026-11-17)

Current Revision 52 runs no longer generate Acid or Floods amulets. Existing
saved instances retain their IDs, tier, reserve and any Tinker grants, but are
shown as retired and have no Apply action; direct activation is rejected before
payment, RNG or world mutation. This is a read-time compatibility policy, not
inventory deletion or replacement. Existing acid statuses and temporary water
continue to expire normally. Divine Floods, flooding potions and unrelated acid
mechanics are unchanged. Historical Revision 50/51 behavior remains available.

Beckoning and Discord have unlimited targeting range at every tier, with existing
visibility, line-of-effect/interception and legal pull-path restrictions retained.
Obstruction remains unlimited range. Roots immobilizes for eight normal turns,
deals no damage at any tier, and retains its six-tile range and existing break
conditions. Existing captured Roots damage values remain loadable but do not
produce damage in current runs; an existing countdown is not extended by loading.
Recasting Roots sets eight turns and clears the obsolete captured damage value.
All surviving amulet energy costs, reserves, and other tier effects are unchanged.

Old Revision 52 recordings use an explicit amulet-rules command boundary: replay
initial generation and the pre-amendment command prefix retain their historical
roster and mechanics. New runs start with the amendment; a live old save adopts
it at its current command boundary. This prevents changing historical recordings
while ensuring removed amulets cannot be used by continuing an old live save.

### Approved screen status tints (2026-11-17)

Only these player statuses tint the game screen: Burning orange-red; Poisoned green; Hemophilia and Bleeding red; Confused white; Silenced and Silenced by aura black. Envenomed and every other status add no tint. Existing underwater blue remains. Overlapping approved color families contribute equally to a single bounded 34/255 veil; duplicate aliases in a family do not increase weight. The veil follows the detached presented state and local illumination, preserving darkness and animation/effect ordering. This is presentation only.

### Ring roster and Willpower amendment — November 17, 2026

Current ordinary ring loot is Stealth, Light, Regeneration, Vampirism, Reaping, Flight, Willpower, Protection, Slaying and Swimming. Fire Resistance, Poison Resistance, Sustain Strength, Ice Immunity and Corrosion Resistance no longer generate or provide their named effects. Existing instances retain their IDs, inventory slots and separate Tinker grants; their UI identifies them as retired. The Swimming island-escape safeguard remains in addition to ordinary loot.

Willpower halves every finite incoming status duration, beneficial or harmful, with duplicate copies multiplying the reduction and the resulting duration rounded up. Permanent/infinite states are not shortened. It applies when a status is granted or refreshed, not again when a saved state is reconstructed or a timer ticks. Historical replay command prefixes retain the earlier ring pool and effects through the independent `ringRulesFrom` policy marker, without changing simulation revision 52.

### Status-gas duration amendment — November 17, 2026

Paralysis, confusion and hallucinogenic gas refresh their respective statuses to five turns on each environmental exposure. Apply the recipient's Willpower duration reduction once to the incoming five turns, rounding up, before retaining the greater of that duration and the existing countdown. Contact never adds durations or shortens an already longer effect. Existing immunity and hallucination recipient restrictions remain unchanged. After leaving gas, ordinary status clocks expire the remaining duration. This changes status duration, not gas-cloud lifetime; caustic/breather damage, inert Blood Mist and other non-status gases are unchanged. Historical replay commands retain their former 2/8/2 durations through the separate gas-rules policy boundary.

## Futuresight: floor-entry rewind (2026-11-18)
Designer text: "Whenever you activate this ability or die, you are reset to when you last entered the floor. This can only be used once per floor."

Current live rules automatically arm it while owned and unused. Activation rewinds immediately without an energy payment or an energy/reserve confirmation. Death also consumes the floor's use and restores the entry image. The consumed-floor ledger survives rewinds, revisits and save/load; leaving/reentering never replenishes a used floor. Each new floor-entry replaces the checkpoint, including ascending and falling. Native UI uses a green ready dot and a 1–20 display-floor tracker, with an X over each consumed floor; no reserve/spiral indicator.

Old seed replays retain their previous ability through an explicit command-index policy boundary. Pre-amendment live saves have no recorded floor-entry checkpoint: loading them establishes a checkpoint at the loaded position, with exact entries from then onward. Floor-entry captures do not contain nested floor-entry anchors.

## Screamer scheduled lockout (2026-11-19)

After a successful paid scream, every Pale Screamer variant, including Greater,
receives a `dontScream` tag and a keyed scheduler deadline. It cannot choose its
scream again until that deadline expires, and all existing action/energy/target
conditions still apply. The deadline uses the maximum actual status duration
applied in that scream's existing recipient pass (normally five world turns).
Greater doubles strength, not duration. Recipient Willpower adjustments are
included; a normal caster's own five-turn allied buff still sets the maximum.
Wake/converge effects outside the radius do not contribute a timed status.
A scream that applies no timed status has no status duration to wait out.

At tick T, five-turn statuses are decremented at the end of T through T+4; the
lockout expires before AI at T+5. Haste/free commands do not advance world time.
This is a local caster lockout, not a scan for whether targets still carry a
status: early removal, death or other casters refreshing the status do not move
its deadline. A constant-size owner/deadline receipt remains after tag expiration so historical
offscreen catch-up still observes the original deadline. Owner identity prevents
cloning conditions from giving a new actor a cooldown it never earned. Due events resolve by
global actor ID, including offscreen actors; missing actors are harmless.

The serialized queue and tag rewind with simulation state, including remembered
futures. `screamerRulesFrom` is a non-rewindable replay command boundary: old
recorded prefixes keep the previous repeat-scream behavior; live saves adopt at
the next command. Markerless desktop journal tails finish under old rules before
adoption and checkpointing. No renderer or ambient timing changes are included.

## Current Avatar starters: at least one combat and energy spender (2026-11-19)
New production Avatar starts select uniformly among all complete tuples drawn
from the six current blessings of each selected god. Require at least one Direct
Combat blessing AND at least one actual energy spender (activation, proc or
upkeep); the same blessing may satisfy both. Two/three combat blessings are valid.
This supersedes any exactly-one restriction and the curated production starter
pools. Historical saves/replays and synthetic profiles retain their recorded
policy. Full eligibility, RNG, compatibility and complexity: [avatar-starters.md](avatar-starters.md).
## Mind Read persistent range outline (2026-11-19)

Mind Read now outlines its actual sensing radius whenever possessed: 5 while
inactive, 20 while active. The outline uses the same persistent, nonblocking
stroke and unknown-space presentation as Reverse Entropy, but Numo's existing
palette color (100,255,114). It follows the union of the exact core vision
origins, including the admitted Half Here clone. Unlike Reverse Entropy's
portal-metric range, Mind Read uses direct Euclidean coordinate distance. Walls,
lighting, undiscovered terrain and concealed portals do not change this outline.
It depicts Mind Read alone, not unrelated floor-wide telepathy or other senses.
No mechanics, saves, RNG, action timing or finite-animation queues change.

### Spent embers are harmless on contact

An ember tile cannot apply or refresh Burning, and does not count as an active
fire hazard to creatures. Active fire and hellfire still burn. Walking onto
spent embers does not extinguish Burning already acquired from actual fire;
that status still damages and expires normally. Terrain effects and ember
presentation are unchanged. An old live save adopts the contact rule at its
next command, while historical replays retain the previous contact rule.

## Normal stairs do not require Space acknowledgment (2026-11-19)

Descending normal stairs, either via Descend (keyboard or button) or by moving
onto the down tile (keyboard, click or path travel), no longer triggers the fall
acknowledgment. The desktop reads a command-local committed travel-cause receipt,
not an input-direction guess. A command that also actually falls is not exempt.
Chasm preflight confirmation, forced-fall outcome acknowledgment, fresh-Space
handling, paralysis waiting and death handling remain unchanged. Receipts are
presentation-only, do not create animations, and are absent from saves/replays.
In current runs, Ragelight is passive. It automatically attempts its normal
lethal-damage rescue while owned, with unchanged cost escalation, level-up
reset, and bypass rules. It has no activation bubble or toggle binding. A red
box between its name and energy-reserve bar shows the cost of the next rescue.

### Steam scalds while occupied

The active steam layer deals the same 4 HP per normal world tick as Burning,
before fire resistance. It does not set Burning. If an actor is already Burning,
its normal fire damage and timer take precedence; it does not take both packets.
Fire protection applies, not gas-only protection. Steam created by quenching in
water still scalds. The player is asked before deliberately entering observed
steam from clean air or another gas. Continuing within steam does not repeat
the entry prompt; forced movement and hidden steam never prompt.
Historic replays keep harmless steam until their recorded transition; old live
saves adopt scalding on their next command.
Current Execute costs 10 energy when a landed weapon hit takes an enemy from
at least 25% maximum HP to strictly below 25% without already killing it.
The finisher is terminal but grants no additional ordinary hit damage or
Reaping credit. A miss, an already-subthreshold target, a hit ending at exactly
25%, or a naturally lethal hit costs nothing. Existing replay prefixes keep
the previous pre-hit trigger.
## Beckoning consumes the pulled enemy's next action (2026-11-19)

Successful player-side Beckoning now gives the effectively relocated enemy one
pending action skip. Amulet Beckoning and allied Horn Beckon share this rule;
there is no Beckoning potion in the current catalog. Invalid, blocked or failed
pulls, allies, the player, stationary decoys and nontargets do not receive it.
Repeated pulls refresh one pending skip rather than stacking them. It is an
action receipt, not paralysis: no new stun animation or timed-status resistance.
The next scheduled full NPC action consumes it, including an admitted Haste
bonus action; a later normal action remains available. Fast cannot generate a
follow-up for a skipped action. Normal upkeep/status/environment clocks continue.
A pending skip persists across save/load, offscreen simulation and time travel;
its identity stamp prevents an actor clone inheriting another actor's skip.
The non-rewindable `beckoningRulesFrom` command boundary preserves legacy replay
prefixes and journal tails; live R52 saves adopt at their next command boundary.

### Ichor fire spreads at half cadence

Existing Ichor flames attempt to spread to neighboring cells on alternate
normal world ticks. Ordinary fire and hellfire still spread every world tick.
Direct ignition, same-cell gas reactions, and Mercuria’s liquid flood are
unchanged. Unlit Ichor does not ignite on its own. Inactive floors use the same
world-tick cadence as active floors. Older replay commands retain their
original fire-spread rules; old live saves adopt the new cadence at their next
command.

### Entropy does not protect against ichor (R52)
Entropy's caustic-gas immunity no longer rejects ichor damage packets. Caustic and Gasser gas immunity, beast/pale gas traits, gear resistance, and the independent Ichor-fish fire immunity are unchanged. Entropy's active toggle does not alter this distinction. This adds no liquid-contact damage: ordinary ichor currently supports habitat, regeneration/siphon and ignition; burning ichor delivers fire damage, not an ichor packet. The generic ichor packet API has no live command producer in the native engine; therefore this correction needs no new intra-R52 replay marker. Pre-R52 direct packet semantics remain historical.

### Combat presentation simplification
Make Slippery retains its activation box but successful repeated movement prevention emits no new box. Commandment's passive one-tile push (including insufficient-energy fallback) has no shove cue; a paid active push has one 240ms white actor-route animation. Each route cell is admitted during actual movement, portal jumps are never interpolated, and a floor transition does not attach old-floor coordinates to the destination floor. Mechanics, payment, collision, arrival and stair/fall behavior are unchanged. Nova's real explosion remains its sole effect; no separate toggle decoration.

Use one short, non-blocking god-color nova for a completed Dream of Time
Gone By return, Ascend, Dreamwalk/Wake, Masque teleport, Sever Fate,
Remember, Distract, Blood Weapon hit, Blood Letting cleanse, Sudden Sleep,
and Overflowing rescue. Anchors, upkeep, charges, blocked casts and blood
refills do not emit these novas. Erase Metaphysical's actual removal emits
a purple nova around the enemy. The cue reveals no hidden endpoints.

### Live R52 divine balance

- **Overflowing:** Start with five death saves. Survive a rescuable lethal event at
  1 HP and spend one save; damage itself never adds saves. Reaching maximum HP or
  gaining a level replenishes all five. Blood Mist, acquisition-order priority,
  and unblockable deaths stay unchanged.
- **Fight Long:** Heal for 3% of actual HP removed by a melee hit, not overkill,
  ranged damage or misses. Rings of Vampirism still contribute 5% each.
- **Sudden Sleep:** 25 paralysis turns after activation instead of 50.
- **Dream of Time Gone By:** Anchor for 20 energy; return remains free.
- **Sleep Walk:** Haste below 10% of effective maximum energy. If energy is
  uncapped, require zero energy as before.

New runs use these rules immediately. Existing live saves and recovered journals
adopt them at their next command boundary; old recorded prefixes retain their
original rules.

### R52 escape collapse and trailing hellfire

The collapse still opens chasms and travels up through the staircase
connections, but leaves both stair tiles and the blue exit intact. Its
permanent Hellfire trail seeds the boss death cell after four world turns,
then follows the same alternating cardinal and diagonal layers every four
turns, half the collapse's speed. Each upper floor ignites when the flame
reaches the stairs below, not when the chasm front does. Hellfire can
cover intact stairs and burns targets that can fly; the ordinary fire
immunity and secondary fire rules still apply. The stair and exit symbols
remain visible through the flames.

Existing live saves adopt this policy for the next collapse they start.
A collapse already underway when the save is upgraded keeps its original
rules, and previously recorded commands are not reinterpreted.

### R52 Blood Mist and creature backgrounds

Blood Mist from Overflowing is harmless and nonflammable. It does not fuel or
spread fire; other flammable material on the same square still behaves normally.
Existing replay commands retain their original gas/fire behavior. New R52
commands use the nonflammable rule after live save admission.

Creature glyphs no longer apply the default dark or blue inset over terrain.
Telepathy still changes the glyph ink, and explicitly authored appearance
backgrounds still override the default.

### R52 Dreamwalking footsteps and Death Gas

While dreamwalking, moving onto a corpse or waiting over one does not stomp
it, delay revival, or affect heart-harvest state. Stomping while awake is
unchanged.

Greater Gassers emit Death Gas both passively and when they exhale a larger
cloud. It applies the ordinary caustic damage and hallucinogenic exposure
rules together, preserving each component's immunities. Ordinary Gassers
still emit caustic gas. Existing saves adopt these changes on the next
recorded command; old replay prefixes retain their original behavior.

### R52 committed abilities and Dire Bear glyphs

Keep Chronicle entries for committed active abilities and visible enemy casts,
even when their animation is absent, the action is free, or their effect
misses. Failed admission must not appear as a successful cast; unobserved
creatures are not identified. Routine AI waits and recoveries stay quiet.

The inherited Dire Bear map, sidebar, and bestiary glyph is a plain black
`B` without a neutral outline. Existing telepathy and explicit appearance
overrides still apply.

### R52 Minotaur attacks

A Minotaur can strike only when an eligible foe occupies the legal next
square of its committed charge, including the second step in a Fast action.
Adjacent targets off its arrow, direct attacks ordered by Implant Thought,
and automatic counterattacks do not qualify. Discord still changes whom
the Minotaur can hit, not where it can attack. Existing replays retain their
original behavior up to the recorded rules boundary.

### R52 thrown Borge offerings

A thrown heart counts as an offering if its projectile reaches the active
Borge altar and its actor and floor provenance match the demanded sacrifice.
The altar consumes the thrown heart and places the one-use shard on that exact
tile, independent of ground clutter or pack space. The satisfied altar can
be walked onto to collect it. Bumping an unsatisfied altar with the marked heart
in the pack still awards the shard directly. An interrupted throw is not an
offering.

### R52 Banshee shriek

Every 30 turns (clock resets on revival), the shriek sends a white Screamer-style
wave across the entire current floor, through walls and fog. The same front is
purple on optically observed squares in the Banshee's line of sight, without
exposing hidden terrain. Only living creatures in its line of sight on its plane
take damage: half of their current HP plus 100, rounded by normal damage rules
and ignoring armor. The Banshee itself is excluded. Historical replay commands
retain the previous flat damage and animation.

### R52 local floor simulation

Each turn simulates the active floor and at most its immediate physical
neighbors. Other floors do not run AI, ecology, hazards, ally following, or
collapse/hellfire pulses. Skipped turns are discarded rather than replayed
when a floor comes back into range. Floor-to-floor transfers still resolve
their direct landing effects without running unrelated distant ecology.
Absolute world deadlines may already be due on return; per-turn effects
do not tick while the floor is out of range. Historical replay commands
retain their original catch-up behavior.
