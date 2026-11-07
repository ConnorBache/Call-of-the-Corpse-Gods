# Revision 50 native verification

Final source host-side tests and Windows cross-build completed successfully.
New normal and debug games use R50. Existing saves retain their original revision;
start a new game to use the new rules. Windows execution remains unverified.

## Final verification

- Full workspace: **1975 passed, 0 failed, 38 ignored**, across
  188 completed target summaries. Counts are executions, not unique tests.
- Host: Linux x86_64, pinned official Rust 1.99.1, unchanged Cargo.lock.
- Test profile: optimization 1, debug info 0, debug assertions and overflow checks enabled,
  incremental compilation disabled, two build jobs and two test threads.
- Eight real R50 acceptance tests cover all 12 starter pools, generated Avatar/Blessed/
  Unblessed starts, command progression, save/load, replay/restart, generated debug and
  playtest floors 6/40, and an admitted Future/Timeweave/item/resource/terrain trace.
- Recording protocol/resume, acquisition, shared funding, Reaping, Cogworks, casters,
  unlimited trinkets, jewelry, potions and stats-only Mini-Me have executed regressions.
- Latest standalone desktop run: 576 passed, 0 failed, 32 ignored. The final full workspace
  includes the desktop tests again. Sixteen native screenshots were regenerated and
  reviewed for item hover, energy/costs, statuses, scrolling and cancel restoration.
- Static catalog/source contracts and 24 optional snapshot-root checks pass.
- Nine packaging self-tests pass. Final Windows x64 release cross-build succeeds.
  PE inspection confirms only SDL2 plus standard Windows DLLs. Stripping preserves
  mapped code/data/resources and loader semantics; embedded encoded art matches its manifests.

## Evidence

- Complete final command, source digest, log hash and counts: `../verification/revision50/windows-release/test-report.json`.
- Full final log: `../verification/revision50/windows-release/workspace.log`.
- Exact build command, pinned environment and binary hash: `../verification/revision50/windows-release/build-report.json`.
- Frozen build/test inputs: `../verification/revision50/windows-release/source-snapshot.json`.
- Pixel review: `../verification/revision50/jewelry-ui-final/REPORT-2026-11-06.md`.
- Windows packaging: `revision50-windows-packaging.md`.

## Limits and explicit implementation defaults

This is an unsigned cross-build. It has not been launched on Windows; Windows
input, rendering, audio, fullscreen and save behavior require platform testing.
Headless native pixels and host-side tests do not establish those behaviors.
Ignored benchmark/opt-in tests were not run. Full campaigns and performance
certification are not claimed. The earlier 896-encounter bare-player diagnostic
predates final jewelry and loadout rules; enemies were not arbitrarily retuned.

Repulsion's exact-center case uses a provisional random valid outward direction,
then ordinary forced movement for up to 3 tiles. The direction uses serialized RNG.
Cogworks uses one 200-HP wall pool. A blocked hostile route can attack the wall with
ordinary melee damage, automatically hitting its zero-DR surface without creature
hit procs; destroying or Shattering one piece removes the entire shared wall.
Obstruction/Cogworks retain their existing cardinal/exact-diagonal grid orientation.

## Resolved integration failures

Earlier failed runs remain in the verification directory as development history.
Corrections include legacy FightLong floating-point order, explicit new caster tags
in parity expectations, valid per-floor save fixture blocks, reserve acquisition in
low-level tests, split-funding expectations, dream-only shard pickup fixtures, and
updated wall/hover UI expectations. The empty-slot negative-zero display was fixed.
Final counts above supersede older stats-only and intermediate jewellery reports.
