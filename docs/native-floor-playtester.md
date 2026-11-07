# Seeded floor playtester (legacy v1 / simulation revision 31)

> Historical compatibility reference. The revision-33 menu and CLI floor workflow
> is now [DEBUG floor testing](debug-floor-testing-v33.md): user-chosen level offers,
> full recovered-loot stash, no autosave, normal Continue preserved. The v1 core
> remains only for existing save/replay compatibility. The historical save/menu
> descriptions below no longer describe new floor starts.

The title menu includes **Play from x'th floor**. Enter a floor from 1 through 40,
choose a vessel, and enter or reroll the seed. Tab switches seed/vessel/floor;
left/right changes the focused vessel/floor; Enter starts; Escape or Back cancels.
The ordinary Descend path remains a regular ranked run, not a playtest.

A single background computation prepares the run while input and rendering remain
responsive. Repeated Start cannot launch concurrent generations. Cancel or leaving
the form invalidates the pending result, including when another run is imported;
that result cannot replace a newer run. The cancelled computation finishes privately
before another one may start. Failures leave the previous run/save unchanged.
No partially prepared run is persisted.

## Explicit starting profile

`all-prior-ordinary-hearts-v1` is a **synthetic full-collection reference**, not a
simulation or prediction of surviving earlier floors. It assumes all initial
ordinary heart-bearing creatures on floors 1 through x−1, including the budgeted
machine enemies, were collected. It uses the actual generated actors and the real
`heart_for`, heart-consumption, level-up, and offer-choice paths. Each legal offer
choice is selected deterministically from a seed/floor/heart/level stream.
Adaptive Armor and exhausted-god choices therefore use their implemented rules.
No hand-authored depth-scaled player stat multiplier is added.

The target floor and deeper floors contribute **no** starting rewards. Godbeasts,
sphinx/rare rewards, later summons, environmental respawns, and creature-carried
extra items do not enter this reference budget. Prior collected ordinary actors
and ordinary generated loot are removed from those floors, preventing duplicate
ordinary rewards on backtracking. Rare encounters/rewards remain untouched.

The starting loadout is a seeded shuffle/sample of the actual ordinary loot from
prior floors: at most two legal weapon hands (respecting two-handed items), armor,
two rings, an amulet, and four potions. Missing categories remain unfilled. The
starter bow and five arrows remain; the starter shortsword is retained only when
not replaced. There is no target-floor equipment generation or quality boost.
Floor 1 has exactly the ordinary starter inventory, zero prior hearts/blessings,
and level 1. The reference restores HP and energy to the resulting legal maxima.

The opening history records seed, starting floor, profile, ordinary heart/blessing
budget, and a Pack prompt. Pack displays exact equipment, grants, bonuses and
consumables. An always-visible footer identifies synthetic/unranked play. The
profile and selected item source floor/IDs are serialized with the run; initial
player state, ordinary history, and seed replay remain available.

## Persistence and compatibility

Playtest scores are excluded at the core archive boundary as well as the desktop
archive gate, regardless of native save-envelope provenance. Summaries have no
ranked score. F5/autosave and Continue retain the playtest marker. Seed replay
exports include starting profile and source data; playback regenerates that profile,
checks it, and supports restart. Temporal snapshots carry it through Revert, See the
Future, haste save/resume and normal history serialization.

New runs use revision 31. Revision 29 and 30 seed startup/replay routes retain their
old opening and entrance generation. The revision-31 normal opening is exactly:
“You begin to adventure down to end the corruption of the Flesh.”
The distinct connected bottom-center Ω entrance is documented separately in
[native-floor-one-entrance.md](native-floor-one-entrance.md). No new escape victory
or boss-collapse mechanic is implemented by this change.

## Verification and reproducible inspection

`sh tools/cargo.sh test -p corpse-core --test playtest_start` covers invalid floor/
seed input, floor 1, floor 40, exact actual prior ordinary heart budget (including
machines), repeat deterministic startup, seed variation, legal carrying capacity,
no target rewards, save/load, seed replay/restart, snapshots and score exclusion.
`floor_one_entrance` covers entrance identity, connectivity, old revisions and both
temporal restoration paths. Desktop `startup_ui` tests cover duplicate starts,
cancelled/stale results, errors, disconnect and single delivery; menu tests cover
all seven rows and standard aspect-ratio bounds.

Actual native-renderer inspection commands (not browser mockups):

```
corpse-desktop --render-test start.png --render-start --playtest-floor 20 --seed 0-5-11:floor-playtest-regression
corpse-desktop --render-game floor20.png --generated --playtest-floor 20 --seed 0-5-11:floor-playtest-regression --pack
```

Windows event-loop/fullscreen behavior remains a platform check. Pure lifecycle
unit tests are not a claim of interactive Windows acceptance.

## Complexity and bounds

Let D=max(10,x)≤40; V≤3600 cells/floor; H be all collected ordinary actors; I≤20(x−1)+3
be transient ordinary loot candidates plus starters; A=72 authored ability entries;
P active-floor actors; C player conditions; L generated ordinary loot count. Core
creation executes once per accepted start/replay restart, not per frame.

- Generation cost is the sum of the actual existing D geometry/population/item
  generators, including their candidate retries, visibility, pathfinding and spawn
  heatmaps. See [generation-complexity.md](generation-complexity.md). It is not O(D)
  wall work. Entrance routing adds its separately documented graph work once.
- Heart collection scans H actors and uses real consumption/stat synchronization:
  conservative O(H(I+A+C)+B(A²+I+C)) time, B actual level-ups. Catalog-weighted
  offer construction is O(A²); source order and all gameplay RNG draws are retained.
  Player-stat helpers use the player fast path; the ordinary source actors/loot
  are cloned per floor, O(P+L) transient space. Log/event production can retain O(H)
  short records during preparation and is reset to the explicit starting chronicle.
- Candidate shuffle is O(I) time; six fixed slot scans/removals plus four potion
  selection and source filtering are O(I). At most six normal equipment settlements
  include their ownership/ability/player/actor scans; conservatively O(AI+P+A²+C)
  plus existing equipment hook costs, not assumed constant native runtime. Selected
  items have no crafts or active effect changes, and placement is on a safe stair.
- The generated world is O(DV+actors+items+metadata) retained memory. A prior active
  run can coexist with one preparing run. At most one worker/one completed result
  is retained, including after cancellation; there is no unbounded spawn queue.
- Loader polling, seven fixed menu hit regions and field focus changes are O(1)
  per event/frame. Text editing is O(S), seed bound 160 bytes before normalization;
  rendering uses the existing font-cache/raster/framebuffer costs, not O(1) pixels.
  No filesystem or world generation runs during form drawing.
- Playtest metadata is O(K), K≤10 selected sources. Snapshot clone adds O(K);
  seed replay generation reruns the same startup cost. Save/load still includes
  full bounded serialization/validation, not O(K) overall. Highscore exclusion
  checks the optional marker in O(1) before archive sorting/writing.

These are conservative source bounds, not measured performance claims. The first
seven core playtest tests completed in 15.88 seconds together on the restored Linux
Rust 1.99.1 unoptimized test build. Deep generation dominated; final coordinated
build timings can differ. Profile release starts across multiple seeds, all three
vessels and floor 1/10/20/40, recording generation and post-generation validation
separately. Native GUI/thread scheduling and Windows behavior remain unmeasured.
