# Revision 35: linked portals

## Implemented

Portals begin on floor10 and connect two cells on the same floor. Entering remains optional: generated pairs never force the ordinary walking route through an endpoint. A paired cell occupied by a same-plane enemy is attacked rather than telefragged; empty entry transfers exactly once. NPC routes, returning allies and hungry/offscreen creatures use the same transition rules.

The glyph is italic ASCII `0`. Matching pair colors follow blue, orange, red, yellow, green, indigo, violet. The initial generator requests one pair per qualifying floor; seven pairs are supported, but increased density is not presented as designer-authored tuning. Unsafe maps may omit a pair instead of rewriting terrain.

Sight projects through the link with traveled-distance limits and normal occlusion. Physical rays preserve direction and remaining range, including thrown items and supported ranged attacks. Linked occupancy/item inspection, route preview and known arrival hazards follow the same geometry without revealing hidden remote state. Portal ray cycles are bounded.

Dreamwalking and Half Here retain native-plane separation. Direct attacks can cross portals inside their own plane; autonomous fire/gas/liquid propagation is unchanged. See [detailed rules and defaults](portals-r35.md) and [generation constraints](portals-generation-r35.md).

## Compatibility and verification

New descents and DEBUG starts use revision35. Saved revisions29–34 retain their prior rules and fixed creature profiles. Original artwork, lore, catalogs, Cargo.lock and the pinned toolchain are preserved.

[Current verification](verification-r35.md) records the final executed tests, frozen inputs and platform limits. Focused tests alone are not a full-workspace pass. Windows cross-compilation/static package checks do not establish Windows runtime behavior. The Windows executable is unsigned and has not been launched on Windows in this environment.

Screenshots distinguish controlled native-renderer fixtures from a generated floor10 captured with DEBUG reveal/repositioning; they are not Windows runtime screenshots or a survival playthrough. No new artistic assets were generated for this update.
