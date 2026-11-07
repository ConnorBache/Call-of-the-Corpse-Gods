> 2026-11-06 correction: carried rings are automatically active and counted toward their independent category cap. "Worn" in historical notes below means carried-active; no manual equip or worn exemption remains. See revision50-carried-inventory.md.

# Revision 50 named rings: fixed-power checkpoint

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

The designer accepted nonstacking copies of the same named ring and these first
three powers (with later additions documented separately): Stealth multiplies negative wearer illumination by 1.2; Light adds
+10 illumination per worn copy in line of sight except on the wearer's tile
(the later explicit additive-Light rule supersedes nonstacking for Light); Regeneration
doubles natural HP regeneration from 0.2% to 0.4% of maximum HP per normal turn,
in combat and outside it, without regenerating energy. Different named powers
can coexist. The initial regeneration/duplicate proposal and the later Stealth
and Light corrections override the unspecified ring powers in the R50 request.

## Identity and nonstacking contract

`revision50_rings` recognizes only `kind: "ring"` with an existing serialized
`Item.effect` equal to an explicitly registered effect such as `stealth`, `light`,
or `regeneration`. Names are display
text, not identity. No display-name fallback, inferred prefix, new schema field,
production item-generation entry, quality multiplier or loot probability is
introduced. A nonempty `equipped` field activates an authored ring at revision
50 and later. Unequipped, unrelated-kind and unknown-effect items do not activate
these powers. Historical revisions retain their old behavior.

`has_effect(run, actor, effect)` is the shared public runtime query. It reads the
equipment owner's inventory, including player-owned Half Here/dream-body
endpoints, and otherwise the existing actor-local inventory. Stats-only Mini-Me
and Waking Nightmare decoys inherit no rings; dead actors supply no active
power. The boolean presence query is the nonstacking rule for powers other than Light:
current copies have identical fixed strength, so strongest-copy ties are equivalent.
Light instead uses `count_effect` to add +10 per distinct worn item. Item
quantity, order and legacy enchantment numbers do not scale these effects.

This query does not remove duplicate items, deactivate their independent Tinker
grants, or alter legacy stat bonuses/enhancements. Those older numeric bonuses
have no authored named-ring grouping or strength ranking. Defining their R50
replacement or duplicate handling remains a designer decision, not an implicit
selection of one entire item. The qualitative tier order, powers, rarity and
production loot distribution remain unauthored.

`effect(item)` and `description(item)` expose the fixed identity and pack text.
The caller applies the revision gate when presenting R50 descriptions. Runtime
light/stealth composition and lighting cache costs are documented with their
vision integration; the shared query introduces no light cache or render clock.

## Natural regeneration

`natural_hp_rate(run, actor)` returns 0.004 for an active Regeneration ring,
otherwise the existing 0.002. The normal-turn engine applies this before the
existing creature Regeneration multiplier and Drown water multiplier, retains
Reverse Entropy suppression, caps at maximum HP and skips shared-health
endpoints. Drown still checks either player Half Here endpoint for water. The
ring changes no player, blessing or item energy resource, and no action or haste
clock. A free/haste movement does not earn an extra regeneration tick.

Both exact and aggregate offscreen HP paths use the same rate and keep their
existing suppression and multiplier rules. Revision 50 shared endpoints cannot
receive an independent offscreen HP tick. The offscreen proxy does not borrow
the absent player's rings or heal the absent player. Living R50 creatures are
currently routed through exact offscreen simulation by the existing feeding
guard; the retained aggregate helper is tested directly rather than represented
as the default R50 runtime route. Existing monster energy-rest behavior is
unchanged and is not an energy effect of this ring.

No new persisted state is added. Item effect/wearing and actor-local inventory
are already carried by raw Run serialization, Snapshot and Future capture.
Snapshot tests verify ring identity and regeneration after restoration. Full
Engine50 save admission/replay and interactive time-travel acceptance remain
closed pending the overall R50 release gate. These tests do not lift that gate.

## Deterministic verification scope

`crates/corpse-core/tests/revision50_rings.rs` covers exact identity and wearing,
legacy gating, fixed-copy nonstacking, separate Tinker grants, normal combat and
haste boundaries, unchanged player/blessing energy, HP cap, creature/Drown
multipliers, Reverse Entropy suppression, Half Here single-pool ownership,
actor-local inventory and Mini-Me/decoy exclusions, raw serialization/Snapshot
restoration, and exact inactive catch-up without absent-player healing.

`offscreen::revision50_ring_tests` covers the retained aggregate branch, elapsed
tick arithmetic, old-revision arithmetic, water/creature multipliers, suppression
and shared endpoints. All fixtures are explicit test inputs, not authored
production ring distribution or ability-reserve capacities.

At this writing the new cases are authored and awaiting the coordinating native
test run. See the latest native-verification record for executed commands and
results. No platform playtest, campaign balance or performance certification is
claimed by this checkpoint.

## Complexity

Let I be the queried equipment owner's inventory entries, C the actor condition
entries, A the normal-turn actor count and E the elapsed offscreen turns. The
inventory slice lookup allocates nothing. The named-ring query scans at most I
entries and a fixed number of hashed condition lookups. Its expected time is
O(I), conservative collision-worst-case O(I+C), with O(1) additional memory.
Canonical effect strings have fixed maximum length; no item-name or grant-list
scan is performed. Dead/unsupported/legacy checks short-circuit. Unlimited worn
jewelry means I has no equipment-slot cap and must not be treated as constant.

The HP-rate query runs once per eligible actor on a normal active or exact
offscreen tick, adding O(sum I + sum C) worst-case lookup work to the existing
actor phase, or O(E × (sum I + sum C)) across E exact catch-up ticks. The
aggregate path queries once per living/revived actor batch, not once per elapsed
tick. Existing actor cloning, COW detachment, floor lookup, visibility, combat,
suppression scans and environment costs are unchanged and are not included in
these incremental bounds. This module adds no retained memory, RNG draws, global
floor traversal or per-frame work. Pack descriptions require one bounded effect
lookup; existing text layout costs remain the UI subsystem's responsibility.

These are source-derived bounds, not measured timings. Run focused integration
and offscreen unit tests, then the complete workspace suite. Profile unlimited
inventory fixtures together with the vision/cache integration before making
speed claims.
