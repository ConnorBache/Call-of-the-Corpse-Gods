# Lighting dirty-cache contract

## Implemented subset

`LightingCache` reuses full tile illumination fields in `terrain_visibility`,
`native_terrain_visibility`, `terrain_visibility_chance`, and `observe_terrain`.
This is a derived, non-serialized, bounded two-plane cache. It is not a whole
observation cache or an actor-visibility cache. Sparse actor-specific queries
retain their scalar implementation (including active Shine self-exclusion through
revision45). From revision46, active Shine is omitted as a source everywhere;
see [Shine suppression](SHINE-SUPPRESSION-REVISION46.md).

Each immutable pass captures exact lighting dependencies and compares them to
its previous signature. This deliberately does **not** pretend failed dirty
checks are free: validation costs O(V + A*B + L), potentially twice for both
planes. It builds vectors and checks tile/door opacity, fire presence, static
sources, and actor emitting abilities. Here V is cell count, A actor count,
B ability count, L fixed sources. It avoids the original per-cell repeated
fire neighborhood/source/actor scans and ray work, rather than eliminating all
input scanning. Identical physical/dream dependencies share one field.

A dirty field builds source-oriented positive light, then clamps, subtracts
ordered darkness, then clamps again. Cost is O(V + sum(positive-source footprint
* ray length) + dark-source-count*V*ray length), space O(V + source count).
The original source ordering's floating-point subtraction is preserved.
Per-cell field access afterward is O(1). Viewpoint/portal LOS and telepathic
checks still run normally. Observation knowledge filtering remains unchanged.

## Invalidating dependencies

- Floor dimensions and exact ambient light bit patterns.
- Wall/tree/closed-door opacity.
- Fire presence (including original non-positive predicate semantics), not its
  magnitude, since any positive fire emits the same strength.
- Valid fixed source integer positions and exact power. Falloff determines
  admission only: the original kernel always uses a fixed falloff of ten.
- Shine emitter position, body/dead/ability/plane eligibility. Tile illumination
  includes its own active Shine through revision45; sparse actor queries preserve
  own exclusion there. Revision46 omits active sources from the dependency list,
  field and appearance. Activation/removal/revision transitions change the list
  whenever they affect illumination; moving a suppressed source does not dirty it.
- Darkness emitter precise floating positions, HP-positive/dead/ability/plane
  eligibility and original actor iteration order.

The signature is exact data, not a probabilistic hash and not a tick number.
Direct same-tick public field edits are therefore detected. Changes to unrelated
HP, ordinary non-emitter movement, seen-memory, tick or RNG do not rebuild.
Save serialization excludes the cache; clone creates an empty cache; equality
ignores it. Load, snapshot/rewind and proxies cannot inherit an unchecked field.
The full-pass projection holds an immutable Run lifetime, preventing ordinary
mutation of its inputs while using the projected fields.

## Instrumentation and evidence

- `vision.lighting_dependency_check`: capture/compare, inclusive of dirty build.
- `vision.lighting_field_build`: rebuild only; exclusive dependency cost is the
  first span minus this child. Build call counts distinguish cold/dirty cases
  from clean reuse without adding production counters.
- Existing `vision.lighting`: sparse reference/actor lighting that remains.

Tests in `vision::lighting_cache_equivalence_tests` compare every field cell's
f64 bits with the untouched original helper, cover direct edits, actor reorder,
planes, clone/load, serialization independence, exact allocation reuse, and
whole generated visibility masks. Full cross-build replay and timing are owned
by the integration harness; no improvement percentage is claimed here.
