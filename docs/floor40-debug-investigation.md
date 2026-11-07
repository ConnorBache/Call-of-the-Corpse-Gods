# Floor 40 DEBUG fire report: investigation

The report describes an “Unsupported native behavior” notice after entering floor 40 from a DEBUG start, with fire visible. The exact suffix, seed, selected powers, loadout, and command were not available. This notice prefixes several distinct command-admission and runtime failures; its prefix alone does not identify a fire defect.

## Verified so far

- Six generated Unblessed DEBUG starts, seeds `floor40-fire-review-0` through `-5`, entered floor 40 and accepted up to 20 wait commands without an unsupported-behavior error. Three contain generated Hellfire. One run died normally before the final two no-op commands.
- Twenty generated Avatar DEBUG starts, seeds `floor40-fire-review-0` through `-19`, entered floor 40 and accepted up to 30 wait commands without that error. Ten contain generated Hellfire. One run ended in ordinary death on turn 29.
- Five additional **artificial stress states**, based on generated Unblessed starts, placed fire across the entire floor and Hellfire on all non-wall/non-chasm cells. Up to 40 waits per state produced no unsupported-behavior error; one state ended in ordinary death on turn 21. These are deliberately modified stress fixtures, not evidence that normal generation blankets a floor in Hellfire.
- `tests/debug_floor40_fire_admission.rs` uses actual DEBUG reconstruction, the production temporal Engine wrapper, normal entry, and four wait turns for both Unblessed and Avatar. Both modes use a fixed generated Hellfire seed. Saving and loading before entry and after every turn preserves full canonical state against uninterrupted execution. Initial direct-linked result before the new fungal-root integration: 1 passed, 0 failed, 37.90 seconds. A later integration run exposed the separate save/resume defect below. After the correction, the current direct-linked regression passes 2 tests, 0 failures in 38.25 seconds, including the revision-50 checkpoint. Evidence: `verification/floor40-debug-investigation/postpatch-regression.log`; compiled-core and edited-source hashes: `postpatch-sha256.txt`. Release-wide Cargo/platform verification remains separate.

The diagnostic probes temporarily inflate player HP for survival; the committed save/resume regression does **not** modify player health, actors, or terrain. Its level-up selections are deterministic test policy only.

## Separate DEBUG save/resume defect found during integration

The later integrated root lifecycle cleanup exposed a deterministic checkpoint
divergence. DEBUG reconstruction directly removed living fungi on prior floors,
but left their active root links/material. Loading the saved run normalized those
links, marked the hidden floors dirty and consequently simulated extra hidden
floors on the next Wait. The uninterrupted run left those floors clean. The first
Wait advanced normally in both paths but canonical state and RNG diverged. This
was not an unsupported-command failure.

`debug_floor::create_run_revision` now invokes the existing fungal-world
normalizer immediately after its prior-floor removals. The normalizer already
returns without changes for revisions before 51. Fresh and loaded DEBUG runs
therefore agree on material, inactive pair history and dirty-floor scheduling
before entry. The prepatch core fails the new stale-root assertion; the patched core passes. The regression checks equality at the initial checkpoint and after
each turn, and separately checks revision-50 root-state compatibility. No
user saves are edited; the change applies during explicit new DEBUG creation.

## Result and boundaries

No runtime correction is claimed for the reported issue. Fire, Hellfire and DEBUG entry are not rejected categorically. Admission guards remain intact rather than being weakened speculatively. The raw R shortcut is already suppressed in production for revisions without Revert; the old V diagnostic ability is ignored as unowned in current R50/R51 activation. Neither establishes this report's cause.

The exact error suffix or screenshot, and preferably the seed and chosen loadout/powers, are needed to match the original failing state. No user save or user computer was accessed or changed. This is cloud-native command testing, not Windows runtime verification.

Evidence and standalone diagnostic probe sources: `verification/floor40-debug-investigation/`.

## Test complexity

For S seeds, G generated floors (bounded at 40), T commands and B serialized run bytes, the probe costs `O(S * (generation(G) + DEBUG reconstruction + T * dispatch))`; dispatch includes existing actor AI, pathfinding, visibility and environment propagation costs. It is not a constant-time turn claim. The save/resume regression adds `O(T * B)` serialization/parse/comparison work and `O(B)` transient values plus two retained Engines. The setup-time cleanup adds the fungal normalizer's existing per-floor actor/link/material traversal and COW costs once per DEBUG creation; see `docs/complexity-ledger.md`. No runtime instrumentation is added.
