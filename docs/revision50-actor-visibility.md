# Revision 50 actor-local targeting visibility

`vision::visible_from(run, observer, x, y)` is the explicit observer boundary used
by independent Nightmare activation. Calls for the player delegate to the existing
`visible` function, preserving historical shared viewpoints, reveal and sensing.
Independent non-player actors use their own position, plane, All-Seeing conditions and
telepathic blessings/statuses. They cannot borrow the player's reveal mask or
Half Here viewpoints. Player awareness of perfect allies remains a player-only
control affordance, rather than making every copy telepathically see itself.

The internal `telepathic_for_owner` helper parameterizes the existing sensing
predicates without changing player calls. Independent queries filter creatures
to the observer's plane and preserve the existing radius-two sensed-area rule.
The final optical query uses the existing tile visibility threshold and plane-aware
lighting/portal route implementation. No RNG is consumed. A successful visibility
query is not an activation or a grant of control over that actor.

## Complexity

Let A be current-floor actors, B observer blessing/status entries, V floor cells,
K portal endpoints and R maximum ray length. Independent targeting scans at most
A + 1 creatures. Each candidate can inspect B powers and, for applicable senses,
perform an existing optical route query, whose portal cost may be O(K²R). The
coarse worst-case sensing bound is therefore O(A(B + K²R)), plus the final optical
and lighting query. Actual candidate filters often avoid those calls. Lighting
cache misses include field/projection construction and cannot be counted as O(1);
the existing vision/lighting implementation defines their bounds. Additional
local memory is O(1) apart from helper route/cache allocations. No persistent
per-copy visibility grid is introduced. These bounds are unmeasured estimates.

## Verification and remaining boundaries

The actor-local visibility source previously compiled. A deterministic Nightmare
regression checks that player All-Seeing does not reveal a blocked square to an
independent actor, rejected activation leaves serialized state unchanged, and an
explicitly visible target consumes only that owner's charge and records its
identity/plane. Tests now construct synthetic owners, not Mini-Me: the superseding
Mini-Me rule grants stats only and no activated powers. The rewritten fixtures
await native verification. Full non-Euclidean, dream-plane and UI targeting checks
remain required for other consumers of this generic API. See
[Mini-Me coverage](mini-me-coverage.md) for the stats-only rule.
