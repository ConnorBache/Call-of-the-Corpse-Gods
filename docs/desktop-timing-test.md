# Desktop timing test (opt-in)

Build with `cargo build --release -p corpse-desktop --features runtime-timing`.
Run `corpse-desktop --timing-test desktop-timing.json --timing-samples 30 --timing-warmup 5 --seed 1`.
On Windows use `corpse-desktop.exe` with the same options (and SDL2.dll beside it).

`--timing-input path/to/save.json` instead observes a supplied native save read-only.
The benchmark never opens a window or reads/writes the usual appearance, profile,
replay or autosave locations. Its generated save test uses a newly created temporary
folder, whose path is recorded in the report. That small benchmark file is retained
for inspection. Input-file parsing and generated scene creation are not timed.

A normal build without the feature rejects `--timing-test` explicitly. Profiling
spans compile to no-ops without the feature; feature-enabled collection starts off.

## Measured operations

- Detached main view observation, tile inspection, route planning, and possible-exit
  checks separately, including failed/no-link cases.
- Ordinary mouse-cell event followed by dirty-refresh projections: inspection and
  route each twice, main view once, and possible exits once. This is the traced
  ordinary unblocked hover path, not every possible UI branch or event-loop latency.
- Ability-detail projection if the player has an ability.
- Save envelope encoding separately.
- Actual forced autosave to an already-created/verified test file, including prior
  file read/compare, encode, atomic write, and fsync; and unchanged-save guard alone.
  Debug/playtest saves disable autosave in the game, so those inputs report omission
  rather than bypassing the guard.
- Actual `Painter::game_at` software framebuffer rendering at 1280×800 and1920×1080,
  after painter construction/resize and warmup. No SDL texture upload, window
  compositing, GPU, vsync or input handling is measured here.

Samples contain wall nanoseconds and nearest-rank p95, conventional median, minimum
and maximum. Result disposal is included in wall time; callee spans end before caller
result disposal. Each operation is measured with collection disabled then enabled,
with warmups before each. Add `--timing-reverse` in a second invocation to run enabled first. This makes profiling overhead visible but is **not** a
feature-free binary comparison: the feature-enabled disabled branch still executes.
Use repeated process runs (and the main harness's feature-free comparison) to assess
noise and order effects. Do not compare results from a busy build host as if idle.

Span rows contain call count, inclusive nanoseconds and exclusive nanoseconds.
Inclusive times overlap; do not add parent and child inclusive times. Isolated
operations also overlap the composed hover operation; do not add their medians to
estimate one full command. Warmup and JSON reporting are outside counters/timers.

Game state is serialized before/after every benchmark invocation and compared exactly;
its RNG, tick, actors, history and other serialized fields must be unchanged. The
actual saved test file is decoded and compared as well when autosave is supported.
No gameplay command is advanced by this desktop test.

## Complexity of the measurement code

For n samples, w warmups, operation cost T, and L profiling labels, measurement costs
O((n+w)T + n log n + recorded-span aggregation); sample storage is O(n+L). Sorting and
JSON reporting are outside operation samples. Optional counters add O(log L) work per
closed span and O(nesting depth + L) storage. The operation's real nested costs remain
those of the game, not O(1) merely because this wrapper calls it once. Temp-file encode
and roundtrip checks cost O(serialized bytes + validation); framebuffer storage is
O(width×height). Seed-generation setup is excluded and has its own generator costs.
