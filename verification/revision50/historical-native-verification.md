# Revision 50 native verification

This is a tested source checkpoint, not a playable Revision 50 release. Normal
Revision 50 startup/storage/replay admission remains closed. The following run
predates the newly approved ring edits; their separate verification is pending.

## Previous stats-only checkpoint checks

- Official pinned Rust 1.99.1 installed with designer approval in the workspace.
- Final complete workspace suite: **1695 passed, 0 failed,
  39 ignored**, across 173 completed target summaries.
  Counts are test executions, not unique tests. The profile uses optimization
  level 1, with debug assertions and overflow checks explicitly enabled.
- Focused R50 integration targets: 97 passed, 0 failed,
  1 ignored. The ignored full Engine50 replay test remains
  blocked by deliberate release admission, not silently considered passing.
- Unoptimized final core unit pass: 306 passed, 0 failed,
  2 ignored.
- Catalog/source contracts, snapshot root schema and JavaScript generator syntax pass.
- Twelve baseline software-rendered screenshots inspected; see the UI QA report.
- 896 paired native diagnostic encounters ran, with 4,820 commands. This is a
  bounded bare-player comparison using old progression profiles, not campaign tuning.

## Issues found and fixed

- Corrected a test fixture's string-to-Field conversion so native tests compile.
- Added missing revision/owner-scoped environment admission for integrated R50
  powers and the explicit caster marker, with legacy/NPC boundary tests.
- Prevented Waking Nightmare decoys from acquiring biological hunger clocks.
- Corrected an All-Seeing test to distinguish a free action from activation success.
- Corrected a preexisting UI test that wrongly included the changing HUD in its
  Chronicle pixel comparison. Production rendering and map/toolbar checks are unchanged.
- Guarded See The Future anchor creation before any mutation pending the new
  all-energy payment rule; preserved existing-anchor serialization/restoration tests.
- Implemented the superseding stats-only Mini-Me rule with no abilities, item
  grants, reserves, statuses or ability resources inherited.

The first unoptimized aggregate run was interrupted by an execution connection
reset, after 75 completed targets and 580 passes. Its incomplete log is retained;
the complete final-source run above supersedes it. A later pre-stats-only run
completed with 1,695 passes and the one stale UI test failure fixed above.
Existing warnings were not
silently treated as errors or proof of missing runtime behavior.

## Evidence and limits

Exact commands, log hashes and counts: `../verification/revision50/summary.json`.
Full log: `../verification/revision50/workspace-stats-only-final.log`.
UI report: `../verification/revision50/ui-qa/REPORT.md`.
Balance method/results: `revision50-player-offense.md`.

No Windows execution, playable R50 launch, R50 campaign acceptance or performance
certification is claimed. Ability capacity/funding, amulet/item definitions,
Cogworks wall health, caster classification and Future funding decisions remain
open. The original UI draft, locked dependencies, pinned compiler
and historical mechanics gates are preserved.

## Expanded item pass in progress

The first jewellery aggregate completed 180 targets with 1817 passing, seven
failing and 39 ignored test executions. It exposed one legacy floating-point
regression in Fight Long (multiplication replaced historical division), now
corrected without changing R50 Vampirism. Six remaining failures were invalid
fixtures: missing empty floor blocks for storage, wrong haste phase, a Roots
damage-provocation assertion, and an expectation predating fixed Bastidion gas.
The corrections await the expanded final run. Two earlier compile attempts found
test-only View cloning and private actor accessor use; both were corrected.

The new potion/hover compile found a missing empty darkness field in a lighting
cache test initializer; this is corrected. No passing aggregate is claimed for
the current potion/hover source yet. The original eleven jewellery screenshots
were inspected; compact status labels and actual item-hover captures are now
awaiting rebuilt inspection.

The official Windows Rust target, pinned MinGW archives and SDL2 SDK are installed
in the approved workspace, with checksums and version probes recorded under
verification/revision50/windows-dependencies. No game Windows build or execution
has run. Dependencies alone do not enable the closed R50 release gate.
