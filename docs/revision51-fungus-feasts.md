# Revision 51 fungus feasts and forward roots

## Current designer instructions

Generated Corpse Fungi begin with two heartless feast victims: one appropriate to
that floor and one whose ordinary first eligible floor is two or three floors
later. Root-bound corpses cannot revive. The root glyph follows the next segment
of the line, rather than the chord from its previous segment. Fungi must not be
introduced on late floors to obtain a deeper victim, and Godbeasts must never be
used as feast corpses.

This supersedes the two canned, same-species corpse props for revision 51. Older
revisions retain their generation, RNG, observed-root glyphs and replay behavior.
Saved actors and roots are not regenerated or retroactively replaced.

## Root acquisition and revival audit

The existing runtime logic was already enforcing the no-revival requirement and
is unchanged:

- `fungi::tick_fungi` considers every corpse within Euclidean distance 10 of a
  living actor with `enemy:beast:fungal-gas`, provided there is line of sight. It
  lays the existing Bresenham root path, including its endpoints, when neither
  the source nor the path is frozen and the path is not burning.
- A saved connection is active only while the source lives, the target remains
  dead, every path cell retains root material and no path cell burns. Cutting,
  burning or moving a bound corpse can sever it. The saved fungus/corpse pair
  remains recorded, so the same source cannot reconnect that severed pair.
- `environment::corpse_rooted` checks the active link, living fungal source and
  intact nonburning path. `engine`'s normal corpse phase and both `offscreen`
  revival paths reject rooted corpses before any revival roll. Haste does not
  add an independent corpse revival clock.
- Corpse ages continue to advance while rooted, so an old victim may rise after
  release. Corpse Fungus itself is separately excluded by `corpses::can_revive_corpse`
  and cannot revive even without roots. Killing the fungus releases its victims.
- A fungus emits 25 gas units plus 25 per active bound corpse. From revision 50
  its gas is hallucinogenic. This emission and ongoing acquisition behavior are
  unchanged; later corpses can be acquired, and severed meals are not magically
  replaced.

## Generated feasts

`fungus_feasts` derives both pools from the same revision-aware ordinary
recipes, including aquatic species as dead specimens. The normal pool uses the current floor.
The deeper pool requires an ordinary first eligible floor of exactly current+2
or current+3 and eligibility in that future floor's recipes. Display/reference
combat depths are not used. Pool order is stable; each corpse consumes one draw
from an independent `seed:floor:depth:fungus-feasts` generation stream. Existing
machine actor, ordinary composition, sleep and loot RNG streams are not consumed.

Only ordinary roster species qualify. Godbeasts, special bosses, event-only
creatures and Sphinx cannot enter these pools. Ordinary paired species and
Banshees remain eligible as dead specimens; a follower uses its authored
leader's depth window. The living-horde pair/Banshee planner is unchanged.
Aquatic corpses may be placed on dry floor: dead actors need no living habitat,
and released aquatic survivors retain the existing stranded behavior. No enemy
stats, rewards or depth windows are invented or changed.

For each setup fungus, reservation searches within the existing radius of 10,
requires clear ordinary line of sight and a nonburning root path, and selects the
two nearest unreserved floor cells. Equal distances use stable cell-index order.
Both cells are reserved together before ordinary actors or loot are placed.
Each new victim is dead, heartless, zero HP and age zero, and gets an active root
connection immediately, using the existing saved root representation.

When two legal cells or either eligible pool are unavailable, the fungus and
both corpses are omitted together. Its authored room geometry remains; the
ordinary population calculation uses only surviving setup actors. No partial
feast or fatal dungeon-generation failure is introduced by this fallback.
The current roster has no +2/+3 introduction on floors 37–40, so revision 51
places no new setup fungus there. Dead Inventors and Mechs, first eligible on
floor 14, are valid deeper meals on floors 11 and 12. A dead Kelpie, first
eligible on floor 16, supplies the floor-13 deeper meal. It does not substitute an in-depth enemy,
Godbeast, future nonexistent floor or artificially strengthened creature.
Existing setup domain/placement rules otherwise remain in force. The unchanged
authored setup 24 metadata allows floors 1–40 in Dungeon, Cave, Meadow, Forest,
Archipelago and Mountain. Requiring an actual deeper meal narrows effective
revision-51 population to floors 1–36; including ordinary aquatic corpses does
not extend that range.

### Paired corpse lifecycle audit

The complete-pair requirement in `population::populate_hordes_revision` applies
to ordinary living horde recipes, not corpse construction or save admission.
`validation::actor_state` validates a pilot's type only when one is present; it
does not require a partner link. `monster_inventor::turn` returns before behavior
for dead actors and handles absent links after revival: an Inventor without a
Mech can flee, while an unpiloted Mech becomes neutral. `lifecycle::ursa_death_cry`
also treats its companion as optional. A solitary corpse therefore requires no
invented link or hidden companion. Blanket pair/Banshee exclusions were removed
after this source audit because they were not authorized restrictions.

Likewise, living aquatic habitat restrictions do not invalidate a dead specimen.
The Kelpie regression saves/loads the rooted corpse, verifies that
`water_blood::register_kelpie_sources` ignores it, then severs its roots and
advances normal turns. Its revival succeeds, `ichor::squirm_if_stranded` handles
the dry-floor survivor without moving it, and the live stranded state saves and
loads successfully. Ordinary Kelpie and River Drake windows remain 16–25 and
22–31 respectively; no aquatic event creatures enter the ordinary pool.

## Forward segment presentation

`hazard_memory::observation_context_revision` computes each nonterminal root
cell's glyph from the cell to the next entry in its saved path. The terminal
cell continues the final incoming segment; a one-cell path keeps the previous
vertical marker. Crossing different orientations still displays `┼`. No path,
active flag, rooting rule, gas volume or RNG changes for this presentation fix.

Live observation, turn-end hazard memory and desktop hazard/travel projection
supply the run's revision. Historical helper APIs remain revision-29 wrappers
for frozen differential callers. Remembered hidden cells retain the last observed
glyph until they are observed again, without reading live hidden root directions.

## Complexity and bounds

These are source-derived estimates, not measured timings. Let F be setup fungi,
A machine actors including already added victims, V floor cells (<=3600), K<=57
ordinary species, T<=57 flavor-placement rows, Q=2 authored pair entries, M<=5
members per candidate, C candidate recipes across the three queried depths,
J maximum kind-ID bytes, B<=441 radius-bounding cells, R<=11 cells per root/ray,
P permanent-fire cells and H total stored root path entries. Let L be a cloned
actor's complete payload and W its existing construction/trait-overlay work.

- Victim pool selection is O(C*M*(Q+K+T)*J) time and O(K*J) extra memory with the
  existing expected hash lookup assumptions. This includes pair checks, linear
  species lookup, final-window lookup, string comparison and stable Vec dedup.
  A cold recipe cache additionally pays existing ordinary-roster construction
  and retained-cache costs documented in `native-monster-depth-windows.md` and
  `flavor-placement-v32.md`; it is not a constant-time catalog lookup.
- Each reservation pass is O(F*B*(R*(1+P)+log B)) time and O(B+F+K*J) scratch,
  plus pool selection above. This includes direct LOS and its existing door
  map lookup/string-key cost (bounded by the <=3600-cell index length), root
  path construction, linear permanent-fire scans and nearest-cell sorting.
  Membership in blocked/reserved sets uses expected hash bounds; pathological
  hash collisions add their existing set-size factors. The normal startup
  pipeline performs three bounded planning passes, not one pass per turn.
- Populating two victims per fungus is O(F*(A+L+W+R)+V) time and
  O(K*J+F*(L+R)+V) added/output memory, plus pool selection. The A scan locates
  each source; actor construction includes its existing template, revisioned
  trait/balance and name work. The root layer is allocated only if absent.
  Existing allocator/encounter checks and transactional floor/Run clone costs
  remain additive, including any copy-on-write detachment.
- Root glyph selection adds O(1) work and no storage per path cell to the
  existing O(H) expected root observation index. The index retains O(H) data
  in the worst case; root-color copying scales with its bounded saved string.
  Existing vent/light/permanent-fire indexes, visibility, hazard serialization,
  native glyph rasterization/cache misses and framebuffer upload are unchanged.
  Remembered hazards are updated only through the existing visible-cell mask.

For profiling, vary setup count, permanent-fire count and root-path density on
representative seeds; measure cold and warm candidate caches separately. This
change makes no platform speed claim.

## Verification

Added deterministic regressions in `revision51_fungus_feasts` cover all 40
floors with four seeds each (36 feasible, four deliberately omitted); same-seed actor/floor equality; exact current/+2/+3
eligibility; no Godbeasts or late fungus; nearest-cell fallback, impossible rooms,
multiple fungi without shared victim cells; unchanged legacy corpses; outgoing
bends, terminal/intersection/single-cell glyphs; and hidden-memory refresh.

`revision50_bastidion` additionally exercises mature rooted victims through
normal/haste turns, severing and subsequent revival, offscreen catch-up after
save/load and both temporal restore paths, dead fungus non-revival, and save/active-turn/released-revival handling for a
solitary Inventor and unpiloted Mech, plus the dead/released aquatic lifecycle.

The corrected square-floor fixture passed the initial nine-test fungus target
(`verification/revision51-fungus-feasts/focused-tests.log`). The earlier combined
run passed all eight then-present Bastidion/root lifecycle tests
(`verification/revision51-damage/focused-tests.log`). Subsequent removal of the
precautionary pair/Banshee exclusions passed their nine-test corpse lifecycle
target in `verification/r51-ui/direct-core-tests.log`. The updated ten-test
corpse lifecycle target, including safe Kelpie revival/stranding, passed against
the preserved native core artifact in
`verification/revision51-fungus-feasts/bastidion-aquatic-tests.log`.
The initial combined feast run exposed two incorrect floor expectations:
Inventor/Mech start on 14, not 15/16, and the dry-land-only pool incorrectly
omitted the valid dead Kelpie for floor 13. The tests and pool source have been
corrected; the updated eleven-test generation target still needs a fresh core
artifact. Earlier passes do not verify this latest pool edit.
Run `tools/cargo.sh test -p corpse-core --test revision51_fungus_feasts
--test revision50_bastidion`, followed by the aggregate native workspace suite.
No native Windows launch or visual-platform verification is claimed here.

The subsequent [root lifecycle/color correction](revision51-root-cleanup.md)
removes orphaned severed geometry and separates authored olive root material from
hallucinogenic gas. It preserves the initial two-victim requirement, ongoing
uncapped acquisition and forward-segment glyphs described here.
