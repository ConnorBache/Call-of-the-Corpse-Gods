# Exact actor-source exclusion field

Through revision45, the original scalar light query excludes a creature's own Shine only while that creature has Shine active. It does not remove all light at that position. Positive contributions use maximum, clamp once, then subtract darkness sources in actor order, then clamp again. Target-specific Shadow in the Dark and darkness-beast overrides are separate caller rules; cross-plane actor sight deliberately uses the raw plane light instead.

From revision46, active Shine suppresses its whole source before either scalar
or cached collection. No source-exclusion arrays are required for new-revision
Shine; other lighting remains. The algorithm below remains the exact legacy
contract. See [revision46 behavior and tests](SHINE-SUPPRESSION-REVISION46.md).

## Representation and correctness

`PositiveSource.exclude` labels active Shine with its source identity. Ambient, fire, placed lights and inactive Shine are unexcludable. Each cell stores its winning positive value and the best value belonging to a different exclusion identity. Excluding one identity therefore chooses either the winner or its runner-up. Repeated contributions with the same identity cannot become each other's fallback. Equal-valued distinct sources correctly populate the fallback. Original source order and strict greater-than comparisons preserve floating-point selection. Every darkness source subtracts from both candidates in original order; a single summed darkness value would change floating-point rounding and is not used.

Signed-zero IDs compare equal; NaN never matches an excluded ID. This mirrors the scalar f64 comparison without assuming all direct test mutations use admitted IDs. Source ID and activation enter dependency capture, so direct same-tick mutations cannot reuse the wrong exclusion field. Clone resets and cache serialization behavior are unchanged.

When no active Shine is present, exclusion arrays are empty: ordinary tile fields retain one value array, no doubled darkness arithmetic. With active Shine, extra storage is approximately 24 bytes per cell (f64 runner-up plus Option<u64> owner on conventional 64-bit Rust targets), at most two plane fields; exact layout is platform-dependent.

## Complexity

Let V be cells, A actors, B ability/active-list entries per actor, L fixed lights, S positive emitters, D darkness emitters, R maximum ray length, F_s positive-source footprint size.

- `exclusion_key`: O(1) time and memory.
- `Dependencies::capture`: O(V + A*B + L) time and stored dependencies; now includes active Shine identity/status. Must be paid once per immutable batch, not per target.
- `Dependencies::build`: O(V + S + sum(F_s*R) + D*V*R) worst-case time, O(V) output; some extra positive rays establish second-best values. No asymptotic increase. Initial build needs measurement; this is not claimed faster for every workload.
- `LightField::at_excluding`: O(1) time and scratch, no scans or geometry.
- `LightingProjection::in_plane_excluding`, `current_excluding`: O(1) queries after projection construction.
- `begin_resource_turn`: O(C + A*(Q+M)) time, C projection setup including dirty builds, Q projected visibility query, M meter/stat derivation; preserves iteration order.
- `observe_resource_history`: O(C + A*(Q+M) + H*A) time for H prior history entries; history matching intentionally unchanged.

Borrowed projection lifetime prevents ordinary mutation during batch reads. Creating a projection separately for every scalar visibility call would repeat O(V+A*B+L) validation and is deliberately avoided.

## Verification added

1. Deterministic 48-map source-order/opaque-ray differential against target-major reference, 121 cells each and five excluded identities, exact f64 bits.
2. Same-tick active status/ID/plane mutation and signed-zero exclusion tests.
3. Full world field comparison against original `illumination_in_plane` for every actor, cell, both planes, across source activation/death, geometry, darkness HP/order, dream and clone edits.

Verified on Rust 1.99.1: `cargo test -p corpse-core --lib exclusion_tests` passed all 3 tests (0.18 s test execution). `cargo test -p corpse-core --test visibility_projection_cache` passed (0.76 s test execution), comparing projected direct/ordinary actor visibility to scalar queries over 16 synthetic maps, 0–7 portal pairs, 33 actors, and nine mutation/clone phases. This is focused coverage, not the full workspace suite or a performance measurement. `cargo test -p corpse-core --lib lighting_cache` also passed all 9 tests (11.70 s test execution), including the six pre-existing cache regression tests.
