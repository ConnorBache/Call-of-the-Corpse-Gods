# Revision 35: linked portals

The designer specified same-floor pairs from depth 10, optional traversal, italic ASCII `0`, matching pair colors in blue/orange/red/yellow/green/indigo/violet order, and linked-space behavior for occupancy, attacks, sight and thrown objects.

## Initial implementation defaults

- One pair is requested on every generated floor from 10 through 40. This is a conservative initial density, not authored depth scaling. Metadata and presentation support up to seven pairs per floor in the specified color order.
- Placement uses a separate deterministic seed/floor stream. Endpoints avoid stairs, doors, fixtures, actors and loot, and the ordinary walking graph remains connected within each original component when every selected endpoint is avoided together. No terrain is rewritten to force a pair; a map without safe candidates may omit it.
- Entering an unoccupied endpoint moves the actor to its mate exactly once. The following step starts at that arrival, not another automatic teleport. A same-plane occupant on either endpoint occupies the shared space; entering an enemy-occupied pair attacks rather than telefragging. Existing ally swap behavior is retained with linked occupancy.
- Portal sight creates a viewpoint through the other end while retaining traveled distance for perception. Occlusion before the entry and after the exit still applies. This is not unlimited remote visibility.
- Physical rays preserve their direction and remaining aim when crossing a portal. Selecting a remote target uses a translated aim through a linked endpoint. Rays stop at blockers/actors; each pair can be crossed at most once in one ray, bounding cycles. A visible target is not necessarily on a valid straight projectile route.
- Ranged distance is the sum of Euclidean lengths of continuous segments, with portal jumps costing no range. It is not the physical map displacement or a sum of raster diagonal steps.
- Portals remain recorded if terrain later changes. Movement still respects the actor's traversal capabilities; opaque endpoint terrain blocks optical/projectile transfer. The save validator checks structural metadata rather than rejecting legitimate altered terrain.
- Links remain within each actor's existing normal or dream plane. Half Here keeps its creation plane. An occupied dream destination blocks beginning Dreamwalking rather than overlapping another same-plane creature.
- Direct attacks and their effects may travel through links. Autonomous fire/gas/liquid propagation is unchanged; the designer did not specify transport for environmental simulation.

## UI and persistence

Observed portal occupants and items share their identity across visible endpoint representations; lists and inventory do not duplicate them. Unseen endpoint metadata does not grant route-preview knowledge. Known arrival hazards participate in movement warnings; unknown remote hazards are not disclosed by the UI. The glyph is an italic rendering treatment of ASCII `0`, not a substitute Unicode character.

All pairs are serialized in floor state and included in snapshots. Legacy revision 29–34 runs retain prior rules. New runs and DEBUG floor starts use revision 35. Tests cover generation, linked melee and occupancy, NPC/feeding/offscreen routes, projectile range, perception, native planes, save/load/replay/Revert/Future and desktop previews. See the final verification receipt for executed results, rather than treating this document as a test-pass claim.
