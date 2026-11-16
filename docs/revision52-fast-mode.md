# Revision 52 Fast mode

Fast mode is an opt-in new-descent setting, separate from Avatar. `fastMode: true` is stored on the run and seed replay record. Missing flags retain Normal behavior and wire shape. Continue, replay restart, snapshots, Future anchors, inverse snapshot headers and archive summaries preserve the recorded mode. Invalid null flags and Fast records outside the current R52 Avatar configuration fail admission; command preflight rejects invalid flags before mutation.

## Route and generation

Fast starts on floor 1 and visits 1, 3, …, 39, then floor 40. Every ordinary physical descent advances two effective depths; 39 to 40 is the final one-depth step. Physical ascent returns to the previous generated floor. The effective number is the canonical `Run.depth` and `Floor.depth`, not a display translation: domain, creature roster and normal stats, light/terrain growth, encounters, items, equipment tiers, special rewards, and effects all read the actual destination depth. Skipped floors are not generated or populated. Player arrival prefetches one physical floor, not one numerical depth. Explicit teleport destinations are already effective depths and are never accelerated a second time. Skipped/non-playable targets are rejected before travel mutation.

A Godbeast scheduled within the crossed two-depth interval is placed on the destination floor. Its kind comes from the crossed five-floor schedule block. Everything else, including ordinary population and any depth scaling of the boss or its rewards, uses the destination depth. For example, 4 to 6 receives the scheduled floor-5 Godbeast. The normal generator keeps its original entry points and RNG streams.

Physical routing is shared by stairs, Ascend, ordinary and forced falls, descent potions, airborne chasm pursuers, queued allies, off-screen transfers, corpses and ground objects. Collapse propagation uses existing physical upper floors. The floor-one blue entrance and floor-40 Administrator are unchanged. Escape collapse propagates from 40 to 39, then through the odd floors back to the blue entrance. DEBUG accepts odd floors 1–39 plus 40, reconstructs only the real prior visited floors, applies the same doubled heart awards and preserves every level-choice pause. Skipped even targets are rejected before generation.

## Compensation and ordinary drops

Only consumed heart blessings and stat awards are doubled. See [heart rewards](fast-mode-heart-rewards.md) for ordinary rounding, Epidite composition, storage and previews. Energy restoration, creature stats, heart counts, Adaptive Armor and other bonuses are unchanged.

Arrow allocation remains the ordinary schedule: each five-effective-floor set chooses two or three distinct depths, each yielding five arrows. Normal therefore retains 10–15 generated arrows per set. Fast generates only selected bundles on visited depths. A set with two visited depths can yield 0, 5 or 10 arrows; a set with three visited depths can yield 0, 5, 10 or 15. Bundles on skipped depths are not relocated or topped up. For seed `0-1-2:fast-arrow-3`, the first set schedules depths 1 and 3, so both Normal and Fast receive 10 arrows in that set. The final set includes visited depths 37, 39 and 40. No additional drop compensation was added.

The existing retained-loot allocator consults at most two preceding effective depths. A skipped depth uses its existing nominal fallback (6 slots when divisible by 3, otherwise 7). It is not synthesized as a floor and does not contribute actual items or hearts. All explicit catalogs indexed by depth retain their normal meaning.

## Sparse identity audit

Runtime world access is by stored floor identity, including save validation, JSON item-memory validation, gas capture pilots and debug loot filtering. The remaining `depth - 1` array indexes are fixed, complete authored per-depth catalogs, not the sparse world array. Both gas pilot implementations independently look up the requested physical depth. Existing clone-based off-screen and dream-plane views retain the mode flag.

## Complexity

Let F be stored physical floors (at most 40 in Normal and 21 in Fast), V a floor's cells (at most 3,600), A its actors, I its items, S serialized bytes, and K complete generation attempts (at most 8, each with at most 32 geometry candidates).

- Mode checks, downward routing, two-depth boss-interval selection and explicit target admission are O(1) time/space. Upward routing scans at most F floor identities: O(F) time and O(1) space.
- Generating N new visited floors retains the existing O(N × K × (G + P) + identity reconciliation) time and COW draft costs, where G includes geometry/pathfinding/quality analysis and P includes population/loot placement. Skipped depths incur none of G or P. The added schedule lookup is constant-sized. No new live-frame generation loop is introduced.
- Travel retains its existing generation, visibility, catch-up, placement and actor costs. Added floor routing is O(F) at an upward boundary and O(1) downward. Multi-floor corpse/item falls traverse only existing/created physical successors, at most F, with each floor's existing O(V + A) or O(V + I) placement and explicit generation costs.
- Save admission retains O(S) parsing/serialization and its subsystem validation. Physical floor/memory lookups now scan up to F identities, adding O(F²) worst-case identity comparisons for at most F memory blocks, bounded by 1,600. No grid or actor payload is duplicated for those lookups.
- Both gas pilots retain Normal’s O(1) floor-array address validation. Fast uses O(F) floor identity lookup, plus gas-name bytes, with O(1) extra scratch. Canonical gas values are unchanged.
- DEBUG adds one O(F) list of actual prior depths, then retains its existing O(A + I) reward enumeration and real heart/offer/ability-settlement costs. Eight-field heart scaling is O(1) per consumed heart; no skipped actors or loot are fabricated.
- Snapshot/replay/archive mode storage is O(1); existing clone, serialization, history and world validation bounds still apply. No directory scan or extra replay file read was added.

See [menu control and rendering](fast-mode-menu.md) for native text/raster/cache costs and [heart rewards](fast-mode-heart-rewards.md) for fixed-field arithmetic. These are analytical bounds, not measured speedup claims. The Fast toggle does not constitute a frame-spike fix.

## Verification status

Final staged workspace/test compilation passed with Rust 1.99.1, locked/offline dependencies and the canonical profiles. With two test threads, the focused suite passed 23 Fast tests and four heart-reward tests; the complete core library suite passed 806 tests, with zero failures and four explicitly ignored tests. The snapshot optional-root schema check passed for 27 roots. Exact commands, source manifests, timestamps, executable hashes and raw logs are retained under `verification/fast-mode-final`.

The failed diagnostic logs are retained. The original replay comparison was corrected to compare independently admitted engines with strict complete equality. Ascend's return value means turn consumption, so its regression verifies the actual depth change and 50-energy debit. A test approach whitelist incorrectly excluded legal water around Ahvamitt's teleport at floor 11; the final fixture requires production passability/habitat, an unoccupied non-falling cell, and records the exact local terrain. A shared DEBUG heart queue now counts doubled blessings consistently for actor, carried and ground rewards without modifying stored items.

Desktop tests, including the opt-in nine-frame menu export, compile but have not been executed in this staging pass. Native capture inspection, combined full workspace tests after integration with other changes, and release packaging remain separate gates. No commit or publication was performed. These checks do not establish a performance speedup or a legal end-to-end traversal.

## Production encounter regression fixtures

The focused suite uses real startup and complete generation/population pipelines for three deterministic seeds. `0-1-2:fast-boss-production-287` covers all seven encounters and the full 21-floor route; its relocated actors include Ahvamitt at 11, Greater Zillennia at 21, and Phantasmii at 31. `fast-boss-production-29` covers relocated Mercuria, Borge and Verdania; `fast-boss-production-3` covers relocated Epidite. The tests compare destination-depth ordinary reward budgets and actor constructors, exercise real entry and interaction/phase functions, and strict-load saves before and after phases. Player approach positions and specifically documented offered/exposed heart setup are controlled fixtures, not a claim of legal end-to-end traversal.

Fast DEBUG tests independently enumerate the real prior visited-floor heart rewards, compare the doubled blessing totals, preserve target-floor actors, finish real offer choices, and strict-load the preparation state at starts 1, 5 and 40. Explicit skipped-depth requests fail before generation. Synthetic route tests separately cover ordinary falls and the 39→40 step, ally arrival in both directions, Ascend, final collapse propagation and the blue-door win. The seed-replay test compares complete admitted state before and after an accepted Wait, and can export genuine unedited generated startup/replay evidence.
