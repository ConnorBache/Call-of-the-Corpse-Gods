# Revision 38 startup and population failure audit

The native built-in startup now retries a complete floor transaction, not merely
its geometry. Each new floor permits eight complete trials, each using a
non-overlapping window of 32 geometry candidates. The original run seed, domain
plan, Godbeast schedule, Nest schedule, staircase coordinates and ordinary heart
budget remain fixed. Only geometry candidate offsets vary (0, 32, ..., 224).
Every rejected trial is discarded, including actor/item IDs, ordinary loot,
unique ability exclusions, metrics, recorded generation stages and DEBUG
reference-heart/loot collection. A successful floor records `populationAttempt`
in its cleanup-stage details. Native new-run creation and later expansion both
use the same boundary.

The historical revision29–37 path retains one complete population attempt and
its previous generated outputs/failures. The public `ensure_floors_with` custom
generator contract also remains single-attempt: an external fixture/configuration
error is not silently retried. `spawn_floor` remains atomic on an explicitly
supplied floor rather than replacing the caller's map.

## Errors classified as geometry/placement-dependent

- Structural geometry quality exhaustion (including its precise last cause).
- Final complete-floor safe-transition failure after all late decorators.
- Generated loot or required arrow capacity; mandatory item invalid/occupied.
- Connected horde packing or inventor/mech safe-position exhaustion.
- Required machine coordinate, occupation or initial-fire conflicts.
- Machine heart budgets leaving no exact ordinary horde plan.
- Dagger-adjacent placement or reachable dream population capacity.
- Nest resident/Honey occupation or Administrator safe-position exhaustion.
- Floor-one entrance fixture/lighting/protected-route conflicts.

A geometry retry does not weaken any of these validators. No required item,
actor, event, heart reward or authored composition is deleted to make it pass.
Successful geometry may admit different ordinary machine arrangements, as in the
existing geometry quality search, but the per-depth exact heart budget remains
the same. Required Godbeast/Nest identities stay tied to the original seed.

Other failures remain immediate: unsupported revision/depth/grid/domain,
invalid explicit horde composition or mixed habitats, unknown creature/heart/
equipment definitions, duplicate population, malformed metadata, oversized
machine groups, out-of-order stages and impossible internal follower-selection
invariants. Static catalog parsing/template assertions are covered by catalog
and roster tests; randomly choosing a different map cannot repair them.
Exhaustion is finite and explicit, retaining its last reason. This is not a
mathematical guarantee for every possible seed or malformed external fixture.

## Direct cause fixes and catalog/budget audit

The geometry companion fixes the tap/stairs exclusion bug directly. It also
uses `ordinary_horde_budget_feasible_revision` when admitting revision38 machine
stamps. Earlier code checked the frozen revision29 ordinary roster even when
startup would populate from the revised roster. Historical setup admission
keeps that old function. Revision38 uses the same candidate pool and exact
feasibility solver as its actual horde planner, so machine packing cannot be
accepted merely because an obsolete roster could complete the reward budget.

Ordinary loot always uses a finite nonempty equipment kind catalog and bounded
2–20 slots. Its ability-grant pool safely falls back to ordinary equipment when
all abilities are excluded. Stair/tap/dais/dagger/Nest/Sphinx/machine reservations
are propagated before ordinary loot is selected. Sphinx, dream and Nest
placement retain strict fixed-coordinate and occupancy validation.

## Complexity

Let F be newly requested floors (<=40), V grid cells (<=3600), A/I existing actors
and items, G a single geometry-candidate cost, and P a complete population cost
(documented in population generation). The native boundary costs at most
O(F * 8 * (32G + P + R)), where R is cloning the scalar run/metadata/inventory
state. Previously generated floor/actor/item payloads remain Arc-shared until
mutation. Peak additional state is one working run and one candidate, plus the
existing geometry/population temporary state; it does not retain all rejected
maps. Each test of an error against the fixed retry list is O(error length).
The budget helper invokes the existing memoized finite horde feasibility solver;
it does not introduce an additional random stream or an unbounded search.

## Verification

`startup::recovery_tests_r38` injects a failing required tap after loot/IDs have
already been generated, then verifies clean retry output including IDs, metrics,
loot and grant exclusions. It verifies exact eight-attempt exhaustion rollback,
one-attempt fatal/configuration errors and unchanged custom-generator behavior.
`tests/startup_recovery_r38.rs` checks known bad seeds through all40 floors,
required Godbeasts/Nest/tap/dream items/Nightmares/Administrator, exact ordinary
heart budgets, historical failure preservation, save roundtrip and equivalence
of batched versus incremental floor generation.

## Final post-decoration gate

Revision38 rechecks `transitions_accessible` against `reachable_without_braziers`
after all population, habitat, Hellfire and topology decoration. This is the same
cardinal, fire/grate-avoiding safety invariant used during geometry generation;
a physically connected route through permanent fire does not count. Failure
rejects the complete trial and retries transactionally. Historical revisions
retain their previous behavior. The gate costs O(V + fixtures) time/O(V) scratch.
A controlled one-cell corridor regression proves permanent fire prevents a
false-positive safe connection, and proves that rejection is retryable.
