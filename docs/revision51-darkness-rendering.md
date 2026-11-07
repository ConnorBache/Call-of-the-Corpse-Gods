# R51 potion-darkness rendering correction

> Historical first correction. The later designer-authored sRGB mapping in
> [R51 illumination calibration](revision51-illumination-rendering.md) supersedes
> the positive-light no-op behavior, glow exposure and screenshot interpretation
> below. Simulation/potion footprint and expiry invariants remain current.

The R50/R51 potion already subtracts 50 illumination in its exact 5×5 footprint
for 20 normal turns (diluted: 25 for 10). Simulation, visibility, stealth,
source ordering, ring self-exclusion, saved timers and RNG are unchanged.

The display previously received the lower scalar illumination, but no potion
footprint in its visual light-source list. Near material exposure only reacted
to **negative** illumination. A 100→50 change therefore left ordinary near
terrain pixels unchanged, and source glow retained its previous strength.
This was a presentation omission, not a failed potion or a visibility-only item.

`vision::observe_terrain` now appends a detached `potion-darkness` cell mask after
radial light footprints. It emits only current cells belonging to the selected
observation plane, no impact origin or actor identity, and no expired entries.
It does not raycast the authored square into a different footprint. Unknown and
remembered ground do not gain knowledge from this payload.

The existing visual darkness factor, `1 - amount / (positive maximum + 100)`,
clamped to 0–1, now reduces emitter glow for this source too. The same factor
also attenuates non-emissive base material exposure for potion masks, so a
positive-to-positive illumination decrease is visible. Raw ambient and resolved
illumination values retain their meanings. No general positive-illumination
ramp or baseline brightness change is introduced, and preexisting aura behavior
is preserved. Player backgrounds use the same ground light and remain exact.

## Complexity and bounds

Let V be floor cells, D be stored potion-darkness entries across observed planes
(at most 2V), F be existing detached source footprint entries, and S be existing
sources. Per terrain observation the added projection is O(D) time and O(D)
output, using O(1) indexed current/plane checks. It adds at most one detached
mask per observed plane. Existing visibility/light-cache work is unchanged.

Per appearance-light build the added scalar buffer is O(V) memory, initialized
in O(V). Mask traversal adds O(D) work and O(1) channel math per final cell.
Existing O(V + F + S) normal-radius work and expanded-radius ray work are
unchanged. Per material coloring the added multiply is O(1); native raster,
font decode/cache and framebuffer costs are unchanged. Native comparison tests
allocate full framebuffers and do O(pixel count) equality checks. These are
source-derived complexity bounds, not measured latency or Windows certification.

## Deterministic regression coverage

`darkness_native_throw_before_after_expiry_and_incremental_pixels` uses the real
throw dispatcher at revisions 50 and 51. It checks the exact 25-cell mask,
100→50 illumination, unchanged outside pixels, changed affected pixels, player
background equality, deterministic saved-state/RNG preservation, expiry and
full/retained/tile repaint equivalence. It reconstructs the old missing payload
from the actual post-dose observation to prove the old 100→50 visual no-op.
Native PNGs are emitted when `CORPSE_R51_DARKNESS_OUTPUT` is set.

`darkness_materials_glow_dilution_profiles_and_ring_exclusion` covers floor,
wall, grass, water and shallow water with default/custom materials and negative,
zero and positive ambient; source glow, dilution expiry, and Light-ring ordering.
The core `potion_darkness_projection_is_current_plane_only_and_read_only` test
covers two-plane selection, hidden masks, expired entries and serialized-state
immutability. These direct native/software-renderer tests do not claim SDL
input-loop or Windows runtime coverage.

## Execution status

The existing native appearance-lighting binary passed 12 tests, with one ignored,
including 41 frozen source cases and deterministic flicker. Both substantive
new native tests passed: actual R50/R51 throw before/after/expiry with retained
and tile repaint equivalence, and materials/dilution/glow/ring coverage.
The 21×21 fixture is supported. R50/R51 full native captures exist, and the R51
fixed frame has been visually inspected: its 5×5 darker patch is visible.

The illustrative multi-level test is corrected after revealing an incorrect
wall-palette test assumption; labeled sheets still await that corrected build
and run. The separate two-plane projection test requires the coordinated core
verification result. These are distinct from SDL or Windows runtime coverage.

## Requested illumination screenshot examples

`illumination_native_examples_same_tiles_levels_sources_and_potion_expiry`
prepares two user-facing PNGs with the same output-directory variable:

- `illumination-levels-native.png`: five terrain rows (floor, wall, grass,
  deep water, shallow water), with ambient −75, −25, 0, 50, 100 and an explicit
  equal-number pair of 80 ambient versus 80 from a real nearby torch.
- `darkness-potion-native-comparison.png`: the same terrain samples before the
  actual throw, while its darkness is active, and after expiry (100→50→100).

The test renders real 2560×1600 game frames, then copies native 3×3 or 7×3 map
crops byte-for-byte into labeled native-renderer sheets. It does not rescale,
recolor, interpolate or draw substitute tiles. Fixed seed, cell coordinates,
presentation time and center-tile direct-visibility chance isolate lighting.
HUD, inventory and log differences are outside every compared crop. Saved-state
immutability, RNG stability, center-pixel changes and exact whole-crop expiry
restoration are asserted. `native-example-evidence.json` records scalar light,
raw ambient, foreground/background RGB, dimensions and RGBA CRC32 values.

The intended evidence also makes the existing presentation distinction explicit:
negative illumination shades terrain; walls and water have illumination-specific palettes;
near default floor/grass do not become a linear brightness slider at positive ambient;
a real emitter adds colored glow beyond its resolved scalar illumination.
These screenshots are pending execution and visual inspection after the rebuild.

This test performs a fixed 50 native frame renders plus existing core projection
work, not a simulation loop. For P framebuffer pixels and K crop pixels, each
copy/check is O(K), each full native paint retains its O(P) raster cost plus
existing scene/font work, and at most a fixed number of full buffers and sample
crops are retained. Labels use the native font cache. No runtime capture or new
production rendering loop is introduced.

## Focused native execution update

The actual R50/R51 potion before/after/expiry test and the materials/dilution/ring
test passed in the coordinated native run (`verification/r51-ui/targeted-visual-2.log`).
The multi-level illustration test exposed an incorrect test assumption that
positive-ambient wall pixels would remain identical. In fact the existing
`illuminated` wall palette changes its glyph ink; native center pixels changed
from an ink peak of RGB 126/112/103 at zero ambient to 174/158/149 at ambient 50.
The test now requires a wall-pixel change, preserving floor/grass equality checks,
negative shading, same-number source differences and potion restoration checks.
The illustrative sheets still require the next rebuild/run and visual inspection.
