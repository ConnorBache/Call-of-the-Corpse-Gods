# Exact-output bulk software painting

## Contract and scope

The renderer remains CPU RGBA at the requested drawable resolution. No image,
font, palette, animation timing, glyph sampling, layout, simulation, saved state,
SDL upload format or gameplay rule changes. Bulk does **not** mean O(1): touching
A pixels still costs O(A). It removes redundant coordinate transforms, clip tests,
function calls, and repeated blend arithmetic. No unsafe casts, endian assumptions,
alignment assumptions or new native dependencies are introduced.

`pixels` remains tightly packed RGBA bytes, row pitch `width * 4`; SDL retains its
existing upload path. Letterboxing and rounding remain the same. Pixel footprints
may vanish below 1x; positive `rect` strokes retain the existing one-native-pixel
minimum. Clipping preserves the same minimum-size rule. Text and ASCII masks
historically preserve destination alpha; native-pixel-derived painting forces it
to 255 even for zero source alpha. The batch implementations preserve this
otherwise easy-to-miss distinction.

## Modified functions and transitive complexity

Let P = framebuffer pixels; A = clipped destination pixels for a draw; G = text
characters; M = sum of rasterized glyph mask areas; F = total font outline
rasterization work on cache misses; H = Chronicle native rows; S = source compound
mask area; K = cache key bytes. Hash lookup is expected, not worst-case guaranteed
constant time (string keys also cost O(K)). Font metric lookup costs are inherited
from fontdue and not treated as free; fallback face counts are fixed and small.

| Function | Frequency and route | Time / extra memory |
|---|---|---|
| `native_bounds` | Once per batch | O(1) / O(1); transforms one clip |
| `native_fill` | Rectangles, logical pixels, tint, blessing dimmer | O(A) / O(1), contiguous clipped row slices; opaque copy or hoisted integer blend |
| `blend_rect` | Uniform logical alpha rectangles, effects, gas/reflection, tooltip | O(A) / O(1), no minimum-width expansion; calls native_fill |
| `rect` | Every background/panel/stroke caller | O(A) / O(1); calls native_fill, minimum positive stroke preserved |
| `pixel` | Sparse cosmetic strokes and varying logical effects | O(A) / O(1); exact logical footprint, calls native_fill |
| `native_pixel` | Sparse residual bespoke strokes and outlines | O(1) / O(1); opaque fast path, existing clip semantics |
| `blend_mask` | Text, role text, prose | O(A) / O(1); contiguous mask and destination rows, selected alpha semantics |
| `text` | HUD/menu/etc. | Warm O(G metric work + M), cold adds F; existing glyph cache and fallback retained |
| `text_role` | Styled typography | Warm O(G metric work + M), cold adds F and weight conversion O(M); existing 2048-entry cache retained |
| `prose_raster` | Prose glyph request | Expected warm O(1), cold O(F + glyph area); exact f32 size-bit key, regular/title face, bounded 2048 entries |
| `prose` | Bestiary/start/modal prose | Warm O(G metric work + M), cold adds F; outlined path adds neighborhood halo overdraw O(M r²), r = ceil(device scale); retains original draw order |
| `picture` | Assets and image-backed menu/background | O(A) / O(1); clip once, source y once per row, original nearest-neighbor sampling and integer RGBA blend |
| `map_glyph_styled` | ASCII map glyph | Warm O(A), cold atlas area resampling plus mask allocation; shear/clip hoisted once per row; existing 512-entry mask policy retained |
| `blend_scaled_mask` | Non-ASCII/compound map glyph | O(A) / O(1), exact f64 coverage scaling and saturating u8 conversion |
| `map_identity_alpha` | Non-ASCII/compound or non-atlas family | Warm expected O(K + A); cold adds F + S + output mask area. Caches 512 masks; key includes glyph, native dimensions, source-scale bits, selected font mode, letter-spacing bits and scale bits |
| `path_tint` | Visible route highlight cells | O(A) / O(1), batched translucent rectangle |
| `blessing_choices` | Blessing overlay | Dimmer O(viewport pixels), plus unchanged bounded offers/text; one batch rather than WIDTH×HEIGHT logical calls |
| `beating_emblem` | Main-menu frames | Cold O(source + destination area); warm O(occupied static samples + dimensions + transformed heart bounding-box area); one-size retained cache, see follow-through below |
| `effects` | Cue projections and painted cells | Existing O(map cells + cue tile generation + sorted effect tiles), then O(painted pixels); uniform tile fields now one batch |
| `inspection_ui::paint_hover_at` | Hover tooltip redraw | Existing measured row/layout work plus O(backing area + glyph pixels); uniform backing now one batch, tooltip color lookup once |
| `game_at` Chronicle fade | Every game repaint | O(H*(256 + row width)), O(3*256) stack bytes; exact original floating composite builds per-row RGB lookup tables, then byte lookups |
| `resize` | Drawable-size change | O(P + released cache storage); clears original caches and new prose/compound caches |
| `new` | Painter creation | Original asset/font decode + framebuffer/layer construction; new maps empty, O(1) additional initialization |

New caches are presentation-only and excluded from serialization. They store
coverage, not foreground colors, alpha, clip, simulation data, or time. Profile
changes cannot return stale compound coverage because every influencing parameter
is in its key; prose fonts are fixed and exact raster size is keyed. Resize clears
both. Hash tables have finite entry caps, not a fixed byte cap: unusually large
masks still use proportionally more memory. This is not advertised as constant
memory independent of drawable size.

## Dispatch audit: what uses the efficient routes

- Game background, map cell backgrounds, borders, rail panels, bars, button
  surfaces, condition meters, selection outlines: `rect` → `native_fill`.
- Map ASCII and italic portal glyphs: existing atlas-mask cache → clipped mask rows.
- Non-ASCII/compound identities: bounded new coverage cache → scaled mask rows.
- HUD, ability/status/inventory strings, menu text, history: `text` / `text_role`
  → existing font caches → clipped mask rows.
- Bestiary regular/title/contrast prose: new exact raster cache → clipped mask
  rows; halo retains per-covered-pixel neighbor order because repeated blends
  overlap and changing order/union coverage would change exact pixels.
- Bestiary PNG/WebP plates, menu fire image, other `Picture` content: clipped row
  sampling and blending. Sampling does not become a raw copy when scaling or
  partial alpha is present.
- Bestiary atmosphere: authored 4×4 logical blocks → bulk `rect`; polygon
  scanlines already emit horizontal `rect` spans. Tiny variable-color animated
  stroke samples still use `pixel` because neighboring colors/coverage differ.
- Route tints, blessing full-view dimmer, effect tile fields, gas/reflection bands,
  and inspection tooltip backing: batched integer-alpha spans. Tooltip keeps its
  original logical viewport preclip and active Painter clip.
- Chronicle fade: row/channel lookup table; alpha untouched just as before.
- Menu heart/veins/ribs: separate source layers retained; one-size occupied-sample
  cache, exact per-axis heart sampling and original vein wave/geometry rules.
- Menu title contours: horizontal fill runs; exact worn-cut predicate retained.
- Embers, heartless slash, edge/environment particles: sparse primitive calls remain;
  they inherit faster `rect`/`pixel`/opaque point writes.
- Frame clear already wrote contiguous RGBA chunks without clip/transform/blend;
  unchanged O(P). No unsafe 32-bit reinterpretation is necessary.
- UI modules compose these painter APIs rather than maintain a separate framebuffer
  rasterizer. SDL still uploads/presents the complete buffer: CPU batch work does
  not establish a GPU/vsync improvement.

## Deliberately unchanged higher-level work

Text fitting/wrapping and font metrics, draw-list construction, observation,
lighting, effect mask construction/sorting, terrain animation color math, panel
layout and UI projection still have their prior complexity. A bulk pixel patch
cannot honestly claim these disappear. In particular measured wrapping can remain
quadratic in text length; changing it requires a separate exact layout test because
floating accumulation, whitespace and long-token splitting are observable. Draw
order is retained throughout, including overlap and outline blending.

## Verification and measurements

See the accompanying verification report and raw evidence. Exact cross-build RGBA
comparison uses the released v16 renderer versus the candidate with identical
inputs. Timing is separate from asymptotic complexity and excludes SDL upload,
present/vsync, startup decode and glyph warmup unless stated otherwise. No Windows
runtime claim is implied by successful cross-compilation.

### Reproduce

With the pinned toolchain and SDL2 available:

```sh
cargo test --release -p corpse-desktop --bin corpse-desktop --locked
cargo test --release -p corpse-desktop --bin corpse-desktop bulk_paint_benchmark --locked -- --ignored --nocapture
PAINT_EVIDENCE_DIR=/an/empty/qa/directory cargo test --release -p corpse-desktop --bin corpse-desktop exhaustive_paint_evidence --locked -- --ignored --nocapture
```

The benchmark prints each raw nanosecond sample plus median/p95. It includes clear,
opaque background, text, prose, repeated compound glyphs, avatar-room game, menu,
Wendigo bestiary, and explicitly synthetic dense animated draw cells. The synthetic
case is a renderer stress scene, **not** a claim about reachable creature counts or
world simulation timing. Pair the same harness with the released baseline and
candidate, warm both, run without competing compiler/tests, and reverse process
order to detect bias. The ordinary unit test has no timing threshold.

## Follow-through: remaining animated menu work

The first paired run left a~21.8ms1080p menu. An isolated component probe of that
build measured anatomy13.00ms, fire-image blit8.09ms, procedural fire1.45ms and
filled title shapes0.93ms. These independent microbenchmarks are not additive
exclusive spans. Source inspection confirmed two needless operations: all three
anatomy layers scanned transparent full-canvas regions every frame, and opaque
fire-image samples still used mixed-alpha arithmetic.

- `menu_raster::Raster::new`: cold O(source-heart pixels + destination area),
  retaining O(occupied vein/foreground samples + native width). Stores original
  RGBA and exact destination-coordinate vein distance, not a recolored image.
  Source layers are immutable per Painter. One native `(width,height)` only;
  old storage is dropped before replacement, and resize drops it.
- `menu_raster::Raster::paint`: warm O(occupied veins + occupied foreground +
  native width + native height + transformed heart bounding-box area). Static
  samples are clipped at draw time. Heart columns reuse a capacity-sized Vec;
  the original forward destination-to-source floating operations are evaluated
  once per axis, and only proved-transparent source regions are rejected. The
  rest phase retains the original integer mapping. No clip/origin is cached.
- `menu_raster::blend`: O(1), original integer alpha semantics; opaque copy avoids
  unnecessary mixed-alpha arithmetic. `storage` is O(1) diagnostic capacity
  accounting, excluding allocator metadata.
- `beating_emblem`: constructs/reuses that one-size cache then paints in the
  original veins→heart→foreground order. No approximate transform bounds,
  accumulated motion, time-key cache or resolution reduction is introduced.
- `picture`: still O(clipped area), O(1) scratch; alpha255 copies four exact bytes,
  alpha0 leaves RGB alone but still forces destination alpha255, other alpha
  values use original integer blending. Source sampling is unchanged.
- `menu_shape`: O(H*E log E + A) worst-case, fixed E≤32 scratch; opaque polygon
  interiors now write horizontal spans. Worn titles retain the exact per-column
  cut predicate and batch surviving runs. Contrary to the earlier broad wording,
  title interiors are dense filled polygons, not merely sparse decorative dots.

### Cache eviction accounting

The glyph-cache tables above describe warm and normal cold work. A cap-triggered
`clear` also visits/drops retained entries (O(entry count) plus allocator cleanup),
then the new raster is built. Dropping a resize-invalidated cache has the same
cleanup qualification. Prose outline calls retain Arc references for the current
line; an eviction cannot free those live bitmaps until the line completes, so
transient line storage still scales with its glyph/coverage data. Cache caps are
entry-count bounds, not constant byte budgets independent of font/device size.
