# Visibility lighting and bulk painting performance

The Call of the Corpse Gods  
Engineering release report · 31 October 2026

**Verified release candidate — all integrated correctness gates passed. Publication is not claimed here.**

This report compares the source baseline corresponding to released **full source v16 / Windows v12** with the merged visibility, lighting and bulk-paint candidate in `visibility-r39/source`. The candidate targets **full source v17 / Windows v13**; the verification below is complete, while publication is a separate step. The intended full-source package includes the build inputs; any optional code-reading ZIP is separate and is not a build package. Full-source packaging/publication is separate from these performance and correctness results.

The measured work is substantially faster without changing tested simulation outputs or pixels. In the normal feature-off core build, crowded Wait falls from **23.263 to 9.095 ms** and seven-portal-pair Wait from **78.028 to 8.117 ms**. The first observation at a new origin is measured separately from cache reuse. At 1080p, the final warm CPU menu painter falls from **40.292 to 10.764 ms**. The merged desktop still has a major portal-workload bottleneck: **37.317 ms main-view observation / 45.908 ms hover refresh** medians. Faster core terrain does not make the entire UI fast. These are component CPU results, **not measured input-to-visible latency** and not a Windows runtime result.

No save policy, save schema, replay rule, creature ID, menu asset or simulation RNG ordering changes are part of this release. The separate save/rewind optimization task is outside this report.

## Measured core results

Every row below has **90 samples per side** from three paired process runs, 30 measured samples per run, with three warmups. The second pair reverses baseline/candidate order. Both sides use pinned Rust 1.99.1, normal Cargo release settings (opt3, thin LTO, debug1), with `runtime-timing` **off**, on the same host with known collaborating heavy jobs paused or completed.

Wait and Pickup start from independently identical canonical states. Setup, input cloning and equivalence checks are outside the core stopwatch. Pickup is a command, not necessarily a world tick. Generated is one real seeded starting world; crowded148 and seven-portals are explicitly derived stress fixtures, not a representative survey of all possible play.

| Command workload | Baseline median / p95 ms | Candidate median / p95 ms | Median speedup |
| --- | --- | --- | --- |
| generated / wait | 3.241 / 6.055 | 2.911 / 5.784 | 1.11× |
| generated / pickup | 1.760 / 4.318 | 1.348 / 2.415 | 1.31× |
| crowded148 / wait | 23.263 / 38.374 | 9.095 / 12.832 | 2.56× |
| crowded148 / pickup | 10.480 / 29.815 | 1.889 / 2.797 | 5.55× |
| seven-portals / wait | 78.028 / 172.974 | 8.117 / 12.803 | 9.61× |
| seven-portals / pickup | 78.017 / 219.212 | 2.719 / 4.036 | 28.69× |

### First observation and movement to an uncached origin

**Cold** resets derived caches by cloning the input before each observation. **Warm** reuses the same origin and geometry after preparation. **Fresh origin** changes the observer position after preparing the geometry; it tests a new-origin observation rather than only replaying the same cached answer. It is a useful proxy for the observation work after teleporting within unchanged geometry, **not a timed teleport command or a complete teleport-to-display measurement**. A changed floor, door or portal can still require a cold geometry rebuild.

| Observation workload | Baseline median / p95 ms | Candidate median / p95 ms | Median speedup |
| --- | --- | --- | --- |
| generated / cold | 1.802 / 4.041 | 0.911 / 1.240 | 1.98× |
| generated / warm | 1.575 / 3.896 | 0.697 / 0.997 | 2.26× |
| generated / fresh-origin | 2.278 / 4.765 | 0.879 / 1.148 | 2.59× |
| crowded148 / cold | 2.025 / 3.863 | 1.245 / 1.920 | 1.63× |
| crowded148 / warm | 1.924 / 2.678 | 1.082 / 1.509 | 1.78× |
| crowded148 / fresh-origin | 2.563 / 3.730 | 1.163 / 1.615 | 2.20× |
| seven-portals / cold | 70.612 / 128.764 | 2.144 / 2.899 | 32.94× |
| seven-portals / warm | 73.795 / 240.479 | 1.092 / 1.508 | 67.60× |
| seven-portals / fresh-origin | 24.529 / 136.680 | 1.117 / 1.603 | 21.96× |

The seven-portal cold median is **2.144 ms**, versus **1.092 ms** warm and **1.117 ms** fresh-origin. The first call is not free, but the improvement is not restricted to a warmed observer. All six command outcomes match the baseline canonical saved state; the harness also compares initial inputs and the full bytes of the final `observe_terrain` result retained for each case/mode in each pair. Intermediate fresh-origin outputs are timed but are not all cross-build byte-compared. Separate exhaustive geometry and projected-actor tests provide broader differential coverage; these finite fixtures still do not prove equivalence for every possible state.

## Final merged desktop comparison

The assembled desktop was tested after merging the final painter changes. It uses **runtime-timing feature-on builds with the collector disabled** for the comparison below: 15 samples per process, three warmups, two passes with baseline/candidate order **AB then BA**, and reversed collector order. Statistics pool **30 samples per side**. This is not the feature-off core timing profile; do not mix the rows to calculate a total or a cross-component speedup. Wall samples include returned-result destruction; internal function spans end before caller disposal. The detached harness excludes SDL event polling, upload/present and actual input-to-visible latency.

The three inputs are the same generated, crowded148 and seven-portal fixtures, with **no temporal history**. All 12 runs leave serialized Run/RNG state unchanged, round-trip the test save, and produce saved envelope bytes identical across all four runs within each workload. The test uses a fresh benchmark-only save directory, not player data.

| Desktop workload / operation | Baseline median / p95 ms | Merged median / p95 ms |
| --- | --- | --- |
| generated / Main view | 5.212 / 6.670 | 3.911 / 6.195 |
| generated / Hover + dirty refresh | 12.018 / 16.444 | 7.023 / 11.097 |
| generated / Save encode | 31.235 / 41.790 | 27.776 / 31.663 |
| generated / Forced verified test save | 31.638 / 44.469 | 28.971 / 47.912 |
| generated / CPU paint 1080p | 8.415 / 10.537 | 4.263 / 6.326 |
| crowded148 / Main view | 9.722 / 11.995 | 4.535 / 6.271 |
| crowded148 / Hover + dirty refresh | 27.262 / 47.609 | 14.056 / 16.074 |
| crowded148 / Save encode | 25.997 / 30.013 | 28.710 / 42.188 |
| crowded148 / Forced verified test save | 27.284 / 33.959 | 30.354 / 43.459 |
| crowded148 / CPU paint 1080p | 6.988 / 10.308 | 4.764 / 7.361 |
| seven-portals / Main view | 107.338 / 133.834 | 37.317 / 66.823 |
| seven-portals / Hover + dirty refresh | 506.254 / 584.120 | 45.908 / 55.065 |
| seven-portals / Save encode | 25.962 / 30.970 | 27.055 / 35.291 |
| seven-portals / Forced verified test save | 27.238 / 31.628 | 27.556 / 32.130 |
| seven-portals / CPU paint 1080p | 11.727 / 16.209 | 7.543 / 9.611 |

At 1280×800, merged paint medians are **2.486 / 2.909 / 4.615 ms**, and p95 **4.368 / 4.075 / 6.550 ms**, for generated / crowded / portal respectively. These game-scene inputs differ from the isolated avatar/menu stress scenes below. Their figures are not interchangeable baselines.

Inspection medians are **0.866 / 2.255 / 3.118 ms**; route medians **0.747 / 2.067 / 1.165 ms**. The composed hover operation already calls inspection and route twice, main observation once, and possible-exit checking once: **do not add the isolated rows to it**. These inputs exercise effectively empty possible-exit checks, not every linked-hover case. Unchanged-save guards remain tiny; forced save includes encoding, verified prior-file comparison, atomic write and fsync. Save/rewind code is not optimized here; the 27–29 ms encode variation is not credited to this release and does not characterize history-heavy saves.

The seven-portal main view remains **37.317 ms median / 66.823 ms p95**, and hover **45.908 / 55.065 ms**. Enabled profiler spans place most remaining view work in the **own/uninstrumented portion of `desktop.observe`**: 33.248–37.625 ms per sample across the two passes, versus 0.980–1.232 ms in `vision.observe_terrain` exclusive and 0.261–0.273 ms in scalar optical routes. These are **arithmetic means from aggregate spans divided by 15 samples**, not the medians above. They identify an owning region, not an isolated cause. The result does not justify assigning all remaining time to lighting, rays or a particular desktop helper.

## Why visibility and lighting cost less

The implementation batches exact work that many targets used to repeat. It does **not** replace the visibility rule with outward shadowcasting, approximate ray symmetry or an all-tile graph.

```text
Core command
  resource history / visibility masks / remember
    immutable VisibilityProjection or optical + lighting projections
      exact dependency scans
      geometry changed? build endpoint-to-tile directed rays
      origin missing? build observer distances
      light changed? build ordered source field
      target queries read those fields
  mutable AI decisions
    scalar optical route + sparse light
    same-call LOS/light reuse; no cross-mutation cache

Desktop dirty refresh
  observe_palette → one lazy actor VisibilityProjection
  detached View → Painter
    clipped row fills / image sampling / glyph-mask blends
    bounded glyph caches + one-size menu anatomy cache
  SDL upload and presentation remain outside these measurements
```

The optical mask captures intrinsic wall, tree and closed-door opacity once per distinct floor in each immutable pass. A clean pass still pays the exact **O(N + K)** dependency guard; this is not an O(1) dirty flag. Geometry changes rebuild endpoint-to-cell ray distances. An uncached observer runs endpoint Dijkstra and combines those distances with exact origin-to-cell rays. Up to **two geometries** and **eight origins per geometry** are retained. No per-ray dirty patch is implemented.

Actor light queries need to exclude the target's own active Shine without excluding other light at the same tile. A cell therefore keeps the best positive contribution and the best contribution with a different exclusion identity. Positive maximum/clamp and darkness subtraction retain their original order and exact floating-point behavior. Inactive Shine is not excluded. No active Shine means no extra exclusion arrays; with it, extra storage is approximately **24 bytes per cell**, platform-layout dependent, for at most two plane fields.

Scalar AI/event queries remain scalar because gameplay can mutate sources, actors, doors and planes repeatedly within one turn. They now reuse the route's distance and existence in the same call, skip light when no optical route exists, and reject distant sources before expensive ability/plane work. They still scan actors. There is no skipped RNG draw, reordered AI loop or cached effective-stat snapshot across mutation.

### Exact behavior retained

- Directed Bresenham raster ties and opaque destination visibility; reversing a ray is not assumed equivalent.
- Portal distance is the minimum **complete route length**, summing ordinary segments with zero-cost transfers. Blocked transfer endpoints, multiple portal chains and route tie order remain meaningful. Public route point sequences still use the original route solver.
- Waking/dream planes, clone and perfect-ally viewpoints, reveal, telepathy and nightmare exceptions retain their rules. Target-specific darkness modifiers remain distinct from raw cross-plane light.
- Cache dependencies include same-tick direct edits, source identity and active status. Caches are excluded from serialization and canonical equality, and reset on clone/snapshot/load/temporal restore.
- Painter clipping, nearest-neighbor sample coordinates, minimum positive strokes, integer alpha, glyph order, outline overlap and destination-alpha quirks remain exact. Animation math, resolution and artwork are unchanged.

### Core function costs

Symbols: **N** cells, **K** portal endpoints (authored maximum 14), **R** ray length, **P** viewpoints, **G** distinct viewed floors (at most 2), **A** actors, **B** ability/active entries, **L** fixed lights, **F** existing floor-selection cost, **T** telepathic work per cell, **S** positive emitters, **D** darkness emitters, **Fₛ** a source footprint. **C** includes viewpoint collection, exact dependency guards and any cold light/geometry/origin builds; it must not be omitted. Hash lookups are expected O(1), worst O(P) for the viewer map. Opacity capture also inherits door-map lookup and numeric-string key formatting; its linear guard bounds assume the existing expected hash behavior and bounded key widths, not adversarial worst-case hash lookup. Counts bounded by current content are stated rather than hidden.

| Function or boundary | Time including relevant helpers | Additional storage |
| --- | --- | --- |
| `optical_cache::Dependencies::capture` / `open` / `point` / `clear` | Capture O(N+K); bounds/conversion O(1); directed ray O(R) | O(N+K) dependencies; O(1) ray scratch |
| `OpticalField::build` | Cold O(KNR+K), only when geometry differs | O(KN+N) |
| `OpticalField::distances` | Warm O(8); new origin O(K²+KR+N(R+K)) | O(N+K) new work; up to 8N retained values |
| `OpticalCache::field` / `clone` / `eq` | Field O(N+K) exact guard plus dirty build; clone/reset and equality O(1) | At most two geometry entries; not a constant byte cap |
| `OpticalProjection::new` / `distance` | Setup O(PF+G(N+K)+GKNR+P(K²+KR+N(R+K))) if all fields cold; distance expected O(1), worst O(P) | O(PN) live origin fields plus handles and geometry |
| `lighting_cache::exclusion_key` / `LightField::at_excluding` | O(1) each | O(1) query scratch |
| `lighting_cache::Dependencies::capture` | O(F+N+A B+L), per immutable light pass | O(N+A B+L) captured dependencies |
| `lighting_cache::Dependencies::build` | O(N+S+Σ(Fₛ R)+DNR), on a dirty field | O(N) field; exclusion arrays only when needed |
| `LightingCache::field`; projection construction | O(F+N+A B+L) exact capture/equality plus dirty build; at most two planes | Bounded entry count; O(N+sources) per field |
| `LightingProjection::in_plane_excluding` / `current_excluding` | O(1) after construction | O(1) |
| `tile_chance_projected` | Expected O(F+B), worst O(F+B+P) | O(1) scratch after fields |
| `VisibilityProjection::new` / `lighting` / `geometry` | New O(A B) viewpoint collection; lazy access O(1), first optical call adds setup C | O(P) handles, shared fields |
| `VisibilityProjection::light` / `actor_chance` | Light O(B); actor chance expected O(F+B), worst O(F+B+P); first access adds C | O(1) query scratch |
| `VisibilityProjection::directly_visible_actor` / `visible_actor` | Worst O(P(P+B+F)); visible actor additionally pays unchanged telepathy/area checks; first access adds C | O(1) query scratch |
| `optical_route`; `tile_chance_lit` | O(F+R) without portals; O(F+K²R) with portals; one route supplies existence and distance | Route/helper scratch O(K), plus returned route |
| `illumination_in_plane` | O(F+fire-window×R+LR+A+A_near B+near-emitter ray work); worst all actors near | O(1) scratch |
| `actor_chance_sample` / `actor_chance` | O(F+route+sparse light+B); no-route return avoids light | Route scratch; same-call values only |
| `detection_chance` | O(effective stats+route+sparse light); second light only for cross-context zero-chance fallback | Route scratch |
| `terrain_visibility` / `terrain_visibility_chance` | Expected O(C+N(T+P(F+B))); worst viewer hashing adds O(P) per query; reveal shortcut O(N) | O(N+A+PN) fields/result |
| `native_terrain_visibility` | Expected O(C+sensed-actor queries+N(A+P(F+B))) | O(A+PN+N) |
| `observe_terrain` | Expected O(C+N(P(F+B)+T+metadata work)+source admission/footprint work); full observation is not cached | O(N+PN+hazard metadata+source footprints) |
| `begin_resource_turn` / `observe_resource_history` | O(C+A(Q+M)); history adds unchanged O(H A) matching, Q visibility query, M meter/stat derivation | O(A M+P) batch scratch plus shared fields |
| Desktop `observe_palette` | One lazy projection per actor batch; O(C+A Q) visibility portion; unchanged terrain, sorting, layout and projection work remain additional | Existing detached view plus shared fields |

Source admission in `observe_terrain` still scans actor abilities/planes and can call scalar actor visibility for emitting actors; disclosed footprints still expand rays. Neither is hidden inside an O(1) source label. These are release-boundary costs, not a claim that all historical project functions have been annotated. Detailed source comments and the linked ledgers cover new/modified helpers and diagnostic/test harness costs. The **project-wide historical function annotation audit remains incomplete**. Cache eviction also drops retained storage; bounded entry counts do not make byte usage constant as floors, glyphs or drawable dimensions grow.

## Bulk painting results and costs

The final comparison uses normal release-profile test-harness executables calling production Painter methods, `runtime-timing` off. Each operation is warmed, then sampled **51 times per run** in **A/B/B/A** process order, with two runs per side. The table reports the **median of the two run medians**, not a pooled 102-sample median. The final quiet window began at 04:09:58 UTC on 31 October 2026.

These timings include CPU software painting only, not Painter startup/decode, cold glyph or anatomy-cache construction, SDL upload, present/vsync, GPU work or input handling. The synthetic dense scene alters detached draw cells; it is not a claim about reachable populations or simulation performance.

| Paint workload | Resolution | Baseline ms | Candidate ms | Reduction |
| --- | --- | --- | --- | --- |
| Repeated compound glyphs | 1280x800 | 6.779 | 2.782 | 59.0% |
| Game avatar room | 1280x800 | 6.152 | 4.603 | 25.2% |
| Wendigo manual | 1280x800 | 3.437 | 1.866 | 45.7% |
| Animated menu frame 37 | 1280x800 | 21.102 | 6.175 | 70.7% |
| Prose | 1280x800 | 1.086 | 0.363 | 66.5% |
| Synthetic dense animated scene | 1280x800 | 11.207 | 4.560 | 59.3% |
| Text | 1280x800 | 1.314 | 0.446 | 66.0% |
| Repeated compound glyphs | 1920x1080 | 9.598 | 4.908 | 48.9% |
| Game avatar room | 1920x1080 | 9.911 | 7.175 | 27.6% |
| Wendigo manual | 1920x1080 | 5.726 | 3.065 | 46.5% |
| Animated menu frame 37 | 1920x1080 | 40.292 | 10.764 | 73.3% |
| Prose | 1920x1080 | 1.436 | 0.631 | 56.1% |
| Synthetic dense animated scene | 1920x1080 | 17.667 | 7.432 | 57.9% |
| Text | 1920x1080 | 2.186 | 0.842 | 61.5% |

The 1080p final menu p95 is **14.707 / 16.846 ms** in its two candidate runs; the game p95 is **10.097 / 9.684 ms**. Shared-host tails remain material. The unchanged `clear` control is noisy, and the 1080p opaque-background difference is only 2.4%; neither is presented as a demonstrated causal optimization. The whole-frame gains repeat in both pairs, but no particular instruction-level cause is isolated by those comparisons.

Uniform pixels now use clipped native row spans. Glyphs reuse exact coverage masks, while image paths hoist clipping and sampling work without changing nearest-neighbor results. The menu retains a **single-size** occupied-sample anatomy cache, not a phase/time cache or a reduced-resolution image. At 1080p it retains **3,942,960 bytes (~3.76 MiB)** of vector capacities, including heart-column scratch and excluding allocator metadata/source assets. Cold build or resize costs O(source pixels + destination area) and was not independently timed in the warm benchmark.

### Painter function coverage

The following is the modified-function audit. Font metric/raster work is not assumed free; hash-table warm-cache bounds are expected, not adversarial worst-case lookup guarantees.

In the authoritative table below the original local notation is used: P = framebuffer pixels, A = clipped destination pixels, G = text characters, M = mask area, F = font raster work, H = Chronicle rows, S = compound source area, K = cache key bytes.

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

| Additional menu helper | Time and storage |
| --- | --- |
| `menu_raster::Raster::new` | Cold O(source-heart pixels+destination area); retains occupied veins/foreground+native width |
| `menu_raster::Raster::paint` | Warm O(occupied veins+occupied foreground+native width+native height+transformed heart bounding-box area); reusable heart-column scratch |
| `menu_raster::blend` / `storage` | O(1) exact integer blend / diagnostic capacity accounting |
| `beating_emblem` follow-through | Construct/reuse one-size cache; same veins→heart→foreground order and exact axis sampling |
| `picture` follow-through | O(A) clipped area; O(1) scratch; exact alpha255 copy, alpha0 destination-alpha behavior, mixed-alpha blend |
| `menu_shape` | O(H E log E+A), fixed E≤32 scratch; horizontal runs preserve worn-cut predicate |

Prose and compound-glyph caches cap entry count at 2048 and 512 respectively; eviction/resize drops retained entries and can incur allocator cleanup. Masks may be large, so these are **not fixed byte budgets**. Layout, wrapping/metrics, draw-list creation, effect sorting, animated color work and full-buffer upload are deliberately retained. Some wrapping remains potentially quadratic in text length. Bulk drawing remains O(painted area), not O(1).

## Remaining bottlenecks and the next useful test

The scope profiler supplies attribution, **not the headline speed result**. It records 15 samples for each of six commands on each side: **180/180 exclusive partitions exactly equal their root's inclusive time, zero failures**. Parent exclusive scopes exclude instrumented children but retain uninstrumented helpers and bookkeeping. **Inclusive scopes overlap; medians and median shares are not additive.** Profile-on versus separate normal-build medians range approximately −11.6% to +49.2%. Negative differences reflect noise/order/build layout, not a benefit from instrumentation.

| Workload | Remaining profiled exclusive median | Interpretation |
| --- | --- | --- |
| Generated Wait | Light dependencies 0.416 ms; light builds 0.340 ms | 10 checks and 2 builds; exact validation remains real work |
| 148-actor Wait | Movement 2.044 ms; AI own work 1.510 ms | 130 movement calls; movement includes cloning, gates, occupancy, interactions and effects |
| 148-actor Pickup | Resource history 0.592 ms; light dependencies 0.572 ms | No isolated proof that O(H A) history matching dominates |
| Seven-portal Wait | Gas diffusion 1.261 ms; optical build 0.836 ms | Gas is unrelated to visibility; cold endpoint preparation remains |
| Seven-portal Pickup | Optical build 0.739 ms | 31% median profiled root share, not a normal-build share |

In crowded Wait, scalar light calls fall **730→10**, and their profiled exclusive median falls **14.323→0.066 ms**. Seven-portal Wait scalar routes fall **4,837→31**, with exclusive time **67.759→0.301 ms**. The work is partly transferred into fields and exact guards, not erased. Optimized Pickup has zero scalar light/route calls in these profiles but still does resource, observation and finish work.

Movement's positional `.iter().find(...)` remains a potential O(A) per-step scan; stable-ID indexes do not make coordinate search indexed. Its measured 2.044 ms cannot all be assigned to occupancy or cloning without narrower spans. The crowded path field was built once in this workload (0.396 ms), not permanently once per floor. Lighting validation can allocate/compare O(N+A B+L) dependencies each pass. Optical dependency capture has no independent span and can be charged to its caller.

For painting, independent final menu probes measure image sampling/blit **4.716 ms**, anatomy **1.941 ms**, procedural fire **1.499 ms** and title shapes **0.798 ms**. These are **overlapping microbenchmarks, not an exclusive partition**, so they must not be added. Full-frame image sampling/copying is the largest measured menu component and remains O(P).

The final merged desktop comparison now shows that **the next profiling target is the own work inside `desktop.observe` on the portal fixture**, not another blind rewrite of core rays. Add exclusive child spans around detached appearance/source-footprint processing, actor/item projection and draw-cell construction, then repeat the same three inputs in both orders with exact output checks. Those candidates are measurement boundaries, not established causes. A later native Windows input-to-present trace is needed to establish actual interactive latency, including event handling and upload/present. Save measurements here are diagnostic only; save-policy and save/rewind optimization remain separate scope.

Before optimizing movement further, add narrower spans for positional occupancy, actor cloning and decision helpers, then repeat the crowded workload. Before replacing conservative guards, establish complete mutation ownership; a tick-only key is unsafe. Before pursuing another menu optimization, measure cold resize/startup and exact pixel output as well as warm paint.

### Proposed budgets and what is not yet achieved

The proposed goals are **16.7 ms input-to-visible**, p95 core component **5 ms for ordinary / 10 ms for stress commands**, p95 hover **2 ms**, and p95 paint **4 ms**. These are goals, not an additive budget or a completed gate. Core Wait p95 is 5.784 ms generated, 12.832 ms crowded and 12.803 ms seven-portals: the proposed Wait targets are **not yet met** in these samples. Several warm paint medians already exceed 4 ms, including 1080p game and menu. All three merged hover p95 values (**11.097 / 16.074 / 55.065 ms**) miss the proposed 2 ms target. Merged 1080p paint p95 (**6.326 / 7.361 / 9.611 ms**) also misses 4 ms. These measured components overlap and must not be summed; actual end-to-end latency is **not measured**.

## Verification and release status

Completed verification evidence:

- Final correctness totals: **1,000 passed, 0 failed, 7 ignored** across **136 core targets + 5 desktop targets**. Core contributes 683 passed/2 ignored; desktop contributes 317 passed/5 ignored. Opt-in evidence runs are recorded separately; ignored benchmarks are not silently counted as suite passes.
- Normal cross-build core comparisons: identical canonical outcomes for all six command cases; final retained `observe_terrain` outputs and inputs match across all three pairs; intermediate fresh-origin outputs are not all byte-compared.
- Exact directed optical differential tests cover every origin/target on deterministic blocker maps with 0, 1, 3 and 7 portal pairs. Lighting differentials compare exact f64 bits, source exclusion/order, planes, identity/activation, same-tick edits and clone resets.
- Projected actor visibility matches scalar queries across 16 maps, 33 actors and nine mutation phases. Frozen-v16 scalar-light/detection oracles cover the scalar follow-through.
- Final compact Windows candidate: cross-build and archive validation pass; native Windows launch/input/render/save behavior remains untested. The candidate is not described as published before release receipts.
- Final merged core+renderer RGBA gate: **100/100 whole frames byte-exact** against original v16; the final size/SHA256 manifest is retained separately from isolated painter evidence.
- Isolated final painting evidence: **100/100 full RGBA frames byte-identical** across five resolutions and the scene matrix; **1,800 anatomy states** (360 phases ×5 resolutions) plus **45 moved/resized/clip/arbitrary-alpha cases** match the frozen renderer.
- Merged desktop correctness suite: **all 5 targets passed; 317 passed, 0 failed, 5 ignored** (264 main + 53 integrations), using the optimized correctness profile. Core doctest invocation passed with 0 tests; it is not an additional behavioral test count.
- Final desktop diagnostic: all 12 test-save envelopes match within workload, all state/RNG immutability and roundtrip checks pass; no temporal-history inputs.
- Latest isolated menu-cache release main test target: **264 passed, 5 ignored**. Opt-in parity/cycle tests were run separately and passed. Earlier checkpoint counts are not substituted for final merged gates.

| Final integrated gate | Status | Required closing evidence |
| --- | --- | --- |
| Core final suite | PASS — 136/136 targets | 683 passed, 0 failed, 2 ignored. Optimized correctness profile: opt1/debug0, debug assertions and overflow checks true. |
| Merged desktop whole suite | PASS — all 5 targets | 317 passed, 0 failed, 5 ignored: 264 main-target tests + 53 integration tests, optimized correctness profile. |
| Merged 100-frame RGBA gate | PASS — 100/100 exact | Final integrated core+renderer correctness build matches original v16 baseline frames byte-for-byte; size and SHA256 manifest retained. |
| Final desktop A/B | PASS — measurement completed | 15 samples × 2 reversed passes; pooled 30 collector-off samples in feature-on builds. All 12 test-save envelopes match within workload; state/RNG unchanged; roundtrips checked. |
| Compact Windows cross-build and package | PASS — candidate artifact, not publication | Final build 5 min 36 s; PE imports, strip section equality, notices and ZIP checks pass. Native Windows runtime remains untested. |
| Native Windows interaction and input-to-visible | Not measured | Real Windows execution and end-to-end event/upload/present evidence; cross-compilation is insufficient. |

### Packaged Windows candidate

`corpse-gods-native-r39-visibility-paint-windows.zip` is **19,657,290 bytes**. Its SHA256 is:

```text
a518d3939d5a18fa67af55a96c59cff06bd8f14728b32c7808e66a9c2aac0645
```

This verifies the retained candidate archive, not Windows runtime behavior or publication. All integrated correctness gates are complete. Full-source packaging/publication is handled separately; this report does not claim publication receipts.

## Reproduction and evidence map

Use the complete source tree, pinned Rust 1.99.1, locked dependencies and SDL2 where required. Do not use a code-reading-only archive to reproduce the desktop. Keep CPU/disk-heavy jobs out of a timing window; preserve raw samples, both process orders and exact-state/pixel checks. Do not compare compact size-optimized Windows builds directly to the opt3 timing rows as though they were the same build profile.

```sh
# Correctness from the complete source root
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_OVERFLOW_CHECKS=true CARGO_INCREMENTAL=0 tools/cargo.sh test --workspace --locked

# Focused exactness gates
tools/cargo.sh test -p corpse-core --lib optical_cache --locked
tools/cargo.sh test -p corpse-core --lib lighting_cache --locked
tools/cargo.sh test -p corpse-core --test visibility_projection_cache --locked
tools/cargo.sh test -p corpse-core --lib scalar_light_culling_tests --locked

# Normal core observation and command harnesses; run separately on both trees
tools/cargo.sh run --release -p corpse-core --example visibility_bench -- /tmp/visibility-evidence 30
tools/cargo.sh run --release -p corpse-core --example timing_test --   --input /tmp/visibility-evidence/generated-input.json --samples 30 --warmup 3   --output /tmp/core-normal.json --state-dir /tmp/core-states

# Diagnostic desktop; feature-on measurements are not feature-off speed results
tools/cargo.sh run --release -p corpse-desktop --features runtime-timing --   --timing-test /tmp/desktop.json --timing-input /tmp/visibility-evidence/generated-input.json   --timing-samples 15 --timing-warmup 3
# Repeat with --timing-reverse to expose collector order effects.

# Painter tests, warm microbenchmarks and exact frame capture
tools/cargo.sh test --release -p corpse-desktop --bin corpse-desktop --locked
tools/cargo.sh test --release -p corpse-desktop --bin corpse-desktop   bulk_paint_benchmark --locked -- --ignored --nocapture
PAINT_EVIDENCE_DIR=/tmp/paint-evidence tools/cargo.sh test --release   -p corpse-desktop --bin corpse-desktop exhaustive_paint_evidence   --locked -- --ignored --nocapture
```

The benchmark desktop uses only its newly created test-save directory and does not access normal player profile/autosave paths. Ordinary tests have no wall-time thresholds. Fresh benchmark output directories are recommended to keep evidence from different builds distinct.

Authoritative local evidence and source contracts:

- `visibility-r39/reports/normal/summary.json` — feature-off command and cold/warm/fresh-origin statistics; per-pair raw JSON and retained states live alongside it.
- `visibility-r39/reports/profiled/scope-summary.json` and `.md` — every scope median, count, partition and instrumentation comparison; `remaining-scope-attribution.md` in the same directory explains attribution limits.
- `visibility-r39/reports/desktop/summary.json`, raw per-case/per-side/pass JSON and `exact-save-verification.json` — final merged desktop collector-off comparison and save/state checks.
- `painting-r39/evidence/final-quiet-timing.json` — final painter samples and paired summary. `final-quiet-run-order.txt`, final host logs, `menu-cache-pixel-parity.json` and `menu-cycle-oracle.log` retain provenance/correctness.
- `source/verification/visibility-paint/final-test-gates.json` — final integrated counts, exact-pixel status and assertion/overflow-enabled correctness profile.
- `visibility-r39/windows-build-final.log` and `package/corpse-gods-native-r39-visibility-paint-windows.verification.json` — final compact Windows build/archive provenance.
- `visibility-r39/qa/merged-rgba-verification.json` and `merged-rgba.log` — final integrated 100-frame exact-pixel gate.
- `visibility-r39/qa/desktop-opt1-runner.log` and `desktop-opt1-tests/` — final merged desktop suite. `core-doctests.log` records the zero-test doctest invocation; `documentation-only-vision.json` verifies the later four-line comment correction changes no executable logic.
- `visibility-r39/qa/benchmark_normal.sh`, `benchmark_profiled.sh` and their summarizers — exact local A/B orchestration. `TEST-PROFILE.json` records the correctness profile, not a timing build.
- Repository contracts: [optical visibility](OPTICAL-VISIBILITY-CACHE.md), [actor light exclusion](ACTOR-LIGHT-CACHE.md), [bulk painter audit](painting-bulk-audit.md), [painter verification](painting-bulk-verification.md), [desktop timing](desktop-timing-test.md), [complexity ledger](complexity-ledger.md), and [Windows cross-build](windows-cross-build.md).

**Evidence interpretation:** measured improvements, asymptotic bounds and bounded regression coverage are different claims. This report keeps them separate. A completed component test is not proof of whole-game, whole-platform or end-to-end performance.
