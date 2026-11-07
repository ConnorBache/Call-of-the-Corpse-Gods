# Possible-exit hover sequencing review

Baseline: released source v15 in `scheduled-r39/source`.
Target integration: unpublished actor-index candidate in `creature-index-r39/source`.

## Change

Only production function changed: `non_euclidean_ui::possible_exit_marks`.

1. Keep the original dream projection, current-floor lookup and bounds check.
2. Move revision applicability and exit enumeration before full terrain observation.
3. Return immediately when disabled or no candidate exit exists.
4. Otherwise run the same terrain observation and apply the same `current || remembered` checks to the hovered tile and all candidates.
5. Map the surviving indices to coordinates in the same order.

The original `observed_exits` helper remains unchanged for all other callers.
Candidate enumeration uses the exact original `pockets::exits`; it does not replace pocket precedence, change portal behavior, sort exits or add observations. The relevant path now enumerates candidates once, not an applicability scan followed by another enumeration. Dream preparation deliberately remains before the fast-path query: checking the physical floor first could omit valid marks or do unnecessary observations when the projected floor differs.

## Equivalence argument

Let E be `pockets::exits(f, at)`, K(i) be observation's `current || remembered` predicate. Original result is empty if revision/floor/bounds fail or if K(at) is false, otherwise coordinates of `[e in E where K(e)]`. New result is the same expression, but computes revision/E before observation. E is deterministic/read-only and observation does not mutate the run; when E is empty, the result is empty for every possible K. Thus dropping observation in that case cannot change returned marks. Full observation remains mandatory for nonempty E, including unseen origins and exits. Revision and bounds failures also retain the same empty result.

No gameplay mutation, RNG, visibility algorithm, save format, travel behavior, actor alias or item alias code changes. The diagnostic timing counts intentionally change because unnecessary observation no longer runs.

## Verification

New tests: `possible_exit_sequencing_matches_v15_across_views_and_topology`, `possible_exit_sequencing_preserves_hidden_and_remembered_endpoints`, and (with `runtime-timing`) `possible_exit_sequencing_skips_observation_when_inapplicable`.

Integrated candidate verification completed with exit status 0: all 15 matching tests passed (9 in `non_euclidean_ui` plus 6 related targeting, travel and hazard tests), including the 3 new tests and runtime-timing call-count proof. Evidence: `creature-index-r39/hover-tests.log` and `hover-tests.exit`. The frozen-v15 oracle matched all 45 hover samples across 9 fixture variants, and run/RNG equality held. The timing regression confirmed 0 observation calls for irrelevant/out-of-bounds/absent links, 1 for relevant exits, and 1 on the old absent-link path. Existing actor/item alias, waking/dream plane, no-projected-sight/legacy, and renderer tests also passed. Full candidate workspace testing and Windows compilation are owned by the integrating worker; no independent Windows runtime claim.
