# Bulk painting verification

## Baseline and build

Baseline: released full source v16 (creature-index r39), unchanged core. Candidate:
render.rs batching/caches, menu_raster.rs one-size anatomy sampling cache, and one
inspection tooltip backing call, plus tests/documentation. Pinned official Rust 1.99.1, normal Cargo release settings (opt3,
thin LTO, debug1); no runtime-timing feature in painter test binaries. No resolution
reduction or new assets. See `painting-bulk-audit.md` for implementation details.

## Correctness

- Final expanded cross-build comparison: **100/100 entire RGBA frame buffers
  byte-identical** to the original painter. SHA256 per image retained in
  `pixel-parity.json`; this is exact comparison, not perceptual tolerance.
- Resolutions:1280×800,1920×1080,900×720,720×900,320×200. Cases include game,
  animated draw, four menu/heartbeat phases, start, blessing overlay, history,
  four bestiary pages, Pack, inspection, hover, styled typography, gas/reflection
  bands, explosion overlay, and an arbitrary-RGBA primitive/clipping scene with
  every alpha value, fallback/compound/italic glyphs and scaled picture samples.
- Scalar-reference fill regression checks all 256 alpha values and clip variants,
  including initially nonopaque destination bytes. Fade checks all channel values.
- Initial desktop debug suite: **317 passed,2 ignored**, all5 targets. Pre-menu-follow-through
  release main-target rerun: **264 passed,3 ignored** (benchmark/evidence tests are
  explicitly opt-in). Final expanded parity harness separately passed. That main
  release rerun included the image-stepper/effects/tooltip changes; the later
  menu-cache final gate is reported below.
- Final combined project gate and Windows cross-build belong to the integrated
  release verification, not this isolated painting report. No Windows runtime
  verification, GPU upload/present timing, user-profile writes or live gameplay
  session are claimed here.

## Timing protocol

Only quiet, normal-release paired runs are reportable. The first correctness
capture logs also contain incidental timings; those ran alongside compilation
and **must not be used as performance evidence**. Fresh fixed-scene benchmark
processes warm each operation, retain51 raw nanosecond samples, report median
and nearest-rank p95, then repeat in reversed baseline/candidate process order.
Compiler/test jobs in the collaborating tasks are paused or completed first.

The synthetic dense scene replaces detached draw-cell glyphs/animations, not
simulation actors. It is a raster stress test, not a claim about reachable actor
populations. Single-function microbenchmarks overlap full frames and their medians
must not be summed.

## Final follow-through results

Final A/B/B/A window:2026-10-31 04:09:58–04:10:34UTC, collaborating heavy jobs
paused/completed. **Release-profile test-harness executables call the production
Painter methods**, using pinned1.99.1, opt3/thin-LTO/debug1 and runtime-timing off.
These are not measurements of the shipped Windows executable or SDL presentation.
Same51 warm samples/run; below is median of the two run medians. Menu uses fixed
frame37; periodic animation correctness is separately checked for the full cycle.

| Path | Original v16 ms | Final candidate ms | Measured reduction |
|---|---:|---:|---:|
| 1280x800/compound_glyphs | 6.779 | 2.782 | 59.0% |
| 1280x800/game_avatar | 6.152 | 4.603 | 25.2% |
| 1280x800/manual_wendigo | 3.437 | 1.866 | 45.7% |
| 1280x800/menu | 21.102 | 6.175 | 70.7% |
| 1280x800/opaque_background | 0.645 | 0.579 | 10.2% |
| 1280x800/prose | 1.086 | 0.363 | 66.5% |
| 1280x800/synthetic_dense_animated | 11.207 | 4.560 | 59.3% |
| 1280x800/text | 1.314 | 0.446 | 66.0% |
| 1920x1080/compound_glyphs | 9.598 | 4.908 | 48.9% |
| 1920x1080/game_avatar | 9.911 | 7.175 | 27.6% |
| 1920x1080/manual_wendigo | 5.726 | 3.065 | 46.5% |
| 1920x1080/menu | 40.292 | 10.764 | 73.3% |
| 1920x1080/opaque_background | 1.119 | 1.092 | 2.4% |
| 1920x1080/prose | 1.436 | 0.631 | 56.1% |
| 1920x1080/synthetic_dense_animated | 17.667 | 7.432 | 57.9% |
| 1920x1080/text | 2.186 | 0.842 | 61.5% |

The unchanged clear control remained noisy:1080p baseline medians1.058/0.488ms,
candidate0.489/0.733ms. No clear optimization is claimed, nor is the2.4%1080p
opaque-background difference treated as a robust causal gain. Whole-frame menu
and dense-scene gains are much larger and repeat in both pairs. Use raw p95 and
samples, not only averages: final1080p menu p95 was14.707/16.846ms; game p95
10.097/9.684ms. Shared-host timing variance remains; this is not a hard real-time
latency guarantee or a proof of a particular instruction-level cause.

Final isolated menu components: fire generation1.499ms, fire-image blit4.716ms,
anatomy1.941ms, title shapes0.798ms, embers0.0004ms. The prior follow-through
probe had anatomy13.003ms and fire blit8.086ms. These probes are independent
microbenchmarks, not additive exclusive attribution. The remaining largest
measured menu component is full-frame image sampling/copying, still O(P).
The final menu is~10.76ms rather than the first pass~21.8ms, and original~40.3ms
in this final paired window. Procedural fire, vein-wave math, heart sample blending,
layout and SDL upload are not claimed eliminated.

At1080p the one-size anatomy cache retains25,350 vein samples and222,849
foreground samples, with **3,942,960 bytes (~3.76MiB)** of Vec capacities including
heart-column scratch, excluding allocator metadata and the pre-existing source
assets. It does not retain all phases or prior resolutions. First-build/resize
cache construction cost is explicitly O(source + destination area); its cold
latency is not included in warm timings and was not independently timed here.

### Final correctness gates

- Final100/100 entire RGBA frames match original v16 (new final manifest retained).
- **1,800** complete anatomy states (360 phases ×5 resolutions) match the frozen
  scalar renderer, plus45 moved/resized/clip/arbitrary-destination-alpha cases.
- Final menu-cache release main target: **264 passed,5 ignored**. Expanded pixel
  evidence and full-cycle oracle were additionally run explicitly and passed.
- An independent read-only review identified the title-span batching opportunity
  and checked cache invalidation/coordinate/alpha risks; no defect was reported
  in the completed initial primitive audit. The executable oracle, not review
  alone, is the correctness evidence.
- Combined-core final verification and Windows cross-build remain the integrated
  release lead's responsibility; this report does not claim those stages.

Detailed final raw results:`final-quiet-timing.json`; component logs and retained
capacity output:`menu-components-before.log`,`menu-components-after.log`;
parity:`menu-cache-pixel-parity.json`; cycle and unit logs in the evidence archive.

## First-pass results (preserved history; superseded by final results above)

All collaborating compiler/test jobs were completed or paused for the A/B/B/A
measurement window (03:45:00–03:45:31 UTC,2026-10-31). Values below are the median
of the two run medians, each from51 warm samples. They are **whole-build measured
comparisons**, not a guarantee about all machines or every game state.

| Path | Baseline ms | Candidate ms | Measured reduction |
|---|---:|---:|---:|

| 1280x800/compound_glyphs | 6.398 | 2.583 | 59.6% |
| 1280x800/game_avatar | 6.253 | 4.052 | 35.2% |
| 1280x800/manual_wendigo | 3.292 | 1.701 | 48.3% |
| 1280x800/menu | 20.905 | 12.375 | 40.8% |
| 1280x800/opaque_background | 0.550 | 0.477 | 13.2% |
| 1280x800/prose | 1.071 | 0.347 | 67.6% |
| 1280x800/synthetic_dense_animated | 11.460 | 4.160 | 63.7% |
| 1280x800/text | 1.299 | 0.436 | 66.4% |
| 1920x1080/compound_glyphs | 9.932 | 3.961 | 60.1% |
| 1920x1080/game_avatar | 10.643 | 6.356 | 40.3% |
| 1920x1080/manual_wendigo | 6.388 | 3.003 | 53.0% |
| 1920x1080/menu | 42.939 | 21.832 | 49.2% |
| 1920x1080/opaque_background | 1.125 | 0.997 | 11.4% |
| 1920x1080/prose | 1.555 | 0.557 | 64.2% |
| 1920x1080/synthetic_dense_animated | 17.731 | 6.690 | 62.3% |
| 1920x1080/text | 2.212 | 0.722 | 67.4% |

### Controls and interpretation

**The unchanged `clear` control also changed substantially** between binaries
(1080p run medians1.034/1.139ms baseline versus0.401/0.429ms candidate;1280p
0.191/0.325 versus0.155/0.171ms). No clear-source optimization is claimed. Codegen,
code layout and host/cache variability have not been separately isolated; this
limits causal attribution and precision. Both repeated whole-frame pairs still
show the same direction and a substantial difference. Small opaque-fill differences
(~11–13%) should be treated cautiously, not marketed as a dramatic bulk-fill win.
The raw distributions, p95 and run order remain available for independent analysis.

Earlier inherited profile values (e.g.9.35ms background and14.62ms full paint)
were a different workload/build/profiling context. They are **not** the baseline
for percentages in this report. No speedup was computed by mixing those numbers
with these new measurements.

### What remained after the first pass (superseded by final results above)

- Animated main menu is the largest tested paint operation:~21.8ms at1080p,
  versus~6.36ms for the avatar-room game. It still computes procedural fire,
  samples three anatomy layers, computes vein waves/heart transforms and
  scan-converts title shapes. These subcomponents need separately instrumented
  measurement before assigning a precise share; none is assumed to dominate.
- Synthetic dense animated game remains~6.69ms1080p; pixels, glyph coverage and
  cosmetic color work still scale with drawn area. The test does not measure AI,
  visibility, saving, route calculation or a full input-to-present latency budget.
- Text measurement/wrapping, draw-list preparation, per-cell animated color work,
  effect mask allocation/order and SDL upload/present are intentionally retained
  as described in the audit. Runtime profiling of the combined game is the next
  authority for broader remaining hotspots.
- Caches avoid repeated glyph rasterization but still blend coverage each frame.
  GPU/dirty-region rendering, fewer pixels or visual changes were not introduced.
