# Native revision 35 — accepted verification

Accepted 2026-10-28T03:02:31.658869+00:00. New runs use simulation revision **35**; legacy revisions remain supported.

## Frozen gate

- Workspace: **806 passed, 0 failed, 1 ignored**, across 139 test targets.
- Documentation tests: **0 passed, 0 failed, 0 ignored**, across 1 targets.
- Counts sum each target summary once; no individual-line/summary double counting.
- Workspace, doctest and post-gate frozen-code checks all exited **0**.
- Rust **1.99.1**, test optimization **2**, debug info **0**, debug assertions and overflow checks **on**, incremental compilation **off**.
- Final gate began **2026-10-28 02:48:28 UTC**. All **917** captured inputs match freeze `bb815b72361fc224321c58bfe952ab63c3da9ed00b38ab48f8361ab2e4aac5ec` after the gate and again at finalization.

[Machine-readable receipt](../verification/integration-r35/final-gate-receipt.json), [workspace log](../verification/integration-r35/workspace-tests.log), [doctest log](../verification/integration-r35/doctests.log), [frozen inputs](../verification/integration-r35/code-freeze.json).

## Correction and preserved evidence

The first workspace attempt exposed a real legacy Riptide zero-axis regression: floating-point `signum` converted zero into +1, moving diagonally rather than straight. It was interrupted, the helper was corrected to preserve zero, and focused regression tests passed before this fresh complete gate. The earlier run is **not** an accepted gate. Its logs remain under [superseded-first-gate](../verification/integration-r35/superseded-first-gate/STATUS.txt); [targeted correction log](../verification/integration-r35/direction-regression-fix.log).

Original artwork, lore/catalog data and historical evidence preservation are documented in the [preservation receipt](../verification/integration-r35/r34-preservation-r35.json). Original top-level r34 verification files remain under `verification/historical-top-level-r34/`.

## Additional evidence and limits

- [Independent portal integration](../verification/integration-r35/independent-integration/receipt.json): occupancy, attacks, movement, rays, throws, range, replay/save/Revert/Future, and dream-plane checks.
- [Desktop checks and screenshot provenance](../verification/integration-r35/portal-ui/desktop-verification.json): generated/debug capture and a separately labeled controlled seven-color fixture.
- [Windows build receipt](../verification/integration-r35/windows-r35-build/windows-crossbuild-receipt.json): corrected frozen source cross-built for x86-64 Windows, with static PE/resource checks. The executable is **unsigned**; **Windows runtime was not tested**.
- No claim of exhaustive seed coverage or balanced survival play. Environmental gas/fire propagation through portals was not inferred as a new rule.
