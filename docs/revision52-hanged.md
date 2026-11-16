# Revision 52 Hanged

Hanged replaces Missed Me in the same serialized blessing slot,
`isheilah-missed-me`. It is permanently passive and gives no maximum reaction,
reaction charge, retreat or activation command. Revisions below 52 retain their
original Missed Me metadata, acquisition and combat behavior.

After ordinary damage has passed immunity, mitigation, resistance and existing
whole-packet rounding, HP pays only the portion above 1 HP. Further damage,
including overflow from the same hit, consumes the owner's main energy at four
energy per HP. Once that energy is depleted, uncovered damage reduces actual HP
and can kill. Existing fractional HP and energy are retained with no additional
rounding. For example, at 1 HP with 1 energy, one damage leaves 0.25 HP and zero
energy. A body already below 1 HP is protected at its actual HP, never healed.

Blessing and amulet reserves do not fund this conversion. It has no activation,
upkeep or reaction cost and does not add to cumulative time-travel costs. A fully
absorbed packet still lands: existing packet-based on-hit, Bonded and Overflowing
rules remain. Actual HP-loss statistics, blood deposition, Masochism, Reaping and
HP-based lifesteal see only HP that was actually removed, before any death rescue.

Explicit Blood Weapon/Blood Letting health costs and equipment/max-health changes
are not incoming damage and are not converted. Execute/Witch-Hatred terminal
strikes, the Borge shard, phase crushing and flesh-collapse deaths retain their
unpreventable bypass. When uncovered damage is lethal, the existing ordered
Rage/Overflowing/Pelian rescue chain sees the already-depleted energy bar.

Half Here and the sleeping Dreamwalk body use the canonical owner's HP and
energy exactly once per packet. New R50+ Mini-Me copies remain stats-only and do
not inherit Hanged. The normal/haste scheduler and offscreen packet adapters share
the same damage kernel; no independent shield pool or timer is added.

## Persistence and compatibility

No saved field or ability ID is added or renamed. Structural raw deserialization
remains faithful: it may retain a historical R52 active-list entry or reaction
count. The supported compatibility loader normalizes each world, initial player,
nested Future and historical/pending state, removing the obsolete active entry and
capping existing reactions to the revised maximum without refilling. Engine::new
also normalizes its current world. Runtime Hanged eligibility never consults the
active list, and maximum-reaction helpers exclude the slot at R52 even on a raw
state. Snapshot and native Future copies preserve their existing exact-state
contract; saved anchors pass through compatibility normalization on loading.

The new catalog is a revision-specific overlay, not an edit to frozen v48/v50
catalogs. It also describes the separately implemented R52 Wait rule for Quick
Hand and Are You Sure About That: waiting restores one reaction up to the maximum.
Raw pre-update R52 command recordings replay under the revised R52 mechanics;
pre-R52 recordings retain their prior semantics. This follows the authorized R52
boundary, not a new claim of byte parity for pre-update R52 recordings.

## Verification scope

Deterministic native regressions exercise actual melee/ranged/direct/status and
environmental packet entry points, same-hit overflow, exact and insufficient
funding, fractional resources, mitigation/rounding, repeated processing,
HP-cost/terminal exclusions, rescue fallback, packet-vs-HP counters, shared
bodies, Mini-Me, offer/equipment/starter acquisition, save normalization,
Snapshot/Future and normal/haste save-resume. Desktop tests check passive row and
binding, reaction cap and non-mutating HP/energy previews.

See the accompanying isolated test logs for executed outcomes. The recovered
baseline has seven exact-source gaps. A focused pass does not certify the full
candidate or platform build; the desktop source has a known missing
stat_presentation::enemy_damage_revision symbol. A separately marked test harness
may fill that unrelated symbol only for focused UI tests, never as release code.

## Complexity

Let A be current-floor actors, K the owner's ability entries and S the existing
resource-history/visibility/stat/synchronization work. The split arithmetic is
O(1) time/space. Eligibility and owner resolution add O(A+K); actual packet
application retains O(S) and actor COW detach costs. No new RNG, per-frame work,
persistent state or independent reserve is introduced. Migration is O(total actor
metadata + K per actor), including nested pilots/initial player, plus existing
COW copy costs when normalizing old states. It runs at loading/new-engine
boundaries, not on every ordinary turn. First R52 catalog use clones the fixed
catalog and text once; later access is O(1). Preview adds O(1) arithmetic after
existing actor/stat derivation. These are source estimates, not measured timings.
