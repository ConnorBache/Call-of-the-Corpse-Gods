# R51 displayed illumination calibration

This presentation specification supersedes the older positive-light behavior in
`revision51-darkness-rendering.md`. It changes no simulation, visibility,
perception, line-of-sight, line-of-effect, source ordering, ring exclusion, save
format, turn clock, or gameplay RNG.

## Display transform

The nine displayed-sRGB multipliers are exactly:

| Illumination | -100 | -75 | -50 | -25 | 0 | 25 | 50 | 75 | 100 |
|---|---|---|---|---|---|---|---|---|---|
| Multiplier | .05 | .20 | .40 | .65 | 1 | 1.50 | 2.05 | 2.70 | 3.40 |

The fixed eight intervals use monotone cubic Hermite interpolation with harmonic
interior tangents. This is continuous with continuous first derivatives, hits
every authored knot, and has no overshoot. Endpoint tangents are .005 and .029.
Below -100 the display multiplier is `.05 * exp((L + 100) / 10)`; above 100 it is
`3.4 + 1.45 * ln(1 + (L - 100) / 50)`. Examples: -150 gives .00033689735;
150 gives 4.40506341181; 200 gives 4.99298781857. The upper derivative decreases
continuously. These are display functions only; raw illumination remains intact.

For the reference background [34,28,32], the nine rounded outputs are exactly
[2,1,2], [7,6,6], [14,11,13], [22,18,21], [34,28,32], [51,42,48],
[70,57,66], [92,76,86], [116,95,109]. A common channel scale handles colors
that approach the display gamut, retaining their hue rather than separately
clipping channels to white. The common peak has a smooth rational shoulder above
`k = max(220, original peak)`: `255 - (255-k)/(1+(target-k)/(255-k))`.
It is identity below the knee and has derivative1 at the knee. High custom
materials retain highlight headroom instead of flattening across75/100. The
reference floor never enters this shoulder and all nine targets remain exact.
Very bright glyphs ultimately reach8-bit gamut limits; not every foreground
channel can keep increasing indefinitely. Neutral0 remains byte-exact even for
original colors above the knee.

## Pipeline and hue

The native material pipeline first constructs its existing zero-light appearance,
including authored palettes, material customization, deterministic noise, blood,
water depth, glyph alpha, and native readability. The old exposure/glow and
light-dependent wall/water ramps are neutralized for this path so brightness is
not applied twice. A single final pass grades completed detached terrain and
entity cells, their backgrounds, edges, underlays, and environmental animation
palettes. Native full painting, retained painting, tile damage, and GPU commands
consume those same finished colors. Late ordinary-terrain contrast floors cannot
brighten magical darkness again. Important directly visible symbols may receive
minimum contrast. The player retains the actual terrain background beneath it.

Ambient and unedited neutral sources use [255,255,255]. Unedited torch, brazier,
fire and burning-liquid source hues use [255,180,100]. Existing explicit source
color edits remain authoritative. Disclosed source contributions determine a
separate weighted hue; none adds an independent exposure on top of scalar light.
The hue adjustment multiplies each material channel by 65% white plus 35% source
hue, then normalizes linear-sRGB luminance to the neutral counterpart before
rounding. This retains green/blue materials and approximately equal perceived
brightness at the same illumination. When tint would leave the gamut, its amount is reduced toward the neutral
material at the same luminance, rather than dimming the result. Unknown or remembered cells do not acquire live illumination knowledge.

Explicit source intensity, glow, flicker, radius, falloff, and floor/wall response
edits can intentionally depart from the reference curve. Their visual source
maximum is compared with the unedited native maximum to produce a separate
display-only scalar adjustment. Defaults give exactly zero adjustment, and
neutral sources retain opt-in visible flicker. Raw illumination is never changed.

The frozen source-v48 color adapters retain explicit legacy lights (`display:
None`). Historical fixture JSON is not rewritten to pretend that the old
rendering already matched the new specification. Old potion tests now assert
that a 100-to-50 scalar change is sufficient even without a darkness hue mask.

## Validation and scope

`light_ui` unit tests check all nine multipliers and exact reference RGB outputs,
monotonicity, derivative continuity at every knot and both tails, diminishing
upper gains, hue retention, and equal-luminance warm tint. Native tests construct
real R51 rooms with identical seed, tile positions, player position and animation
time, with Perception 300 keeping the sampled tiles directly visible at -100 as
per the unchanged R51 formula. Five material rows cover floor, wall, grass, deep
water and shallow water at all nine levels, plus neutral80 and real nearby torch80.
The latter extra neutral sample makes the equal-number comparison explicit.

The screenshot sheet copies 3-by-3 crops from native 1280-by-800 game frames
without rescaling or recoloring. Full frames and machine-readable RGB/CRC32
records accompany it. Each scene checks saved-state/RNG immutability and exact
full/retained/tile pixel agreement. The isolated SDL GPU test checks all55 scenes
against software pixels with the existing maximum three-channel-value blending
rounding tolerance. Linux offscreen/software-GL evidence is not Windows or
hardware-GPU certification. Execution status must be read from the accompanying
verification output; test existence alone is not a pass claim.

## Complexity and bounds

Let V be observation-grid cells, F disclosed source-footprint entries, S sources,
C finished cells, U underlays and A fixed animation-palette anchors. The hue
accumulator adds O(V) initialization/memory and O(F) fixed-channel work alongside
the existing light-field build. Source-family grouping and any expanded-radius
conservative rays retain their existing O(V+F+S) ordinary-radius and
O(S*V*max(width,height)) expanded-radius worst-case costs. The final pass is
O(C+U+A) time, fixed arithmetic per channel, with O(1) transient scratch. Each
cell retains one optional display transform and the exact source ink/overlay
needed for animated reconstruction, O(C) extra retained memory. It does not
store a second framebuffer or query visibility while painting.

Full native painting and GPU mask cache misses retain their existing raster/font
costs. For P framebuffer pixels, native comparison/equality and GPU readback
validation are O(P) per frame. The screenshot suite has a fixed55 scenes and
copies only native crop pixels. Symbolic bounds are source-derived, not latency
or hardware-performance measurements.

The blue swimming veil retains its existing34/255 alpha and86.7% of pixel
contrast, but its color follows the already-observed player display light.
At-100 its source is [1,4,9], so it cannot relight dark map pixels blue. Its
eligibility and flight/phase/Dreamwalking exclusions are unchanged. The veil
stays stable between world observations rather than flickering the entire UI.
A changed veil color invalidates the entire retained/tile frame even when the
player remains swimming. The detached color lookup is O(C) per paint/scene
comparison with O(1) scratch; it reads no live or hidden world state.
