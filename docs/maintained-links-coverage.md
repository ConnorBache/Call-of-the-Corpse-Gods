# Selected-creature relationship cleanup

`maintained_links.rs` ports all five source tethers: Bonded By Unseen Threads, I Came For You, Make Slippery, Riptide and Unspeak. It removes the exact source active ID, target/timer keys and applicable target status, without payment, refund, RNG, log, damage or rescue.

120 frozen-source cases pass across direct clearing, actual-death event handling and migration normalization. Cases include living/dead/zero-HP/missing/off-floor/duplicate targets, inactive stale state and shared-body projection.

Normal current-run IDs are validated unique. The migration normalizer deliberately leaves ambiguous duplicate IDs untouched, as well as missing actors, living off-floor targets and zero HP awaiting rescue. Normalization of each historical state remains a separate persistence pipeline concern.

Hidden-floor simulation must explicitly forward the actual-death event to the real world after committing affected actor blocks. Passing only an absent-player proxy cannot clear a player's remote tether. Hidden logs or observations must never be forwarded as part of this event. Actual death is established before cleanup; successful rescue does not end a tether.

Complexity for N actors across all floors: expected O(N) time/storage, with exactly five links per distinct owner. Vector order preserves source actor order; hash collections are used only for membership and identity indexing, never random decisions or gameplay iteration. Changes detach only the touched actor blocks through Arc copy-on-write; worst-case copying is O(N) when every floor contains a changed owner. Clone synchronization adds O(A) for current-floor A actors. String keys have bounded authored length; depth-key allocation is included in the temporary location vector.
