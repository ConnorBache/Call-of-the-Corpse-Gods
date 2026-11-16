# R52 Minotaur ordering and collapse shape

R52 is patched in place; R51 and earlier retain their old behavior.

## Minotaur

The player's accepted action resolves before a fast Minotaur's haste step. Its
normal step still follows during the ordinary world turn. The two-step committed
charge direction is not retargeted before the player's attack, and the number of
steps is unchanged. A player's successful haste movement already precedes the
haste phase and keeps that ordering. A normal-phase action does not acquire an
extra haste step. Floor-changing actions do not inject the deferred haste pass
into the arrival floor; normal and offscreen scheduling remain unchanged. Invalid actions do not spend a charge step. Forced slowness
turns and stranding retain their existing action/turn counts. Dead, paralyzed,
sleeping, stranded and non-hasted actors retain the ordinary haste eligibility
checks. The ordering applies to allied as well as hostile Minotaurs.

## Collapse

Each newly reached floor starts with cardinal-only expansion (north, east, south,
west), then diagonal-inclusive expansion (all eight neighbors), alternating on
successive existing two-normal-turn pulses. Including cardinal neighbors during
the diagonal pulse fills the spaces between diagonals: strictly diagonal-only
expansion from a cross would leave holes. The unobstructed first three pulses
therefore cover 5, 21 and 37 cells including the origin, rather than 9, 25 and 49.
Walls still erode. The result is a solid stepped/octagonal front, not a square.

Only the previous frontier expands; new staircase seeds wait for the following
pulse and always start cardinal-first even if seeded on a diagonal pulse.
`diagonalNext` is serialized per floor and defaults to false in older saves.
Save/load and snapshot-based time travel therefore retain the exact next pulse.
R51 and earlier use all eight neighbors on every pulse and omit the false field.
No pulse frequency, flight, fall/death, fixture removal, stair propagation or
victory rule changes, and no simulation RNG is consumed by collapse.

## Complexity

Let A be active-floor actors, K their maximum ability count, S their total owned
payload size (conditions, strings, abilities), V cells per floor,
F current frontier cells, D reached floors (at most 40), I loose objects, L removed
actors and N world actors. Haste ordering adds one actor selection pass to an
accepted non-normal action. Stable phase sorting remains O(A log A) time and
O(A) phase scratch plus the largest individual actor clone per pass. Current
actor-ID resolution scans the active actor list and selection clones each actor,
so selection adds O(A² + A*K + S) worst-case work, not just ability checks;
actors excluded from
a pass do not pathfind or take actions. Existing eligible action costs, including
charge pathfields (terrain/actor/hazard scans and ordered distance-field work),
remain additive and are not counted as constant-time selection.

Collapse remains O(sum(F+V+A+I) + L*N) per pulse across reached floors, including
terrain copy-on-write, entity/fixture scans and maintained-link cleanup. Each
frontier cell tests at most eight neighbors. The boolean adds O(D) persistent
space and O(1) work per floor per pulse; existing masks/frontiers use O(D*V)
space. No new rendering or native-library work, cache, recursion or per-frame
simulation is added. These are source-derived bounds, not benchmark results.
