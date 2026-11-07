# Half Here shared endpoint

`half_here.rs` holds stable shared-owner identity and endpoint lifecycle. The endpoint is not an independently acting ally. Owner resource and ability projections are serialized ordinary fields; local body conditions stay on the endpoint. No pointers or object links enter saves/history. The live clone is resolved only when the player owns and activates Half Here, the linked ID is alive on the current floor, and its owner matches the player.

Implemented helper scope: exact create/recreate, authored cyan-only color normalization, owner ID lookup, current origins/range, projection sync and cross-floor dismissal. Dismissal creates no death, corpse, heart or event. Projection copies owner stats/resources/ability order/equipment grants and clears endpoint adaptive DR; it does not overwrite body-local status.

`tests/half_here.rs` passes11 complete-state frozen-source lifecycle fixtures, including inactive/lost ability/dead/terminal/off-floor cleanup and custom-color preservation. Six derived-stat fixtures verify owner versus body-local buffs, equipment sharing and stale projection reads. Seven additional complete-state forced-floor/collapse endpoint cases pass, including shared damage before owner transfer, water, lethal owner and both-body collapse. Shared combat, environment, scheduler and activation integration are being verified separately; helper parity alone does not admit independent endpoint turns or duplicate resource pools. All seven complete command/history/event traces pass normal/haste clocks, endpoint targeting, flight expiry, stair dismissal, Revert and absent optional endpoint projection fields. The latter found a native inverse-import bug: missing patch values must preserve source deletion semantics rather than become explicit null. Strict whole-history equality is retained; no source-parity exception is needed. Activation and environment cases are tested by their owning suites.

## Complexity

Let A be total actors, Ac current-floor actors, D<=40 floors, P owner projection size, and K actor-map entries (<=D in admitted state).

- Owner lookup O(1). Current clone lookup O(Ac + player ability/active counts). Origins/range adds at most two endpoint checks; currently a second O(Ac) lookup materializes the clone reference.
- Sync O(Ac+P) time plus O(P) temporary owner clone, and COW detachment O(Ac*actor size) if the current actor block is shared. It does not iterate all bodies for resources or tick clocks.
- Dismiss O(A + D*K) worst-case time, O(K) changed-depth scratch, plus COW copies only for blocks containing matching endpoints. It preserves unrelated actor order and does not detach actor blocks without removals.
- Create adds dismissal, one O(actor projection size) construction, ordered insertion, and sync. ID allocation is exactly one increment; no RNG is consumed.

Combat/environment damage and temporal serialization are additive subsystem costs. No timing or platform-performance claim is implied.
