# Native revision 36 — accepted verification

Accepted **2026-10-28 15:39:08 UTC**. New runs and DEBUG starts use simulation
revision **36**; legacy revisions retain their historical semantics.

## Frozen release gate

- Workspace: **865 passed, 0 failed, 1 existing ignored**, across 142 targets.
- Documentation tests: **0 passed, 0 failed, 0 ignored**, across one target.
- Commands: `sh tools/cargo.sh test --workspace --locked --all-targets --no-fail-fast`
  and `sh tools/cargo.sh test --workspace --locked --doc --no-fail-fast`.
- Rust/Cargo **1.99.1**; test optimization 2, debug info 0, debug assertions and
  overflow checks enabled, incremental compilation disabled.
- All **1,057** source/build inputs matched before and after the full gate.
  Fingerprint: `f0b38fb5e06763c1cae69d2fc0a72c8cc647cd612ca75b50ba5c2ae1eff2ec03`.
- Gate started 15:25:54 UTC. Workspace execution took 790.731 seconds;
  documentation execution took 1.216 seconds. These are measured gate times,
  not game frame-time claims.
- Counts sum each test-target summary once; individual result lines are not
  counted again. The complete gate exited 0 without source edits or failures.

[Gate receipt](../verification/integration-r36/final-gate/final-gate-receipt.json),
[workspace log](../verification/integration-r36/final-gate/workspace.log),
[doctest log](../verification/integration-r36/final-gate/doctests.log),
[frozen inputs](../verification/integration-r36/final-gate/code-freeze.json).

## Focused and independent coverage

The final combined affected-core sweep passed **222 tests across 24 targets**
before the full gate. It includes the new 13-test independent integration suite,
17 physical-topology tests, generation tests and prior portal/creature/movement/
projectile/temporal/DEBUG regressions. Read the
[focused receipt](../verification/integration-r36/independent/combined-focused-receipt.json)
and [complete focused log](../verification/integration-r36/independent/combined-focused.log).

Cases exercise exact committed fair draws, blocked/rejected/pure-query RNG
stability, shared occupancy without telefragging, one real item identity, both
stochastic NPC movement outcomes and offscreen movement, native Dreamwalking and
Half Here planes, actual throws/potions/NPC shots, and local optical distance and
occlusion. Save/load/replay/Revert/Future reproduce topology and random outcomes.
Complete DEBUG starts at floors 20 and 40 retain revision-35 hearts, choices,
loot, stats, rewards and costs. Fresh ordinary starts remain unmarked by DEBUG.

Independent review caught and corrected pre-gate edge cases: adjacent source
aliases were not genuine zero-range jumps; legacy throws retained their second
raster-blocker check; a charged legacy Moonbeam still discharges when out of
range; actual random NPC misses retain spend/cooldown/phase and cannot apply
effects beyond the authored range. These were corrected, not hidden by weakened
expectations. Early incomplete-compilation/capture attempts are preserved and
clearly marked intermediate or superseded, not accepted release gates.

## Generator evidence

Assignment uses the live **eight-neighbor adjacency and existing wall-corner
rule**. CA components remain cardinally connected. Two independent directed
checks prove traversal both with existing portals and with all portal endpoints
excluded, so the pocket cannot make previously optional portals mandatory.
Adversarial cases reject a trap with only one safe stochastic branch and a trap
that could otherwise escape only through a portal.

- Geometry sample: 131 accepted pockets / 144 floors across six domains and
  depths 20/30/40. There were 343 literal assignment attempts: 212 rejected
  complete attempts and 131 accepted. Accepted groups included 131 two-source
  groups / 624 groups; 70/30 governs proposal draws, not this conditioned ratio.
- Four fully generated/populated 40-floor runs: **77/84** eligible floors had
  pockets; seven safely omitted them. **0/76** floors below 20 had pockets.
- The detailed populated floor-20/30/40 subset accepted 11/12 pockets. The sole
  omission had no eligible CA component within the shape budget, not a forced
  repair or malformed attachment.
- No safety rejection occurred in those random samples; the separate adversarial
  tests explicitly exercise safety rejection. Neither sample is exhaustive.

[Current generation summary](../verification/integration-r36/generation/summary.json),
[geometry sample](../verification/integration-r36/generation/geometry-sweep.json),
[complete populated inventory](../verification/integration-r36/generation/populated-sweep.json).
Earlier four-neighbor assignment receipts are retained as superseded evidence.

## Native-renderer evidence

The full frozen gate regenerated the captures using the actual native rasterizer:

- [Generated floor-20 pocket](../verification/integration-r36/desktop-ui/non-euclidean-generated-floor20-debug.png):
  DEBUG generated map, controlled player placement and full reveal, no turns simulated.
- [Possible exits](../verification/integration-r36/desktop-ui/non-euclidean-possible-exits-controlled.png):
  controlled topology fixture; cyan inset and possible-exit marks, not generated play.
- [Epidite at nine turns](../verification/integration-r36/desktop-ui/epidite-9-turns-controlled.png)
  and [ten turns](../verification/integration-r36/desktop-ui/epidite-10-turns-controlled.png):
  controlled clock fixtures; subscript present at nine, absent at ten, exact
  `X turns until judgement` description. Tests also cover 30, 1 and rollover.

See [capture commands/provenance](../verification/integration-r36/desktop-ui/README.md)
and [capture receipt](../verification/integration-r36/desktop-ui/receipt.json).
Headless native rasterization is not interactive-window or Windows runtime proof.

## Windows and preservation

The exact frozen source cross-built for x86-64 Windows with official MinGW-w64
and SDL2 2.32.10. Static PE checks found the expected 84 embedded RGBA plates,
seven icon resources, manifest and dependency closure. The only non-system DLL
dependency is the supplied verified SDL2.dll. The executable is **unsigned**;
**Windows runtime was not tested**. Its SHA-256 is
`625fa2a4f2ec4d2014b07ab66c17d4cdebc1a8e4a6820c1e4529705a82fce5d0`.

[Windows receipt](../verification/integration-r36/windows-build/windows-crossbuild-receipt.json).
The preservation audit retains all original file paths, verifies **316 authored
art/catalog/data files** and **160 prior verification files** byte-for-byte, and
preserves the original three top-level revision-35 receipts separately.
[Preservation receipt](../verification/integration-r36/preservation-audit.json).

## Explicit limits

No claim of exhaustive seeds, survival balance, interactive desktop acceptance,
or Windows launch/input/save verification is made. NPC possible-shot admission
has a 256-prefix pure search budget; an unresolved search or uncertain central
cone heading may be declined as an AI aiming choice. Actual emitted rays retain
fair committed physics. Autonomous gas/fire/liquid propagation is not transported
through pockets. See [rules and defaults](non-euclidean-r36.md).
