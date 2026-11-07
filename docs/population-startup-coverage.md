# Native population and startup

`startup::create_run(seed, mode, dev)` now builds an actual native ten-floor run. `startup::ensure_floors` grows the world through floor40. Neither invokes JavaScript or substitutes a pre-generated fixture. Geometry is the separately verified generation pipeline. `create_run_with` is an explicit dependency-injection boundary used to isolate startup tests, not the normal entry point.

Implemented native: exact budget-feasibility DAG and weighted horde planning; independent population RNG streams; initial sleep; all authored actor templates including Pale/Greater, fungi and depth-band Zillennia; inventor carried potions and linked pairs; connected placement and aquatic habitats; heatmaps/spacing; setup reservations and fungus-feeding corpses; encounter metadata; Sphinx optional-room search and rewards; retained-loot accounting, excluded embodied-god grants, arrow bundle distribution, dais/tap placement; initial IDs, starter RNG draws, ability state, logs, entry hooks and terrain/item observation.

Immutable catalog snapshots hold authored actor templates, finite depth/habitat recipe expansions, weighted single-creature rosters and constants. `tools/export-population-catalog.mjs` regenerates these from the frozen source during development only. Runtime uses typed native objects. Recipe planning, feasibility and all placement execute in Rust; no seed-specific output catalog is used in production.

Ground item placement and legacy pile normalization preserve item identity, quantity, entry order, nearest-cell ties, direct falls, terminal discard, dirty flags and metrics. Generation placement uses the exact known-free-cell fast path, with the same metrics as authoritative placement. Public full-floor and ensure-floor mutations are transactional on failure.

## Differential evidence

`tools/cargo.sh test -p corpse-core --test population`:

- 240 full horde plans:40 depths,3 seeds including astral Unicode, land/water eligibility.
- Every bestiary actor template plus fungus and Pale samples, awake/sleeping state, IDs and exact RNG draw counts.
- Seven full ordinary/machine population phases, complete heatmap weights/flags/candidate order and complete floor state.
- Fifteen complete spawnFloor cases spanning all Godbeast event types, Sphinx, setups, budgets and late floors; all resulting run fields checked.
- Twenty Sphinx structural attempts, room/reward cells, complete modified floor and draw counts.
- Ten item-placement/normalization boundary cases including fractional/outside origins, occupied spaces, falls and bottom-floor discard; full state/metrics/callback sequence.
- Complete ten-floor startup in Avatar, Blessed and Unblessed, both isolated against oracle geometry and fully native procedural generation; every serialized field compared. Default production entry is exercised by the native test.

This establishes the tested generation/startup cases, not every gameplay interaction or Windows launch. Creature construction does not establish complete runtime AI behavior. Additional traversal/arrival and more seed/mode coverage are being integrated separately. No speedup claim is inferred from the port.

## Complexity

Let V be cells/floor (2291 standard;3600 supported maximum), K authored recipes, H selected hordes, M members/horde (<=12), S setup footprint entries, A actors, I generated/existing items, D generated floors (<=40), C feasibility states and T distinct cost transitions.

- Immutable catalog: one typed parse per process, O(catalog bytes) time and memory. Template clone O(actor template size). Each actor creation adds at most9 slime writes and bounded inventory work.
- Exact budget solver: O(C*T) time and O(C+T) memory. Budget<=48, groups<=96, actors<=194, positive<=groups, banshees<=1; groups decreases on every recursion edge. Recipe filter and weighted selection add O(H*K) work plus bounded pair-adjustment queries. Current feasibility helper does not maintain a global cross-call cache; repeated setup checks include their own solver cost.
- Heatmap O(V+room/connector metadata+S+25*fixtures), O(V) scratch. A sampler pick is two O(candidate count) passes; one spacing update touches a radius8 disk.
- Horde placement O(H*V*r²+H*M*V), r<=6, plus habitat BFS O(V), catalog/feasibility costs above, and fire exclusions. Each radius inspects every eligible anchor once; there are two fixed radii. No random retry or discarded required member.
- Machine planning O(S*setup footprint lookup+S) with bounded group12; occupancy validation O(A+I+S), plus per-kind fire-exclusion costs. Encounter records preserve insertion order.
- Sphinx existing-door search O(doors*V), fallback O(V) fixed-size stencils, O(V) temporary memory. Floor visibility refresh costs O(A*121*ray length) worst case for razorleaf emitters, separately from geometry.
- Spawn integration includes geometry-independent planning, O(I*V) roulette, O(I²) ground collision verification and metrics, retained-budget scan O(D), plus cloned current floor O(V+metadata). Transactional Run cloning shares floor/actor/ground blocks but copies scalar collections/player/inventory/history strings; this is not universally O(D) for large temporal strings.
- Ground placement O(sum(V_d+I_d)) over <=40 visited floors, plus newly requested generation. Free integer origins use O(I_d) occupancy construction. Normalization O(total I)+cost of displaced placements; O(max I_d) temporary occupancy.
- New run: sum of exact generation and spawn costs over10 floors, starter catalog filtering, then current-floor visibility and memory. `ensure_floors` extends only the missing suffix through40. Debug test elapsed times are test-run observations only, not gameplay performance benchmarks.

Revision 51 replaces the two fixed fungus corpse props with a current-depth and
an actual +2/+3-first-appearance victim, immediate active roots, and whole-feast
omission when impossible. See [the feast contract](revision51-fungus-feasts.md)
for exclusions, late-floor bounds, deterministic streams and additional costs.
Legacy population differential APIs retain their original corpse behavior.
