# Timeweave state primitive — data only, not movement behavior

The passive description is exactly: “You can freely travel along your time weave,
which you leave behind as you move.” This module does not grant the ability,
change Revert, choose arrow placement, determine free movement, or resolve forced
movement, portal, stair, floor or dream rules.

## Representation and admission
`Trail` owns a private oldest-to-newest VecDeque and a private HashMap keyed by
Cell(floor identity, plane, tile index). The floor integer is an opaque canonical
identity supplied by the movement integration, not inferred from UI depth.
Each Segment explicitly supplies occupied cell plus directed from/to endpoints.
No constructor assumes whether the occupied cell is from or to. Coordinate
geometry/ability/revision admission belongs to the implemented revision41 integration
and boundary validator, rather than this container. Endpoint continuity is not imposed while crossing policy
is pending. Serialization is `{segments:[...]}` in chronological order; the
randomly ordered derived map never reaches disk. Unknown fields and duplicate
occupied cells are rejected on decode. This preserves one segment per cell.

`remove_through(cell)` removes that existing segment and all older segments;
absent cell is a no-op. `append(segment)` performs that collision removal, then
adds the supplied segment as newest. Following an existing weave can use lookup
without append; this primitive does not decide whether following causes a cut.

## Costs
Let L be live segments, H peak allocated deque/map capacity, k removed segments.
Lookup expected O(1), append expected amortized O(1+k), prefix deletion expected
O(1+k). No Vec front shifts, whole-trail search or survivor serialization per
append. Hash worst cases are O(L) per operation; capacity growth can copy O(L)
entries. Each removed segment is visited once. Memory retained O(H), not O(L)
after a large deletion: capacity is intentionally not shrunk on each collision.
Canonical equality, encoding, decoding and derived-index rebuilding cost O(L)
expected. Decode additionally retains O(L) lookup storage; enclosing save byte
budgets remain required, and this primitive does not introduce a gameplay cap.

Proposed `Run.timeweave: Field<Arc<Trail>>` preserves Missing on old save output.
Run/snapshot Arc clone is O(1), BUT first mutation while shared clones the entire
Trail and map, O(H+L) conservatively including retained hash-table capacity. Existing command candidate cloning can cause that every
movement, even after completed Revert snapshots are removed. This is explicitly
NOT an end-to-end O(1) movement claim. Measure long trails. A later authoritative
owner/accepted-edit strategy may avoid copies, but must preserve candidate
rollback, Future restore and crossing semantics; not silently assumed here.

## Integration status
The core module, optional Run field, explicit snapshot clone field and primitive
regressions are integrated. Movement rules, revision41 gameplay admission and
replay policy are owned by their separate integration changes. Old29–40 rules
must not acquire new behavior merely because this field can be parsed.
The release checker tools/check-snapshot-roots.py tracks optional root presence;
legacy overwrite exports must not acquire an extra absent remove patch solely
because the new field exists in the schema.

Tests implemented: chronological collision and repeated cell replacement, absent/no-op,
oldest/newest deletion, exact JSON roundtrip/order, duplicate/unknown rejection,
floor/plane isolation, explicit reversed endpoint ownership, shared Arc snapshot
preservation,4096-entry prefix deletion. No timing claim is implied by these tests. A full Run serde test also checks
Missing omission, Null preservation, exact re-encoding and snapshot Arc sharing.

## Initial conservative integration (superseded by candidate probe below)
The approved ordinary case is now wired in engine.rs: voluntary waking A→B
places an arrow at A pointing B; B→A follows that exact reversed edge freely
and does not append or overwrite. A wrong-side arrival at an occupied weave
cell does not delete it. A later new departure places a new source arrow there,
removing that old source segment and all older segments before append.

The early free branch is after normal blockers/free-action admission but before
chill, resource-turn initialization, haste advancement and world turn. It runs
the ordinary movement arrival hooks and finish(false), retaining pickup, door
settlement and observation, without consuming the haste phase. The command's
actual coordinate-write flag—not Move's boolean—is proof of displacement.
Attacks and interactions are never accepted as free steps. Unexpected endpoint
changes after strict preflight error within the existing command candidate.

Coverage is deliberately bounded: waking floor/grass unit-grid moves with no
portal remap, transitions, forcedMove, confusion, grab, hemophilia, riptide,
paralysis, source ice or sticky source terrain. These special cases still use
normal charged handling without a new benefit/record; their final Weave policy
is PENDING, not claimed implemented. No gameplay direction RNG is drawn by the
preflight. Public movement calls used by currents and abilities do not receive
the command-only recording capability. This is not a general teleport history.

Preflight scans existing actor/ability predicates and calls existing passability,
portal and terrain queries: O(A + transitive predicate costs), not O(1) total.
The free path currently repeats this read-only admission in its wrapper; no
movement or RNG draw is repeated. Normal movement/finish costs remain, including
existing snapshots/candidate copies and potentially O(H+L) trail COW detachment.
Tests cover source collision timing, charged attacks, no appended free edges,
unchanged world clock/phase, and revision41 haste without pending Revert anchor.

## Compact unpublished revision41 wire
The typed Segment API remains unchanged, but Trail now serializes chronological
segments as four-tuples `[floor, plane, fromTile, toTile]` inside the same closed
`{segments:[...]}` envelope. Plane is the exact `waking`/`dream` enum. Occupied
cell and source are reconstructed identically, and destination shares the source
floor/plane. The serializer rejects any typed Segment that would lose those
facts (cell!=from or cross-floor/plane); it does not silently normalize it.
Decode rejects duplicate occupied cells, unknown envelope fields, wrong tuple
lengths/types, invalid plane names and integers outside u32. Boundary validate_run
still checks actual floor existence, tile bounds and nonzero adjacency. Tuple
encoding is O(L) with O(1) wrapper scratch; decoding holds O(L) temporary tuples
plus the rebuilt O(L) deque/map. The first verbose-wire size evidence is retained
separately; no previously published revision41 save format is being replaced.


## Current voluntary movement/status/dream coverage
Voluntary local adjacent movement now uses the same original movement kernel for
ice, sticky terrain, held states, doors, confusion and waking/dream planes.
A potential reverse edge is detected by one expected lookup normally, or at most
eight neighboring lookups under confusion; no game RNG is drawn by that check.
Only such a relevant candidate clones Run and the pending presentation batch.
The actual coordinate-commit trace records resolved source/destination/plane and
whether the command attempted an attack. A successful exact reverse edge commits
movement/arrival/finish(false), with no haste/resource/world time prelude and no
new segment. Held/slipped/nonmatching/attacking/error probes discard all cloned
Run state, RNG, logs and presentation output, then execute the original charged
path. Confusion therefore uses the resolved edge, not the pressed direction.
Ordinary charged steps also record resolved edges only after actual displacement.

Presentation isolation clones the existing Batch before replacing thread-local
storage. It retains ordinary events and the ordered explosion-dedup map, rather
than flattening them; accepted emissions use the original cap/replacement logic.
Drop restores the outer batch on failure or unwind, including nested probes.
No outer batch is fabricated. Event transaction tests cover nested discard,
unwind, accepted dedup ordering and the existing128 ordinary-event cap.

Costs are deliberately explicit: reverse eligibility O(F+ability count+8 expected
lookups); successful/failed probe additionally O(R) for deep-owned Run metadata,
archive/replay/player/inventory bytes and O(E+B) presentation cloning, plus normal
movement/arrival/COW costs. R is not just the trail. In the core command candidate
Future has already been taken out, but archive/replay remain owned and may dominate
long-session probes. Failed probes execute movement work again on the charged
path, from the original state/RNG. No unsafe move-out optimization is assumed.
The Trail COW O(H+L) caveat still applies to new ordinary segments; free following
has no trail append/detachment. These costs require the long-session benchmarks.

Forced displacement, portal/non-Euclidean transitions, stairs/pits/chasm crossings
still have pending policy and do not gain free travel or trail recording. Probe
classification rejects those edges before transition side effects. This is not
an implementation of a universal rewind/teleport history. Storage geometry remains
same-floor/same-plane adjacent source-owned edges. Dream edges use the distinct
Dream plane identity even though the kernel temporarily projects floor handles.
