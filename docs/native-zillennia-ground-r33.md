# Revision 33 — deeper Zillennia ground contact

## Authority and scope

Latest designer clarification (instruction 14): Zillennia clears what is on the
floor, including hearts and grass, **not corpses and not water**. This overrides
any earlier interpretation of “erase whatever they touch” as universal destruction.
No living-creature deletion, instant kill, damage, multi-cell body or area aura
has been added. Existing attacks and their damage are unchanged.

“Deeper” defaults to **Greater and above**, the existing canonical quality bands:
Greater 21–24, Hulking 25–28, Ancient 29–32, Timeless 33–36 and Transcendent 37–40.
Infant 1–4, Child 5–8, Lesser 9–12, unqualified 13–16 and Mature 17–20 are unchanged.
The criterion is the actor's stored quality, not its current floor. This cutoff is
an explicit implementation default, not a newly authored numerical balance rule.
All behavior and inspection text are gated to simulation revision 33; revisions
29–32 do not acquire this mechanic.

## Implemented behavior

A living qualifying Zillennia clears its own occupied tile on successful arrival:
normal movement, movement-only haste, forced current/push/teleport movement, ally
swaps, stair/fall transfers and exact offscreen movement share the existing
`travel::enemy_moved` hook. The separate Float Away displacement also invokes
contact after coordinates change. Initial generated placement clears the current
tile, including inactive floors, without activating an unvisited floor's clock.
This is radius zero. Failed/blocked moves do not clear a destination; stationary
turns do not run an invented erasing pulse.

- Remove **all loose ground entries** at that cell, including hearts, equipment
  and consumables. Preserve every other ground entry and its relative order.
- On ordinary `floor` or `grass` with no Ichor, grass becomes ordinary `floor`;
  snow, surface slime and ground-blood overlays clear. This conservative cover
  allowlist is explicit. Existing substrate identity is retained.
- Preserve the player, living actors, corpses, corpse-carried loot, actor-carried
  items and player inventory. Dead or zero-HP Zillennia do not clear anything.
- Preserve water/shallow-water tiles, Ichor, currents, dissolved water blood and
  permanent blood sources. Cover on liquid/Ichor cells is not rewritten.
- Preserve stairs, walls, trees, pits/chasms, doors, grates/bridges, structures,
  bloody-wall food, roots, traps, gases/fire and all non-allowlisted floor layers.
  Immutable pocket-biome theme/flower decoration metadata is not a ground cover
  layer in this implementation and is not rewritten by ground contact.
  Loose items still clear if placed on a liquid or structure-bearing tile; the
  underlying tile/structure/liquid is untouched.

## Persistence and knowledge

The mechanic consumes no RNG and adds no schema fields, hidden inventory, delayed
work or independent state. It uses canonical `Run.ground`, `Floor.tiles`, `snow`,
`slime` and `blood` and the existing Arc copy-on-write boundary. Existing save,
replay, Revert and Remember the Future snapshots therefore retain both erased and
restored state normally. Initial generation retains the prior clean-floor policy;
subsequent actual changes mark the floor dirty.

It does not directly change `seen`, remembered terrain, remembered items or
remembered hazards and does not produce a global log/event. Normal observation
updates knowledge only after a tile is visible again. Read-only inspection prose
is added only through the already observation-gated creature adapter.

## Verification

`bash tools/cargo.sh test -p corpse-core --test zillennia_ground_r33` covers:
all 40 qualities × five revisions; normal contact and radius zero; multiple loose
items/hearts; corpse and carried-loot preservation; player/actor inventory;
living occupancy (no erasing attack); water/Ichor and all floor fields retained;
forced currents; actual haste phases; direct Float Away; initial current tile and
real startup population; unvisited floor-clock preservation; exact offscreen
catchup with stale knowledge until re-observation; save/load, Revert, future
restore and command replay.

Desktop metadata test:
`bash tools/cargo.sh test -p corpse-desktop --bin corpse-desktop creature_abilities_ui`.
This also retains the frozen 1,024-case legacy prose differential suite.
Full-workspace verification belongs to the integrated revision33 build, not a
claim inferred from these focused tests. No platform or visual screenshot claim
is made by this mechanic's tests.

Integrated-source focused results (2026-10-26): all 11 Zillennia regressions pass,
including a final rerun after the legacy early exit; remember 2/2, temporal-command
parity 1/1, travel 4/4 and travel-command parity 1/1 pass. Both desktop prose tests
pass, including all 1,024 frozen fixture cases. The pinned toolchain does not have
rustfmt installed; no toolchain/package version was changed.

## Complexity

Let A be floor actors, F generated floors (at most 40), G floor ground entries,
Z qualifying occupied cells (at most the map's N cells; normally one authored
Zillennia), and N map cells (at most 3,600). Let SF be the full serialized size of
one floor's arrays/maps/environment and SG the full ground block including item
strings/metadata.

- Each arrival does an O(A) actor lookup, O(F) floor lookup, at most two O(G)
  stable scans, and O(1) cover work: **O(A + F + G)** expected time, O(1) scratch.
  The nonqualifying path exits after actor lookup without a floor/ground write.
- Initial placement scans A actors into an expected O(Z)-space hash set, scans G
  entries once/twice, and checks/clears at most Z independent cells:
  **O(A + F + G + Z)** expected time, **O(Z)** scratch.
- A shared floor or ground block incurs existing Arc COW cost **O(SF + SG)**
  time and additional memory, only for blocks that actually change. No ground
  or optional overlay is created merely to clear an already-empty cell.
- Existing arrival burning/submersion/travel work, pathfinding, observation and
  environment ticks retain their own costs; they are not included in a false
  constant-time movement claim. Dirty flags use existing redraw/observation
  paths; no extra rays, raster assets or GPU uploads are introduced here.
- Serialization and temporal cloning retain normal canonical-layer costs. No
  extra retained mechanic state or per-frame simulation work is added.

These are asymptotic bounds, not measured timings. Profile crowded ground-item
floors, multiple snapshot references, haste and exact offscreen catchup before
claiming a performance improvement. Hash-set iteration order cannot affect
results: each cover write is independent and ground retention is stable.
