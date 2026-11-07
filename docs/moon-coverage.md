# Native Valta and Moonspawn

`monster_moon` ports approach blink and charged moonbeam. Blink candidates preserve row-major eligibility, three-cell target radius, five-cell blink distance, LOS, habitat and occupancy, one source RNG selection, teleport cues, terrain hooks, costs and cooldown. Charged beams preserve blocked-shot retention, charge removal before untargetable/range checks, combat power120 and source presentation order. Dedicated caster calls explicitly disable approach-only blink, keeping Valta's retreat policy distinct from Moonspawn pursuit.

17 raw spell cases, two offscreen compositions and five complete command/history/event traces pass against frozen v48. Cases cover both species, cost/cooldown/rooted gates, charging, lethal beam, invisible/distant targets, obstacles/interceptors, normal/haste clocks, cornered melee and permanent burning. No JavaScript is invoked during native gameplay.

## Complexity

For A actors, B bulwarks, L ray length, V cells and S actor metadata bytes, blink checks at most49 bounding-square candidates with O(A+B+L) eligibility per candidate; source disk radius limits accepted cells further. Candidate clones currently require O(49*S) bounded scratch, plus one actor COW block detachment. Beam tracing is O(L*A) with current projectile occupancy lookup; attack/visibility/resource bookkeeping are additive subsystem costs. Teleport terrain refresh may scan V cells or emitters; it is not treated as constant time. Offscreen behavior uses existing environment/catch-up costs. These are analytical bounds, with no benchmark or full-game-parity claim.
