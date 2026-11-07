# Revision 51 water-depth presentation

The designer requested lighter blue shallow water, richer blue deep water, and a blue screen tint while in deep water. This is a native presentation change, not a new swimming, vision, damage, or turn rule.

## Materials and preferences

Native default foreground/background/edge colors are now:

- Shallow water: `#97d6f5` / `#2c5e82` / `#b7e8ff`.
- Deep water: `#417de0` / `#0a235a` / `#6ea5f7`.

The existing lighting, material variation, bank smoothing, remembered-map treatment, dissolved-blood stain, and animated tilde geometry still apply. The former identical zero-blood water override no longer collapses both depths to one palette. All directly seen water, including legacy floors without a dissolved-blood field, uses depth-specific blue illumination anchors. Authored explicit material colors/backgrounds/edges and explicit water illumination anchors remain authoritative. Frozen v48 appearance schema/default data and historical non-Flesh rendering are unchanged.

## Immersion and rendering

The detached `View.deep_water` value reads the existing canonical `environment::is_swimming(run, &run.player)` query. It does not infer depth from appearance glyphs, hover, selection, or stale `underwater` status. It excludes shallow water, Ichor, dead bodies, temporary flight, Phase Through, Ascend, Ring of Flight, Dream-crafted flight, and Dreamwalking flight according to existing gameplay rules. Only the current player is the viewpoint; a wet Half Here clone or sleeping body cannot tint a dry or flying player. Terrain contacts retain the canonical simulation's existing interpretation, including any existing bridge/ice rules; no independent immersion mechanics were invented.

The last gameplay-base primitive is a 34/255 blue `[20,78,170]` alpha veil. More than 86% of foreground/background color differences survive, keeping text, glyphs, and resource information legible. The veil does not alter the observation colors or saved world. It is not applied to menus or separately composed modal/hover overlays. No persistent Painter tint setting can leak into another screen.

Both retained and tile renderers invalidate the whole frame when immersion changes, including the HUD/footer. Each damaged region is cleared/rebuilt and then tinted once, preventing repeated alpha buildup. Animation repaint, entry/exit, flight, loading another world, and viewport resizing use the same primitive. GPU command recording emits the same clipped alpha quad and does not software-paint the framebuffer.

## Complexity

Let F be stored floors, K the fixed ability catalog size, A/P the player's ability/active-ID counts, I the inventory count, E the largest enhancement-list length, H native framebuffer height, S selected tile-row spans, and Q selected native pixels. Each observation adds one canonical swimming query: floor lookup plus existing flight resolution, conservatively O(F + K(A+P) + I(E+1)) time and O(1) extra space, excluding existing shared catalog initialization. Typical common ground/flight paths return earlier. No visibility scan, new cache, RNG draw, or save field is added. The View and retained Scene each add one boolean.

Each wet software base paint adds O(H + S + Q) alpha raster work and O(1) scratch, through existing clip/span iterators. Full paints have Q = width × height. A transition adds O(1) invalidation but necessarily causes the existing full-frame descriptor/raster/upload work; this is not claimed O(1) rendering. A steady retained wet frame touches only selected damaged pixels; a fully unchanged frame does no paint. GPU command recording adds one O(1) quad (existing batch storage) and O(Q) fragment blending. No new glyph rasterization or mask-cache misses are introduced by the veil. Measured timings are not claimed.

## Verification

`water_render_tests` covers read-only serialized state/RNG behavior, actual depth versus glyph/hover/status, Ichor, death, temporary flight, Phase Through, Ascend, Ring of Flight, flying dream viewpoint versus sleeping body, three native sizes, real Engine save/load recomputation, retained/tile equivalence across entry/exit/reload/flight/animation, unchanged menu pixels, fixed alpha/readability bounds, and GPU command reset. `water_depth_native_gpu_pixel_parity` is an isolated ignored SDL test for actual offscreen GPU raster comparison. Existing compiled/reference ground oracles cover both native material adapters and custom appearance profiles.

Set `CORPSE_R51_WATER_OUTPUT` to export 1280×800, 1920×1080, and 960×1200 native screenshots. Palette test “before” captures reconstruct the previous zero-blood appearance at 100 illumination through explicit profile overrides. Separate `*-binary-before-{shore,deep}.png` captures use the actual pre-change executable and the saved valid `shore.json` / `deep.json` fixtures. Veil “before” captures disable only the detached immersion flag at the same player position; all are rendered natively, not image-edited mockups. These images verify native rendering in the cloud, not Windows desktop/display behavior.
