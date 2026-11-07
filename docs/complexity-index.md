# Native complexity and performance index

This is the migration's central index. Costs are documented alongside subsystem boundaries; a missing entry is a coverage gap, never an assumption of constant time. The frozen JavaScript implementation remains the behavior oracle.

## Subsystem reports

- [Foundation, snapshots, serialization and temporal state](foundation-complexity.md)
- [World generation, topology, geology, setups and quality retries](generation-complexity.md)
- [Population and startup](population-startup-coverage.md) includes placement and startup complexity.
- [Actors, combat, movement and perception](actors-combat-complexity.md)
- [Godbeast interaction boundaries](godbeast-interactions-coverage.md)
- [Yeti and passive monster integration](passive-monster-coverage.md)
- [Enemy metadata and legacy inference](enemy-traits-coverage.md)
- [Mini-Me snapshot construction](mini-me-coverage.md)
- [Equipment ability ownership](equipment-ability-coverage.md)
- [Divine reaction decisions](divine-reactions-coverage.md)
- [Maintained selected-creature links](maintained-links-coverage.md)
- [Ordered divine rescue helpers](divine-rescue-coverage.md)
- [Divine combat helper ordering and costs](divine-combat-coverage.md)
- [Abilities, inventory, crafting and progression](abilities-inventory-complexity.md)
- [Environment execution and coverage](environment-port-coverage.md) includes propagation and offscreen boundaries.
- [Desktop rendering, input and manual](desktop-complexity.md)
- [Native appearance validation, presets, history and fixed editor](appearance-complexity.md)

Definitions are local to each report: do not assume that `N` means the same thing across unrelated modules. Every hot operation should identify the collection it traverses, frequency, allocation/copy behavior, invalidation rules, and worst-case versus expected cost. Boundaries that parse or serialize an entire run must say so even when the ordinary in-memory operation uses shared blocks.

## Measurement rules

1. Verify equivalent initial and final states before comparing speeds. Use the same seed, commands, active actors and generated floors.
2. Report release/debug configuration, sample count, warmup, median and tail latency. Distinguish simulation, rendering, startup, save and undo.
3. Do not extrapolate an isolated snapshot microbenchmark to full-game performance.
4. Record source/fixture hashes and exact commands. Run exports successfully before consuming their output; fixture tests assert nonempty expected scenario counts.
5. Explain concurrent build/test load when timings were gathered on this shared machine. Repeat matched runs under quiet load before making a release performance claim.
6. Optimizations preserve iteration order, RNG draws, floating-point semantics, mutation visibility, observation rules and save/rewind behavior.

## Current evidence and limitations

`foundation-native-benchmark.json`, `foundation-reference-benchmark.json`, `generated-native-benchmark.json`, and `generated-reference-benchmark.json` are development measurements. Some were recorded before later execution coverage expanded; they are not a release certification. Shared snapshots substantially reduce capture work, while restore/export and compatibility normalization can still traverse the whole state. Unsupported gameplay remains explicitly guarded and cannot be omitted from a benchmark while describing it as complete-game coverage.

Additional benchmark protocol: `tools/benchmark-generated-commands-reference.mjs` and `examples/generated_commands_bench.rs` use the same six genuine Avatar seeds and472 commands, with two warmup traces and six measured repetitions per seed. Setup and rendering are excluded; normal temporal simulation dispatch is included. Run only after correctness checks and under a quiet shared host. These scripts do not themselves certify every unvisited gameplay combination.

- [Revision 37 terrain and sub-biome rules](sub-biomes-r37.md); costs are in the [complexity ledger](complexity-ledger.md).
