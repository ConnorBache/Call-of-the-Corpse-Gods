# Revision 52 exact-zero health

Living fractional HP remains exact simulation state. Whole-number current Health
labels now round upward, matching the existing creature hover convention. The
player HUD, enemy cards, creature headings and inspectors never show positive HP
as zero. Bars still use the actual fraction. Damage rounding, energy formatting,
maximum health, healing and regeneration rates are unchanged.

The ordinary melee/direct/environment damage kernels already resolve death at
HP <= 0, including exact equality. R52 now also settles unresolved nonpositive
living actors at the command boundary and before offscreen actions or natural
regeneration. Offscreen floors with unresolved deaths use the precise lifecycle
instead of clean-floor bulk regeneration. Enemy maximum-health synchronization
also resolves any newly nonpositive health immediately. Ordinary regeneration
cannot revive a nonpositive actor. Earlier simulation revisions retain their
historical boundary and regeneration behavior.

All settlement uses the existing ordered death/rescue lifecycle. Hanged continues
to spend main energy at four energy per HP after mitigation, preserve fractional
positive survivors (including 0.25 HP), and allow lethal uncovered remainder.
Rage, Overflowing, Pelian and actual corpse revival are not replaced. Linked-body,
invulnerable and disappearing-body rules remain in the same lifecycle. No new HP
threshold, damage packet, RNG draw, saved field or revival mechanic is introduced.
Snapshots remain faithful; command processing after restore settles stale zero
rather than rewriting the snapshot or silently regenerating it.

## Complexity

Let A be actors on the affected floor and Z unresolved nonpositive actors. The
R52 command/precise-offscreen boundary adds an O(A) scan and O(Z) transient IDs;
the usual case allocates no ID storage when Z is zero. Each candidate is rechecked
because preceding death hooks may remove/rescue linked actors. Existing indexed
actor lookup, copy-on-write detachment, visibility, rescue, loot, corpse and death
hooks are additive, not O(1). Their worst-case costs remain as documented in the
actor/combat and offscreen subsystems. Dynamic-floor admission adds O(A) worst-case
scan and O(1) memory, and routes a stale zero floor through existing precise turns
until death is resolved. HP label conversion is O(1) time/memory per label; existing
font/layout/raster costs and bar drawing are unchanged. No cache or persistent
state is added. These are source estimates, not measured performance claims.

## Verification

Deterministic regressions in `revision52_zero_health.rs` cover player/enemy exact
zero and overkill, no duplicate death, normal/haste stale zero, positive fractional
survival, Hanged's 0.25 HP residue, ordered rescue, corpse revival, enemy health
synchronization, offscreen settlement and save/snapshot continuation. Desktop
regressions exercise whole-HP conversion, actual enemy card labels/exact bars and
player HUD pixel differences. Existing Hanged, damage-rounding and lifecycle suites
remain relevant. Executed outcomes are recorded separately; source/test presence
is not a claim of a full workspace or platform pass.

### Executed focused results

- `tools/cargo.sh test -p corpse-core --test revision52_zero_health --test revision52_hanged --offline`: Hanged 21/21 passed. The first new zero-health run was 6/7 because its offscreen fixture accidentally duplicated floor 2; the fixture was corrected, not the behavior.
- `tools/cargo.sh test -p corpse-core --test revision52_zero_health --test revision51_damage_rounding --offline`: final zero-health 8/8 and damage-rounding 13/13 passed.
- `tools/cargo.sh test -p corpse-desktop --bin corpse-desktop zero_health --offline`: all 3 focused desktop tests passed, including actual player-HUD pixels. An initial `--lib` attempt found no library target and ran no tests.

All commands used the shared `corpse-center-target` after an exclusive build
lease. Logs are under `verification/revision52-zero-health/`. This is focused
native evidence, not a full-workspace, Windows build or interactive-platform pass.
