# Revision 50 combat hooks

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

Revision 50 gates preserve older-save behavior. New deterministic helpers cover
Execute's strict quarter-health comparison and 20-energy landed-hit payment,
explicit Magickal Caster Witch-Hatred, Unstoppable Force advance/push/swap,
Commandment passive/paid maximum push, source-aware Oblivious immunity and
Missed Me retreat with no reaction payment when trapped. Masochism refreshes ten total turns of haste when one or more HP thresholds
are crossed, per the designer clarification. Healing rearms thresholds without
a saved latch. A longer existing haste effect is not shortened. The shared HP
loss hook covers melee/ranged attacks, sourced and environmental damage, bleed
and HP-cost damage. Derived maximum health supplies the fraction denominator.
Normal and haste phases use the same hook and existing hastedFresh clock rules.

The Magickal Caster predicate deliberately requires the explicit trait
`enemy:beast:magickal-caster`. The existing dedicated-caster AI predicate is not
an authored classification and is not silently substituted. R50 births explicitly
tag Witch, Hemogoblin Bloodmancer, Dryad, all seven horn kinds (Unicorn through
Heptacorn), Saidribee Queen and Valta. Frozen catalogs and legacy constructors
remain unchanged. Nest Queens receive the trait through a versioned pocket-birth
wrapper while preserving the original numerical profile. Witch/Queen summons
preserve their existing newborn profiles and do not inherit the summoner's tag;
Night Crawlers, Skin Walkers, workers and Guardians remain unclassified.

Attack integration must invoke lethal-hit selection after landing, and attack
movement once after reactions on both hit and miss. The terminal hit uses the
existing terminal-strike path. Source immunity must gate non-damage enemy effects
as well as attacks and sourced damage. Anonymous environmental damage is not an
enemy source. Visibility uses the established perception rules, including
telepathy. Commandment push and Missed Me use the existing forced-movement pipeline.
Force stages both actors before shared arrival callbacks, settling the non-player
first so player floor travel happens last. The shared arrival helper handles
terrain contact, burning, doors, corpse stomping, Black Pit, Grappler, stairs and
falls. Nonflying enemies cannot be pushed into a chasm, matching existing forced
movement. Actual position/depth change, rather than an action-consumed boolean,
controls retreat reaction payment. Temporary forcedMove values are restored.

Old Beat to Death stat debuffs and landed-hit accumulation, I Hit Back counters,
old Waking Nightmare absorption and Dream Craft energy bonuses are disabled at
revision 50. New identities are not aliases for old mechanics.

## Complexity

Let A be current-floor actors, K actor ability/condition entries, V floor cells,
P portal aliases/exits, L light sources, and D the portal route graph size.
Execute is O(A + K) for endpoint lookup/payment with O(K) worst-case COW mutation.
Witch trait matching is O(K) after endpoint resolution. Masochism crossing count
is O(1), no allocation. Its runtime hook also derives effective maximum health,
including existing inventory/enhancement/stat and shared-owner lookup costs; it
is not an O(1) operation overall. It performs one haste update per HP packet and
adds no per-frame work or persistent threshold cache. Oblivious adds the existing visibility/lighting/detection
cost: the full cost is not O(A), and cold lighting/route projections retain their
existing subsystem bounds. No new visibility cache is introduced.

A one-tile push or swap scans actor occupancy and portal aliases: O(A*P), plus
existing same-plane and portal graph queries and arrival callbacks. Arrival can
detach O(V) terrain or O(A) actor COW backing and write terrain effects. Maximum
push stops after V steps or a repeated cell, using O(V) scratch and conservatively
O(V*(A*P + V + K + D^2)) time including cold route/arrival work. Missed Me checks
eight candidates and all portal outcomes; distance routing and collision costs
are paid for each candidate, not treated as constant. The portal graph may have
multiple exits; actual committed branching uses serialized gameplay RNG.

Helpers execute once per attack/packet/retreat; no per-frame work or renderer
changes. Maximum push is bounded by floor cell count, including portal cycles.
No measured timing claims. Profile representative ordinary and portal floors,
with cold and warm lighting data, before optimizing these estimates.

## Verification

`crates/corpse-core/tests/revision50_combat.rs` covers strict Execute thresholds,
energy shortage, revision gating, explicit caster matching, Force push/swap,
Commandment distance and payment, unseen-source immunity, trapped retreat,
threshold retriggers, removed legacy effects, nonflying chasm rejection, forced
flag restoration, ranged Force movement and corpse stomping. These are native test sources,
not executed tests: the pinned Rust compiler was unavailable during this work.
Run `tools/cargo.sh test --workspace` when the approved toolchain is available.
Portal/hazard, off-screen, save/restore and complete engine interaction parity
remain integration verification requirements; helper tests alone are not proof
of full system correctness.

Masochism regression sources additionally cover simultaneous crossings granting
ten total turns, healing retriggers, a stale stored maximum versus the derived
maximum, preservation of longer haste, normal/haste phase freshness, legacy
revision and ability ownership gates, and common damage-packet integration.
These tests remain unexecuted pending the approved native toolchain.

## Ability-reserve payment

Execute, active Commandment, Never Surrender admission/payment and paid Rage
rescues now use the shared ability-reserve API. Zero-cost first Rage rescue still
needs no fabricated reserve. Positive costs with no authored reserve cannot be
paid, even when shared player energy is sufficient. Exhausted explicitly present
reserves can fall back to shared energy under the central payment rules; partial
funding behavior remains owned by that API. Legacy revisions retain shared-energy
payment. Divine environment/combat modules contain no other direct paid triggers.

The revision50_combat test fixture explicitly declares exhausted Execute and
Commandment reserves, preserving its existing shared-fallback assertions without
inventing production capacities. Four added regression tests cover own-reserve
Execute/Commandment, matching Surrender admission/payment, missing reserve denial,
Rage's free first rescue, paid reserve rescue and legacy shared payment.

Birth classification is O(B) ability inspection for a fixed twelve-kind match,
with at most one appended trait and no RNG. Existing constructor/population costs
are unchanged; no per-turn name or AI-predicate inference is introduced. Added
regression sources check every approved positive, all other catalog negatives,
legacy/profile preservation, ordinary births, nest birth, real summons and
Witch-Hatred telepathy/terminal strikes. They await integrated Cargo validation.
