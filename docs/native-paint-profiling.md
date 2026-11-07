# Native paint profiling harness

This test-only diagnostic locates actual software-paint costs before changing the
production renderer. The frozen resolution candidate remains untouched. It does
not measure SDL upload/present, Windows latency, event throughput, simulation,
save/load or observation time.

## Run, only with an otherwise quiet CPU

Build with the pinned toolchain and the same release settings as the resolution
matrix. The ignored test requires an explicit normalized actual tick48 fixture,
source-variant label and a **new** report path:

```sh
CORPSE_PAINT_INPUT=/workspace/scratch/69f53637ba6f/overnight-timing/reports/live-probe/roundtrip.json \
CORPSE_PAINT_OUTPUT=/absolute/new/report.json \
CORPSE_PAINT_VARIANT=preopt \
CORPSE_PAINT_SAMPLES=20 \
tools/cargo.sh test --release -p corpse-desktop \
  render::paint_profile_bench::real_tick48_native_paint_profile \
  -- --ignored --exact --nocapture --test-threads=1
```

Collector unit tests match `paint_profile_nested_exclusive_and_disabled_control`
and `paint_profile_summary_preserves_raw`. Normal workspace tests remain required.
Do not interpret a compilation-only gate as runtime or regression validation.
Archive the source hash, exact build/run command, toolchain and report together.

## Workload and controls

- Actual tick48 detached View, default appearance, cleared helper provenance and
  empty notice, matching the resolution matrix's genuine live-game presentation.
- Game and real selected-first-item Pack at 1280x800, 1920x1080 and 3840x2160.
- Warm full paint uses n paired samples per arm. Native caches are invalidated by
  real resize for three additional first-paint samples per arm. Resize allocation
  and constructor/font/asset setup are outside the paint interval.
- One real post-Move observed frame at 1920x1080, if one of eight bounded direction
  probes moves the player. Probes dispatch only on independent engine clones,
  outside timing; no actors, effects, reveal flags or gameplay are fabricated.
- Actual cosmetic clock1000-to1125 and observed pointer/route projection at
  1920x1080. The report discloses whether the clock changes any pixels rather than
  presenting a zero-animation scene as animated load. The hover case is game
  highlighting, not an invented creature tooltip.
- Transition rows first paint the previous scene outside timing, then measure the
  changed full frame. This is full painting, not retained/damage planning.
- Collector OFF and ON alternate order per pair in one release test binary.
  Raw records keep pair/order, whole-paint elapsed, CRC and that same sample's
  subspans. Every paint must match the expected exact framebuffer CRC. This is a
  detector for instrumentation changing output, not a replacement for pixel
  comparison against the frozen baseline after algorithm changes.
- The source engine save/RNG/replay must remain unchanged. The independently moved
  clone is also unchanged by painting. No fixture or existing report is written.

## Reading attribution

**Inclusive times overlap. Never add inclusive category totals.** Each scope also
records exclusive time, subtracting immediate child intervals. The sum of all
exclusive values must equal the same paint's root inclusive value. Outer wall
clock minus the root is reported separately. The primitive-by-layer table assigns
nested primitives to the nearest scene layer without moving text/mask time into
both totals. It is another view of the same measurements, not additional work.

Clear, opaque fill, alpha fill, ordinary masks, alpha1/nonunit scaled masks,
alpha1/nonunit ASCII, compound preparation, text, role text, metrics, prose,
pictures and Chronicle fade have separate scopes. Game layers include background,
Chronicle, map, HUD sections, enemies, interactions, footer and Pack. Animated
terrain and dream reconstruction are explicitly preparation, not raster costs.

Pixel visits are actual clipped raster-loop visits, including zero-coverage mask
samples and repeated overdraw. They are not unique changed pixels, nonzero writes
or total upstream samples. There are no per-pixel timers. Existing raster counters
provide the whole-frame count; sparse native-pixel work not attributed by the
batch guards is reported as a residual, and its time stays in its enclosing scope.
Cache misses count text/styled-text/ASCII/compound insertion events, not heap size.

ON includes timers, span bookkeeping and raster counter overhead. It is diagnostic
and must not be called production latency. OFF disables collection and counters
but still executes test-only guard checks. Non-test binaries compile all new
instrumentation out. Compare OFF/ON and the frozen resolution control instead of
assuming either instrumentation path is free. No per-pixel clock or collector
lookup was added. Cold n3 is only diagnostic, not a meaningful tail estimate.

## Complexity and bounded memory

Let K=29 fixed scope kinds, L=14 fixed scene kinds, D<=32 nesting depth, R the
number of primitive/scene calls, Q the raster rows visited, P the clipped raster
pixel visits, and n in [3,100] requested paired samples. Each ON span start/end
uses O(1) time, no heap allocation, and two monotonic clock reads. Each batch's row
counter adds O(1), so instrumentation is O(R+Q), separate from existing O(P) work.
OFF pays O(R+Q) conditional guard/counter checks in test binaries only. Existing
raster stats still conditionally account for native sparse pixels.

The thread-local collector uses fixed O(D+K+KL) storage, independent of calls,
resolution and frame size. No growing event list is retained while painting.
Reset, fixed-array snapshot, CRC scan O(framebuffer bytes), JSON conversion and
summary O(n log n) occur outside the timed root. Reports retain O(n(K+KL)) bounded
stats across nine maximum workload rows; absent categories are omitted. Painting
retains its existing framebuffer, font/mask caches, observation and scene memory.
The movement setup is at most eight independent original-engine clones and actual
commands in sequence, with one successful result retained, outside timed work.
No timing result is claimed until the benchmark and controls have actually run.
