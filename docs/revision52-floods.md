# Revision 52 Floods consumes shallow water

This is the current released Floods contract. Native Apply selects a target and
emits `UseAt`, exactly as in the published Avatar World build. The proposed
[radius-four radial behavior](revision52-radial-floods.md) remains deferred.

## Authority and revision boundary

Designer instruction, 2026-11-07: “Instead of floods stop at water, have floods
be consumed by other shallow water, turning it into deepwater”. This supersedes
the shallow-water stopping rule in `revision50-control-amulets.md` and
`revision50-wave-teleport-complexity.md` for exact simulation revision 52 only.
Revision 50/51 saves and replays retain their original behavior.

## Executed behavior

Each lane of the directional 3-by-5 wave independently reaches its first
preexisting shallow-water cell, includes that cell in the preview and committed
wave animation, converts it to deep water, and stops. Existing deep water still
stops the lane without including that cell. Neighboring lanes continue normally.
The footprint and shallow-water membership are captured before terrain writes,
so this cast's new shallow water cannot stop its own wave.

Current narrow implementation inference: the consumed shallow cell is part of
the reached footprint, so a creature there receives the ordinary one damage
packet and up-to-two-tile push before terrain conversion, subject to the same
source/plane/protection gates. There is no additional conversion damage. The
designer's sentence specifies conversion and lane consumption, not a separate
creature exception; terminal impact is not a separately confirmed design change.

Both new shallow water and converted deep water last 12 normal world turns.
Natural shallow water returns to shallow water. Promoting earlier temporary
shallow water replaces its single ledger entry in place, preserves its first
true predecessor, and sets a fresh expiry at this cast's tick plus 12. Thus a
second Floods wave turns its first wave's leading shallow cells deep; a third
wave stops at those deep cells. The original shallow records cannot later
resurrect expired water, and the underlying floor/grass/snow/sand remains intact.
An independent later terrain replacement still wins over expiry.

Closed doors, portals and aliases, grates, permanent authored terrain, Borge and
Epidite keep their existing terrain-replacement protections. No general terrain
stacking is enabled; unrelated temporary edits remain ineligible. Quenching,
source protection, forced movement, ordinary admission and payment rules are
unchanged. Floods costs 30 with the existing player-energy fallback. This
historical amendment originally used a 50-capacity reserve; the later R52
reserve amendment caps it at 10 for the released targeted `UseAt` wave.
R50/R51 retain their original reserve rules. A paid empty wave remains possible,
including against deep water. The frozen Tier 1–9 damage table is unchanged
(20 through 84).

No model field or migration is added. The existing temporary-terrain record may
contain `water` only at revision 52, with the same original-ground whitelist and
single-record-per-cell validation. Save/load, Snapshot, normal-turn expiry and
offscreen catch-up continue through the existing owner paths. Preview stops at
unseen terrain before inspecting it. Animation uses the actual footprint,
independently filters observed cells, and consumes no gameplay RNG.

## Complexity

Let C be reached wave cells (C <= 15), T existing temporary terrain records,
M permanent terrain membership entries, P portal/topology entries, G grate
entries, F floor payload bytes, A floor actors, and U the existing native
visibility-query cost including cache misses. Geometry and shallow-membership
capture take O(C) time and O(C) scratch; the small membership checks in the two
terrain lists cost O(C²), bounded by 225 comparisons. Admission/merge costs
O(C*(T+M+P+G+B)) where B is Borge/Epidite protection-query work. The existing
portal alias/exit helpers may allocate lists proportional to their bounded
topology entries; this cost is included, not treated as free constant work.
Writes may trigger the existing O(F) floor copy-on-write when a Snapshot shares
its payload. Deep promotion adds at most C records and never adds a second
record for the same cell. Expiry remains O(T) scan/storage, plus existing
restoration-cue visibility queries. Actor collection, sorting, source guards,
damage/arrival hooks and up to two forced moves per target retain the costs in
the R50 wave ledger. Preview and animation add O(C*U) visibility work and O(C)
detached cells. No new rendering loop, native-library call, RNG, pathfinding,
cache or animation-driven simulation is introduced. Native wave painting and
frame upload retain the renderer's existing candidate/pixel costs.

These are source-derived bounds, not runtime measurements. Profile casts with
large terrain ledgers and cold visibility caches separately from warm frames.

## Verification

Focused deterministic regressions are in `revision52_floods_tests.rs`: mixed
lanes, diagonal consumption, repeated casts and resource fallback, overlapping
expiry and original terrain restoration, later independent terrain edits,
raw save/load and Snapshot, offscreen catch-up, normal/haste profiles and all
nine damage tiers, hidden terrain/actors and animation-cell bounds, protected
ground/doors/portals, Duel/status protection, invalid records and exact legacy
shallow-water stopping. Existing R50 wave and control tests remain in place.

2026-11-07 cloud core verification: all 14 new R52 Floods regressions and 11
existing wave/teleport tests pass within the full core-library run (638 passed,
2 existing manual benchmarks ignored). The seven amulet-energy integration
tests and ten Snapshot-overwrite integration tests also pass. Core source hashes
were identical before and after the run. Receipts are
`verification/r52-floods/core-full.log`, `core-full.exit`, `core-before.json`
and `core-after.json`. An earlier focused pass had 21/22 passes because the
new save/load fixture accidentally removed required ground blocks; preserving
the empty per-floor blocks corrected that fixture, with no runtime workaround.

Native CPU rendering subsequently passed all six dedicated tests and 23 related
regressions, with one existing isolated GPU test ignored. Fifty landscape/portrait
PNGs were captured; full, retained and tile rendering were byte-identical, and
representative frames were visually inspected. See r52-floods-native-ui-verification.md
and verification/r52-floods-ui-03. The final whole-workspace and Windows cross-build
gates remain separate; none of these checks claims Windows runtime verification.
