# Tile-paint prototype: source contract and complexity

## Status and boundary

This is a source-level description of the opt-in `CORPSE_TILE_PAINT=1` candidate with the V2 bitset mask builder, not a universal performance or release claim. V2 timing, exact-pixel regression, software SDL upload/readback and integration gates have completed; the final report separates their scopes and records mixed workload results. Windows/GPU runtime verification remains pending. V1's interval-push/sort runtime regression and raw evidence remain preserved below. With the option unset, `main.rs` keeps the full-paint policy documented in [full-painting-runtime.md](full-painting-runtime.md). This candidate is separate from the earlier `RetainedGame`/Scene-index experiment; its previous results do not establish this candidate's behavior.

The tile layer consumes the existing, already-redacted cached `View`. It does not query hidden world state, change visibility, advance simulation, alter saves, or make every hover observe the world again. Existing dirty-view refresh, cosmetic/animation eligibility and the loop's 8 ms wait remain upstream. This document does not certify Weave mechanics, layout decisions, Windows behavior or release readiness.

Source boundaries are `crates/corpse-desktop/src/tile_damage.rs`, `tile_overlay.rs`, `render.rs` and `main.rs`; existing effect generation is in `effects.rs`. Paths and function names below refer to those files.

## What is compared, retained and painted

`TilePlanner::replace_view` builds a flat index of the new observed layers, then compares old/new per-slot ordered paint stacks by borrowing the two Views. `paint_eq` checks paint inputs, including strings, colors, visibility/memory flags, ordered underlays and animations; relevant floating-point values use bit equality. It neither hashes a Scene nor normalizes numeric values. A separate order check marks old/new footprints when layer coordinates change at a global paint-order position.

There are two reusable indices, each with slot head/tail arrays, layer next links, conservative native-pixel footprints and animated-layer IDs. The planner retains indices and dirty marks, not cloned Scenes or Cells. It retains the ordinary final RGBA framebuffer between requests; it does **not** add the former clean-base framebuffer or a second full-frame pixel copy. Existing custom material/light reconstruction and Dreamwalking painting can still rebuild or clone Cells in `Painter::game_at`; “no Cell clones” describes this planner, not every existing renderer branch.

The comparison rules are conservative:

- **Both no paint layers:** empty old/new stacks do not mark that slot from the stack comparison. This means no observed paint layers, not a new assertion that the world tile is unseen or irrelevant. Nearby text spill, paths, panels, effects, global invalidation or other composition can still touch its pixels.
- **Same paint, no local animation:** skip a tile mark from that comparison. Global material/light animation or Dreamwalking may still schedule its footprint.
- **Changed paint or memory state:** mark the slot and all old/new layer footprints, including annotation spill. Old footprints restore pixels after a layer disappears or moves.
- **Animation:** local animated layers are indexed. A new eligible time marks those footprints; global animated appearance/Dreamwalking marks all indexed layer footprints. Reduced motion uses a fixed time. This is not a claim that all animation becomes local.

Changed Chronicle/HUD/enemy/provenance fields mark their panel regions. Paths, exits, hover and selected actors have old/new marks, including explicit marks around in-place presentation changes. `tile_overlay::Tracker` compares pack, hover, effect, shutter, modal and save-error presentation state. Some of those small descriptors own text or clone inspection data; there is no allocation-free guarantee.

`finish_dependencies` expands affected translucent panel footprints **before** resetting their backdrop. Any nonempty damage also marks all current effect footprints. It then repeatedly adds intersecting panel dependencies until closure, so text and translucent layers are replayed against a restored background. Current inspector/report/hazard/ability/debug modes use conservative panel or band bounds; choices use a full-modal fallback. A full modal expands any intersecting damage to the frame. First use, invalid geometry/context and screen/display invalidation also take the full path. Animated global footprints and upload-only fallback are distinct from full CPU repaint.

V2 `RowSpans` unions rectangles into a reusable row-major `u64` bitset, then scans set-bit runs into the same exact native-pixel row intervals. Adjacent runs join across word boundaries; there is no sorting, duplicate-interval allocation or L-shape bounding-box collapse. Duplicate rectangles still repeat word-union work. `paint_base` invokes `game_at` once for a nonempty candidate frame. The renderer makes **one full paint-order pass over C layers**, testing each cached footprint against the mask, rather than one full pass per dirty rectangle. Accepted layers preserve normal ordering. Raster primitives clip to row spans; effects, tooltips, inventory/modals, shutter and the save-error banner then follow the existing composition order. Empty damage skips this composition.

This does not mean only semantically changed pixels are written. A conservative tile/annotation/panel footprint may include unchanged pixels, dependency closure can broaden it, and intersecting layers can overdraw the same pixels. Pixel equivalence with full painting is a test obligation, not a consequence of naming the mask “exact”: the union is exact for the supplied conservative rectangles.

## Variables and costs

For one comparison/candidate frame, use the larger old/new sizes where needed:

| Symbol | Meaning |
| --- | --- |
| N | Map coordinate slots, width × height; not only visible/changed tiles. |
| C | Observed paint layers, including multiple layers at one slot. |
| B | UTF-8 bytes examined by borrowed comparisons and footprint preparation. |
| L | Other variable entries traversed: underlays, animations, UI metadata and path lists. These are not all fixed-size scalars. |
| W, H, F, Q | Native pixel columns, rows, framebuffer area F = W × H, and words per row Q = ceil(W / 64). |
| Sraw, Smerged, R | Sraw counts logical rectangle-row contributions, including duplicates; it is no longer allocated interval entries. Smerged counts final row spans. R counts set-bit runs emitted within words before adjacent runs join. |
| E | Effect work candidates: cue visits and generated/projectile-loop positions, including positions suppressed before emission. Final distinct effect descriptors can be fewer. |
| K | Unique dirty tile IDs; additional footprint/panel marks are accounted separately. |
| P, A | Total raster pixel visits, including overdraw, and distinct pixels in the final damage union. P need not equal A. |
| D, U | Panel dependency entries and output upload rectangles. Current modal alternatives bound D; U depends on span fragmentation. |

| Phase and frequency | Time and important assumptions |
| --- | --- |
| New View: `Index::build`, `replace_view` | O(N + C + B + L). Initializes slot/layer arrays, counts annotation characters, prepares every new layer footprint, compares all slots and borrowed variable fields, and checks global layer order. Selection changes can add another C scan. Short-circuit equality may reduce actual work, not the worst-case bound. |
| Animation/presentation candidate | Local animation O(animated layers); global animation O(C) marks. In-place path/selection marking includes path lengths, a possible C scan and footprint text work. Overlay comparison includes owned/compared text and variable lists. |
| Effects descriptors | If cues exist, flat tables add O(N + C + E + effect/footprint text bytes) work and O(N + emitted descriptors/text) scratch. `effects::tiles` allocates outputs and traverses projectile steps even when some emit no tile. The existing subsequent `Painter::effects` repeats projection and still uses a BTreeMap: up to O(N + C + E log(E + 1) + text/raster work), with its own allocations. The new tile planner is flat; the entire effects pipeline is not tree-free. |
| One V2 mask construction: `finish_damage` | Reset costs O(H + H × Q). Adding clipped rectangles costs O(K + rectangle marks + Σ h_r × q_r), with clipped rectangle height h_r and covered words q_r. Emission costs O(H × Q + R), scanning all mask words and joining adjacent runs. Combined: O(H + H × Q + K + marks + Σ h_r × q_r + R). No sorting or duplicate interval pushes remain; duplicates still repeat word work. |
| Footprint intersection | `RowSpans::intersects` scans the footprint's covered pixel rows and binary-searches each row. Worst case O(h × (1 + log(smax + 1))) for footprint height h and largest merged row list smax, including empty-row visits. Point containment also binary-searches a row; neither is universally O(1). |
| Dependency closure | At most D successful addition rounds, plus a terminal scan, with up to O(D²) rectangle-intersection tests and up to D + 2 mask constructions on the nonempty path. Effects are marked together rather than running effect-to-effect quadratic closure. The small current D is a bounded UI policy, not permission to ignore rebuild costs. |
| Nonempty base composition | One C order pass plus the sum of its footprint-intersection costs, existing layout/text/animation helpers, row-clip searches/segment visits and O(P) pixel work. A coarse intersection bound is O(C × H × log(Smerged + 1)). Expected path HashSet construction/membership remains; text fitting/wrapping, cold glyph rasterization and global material/light reconstruction retain their own costs. For example, Chronicle wrapping can be quadratic in its text length. |
| Clear/reset and upload planning | Masked clear still visits H rows, with row searches/segments and cleared pixels. `clear_frame` clears K flags and mark vectors. `upload_rectangles` costs O(H + Smerged) and allocates U rectangles, vertically merging only consecutive identical row span lists. |

Thus neither comparison nor a nonempty frame is O(K)-only. Even K = 0 can require comparison, animation, UI/effect preparation, span construction or presentation work. The simulation, save flush, observation, lighting and pathfinding costs upstream are unchanged, not hidden inside a claimed dirty-tile bound.

## Storage, uploads and fallback

The planner's retained capacities scale with two O(N + C) indices, O(N) dirty flags, O(K + marks) candidate storage, O(H + retained merged-span capacity) row vectors, and the V2 mask words. Live mask storage is 8 × H × ceil(W / 64) bytes; retained vector capacity can be larger. This is a one-bit-per-native-pixel union mask with row-end padding, not an RGBA framebuffer copy. Capacity can remain above the current live size after a large frame. Index, word and row buffers are reused but can reallocate on growth. Overlay frames, strings, pack changes, effect tables/output, path sets, dependency vectors and upload-rectangle vectors still allocate; counters are not a complete heap inventory. Existing framebuffer/glyph/profile/View storage remains separate. The final pixels occupy 4F bytes; there is no additional 4F-byte clean base in this candidate.

With diagnostics enabled, `raw_span_entries` now reports logical rectangle-row contributions, **not allocated interval entries**. `merged_span_entries` describes the emitted final row lists. `bitset_words_touched` counts covered-word union operations, including duplicates; `bitset_words_scanned` counts words visited during emission, not the additional reset clear. `bitset_capacity_bytes` reports allocated word capacity separately from `span_capacity_bytes`. These values describe the latest mask construction and are overwritten when dependency closure rebuilds it; they are not automatically sums of all preparation work in a frame.

`tile_overlay::upload_source` borrows each rectangle from the final framebuffer using full framebuffer pitch, with checked bounds; it makes no application-side rectangle/prefix copy. Normal requested transfer is 4A pixel bytes through U SDL `texture.update` calls. The borrowed slice spans pitched gaps, while the requested rectangle identifies the pixels to update. Driver copies, synchronization and per-call overhead are external costs.

`upload_plan` currently switches to one full 4F-byte upload if more than 128 rectangles would be requested, or a full upload is explicitly required. This bounds update-call fragmentation, **not CPU paint damage**, and is an unmeasured prototype policy rather than a universal GPU optimum. The plan records requested and actual bytes/calls. Expose/restored windows can present the retained texture without paint or upload; device/target reset requests full upload, recreating texture storage where required. A required present still clears/copies the full texture to the canvas and presents it; partial texture updates do not imply partial driver presentation. Other screens retain full painting.

## Preserved V1 evidence and V2 verification

V1 pushed an interval into every covered pixel row, sorted each row, then merged overlaps. Its construction cost was O(H + K + marks + Sraw + Σ s_y log(s_y + 1)), with actual duplicate interval storage before merging. That runtime regressed; the original [timing-v1 evidence](../../../reports/tile-paint-prototype/timing-v1/) and [attribution-v1 evidence](../../../reports/tile-paint-prototype/attribution-v1/) are retained under `append-recording-trial/reports/tile-paint-prototype/`. They are not replaced or relabeled as V2 measurements. V2 changes only this mask builder and its diagnostics, not the flat layer indices, panel closure, C-order paint pass, upload policy or other architecture. Removing sorting does not by itself establish an end-to-end speedup: V2 clears/scans H × Q words even for sparse or empty masks and still repeats unions/closure passes.

V2 measurements are recorded separately in the final report and raw evidence; results are mixed, not a universal speedup. The full workspace gate with the final same-clock dependency guard passed 1,299 tests (0 failed, 29 ignored; 148 groups). The V2 headline and reversed-order timing binary predates that narrow correctness guard. Validate mask equality for empty/full/duplicate/overlapping rectangles, word boundaries and partial final words, then exact final pixels against full painting for changed/missing layers, memory transitions, overlapping annotations, local/global animation, paths/selection, effects, tooltip/pack closure, bounded and full modals, shutter/banner removal, resize/profile/context changes and reduced motion. Check sparse/last-row pitched uploads, empty-damage expose and the >128-call fallback with actual SDL readback. Keep canonical gameplay/save checks separate from pixel checks.

Measure comparison/footprints, mask preparation/closure, base and overlay composition, upload and present separately, while also retaining actual command + durable save + observation + paint end-to-end samples. Report n, median, p95, p99 and worst observed with same-sample attribution; small-sample maxima are not theoretical worst cases. Record K, C, logical raw contributions/merged spans, bitset touched/scanned words and capacity, requested/actual upload bytes/calls, allocation growth and full fallbacks. Include both static hover and broadly changing commands; do not infer a speedup from fewer pixels alone. Platform/driver results and release gates remain separate.


## Same-clock global animation dependency

When a replacement View has a different immutable terrain snapshot identity (including presence/absence) or terrain layer count while global material/light animation is active, old and new cached footprints are marked even if the presentation clock is unchanged. Time-zero Cell colors alone do not certify identical positive-time flicker. This uses Arc identity and existing cached footprints, not a deep snapshot comparison or new world query. The default-profile non-global case remains clean. Regression fixtures cover equal time-zero Cells with distinct positive-time light frequencies, same-identity reuse, and count/presence changes. This is a correctness guard, not a performance optimization.
