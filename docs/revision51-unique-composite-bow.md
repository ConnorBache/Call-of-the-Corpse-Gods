# Revision 51: one Composite bow per dungeon

Designer instruction: “Also, only one composite bow should spawn per dungeon.”

Simulation revision 51 limits generated Composite bows to one across the entire
run, including pre-generated floors. Ordinary floor loot, Sphinx prizes and
Mercuria use the same persisted `compositeBowGenerated` budget. Once the first bow
is generated, the weapon entry is excluded from later permanent-item draws;
remaining ring and amulet entries retain equal weight. Ordinary slot counts,
permanent counts and distinct first two permanent categories are unchanged on
normal floors. The later primary-loot instruction replaces ordinary drops on
floors 1–4 with potions/rings/amulets only; see [primary loot](revision51-primary-loot.md).
This is an at-most-one rule, not an extra guaranteed placement or a new drop rate.

The flag is set on generation, not pickup or ownership. Leaving the bow behind,
dropping it, losing it or destroying it cannot open another slot. Floor-generation
attempts operate on the existing detached candidates, so failed attempts do not
consume the published run's budget. All choices still use their existing seeded
generation streams, never simulation RNG. Revision 51 seeds are deterministic;
their post-bow loot draws intentionally differ from older revisions.

The normal starter Bow and arrows, bow damage, unlimited trinket carrying, actor
item copies, and ownership transfer are unchanged. Creature generation creates
potions, not additional bows. Debug reconstruction and synthetic deep starts use
normal floor generation and retain its budget when staging or carrying loot.
Ground-item placement, falls and drops relocate an existing identity rather than
create a new bow and do not reset the flag.

## Persistence and temporal boundaries

New revision 51 shells start with a false flag. Saves, cloned rollback snapshots,
native/serialized See The Future anchors and restores retain it. Timeweave only
repositions the player and does not change the generation budget. A Future restore
restores the complete earlier world and its matching generation state; it does
not retain a bow from the discarded future alongside newly generated loot.

Simulation revisions 29–50 retain their original constructors, IDs, RNG order,
optional-field absence and snapshot patch encoding. Older saves are not upgraded
and no preexisting bows are deleted. A defensive revision 51 import missing the
new marker scans every stored ground floor, inventory, actors (including nested
pilots and copied inventory), item memories, and debug/staged loot before further
generation. It records any surviving evidence without changing those items.
An external revision 51 save that has discarded both the marker and all evidence
of a lost bow cannot reconstruct that historical loss; normal saves always retain
the marker. Invalid null/non-boolean markers are rejected by save validation.

## Checks

Focused filter: `tools/cargo.sh test -p corpse-core unique_composite_bow`.
Legacy loot/reward fixtures: `tools/cargo.sh test -p corpse-core equipment::generated_loot`.
Snapshot schema: `python3 tools/check-snapshot-roots.py`.
Required aggregate check: `tools/cargo.sh test --workspace`.

Authored regressions cover forty successive floor/event streams across 64 seeds,
real forty-floor generation including Sphinx rooms, debug and synthetic deep
starts, actual Mercuria activation, pickup/drop/loss, save/load, snapshot/Future
restoration, missing-marker ownership evidence, starter items and exact legacy
draw/ID preservation. Executed results belong in the coordinated verification
report; this document does not claim they have passed.

## Complexity

- Normal budget lookup/set is O(1) time and memory, with one serialized boolean
  per run/temporal state. It performs no per-turn or per-frame world scan.
- Revision 51 ordinary generation keeps O(L + E + P) time and memory for at most
  L=20 slots, exclusion bytes E and total returned item payload P. Candidate
  groups are fixed arrays of two or three entries; there is no rejection loop.
  Event selection is O(1) plus returned payload allocation. Existing <=50 catalog
  and modifier costs remain unchanged.
- A missing-marker import scans O(I + A + G + M + D) entries: inventory items I,
  actors/nested pilots A and their items, ground items G across at most 40 floors,
  remembered/detected items M, and debug/preparation items D. Each identity check
  compares fixed-length tags, with O(h) stack for nested pilot depth h and no COW
  detachment or extra item storage. This scan runs only once per missing marker.
- Snapshot/Future copying adds O(1), while existing metadata serialization and
  validation still scale with their complete payloads. Floor retry bounds and
  all generation/population costs remain those of the existing pipeline.

These are source complexity bounds, not measured wall-clock timings.
