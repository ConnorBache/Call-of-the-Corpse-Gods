# Chasm color and contrast correction

> Superseded in part by the 2026-11-09 clarification: chasms now use ordinary visible-terrain illumination, including negative-light foreground contrast rescue. The dark Brogue base palette and knowledge-safe remembered colors remain. Historical no-rescue claims and negative-light color receipts below do not describe the current implementation. See [ordinary chasm illumination](revision52-normal-chasm-illumination.md).

## Source and cause

The previous chasm source palette was already correct: Brogue's `chasmForeColor` is `(7,7,15,4,4,8,0,false)`, with percentage channels and static additive variation. The adapter retains that tuple and the existing deterministic cosmetic coordinate hashes, giving red/green 18–28 and blue 38–59 in 8-bit sRGB at neutral light, against black. The source links and original-versus-CE midpoint choice are recorded in [r52-negative-illumination.md](r52-negative-illumination.md).

The conspicuous appearance came later in CotCG's presentation pipeline:

1. Negative-light finishing lifted all terrain glyphs to 3:1 foreground contrast, including deliberately dim chasm ink. Darkness could therefore make chasms brighter than neutral light.
2. Boundary strokes passed through the generic important-symbol color transform. Its translucent-ink contrast floor whitened the same source blue-black color before the boundary was composited and illuminated.

The correction excludes only current bare chasm glyphs from that negative-light contrast rescue. Creatures, items and fire at the same location keep their normal rescue. Ordinary terrain, pits and walls are unchanged. Exposure, source tint, negative desaturation, visibility and all simulation illumination mechanics are unchanged. In the default palette, boundary strokes now use the same source ink without the important-symbol whitening. Stroke locations, alpha, width, glyphs and passability do not change. Explicit chasm color/background or grading edits retain the previous boundary transformation path.

## Native color evidence

Measured from the same detached scene, seed, tile and presentation time:

| Illumination | Glyph before | Glyph after | Chasm-side boundary before | Boundary after |
|---|---|---|---|---|
| -100 | 88,88,102 | 6,6,7 | 13,13,14 | 4,4,4 |
| -25 | 81,77,172 | 17,16,36 | 40,39,50 | 11,9,22 |
| 0 | 23,20,56 | 23,20,56 | 53,52,70 | 14,12,35 |
| 100 | 78,68,190 | 78,68,190 | 176,172,232 | 48,41,119 |

These are source/render color values, not a Brogue pixel-parity claim. The native glyph remains CotCG's existing `⛶`. Native 1280×800 software-renderer screenshots were captured before and after at -100, -50, -25, 0, 50 and 100. Positive-light glyph calibration is byte-identical in the sample; only its boundary becomes quieter.

## Verification and limits

Focused tests cover static source ranges, black backgrounds, source-colored boundaries, every established light knot, unbounded detached light, actor/item/fire readability, explicit appearance overrides, finishing idempotence, source-ink restoration, unchanged serialized Run/RNG, remembered/hidden information, and exact full/retained/tile pixels at repeated times and light transitions. The existing compiled-ground/reference oracle includes chasms, legacy palette, multiple profiles and animated times.

The disposable Linux harness has an explicitly test-only `enemy_damage_revision` compatibility bridge for an unrelated source-recovery gap. The bridge is not in this patch. Focused results do not certify the incomplete recovered workspace, Windows runtime, physical GPU, or final integrated build. No publication or desktop changes were made.

## Complexity

Let C be detached cells, A animation anchors and U underlays. Existing finishing remains O(C+A+U). Fixed profile-default checks occur once per terrain rebuild. Each of at most four existing boundary edges per cell uses O(1) channel math, and a bare chasm skips the existing fixed 12-step contrast search. Extra memory is O(1), with no persistent cache, additional world/visibility queries, simulation RNG, new glyph work, or extra raster geometry. Pixel coverage and glyph cache-miss costs are unchanged. These are analytic bounds, not measured performance results.
