# Generation-focused revision-36 evidence

`generation-integration.log`: eleven focused integration tests, all passed, including an independently constructed walking graph and a counterexample that is safe only through portals (rejected). Both full and portal-free safety proofs are active in the final sweep.

`generation-proposals.log`: three focused literal-algorithm unit tests, all passed. The final workspace gate is recorded separately by the release integrator.

`geometry-sweep.json`: 144 geometry-only cases (six domains, depths20/30/40, eight seeds); actor and ground inputs intentionally empty.

`populated-sweep.json`: four ordinary seeded runs fully generated and populated through floor40; all160 floors inventoried, with detailed retry counts for selected depths20/30/40 (twelve cases), using real actor/ground exclusions and identical metadata on regeneration.

Counts are deterministic. Elapsed times are concurrent optimized-test wall time, not platform or renderer benchmarks. Samples do not prove exhaustive seed coverage. A proposal probability is not an accepted-outcome ratio. No distribution was adjusted to obtain these results.

`summary.json` records totals, commands/profile and source hashes. No runtime screenshot, Windows execution or overall-workspace pass is claimed by these generation-only files.
