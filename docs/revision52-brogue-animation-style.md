# Brogue-style gameplay animation

## Current rule

Every gameplay effect is a lit tile or a moving glyph. Actual ability-use and proc receipts, ranged effects, and selected ambient features retain their colors and their already-admitted observed cells. Ordinary walking, shared melee hit/miss/damage and routine natural regeneration no longer add action ornaments. Actual distinctive procs, statuses and deaths may still light their tile.

- Magic uses advancing tile illumination with a tapered fading tail. Physical projectiles move one canonical item glyph (or the physical spit glyph) through the exact recorded cells, including portal discontinuities, without a luminous trail. Explicit delivery metadata keeps a magical acid bolt distinct from physical spit even when their old style labels match. Magic steps last 32 ms per admitted cell; physical projectiles last 40 ms per cell. Impacts follow the route duration rather than a fixed 140 ms total. See [the Firebolt and projectile amendment](revision52-magic-projectile-travel.md) for source references, staging and replay compatibility.
- Area casts and local procs illuminate their actual cells. Teleport and symbolic-link receipts illuminate only their independently known endpoints. Stationary old-form/death glyphs are not stretched, folded or painted as ghost overlays.
- Semantic style IDs remain receipt metadata and determine palette or existing HUD anchor. They no longer draw rings, shields, slashes, sparks, gear outlines, bottle halves, membranes or other vector art. Inventory/resource feedback is one restrained rectangular highlight in the real item/resource tile.
- Dreamwalk, Wake and Ascend illuminate only the previously observed player tile. The old outgoing map is never crossfaded over the new plane. Existing internal transition API names remain for compatibility; they now paint that one tile.
- Water moves the actual `~` glyph gently within its own cell. Gas and reflections use faint full-tile illumination. Existing fire, torch and material/light palette variation remains cell lighting, not particles or deforming scenery.
- Reduced and Instant feedback remains static; projectile travel becomes an endpoint light. Area footprints stay complete. No animation consumes RNG, dispatches gameplay, waits for input, changes canonical saves, or modifies the simulation.

This updates gameplay/ability feedback. The menu’s existing authored illustration is not a gameplay animation and is unchanged.

## Complexity and portability

Let E be queued receipts (existing cap 4096), C the total exact cells in those receipts, V the visible map cell count, K the number of distinct emitted effect cells, P the raster pixels touched, and G the glyph raster/mask work.

On command receipt, filtering shared melee builds a BTreeSet of ranged action IDs and retains events in O(E log E) time and O(E) extra storage. Existing event sorting is O(E log E); queue-cap eviction cost is unchanged. No additional world lookup occurs.

The presentation receipt adds explicit local/magic/physical delivery metadata and preserves the supplied physical item glyph, O(1) fields plus glyph bytes. A correctly hidden shooter route still does not suppress the observed victim’s impact. The separately gated Firebolt travel amendment changes simulation behavior as documented in its own entry.

Each action projection checks delivery metadata and emits either one physical projectile tile in O(1), or traverses the C receipt cells in O(C) time and O(C) output. Magic retains at most four lit tail cells per single route at the authored timing, but scans the admitted route; this is not constant-time projection. Palette/style strings are short authored metadata; arbitrary supplied text copying and classification additionally cost their byte lengths. Legacy rays retain their existing bounded step scan. Full painting builds O(V) knowledge/actor/light tables and merges emitted cells through a BTreeMap in O(V + C log(K+1)) time and O(V+K) scratch. Retained/tile damage projection repeats projection as documented by the existing tile renderer; this is not an O(changed tiles)-only pipeline.

Native software illumination costs O(P); GPU quads add submission/driver work and target pixel shading. Warm moving-glyph drawing uses cached masks and costs O(G); a cold cache also includes font rasterization/mask construction. The HUD highlight has O(P) time and O(1) scratch, replacing up to 150 line segments per old mark. Ambient water uses one cached glyph draw with bounded within-tile translation. Gas uses one full-cell quad instead of a changing partial band.

Transition painting scans the existing detached outgoing View in O(V), then paints one tile in O(P). Its upstream conditional full observation and O(View bytes) retained snapshot are unchanged, as is conservative map-region invalidation until expiry. This change does not claim to optimize those costs.

No new timing measurement is implied. Measure representative busy visible rooms, portal routes, large affected areas and cold/warm raster caches to profile. Full workspace tests and final release/platform checks are separate from focused renderer regressions.

## Validation

The former tests demanding unique vector silhouettes now assert the new primitive contract directly, rather than preserving the superseded art. New regressions cover exactly one projectile glyph, portal gaps, endpoint-only reduced motion, exact area/proc lights, quiet ordinary movement/melee, retained ranged impact/ability procs, and one-cell transition bounds. Existing actual A01–A12, potion, Dreamwalk/Wake/Ascend, construction/removal and retained/full/tiled parity tests remain part of the focused checkpoint.

Focused native animation tests passed 25/25; the core animation contract passed 19/19. The complete native binary suite passed 779 tests, failed 2, and ignored 36. Both failures are the known unrelated missing-stat recovery bridge in bestiary damage assertions. The test-only bridge is confined to the disposable harness and excluded from this patch. These results do not certify the final integrated source or Windows runtime.

Receipts are `verification/brogue-style/focused-4.log`, `core-contract-1.log`, `native-all-1.log` and `test-report.json`. Native captures include actual amulet, inventory, transition and construction commands plus exact full/tile projectile frames. Travel, impact, Floods and Dreamwalk images were visually inspected, including clearly labeled crop derivatives. The initial failures and type-only retry are retained in separate logs. The historical 170-entry trigger matrix is an audit, not 170 newly executed scenarios or Brogue pixel parity.
