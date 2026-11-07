# Reproducible in-play timing diagnostics

This is opt-in measurement, not a performance optimization. Simulation revision,
RNG, save rules, and gameplay decisions are unchanged. A normal build has no
`runtime-timing` feature, and all span guards compile away. A diagnostic build
retains a thread-local collector (disabled initially); even a disabled collector
has guard/TLS cost, so compare it against a separate feature-OFF build.

## Reproduce

From the complete project root with pinned Rust 1.99.1, run
`sh tools/run-timing.sh` (Linux) or `powershell -File tools/run-timing.ps1` (Windows).
These scripts build and run release diagnostics, preserve JSON samples and exact
post-command states, and verify cross-feature byte-identical serialized state.
Normal platform build prerequisites still apply. The scripts reproduce the workload
with Cargo's default release profile. The published cloud measurements instead used
these compact-release environment overrides (set them before running a script for
the matching profile): `CARGO_PROFILE_RELEASE_OPT_LEVEL=s`,
`CARGO_PROFILE_RELEASE_LTO=fat`, `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`,
`CARGO_PROFILE_RELEASE_DEBUG=0`, `CARGO_PROFILE_RELEASE_STRIP=symbols`.
Compiler/platform differences still affect timings. The code-browsing-only ZIP is
not sufficient to build the desktop (it omits art and test fixtures).

Direct commands:

```
cargo run --release -p corpse-core --example timing_test -- --samples 30 --warmup 3 --output core-normal.json --state-dir states-normal
cargo run --release -p corpse-core --features runtime-timing --example timing_test -- --samples 30 --warmup 3 --output core-profiled.json --state-dir states-profiled --export-input generated-input.json
cargo run --release -p corpse-desktop --features runtime-timing -- --timing-test desktop.json --timing-input generated-input.json --timing-samples 30 --timing-warmup 3
```

The core example accepts `--input PATH` for a Run JSON scenario. It does not alter
the input file. The desktop accepts native saves or plain Run JSON via
`--timing-input PATH`, measures read-only projections, and uses a newly created
benchmark-only temporary directory for actual save I/O. It never uses a player
profile directory. `--timing-reverse` reverses collector-off/on block order in the
desktop benchmark; repeat in both orders to expose time-order noise.

## Scenarios and scope

The core runner generates one real r39 Blessed starting world, then tests Wait
and Pickup from independent identical starts. Derived fixtures are explicitly
synthetic: all NPCs removed (terrain/hazards retained), 32 added awakened hunting
rats, and the first three floors marked seen/dirty for inactive-floor processing.
Generation, cloning, state verification, and returned outcome disposal are outside
its stopwatch. Each measured sample is one command, not a whole playthrough.
Actual tick advancement is recorded; Pickup need not consume a world tick.

The desktop runner separately measures observation, inspection, routes, possible
exits (including expensive negative checks), the ordinary hover+dirty sequence,
ability projection where available, save encoding, forced verified atomic save,
unchanged-save guards, and software framebuffer rendering at two resolutions.
These operations overlap; never sum all their medians as one frame budget.
It does not measure SDL polling, texture upload, present/vsync, GPU time, full
interactive input-to-display latency, or native Windows runtime on a Linux host.

## Interpretation

Each label reports call count, inclusive nanoseconds and exclusive nanoseconds.
Inclusive time includes child scopes and must NOT be summed across nested labels.
Exclusive time subtracts direct children and can be added within a measured root.
Labels aggregate static call sites, not stack traces; repeated/recursive calls to
the same label accumulate. Single-thread game execution is measured; each thread
has its own collector, and unjoined worker timings are not silently aggregated.

Clocks, stack operations, and counter aggregation have observer overhead. The
collector resets between core samples, so first insertion/allocation per label is
part of instrumented execution. Child accounting overhead is attributed to its
parent's exclusive time. Compare enabled vs disabled collector AND separate normal
build. Do not subtract a fixed guessed overhead or present detailed instrumented
shares as exact uninstrumented percentages. Raw samples, warmup count, seed,
scenario provenance and exact-state equality are retained for scrutiny.

Use release builds, stop other known CPU/disk-heavy work, record hardware/build
flags and repeat. Median/p95 are descriptive sample statistics, not guarantees.
Sparse sample tails, cloud scheduling, thermal state and warm caches limit claims.
One generated entry and synthetic stress cases do not establish whole-game
bottlenecks, late-game history/save growth or every conditional effect.

## Complexity of the diagnostics

Let L be distinct recorded labels, D nested scope depth, N measured samples and S
serialized state size. Disabled compile-time spans have O(1) zero-sized bodies
which optimize away. Enabled scope entry is amortized O(1); close is O(log L)
BTreeMap accumulation plus O(1) parent accounting. Collector storage O(D+L).
Reset drops O(L) entries; taking its map is O(1); JSON export is O(L+label bytes).
Each sample adds its actual measured pipeline plus O(S) equivalence verification
outside the timed region. Percentiles sort N values in O(N log N), O(N) storage.
Full raw reports use O(NL) space. Save/render helper costs remain those of their
real implementation; instrumentation does not change their asymptotic workload.

## Safety and coverage

Timers never enter Run, RNG, save data, turn rules, or rendering animation clocks.
Tests check nested inclusive/exclusive identities and disabled collection. The
runner compares complete saved outputs with collector on/off and preserves exact
files for cross-build comparison. Feature-OFF checks and existing regressions
remain required. New diagnostic functions include their own complexity comments;
this does not claim the separate project-wide annotation audit is complete.
