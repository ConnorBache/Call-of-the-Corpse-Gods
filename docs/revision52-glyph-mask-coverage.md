# Native glyph mask coverage correction

The default glyphs keep their original characters, fonts, colors and alignment conventions. Printable single-character Brogue ASCII continues through the unchanged 128×232 atlas coverage path. The already-repaired middle-dot ember continues to rasterize directly at its final native font size.

Other glyph identities now rasterize their existing font outlines at four times the previous device-relative source resolution, then area-average alpha coverage to the final native mask. This replaces nearest-neighbor pixel selection. The extra source coverage prevents small tree-tap/pit/arrow masks from being enlarged into visible source-pixel blocks; coverage integration preserves thin fire/root/entrance/chasm strokes instead of skipping them. The same path also serves tree/Veridian, wards, Borge altar and custom compound identities. Tight ink fitting, per-cell padding and centering remain, with small rounding differences as the outline's bounds become more accurate. No new artwork or simulation rules are introduced.

The final mask is cached and shared by software tinting and GPU recording. There is no bilinear texture scaling at presentation time and no low-resolution full-frame enlargement. Clipping, retained/tile repaint, cache reuse, opacity and font selection keep their existing contracts. This does not change the separate chasm palette or animation-style changes.

## Complexity

Let G be glyph characters, E the total font-outline complexity, S the total temporary supersampled coverage pixels (individual glyph bitmaps plus the assembled mask), B the final native ink mask pixel count, and K the existing identity cache size. A cold miss costs O(G + E + S + B) for the normal uniformly downsampled glyphs, including fontdue native raster work, mask assembly/bounds scans and area integration. Fourfold sampling in each source dimension increases the source-area constant approximately sixteenfold versus the previous mask; this is not a free quality change. Retained cache memory remains O(B) per key, at most 512 keys. Cold temporary memory is O(G + S + B), plus fontdue raster working storage. Warm frames retain the same expected O(key bytes) lookup and O(clipped B) tint/blend; GPU recording/upload follows existing cache behavior. Eviction/resize can release O(sum of retained mask bytes). The general resampling helper supports arbitrary mixed-axis sizes and costs O((source_width + output_width) * (source_height + output_height)), not universally O(S+B). Authored ordinary glyphs are downsampled; extremely narrow custom punctuation can still select an upsampled fit.

These are algorithmic bounds, not hardware timing guarantees. Focused tests measure correctness at fractional/native scales and against retained/tile/GPU-mask paths. The final combined candidate still needs the full workspace and Windows build gates; isolated painter checks do not certify physical GPU or monitor transitions.

## Verification

`render::glyph_mask::tests` checks exact coverage integration, crop boundaries, fractional/mixed-axis resize symmetry, solid coverage and no seams. `render::glyph_mask_tests` exercises the identified symbols and compounds, six native scales, three cell sizes, default/IBM font selection, cache reuse, GPU mask equality, clip behavior and full/retained/tile frame equality without simulation mutation. Existing ember, native-resolution, appearance and GPU regressions are rerun separately.

Set `CORPSE_GLYPH_COVERAGE_OUTPUT` to a directory and run the glyph mask tests to write native 1280×800, 1920×1200 and 2560×1600 comparison boards. The board deliberately includes the historical Epidite missing-character example as a diagnostic row; it does not reintroduce that removed animation path.
