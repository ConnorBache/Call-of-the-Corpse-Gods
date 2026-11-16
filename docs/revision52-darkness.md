# Revision 52: Darkness sets a radius-3 circle to -100 light

## Authored behavior

The designer instructed “Have darkness turn the darkened area into -100 light
immediately,” then “Darkness, instead of being a 3x3 square, should be a radius 3
circle.” The effect takes hold during potion application, before observation or
animation. R52 has an exact effective-light override of -100 in a plane-local
Euclidean radius-3 circle: cell-center offsets satisfy dx² + dy² <= 9. Its 29
cells span seven cells along either axis, including (±3,0), (0,±3) and (±2,±2),
but excluding (±3,±1) and (±3,±3). Floor bounds clip this mask. Walls do not
reshape it, and it does not spread through portals. Thrown potions still use
the existing physical projectile landing coordinate before blocking terrain;
the resulting area includes walls and cells behind them without granting sight.
The previous implemented footprint was 5×5, although the latest instruction
described it as 3×3; the requested radius 3 is applied literally. Ordinary duration remains
20 normal world ticks; dilution retains the same footprint and -100 level for
10 ticks. Haste/free actions do not independently advance expiry. Overlaps keep
one entry per cell and the latest expiry; neither magnitude nor durations add.

All existing and later light sources, including ambient, fire, fixed torches,
passive Shine and mobile/carried Light rings, remain suppressed inside the active
mask. Light sources are not removed, clamped, or frozen. They can continue lighting
unaffected cells, moving, turning off and appearing. At expiry, effective light
is recomputed from the current sources rather than restoring a stale screenshot
or the pre-potion source values. Expired entries are ignored by read-only queries
even before the ordinary active/inactive-floor cleanup runs.

Tile illumination and actor perception use the same override. A Darkness Aura
actor cannot regain light from a carried ring inside the mask. Actor shadow and
Stealth modifiers retain their existing rules and cannot raise the -100 floor.
The R51 literal sight formula is not changed: P=100 still gives tile range 20
and creature range 10 at -100 (before Stealth and optical blockers). Darkness is
not a separate opaque-vision flag.

## Compatibility and presentation

The existing `revision50Darkness` sparse data remains unchanged. Its `amount`
field preserves dose metadata (50 ordinary, 25 diluted); R52 interprets any
live entry as the absolute mask rather than subtracting that amount. R50/R51
save, replay, item-description and cache behavior stays additive/decremental,
with the exact historical 5×5 square. Existing saved masks remain explicit cell
lists: loading never expands, moves or reshapes an already-applied dose. This is
a follow-up within the unreleased R52 candidate, so replaying R52 dose commands
now creates circles; the superseded interim R52 square candidate is not a frozen
replay target. Shipped revisions through R51 retain their exact original commands.
The cache dependency explicitly includes the new semantic rule, so a same-tick
revision change cannot reuse a field built under different rules. Admission and
new-run selection are revisioned centrally; old runs are never silently upgraded.

Serialization, snapshots, See The Future capture/return and inactive-floor catchup
retain the same floor data and absolute world-tick expiry. Remember's passive
ability acquisition does not own or rewrite floor masks. Copying or activating
an emitter after the dose still goes through the effective-light queries.

Detached observation emits `potion-darkness-override` only for currently observed
cells in the selected normal/dream plane, with redacted origin and no actor ID.
It does not reveal unseen or remembered cells. Rendering preserves the existing
distinction between detached ambient and effective light, and grades visible cells
using the existing
-100 sRGB calibration (5% neutral multiplier), not an invented pure-black palette.
The marker suppresses positive visual glow and prevents custom ambient/source
controls from lifting the masked cells. This is presentation only and does not
change the literal perception formula or saved source values.

## Complexity and ownership

Let V be floor cells, D live/stored darkness entries (D <= V per plane), A actors,
I carried-item entries, B ability entries, K portal endpoints, R maximum optical
ray length, G distinct carried-ring owner groups and Fp detached light-footprint
entries. These are source-derived bounds, not measured latency:

- R52 application scans 49 offset candidates and keeps at most 29 cells, for
  O(49 + 29D) time and O(29) scratch using the existing unique-cell search. R50/R51
  retain O(25D), O(25) scratch. Expiry remains O(D).
- Scalar illumination adds an O(D) live-mask check (O(1) extra scratch). A covered
  tile returns immediately; uncovered queries retain their existing emitter/ray
  and ring-geometry costs. Scalar ring contribution adds O(floors + D) before its
  existing O(AI + G*(floors + K²R)) work. Actor-local aura bypass uses that same
  suppressed bonus. No world scan is added to the detached UI modifier helper.
- Cache capture adds one semantic boolean to exact dependency equality. Existing
  O(V + AB + sources + D + ring-route geometry) capture and dirty rebuild remain;
  the final override adds O(D) time and O(1) extra scratch, updating the normal,
  source-excluded and ring-bonus fields. Queries remain O(1). Cache holds at most
  two plane fields and all existing optical/source-dependent costs still apply.
- Observation keeps O(D) mask traversal/output and current-cell/plane admission.
  Appearance-light construction adds one O(V) boolean buffer initialized in O(V)
  and constant work per mask entry and final cell. Existing O(V + Fp + sources)
  rendering work and expanded-radius conservative rays are unchanged.
- Software pixel tests perform fixed-size native raster/font work and O(P) full
  framebuffer comparisons, where P is framebuffer pixels. They are not a new
  gameplay/render-loop cost. No timing or Windows/SDL certification is inferred.

## Regression coverage and execution status

Radius-3 verification passed 17/17 core tests selected by `darkness`, 7/7 native
tests selected by `darkness`, and the separate unbounded-display guard (1/1).
Actual P0 before/after/expiry and high-Perception after PNGs were inspected. Exact
binary hashes, source-file hashes, logs and screenshots are recorded in
`verification/r52-darkness-circle/final-receipt.json`. New deterministic checks cover all cells for
full/diluted doses at interior and clipped-edge centers, inclusive axial and
diagonal boundaries, legacy squares, actual wall-intercepted throws, unchanged
terrain, hidden-cell admission, saved cell lists and exact expiry. Native high-P
and P=0 tests now require the circular mask and its boundary pixels. New evidence
is stored separately under `verification/r52-darkness-circle/`; previous
5×5 receipts and Space images remain historical and are not current circle proof.


Core tests cover ordinary/diluted drink and throw, normal/haste dispatch phase,
raw ambient outside [-100,100], old-revision behavior, exact footprint and edge
clipping, existing/new/mobile lights, ring/Aura behavior, source preservation,
overlap clocks, exact expiry, same-tick cache invalidation, literal Perception,
current-only redacted observation, normal/dream selection, save/load, snapshot,
Future restore, inactive-floor expiry, replay/restart and truthful Pack text.

Native tests use the real potion command and detached observation to verify
100 -> -100 -> 100, immediately changed affected pixels, unchanged outside
pixels, the exact existing -100 multiplier, and full/retained/tile repaint
agreement. Separate materials and custom-profile tests check sources, dilution
and no saved-state/RNG mutation. Screenshots are native frames/crops, not mockups.

Before the radius-3 instruction, the focused cloud run passed 14/14 core tests selected by `darkness`,
including all nine new R52 regressions and retained potion/perception/plane checks.
The desktop `darkness` selection passed 7/7, and the separate unbounded detached-
illumination guard passed 1/1. Actual high-P and P=0 native screenshots were
visually inspected. Logs, exact binary hashes, screenshot hashes and the measured
P=0 evidence are recorded in `verification/r52-darkness/final-receipt.json`.
A targeted pass does not imply a full workspace pass, Windows runtime verification,
or SDL input-loop coverage. No user PC was accessed.

### Ordinary zero-Perception presentation

A separate native scene uses the normal initial P=0 rather than the high-P
lighting diagnostic. Before the dose, +100 light makes the known patch currently
visible. After the dose, the literal formula makes those -100 cells unavailable
to direct sight: they retain only their dim remembered-map appearance. No current
illumination, ambient value or live mask is exported for them. The test captures
before/after/expiry, checks a visible center-pixel change and darker mean RGB,
unchanged outside pixels, retained/full repaint equality, and expiry restoration.
The native test passed and its actual frames were inspected. The affected center
mean RGB fell from 109.506410 to 16.576603, while outside pixels stayed byte-identical.
The radius-3 patch is clearly dark in ordinary play; its remembered gray dots and
the player glyph remain readable. The same-scene native test verifies all 29 cells,
including axial tips, and keeps just-outside (3,1) cells byte-identical. Existing
rectangular on-screen cells make the tile-coordinate circle vertically stretched;
no map aspect-ratio or layout change was made. The high-P diagnostic is nearer black because it
uses current -100 illumination rather than remembered-map styling. Both paths
restore the original terrain pixels at expiry. The JSON receipt and native PNGs
are under `verification/r52-darkness-circle/native/`.

### Raw source values versus the inherited observation range

The retained gameplay observation bounds its ambient sample to [-100,100], and
scalar/cached effective gameplay lighting already has that range. A raw saved
floor light of 250 therefore remains 250 in the source but is reported as 100
in the existing ambient observation; R52 does not introduce that bound. The
new extreme-source test originally expected the raw number in that bounded field.
Its expectation is corrected while independent assertions still require unchanged
floor lights and torch powers. The existing detached-input rendering regression
exercises -150 through 250 and ensures the display function itself retains its
unbounded tails. No gameplay range or rendering-tail change is part of Darkness.

The unchanged observation bound is independently recorded from preserved R51
`crates/corpse-core/src/vision.rs:956-958`, including its file SHA-256 and exact
expression in `verification/r52-darkness/raw-light-contract.json`. The corrected
extreme-source test and the independent >100 display guard both passed; no
production change was made to satisfy the mistaken expectation.
