# The Borge — simulation revision 33

The fourth **defined non-final Godbeast candidate** is the God-Obelisk event,
not a new creature actor. The designer confirmed seven Godbeast encounters on
floors **5, 10, 15, 20, 25, 30 and 35**, then the existing Flesh Administrator on
40. A deterministic four-type bag reuses Mercuria, Zillennia, the existing God-Tree
**Verdania**, and Borge, without inventing the three future designs. The old
`Veridian` internal tree/sap identifier stays compatible; r33 presentation says
Verdania. Revisions 29–32 retain their original domain plan and draw ordering.

## Interaction and identity

* Borge occupies 25 solid central cells (5×5), surrounded by a two-cell clear ring
  in a 9×9 protected room. The center is the closest deterministic position to
  map center that does not conflict with either stair sanctuary. Existing
  connectivity repair connects the perimeter and altar without cutting the body.
* Bump or `Interact` with an edge for **“The Borge demands a sacrifice...”**.
  No damage is dealt. The first interaction commits the event, including an empty
  floor; later interactions never reselect or create another quest.
* Select the living eligible actor with greatest maxHP, then DR, then Defense;
  lowest actor ID breaks ties. Player, player clones, heartless entities,
  invulnerable Sphinx/protected actors, physical mechs without obtainable hearts,
  unsupported heart species and a creature already pledged to another open
  Borge are ineligible. Selection uses no RNG and spawns no new sacrifice.
* The target is visibly marked without revealing a hidden actor or a Skin
  Walker's true identity. On death its one heart receives the exact source actor
  ID and Borge floor. Ordinary hearts are unchanged; a marked Jackal heart is not
  merged into a species stack. Corpse drops, carry/drop, floor travel, save/load,
  replay and temporal snapshots retain this identity.
* Bump/Interact with the altar while carrying **that** heart to offer it.
  A same-species heart is not accepted. The offering consumes the heart without
  applying its food stats, energy or blessings, and yields exactly one Shard.
  Eating/crafting the heart remains an ordinary deliberate consumption; it does
  not reset the quest or create a replacement sacrifice.

## Shard interface and permanent death

The Shard is a one-handed equippable weapon, or an inventory **Use on…** action
with adjacent-cell targeting (`UseAt`). Equipped normal bump attacks also use it.
It accepts adjacent living creatures, including allies, neutral creatures,
Sphinx and the Administrator; it does not target environmental obelisk cells.
Out-of-range, empty, blocked-diagonal or otherwise invalid use does not consume
it. It cannot be thrown or fired. A successful use has no accuracy roll: consume
one Shard, set the victim's existing no-revival trait, then run the existing
unpreventable death/lifecycle path. Phoenix revival is impossible. Special death
hooks remain active, including Administrator collapse and mech/pilot handling.
Revert/See the Future restore the whole branch, not an out-of-band permanent flag.

Exact item description:

> A shard of the dark obelisk. Any creature which it bleeds will fall into an endless nightmare. It can only be used once.

## Verification and cost

Focused regressions are in `crates/corpse-core/tests/borge_event.rs`; desktop Pack,
inspection and target behavior has separate focused tests. These are source tests,
not a claim of platform screenshot verification. Run `CARGO_INCREMENTAL=0 bash
tools/cargo.sh test -p corpse-core --test borge_event`; the full workspace remains
the integration release gate.

Variables: V floor cells (2,291/2,500/3,600), F generated floors (≤40), A actors on
active floor, B actor trait/condition keys, H one heart template's stored payload,
I inventory entries, G ground items, S avoided stair cells (2).

* Plan: O(8) time and memory once per plan query; no simulation RNG.
* Placement: O(V×S) center scan +81 writes, O(1) additional scratch. Existing
  generation/connectivity/topology costs are unchanged and **not** included in
  that placement bound; see the generation ledger. Body/altar lookup is O(1).
* First demand: O(A×(F+B+H)) time, O(H) transient template clone. F accounts for
  already-pledged target checks. The fixed heart catalog initializes once by
  parsing its bytes (cold-cache O(catalog bytes) time/memory), then expected O(1)
  key lookup; an unknown heart template is rejected.
* Provenance tagging: O(F) at the selected actor's ordinary death, O(1) item/event
  fields. Offering: O(I) scan/removal, O(1) new reward and no food computation.
* Shard: O(I+A+B) local admission/consume/flag work, **plus existing** resource,
  death, heart/ground placement, equipment reconciliation and boss lifecycle
  costs. Item placement may traverse up to 40 floors and trigger existing floor
  generation; this path is not described as constant-time.
* Target display: O(F) per visible actor; tile feature projection is O(1) per
  observed cell. Native text/glyph rendering and visibility use the desktop's
  existing renderer/visibility costs. No extra persistent render cache.
* Snapshot/save/replay data: O(1) additional fields per Borge event and selected
  heart; existing serialization and copy-on-write costs still apply. No external
  quest ledger or wall-clock state.

Bounds are source analysis, not performance measurements. Profile cold/warm
catalog selection on dense floors, generated central rooms across seeds/domains,
and inventory/ground overflow plus boss death separately before optimization.
