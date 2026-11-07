# Revision 37 generation contract and bounds

The terrain pass runs after the complete initial ordinary, machine, Nest and Godbeast population is placed. It consumes independent seed/floor generation streams, never the live simulation RNG. It does not reroll ordinary monsters to fit a theme. Historical revisions 29–36 use their original terrain and population order.

## Habitat footprints

Each actual Wendigo or Yeti anchors a radius-five mountainous footprint. Snow covers suitable local walking surfaces. Radius-four ice uses three Moore-neighbor cellular-automaton rounds, with a live monster anchor and only its cardinally connected component retained. The initial live probability is 0.62; live cells survive with at least three live neighbors and dead cells are born with at least four. The anchor is held alive in each round. Very narrow geometry can produce a one-cell component; the generator does not erase a creature or carve through fixtures to enlarge it.

Mountains propose up to six chasms per anchor, with a target of one eighth of suitable local cells, at most eighteen local candidates. Across all mountain and forest decoration, at most 64 blocking changes are attempted. Every proposed chasm or tree receives an ordinary walking before/after reachability proof. An occupied tile, protected item site, fixture, stair margin or authored special chamber is not replaced.

Forests use radius-three neighborhoods of the established Dryad, bear, tiger and short/long Saidribee associations. Existing grass and trees are retained. New fuel is not added to a gas component containing an existing vent or fire source. Existing grass can receive route-safe trees. A footprint is recorded only where actual grass or trees exist, never merely a color label on bare floor. Consequently some monsters in unsuitable, protected or fuel-constrained surroundings do not receive a forest. Flower positions remain a cosmetic subset of the grass.

Hellfire is placed after the ordinary habitats and before Corn preflight. Up to six interior ordinary wall cells per floor 25+ become floor plus the persistent Hellfire overlay. It never replaces the outer boundary, stair protection, Sphinx reward room, Nest or other protected fixture. Some open/water-heavy floors have fewer than six suitable walls, including none. Initial fire state is initialized immediately.

Changed ordinary geometry receives the existing depth-dependent ambient-light formula. Authored protected light sites are not rewritten.

## Corn dimensions

Floors numbered 29 or greater, beginning with actual Pentacorn eligibility, have a one-third proposal probability. Up to 64 shuffled candidate centers are inspected, with up to four safe full circle candidates evaluated. The radius-five circular overlay contains 81 grid cells. It can replace ordinary walls, trees, chasms or unoccupied water, but it preserves protected fixtures and rejects a circle that would change occupied aquatic/non-floor terrain.

The chamber is filled to at least eight Corn residents without detaching existing mixed hordes. Missing residents are added explicitly and keep their normal authored hearts. At least one is the strongest odd horn count currently eligible: Pentacorn on floors 29–35, Heptacorn on 36–40. The remaining added species are chosen from depth-eligible Corns. These are intentional extra biome rewards, not an invisible reduction in the ordinary population. Current DEBUG reconstruction includes them. Revision-37 legacy reference reconstruction adds those resident hearts after its original ordinary-heart set; historical legacy reconstruction remains unchanged.

A Corn candidate is a transaction: geometry, actors, IDs and spatial links commit together only when all required content and route checks pass. Standard floor links are initialized in that candidate, then Corn-local placement requests three random CA pockets (6–16 cells apiece), four portal pairs and two ordered speed-up strips. The acceptance minimum is three local pairs, one random CA pocket, one ordered strip, eight Corns and the required high odd-horn resident. Failed candidates leave no changed terrain, extra actors, rewards or ID increments.

Accepted circles are substantially denser than the ordinary whole-floor default of one portal pair, one random pocket and up to two ordered strips. Placement failure never silently publishes an ordinary-density room under the Corn label.

## Ordered speed-ups and combined safety

From depth 20, up to two ordinary speed-up strips are attempted, favoring straight hallway sites. Candidates use straight runs of 10, 8, 6 or 4 cells, in that order. Every consecutive pair uses the same serialized physical-link structure as non-Euclidean terrain. Entering from either end normally advances two grid cells per move. Walking directly within the pair already occupied is an ordinary step, allowing reversal or landing inside a strip without being returned to the same position. Projectiles retain the shared physical ray semantics. Neither ordinary random pockets nor ordered strips create projected vision.

Every candidate is checked against the whole combined movement graph, not just its local cells. The proof includes both outcomes of stochastic groups, ordered-pair walking behavior and all existing portal pairs. It is repeated with every portal endpoint excluded together, preserving optional portal traversal. Queries and proofs do not consume live gameplay RNG. Save admission allows up to eight random pockets and eight ordered strips, at most 48 cells each; the generator's current normal/Corn defaults are lower.

## Complexity and determinism

Let V be floor cells, E at most 4V cardinal edges, A initial actors, I ground items, F fixture entries, and K at most 48 cells in an individual spatial area. D is total radius-bounded habitat cells examined, O(A) with the fixed radii. Relevant costs, once per newly populated floor:

- Protection, terrain layers, gas components and ordinary reachability: O(V + E + A + I + F) time and memory.
- Habitat construction: O(D + 64(V + E) + V), including every attempted blocking-cell reachability proof. Ice uses fixed three-round, fixed-radius CA work per anchor. Sorted output and stable candidate order avoid hash-iteration effects on RNG.
- Hellfire candidates: O(V + F) time and O(V + F) scratch; initialization is O(V) over the existing environment channels.
- Each Corn candidate clones floor and population state, O(V + A + I + F) plus variable existing metadata sizes. Up to four candidate copies are evaluated serially, not retained together.
- Random links: at most 48 shapes times 64 assignments per requested area. A proposal costs O(K² log K + V + E + P + L), including assignment construction and combined route proof, with P at most seven portal pairs and L total linked cells. Corn requests at most three additional random areas. Rejected candidates are bounded, not retried indefinitely.
- Corn portals: at most four times 32 proposals, each O(V + E + L), including articulation/optional-route and combined topology checks.
- Ordered candidates: O(V) fixed-length line enumeration plus O(V log V) sorting, and at most 96 whole-graph proofs per placement call. There is one ordinary call and at most one Corn-local call per candidate.
- Extra scratch is O(V + E + A + I + F + L), plus a single transactional candidate's cloned state.

Physical metric routing has a separate O(N² C) bound, where N is its endpoint-node count and C is the supplied clearance-check cost (one for pure metrics, potentially O(V) for raster clearance). Dense topology therefore increases runtime query cost and must be measured; the generation bounds are not a claim that rendering or combat cost is unchanged. Renderer work and runtime admission tests are reported separately.

## Regression coverage

`sub_biomes_generation_r37.rs` covers historical no-op behavior, actual monster anchoring, vegetation, interior-wall Hellfire, depth gates, Corn residents/high odd horns, deterministic ordered links, complete combined route proofs, populated multi-seed generation, and an actual generated Corn floor accepting a turn and exact save/load. The `sub_biome_survey_r37` example emits concrete accepted counts, including actual vegetation counts, without changing generation or adding gameplay RNG draws. Failed preliminary logs remain separate from successful receipts.
