# Revision 51 blocked enemy pursuit

When the existing planner selects a step toward a hunting enemy's last-seen
target, a newly occupied or newly impassable step can be replaced before the
movement action is attempted. The enemy chooses a legal step that strictly
reduces its remaining navigation action cost. It commonly steps diagonally past
a slower creature. It can also start a real detour which temporarily increases
straight-line distance. This is not a Euclidean/Manhattan-distance heuristic.

The fallback is revision 51 only. Revisions 29–50 retain their original path
choices. Clear planned steps are unchanged, including the existing hazard-first
planner. The replacement uses the shortest legal movement-action cost, counting
orthogonal and diagonal moves equally and one extra departure action on sticky
mud, roots, or nonimmune slime. Flight and slime immunity remove their existing
sticky cost. The source's already-earned `stuck` escape is honored. Slow/haste
still schedule the same actions at the same times; this does not create a bonus
action or predict how other creatures will move on future world turns.

## Scope and knowledge

- The enemy must already be hunting and the supplied goal must equal its
  existing `last_seen`. The fallback never reads the quarry's actual position
  into its route, changes `targetId`, refreshes sight, or acquires a new target.
  An unseen player or decoy moving elsewhere does not redirect the route.
- Ally/neutral orders, stalking waypoints, wandering, corpse feeding, fleeing,
  retreat, committed Minotaur charges, forced moves, confusion, paralysis,
  rooted creatures, and other action restrictions are not reinterpreted.
- Existing AI uses full floor terrain/topology, not the player's explored map
  or an enemy exploration grid. This retains that contract and its physical or
  dream-plane floor. No new observation, reveal, or visibility mutation occurs.
- Current living nonquarry blockers and all their logical portal aliases are
  obstacles. The target's remembered cell is a terminal contact goal, rather
  than permission to enter an occupied body. Its final contact cost may lead to
  an attack from adjacent legal habitat, as in the existing pursuit field.
  If that remembered target is on forbidden stairs, the terminal goals are
  instead its legal adjacent contact cells, honoring wall-corner clearance,
  habitat and occupancy. No terminal exemption allows an enemy onto stairs or
  an occupied adjacent cell. This adds at most eight probes per target alias.
- If the blocker disconnects the goal, the source already is the goal, or no
  currently legal move strictly improves the cost, the existing blocked action
  remains. The enemy does not wander randomly, wait for a predicted ally move,
  pass through bodies, or grant itself another action.

## Admission and determinism

The graph uses current hard terrain, grates, event solids, Clay Soldiers,
temporary obstructions, Bulwarks, flight, phase, aquatic/ichor/slime habitat,
wall-corner rules, ordered speed-up pairs, and physical/dream occupancy. The
existing slime-trail path allowance remains in future route cells; the selected
first step is also checked against current movement admission.

Deterministic portals cost one move. A stochastic edge is admitted only when
every arrival is traversable and has a finite remaining cost. Reverse Dijkstra
uses `departure cost + maximum remaining exit cost`, the shortest **guaranteed**
navigation cost. An unpredictable cycle with no finite guaranteed progress is
not treated as a shortcut. No random exit is sampled during planning. The
ordinary movement pipeline alone commits any authored portal RNG draw.

Equal-cost routes use the existing direction order N, E, S, W, NE, SE, SW, NW.
There is no new gameplay RNG or persisted field. Save/load and temporal snapshots
retain their existing state formats. Failed original movement is never attempted
before replacement, so movement payments, status rolls, sticky struggles, and
events are not executed twice. Movement still goes through `engine::move_actor`.

## Complexity

Let V be floor cells (at most 3,600), A current actors, L spatial links, G grate
entries, M terrain-modification entries/cells, T temporary-terrain entries, F
floor lookup cost, and B the actor's ability/condition query cost. These bounds
are static analysis, not measured timings.

Every eligible already-planned pursuit step performs a bounded-eight-outcome
collision/terrain probe. Its helpers include actor/alias and plane lookup,
O(A*(F+L+B)) worst-case, and movement terrain checks; a solid step can additionally
invoke the existing Cogworks breach planner. The latter retains its separately
documented mask and distance-field cost. A clear step does not build a new graph.

Only a blocked step builds a fresh occupancy mask O(A*(F+L)) and V-cell terrain
mask O(V*(F+M+T+G+B)), including helpers rather than claiming constant-cost
passability. The graph has E <= 8V movement edges and K total arrival references
(K <= 2E under admitted two-outcome spatial links). Building edges includes
portal/ordered-pair scans O(V*L + K log Kmax), with Kmax <= 2 under current
admission. The reverse shortest-guaranteed-cost pass is
O((V+E+K) log V) time and O(V+E+K) extra memory. Selecting a first step scans E
edges once and performs at most eight live admission checks. There are no eight
separate path searches. Nothing is cached beyond this call, so actor movement
cannot stale the fallback's occupancy mask. Per phase, multiply this work by
the number of actually blocked eligible pursuit requests, at most the existing
scheduled movement requests; normal and haste phases remain separate.

The timing span `path.blocked_reroute` isolates mask/graph/solve work for future
representative crowded-floor profiling. It does not include the original
planner, final movement, environment effects, visibility, or rendering. No
native/Windows performance or platform behavior is claimed from asymptotics.

## Verification

`revision51_enemy_reroute` tests cover unexpected slow-body occupancy, diagonal
detours, exact deterministic ties, disconnected routes, wall corners, phase and
flight, aquatic/slime habitat, sticky costs and the actual movement struggle,
known goals versus hidden actual targets, unchanged legacy revisions and clear
steps, restricted states, newly solid terrain, portal cost/occupancy aliases,
guaranteed stochastic progress, and exact Chebyshev costs on a 3,600-cell graph.
Focused and aggregate execution results are recorded in the release verification
report; test existence alone is not a passing or platform-verification claim.
