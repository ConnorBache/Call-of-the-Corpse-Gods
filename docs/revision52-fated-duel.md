# Revision 52 Fated Duel termination

Designer instruction: “Killing an enemy, or them leaving the fated duel area should end the fated duel.” Here “enemy” means the selected Duel target. Unrelated deaths or movements do not end the Duel.

## Executed behavior

For simulation revision 52 and later, the selected target must exist as a living world actor on the anchored floor and inside the existing inclusive 5×5 square: at most two cells in each axis from `r50DuelX`/`r50DuelY`. The square does not recenter on either actor. A different floor is outside the square. No new plane, hostility, visibility, distance, resource, or duration rule is introduced.

Terminal target death, target removal, and the first committed target departure remove the owner's active Duel plus its four `r50Duel*` condition entries. Returning, resurrection after terminal death, and repeated contacts do not reactivate it. Death prevention that successfully restores the target before terminal death preserves the Duel. The existing owner-departure rule, three-energy upkeep, and activation prepayment remain unchanged.

The attack predicate independently verifies the target through the global creature index. It cannot grant protection from stale state before normalization. Each normal and hidden-floor upkeep path validates its own owner before charging. Event cleanup handles ordinary/haste movement, forced push/teleport, Float Away, direct player teleports, floor travel/fall/arrival, actual death, and explicit living-actor removals. Existing command/turn completion normalizes the remaining candidates before observation. The pre-existing offscreen and environment admission whitelists for independent NPC Duels are unchanged. The hidden-floor regression exercises actual-world target-death copyback using an admitted target-only hidden floor; it does not claim support for independently maintained NPC Duels on an otherwise unsupported hidden floor. Hidden-world copyback normalizes targets against the actual world, without treating a travel-admission probe's temporary player coordinates as a committed move.

Half Here and the Dreamwalking body continue to use their owner's Duel. They do not gain independently anchored Duels or upkeep. Independent copied owners retain their own existing anchor, chosen target, active entry and reserve. An owner leaving affects that owner only; its target leaving affects every owner selecting that target.

Revision 50/51 retain their previous owner-only expiry and immunity predicates. The added candidate index is derived only and does not change serialized state, actor order, RNG, wire identities or historical revisions.

## Persistence and temporal behavior

No new save fields or condition names are added. Strict load, compatible timeline migration, Snapshot restoration, and both text/native Future restoration normalize invalid R52 Duels from each restored world's own target state. A valid old Snapshot/Future restores its live Duel even if the discarded future had ended it. Invalid stale links in an older R52 save lose protection and their next upkeep charge; paid prior upkeep is not refunded. Replay remains deterministic within the current R52 rules. Old R52 recordings that depended on the previous stale-target bug can intentionally diverge, since this is a correction within R52 rather than a new simulation revision.

## Candidate-index invariant and audit

The existing `ActorFloors` identity registry holds a shared, nonserialized candidate-ID slice. At structural reconciliation an independent actor is included if it has the Duel ability, an active Duel, or a Duel-target condition. It is a conservative superset: expiry/removal of an ability may leave a harmless candidate until the next rebuild; every use checks current state.

Production `active` assignment sites were audited. All toggle activations target the player, which is always checked separately and never relies on this slice. The only ongoing NPC active-array projection is Half Here; it is a shared endpoint, not an independent owner. Dreamwalking bodies clear their active arrays and use their owner. NPC upkeep, equipment, effect expiry and death paths only remove existing activation. Mini-Me R50+ deliberately copies stats only, not powers. Independent copied owners present in loaded/copied worlds enter through actor insertion, full replacement, deserialization, clone or restore. Structural/public mutable access already invalidates the identity registry. These paths therefore rebuild candidate membership before use.

Trusted in-place actor/slice writers must not introduce an independent NPC Duel; such a new activation must use public invalidating mutation or explicitly invalidate the registry. This additional invariant is documented at the trusted accessors. Tests cover a warm index followed by public active/ability/target mutation, copied-owner birth, clone/load, expiration and target removal. Shared endpoint projection is covered separately. Any future NPC activation feature must honor this contract.

## Complexity ledger entry

Variables: A = world actors, P = nested pilots, B = previously indexed actors, F = retained floor maps, V = copied vacant slot entries, d = decimal digits of creature IDs, H = historical slot high-water sum, K = active/owned ability entries inspected, C = condition entries shifted when expiring, D = independent Duel candidates (D ≤ A), W = movement/removal events in a phase, E = expiring owners. Candidate list order follows canonical actor order; it is not used to choose an outcome or consume randomness.

- Clean attack guard: one indexed target lookup, O(d + K) time and O(1) extra space. No owner scan, pathfinding, visibility traversal, sorting, or RNG is added.
- One owner's upkeep validation: O(d + K + C) if expiring, O(d + K) otherwise, O(1) local scratch. It does not scan other owners.
- Committed movement/removal and boundary normalization: O((D + 1) × (d + K) + E × C) time, O(1) scratch plus a shared Arc handle for the immutable candidate slice. No new all-actor traversal per ordinary step. For W events, the clean added bound is O(W × (D + 1) × (d + K) + total expiry work). In the adversarial D=A case, this can still be quadratic across a phase. Ordinary worlds with D=0 do only the separate player check and index access.
- Candidate construction piggybacks on existing identity reconciliation. It adds O(sum K) work over actors and O(D) retained storage. Full reconciliation remains O((A + P + B) × d + F + V + sum K), with O((A + P) × d + F + D) replacement storage and existing O(H × d) slot history. Structural mutation can force this cost at the next lookup; it must not be reported as a constant-time cache hit.
- State mutation detaches only actor blocks containing an expired owner. First write to a shared block additionally copies that block's complete actor payload, including nested data. Player expiry also uses existing Half Here/Dreamwalking projection: O(current-floor actors + owner payload) and possible endpoint-block COW. These costs are in addition to the indexed predicate, not hidden in O(1).
- Legacy R50/51 retain their existing full persisted-owner settlement scan. New derived metadata is shared across cheap actor-map clones and is excluded from serialization and state equality.

These are source-derived bounds, not measured performance claims. Verify representative large-floor runs with and without independent Duel owners before making timing claims. The deterministic tests verify state outcomes, not frame rate or platform rendering.

## Regression coverage

The dedicated `revision50::revision52_duel_tests` module exercises the inclusive 5×5 anchor, death/removal/off-floor stale guards, no later upkeep, immediate exit/reentry, unrelated deaths, direct/periodic/environment/explicit death, rescue, independent reserves, shared endpoints, candidate-cache invalidation, forced pushes/teleports, Snapshot/Future restoration, actual falls/floor travel, Dreamwalking owner routing, nightmare removal, hidden target-death copyback and independent-owner upkeep, strict/compatible loading, haste/replay determinism, and revision compatibility. See the final task verification report for the exact executed test commands and their results; compilation alone is not a gameplay pass.
