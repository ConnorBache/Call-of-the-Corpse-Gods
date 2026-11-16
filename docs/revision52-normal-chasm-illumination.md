# Ordinary illumination for chasms

## Current design

The designer's latest clarification is: “Chasms shouldn't be artificially dark in illumination.” This replaces the earlier no-contrast-rescue requirement, not the authored Brogue base color or the knowledge boundary.

The audit found one chasm-specific suppression inside detached illumination finishing: `quiet_chasm` skipped the standard negative-light foreground contrast rescue for bare current chasm terrain. Positive illumination already followed the shared exposure and source-tint curve. The suppression is removed. Visible chasms now receive the same exposure, negative desaturation and negative-light foreground rescue as ordinary terrain, starting from their existing authored color.

The source palette still uses the static Brogue tuple `(7,7,15,4,4,8,0,false)` against black. Neutral chasm ink is not raised to the generic terrain readability color. Remembered chasms retain the dim remembered version of that source ink rather than being raised to gray. Like other remembered terrain, they do not respond to hidden current illumination. Decorative boundaries keep their existing source palette, alpha and geometry; they already receive the shared foreground illumination transform. Actors, items, fire, custom profiles and other terrain retain their existing paths.

This change does not add light sources or modify illumination values, perception, sight/line-of-effect, discovery, movement, falls, simulation RNG, saves, normal/haste phases or time travel. It consumes only detached presentation data. The tests exercise loaded state and verify byte-identical serialized Run data before and after observation/painting.

## Verification

The focused native tests compare chasm and ordinary-terrain illumination from identical source ink at twelve illumination levels, including unbounded tails, with neutral and warm tint, for visible and remembered knowledge. Additional scene tests check the source palette, calibrated foreground and boundary output, black background, custom controls, unchanged entity/fire readability and idempotent finishing/restoration.

Real direct-LOS and wall-occluded remembered scenes are captured by the native 1280×800 software renderer. Existing regressions check hidden terrain/light/actor changes, unknown cells, save deserialization, exact full/retained/tile pixels, light/visibility transitions and repeated timestamps. Compiled-ground reference and ordinary remembered-geometry tests provide adjacent coverage.

The focused run passed all 16 chasm-filter tests, 11 illumination tests, both compiled-ground/reference oracles and the ordinary remembered-geometry test. Two illumination cases overlap the chasm filter. The existing isolated GPU test was ignored. The new identical-source comparison failed against the old implementation before the change and passed afterward.

The same sampled visible chasm changes from `(6,6,7)` to `(88,88,102)` at illumination -100 through ordinary terrain rescue. Its 0/+50/+100 full frames are byte-identical before and after. Negative-light differences are confined to existing chasm glyph pixels; backgrounds and edges are unchanged. Remembered ink stays `(12,11,16)` against black, with identical tile CRC across hidden illumination -100/0/+100. Native visible -100/+100 and remembered +100 captures were visually reviewed.

Exact executed counts, source hashes, captures and before/after color comparisons are recorded under [verification/r52-normal-chasm-illumination](../verification/r52-normal-chasm-illumination/). These focused results are not a final workspace, Windows-runtime or hardware-GPU certification.

## Complexity

For C detached cells, A animation anchors and U underlays, finishing remains O(C + A + U) per observation/rebuild. Each visible negative-light chasm now runs the same fixed RGB arithmetic and bounded 12-step contrast-gain search as other terrain, with O(1) extra scratch. No new cache, stored field, allocation, world query, visibility computation, RNG use, glyph mask or raster pass is introduced. Existing covered-pixel and cold glyph-cache costs are unchanged. These are analytic bounds, not benchmark measurements.
