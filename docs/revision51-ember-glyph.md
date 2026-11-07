# Revision 51 ember glyph raster repair

The authored terrain `feature:embers` remains U+00B7 MIDDLE DOT (`·`), with the same appearance color, overrides and font fallback order. The former general non-ASCII identity fitter cropped the dot's few raster pixels to its ink bounds, then nearest-neighbor enlarged that tiny bitmap to fill the cell. This made the ember an oversized block rather than the authored punctuation mark.

The dot now uses the selected existing font outline at its native cell-relative font size (17 at a 15 × 24 cell), centered in the same cell. Its alpha mask is not scaled after rasterization. Other symbols and compound creature identities retain their existing fitter. No new artwork, fire mechanic, animation, visibility access, or color is introduced. Custom ember symbols continue through their existing symbol path. Software and GPU share the same cached native mask.

## Complexity

Let E be the fixed middle-dot outline edge count, B its native ink bitmap area, and K the existing bounded identity-cache size (at most 512 entries). Per first use at a font/cell/native-scale cache key, the dot costs O(E + B) raster work and O(B) temporary and cached bytes. It skips the prior source-mask scan and nearest-neighbor fit. Warm paint is expected O(1) hash lookup for the one-character key plus O(clipped B) tint/blend; GPU command recording uses one cached quad with its existing cold O(B) upload and atlas memory cost. Evicting the existing bounded cache on a miss still releases O(total cached mask bytes). Resize invalidation and framebuffer/upload costs are unchanged. There is no per-frame font construction or Run access.

## Regressions and captures

`render::ember_glyph_tests` checks exact native font-mask equality at 0.75×, 1×, 1.25×, 1.5×, 2× and 3× drawable scales, five tile sizes, the default Inconsolata and selected IBM faces, small centered ink geometry, antialiasing, warm-cache stability, clipping, shared GPU mask geometry, visible/remembered observations, custom glyphs and read-only simulation.

Set `CORPSE_R51_EMBER_OUTPUT` to an output directory and run the desktop test binary with `ember_glyph_tests --test-threads=1` to capture full rendered scenes and native-size glyph detail boards at 1280×800, 1600×1000, 1920×1080 and 2560×1600. These are headless raster tests, not physical Windows monitor/DPI-transition verification.
