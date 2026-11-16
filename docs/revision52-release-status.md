# R52 candidate scope and verification limits

The sections below are historical checkpoints, not interchangeable certification
of the latest source. The current Avatar/world follow-up starts from the exact
published R52 Gameplay and Visuals complete-source checkpoint, retaining its
disclosed seven-file reconstruction history. This follow-up itself required no
additional recovery. Its authoritative final source/test/build receipts belong
under `verification/r52-avatar-world-final/`; only successful matching receipts
certify that combined source. Focused UI, roots, Duel and generation checks do
not replace the complete workspace gate. New R52 behavior is specified by
`current-design-amendments.md` and its linked subsystem documents. The source
snapshot now covers all documentation and dependency notices as well as code,
assets and fixtures.

This follow-up preserves the R51 candidate. New normal and DEBUG runs use simulation revision 52; older saves and replays retain their recorded revision. All work and verification described here ran in the cloud, without accessing the user's PC.

## Darkness and action feedback

Darkness immediately sets effective illumination to -100 in the approved radius-3 circle (29 cell centers), restores current-source lighting after expiry, and renders only observed changes. The focused circle checkpoint passed 17 core tests, seven native/render tests and one unbounded-display guard. Ordinary zero-Perception and high-Perception before/after/expiry frames were inspected. Earlier square receipts are historical only.

Normal, Reduced and Instant action-animation settings passed 20 focused persistence/UI checks. The approved feedback matrix maps 170 entries, including explicitly static or ordinary-action cases. Its focused checkpoint passed 73 core and 23 native tests; actual amulet, inventory, Dreamwalk and Ascend frames were inspected. A subsequent Cogworks construction/removal reversal passed one core and one native test with eight inspected frames. The matrix distinguishes executed examples from source-only cases; it does not claim 170 independently executed gameplay scenarios.

## Targeting and creature follow-ups

R52 uses the reachable physical line: Iceblast detonates on the first creature or last reachable square before terrain, and Blink lands on the last legal empty square. Lightning keeps its explicit piercing behavior. Sparkle remains lit until extinguished; ordinary water quenches it and Ichor-covered water does not. Speed-up uses saved Non-Euclidean destination links with adjacent reciprocal pairs. Matching numbers identify paired endpoints, not one label for a whole region.

Dragon takeoff costs eight from its full 50-energy pool, allowing six activations without replenishment. Flight lasts six normal turns and requires a route with a planned safe landing. Blorb adds one protected caustic damage after a positive melee hit. Each positive Killspawn melee hit adds a three-turn bleeding budget of rounded half the actual hit, subject to the documented protections.

The boundary-focused creature/Darkness pass completed 33 tests, including inactive/resting allies, Fast ties and malformed-save rejection. Amulet targeting passed 35 tests and restorations 14. The native aiming/speed/copy/status batch passed 11 tests with 17 actual screenshots. Exact scopes are recorded in verification/revision52-enemy-followups and verification/r52-targeting-sparkle.

## Role-specific balance

The integrated catalog contains 550 birth profiles, retaining the original 150 Fighter rows. Contextual families, casters, supports, saved special coefficients and the deliberately harder Nest are included without retroactively retuning existing actors. The final focused core checkpoint passed 646 tests, with zero failures and two ignored. Independent final-library checks passed 31 harness fixtures, exact compiled-catalog equality and 240 sampled combat comparisons.

The all-hearts Fighter calibration used 192 generated worlds plus 64 held-out worlds. Its combined floor estimates remain within 1.72 percentage points of target, with wider disclosed held-out variation. Contact-mechanic validation includes complete Killspawn bleed tails. Other roles use their own spatial/support/control scenarios, not a renamed Fighter metric. See revision52-fighter-balance.md and revision52-all-role-balance.md for curves, sparse outliers, trapped-gas censors and scenario definitions. These are not campaign-safety or per-seed guarantees.

## Amulet tiers and Bestiary

Names use “Tier X Amulet of Y”. Higher tiers appear as new drops; existing items retain their birth tier and separate remaining energy. Direct damage types are fitted independently toward approximate rising casts per individual, preserving Tier 1 values and area/control tradeoffs. Costs and 50-energy reserves remain unchanged. Blinking, Beckoning and Teleportation gain one target-range tile per tier; Discord and Obstruction gain one duration turn. Birth tiers and captured periodic rates survive copies, saves and time travel.

The tier mechanics checkpoint passed 654 tests, including all 25 new tier regressions. Native tier presentation passed eight focused and 50 related regressions, with three preexisting ignored tests and eight inspected captures. Fractional-scale cancellation residue was corrected without relaxing the exact pixel oracle. A subsequent final-catalog diagnostic passed 23 actual-dispatch groups, including all 63 tier/effect first packets. The broader-role durability analysis retains approximate bands and distinct creature durability. See revision52-amulet-tiers.md and revision52-amulet-tier-ui.md.

Bestiary pages show natural base-stat ranges across authoritative spawn floors, rather than stale revision-33 bodies or an invented selected floor. Legacy runs use their recorded revision; context-only creatures retain honest separate templates/ranges. Unknown pages and persisted discovery stay protected. Fifteen focused tests plus 56 existing tests passed, with one preexisting ignored benchmark and 16 native landscape/portrait captures. See revision52-bestiary-stats.md.

## Floods shallow-water consumption

Each R52 wave lane consumes at preexisting shallow water and deepens that terminal cell. Existing deep water still stops it. Promotion preserves the true underlying terrain and uses the existing 12-normal-turn lifetime. The terminal shallow cell retains ordinary damage/push as a narrow implementation inference, not a separately confirmed creature exception. Earlier revisions retain their former rule.

All 14 new Floods regressions and 11 old wave tests pass within the 638-test core library checkpoint, with two preexisting ignored benchmarks; seven amulet-energy and ten Snapshot tests also pass. Six native tests and 23 related regressions pass, with one existing GPU test ignored, 50 real captures and strict full/retained/tile pixel equality. Repeated casts, independent lanes, expiry, immersion tint, aiming, animation and hidden-water privacy are covered. Earlier fixture-only failures are retained in their original attempt logs. See revision52-floods.md and verification/r52-floods-ui-03.

## Final release gates

The actual DEBUG floor-40 fire-entry regression passes for R52 alongside retained R51/R50 checks: three tests, including Unblessed/Avatar entry and four turn/save-resume comparisons. That focused checkpoint predates the final additional-role catalog. It is not a reproduction or explanation of the original unsupported-behavior notice.

An interim pre-tier source checkpoint passed the complete Linux workspace suite: 2,468 passed, zero failed and 42 ignored across 204 targets. Its snapshot is c1d3348f022d4d132a46006f166cab1096b39362aea5e30b77acdd84f8e1d578, with receipts in verification/r52-resolved-workspace-01. It does not certify later changes.

A fresh unfiltered workspace pass and Windows x64 release cross-build are required on the final frozen source before packaging. Their authoritative receipts are generated under verification/r52-final-workspace-01. These design notes alone do not claim those gates passed. The R52 packager accepts only successful matching source/test/build reports and rejects stale evidence or overwriting an existing archive. R51 remains separate. Synthetic packaging tests are not game execution.

Windows runtime, physical GPU, monitor/DPI transitions, filesystem power-loss behavior and the original unsupported-behavior report remain unverified unless a later explicit receipt says otherwise. Linux raster frames and static PE/import checks are not Windows runtime testing.

## Movement and darkness follow-up (2026-11-07)

This R52 amendment consumes Weave of Time on arrival, moves fast Minotaurs after
the player's action, alternates collapse expansion cardinal-first then diagonal,
removes linked-space player aliases, simplifies arrows and topology markers, and
implements the designer's silhouette-darkness and Brogue chasm presentation.
See `revision52-weave-consumption.md`, `revision52-charge-collapse.md` and
`r52-ground-markers.md` for exact mechanics and scope. Existing R52 saves receive
the amended behavior. Historical <=R51 mechanics retain their gates; previously
recorded R52 replays involving changed mechanics can yield different results.

The original certification above covers the original R52 candidate, not this
amendment. Only the matching successful reports under
`verification/r52-movement-final-01/` certify this follow-up. Focused reports
under `verification/followup/` and `verification/r52-movement-focused/` are not
full-workspace certification. Windows cross-compilation is not runtime, GPU, or
physical monitor/DPI testing on Windows.

## Accuracy amendment: approved birth baselines

The ratio-accuracy amendment floors denominator Dodge at one. New players start
with Accuracy 15 and Dodge 10. New non-player creatures use Accuracy 15 plus
existing species/floor offsets and Dodge 10 plus existing offsets. The designer
approved the creature Accuracy baseline provisionally on 2026-11-08, resolving
the prior early-enemy zero-hit blocker. No individual birth catalog was retuned.

The movement build and earlier accuracy checkpoint do not certify this newer
source. A fresh complete workspace pass, Windows release cross-build and
matching package receipts are required. See `revision52-accuracy-formula.md`
for exact compatibility boundaries, impacts and current verification scope.
