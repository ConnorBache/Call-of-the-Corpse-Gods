# R52 minimum starting creature Dodge

## Approved rule

On 2026-11-09 the designer said, “Let's base start everyone at 10 dodge as the minimum.”
This is a fresh real-body birth rule, not a lower bound on effective combat stats.
New R52 non-player bodies use `max(10, 10 + authored Dodge offset)`. Profiled species
retain the exact natural/Nest/Queen/Witch/Birther/Greater Birther key's first-appearance
offset on every floor. Higher existing starting values are unchanged. New players
already start at Dodge 15 and remain 15. Accuracy and the ratio formula are unchanged.

The sole simulation change is the existing shared `combat::creature_birth_dodge`
conversion. No catalog row, growth rule, temporary status, item, spell, hit override,
RNG call, population recipe or serialized field changes.

## Birth-path audit

- Generic population templates and revision-33 additions use the physical birth
  overlay. This includes ordinary hordes, machine enemies, generated corpses,
  fungus-feast victims and Corn Dimension residents.
- Natural floor profiles, Nest residents and Queen/Witch/Birther children replace
  raw birth stats through the same conversion, using their exact contextual keys.
  The later overlay replaces values instead of adding a second baseline.
- All fungus/vent-fungus/Bastidion floor templates, all Zillennia qualities,
  nine ichor-fish species, the Sphinx room guardian, Flesh Administrator, Ahvamitt
  and the eight Phantasmii Nightmare tiers also use the same conversion.
- Ordinary copies, Mini-Me snapshots, Clay Soldier live-stat proxies, Half Here
  and Dreamwalking endpoints retain their existing copied stats. Loaded, grown,
  wounded, revived, travelled and temporally restored bodies are not reborn and
  are not traversed or retuned. Their recorded Dodge can remain below ten.
- Waking Nightmares are intentionally statless one-hit hallucinations, not new
  species bodies. Their zero stats and special dissolution behavior remain exact.
  Transient environmental/spell point actors are also not creature births.

Existing R52 saves keep their recorded actors but newly generated bodies use the
minimum. Earlier simulation revisions retain their exact births. As with other
same-revision amendments, replaying an older R52 history that creates affected
bodies can diverge; this is not a retroactive save migration.

## Numerical effect versus the released first-appearance baseline

Among the 550 authored floor-profile rows across 64 exact keys, 106 rows across
15 keys change: 94 rows across 13 natural keys and 12 rows across the two contextual
keys `nest:short-saidri` and `queen:short-saidri`. The other 444 rows/49 keys stay exact.
Every affected new profiled body now starts at Dodge 10.

| Exact key | Released starting Dodge | New starting Dodge |
| --- | ---: | ---: |
| blorb | -1 | 10 |
| hemogoblin | 5 | 10 |
| needleling | 5 | 10 |
| pale:Jackal | 6 | 10 |
| rat | 7 | 10 |
| shellback-beetle | -4 | 10 |
| short-saidri | -1 | 10 |
| dryad | -3 | 10 |
| pale:Biter | 0 | 10 |
| pale:Screamer | -3 | 10 |
| pale:Spitter | 0 | 10 |
| hemogoblin-bloodmancer | 9 | 10 |
| bear | 1 | 10 |
| nest:short-saidri | 6 | 10 |
| queen:short-saidri | 6 | 10 |

Special/fallback bodies change too. All 40 fungus tiers were Dodge 2 and become 10.
Infant Zillennia (floors 1–4) were -6 2/3 and Child Zillennia (5–8) were 1 2/3;
both now start at 10. The first Nightmare tier (1–5) rises from 1 to 10. The first
three ichor-fish species rise from 9 1/3, 2 1/3 and -2 1/3 to 10; the six higher
fish species remain unchanged. Other low generic fallback templates include
Bombardier 7, Samara 2, Bastidion 2 and Pale Breather 4 2/3, all now 10.
The compatibility `ichor-fish` template also rises from 9 1/3 to 10. These are
raw starting stats; existing effective-stat integer rounding is unchanged.

At effective Accuracy 15, an ordinary alert melee target with Dodge 10 is hit
75% of the time. Previously 14 of the 15 affected profiled keys gave 100%; the
Bloodmancer at Dodge 9 gave 83 1/3%. This is a 25-point or 8 1/3-point reduction,
not a change to the formula. Existing sleeping/wandering surprise guarantees
remain guaranteed. Ranged base chances still multiply by `0.9^range`, and Bow
miss reduction and explicit overrides keep their existing order.

Penalties may still lower effective Dodge below 10, including below zero.
The calculation alone still guards its denominator with `max(Dodge, 1)`.
Player Dodge 15 and creature Accuracy are unchanged, so this change alone does
not alter those creatures' chance to hit a fresh player.

## Deterministic verification

`revision52_birth_dodge_minimum` covers every generic species template, all 550
profile rows (also projected across floors 1–40), exact contextual keys, negative
and fractional fallback offsets, all special tiers and all nine eligible fish,
production save/load, snapshot restoration, copying, revival, below-ten dynamic
penalties, Mini-Me/Clay copies, statless decoys, player Dodge 15, actual seeded
75% alert attacks and unchanged sleeping/wandering guarantees.

The existing `balance52_integration`, `revision52_stat_followups` and
`combat::revision52_accuracy_tests` use an independent updated birth oracle and
exercise real contextual births, identity/rewards/clock invariants, lifecycle,
travel, Weave/Future, legacy formulas, range loss and hit/miss overrides. Exact
commands, named tests and results are recorded under
`verification/revision52-birth-dodge/`. These focused checks are not a complete
workspace, packaged release or Windows runtime certification.

## Complexity

For a finite scalar offset, the new minimum adds one constant-time comparison
at each existing new-body Dodge assignment: O(1) time and O(1) extra memory.
There is no new allocation, RNG draw, cache, traversal, world lookup, per-frame
or per-attack work. Existing first-appearance profile selection retains its
O(R*L) fixed-catalog scan (R=550 rows, L=key bytes), existing tuning scans and
one-time catalog parsing. Save/load/copy/revival and effective-stat derivation
retain their previous costs. These are analytic bounds, not measured timings.
