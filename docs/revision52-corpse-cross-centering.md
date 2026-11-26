# Corpse cross centered on native glyph ink

Latest designer correction: “The X needs to be centered on the glyph.” This follows the larger red X and retained living-color change. It does not change those choices.

## Rendering contract

The old marker used the middle of the map cell. That is not the middle of its creature glyph: ASCII atlas masks have different ink margins, compound glyphs are deliberately lifted, and typography can move and resize the ink. The correction attaches the X to the midpoint of the actual glyph's nonzero native raster bounds, after font selection, source coverage integration, typography fit, offsets, emphasis and device-pixel rounding. For multi-character glyphs the bounds cover the complete painted string, not its first character, advance width or an approximate character cell. The existing proportional serif fallback for Ω is included.

The underlying creature glyph paint returns its geometry to its caller. Both map and effect repaint call the same marker with geometry from their glyph paint, so there is no second typography calculation, font metric approximation, framebuffer color scan or simulation lookup. The returned bounds are native half-open pixel-area bounds: the center can fall on an integer or half-integer native pixel coordinate.

Bounds ignore RGB and opacity. A black Dire Bear glyph still has the same attachment center. Its existing identity outline is painted unchanged and does not expand or move the attachment bounds. Underlays, backgrounds, rank/superscript/subscript marks, selection, and heartless rings never contribute to the glyph bounds. Actual compound characters remain one authored glyph string. The metadata is computed before viewport, damage and tile clipping; clipping a fragment must not pull the X toward the surviving part. The X itself remains clipped to the cell and existing raster/damage bounds. If a saved custom glyph is genuinely blank or inkless, the previous tile-centered X is retained so death indication is not silently removed.

On a harvested corpse the heartless ring paints over the X at the exact same native center, including the blank-glyph fallback. Its outer diameter stays unchanged, with a wider dark center so the ring reads as an o even on narrow map tiles. All four red arms stay visible. Living heartless markers and eligibility are unchanged. That shared coordinate adds O(1) placement work and no storage; the ring retains its existing O(P_ring × 16) time and O(1) scratch for P_ring clipped native pixels.

X endpoint span, red source color `#f53640`, antialias samples, stroke and outline width, local light/dream treatment and opacity are unchanged. Glyph mask bytes and positions are unchanged. Existing saved appearance settings, creature color/glyph identity, privacy, hallucination projection, observation knowledge, simulation, RNG and save format are unchanged.

## Complexity and storage

Let M be the final native glyph-mask area, H its rows, R its maximal nonzero runs, G the UTF-8 glyph key length, C the number of cached masks and P the clipped native marker bounding area.

- On ASCII cache insertion, the existing O(M) row-run construction is unchanged. An additional O(H + R) pass computes ordinary and italic ink bounds from those same runs; it does not scan another bitmap or rasterize another font. It adds two constant-size optional bounds per cache entry, O(C) extra metadata. Existing ASCII cache limits are unchanged (at most 513 entries before the next insertion clears it).
- On compound/fallback cache insertion, an additional O(M) scan computes bounds from the final coverage mask. Native font outline rasterization, oversampled temporary glyph raster/coverage integration and final mask ownership retain their existing costs and cache behavior. This adds one constant-size optional bound per entry, O(C) metadata, within the existing 512-entry cache limit. Source glyph length is already bounded by validated appearance data; no new persistent or world-sized cache is introduced.
- Warm painting returns translated bounds in O(1) work and O(1) scratch after its existing expected cache lookup (O(G) key hashing for compound glyphs). It does not rescan mask pixels for placement. Existing painting costs, including clipping, sparse row traversal, font raster cache misses and GPU mask copies, remain unchanged.
- Each corpse marker takes O(P × 16) time and O(1) scratch using the existing fixed 4×4 coverage samples; recording still emits O(P) existing native-pixel quads. The new center calculation is O(1). Glyph-cache metadata is independent of color, alpha, viewport and frame damage, so none become extra cache keys.

No timing improvement is claimed. This is a correctness fix, with the cold-cache costs explicitly accounted for. Full-frame, retained and tiled repaint scheduling are unchanged.

## Verification contract

Six centering regressions live in `crates/corpse-desktop/src/corpse_marker_tests.rs`, under the `corpse_center_` prefix:

1. 600 comparisons of returned bounds with an independent framebuffer-pixel oracle: 15 strings, two logical glyph sizes, four font/profile choices and five drawable sizes, including fractional DPI and letterboxed origin. It includes Brogue ASCII, IBM Plex Mono, Inconsolata/fallback, compound strings, Ω's proportional serif fallback and blank ink. Black glyphs and identity outlines do not change geometry.
2. Italic ASCII shear and bottom-aligned glyph bounds compared with actual painted pixels.
3. Red cross pixel extent and rotational symmetry around the measured glyph center. Many cases explicitly differ from the old tile-centered image.
4. Partial clipping preserves the full glyph anchor; replaying recorded native-pixel GPU commands matches software pixels; blank saved glyphs retain the tile fallback. This is command-recording validation, not hardware-GPU readback certification.
5. Real legacy/current corpse layers, including a custom `~` glyph on animated water and hallucination variants, keep terrain animations separate, so the water-wave branch cannot become an unmeasured corpse glyph. Ordinary map and effect repaint agree for ASCII, fallback and compound glyphs with default/custom profiles; the saved run remains byte-for-byte unchanged.
6. Optional native PNG captures: nine glyph boards (default/IBM/custom offsets at 1280×800, 1920×1200 and 2560×1600) and eight actual save/observation scenes (legacy revision 29/current revision 52, default/custom appearance, 1280×800/1920×1200). Each board pairs glyph-only with glyph-plus-X at two logical sizes. Scene fixtures pass ordinary save validation, round-trip the save and assert direct visibility. Scene pairs include an individually colored rat, Dryad, Hemogoblin, black-outline Dire Bear Vampire Bat and Pale Birther; custom scenes use a caret rat and the explicitly supported historical compound Pale Birther glyph. Legacy default Pale Birther also retains its authored compound glyph. Arbitrary compound overrides for other creatures remain rejected by appearance validation.

Run `CORPSE_CENTER_OUTPUT=<isolated directory> bash tools/cargo.sh test --locked --offline -p corpse-desktop corpse_center_ -- --test-threads=2` under the coordinated pinned build environment. Inspect the native PNGs rather than treating file existence as visual approval. Existing corpse identity, retained/tile repaint, glyph-coverage, ember, outline, privacy and hallucination regressions remain integration requirements. Current execution evidence and any failure logs are recorded separately under `verification/r52-corpse-center/`; final source-bound verification is owned by the release coordinator. Windows cross-compilation does not establish interactive Windows or accelerated-GPU behavior.

## Development result

The coordinated pinned Rust build passed all six focused centering tests and generated all 17 expected native PNGs (`centering-attempt3.log`, exit 0). The same final test executable also passed 35 related renderer/identity/privacy tests, including those six centering cases; one pre-existing opt-in hallucination benchmark was ignored. This is focused verification, not an aggregate workspace or platform certification. The default and IBM glyph boards, custom-offset fractional-DPI board, legacy custom scene, and current default/custom scenes were visually inspected: X intersections follow the glyph ink rather than the grid center, while colors and the Dire Bear outline remain visible. Final release verification separately re-runs and reviews its source-bound capture set.

Earlier failed development attempts are retained: the first rejected an invalid arbitrary Rat compound override; the second caught a test-only access to the remembered-tile field wrapper. The fixture was corrected to use supported appearance admission and the typed field API. No production validator, save rule or visibility gate was relaxed.
