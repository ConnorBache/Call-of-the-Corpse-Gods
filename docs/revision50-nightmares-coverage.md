# Revision 50 Waking Nightmares

The plural ability creates independent serialized Actor entries with two condition
keys (`wakingNightmare`, `wakingNightmareOwner`). Existing nightmare resource and
deadline fields are reused. Hallucinations have no inherited powers, equipment,
shared health, resurrection, or ordinary death rewards. Their ally faction makes
them valid enemy targets; phase ordering excludes them from normal and haste AI.
Engine integration must also exclude upkeep/movement and dissolve them on a landed
strike before hit effects or death processing. A missed strike must leave them intact.
The singular legacy ability remains a separate revision-gated implementation.

## Independent owner API

`preflight` and `create` remain player wrappers. `preflight_for` and `create_for`
accept an explicit independent actor ID and target square. They validate ownership,
life, charges, target bounds/occupancy, actor-local visibility and ID admission
before mutation. Half Here/body projections and existing decoys are not independent
owners. An independent actor spends only its own Nightmare charge; the new decoy records
its owner's identity, presentation, faction and dream plane, with no copied powers
or inventory. The player wrapper retains the original ally faction behavior.

`tick_actor` recharges each independent living owner's serialized `nightmareAt`
deadline in the normal-turn scheduler. Half Here/body projections and decoys do
not receive extra charges. Copy and original can share the same initial deadline
without sharing their charge count or later deadline mutation. No haste-phase
recharge or ordinary energy regeneration is introduced. Explicit creation does
not select an AI target or automatically execute during an ally turn.

## Complexity

Let A be current-floor actors, W be world actors including mounted identities,
F be floors, H be retained identity reservations, d be decimal ID digits, V be
floor cells, P be observer origins, K be portal endpoints and R be maximum ray length.
Each activation checks A occupied positions, one visibility query, and identity
admission. Occupancy comparison uses portal canonical-cell lookup. Visibility
delegates to existing lighting/optical caches: cold lighting/geometry builds and
per-origin portal-route costs are additional, not constant (existing vision code
documents up to O(P*K²*R) route work). Registry admission is O(d) when clean and
O(W*d + F + H) when rebuilding. Actor insertion/removal can copy O(A) shared actor
data on temporal snapshot detach; removal is a stable O(A) scan. Created actors
have constant-size state plus player presentation strings. Total retained memory
grows with the number of live hallucinations; no invented gameplay cap is imposed.
Phase filtering adds O(A) marker lookups without additional asymptotic storage to
existing phase ordering (haste sorting remains O(A log A)). Recharge is O(B) in
owner ability count B plus actor lookup, once per independent owner per normal
turn, with O(1) serialized deadline state per owner. Mutating a shared actor block
can additionally detach its full actor data. Independent-owner activation adds
the observer-specific sensing scan documented in
[actor-local visibility](revision50-actor-visibility.md); its A-dependent optical
work is additional to occupancy and registry costs above.

These are source-derived bounds, not measured timings. Test file
`revision50_nightmares.rs` covers pure rejection, ID collisions, serialization,
multiple independent decoys, phase exclusion, resource debit, recharge boundaries,
and direct dissolution. Full combat/normal-turn integration requires engine tests.

Boundary audit additionally excludes hallucinations from stair queues/arrivals,
non-stair ally transfers, falls, current displacement, offscreen approximate
upkeep, terrain exposure, and biological status application. Current occupancy
still includes decoys; enemy targeting and projectile collision remain intact.
These guards are O(1) marker lookups per existing cohort member and introduce no
new asymptotic scan or allocation. Tests cover stair/fall immobility and fire/status
immunity without consuming RNG. Direct engine death handling must restore a
hallucination instead of entering ordinary death/rescue/reward processing.

Direct resolved breath footprints, ruin pulses, curse projectiles, physical grab
contact, and immediate explosion impacts now dissolve a decoy at their individual
dispatch boundaries. These use the existing range/plane/LOS/projectile admission;
no generic damage-source heuristic is used. Horn bolts, moon lasers, water jets,
and ordinary attacks already use the central landed-attack path, preserving miss
rolls. Beckon still spends its ordinary action but cannot relocate a hallucination.
Direct-impact hooks add one identity lookup and, on a decoy hit, the O(A) stable
removal above. Tests distinguish passive sourced fire from a resolved breath,
exercise ruin and curse impacts, verify curse payment/recovery, and retain a decoy
behind a blocking wall until an immediate blast reaches its square.

Latest source compiles with the installed Rust 1.99.1 toolchain. The core suite is
running; this update does not claim a passing result. New owner regressions cover
independent recharge/save-load state, no recharge for projections/dead/legacy
actors, player-reveal isolation, rejection before mutation and owner-only charge
debit. The superseding Mini-Me rule grants stats only, so these ability-owner tests now
use synthetic actors and do not imply Mini-Me inherits Nightmares. Rewritten
fixtures await native verification; see [Mini-Me coverage](mini-me-coverage.md).
