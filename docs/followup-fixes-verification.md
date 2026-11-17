# Follow-up corrections, 2026-11-17

## See the Future
4 core missing-reserve regression tests passed, including real capture/return. 10 desktop checks passed across confirmation state, indicator pixels/state, payment gates, approved copy, targeting accounting, and session save/restore. Two opt-in timing tests were ignored. One historical prose assertion was corrected to match previously approved concise text; the corrected test passed. No Windows runtime verification.

## Borge-marked hearts
1 core pickup regression and 1 desktop inventory/header regression passed. Shared display naming now shows `(marked)` in pickup text and inventory/detail UI without altering provenance, raw names, or rewards. Legacy/current row paths both covered.

## Never Ending Flesh
The previous internal ID remains for save compatibility. Revision52 replaces Pursuit with a passive: crossing from at least25% HP to living below25% HP spends30 energy, own reserve first, and restores25% maximum HP. This is not lethal-hit resurrection, does not loop while below threshold, and does not spend if unaffordable. Legacy revisions retain Pursuit.

3 new core regressions, 1 desktop metadata/passive/targeting regression, 10 existing legacy Pursuit tests, and 1 weave/Pursuit compatibility test passed. Eligibility does not clone the player. Full workspace suite and Windows playthrough were not run for this patch.

### Pending designer clarification
Never Ending Flesh's nonlethal-only crossing behavior and maximum-HP healing basis are provisional implementation assumptions, not settled design. An earlier lethal-hit question remains unanswered. Do not publish this combined candidate as a playable release until the designer clarifies. Tests above verify the provisional implementation, not approval of those choices.

## Borge hostile occupancy
4 focused core tests passed for movement/flight/phasing/forced entry, teleport/spawn/revival gates, legacy/allied behavior, and deterministic existing-save relocation. Current hostiles cannot enter Borge's body or altar through shared placement gates; player interaction remains unchanged. Existing overlapping living hostiles relocate to nearest safe floor/grass/shallow square at load/restore without RNG or stat changes. If no valid destination exists, load explicitly rejects rather than retaining the invalid overlap. These focused tests are not exhaustive coverage of every encounter/Windows runtime.

## Separate approved-fixes export
This export excludes the provisional Never Ending Flesh implementation. Its earlier verification notes describe the preserved combined candidate, not this export. Existing Relentless Pursuit behavior remains unchanged here pending the designer's clarification. See the Future, marked-heart, Borge placement, and ordered-animation corrections remain included.

### Approved-only integrated verification
After removing provisional Flesh and integrating ordered animations, 79 distinct targeted tests passed: current Future payment/confirmation/indicator/session save cases; Borge placement and marked-heart pickup/inventory; original Pursuit and weave compatibility; observer rollback; and34 animation/presentation tests. Two opt-in timing tests remained ignored. Full workspace suite and Windows runtime/playthrough were not run. Ordered presentation commits simulation atomically and reveals detached action-boundary observations; it is not a change to simulation scheduling or RNG.
