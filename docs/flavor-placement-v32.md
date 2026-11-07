# Flavor-first creature placement, revision 32

## Intent and authority

The designer requested flavor-first depth placement, then numerical rebalance. This pass
uses the existing, owner-authored Natural History and current mechanics. It does not rewrite
lore or grant abilities mentioned only in prose. In particular Witch remains the existing
Slow/Hemophilia/Poison controller; her history is not permission to implement births.

The 57 explicit final bounds in `src/catalog/flavor-placement-v32.json` cover all 55 ordinary
leaders and their two paired followers. They are final ranges, **not** inputs to the former
two-floor widening. The metadata's per-creature target is gross incoming damage over a
completed creature lifetime as percent of initial player max HP, never damage per hit.
Actual numerical outcomes and net loss after regeneration belong to the separate combat
balance report; a role target is not a measurement or a survival guarantee.

Major flavor changes: Witch 8–17 →27–36; Vampire 26–34 →30–38; Banshee 26–35 →29–37;
Inventor/Mech 11–20 →14–23; Greater Breather 18–27 →22–31; Greater Birther 20–29 →23–32.
Non-divine Yeti moves 30–39 →27–36, while divine Moonspawn/Valta move to33–40/34–40.
Minotaur moves37–40 →33–40 but remains a polarizing **Fighter**, not a Skirmisher:
committed movement is an exploitable restriction, so its direct-fight target is high.
Dragon remains38–40 and Chimera35–40, both Fighter/Artillery hybrids.

## Every appearance path

- Ordinary recipes retain thematic family groupings, pair linkage, habitat exclusions,
  Banshee cap, floor-one danger gate, sizes, heart rewards and exact blessing budgets.
  Introduction weight starts at the new final first depth.
- Authored active creature arrangements use the intersection of their members' new
  windows. Geometry, biomes and introduction roll remain unchanged. Previously disabled
  arrangements51 and57 stay disabled; this pass does not revive unreachable designs.
- Dynamic water watcher setup108 has no frozen catalog member list. Revision32 explicitly
  selects an eligible current lower-end species before stencil construction, retaining
  hazard exclusions and never orphaning a linked pair. Frozen watcher selection is
  preserved for prior revisions; this closes the hidden Witch15/Yeti37 backdoor.
- Corpse props can revive, so a separate revision32 40-floor corpse roster obeys the same
  species bounds. The former depth15 Witch corpse becomes Tiger; Witch corpse appears33.
  Every corpse entry is tested for actual depth eligibility.
- Pale births keep existing child species. Conditional manual entries follow the moved
  parent windows, including Greater Birther →Birther →Jackal transitively.
- Godbeast bag/domain schedule remains unchanged, including first event on4/5. All ten
  Zillennia quality bands and nine Ichor species' existing event windows remain unchanged.
  Mercuria/Veridian are environmental events, not ordinary killable enemy templates.
- Sphinx remains an invulnerable optional chamber guardian from12, Samara a rooted terrain
  hazard from8, Bastidion a rooted corpse/gas ecology actor throughout1–40. No artificial
  finite kill-life percentage is asserted for an invulnerable or unbounded exposure case.
- Administrator remains the single fixed floor40 final Fighter; collapse and ascent are
  separate from the direct-combat damage budget. Greater Jackal stays an unspawned legacy
  template, Ichor Fish stays an alias, and player duplicates are not added to the bestiary.

## Compatibility and presentation

`generate_floor_full`/`generate_floor`/`generate_floor_candidate`, ordinary `candidates`,
and legacy setup APIs retain their frozen behavior. Explicit revision-aware wrappers select
new data only for32, with separate immutable candidate caches. New startup and lazy-floor
creation use the saved revision. No saved actor is relocated or retuned by this module.
No process-global switch, gameplay RNG draw, new reward or trait is introduced by lookup.

`update_manual_depths` reconstructs ordinary and conditional depth arrays from current
runtime producers instead of retaining stale conditional lists. It leaves descriptions,
Natural History, artwork and mechanics unchanged. The 79-page manual keeps its existing
identity set and displays current revision32 actor birth stats rather than stale exported
numbers. Legacy runs retain legacy generation, even though the encyclopedia describes new runs.

## Complexity

Let K=57 placement rows, L=55 leaders, P=2 pairs, M≤5 group members, R the attempted
recipes and C the retained recipes. Placement parsing is once-only O(B) time/memory for
B JSON bytes. Bound/reference lookup is O(K) time and O(1) extra memory. Corpse lookup is
O(1), with a fixed40-element table. No per-turn actor work is added.

The ordinary recipe builder keeps its existing O(L²) possible recipe enumeration and
bounded group work. On a cache miss, depth filtering is O(LK); introduction weighting adds
O(M(L+P+K)) per attempted recipe, while sorted duplicate keys cost O(M log M). Existing
encounter score/reward helpers and hash collision costs still apply; see
`native-monster-depth-windows.md`. Two habitat caches across40 depths retain O(80CM) data
per revision family. Cache lookup is O(1); horde feasibility/memoization costs are unchanged
in structure and still depend on candidate reward/size classes, not merely species count.

For an authored setup of M members, new eligibility is O(MK); at most45 definitions are
filtered before the existing bounded stencil selection/packing. Native geometry, lighting,
visibility, pathfinding, packing/repacking and exact-budget checks remain their existing
costs: the wrapper is not an O(1) claim about full generation. No rendering/native-library
cost is involved in placement lookup. Manual synchronization is build-time only, scans40
floors and79 entries, and pays JSON parsing/writing plus candidate cache misses.

## Verification

The final6 focused tests also cover all40 dynamic watcher choices and the original
regression seed0 across a fully generated40-floor world, including every living and corpse
actor. Both current-birth and Zillennia-quality desktop stat-display tests passed.

`tests/flavor_placement.rs` checks all57 IDs are reachable somewhere in the candidate
catalog, exact pairs, no out-of-band species, wet/dry habitat, positive introduction
weights, unchanged legacy30/31 recipes, unchanged authored disabled setups, and all40
corpse choices. It exercises320 plans (4seeds ×2habitats ×40depths), comparing deterministic
repeat output and exact unchanged reward/mean-size budgets. These tests are bounded
coverage, not uniform occurrence or whole-game balance certification. The existing
40-floor generated-population and manual/runtime alignment tests are run separately.

## Final calibration policy clarification

Initial proposal midpoint `reference_depth` values are diagnostic locations, not authority
to permit lethal introductions. Fit gross completed-life damage at the earliest actual
ordinary/conditional appearance, then report first/middle/last depth, net loss, deaths and
censors. Broad overlapping Ichor windows are especially important. Later weakness is
preferable to violating introduction budgets; no hidden level scaling or range narrowing
is added. This supersedes the initial midpoint-fit suggestion without moving species again.
