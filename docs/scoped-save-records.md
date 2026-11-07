# Scoped metadata save records (experimental)

The metadata-v2 build identifiers select an explicit version-2 ordered operations
batch. Older build identifiers still select the unchanged command-receipt grammar.
A new reader accepts both; an older reader rejects the new build identifier.
Mechanics revisions 39, 40 and 41 do not change.

Operations are command+stamp, inventory-display-order+stamp, or run-ID+stamp.
Metadata carries a final value, not a gameplay command: it changes neither RNG,
turn count, seed-replay ledger nor the replay-policy command boundary. Missing,
Null and ordered ID vectors are distinct. Run ID Null is rejected because the
existing run-identity domain does not admit it. Unknown fields and nonfinite IDs
are rejected; order membership and duplicate elimination are not invented here.

## Exactness and checkpoint projection

Legacy checkpoint import continues its existing inventory-order normalization.
A new metadata checkpoint atomically writes the checkpoint AND one initial order
operation restoring the exact live field. Later operations replay exactly, in
order, with no final-tail normalization. This makes Null, stale IDs and duplicates
observable exactly as they were live, including before a following Future capture.
A raw/plain checkpoint still has its prior normalization policy. This distinction
is intentional, not an assertion that those two representations preserve identical
UI metadata. No checkpoint is acknowledged before its complete atomic buffer is
written/synchronized. If the buffer cannot fit the 32 MiB trial cap, it is refused;
metadata is not dropped to squeeze it in.

A clean latest-41 metadata recording can supply one-use resume authority. Its
original immutable bytes and identity are rechecked once under the writer lock.
Known edits append from the loaded durable base; adoption does not acknowledge a
later live token. Unknown edits revoke append authority but retain a conflict
guard. Clones inherit only a shared immutable conflict guard, never append power.
Replacement/truncation therefore cannot turn a fallback into permission to
silently overwrite another recording. The existing cooperative-lock and bounded
per-append verification limitations remain unchanged.

## Complexity and bounds

Let I be order-vector length, D identity-string bytes, Q queued operation payload,
B recording bytes, and C command simulation cost. Setting order is O(I) time and
O(I) extra memory for exact bit comparison, selected capture and bounded JSON
preflight; it does not scan inventory membership or the world. Identity is O(D).
No-op equality costs O(I)/O(D) but changes no epoch. Dispatch is O(C + command
payload), independent of order metadata. The first-pass queue holds at most 32
operations; error/panic/overflow/unknown mutation still require a checkpoint.
Pending capture clones O(Q), including a compatibility command-only projection.
Encoding/strict parsing is O(Q); the 256 KiB batch cap bounds wire work. Order count has no new gameplay cap: admitted serialized bytes bound parser
allocation and metadata preflight. ID strings keep the existing 200 UTF-16-unit limit.

Checkpoint work remains O(serialized current state and required anchors), plus
O(I) for the initial order restoration record. Its extra frame consumes one of
65,536 record slots and counts toward 32 MiB. Resume load is O(B + replay costs),
first locked adoption O(B), and retains at most 32 MiB immutable bytes until
adoption/drop. Cloning its guard shares bytes via Arc and copies only paths.
Routine append remains O(new record bytes + bounded attachment checks), plus
filesystem sync latency; no old-prefix copy or full-state diff is introduced.

## Scope left unchanged

Unknown mutable escapes, failed/panicked commands, queue overflow, uncertain
writes and corrupt/torn files retain conservative fallback rules. This change
only makes the explicitly known UI order and run-ID edits recordable. It is not
an all-fields journal and does not silently relax durability or conflict checks.

## Remaining checkpoint triggers and their reachability

- **Initial/new destination or old-format migration:** a complete base state is
  required before independent playback. Latest clean metadata-41 reopen instead
  adopts its verified original cursor; it does not rewrite that base.
- **Unknown mutable access/admin edit:** no authoritative operation describes the
  change. A current-state checkpoint remains necessary under this recording model;
  a cheap token is not a world-change proof.
- **Dispatch error or panic:** ordinary gameplay refusals (insufficient energy,
  unavailable target, paralysis, cooldown, invalid selection) generally return a
  successful non-turn outcome and remain appendable. Actual errors include
  unsupported/invariant-broken state and invalid Future anchors. Some errors may
  publish prefixes, so this pass retains checkpoint fallback rather than guessing
  that an Err means no change.
- **More than32 pending operations:** this is an exceptional unsaved backlog or
  programmatic batch, not a32-turn periodic checkpoint. The event loop flushes
  before queued events and after the loop. A future bounded multi-frame queue
  could remove this fallback without changing the v2 format, but must refuse new
  commands before exhausting its byte budget, not discard already executed work.
- **Uncertain append or recovered torn suffix:** current recovery evidence keeps
  source identity, not the exact intended suffix/base-token transaction needed to
  authorize truncation or acknowledge an already-written record. A verified
  checkpoint avoids duplicate replay. Safe suffix retry is possible in principle,
  but requires retained exact-append evidence and its own fault/replacement gates;
  it is not implemented here.
- **External conflict or capacity:** these are persistent refusal/latch conditions,
  not permission for a hidden checkpoint rollover. Original bytes and pending
  state remain protected until explicit reload/new destination/export choice.

No queue, error-replay or recovery authority was weakened by the scoped metadata
optimization. New metadata v2 removes known inventory-order and run-ID triggers;
unknown edits are deliberately not assumed to be those same small changes.
