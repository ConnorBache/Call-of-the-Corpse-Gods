# Revision 39 verification

## Result

**951 passed, 0 failed, 1 existing ignored across 154 workspace targets.**
`cargo test --workspace --locked --all-targets --no-fail-fast` completed in
910.280 seconds. The separate documentation-test invocation passed with zero
doctests. There were no filtered tests in either final invocation.

All 1,077 frozen code/build inputs remained unchanged before/after both the
aggregate test gate and Windows cross-build. Fingerprint:
`3519b868496999a7d6f73d74bb5ee225f974b2540646135d81ceb568314c0f73`.

## Implemented and exercised

- Normal equipped main/offhand Shard attacks and normal accuracy; no Apply bypass.
- Only confirmed hits consume the weapon and permanently kill the resolved target.
- Melee misses retain the weapon; thrown misses remain recoverable on the same
  floor, including Dreamwalking, without affecting overlapping waking actors.
- Deflection, counterstrikes, Nova, Erase Metaphysical, portals and non-Euclidean
  projectile routing; permanent death retains ordinary death/finale bookkeeping.
- Ahvamitt remains a noncombat interaction, not a false successful weapon hit.
- Stone sentence occurs only in the ordinary player kill chronicle; no Nightmare
  dissolution, miss, non-player death or unrelated kill receives the sentence.
- Dream ground ownership survives save/load and temporal snapshots, is cleared on
  pickup, and is hidden from waking live/remembered/detected display and inspection.
  Waking drift, floor collapse, placement occupancy and Zillennia effects do not
  acquire or erase a dream-owned Shard. No new general dream-loot feature was added.
- New/default/DEBUG revision 39 admission and explicit historical revision rules.
  Four fixed seed worlds (160 floors) matched revision 38 exactly after normalizing
  only the revision field; generation/art/balance are not changed by this release.

The focused 27 core, 17 independent combat/replay, 5 desktop and 4 lead tests are
included in the final aggregate count, not added to it a second time. Initial
review logs retain found-and-fixed defects and fixture corrections for provenance;
the final frozen gate supersedes interim test receipts.

## Windows and assets

Rust 1.99.1, official Debian Trixie GCC14/MinGW-w64 and official SDL2 2.32.10
produced a successful x86-64 Windows cross-build. Static checks verified PE
architecture/imports, all 84 embedded RGBA plates, 7 icon images and the application
manifest. **The executable is unsigned and was not launched on Windows.** These
checks do not establish Windows rendering, input, launch or save behavior.
Original source art, runtime art, catalog data and historical verification remain
byte-for-byte preserved. Current source packaging separately checks old evidence
and retains the prior three top-level receipts under the historical folder.

## Authorship

The authored Borge/stone replacements are applied. Batch 2 approvals 12, 13, 18,
19 and 20 remain unchanged. Items 15 and 17 remain unanswered/unapproved; the
explanation of 17 is not approval. The retained disintegration sentence and new
user stone sentence have mixed provenance. The historical <=38 Apply guard text
has not been globally deleted. See the exact [copy receipt](../verification/integration-r39/player-text-changes.json).

## Evidence and limits

- [Aggregate gate](../verification/integration-r39/final-gate/final-gate-receipt.json)
- [Frozen inputs](../verification/integration-r39/final-gate/code-freeze.json)
- [Independent focused review](../verification/integration-r39/independent-review/focused-receipt.json)
- [Desktop focused review](../verification/integration-r39/desktop-ui/receipt.json)
- [Windows cross-build](../verification/integration-r39/windows-build/windows-crossbuild-receipt.json)
- [Toolchain restoration](../verification/integration-r39/toolchain/receipt.json)

These controlled regressions are not a new natural-world balance study or a
Windows runtime test. Historical r38 broad generation stress receipts are
preserved evidence, not claims that those full stress samples were rerun here.
