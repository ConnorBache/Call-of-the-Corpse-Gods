# Revision 37 verification

CORPSE GODS — REVISION 37: SUB-BIOMES AND ENVIRONMENTAL TERRAIN

Implemented: monster-driven forests and mountains; contiguous ice with one-third slipping when moving OFF it; melting snow and cosmetic puddles; ordered two-way speed-up terrain; permanent Hellfire from floor25; round Corn dimensions from Pentacorn eligibility (floor29).

New descents and DEBUG starts use revision37. Explicit revisions29–36 retain historical rules. Original316 catalog/art files and219 historical verification files remain byte-identical; old top-level receipts preserved separately. No bestiary lore/art replacement.

VERIFIED
902 passed, 0 failed, 1 existing ignored across 148 workspace targets. Documentation tests passed. 1068 frozen inputs unchanged across gate and Windows build. The first aggregate attempt was interrupted during compilation; its logs are retained separately, and the final completed gate is authoritative. The first completed gate had900passes and2stale test-boundary failures (revision37 was still expected to be unsupported). Only those two tests were corrected to reject38 and accept37; production code did not change. Failed-gate evidence is preserved.
Independent review:160 unique floors,320 repeated deterministic floor results, geometry/directed-route/optional-portal/ice-anchor checks. Actual generated Corn-floor Wait/Move/Wait plus save/resume regression passed.
Three independently sampled accepted Corn dimensions:8 residents each,4 portal pairs each,8–10 speed-up cells and15–32 random non-Euclidean cells within81-cell circles (31/39/48 topology cells). Normal floor portal generation allows at most one pair; Corn topology is concentrated locally rather than spread across the whole floor.
Separate120-floor generation survey:0 generation errors,23 mountain floors,28 forest footprints all vegetated,204 Hellfire cells,128 ordered areas. One accepted Corn chamber among36 eligible samples: one-third is proposal probability, NOT a promised final occurrence rate. Safety/protected geometry may reject proposals.
Optimized Linux core measurements under concurrent build load: complete Corn-floor Wait53–116ms;40 route queries0.67–1.29ms;40 ray previews2.73–13.4ms. These are not Windows/desktop rendering benchmarks.

DEFAULTS / LIMITS
Ice excludes flying/Wendigo/Yeti; blocked movement, attacks and forced displacement do not roll. Snow leaves cosmetic puddles, not new swimming/current mechanics. Permanent-burning creatures follow existing water exemptions. Mountains preserve routes. Ordered speed-up pairs advance two cells; a direct step to one's current pair partner is ordinary walking so reversal cannot bounce. Random non-Euclidean and projectile behavior is unchanged.
Corn chambers contain at least 8 ordinary heart-bearing Corns; additional residents increase rewards; no claim of unchanged balance. Highest eligible odd horn count is included. Safe optional generation may omit features.
Native-renderer screenshots are full-reveal generated maps with a level1 observer and no turns (plus one controlled terrain demonstration), not balanced DEBUG loadouts. Existing fixed-layout sidebar truncation remains. Full-world QA JSON captures are evidence, not player-save import examples.
Windows x64 cross-build and static PE/resource checks passed. Unsigned executable; WINDOWS RUNTIME NOT TESTED.

PLAYER TEXT
Original prose is unchanged. New assistant-authored copy inventory: verification/integration-r37/desktop-ui/new-player-text-provenance.json. The exact supplied slip/puddle sentences remain user-authored. Replacement batches are curated separately; unknown historical authorship is not falsely called proven LLM text.

Evidence: INTEGRATED-VERIFICATION.json and verification/integration-r37/. Initial failed/interrupted attempts are retained, not substituted for final results.


See [terrain contract](sub-biomes-r37.md) and [generation contract](sub-biome-generation-r37.md).
