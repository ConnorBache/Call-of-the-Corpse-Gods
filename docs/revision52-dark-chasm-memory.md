# Dark chasms in memory

> Superseded in part by the 2026-11-09 clarification: chasms now use ordinary visible-terrain illumination, including negative-light foreground contrast rescue. The dark Brogue base palette and knowledge-safe remembered colors remain. Historical no-rescue claims and negative-light color receipts below do not describe the current implementation. See [ordinary chasm illumination](revision52-normal-chasm-illumination.md).

## Request and cause

The designer requested: “Chasms should be dark whether you're remembering them or not.”

The observed terrain already supplies the authored Brogue-like chasm ink against black. Remembered terrain uses the deterministic, dimmed memory version of that ink, without reading current illumination or hidden terrain. The final native readability stage exempted only **current** chasms. When painting a remembered chasm it instead raised the glyph to neutral gray 64 and its background to gray 7, overriding the deliberately dark source colors.

The fix extends that existing chasm terrain exemption to every knowledge state. It does not replace the source palette or change any illumination curve. Entity ink is handled before this terrain exemption, and fire retains its existing handling. Other remembered tiles keep their readability floors. Explicit material and global/fog appearance settings remain authoritative.

## Scope and invariants

This is a detached rendering correction, with no simulation, sight, discovery, movement, hazard, RNG, save schema, command dispatch, normal/haste phase, or time-travel changes. The same final color stage serves normal observation, restored saves, repeated frames and retained/damaged-tile repainting. Hidden live terrain, light or actors never enter the remembered color decision. Unknown cells stay absent.

The existing current-chasm light response remains intact: positive illumination still exposes the dark source ink, negative illumination still dims and desaturates it, and neither current nor remembered chasm terrain receives a generic readability lift. No bright override is forced on personalized chasm palettes.

## Measured default palette

For the deterministic native regression scene, sampled at the same chasm coordinate:

| Knowledge | Live illumination | Foreground sRGB | Background sRGB | Foreground relative luminance |
|---|---:|---|---|---:|
| Visible | -100 | 6, 6, 7 | 0, 0, 0 | 0.18% |
| Visible | 0 | 23, 20, 56 | 0, 0, 0 | 0.97% |
| Visible | +100 | 78, 68, 190 | 0, 0, 0 | 9.47% |
| Remembered | any hidden illumination | 12, 11, 16 | 0, 0, 0 | 0.35% |

Visible positive-light color remains a dark blue on black; the native +100 screenshot was inspected rather than treating one blue channel's maximum as perceived brightness. The previous remembered paint override was foreground 64,64,64 and background 7,7,7. The fix retains the dim remembered source instead. All visible source values and light response remain unchanged.

## Verification

Deterministic regressions cover:

- Real directly visible and LOS-hidden remembered chasms at illumination -100, 0 and +100, native 1280×800 screenshots, exact final foreground/background preservation and native tile-pixel brightness bounds.
- Hidden terrain/light/actor changes, unknown terrain, unchanged serialized state/RNG and remembered rendering after save deserialization.
- Visible and remembered actor/item ink, fire, other terrain memory floors, custom foreground/background/opacity, remembered brightness and global exposure.
- Exact full/retained/tile renderer pixels through visible-to-remembered and remembered-to-visible transitions, plus repeated presentation times.
- Existing chasm palette/light calibration and compiled-ground/reference tests.

The combined native desktop test binary (compile 16) passed all 15 chasm-filter tests, 11 illumination tests, both compiled-ground/reference oracles and the ordinary remembered-geometry readability test. The separate isolated SDL/GPU calibration test was explicitly ignored in this focused run. These counts overlap on two chasm tests.

The regression was demonstrated failing before the fix, with the exact unwanted gray foreground/background. Native screenshots were then captured and visually reviewed after the fix. All six existing visible scene color records **and full-frame RGBA CRCs** are identical before and after, including +100. The remembered chasm's tile RGBA CRC is identical at hidden illumination -100, 0 and +100. The +100 before/after remembered-frame difference is confined to the chasm region (43,216 changed native pixels, bounding box x=798..899, y=186..642).

Logs, native captures, exact color values, pixel-comparison receipt and patch are in [verification/r52-dark-chasm-memory](../verification/r52-dark-chasm-memory/). The final workspace aggregate remains the integration owner's responsibility. Linux native software rendering is not Windows runtime or hardware-GPU certification.

## Complexity

Let C be detached cells painted. The changed predicate is O(1) per cell, with fixed three-channel data and no allocation, cache, world lookup or visibility computation. The full stage remains O(C) with O(1) extra memory. Existing rendering cost remains proportional to covered pixels and existing glyph/cache-miss work; the fix adds no geometry, masks, glyphs or raster passes. No new algorithm or performance measurement is claimed.
