# Revision 35: portal placement and persistence

The designer requires paired portals on the same floor from depth 10 onward, with entering them always optional. This generator adds one pair per eligible floor as an initial implementation default, not an authored density curve. The serialized representation supports at most seven pairs, with colors blue, orange, red, yellow, green, indigo, violet in that order.

Portals are metadata, not replacement terrain. A pair stores two distinct cell indices and its color index. Runtime actor/item identities remain separate from this metadata. Revision 29–34 floors remain byte-identical and do not acquire portal fields.

Placement runs after ordinary population, special actors, fixtures and ground items. It uses a private seed/floor stream and therefore consumes no simulation RNG, ordinary loot RNG, IDs, actor budget or item budget. Candidates are upstairs-reachable, unoccupied floor, grass or shallow-water cells, outside known room connectors, fixture cells and Sphinx chambers. Stairs, pits, solid structures, trees, walls, grates and permanent fire are excluded. Each candidate has at least three open cardinal neighbors.

The generator computes articulation points in the ordinary nonflying cardinal walking graph. After selecting each endpoint it removes that cell from the graph and recomputes articulation points before selecting another. Consequently, avoiding all chosen endpoints together cannot split any preexisting walking component. It does not rely on portal travel or flying to demonstrate optionality. If insufficient safe candidates exist, it emits only complete pairs; it never rewrites terrain, blocks a required route, or forces a portal into an unsafe location.

Save validation checks the depth gate, maximum count, valid cell indices, distinct endpoints, no reused endpoint, sequential color indices and revision compatibility. It checks both waking and Dreamwalking floor metadata. It deliberately does not recheck initial terrain or connectivity after gameplay has modified the floor; legitimate terrain-changing abilities must not make a save structurally invalid.

## Complexity

Let V be the number of floor cells, E the cardinal walking edges (E≤4V), P the requested pair count (P≤7), A actors, I ground items and F fixture/connector entries. Iterative Tarjan articulation analysis is O(V+E) time and O(V) temporary memory, with no recursive call-stack growth. Placement repeats it at most 2P times, so total placement is O(P(V+E)+A+I+F) time and O(V+A+I+F) temporary memory. Metadata occupies O(P). This occurs once per newly generated floor. Structural save validation is O(P) time and O(P) temporary memory per floor. The standalone optionality diagnostic recomputes articulation analysis per endpoint and is O(P(V+E)), not a constant-time check.

Measurements and gameplay topology checks are recorded separately; these bounds are not a timing benchmark or a claim that all possible terrain modifications preserve bypass routes.

## Non-player travel integration

Revision 35 also uses the same paired-cell distance fields for hunger routing,
offscreen pursuit, and queued ally stair-return progress. Local steps go through
central movement once, rather than assigning coordinates directly; this keeps
local diagonal checks, remote occupancy, and arrival contact hooks together.
Cross-floor placements are not walking through a portal, but reserve either
representation of its logical cell. The revision <=34 direct movement branches
remain unchanged.

Portal-aware fields retain their existing BFS/Dijkstra bounds with O(V) mate
index scratch and at most seven pair links. Food routing still builds at most two
fields. Offscreen pursuit rebuilds its O(A) occupied-cell set after central moves
(the same strategy already used after special AI movement). Landing occupancy
checks add O(P), P<=7, mate lookup; no per-candidate path search is introduced.
