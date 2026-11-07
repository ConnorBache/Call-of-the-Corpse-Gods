# Equipment ability ownership

`equipment_abilities.rs` is a source-faithful ownership phase, not a replacement for complete resource/terrain settlement.192 frozen-source cases pass the pure projection, learned-only ownership and load-time reconciliation, including duplicate item grants, dormant acquisition order, Remember overlap, Nightmare recharge pause, reaction subtraction, Mini-Me cleanup, invalid grants and two-handed bow equip identity.

`reconcile(run, settle_environment)` updates ownership/counters and returns a crushing flag. It deliberately does not settle resource capacities, commit death, synchronize final bodies or cause falls. Equip adapters call with true, then perform those phases in source order; load normalization calls false and performs the appropriate final sync. `EquipmentAbilityState.active` is a pure forecast: source synchronization removes lost previous grants individually rather than blindly replacing unrelated active metadata.

No equipment reapplication refills previously acquired resources. Learning an equipped power promotes ownership without reinitializing counters. Source acquire markers and first-acquisition order are retained when equipment becomes dormant.

Costs: A ability/history entries, I inventory, C fixed72-entry catalog, N world actors. Projection is O(A + I×C) expected time/O(A) temporary membership/order storage. Reconciliation includes O(A²) vector membership and acquisition checks; each lost linked grant may scan O(N), and Mini-Me removal may detach changed actor blocks. Full resource/effective-stat costs and ordinary fall/travel costs are caller-owned and additional. Hash membership never determines gameplay iteration order; order comes from source vectors.

Full equip/use/drop/throw resource/flight/death and generated-ability execution admission require the separate equipment integration suites. Unknown or unimplemented granted effects remain explicit runtime gates until supported.
