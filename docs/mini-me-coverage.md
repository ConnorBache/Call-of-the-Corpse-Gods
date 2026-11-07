# Mini-Me construction boundary

`mini_me.rs` supplies row-major adjacent placement, detached prospective clone construction, commit and all-floor dismissal. Activation payment, capability validation, toggling and action events remain caller-owned.

## Historical revisions, before 50

The clone preserves run-start actor fields and abilities, replaces identity/conditions/active state, rounds each innate stat and equipped base contribution independently, and copies authoritative hand1/hand2 melee weapon metadata. Bows do not become melee weapons. Enchantments and enhancements are excluded when base provenance exists; historical missing base provenance uses the authored bonus fallback. Maximum HP is exactly 100 + 6 Defense, as in source; no speculative extra HP bonus is added.

Complexity: eight candidate cells each scan N active-floor actors: O(N). Detached construction is O(A + I + S), where A is copied actor data, I inventory size and S stat fields (fixed schema). Commit is amortized O(1), or O(N) when copy-on-write detaches a shared actor block. Dismissal is O(W) across W world actors plus maintained-link cleanup; only blocks containing removals are detached; untouched floor actor blocks remain shared. No RNG draws occur.

Source fixture: 48 cases covering absent initialPlayer, signed fractional historical contributions, main/offhand order, bows, blocked neighbors and Phase Through. Full activation, copied-power combat and travel require separate execution tests; construction alone is not complete gameplay coverage.

## Revision 50 stats-only summon

Superseding designer instruction: “Mini Me doesn't get any abilities, just the
stats.” The earlier complete-copy implementation is no longer Mini-Me's behavior.

`stats_preview` builds a fresh Actor and snapshots the player's current effective
numeric Stats: Offense, Defense, Accuracy, Dodge, DR, Perception, maximum energy
and the numeric maximum-HP bonus. These values include current jewelry and numeric
buff/ability modifiers once. They remain frozen on the summon after the player
removes equipment or a modifier expires. This is a numeric snapshot, not a copied
buff duration or passive power. Current HP, maximum HP and current energy are
preserved; the base-HP contribution is calculated so later health synchronization
does not discard the snapshotted maximum. Values are captured before activation
payment, as at the existing preview/payment boundary.

These are already-derived values, so R50 independent actors with the existing
Mini-Me marker skip a second base-stat normalization in `effective_stats`.
Fractional Sustain Strength results therefore remain exact (7.5 Offense stays
7.5, rather than becoming 8). All eight numeric stat fields are preserved. No
new marker or save field is needed. Ordinary actors, the player and pre-R50
Mini-Me retain normal base-stat rounding. Newly inflicted conditions on the
summon still use the ordinary modifier rules, without inheriting Sustain or any
other live ring power.

No abilities, active powers, equipment grants, ability order, conditions, item
inventory, reserve map, blood, reactions, nightmares, target links or temporal
anchors are inherited. No recursion or activated-power control policy is needed.
Only the Mini-Me identity marker, creation-plane marker and Greatsword attack
metadata are initialized. Presentation uses the player's glyph/color and the
Mini-Me name. The summon remains a perfect ally using existing orders and AI.
Its attack baseline is 45 + 2 × its frozen effective Offense, with no offhand.

Placement checks use the fresh stats-only actor. In particular, the player's
Phase Through cannot cause a non-phasing summon to appear inside a wall. Pit and
chasm exclusion and stable row-major placement remain. Spawn commit consumes one
actor identity, not a batch of item identities. `actor_equipment::inventory`
returns no inventory for an R50 Mini-Me even if stale copied-item fields exist.
Ordinary future damage, regeneration and newly applied conditions follow existing
ally rules. Source player powers such as flight or haste are not inherited.

Legacy revisions retain their original level-one/base-equipment construction.
The R50 release gate remains independent from this ability's support admission.
Generalized independent-owner Duel and Nightmare APIs are not Mini-Me abilities;
their tests now use explicit synthetic actors rather than Mini-Me construction.

## Revision 50 complexity and verification

Let A be current-floor actors, W world actors, I carried/worn item count, E item
bonus/enhancement entries, B player ability/condition entries, and D a shared actor
block's bytes. Stat snapshot construction inherits the full effective-stat lookup
and derivation cost, including actor lookup, I/E traversal and B-dependent hooks;
it is not constant time. The produced Actor is fixed-size apart from presentation
strings and fixed identity/attack markers. There is no O(item bytes) inventory
copy. Placement derives one stats-only probe then checks eight cells, including
existing passability/portal and A-dependent collision costs. Commit uses the
existing creature identity registry admission and appends one actor; first-write
COW can copy D bytes. Dismissal retains its all-world actor/link cleanup costs.
No RNG is introduced. These are source-derived bounds, not measured timings.

New regressions cover effective numeric snapshotting, no powers/resources/items,
resource and HP preservation, equipment-removal independence, health sync,
serialization, one-ID commit, invalid power-bearing commit rejection and wall
placement when the player can phase. Prior ability-copy tests have been removed
or rewritten as generic independent-owner tests. The post-rewrite core unit suite
and real activation, wait, dismissal, payment and blocked-placement tests pass.
The historical 813-command Mini-Me trace also passes. See
`revision50-native-verification.md` for the final aggregate result.

The fractional-snapshot correction adds a fixed revision/identity check and one
fixed-key condition lookup per effective-stat derivation, expected O(1) time,
O(C) conservative hash-collision worst case for C conditions, and O(1) scratch.
It adds no scan, RNG, item or ability-copy logic. New real-activation regressions
cover fractional equality across all eight stats, later owner changes and clone
health synchronization, newly applied full-strength clone penalties, Snapshot
and raw-save restoration, plus ordinary/pre-R50 rounding. Those new cases await
the coordinating native run and do not retroactively widen the pass above.
