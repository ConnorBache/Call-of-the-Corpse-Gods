# Fixed-page native bestiary

The manual presents every authored appearance, alternate form, mechanic, cost and appearance source on one 1280 x 800 logical page. The stat matrix is exactly HP, Energy, Accuracy, Offense on its first row and Defense, Dodge, DR, Perception on its second. Missing character-dependent values say Varies rather than inventing numbers. There is no content scrolling. Arrow keys, wheel and mouse controls browse entries; depth and category controls choose the roster.

Presentation only: catalog descriptions and mechanical text are preserved, without generated narrative. The page has three asymmetric columns: a 400-pixel art/stat column selected by a deterministic hash of creature ID and family, and two 292-pixel text columns with 288-pixel wrapping bounds. Headlines use 42 to 88 logical pixels according to name length, with creature-specific reversed-ink or acid-underline treatments. Font-metric wrapping tries sizes 20 through 16 logical pixels and chooses a balanced split, preferring section boundaries whenever both columns fit within 570 pixels. It never truncates content. Alternate forms omit only identical complete leading sentences already displayed in the base appearance; the exact original descriptions are losslessly reconstructed in tests. The exact boilerplate source sentences listing every biome or merely saying roaming groups are omitted; actual depth and unique conditional/source details remain. Fast and Slow headings omit their explanatory parenthetical only when the exact authored effect sentence already supplies that information; effect sentences and costs remain unchanged. The regression validates every one of the 81 entries against that fixed budget and reconstructs descriptions, forms, mechanics and sources to check preservation. Future catalog additions must pass that test.

Each of the forty depths has a distinct paper palette. Five descending print treatments progress from aged yellow paper through red, violet, charcoal and near-black, with increasingly heavy print rules. The creature-specific signature sets art placement and headline treatment. The animated scenery is painted before illustrations and type; opaque art pixels remain foreground, while transparent cutout areas reveal the scene. These decorations have no gameplay randomness or simulation effect.

Illustrations preserve source aspect ratio inside a 400 x 378 logical-pixel area, without a frame. On a cold selection, a boundary-connected near-black background is made transparent for legacy opaque plates. Interior black ink not connected to the background remains intact. Existing transparent pixels remain transparent. A dark identity swatch preserves contrast for authored pale dungeon glyphs.

## Complexity and bounds

Let N be catalog entries (currently 81), L maximum identifier length, C selected-entry characters, P selected decoded-art pixels, B selected encoded bytes, and A native drawable pixels touched by drawing. Each frame scans at most 23 visible IDs against the catalog: O(23 N L) lookup work, plus O(C) selected-entry cloning and layout storage. At most five font-size candidates wrap and partition the selected text in O(C) time and O(C) temporary memory, assuming fixed-font character metrics are constant-time. These are fixed bounds, not a claim of constant glyph or raster work.

Cold selection performs embedded lookup and decode as documented in native-embedded-art.md, O(B + P) decode work under that decoder's assumptions. Boundary matte preparation visits each pixel at most once and enqueues a constant number of neighbors: O(P) time and O(P) temporary memory. The cached selected image retains 4P RGBA bytes; old/new images can overlap during replacement. Warm frames perform neither decode nor matte processing. Picture compositing is O(A); text rasterization also includes fontdue glyph raster work at the native pixel size. Raising drawable resolution increases those costs even though the logical layout is unchanged. There are no timing measurements in this document.

Validation: tools/cargo.sh test -p corpse-desktop manual::tests checks all-entry fit, authored content preservation, exact stat order, aspect containment, deterministic depth variation and catalog/art coverage. Headless --manual-entry ID --render-test PATH and --render-size WIDTHxHEIGHT allow representative full/minimum-size visual checks. Windows interactive behavior remains a separate platform verification.

## Depth-dependent stat projection

Bastidion reads `population::catalog().fungi[selected_depth - 1]`, the same immutable birth template selected by `create_population_actor`; the exporter’s last-depth deduplication is not used for its displayed stats. Its label identifies the current depth. Tests compare all eight values at every depth, including depth 1 (154 HP, 9 Defense) and depth 40 (622 HP, 87 Defense). Other shared species match fixed actor templates; Zillennia is identified separately by quality, whose stats are fixed within its depth band. Character-dependent companions retain Varies and an explicit character-dependent label.

This projection adds an O(1) indexed lookup and eight scalar/string conversions per frame. The first access to the core population catalog can initialize its OnceLock by parsing the complete embedded immutable JSON: O(J) parse time and retained data memory for J catalog bytes, with parser allocation overhead. Subsequent accesses do not reparse. This is data access, not simulation or live-state discovery.

## Creature-specific animated scenery (October 20, 2026)

The dominant background is now a recognizable environment selected by depth in `manual_environment.rs`: depths 1–10 are layered caves with stalactites, stalagmites and mineral drips; 11–24 are constructed masonry and arched bays; 25–29 introduce leaning broken arches, fallen masonry and a split floor; 30–35 become infernal basalt formations around an animated molten chasm; 36–40 develop increasingly incompatible perspectives, skewed nested gates, impossible stair flights and inverted pillars. These are visual environments, not generated narrative or gameplay terrain.

As a secondary layer, `manual_background.rs` explicitly assigns all 81 creature IDs a semantic motif and scale. The 30 motif families include rooted fungal canopies and spores, slime, blood streams, bee combs, wing beats, roots, regeneration membranes, ichor currents, kelp, shell facets, teeth, needles, claws, blades, horns (with the authored horn count), lunar arcs, void distortion, crystals, gears, frost, flame and sound ripples. A deterministic ID signature places the composition; it does not choose a random theme. Every catalog entry has a unique tested composition at every depth. Future entries must add an explicit profile rather than silently borrowing a fallback.

A low-frequency moving field connects the foreground motifs into an atmosphere. Flow direction and geometry follow the motif: horizontal ichor movement, falling blood, rising fire, radial lunar/void motion, and drifting fungal haze. Depth increases ambient heat, drift speed and rise height; the infernal band adds drifting cinders. The deeper underglow becomes stronger and reaches farther upward, while architectural deformation grows through the ruined and nightmare bands. The existing forty paper palettes and typography remain unchanged. Creature strokes are alpha-composited into the environmental surfaces rather than replacing them with the page color. Scenery is deliberately quiet beneath the text and fades smoothly around the illustration without adding a frame. Six representative creature backgrounds at all forty depths retain at least 4.5:1 contrast against the page's text ink in text/navigation/stat regions; illustration colors remain authored.

The desktop's existing cosmetic frame cadence now redraws the manual at approximately 12 Hz while open. It passes elapsed milliseconds only to the presentation module. Reduced motion holds time at zero and does not request animation redraws. There are no simulation calls, RNG consumption, save changes, new lore, or gameplay decisions in this path. `paint()` remains a stable zero-time entry point for existing render tests.

Reproducible headless review: `--manual-entry bastidion --manual-depth 40 --animation-frame 1700 --render-test review.png`. The optional depth is a render-only override: it changes the page palette, scenery and depth-dependent stat projection while preserving the actual encounter catalog and selected entry. Its navigation roster still reflects that entry's normal selected roster; this QA option does not introduce an encounter at that depth. `--animation-frame` is milliseconds and must be finite and nonnegative; persisted reduced motion takes precedence. Normal interactive depth browsing continues to select the actual roster.

### Additional complexity and validation

Let S be generated strokes (hard cap 4096), G the fixed atmosphere-grid samples (320 × 151 = 48,320), Q the line samples per stroke (at most 97), and A the native pixel area touched including overdraw. Profile matching and signature hashing take O(L) identifier work. Geometry generation takes O(S) time and memory, with fixed 12 secondary motif anchors and at most 34 secondary cinders in the infernal band. Ring tessellation is fixed at 16 edges; magnitude is authored and bounded by 10. Elapsed time is normalized to a 600-second cosmetic cycle before arithmetic and cannot grow work or allocations. Non-finite inputs produce the same phase as zero. The temporary vector reserves 2048 strokes and can grow to 4096; it is discarded after painting, with no unbounded particle history or caches.

Atmosphere evaluation takes O(G) trigonometric/arithmetic work and O(1) scratch space, followed by rectangle rasterization. Stroke rasterization takes O(SQ + A), not merely O(S): every sampled point calls the native-resolution alpha painter, and native covered pixels grow with drawable scale. Environment rasterization adds at most P=52 polygons and 300 structural lines. Each polygon has V≤32 vertices, at most H=301 two-pixel logical scan rows, and a fixed 32-value stack intersection buffer. Scan conversion costs O(P H (V + V log V) + A), including intersection sorting; structural lines have at most 1281 samples each. Arch construction uses two transient ≤20-vertex vectors, so its extra memory is O(V), not per-pixel. Full background cost is O(L + G + SQ + P H (V + V log V) + A), with the fixed structural-line sampling bound included and O(S + V) extra memory beyond the existing framebuffer. Fixed-grid costs are independent of native resolution, while fill/compositing and SDL upload remain O(native drawable pixels). Existing selected-art decoding, cutout processing, text wrapping, font rasterization/cache misses, and upload costs above still apply; animation now invokes the warm-frame work at the bounded cosmetic cadence instead of only on selection. Measured optimized paint timings are recorded separately in native-performance-report.md; profile representative high-DPI frames before optimizing.

Tests cover all 81 explicit profiles, unique geometry at all 40 depths, the requested cave/dungeon/ruin/hell/nightmare depth landmarks, monotonic ambient depth parameters, every-depth environment repeatability/animation/clipping, deterministic animated geometry across all entries/depths, finite bounded geometry at ordinary/extreme/non-finite times, representative repeatable animated raster output and text-ink contrast. Existing all-entry fit, exact text/stat projection, art containment and catalog coverage regressions remain in place. Run `tools/cargo.sh test --workspace`; review multiple entry/depth/time PNGs at native and minimum resolution. Native Windows motion and reduced-motion interaction still require platform verification.

Linux visual verification for the initial animated-background revision (before the later depth-environment revision): the optimized SDL application was opened in the cloud Linux desktop, the manual was browsed from depth 1 through 20 to 40, End/Left creature navigation was exercised, and active animation, fixed layout and readability were inspected. Representative stills covered Blorb, Hemogoblin, Bastidion, River Drake, Moonspawn and Transcendent Zillennia. This does not verify Windows launch, input, compositor behavior or performance.

The later depth-environment revision was reviewed in five rendered stages (depths 1, 15, 25, 33 and 40) with Bastidion held constant to isolate the backgrounds. These still-image checks are separate from the earlier live walkthrough; updated interactive verification and packaging should use the latest executable.

The final depth-environment build was subsequently verified live in the optimized Linux SDL application at depths 1, 15, 25, 33 and 40 using PageDown navigation. The updated 60-second walkthrough showed active motion, legible content and no clipping. Windows remains cross-compiled, not runtime-verified.

### Clean artwork correction (October 20, 2026)
Removed all auxiliary creature decals and material-interaction geometry. The original embedded cutout artwork, composition sizing, solid depth colors, Inconsolata, Natural History placeholders, stable columns and roster are unchanged. The interaction raster pass and its helpers are deleted, so it consumes no per-frame time or memory; remaining rendering complexity is unchanged.

## Dark artwork preservation correction (October 24, 2026)

The bestiary no longer discards all boundary-connected RGB values below 29.
Authored transparency is used unchanged (including the revised Vampire). Opaque
plates estimate each matte channel from perimeter 0.1%–99.9% quantiles with a
three-level encoding-noise margin, then flood only matching matte pixels from
the boundary. This preserves shadows brighter than the black matte and the
Zillennia bodies darker than their gray matte. Enclosed ink is retained.
Original RGB values, PNG masters and distribution WebPs are unchanged.

This is conservative color-key rendering, not semantic image reconstruction:
a foreground pixel identical to the matte and connected to it cannot be
unambiguously recovered. No new anatomy or color is generated. Native before/after
QA covers Giant Rat, Bastidion, Vampire, Infant/Timeless Zillennia and Dragon.

For P source pixels, selected-plate cold preparation remains O(P) time and O(P)
extra memory (visited bits and pending indices); a fixed 3×256 histogram adds
O(1) storage. Authored-alpha plates need only the O(P) alpha scan and no flood
allocation. The existing selected-art cache makes this a selection-time cost,
not a per-frame cost. Rendering, simulation and gameplay are unchanged.

## Unclipped directional ink ground (October 24, 2026)

Latest owner direction adds generated directional charcoal brushwork behind all
79 creatures. These are two embedded transparent raster assets, not the removed
procedural decals. The narrow sweep is used when its complete bounds fit the
screen; a broad directional version backs tall or wide silhouettes without
shrinking the original creature. Existing creature PNG/WebP masters and the
conservative matte recovery algorithm remain unchanged.

Draw order is solid depth field, ink, creature, then all UI. No broad column,
stat, footer or empty-title masks erase the ink. Actual existing opaque title
and section-heading fills naturally occlude the art behind them. Text retains
its original ink color, with a narrow inverse-color glyph outline when any
part of its line needs contrast. Whole lines use the same treatment, never
per-letter switching. This option is scoped to manual painting and reset on
exit; other screens keep the original prose rendering path.

For P creature source pixels, cold ink coverage fitting checks every occupied
source pixel against ink alpha >=240 at up to37 candidate scales (1.2–3.0),
O(P) bounded time and O(1) scratch. Its scale and narrow/broad choice are cached
with the selected art. Two1536x1024 ink images add O(S) PNG decoding and retained
RGBA memory, about12MiB total. Warm-frame ink work is O(A) native touched pixels;
canvas-fit choice avoids oversized long strokes and preserves original art size.
Text contrast scanning is O(G) for G rasterized glyph pixels. Outlined lines add
O(G*r*r) native alpha work, r=ceil(drawable scale), and O(G) transient cached
glyph bitmaps per line. Existing framebuffer/upload/font costs still apply.
No animation, simulation, lore, stats or roster changes are introduced.

Validation uses native rendered light/dark pages, all79 entries, source-pixel
ink coverage using the actual selected asset and clamped render placement,
canvas containment, and opaque-title occlusion regressions. Native resampling
is additionally visually reviewed rather than inferred solely from coordinates.

Assets: assets/bestiary-ink-splash.png and bestiary-ink-splash-broad.png within
corpse-desktop. Built-in image generation prompts requested a dense charcoal
lower-left-to-upper-right brush sweep with ragged directional edges, transparent
background and no text/creature/scene. The broad variant widens the central dark
mass so its complete irregular edges fit the page. Original generated alpha is
preserved. No original monster artwork was regenerated or recolored.

## Continuous first-appearance page colors (October 25, 2026)

The page remains spatially solid. Normal visual depth is the selected creature's
first appearance, independently of its combat calibration reference floor.
Using combat midpoints caused 22 backward/lightening steps in the existing
79-page order, so that provisional coupling was removed. Normal page navigation
now never lightens. The original interpolation between parchment on floor1,
blood red on floor25 and black on floor40 is preserved without coarse tiers or
new anchors. Bastidion, the first page, uses the original parchment anchor.
Dragon first appears at floor38 and therefore uses near-black; pure black remains
the floor40 preview anchor. The 79-entry roster, first-appearance ordering and labels,
authored Natural History, foreground art, directional paint-splash coverage and
all fixed UI placement remain unchanged. This does not reintroduce the superseded
animated scenery or spatial gradients.

Native `--render-test --manual-entry` uses the exact live page path by default.
Only an explicit `--manual-depth 1..40` overrides visual depth; fractional values
are supported for continuous-palette QA, while depth-dependent Bastidion stats
use the lower whole floor. The override does not mutate selected entry, roster,
spawn eligibility or combat calibration. Current creature stats still come from
the revision31 birth constructor. Existing integer preview commands remain valid.

## Visual-only bestiary composition revision — 2026-10-25

This section supersedes the earlier oversized title / generated-ink coverage rules.
The 79-entry roster, authored assets and JSON text, stat calculations, controls,
first-appearance palette, Inconsolata faces and no-scroll behavior are unchanged.

The page resolver uses the occupied alpha silhouette, not square source canvases:
- Broad specimens use a shallow spanning illustration, one row of eight stats,
  and two lower reading columns when the full copy fits at 18px or larger.
- Tall specimens use a right-hand art/stats stage and a wider left reading column.
- Compact specimens use a left-hand stage and continuous right-hand prose.
- Long entries first receive a wider reading area. The longest narrative uses a
  continuous upper narrative, separately positioned complete ability/location
  groups below, and a smaller specimen; it does not abbreviate the history.
- Section-leading space separates prose groups; titles are outside every art
  stage. No text outlines or scratch backing crosses the content regions.
- A flat same-hue lighter organic specimen field is used only for dark void pages whose
  background luminance is below 0.08. Its edge uses the existing broad-ink alpha,
  recolored to one calm burgundy value and contained within the art stage; there
  are no scratch highlights, border, glow or shadow. It is not a sprite edit. All other regions
  retain the established solid depth color. Secondary navigation/labels retain
  at least 4.5:1 contrast against that field's page background.

Only fully transparent outer margins are trimmed after the established cutout
routine; every occupied RGBA pixel is copied verbatim. Art proportions are never
stretched. Artwork dimensions and content length select the composition; no
simulation state or cosmetic random stream is involved.

### Complexity and bounds
Let P be decoded selected-art pixels, C selected-entry authored characters, L its
wrapped lines, A destination specimen pixels, F output framebuffer pixels, and N
catalog entries. The existing O(N log N) catalog ordering and O(N) selected-entry
lookup are unchanged. On a selection cache miss, decode and original matte/flood
fill cost O(P), then occupied-bound scan and transparent-margin copy cost O(P)
time and O(P) temporary/output storage. The resulting trimmed Picture is cached
by asset ID; warm frames neither scan nor trim its pixels again. The optional
void backing is decoded/recolored only when its cached target color changes:
O(S) time and O(S) retained storage for S existing backing-asset pixels. Its
warm paint is bounded by the same destination art area A.

The page resolver tries at most a fixed number of region configurations and
font sizes16–20; it wraps O(C) text and stores O(C + L) temporary line/span data.
The actual accepted 79-entry layouts are required by tests to use body sizes of
at least18. Multi-region long pages wrap a fixed number of complete content
subsets; the same O(C + L) bound applies. Font rasterization/cache and native
framebuffer scaling retain Painter's existing costs. Each warm render is
O(N + C + L + A + F), not O(1). No new persistent per-page layout cache was added.

Focused tests check actual resolved art/text/stats bounds, no orphan headings,
full authored description/history/form/ability/location reconstruction, every
occupied art pixel's preservation, unchanged stat order, and pixels in protected
header/navigation/footer regions. Full native/workspace and resized-render QA
must be reported separately; successful compilation is not visual acceptance.

## Continuous foreground colors — November 8, 2026

The new [Bestiary continuous colors](bestiary-continuous-colors.md) entry supersedes
the earlier threshold-based black/white and stepped-secondary contrast selection.
It documents the requested smooth foreground/unread-marker gradients, fixed glyph
keyline, continuous void field, exact complexity costs, and still-pending native
verification. Original creature artwork and authored alpha remain unchanged.
