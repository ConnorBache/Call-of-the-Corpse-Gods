# R38 geometry failure audit

## Fixed reservation collision

Revision 38 uses `reserve_godbeast_room_revision`. Distant Verdania tap candidates
and the deterministic in-room fallback exclude the complete 3×3 footprint around
both planned stairs. The footprint, not just the final stair cell, matters because
stair recesses also create wall/torch cells. The tap remains protected from later
setups and terrain cleanup by `protected_generation_cells`.

The legacy public room function delegates to revision 29. Explicit revisions
29–37 retain their old candidate list, draws, metadata, and errors. The known
`r37-event-count-0047` floor-20 collision is reproduced under revision 37 and
checked under 38. Prior progression failure `heart-audit-000071` used revisions
31/32 and floor 27; its published diagnostics remain historical evidence, not
silently overwritten.

## Bounded candidate recovery

A native whole-floor caller can request candidate windows 0, 32, …, 224 through
`generate_floor_full_revision_from_attempt`. Each window still applies the exact
same quality, connectivity, landmark, fire, vent, and mandatory-Nest checks to at
most 32 candidates. Windows preserve the original seed's domain, Godbeast bag,
Nest depth, and stair plan; only the existing candidate geometry stream advances.
No required content or safety validation is removed. Exhaustion still returns an
explicit error rather than accepting an unsafe candidate. Historical revisions
reject nonzero windows.

Revision 38 treats these existing structural failures as candidate rejection,
rather than aborting the whole run on the first candidate:
- Protected geometry prevents a repairable/accessibly connected stair approach.
- Protected authored geometry leaves an inaccessible room.
- A layout cannot assign an exit route or cardinal room bridge to its room graph.
- A diagonal doorway has no cardinal approach.
- A connectivity repair cannot consume terrain productively.
- No safe Verdania tap location exists.

The preexisting retry classes remain: scaffold/exit/budget/entrance failures,
unsafe furnace/vent/recess containment, and an unplaceable required Saidribee Nest.

## Audited failure boundaries

- Unsupported depth, dimensions, domain, revision, recovery window, or R38
  candidate number are configuration errors and remain rejected immediately.
  R38 candidate numbers are bounded to 0–255 to avoid arithmetic overflow.
- `actual_up` is integer-only and follows the established stair planner's bounded
  clamping policy. Extreme integer coordinates cannot index outside the grid.
- Native room/schema conversion, frozen catalog parsing, and generation stage
  order errors are programmer/data invariant errors, not random candidate failures.
  They are not masked with retries.
- Missing entrance metadata is an invariant violation: native candidates paint
  the entrance before cleanup. It remains an error.
- Sphinx, ordinary portal, random non-Euclidean, and ordered-speed-up placement
  have bounded optional proposals; safety rejection is absence of an optional
  feature, not a failed generated world.
- Phantasmii duplicate population is invalid repeated invocation. Insufficient
  reachable dream-population cells or blocked required dagger placement are
  post-geometry population failures handled at startup's atomic boundary; this
  module does not weaken them.
- Nest room/inhabitant/honey metadata and occupancy remain validated. Its native
  geometry reserves all cells before ordinary actors/items. Missing metadata is
  not cured by silently omitting Queen or honey.
- R37 sub-biome generation is already atomic for optional Corn chambers and
  graph-validated for added chasms/topology. It never changes stair cells or
  protected quest positions. No new relaxation was made for R38.

## Complexity

Let V be floor cells, A≤2 planned stairs, C=32 candidate attempts/window, W≤8
startup recovery windows, and G the preexisting full candidate cost (including
layout, setup packing, topology and connectivity searches). Tap selection costs
O(V·A) time and O(V) candidates. Recovery adds at most O(W·C·G) time and no extra
retained candidate copies beyond the startup transaction; a single floor still
uses O(V) transient topology/room layers, excluding ordinary retained run state.
Actual candidate-generation G includes existing O(V² log V) connectivity repair,
not merely a linear tile scan. Initial successful first windows incur no retry
work. Tests cover invalid windows, deterministic windows, preserved stairs/domain/
Godbeast identity, and the verified collision. Finite tests are not an exhaustive
proof over every possible seed.

## Revised roster preflight

Setup packing previously tested the remainder of its machine/enemy heart budget
against the frozen revision-29 ordinary roster, even on current floors using the
revised roster. R38 now checks that remainder against its own revision's eligible
horde budgets before accepting each stencil. Older revisions retain their old
preflight exactly. This prevents an apparently valid machine arrangement from
leaving an unrepresentable ordinary-horde remainder; it does not alter total
budgets or invent creatures. The helper is the same bounded feasibility search
used by population planning (see population/startup audit for its bounds).

An independently discovered second structural family is preserved by the
`生成-é-🌱-37` regression: normalized seed hash 2657626501, floor 13, carried
upstairs (15,18). R37 aborts its first candidate with "Authored geometry leaves an
inaccessible room". R38 rejects that candidate and runs the next ordinary
quality-checked candidate in the same window. This is not a content-dropping
repair of an unsafe map.

## Post-population hazard-free route preservation

Independent 1,024-world QA found generated maps whose base geometry had safe
stairs, but whose later mountain chasms removed the last fire-free approach.
The R37 habitat deletion proof used a bare terrain BFS that allowed permanent
fire and grates as substitute routes. This was weaker than the base generator's
safety contract. Verified seeds include `r37-event-count-0078` floor 30, `93`
floor 32, `生成-é-🌱-113` floor 27, `0-1-2:qa-120` floor 22, and
`11-10-9:qa-127` floor 27. The focused regression checks all 40 populated
floors for each of these five seeds. On the first seed, restoring only the
six late chasm cells restores safe stair access; crossing permanent fires also
connects the broken bare graph, proving why the old check incorrectly accepted it.

R38 habitat tree/chasm deletions now retain both the prior bare terrain proof and
the complete before/after `reachable_without_braziers` set, and require accessible
stair approaches after each accepted deletion. The current candidate is restored
if either proof fails. This rejects only unsafe optional terrain proposals; it
does not remove the monster or its habitat anchor, add a forced portal, or carve
through protected scenery. R37 uses the exact previous branch. Two additional
bounded BFS scans per attempted deletion retain O(V+E) time and O(V) scratch, with
at most 64 deletion attempts per populated floor. No new player-facing copy.
