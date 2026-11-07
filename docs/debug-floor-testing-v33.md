# DEBUG floor testing (revision 33)

Choose **Play from x'th floor** on the existing title menu. The DEBUG setup accepts
floors 1–40, the normal Unblessed/Blessed/Avatar vessels, a random seed by default,
and an editable exact seed. F2 rerolls. F3/F4/F8 cycle the three distinct gods in the
normal seed prefix (Blessed uses the first; Avatar uses all three). Copy/reuse the
full displayed seed to reproduce the same world. Ctrl+C copies the exact seed in setup
or the DEBUG game; Ctrl+V pastes it in setup. Enter reconstructs the preceding
floors, then displays the **real offer at each earned level**; nothing auto-picks a
power. Unblessed offers execute the ordinary seeded +5 Offense-or-Defense grant.
Choices are sequential, so acquisition order and effects such as Adaptive Armor
apply only to subsequently collected hearts, not retroactively.

The separate DEBUG loadout picker retains **all** recovered ordinary loot, including
carried items and arrow bundles. Tab switches between stash and normal Pack; arrows
or Page Up/Down select, Enter moves, E equips through normal slot rules, Space enters.
With no selection, you keep starter equipment. Overflow is retained, never silently
discarded and never used to enlarge the normal twenty-slot inventory. F4 reopens the
stash during DEBUG without restoring health or energy again. Stash edits are
zero-turn actions reconciled with the temporal closing frame, including explicit
saves, so Revert/Future never lose or duplicate recovered item identities.

## Exact reconstruction assumptions

- Uses the same seed's real revision-33 generator, including actual machines,
  pocket populations and initially generated Zillennia, not a mean-stat profile.
- Assumes every naturally generated living, heart-bearing actor on floors strictly
  before the target was defeated and its actual constructed heart consumed once.
  Generated ground hearts are consumed as well. Initially dead/heartless actors do
  not manufacture hearts. Prior collected actors and ordinary loot are removed
  from their source floors, preventing duplicate rewards on backtracking.
- Every ordinary initial ground item and carried item from those preceding floors
  is recovered. Tree taps stay on their source floors as world fixtures; initially
  loose Royal Honey is retained as a consumable, never automatically eaten. Optional
  Sphinx prize cells are not treated as earned. Event-activated
  rewards and finite summoned extras are not generated or silently included.
- No survival, routing, potion/ammo use, environmental losses or other unknown combat
  consumption is simulated. Full health and energy at target entry are an explicit
  debug assumption. Target-floor hearts and loot are never harvested.
- Target selection, heart consumption and choice preparation run no world turns.
  Real target-floor entry setup occurs after loadout confirmation.
- All DEBUG/playtest autosave and high-score updates are disabled. Normal Continue
  is preserved. F7 explicitly exports a separate `<normal-save>.debug.json`; importing
  that file retains the DEBUG marker. Seed-only DEBUG replay export is rejected;
  use the full debug state instead. Legacy v1 playtest replays remain supported.

## Headless workflow

```sh
CARGO_INCREMENTAL=0 sh tools/cargo.sh build -p corpse-core --example debug_floor
# Start; emits exact seed, real current offers, stats, inventory and full stash.
target/debug/examples/debug_floor new '0-5-11:my-test' Avatar 12 /tmp/debug.json
# Read the printed offer IDs; choose one of those, once per earned level.
target/debug/examples/debug_floor choose /tmp/debug.json <CURRENT_OFFER_ID>
# Repeat inspect/choose until offers and remainingHearts are both empty.
target/debug/examples/debug_floor inspect /tmp/debug.json
target/debug/examples/debug_floor take /tmp/debug.json <ITEM_ID>
target/debug/examples/debug_floor equip /tmp/debug.json <ITEM_ID>
target/debug/examples/debug_floor put /tmp/debug.json <ITEM_ID>
target/debug/examples/debug_floor enter /tmp/debug.json
# Actual desktop renderer, sharing the live setup/offer/stash surfaces:
target/debug/corpse-desktop --debug-floor 12 --seed '0-5-11:my-test' --render-start --render-test /tmp/setup.png
target/debug/corpse-desktop --generated --debug-floor 12 --seed '0-5-11:my-test' --render-game /tmp/choice.png
target/debug/corpse-desktop --fixture /tmp/debug.json --render-game /tmp/floor.png
```

No headless command automatically chooses abilities. `new` requires an explicit
seed (desktop default is OS-random), and every mutable CLI invocation writes only
the explicitly supplied debug file. It refuses to mutate a normal save.

## Rough level milestones: r32 checkpoint

Across the existing **127 successfully generated r32 worlds**, collecting all initial
hearts once (not a survival model), level is first earned **during** these floors:

| Level | Median floor | Minimum–maximum |
|---|---:|---:|
| 1 | Start | — |
| 2 | 1 | 1–1 |
| 3 | 1 | 1–1 |
| 4 | 2 | 2–2 |
| 5 | 4 | 3–4 |
| 6 | 6 | 6–7 |
| 7 | 10 | 10–11 |
| 8 | 16 | 16–17 |
| 9 | 25 | 24–26 |
| 10 | 38 | 37–38 |

No sampled world reached level 11 by floor 40. These are r32 checkpoints, not a claim
about the changed r33 population. Source: `relocated-player-progressions-r32.jsonl`,
`exit.all_initial_unblessed.level`; thresholds need 2^L−2 total blessings for level L.

## Updated r33 checkpoint (32 fixed worlds)

The r33 refresh uses `heart-audit-000000` through `heart-audit-000031`, the same
fixed identities as the first 32 r32 worlds. **32/32 generated successfully; no
failures or replacement seeds.** It executes the real generator, heart constructors,
consumption and Unblessed grants. All initially generated hearts, including Nest
inhabitants and initial Godbeasts, are included exactly once. Hypothetical births,
activated event fish and optional quest rewards are not added.

| Level | Median floor | Minimum–maximum in 32 worlds |
|---|---:|---:|
| 1 | Start | — |
| 2 | 1 | 1–1 |
| 3 | 1 | 1–1 |
| 4 | 2 | 2–2 |
| 5 | 4 | 4–4 |
| 6 | 6 | 6–7 |
| 7 | 10 | 10–11 |
| 8 | 16 | 16–17 |
| 9 | 25 | 25–26 |
| 10 | 38 | 37–38 |

No r33 sampled world reached level 11 by floor 40. These are first-earned levels
**during** a floor under full collection, not guaranteed player outcomes or entry
levels. Medians match the larger r32 checkpoint; narrower sample ranges do not
establish tighter universal bounds.

Raw executed profiles: `tools/monster-balance/player-progressions-r33-debug32.jsonl`.
Summary: `tools/monster-balance/level-milestones-r33-debug32.json`. Reproduce:

```sh
CARGO_INCREMENTAL=0 sh tools/cargo.sh build -p corpse-core --example heart_progression_audit
target/debug/examples/heart_progression_audit 32 0 heart-audit 33 > census.jsonl
python3 tools/monster-balance/summarize_level_milestones.py census.jsonl milestones.json
```

The milestone summarizer validates level thresholds, unique seed identities and
40-floor completeness and retains generation failures rather than replacing them.
It intentionally does not reuse the old ordinary-budget equality: initial Nest
blessings are additive conditional population, not changes to the ordinary budget.
Measured audit runtime: 475.54 seconds for 32 worlds on this shared Linux executor;
this is an analysis workload, not the interactive startup time.

## Revision 37 terrain and Corn residents

New DEBUG setups use revision37. Monster-driven habitats are generated from the actual initial roster before prior-floor hearts are collected. Accepted Corn dimensions can add local residents with ordinary hearts; those extra hearts count toward DEBUG progression. Optional quest prizes and summoned creatures remain excluded. Terrain and population are seed-derived, not an average-floor approximation.


## Revision 50 carried jewelry

R50 keeps the same recovered stash, but normal Pack limits are five rings, five
amulets, five potions and five hearts independently, before the existing blessing
bonuses. Trinkets are unlimited. Rings and amulets become active immediately when
carried; Enter returns them to the stash and removes their powers. E is only a
carry-jewelry shortcut, never a capacity bypass or a separate equipment slot.
Returning a capacity-granting item is refused if it would leave another category
overfull. The UI shows each finite category count. Earlier revisions retain the
original twenty-slot inventory and explicit equip behavior described above.
