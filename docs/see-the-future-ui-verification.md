# See the Future UI update

- Keyboard F and clicking See the Future both ask `Spend all your energy to see the future?` before capturing the snapshot.
- Activating again with a saved snapshot asks `Return to your saved state and undo everything since?`.
- Y dispatches exactly once. N or Escape dismisses without spending energy, advancing time, or altering the snapshot; other keys leave the question open.
- The ability's state is an outlined box, with a purple spiral when a snapshot is present. Generic numeric reserve text is not its state indicator.
- Temporal save data, replay commands, and the capture/return mechanics remain unchanged. The existing confirmation sheet uses Y/N, with Escape also cancelling.

Validation results are recorded with the candidate build. Native test coverage is not a Windows runtime verification.

## Missing reserve admission repair
The current acquisition path creates the reserve. An older/debug checkpoint can nevertheless contain the owned ability without that bookkeeping entry, and capture checks payment before equipment reconciliation. Reproduced this shape and added a revision-52+ load/admission/restore repair: create the absent Future entry at 0/10, never grant energy, never replace a valid entry, and continue rejecting malformed entries. No failing user save was supplied, so this is a reproduced error path rather than proof of how their original file acquired the omission.
