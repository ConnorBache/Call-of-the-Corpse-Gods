# Revision 51: primary loot on floors 1–4

Designer instruction: each of the first four floors has a primary item category.
It drops exactly three of that category and one or two of each other category.
There are two potion floors, one ring floor and one amulet floor; afterward loot
returns to normal.

The dungeon seed deterministically shuffles `[potion, potion, ring, amulet]` with
three Fisher–Yates draws from a dedicated `:early-floor-primary-loot` stream.
Each floor's existing `:floor:N:loot` stream chooses each minor count independently
as one or two, then uses the existing uniform named subtype pools. The schedule is
recomputed from the saved seed, not advanced by visits, turn order, retries or
rendering. Nothing consumes the simulation RNG.

The 5–7 scheduled items **replace** the floor's ordinary allocation, rather than
being added to its old potions/permanents. Each item has its own allocator ID;
amulet instance IDs match. Repeated named subtypes remain possible, as before.
This change introduces no rarity tiers, anti-duplicate catalog rules or guaranteed
specific powers. First-four-floor ordinary drops do not include a Composite bow.
Special-event rewards, monster-carried items, quest items and the separately seeded
arrow bundles retain their existing rules and are not part of these category
counts. Those special rewards still share the one-Composite-bow dungeon budget.

## Placement and later floors

Startup uses the same legal-land spawn sampler, reserved machine/Sphinx/pocket/
quest cells, item dais, arrow placement and atomic whole-floor retry boundary.
An unsuccessful population attempt publishes neither items nor consumed IDs.
The additional ordinary items therefore cannot be placed on walls/stairs, on top
of each other, or in reserved quest/special slots. No new placement fallback exists.

The historical three-floor retained-slot accounting is deliberately preserved:
the early increase must not make floors 5–6 pay back those extra drops. Existing
`ordinaryLoot` and `retained*` diagnostics remain that normal allocation ledger.
Early floor item-stage diagnostics separately record `ordinaryLootGenerated` and
actual potion/ring/amulet drop counts; `items` includes the placed ordinary items
plus arrows and the existing pre-event placements as before.

Floors 5–40 use the existing normal algorithm without schedule/count draws.
Given equal incoming IDs, floor RNG, allocation and Composite-bow budget, the
items and final RNG state are exactly the pre-schedule algorithm's result.
Whole-run later drops can still differ because the early schedule intentionally
changes allocated IDs and removes early ordinary-bow opportunities. No promise
of whole-run equality to old revision-51 seeds is made.

## Persistence and compatibility

No new save field is needed: the seed plus simulation revision determines the
schedule, and generated items are already serialized in their floors. Save/load,
snapshot/Future restores, ordinary seed replay and debug recreation use the same
run-scoped generation entry point. Existing generated floor items are never
rewritten on load. Debug prior-floor collection stages the exact original IDs,
not a fresh roll. Timeweave does not regenerate floors.

Revisions 29–50 retain every old item, count and RNG draw. The stateless
`generate_floor_loot_revision` compatibility helper has no dungeon seed and still
implements normal allocation; production floor generation uses
`generate_floor_loot_for_run`, which owns this run-seed schedule and bow budget.

## Verification

Focused filter: `tools/cargo.sh test -p corpse-core primary_`.
Related budget checks: `tools/cargo.sh test -p corpse-core unique_composite_bow`.
Required aggregate check: `tools/cargo.sh test --workspace`.

Authored regressions cover all twelve permutations across 512 seeds; exact counts,
both minor counts, independent IDs, duplicate-subtype allowance, RNG determinism,
old slot counts 2 versus 20, invalid-count atomic rejection, all later depths
under both incoming bow-budget states, revisions 29/43/46/50, eight real generated
six-floor worlds, legal/nonoverlapping ground placement, arrow counts, unchanged
normal accounting, saved worlds, snapshots/Future, seed-replay restarts and debug
restarts. Actual execution results belong in the coordinated verification report.

## Complexity and bounds

- A primary lookup hashes S seed characters and allocates an O(S) key, followed
  by three fixed swaps: O(S) time and O(S) temporary bytes, O(1) schedule memory.
  It runs once per generated first-four-floor attempt, never per turn/frame.
- Construction takes exactly two minor-count draws plus one named-subtype draw
  per item. For L=5–7 returned items and P total item payload bytes, time and
  returned memory are O(L + P), with O(1) extra scratch. The authored catalogs
  have fixed size and selection is direct indexing; there is no rejection loop.
- Recording counts scans L items three times, O(L) time and O(1) extra memory.
  Later floors add only the O(1) depth guard and no schedule hash or RNG draw.
- Existing sampler setup includes V floor cells, H hazard/setup records and R
  occupied/reserved cells: O(V + H + R) expected time and O(V + R) memory.
  Each item samples at most two V-sized scans and checks G existing ground
  records before insertion: O(V + G) expected time. For L ordinary items this is
  O(V + H + R + L(V + G)) expected time, not merely O(L). Hash-set operations
  assume amortized O(1); pathological collisions can raise each occupancy check
  to O(R + L) and reservation construction to O(R²), giving the conservative
  O(V + H + R² + L(V(R + L) + G)) bound. V is at most 3,600 and L at most seven
  here. Existing fixed-radius heatmap/spacing loops are bounded constants.
  No separate path search or spatial pass was added. COW floor/state cloning and
  bounded complete-floor retries still include all geometry/population costs,
  so these placement bounds are not claims about the entire world generator.
- Save/load and temporal copies keep their existing world-size complexity; no
  new persistent cache, schema root, per-turn scan or additional replay ledger
  exists. These are source bounds, not measured timings.
