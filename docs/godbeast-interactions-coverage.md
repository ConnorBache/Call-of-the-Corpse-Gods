# Mercuria lever and Veridian tap

`godbeast_interactions.rs` implements activation and collection separately from input and inventory presentation. Lever activation happens at the source movement boundary before sticky-ground/hemophilia resolution; it does not move the player. Haste-only movement cannot activate it. The reward consumes the independent seed/depth Mercuria population stream, not gameplay RNG. Existing item placement scatters/falls/discards consistently and creates missing floors when required. Repeated activation cannot duplicate rewards.

Veridian checks exact Chebyshev adjacency, then both the run-wide harvest latch and every floor's tapped event. Failure keeps the tap and costs no action. Success marks the tree/run, creates one identified sap with next ID, and the inventory caller breaks the tap, inserts sap, emits the event and advances time. Sap consumption and blessings are verified in the potion suite.

Raw oracle:24 Veridian boundary cases and24 Mercuria initial-flood cases. Separate nine-scene72-command lever suite covers normal/haste/sticky ground/hemophilia/already-created reward/already-activated state and direct/distant/repeated interaction; full state, events and history are compared. Logs distinguish successful current suite from earlier fixture construction errors.

Complexity: first Mercuria activation allocates O(V) ichor cells; otherwise O(1). Reward generation is bounded catalog selection plus ground placement O(sum(V+I)) across at most40 floors and any needed procedural generation. Veridian scans F<=40 floors: O(F) time, O(1) additional state. Subsequent flood propagation remains the independently tested frontier in ichor.rs, not repeated full-floor searches.
