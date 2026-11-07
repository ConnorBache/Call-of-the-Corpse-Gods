# Native generation coverage, frozen v48

The Rust generator executes the complete frozen-v48 world pipeline. `generation::generate_floor(depth, seed, domain, godbeast, size, actual_up)` returns an owned typed `Floor`; `generate_floor_full(depth, seed, actual_up)` selects the seeded domain and Godbeast schedule for normal 79×29 startup. Immutable catalogs are compiled in. Gameplay uses neither JavaScript nor subprocesses. Development exporters read frozen `dist/src` solely to produce parity oracles; they do not modify it.

## Executed and differential-tested

- Room masks: all four current shapes and three size classes, exact ordered cells, cellular/diagonal passes and RNG; 84 cases.
- Passages: four styles, all cardinal directions, offsets, bends and constructed endpoint flags; 576 ordered footprints/paths.
- Stairs: independently seeded vertical plan, depths1/4/5/6/20/40, sizes79/50/60, actual/clamped upstairs; 378 cases. Domain bags and independent nonrepeating Godbeast bags: seven full plans.
- Value fields: 42 arrays, exact Float32 lattice/output bits and draw state. Ordinary numeric/text blessing budgets400; base encounter quotas120; retained loot480 nominal plus12 historical-record cases.
- Accretion:42 direct constrained/unconstrained layouts with stage snapshots and RNG; loops126 straight/bent/acceptance-callback cases. Room tree, sectors, exits, jambs and bounded growth all retain frozen order.
- Geology:108 plans and108 constrained layout→loops→materialization cases across all6 domains, all3 sizes, six seeds including unsigned boundaries. Full fields, landmarks, core/spine reservations, transitions, terrain and RNG.
- Stairs/recess protection, connectivity, diagonal cleanup, geological bridges, shores, currents and substrate refresh:48 complete floors and diagnostic maps.
- Basic vents, habitation lighting, fuel/gas graphs, furnace safety, emitter exclusions and vent reconciliation:48 full intermediate/final floors with RNG.
- Topology:48 full measurement/monotone-repair/acceptance cases, including corresponding-component landmark survival baselines and quality gates.
- Diagnostics: six whole stage sequences plus156 malformed metadata mutations.
- Authored setups:180 full stamped floors across depths1/10/20/40,48 packing/door floor cases including exact random state;144 Godbeast room/event geometry cases. Eligibility and introduced-depth masks are finite immutable40-depth exports, not seed-dependent generated geometry.
- Complete generation:26 full-field frozen reference floors. Seeds0/1/42/12345 at depths1/5/10/20/40; all6 explicitly requested domains at depth17, Godbeast rooms, actual-up override, and legacy50/60 geometry. Candidate rejection/retry is exercised (seed0 depth5 accepts attempt1). All quality gates remain fixed; the bounded32-candidate search never falls back to relaxed quality.

The supplemental `generation_sweep_parity` compares a deterministic typed Float64-LE structural hash of every Floor field on156 cases: every depth1–40 for seeds0/1/42, plus unsigned boundary seeds across all6 domains at depths1/20/40. Reference accepted-attempt counts are143 at0,10 at1,2 at2 and1 at3. The hash is a regression checksum, not a cryptographic equivalence proof; the26 complete-field comparisons remain the direct oracle.

Population, actor quality selection, horde feasibility and startup are implemented by the population subsystem, which consumes the native geometry API. Those tests and coverage belong to its owner; schema parsing alone is never counted as executed coverage here.

## Explicit boundaries

Unsupported floor dimensions, domains and depth ranges are rejected. Invalid passage directions and coordinates are rejected instead of fabricating JavaScript NaN footprints. Legacy saved graph shape labels `circle`/`eye`/`open` parse and validate, but invoking topology loop addition on those labels remains explicitly unsupported; all currently generated shapes are supported. Legacy standalone terrain preview algorithms are not runtime floor generation and are not ported. A32-candidate quality exhaustion returns an error rather than a substitute floor.

Differential fixtures cover fixed inputs, not every seed or floating-point library. Current evidence is Linux x86-64; Windows-specific math and execution remain unverified. No speed comparison or exhaustive equivalence claim is made.

## Reproduction and hashes

Run from native workspace root. Each `tools/export-*-fixtures.mjs` accepts the frozen reference directory as its optional argument. `tools/export-setup-catalog.mjs` reproduces the immutable setup/bestiary catalogs. `tools/cargo.sh test -p corpse-core --test NAME` runs a suite.

| Suite | Fixture SHA-256 |
|---|---|
| generation_parity | c880cbed23e10f79b5403c4ab880199796c22b8cdecc3ae291e0a807f2069e3c |
| geology_parity | 86e03c9f9d6b7860f072a629dd61bb98b3ca28543c9fd3355601b3163359cb64 |
| cleanup_parity | b6fb1ca7fad6fd1f9d929e3ea6333e02635f2d3cc77065b332fc8249e35371df |
| generation_environment_parity | ae078f922faf0f54fedce6bd82e68e1ed6188f4db89bd2022882acff28b80dd3 |
| topology_parity | b000f8004baaf8f3daeadfffdd5034e9b6712d49cbe78115322f925e6c98a642 |
| generation_validation_parity | 816144c699dd645f8c06e1df6ae883b3160297c763fbc3e66ecd731639ff17e7 |
| setups_parity | 5ffb52fc9af50a891e7054dc245dfb6f7d884421aaa98e25cacf5beb46063f62 |
| world_generation_parity | f83bcddbb25818d3b1e9dada3be6a5ece4c604621be6105999aa54ca51b07aa3 |
| generation_sweep_parity | 00792cac58554cd974acb86e3923c0a464da14b652b512af57174208e914dee4 |

Complexity and known overhead are in [generation-complexity.md](generation-complexity.md).
