# Native simulation comparison

## What was measured

Normal simulation dispatch with temporal history on six generated floor1 Avatar runs, ten generated floors per run. Rendering, autosave, filesystem and startup excluded. No Windows-runtime or end-to-end FPS claim.

Both implementations replayed the same472 commands from identical source fixtures. Each seed ran two warmup traces and six measured repetitions. Rust checks final source-state parity before timing. Reference JavaScript ran in v24.19.0, not a browser renderer. No concurrent builds/tests ran during timing.

| Seed | JS median ms | Rust median ms | Median ratio | JS p95 ms | Rust p95 ms |
|---|---:|---:|---:|---:|---:|
| 00-09-10:independent-native-walk-0 | 79.897 | 6.303 | 12.68× | 94.544 | 8.632 |
| 00-09-10:independent-native-walk-1 | 83.526 | 5.118 | 16.32× | 96.003 | 6.264 |
| 00-09-10:independent-native-walk-2 | 76.801 | 4.242 | 18.11× | 89.801 | 5.063 |
| 00-09-10:independent-native-walk-3 | 81.574 | 4.792 | 17.02× | 102.616 | 6.563 |
| 00-09-10:independent-native-walk-4 | 78.568 | 4.757 | 16.52× | 90.278 | 5.380 |
| 00-09-10:independent-native-walk-5 | 86.779 | 6.233 | 13.92× | 101.531 | 7.912 |

Ratios are per-seed medians, not a claim that every mechanic, frame or platform runs this many times faster. These are initial-floor traces, not a deep-floor worst-case benchmark.

## Architecture

Typed Rust state and copy-on-write floor/actor blocks avoid repeatedly copying unchanged dungeon data into every temporal snapshot. Simulation randomness and iteration order match the frozen reference. Save/export still traverse full state, and native automatic persistence incurs filesystem I/O. These separate costs are not hidden inside the simulation result.

## Reproduce

Run tools/cargo.sh run --release -p corpse-core --example generated_commands_bench and tools/benchmark-generated-commands-reference.mjs in sequence on an otherwise quiet machine. The fixtures and raw JSON timing reports are included in the source archive.

Fixture SHA256: 4edc10b3e7593be9243b4d8a4548c4d6072eb13de8056b64bdc6a34e4a61a7f3


## Animated bestiary warm-paint sample (October 20, 2026)

A release-mode Linux software-painter test renders the complete bestiary page, including animated atmosphere, geometry, cached artwork, text layout and glyph compositing. Each case uses two warmup frames followed by 24 timed frames, 83 ms apart in cosmetic input time. These are mean CPU paint times on this shared cloud host, not end-to-end frame rates. SDL upload/compositor, cold decode/matte preparation, window events and Windows execution are excluded. This final depth-environment sample ran alongside core tests and is therefore an approximate shared-load sample, not an isolated benchmark.

| Page | Depth | 1280×800 mean ms | 1920×1080 mean ms |
|---|---:|---:|---:|
| Blorb | 1 | 9.059 | 12.341 |
| Wendigo | 15 | 12.691 | 16.793 |
| Bastidion | 40 | 10.986 | 18.265 |
| Transcendent Zillennia | 40 | 11.261 | 16.964 |

Reproduce with `tools/cargo.sh test --release -p corpse-desktop benchmark_animated_manual_warm_frames -- --ignored --nocapture`. The normal correctness suite intentionally ignores this timing-only test. Logical geometry is hard-bounded and the manual redraw cadence is approximately 12 Hz; elapsed time never drives simulation. See native-manual-layout.md for asymptotic costs and separate Linux/Windows verification scope.
