# Revision 51 Ichor-fish fire immunity

Designer direction: Ichor fish, sharks and leviathans are immune to fire.
All lesser, standard and greater forms now ignore fire damage and cannot acquire
Burning. Normal fire, burning Ichor, permanent fire and Hellfire, Firebolt,
incineration potions and the fire-breath cone use the same revisioned identity
rule. The legacy Ichor Fish identity is also recognized. Ordinary mercurialized
creatures do not gain this immunity. Physical, cold, lightning, acid, poison,
bleeding and explosion damage retain their existing rules.

Protection is derived from run revision and actor identity, not an added save
marker or a deferred trait initialization. Revision 50 and older keep their
existing damage, ignition and RNG behavior. Current loaded or time-restored
actors are protected immediately; an inherited Burning status and its ownership
metadata are cleared on contact refresh or before the next normal environment
turn's snow/ice contact, fire emission and damage. Haste contacts use the same
refresh helper, while ongoing environment/status damage remains a normal-clock
operation. Active and offscreen precise simulation share this environment path.
No migration upgrades old runs and no new serialized fields are introduced.

Visible current creature descriptions include fire immunity. The current
Bestiary's nine entries say “Immune to fire and burning.” Owner-authored natural
history remains unchanged. The observation-only ability description does not
write new traits into actors or expose hidden actors.

## Complexity and bounds

Let L be actor identity bytes, C its condition count, A the current-floor actor
count, and W the bytes in a detached actor/floor COW payload. A fixed 10-identity
set plus exact legacy-name aliases is checked in O(L) worst-case time, O(1)
scratch, without a catalog scan, RNG or caching. Known kind takes precedence over
display name. The new check runs per admitted fire packet, ignition attempt and
existing fire-hazard query. It adds no map or pathfinding traversal.

An inherited Burning cleanup performs three ordered condition-map removals,
O(C) worst-case time, O(1) extra scratch. Normal-turn cleanup uses the already
collected living contact cohort; its additional work is O(A*(L+C+d)) with warm
actor lookup, where d <= 16 is the decimal identity-trie depth. A cold/invalid
registry adds O((N+P+B)*d+F+V) reconciliation and O((N+P)*d+F) transient storage:
N stored actors, P nested pilots, B previous indexed actors, F retained floor
records and V vacant-slot entries copied after a shared snapshot. Existing
retained high-water slot history costs O(H*d). First mutable COW access adds
O(W) copying. See [creature identity indexes](creature-identity-index.md) for
these unchanged helper contracts. They remain additive, not claimed constant. The existing native environment scans,
propagation, actor actions, AoE footprint/LOS and offscreen elapsed-turn work are
unchanged. Clean stationary offscreen floors cannot skip a living Burning
actor, because Burning already selects exact simulation. Dead corpse decay is
unchanged.

Visible description assembly adds one bounded trait string and duplicate-set
entry only for an already-observed current Ichor actor. Existing string output,
manual wrapping/font metrics, rasterization, visibility and catalog costs remain
unchanged; manual copy adds one fixed-length entry per species. No per-frame
world mutation or extra world scan is introduced. Bounds are source-derived,
not measured timings.

## Validation scope

`crates/corpse-core/tests/revision51_ichor_fire.rs` covers all nine forms across
fire packet entry points, unrelated damage, exact identity fallback, hazard and
Hellfire contact, existing Burning cleanup/no emission or ice melt, thrown and
drunk incineration, cone AoE, Firebolt, offscreen catch-up, save/load, Snapshot,
Future and fixture replay. Desktop coverage checks current and legacy observed
ability prose and all nine Bestiary entries. Execution results are recorded
separately; these are native deterministic cases, not platform gameplay claims.

The focused optimized core target passed all 9 tests with debug assertions and
overflow checks enabled (`verification/revision51-ichor-fire/core-test.log`).
Desktop prose and final-tree aggregate execution remain with the integration
owner. No Windows launch or platform gameplay verification was performed here.
