# Revision41 — Weave of Time (integration in progress)

The designer replaces Revert with a passive **Weave of Time**:

> You can freely travel along your time weave, which you leave behind as you move.

Confirmed ordinary direction: moving A→B leaves a teal directed weave at A pointing
at B. Moving B→A along that arrow backward is free when actual movement succeeds;
a bump attack is not free. Following the weave creates no new segment. A new
segment placed on an already occupied SOURCE cell removes the old segment there
and every older segment, retaining newer continuation. Merely entering an occupied
cell from another side does not by itself create a collision.

## Implemented ordinary scope

The integration covers deliberate adjacent waking/dream movement through the
existing movement kernel, including status-randomized movement, ice, sticky terrain
and doors. Only a possible reverse edge starts an isolated candidate movement probe.
Success commits its actual movement and arrival effects without advancing the
world/haste/resource turn. Failed, attacking or nonmatching probes discard candidate
state, RNG and presentation events before the normal charged path executes.
Following creates no new segment. The input command remains in the replay ledger.

**Not a completed release:** the new status/dream and transaction tests are pending
execution. Stairs, portals, teleports and currents/forced displacement remain explicit
policy gates; they do not create/follow trails through the ordinary command hook.
No old-save migration is inferred.

## Retired Revert work

Revision41 has no five-turn retained snapshots, closing snapshot, pending-haste
rewind snapshot, inverse discovery, sealing, or history export. Current checkpoint
contains current state (including present trail) and any explicit See The Future
anchor. Future still spends/restores its existing cumulative energy policy and
preserves live replay/run identity; it does not require completed-turn history.

The command candidate and phase-before rollback clones remain for error handling
and existing movement rules. They were not solely Revert architecture. Legacy
29–40 Revert code/catalog/replay paths remain isolated for old recordings; they
are bypassed by revision41 ordinary dispatch. New rules reject nonempty retired
history rather than silently discarding it.

See [timeweave-state.md](timeweave-state.md),
[timeweave-visual.md](timeweave-visual.md), and
[weave-revision41-catalog.md](weave-revision41-catalog.md) for representation,
visibility, compatibility and complexity. No performance conclusion is asserted
before the matched action/checkpoint/long-collision measurements.
