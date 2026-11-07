# Revision 37: sub-biomes and environmental terrain

The designer's 28 October 2026 seven-part terrain specification is implemented for new descents and DEBUG starts. Explicit revisions 29–36 retain their original generation and simulation rules. Old saves are not silently upgraded.

## Authored rules

- Sub-biomes follow actual initially generated monsters, rather than choosing a theme and assuming suitable monsters will appear. Existing population is generated before the new habitat-decoration pass; it is not rerolled.
- A valid voluntary move **off** ice has a one-third chance to consume its action without moving. The player reads “You slip on the ice.” Flying creatures, Wendigos and Yetis are exempt. Entering ice, blocked movement, attacks and forced displacement do not roll. Ice uses contiguous cellular-automaton components anchored on the actual ice monsters.
- Fire or a non-flying burning creature contacting snow melts it and quenches ordinary burning. Permanent-burning creatures retain their intrinsic fire, consistent with water. A cosmetic puddle remains with the designer's exact inspection sentence, “a puddle sits on the ground.” Puddles do not introduce swimming, currents or new liquid mechanics. Ice does not melt merely because snow does; ice melting was not specified.
- Actual Wendigo and Yeti locations receive mountainous regions with more chasms, without cutting existing walking routes. Forest-associated monsters use the established Dryad, bear, tiger and Saidribee habitat associations, rather than a new ecology.
- Ordered non-Euclidean speed-up areas pair consecutive squares along straight runs to advance two cells per move, favoring hallways. They use the same movement, occupancy, projectile and no-projected-vision rules as other non-Euclidean links, with predictable two-way traversal rather than mirrored endpoint bouncing or random double-source branches. When already on a pair endpoint, stepping directly to its partner is an ordinary one-cell move, allowing reversal or departure from interior starts; entering a different pair still advances through its link. This walking-only exception does not alter shared occupancy or projectile transfer.
- From floor 25, selected ordinary wall cells become walkable Hellfire. Its black/red/orange rendering differs from ordinary fire. The terrain remains burning and ignites entrants under the usual fire immunity rules. Fire spreading outward is ordinary finite fire, not new Hellfire.
- Corn dimensions begin at actual Pentacorn eligibility. They are round overlays with many Corns and unusually dense portals, ordered speed-ups and random non-Euclidean pockets. At least one resident has the strongest odd horn count currently eligible for that depth.

Detailed population ordering, candidate limits and safety proof: [generation contract](sub-biome-generation-r37.md).

## Initial numerical defaults

These counts and generation probabilities are implementation defaults, not additional designer-authored rules:

- Mountain footprints use radius five around each actual ice monster, with local radius-four cellular ice. The ice automaton runs three Moore-neighbor rounds, retaining the cardinal component containing the monster.
- Forest footprints use radius three around established forest-associated monsters. Terrain removals across habitat decoration are bounded at 64 per floor and individually checked for route preservation.
- Hellfire replaces up to six safe ordinary wall cells per eligible floor. It does not replace outer boundaries, stair protection or authored fixtures.
- Speed-ups begin with the existing non-Euclidean depth threshold, floor 20. Up to two ordinary ordered runs favor hallway sites.
- Corn dimensions have a one-third proposal probability on eligible floors, starting at floor 29. A radius-five circle is the initial size. The chamber has at least eight Corns, including a Pentacorn on floors 29–35 and Heptacorn on 36–40. Additional residents keep their normal authored hearts, and DEBUG reconstruction counts those extra rewards; this update does change the rewards of accepted Corn floors.

An accepted Corn chamber must contain at least three local portal pairs, one ordered strip, one random pocket, eight Corns and the required high odd-horn resident, with a valid route proof. Up to four complete circle candidates are tried; rejected candidates leave no geometry, actors, rewards or ID changes. Generation may omit optional features when safe placement is unavailable. Accepted density, sampled omissions and concrete test evidence are recorded in the release verification, rather than equating a requested count with a successful placement.

## State and visibility

Ice, snow, cosmetic puddles, Hellfire and remembered terrain are serialized state. Speed-up topology is stored separately from random non-Euclidean topology, but both use the same physical-link helpers. Save/load, replay, Revert and Future preserve actual state and gameplay RNG. Remembered terrain describes the last observation; unseen melting and unseen exits are not leaked.

The original bestiary lore, illustrations and ability prose are not rewritten. New assistant-written player text is tracked for the designer's subsequent ten-at-a-time replacement workflow. The exact supplied slip and puddle sentences are designer-authored.

## Verification limits

Native software-renderer captures show the actual game renderer. They are not proof of a Windows interactive session. Windows artifacts are unsigned cross-builds with PE/import/resource inspection; Windows launch, input and platform save behavior remain untested unless a separate runtime receipt states otherwise.
