# Plain tile-highlight removal

Implements the 201-row v13 checklist by presentation semantics rather than ability
identity. H001–H197 and H199–H201 are the generic local tile-highlight branch,
including simultaneous multi-cell and staggered local Travel highlights. They are
removed before Effects phase scheduling and scene capture. This includes ordinary
player and enemy death; terminal admission and the fresh Space requirement remain
unchanged. H198 floor/plane departure tint is disabled in both transition creation
and rendering. Empty batches do not create a deferred-frame input gate.

Preserved: actual Magic/Projectile Travel, specialized nova/box/line/thread,
radial reveal, moving-actor and light100 patterns, HUD/inventory receipts, ambient
rendering, and all simulation mechanics. Specialized patterns are preserved even
when visibility admits only one cell, and retain Reduced/Instant behavior. Explicit
P57 Nova and E37 Bombardier `burst-ember` explosion footprints and legacy held
explosion markers were not checklist highlights and remain enabled. Their attached
generic local flashes do not survive merely because they share an action ID.

The admission rule is O(1) per event, O(E) per batch with no new allocations. E is
bounded by the existing 4096-event cap. Remaining scheduling is O(E log E) with
O(E) scratch; dropped highlights consume no phase duration, scene snapshot hold,
or replay/autoexplore/input wait. Existing shape rendering complexity is unchanged.
No timing benchmark or Windows runtime validation is claimed by these changes.
