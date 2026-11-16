# Revision 51 Speed-up tiles

## Authored correction

Speed-up areas are their own terrain, teal like Non-Euclidean space. They are
ordered Non-Euclidean space, not a speed buff, timed ability or tile modifier.
The r37 ordered assignments, local reversal exception, shared occupants,
projectile transfer and no-projected-vision contract remain in force.

New r51 generated cells are `speed-up` rather than grass, floor or shallow water
under a green inset. Saved topology remains exact and deterministic. A new
optional `speedUpTerrain` flag selects actual-terrain semantics: replacing either
endpoint suspends the pair, and restoring its exact tile reactivates it. <=50
runs retain historical overlay behavior and never acquire new fields on load.

Lighting sees Speed-up as open, non-emitting terrain. It does not project sight,
act like water, acquire currents or add an action clock. Hazard overlays remain
separate. Black Pit and Cogworks retain exact previous terrain and restore it
through existing turn/removal paths. Descent potion ground selection admits the
new ground type. Amulet temporary-terrain admission still protects spatial links.
Monster wall-seal's existing ordinary-floor-only admission does not target this
new nonordinary terrain. Tile mutation bypasses no existing COW/index/cache path.

Current/remembered terrain identifies Speed-up tiles. Unseen replacements do not
update remembered map markers, previews or descriptions. Item and actor layers
retain their existing single physical identities, foreground priority and teal
ground beneath them. Cosmetic animation restores detached topology markers.

## Adjacent destination mapping, clarified for revision 52

The designer clarified that Speed-up uses the same Non-Euclidean mapping, with
the paired endpoints always next to each other. The existing generator already
builds successive one-source pairs along a row or column. For example, cells at
x=11 and x=12 share one link number; x=13 and x=14 share the next. The number
identifies the linked destination, not the entire Speed-up region. Both terrain
types use the same floor-wide `group_number` and observed-exit adapter. A normal
two-source Non-Euclidean group retains its two possible return destinations.

This clarification needs no new runtime mechanic, UI explanation, save field,
save rejection, random draw or remapping. Existing R51 and R52 mappings are
identical for the same generation input. Numbering follows serialized group
order, not viewport order, observation order or current material. Hover and
click routes use the actual saved endpoints and only known destinations. The
local reversal exception, shared occupancy, rays and local sight remain intact.

The generator proposes only even-length lines (4, 6, 8 or 10 cells). In an
odd-length eligible corridor, cells outside an accepted complete pair stay
ordinary ground. Repeated installation is a no-op; proposals overlapping saved
spatial links are excluded. There is no Speed-up casting ability or cast state.
Replacing either or both endpoints suspends the pair without changing its
saved destination or number; restoring both exact terrain cells reactivates it.

## Verification

Deterministic source regressions are in core `revision51_speed_up_tiles.rs` and
desktop `non_euclidean_ui::revision51_tests`. They cover generation, legacy
behavior, normal/haste movement, rays, replacement and restoration, admission,
save/resume, terrain memory, Revert/Future, same-tick vision, hidden replacements,
item/actor layering, and animated renderer preservation. Capture opt-in:
`CORPSE_SPEED_UP_REVIEW=<directory>` with the desktop `revision51_speed_up` filter.
Execution results are recorded separately; this file alone is not a test pass.

The `revision52_speed_up` regressions in those same core and desktop files add
seeded R51/R52 mapping equality, adjacent disjoint endpoint pairs, odd eligible
lines, repeated installation/replacement, exact save/fixture replay and restart,
mixed-type destination glyphs, viewport subsets, hidden destinations, click
preview versus actual normal/haste movement, and native resized/animated digit
ink. These review the existing behavior rather than introducing new rules.
