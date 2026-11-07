# Full-snapshot Revert trial (not a released build)

This isolated branch starts from published source v19. It does not use the rejected unified mutation journal or native Format2. Stable source/Windows/code artifacts are unchanged.

## Semantics

Before any pending-haste restore, capture live cumulative `timeCosts` and add 2 energy / 1 charge. Restore the selected completed opening via `Snapshot::restore`. Preserve live runId, replay (including the activation command), and the compatibility history buffer. Runtime history remains separate and pops exactly one completed opening. Restore all other snapshot state: world, RNG, player, inventory, nextId, metrics, achievements, Future, resourceHistory, visible log and logKinds. Truncate the live archive to the snapshot cursor, or remove it if absent. Clear haste.

Apply existing `retain_costs` after restoration, while the restored snapshot's original ledger remains present. This deducts only the positive difference between desired cumulative costs and restored ledger, clamps energy/charges at zero, records actual energy lost in restored telemetry, then installs the live ledger. Never overlay live energy, charge counts, or all resourceHistory: that would defeat rewind or double-count costs. Ordinary outer dispatch still emits its activation log/event and settles equipment/powers/doors.

Intentional change: later changes to fields that did not change during the selected old turn are now overwritten too. The former selective inverse semantics are not the oracle for this new behavior. Old command tails must still replay under their original policy before migration; compatibility handling is a release gate.

## Cost model

Let S be serialized world bytes, H retained openings (at most5), M owned non-floor/header bytes, R shared roots, D bytes freed when an evicted value loses its final owner, and B emitted checkpoint bytes. Snapshot capture/restore costs O(M+R+D), not O(1); large immutable floor/actor/ground blocks remain shared. Ordinary commands do not discover or encode inverse patches. Their existing simulation, snapshot captures, and metadata bookkeeping still run.

Checkpoint export uses the existing Frame/Patch JSON schema with whole root-field replacement/removal patches rather than a recursive old/new diff. Cold export is O(H*(S+R)+B), with O(S+B) scratch/output; strings and JSON values still allocate. Warm cache checks are bounded by H but copying encoded history remains O(B). Full snapshots may materially increase checkpoint bytes and latency; no improvement is claimed until both ordinary and checkpoint distributions are measured. Revert uses whole-snapshot restore plus ledger/telemetry work, not the old recursive diff/rebase.

The command-log framing, atomic write/fsync/locking cadence and 32MiB cap are not relaxed. All timing results must include checkpoint spikes, and all five Reverts plus save/load/Future/haste must be tested before release.
