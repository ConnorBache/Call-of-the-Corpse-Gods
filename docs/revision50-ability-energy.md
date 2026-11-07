# Revision 50 ability reserves and Future payment

The approved resource rule is explicit: each acquired blessing starts with 50
energy. Pay the blessing's remaining energy first, then any remainder from
player energy. Insufficient combined energy rejects without spending either
pool. Successful level-up restores every acquired blessing reserve to 50,
including dormant grants, but never restores player or amulet energy.

## Payment and acquisition

`revision50_ability_energy.rs` stores `AbilityReserve { capacity, energy }` by
canonical ability ID in `Actor.abilityReserves`. `plan_payment` admits the full
split before any mutation. `commit_payment` rechecks both original pools before
writing either. Missing or malformed reserves, invalid costs, unrepresentable
subtractions and stale plans reject. The old partial-funding guard is removed.
Zero-cost effects still need no reserve payment. Pre-50 calls retain their
original shared-player-energy semantics and do not acquire new fields.

`acquire` installs authored 50/50 state once and preserves any valid existing
charge, including a dormant or exhausted reserve. Acquisition IDs must belong to
the R50 catalog or supported Remember-copy traits. `initialize_reserve` remains
an explicit low-level constructor that rejects replacement, also used by
fixtures with nonproduction capacities. Level-up normalizes all stored reserves
to 50/50 after validating every entry before experience, level or RNG mutation.

Real callers now include starter-shell construction, `choose_offer`, equipment
ownership reconciliation/promotion, underfoot pickup, Remember and Tinker.
The equipment preflight checks primary and additional grant IDs against the
current R50 catalog before changing a target slot or reconciling the pack.
Retired/unknown IDs reject unchanged, including invalid unequipped peer items.
Legacy and ungranted special-item behavior remains unchanged.
Carried item grants acquire their reserves even while unworn. Equipping makes
the grant available, not newly charged. Unequipping, expiration, dropping,
promotion to learned ownership, duplicate grants and later reacquisition keep
the same ID's charge. Reconciliation may discover a newly acquired grant, but
never refills an existing one. Pickup also materializes an amulet's own item
energy once; its separate reserve is not a blessing reserve.

`can_pay` and `spend` resolve Half Here and Dreamwalking bodies to the canonical
owner. Copied reserve maps cannot become independent funds. Only actual player
energy loss is recorded in player resource history; reserve-only payment is not
reported as player loss. Mini-Me retains the independently implemented stats-only
rule and inherits no abilities, equipment or reserve map.

## See The Future

R50 Future has an explicit special plan: drain its own reserve and all current
player energy, leaving every other blessing/amulet reserve unchanged. A valid
zero/zero payment can capture. Missing or malformed Future state rejects before
command bookkeeping. Positive own reserve is an energy-paid activation for
silence even if the player bar is zero. Returning to an existing anchor is free.

Capture prepares a detached paid Run, records only the player portion in the
existing temporal cost ledger, synchronizes shared bodies, and builds the anchor
before publishing the changed Run. Fallible capture cannot leave a partial
payment. The anchor contains the post-payment zero pools and exact other
reserves. Raw save/load, Snapshot, and Future restore preserve those values and
Reaping's independent damage carry. The desktop registry now admits Future and
shows all-own-reserve plus current-player cost and reserve-only silence.

## Integration boundaries

Activation, upkeep, reaction and terrain/combat adapters use explicit canonical
IDs. This module does not infer enemy reserve capacities or convert ordinary
consumable gains, health/energy damage, equipment cap changes or regeneration
into blessing payment. Target admission and actual effects remain caller-owned.
Unspeak preserves detached-target preview, Ascend retains rollback, and Tinker
validates before payment and commits its existing two RNG draws only on success.

The global R50 startup/current-engine/storage/replay release gates are separate
from these mechanisms. This change does not open them or claim a playable build.

## Regression coverage and verification

Source regressions cover own-only, partial split, exhausted fallback, combined
insufficiency, stale commit, missing/invalid state, invalid all-or-nothing refill,
real paid activation, dormant pickup/equip/unequip/drop/reacquisition, offers and
learned promotion, Remember, Tinker, canonical shared bodies, level-up exclusion
of player/amulet energy, raw roundtrips, Snapshot and paid Future restoration.
Future cases cover all four positive/zero own-player combinations, silence and
free return, exact paid anchor state, and unchanged other reserves/Reaping carry.
The existing Mini-Me tests assert no copied powers/reserves.

The actual R50 starter-shell regression is explicitly ignored while the shared
startup gate remains closed; remove that ignore at coordinated release admission.
No Cargo/test execution occurred while these source edits were in flight.
The coordinating verification pass must run the following native filters:

- `revision50_ability_energy` and `revision50_energy_lifecycle`
- `revision50_amulet_energy` and module `revision50_amulets::tests`
- `revision50_future_reserve_guard` and `revision50_preserved_edges`
- `revision50_tinker_history`, `revision50_mini_me`, `revision50_reserve_finish`
- desktop `revision50_ui_tests` and `revision50_future_ui_tests`
- the final workspace suite, including unchanged pre-50 parity fixtures

## Complexity and limits

Let B be stored reserves, I inventory items, G total carried/owned grant IDs,
L their total ID bytes, and A catalog entries (currently 72). A single payment
is expected O(ID bytes) hash work, with O(B × ID bytes) worst-case collision work,
and O(ID bytes) plan storage. Split arithmetic and revalidation add constant
scalar work. Future's two-pool planning uses the same lookup, never a scan of all
reserves. Validation/refill is O(B+L) time and O(1) scratch, once per level-up.

Acquisition validates existing B entries and supplied IDs against A catalog
entries or the existing copied-trait registry, adding O(G×A×maximum ID bytes)
metadata checks, then performs expected O(L) ID lookup and insertion work.
Reconciliation additionally scans I items and G IDs and allocates copied IDs:
O(B+I+G+L) expected work outside those metadata checks, and O(G+L) temporary
memory, plus O(new IDs) retained state. Worst-case hash collision work is
O(G×B×maximum ID bytes); cold catalog/trait registry parsing costs its input
bytes and retained table allocation once per process. It runs at
existing ownership boundaries, not on rendering. The existing equipment
catalog/ownership/condition scans and environment/health settlement costs remain
additive; they are not included in the reserve lookup bound. Direct equipment
preflight adds O(G×A×maximum ID bytes) metadata work for its inventory grants,
with O(1) scratch after catalog initialization, before any equipped-field change. Pickup adds this
acquisition work for each newly collected item. Tinker retains its catalog/god
pool scans and two local RNG draws. Remember retains target trait initialization
and filtering costs and adds acquisition for the copied IDs.

`spend` additionally clones the owner Actor and uses existing actor lookup,
mutation journal and resource-history helpers. Its time/space includes Actor
collection bytes and those helper costs, not a claimed constant-time owner lookup.
Future capture adds a detached Run clone and anchor scan: O(owned Run collections
+ world/metadata finite-scan nodes + any COW-mutated world blocks) time and memory.
Arc-backed immutable blocks are shared initially. Resource-history and shared-body
synchronization can scan actors, compute visibility or detach actor blocks;
those existing transitive costs are additive. Save/restore retains its existing
serialization/validation costs. No gameplay randomness or renderer was added.
UI adds keyed reserve lookup and bounded cost-text creation to the existing
catalog/ability projection; font rasterization and frame painting are unchanged.
These are source-derived bounds, not measured timings.

## Activation-turn maintenance correction (2026-11-07)

Priced modes that grant their benefit immediately now prepay their first normal
turn of maintenance when enabled: Phase Through, Rushing Torrent, Black Pit,
Shadow In The Dark, Not There, Turning The Wheel, Fated Duel, Unspeak,
Dream-See, Mind Read and Bonded By Unseen Threads.
Payment uses the blessing reserve then player energy, with all-or-nothing
admission. Free toggles and haste do not spend another turn of maintenance.
The next normal upkeep boundary consumes this prepaid coverage rather than
charging twice. Deactivation does not refund or erase paid coverage; switching
Unspeak to a more expensive target pays only the difference without extending
that coverage. Exact funding keeps the mode active through its paid turn.

Riptide retains its already-immediate 15 activation cost and subsequent 10 upkeep.
Attack/reaction costs remain payable on their trigger, not when armed. The designer
explicitly restored Dream-See and Mind Read to 1/turn, Bonded By Unseen Threads
to 15 activation plus 8/turn, and I Came For You to 10 activation. Bonded admits
and spends the combined 23 atomically on first activation. Re-enabling Bonded
within the paid turn charges its distinct 15 activation fee again but does not
charge that turn's 8 upkeep twice. Implant Thought retains its R50 15 cost.

Two numeric conditions per prepaid ability hold the covered tick and amount.
They serialize and restore with the Actor, so Snapshot and Future naturally
restore coverage together with the corresponding resource state. Level-up and
travel do not refund an already-paid turn or extend its tick. Historical revisions
ignore this bookkeeping. No random draws or new simulation clock were added.

Complexity: for K actor conditions and ID length L, prepayment adds two expected
O(L) keyed accesses, worst-case O(K*L) hash-collision work, O(L) scratch and two
retained entries per affected acquired ability. Existing reserve-payment Actor
cloning, mutation journal and shared-body synchronization costs remain additive.
Upkeep remains linear in active abilities times those lookups and existing spend
costs. This is source analysis, not a performance measurement.
