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

## Numbering status

Destination-number semantics are under clarification with the designer. Do not
publish provisional connection labels as a completed numbering implementation.
No new movement, direction, portal or vision semantics are authorized by labels.

## Verification

Deterministic source regressions are in core `revision51_speed_up_tiles.rs` and
desktop `non_euclidean_ui::revision51_tests`. They cover generation, legacy
behavior, normal/haste movement, rays, replacement and restoration, admission,
save/resume, terrain memory, Revert/Future, same-tick vision, hidden replacements,
item/actor layering, and animated renderer preservation. Capture opt-in:
`CORPSE_SPEED_UP_REVIEW=<directory>` with the desktop `revision51_speed_up` filter.
Execution results are recorded separately; this file alone is not a test pass.
