# Revision 52 directional amulet aiming

The four continuing bolt amulets, Lightning, Firebolt, Poison and Acid, can be
applied in an empty direction. A creature is not required on the selected tile.
Their existing simulation extends the selected slope beyond that tile. Lightning
pierces creatures; Firebolt, Poison and Acid stop at the first creature. Terrain
still stops them, and Acid retains its eight-tile physical range.

## Existing controls, corrected origin

Apply an amulet in Pack, then either click a known map tile (including an empty
tile), or press a movement direction and Enter. The first keyboard direction for
one of those four amulets is now relative to the player, not a stale mouse-hover
tile left over from before Apply. Subsequent movement keys adjust the aim cursor.
Moving the mouse returns to ordinary mouse aiming. Escape cancels without payment.
There are no new key bindings.

The old mouse route already admitted empty tiles. The reproduced UI defect was
the first keyboard direction retaining old hover coordinates: from player (10,10)
and old hover (30,4), Right selected (31,4), not the eastward tile (11,10).

Endpoint and footprint effects retain their contracts: Iceblast and Blinking
select visible endpoints, Obstruction selects its wall footprint, and Beckoning,
Discord, Roots and Teleportation still require their eligible creature targets.
Revision 52 Floods retains its targeted cardinal/diagonal wave and enters target mode.
Legacy revision 50–51 keyboard cursor behavior is unchanged.

## Boundaries and verification

Mouse and Enter use one production target-admission helper and the existing
UseAt command. No actor ID is required by projectile admission. Unknown cells and
hidden actors do not become preview or blocker-message probes; only the actual
paid simulation resolves physical collisions. No range, damage, targeting class,
resource rule, plane rule, save schema or RNG stream changes.

`directional_zap_ui::tests` exercises Pack keyboard activation, first/subsequent
ordinary direction keys, empty-space mouse/keyboard parity through the production
admission and dispatch helpers, cancellation, target-class exclusions, walls,
trees, doors, Godbeast fixtures, Acid range, hidden state, other planes, haste,
Spellslinger, save/load and See the Future. The opt-in
`CORPSE_DIRECTIONAL_ZAP_REVIEW` output captures actual native renderer pixels
before and during a command-driven empty ray. These are native CPU handler/render
tests, not evidence of physical OS input delivery or Windows runtime testing.

## Complexity

Let I be carried item count and V the number of projected map cells. First-key
origin classification scans I items once, O(I) time and O(1) extra memory, without
terrain observation, pathfinding, actor lookup or RNG. Prompt classification is
also O(I), only when entering item aim. Subsequent keyboard cursor coordinates use
O(1) work. Target confirmation retains existing target-admission costs and the
existing O(V) projected-cell membership scan, with O(1) new scratch memory.

Preview/launch costs are unchanged: their existing visibility, floor clone,
portal planning and collision traversal costs are documented in the amulet aiming
ledger. Native draw/upload, glyph-cache misses and action-animation costs are
unchanged. No new per-frame or offscreen simulation work is introduced. These are
asymptotic bounds, not measured frame timings.
