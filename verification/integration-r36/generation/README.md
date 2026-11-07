# Generation-focused revision-36 evidence

Current assignment adjacency is the live eight-direction grid with its existing wall-corner rule. CA contiguity and both full/portal-free safety proofs remain conservatively cardinal.

`generation-integration.log`: eleven focused integration tests, all passed, including an independent walking graph and a counterexample safe only through portals (rejected).

`generation-proposals.log`: four literal-algorithm unit tests, all passed, including diagonal-only destination eligibility, forced diagonal next source, either-wall corner rejection and the existing tree-corner rule. The final workspace gate is recorded separately by the release integrator.

`geometry-sweep.json`: 144 geometry-only cases (six domains, depths20/30/40, eight seeds), actor and ground inputs intentionally empty.

`populated-sweep.json`: four ordinary seeded runs fully generated and populated through floor40; all160 floors inventoried, with detailed retry counts for selected depths20/30/40 (twelve cases), using real actor/ground exclusions and identical metadata on regeneration.

Counts are deterministic. Elapsed times are concurrent optimized-test wall time, not platform or renderer benchmarks. Samples do not prove exhaustive seed coverage. Proposal probability is not an accepted-outcome ratio. No distribution was adjusted to obtain these results.

`summary.json` records totals, commands/profile and source hashes. `superseded-cardinal-assignment/` preserves the earlier four-neighbor inference's evidence for audit only, not current results. No runtime screenshot, Windows execution or overall-workspace pass is claimed by these generation-only files.
