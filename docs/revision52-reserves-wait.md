# Revision 52 reserve ceilings and Wait recovery

The 2026-11-08 designer request reduces amulet and blessing reserves from 50 to
10 energy. This applies to existing revision-52 reserves as well as new ones.
Earlier simulation revisions keep their existing 50-energy rules.

## Mechanics

- New R52 blessing acquisition and all ordinary/special amulet generation and
  first acquisition use 10 energy. Existing charges below 10 are not refilled
  by reconciliation, dropping/reacquiring, raw migration or restoration.
- R52 level-up restores acquired blessing reserves to 10, including dormant
  grants. It does not refill amulets or the player's energy bar.
- Amulets still spend their own energy first and then player energy. Blessings
  still use their own reserve then player energy. Costs, target admission,
  Appropriate's discount, Spellslinger's action rules and atomic rejection are
  unchanged. A pre-amendment R52 reserve containing 50 can supply at most 10.
- Reaping keeps its existing direct-hit threshold and carry, and replenishes
  amulets only up to 10. Full reserves waste whole awards as before. No passive
  recharge, offscreen recharge or wait recharge is added.
- A Wait action restores exactly one reaction, capped at the actor's existing
  maximum. Missing reaction state counts as zero. The action restores one in
  either normal or haste entry phase; the world/haste ticks themselves restore
  none. Earlier revisions still fully refresh on Wait. Maximum reaction policy
  belongs to the independent Hanged amendment, not this change.

## Saved-state and restore boundaries

`storage::deserialize`, the compatible loader, `temporal::Engine::new`, Snapshot
restore and native/text Future restore clamp valid R52 reserve state. Stored
ability capacity becomes min(old capacity, 10), energy min(old energy, new
capacity), and amulet energy min(old energy, 10). Neither exhausted reserves nor
smaller fixture capacities receive a refill. Normalization covers player, all
floor actors including carried pilots, copied/carried inventories, ground items,
remembered/detected item projections and debug stash/candidates. Invalid entries
reject before normalization writes anything. Unrelated player energy, HP, RNG,
reactions, Reaping carry, activation ledgers and time costs are unchanged.

Future anchors remain immutable; restored state is normalized before publishing.
Snapshot restore validates and prepares the normalized image before changing the
live run. The low-level canonical fixture reader and raw serde serialization
remain exact rather than silently becoming migration APIs. Pure R52 payment and
UI projections also enforce the ceiling for direct raw-state callers. Frozen
R50 helper APIs retain their original semantics; production callers pass the
simulation revision explicitly.

## Complexity

Let A include all actors and nested pilots, B all blessing reserve entries,
I physical/copied/carried/debug items, K remembered/detected item records, and
D the nested-pilot depth. Load/restore normalization takes O(A+B+I+K) traversal
time and O(D) call-stack memory. It performs a validation pass before mutation.
Changed Arc-backed actor/ground blocks incur their full payload copy cost when
shared; unchanged blocks do not detach. No floor tile scan, RNG or recursive
Future-anchor parse is added. The boundary clone is already part of Snapshot
and native Future restoration. No normalization pass is added per world tick.

Payment and display projection add O(1) arithmetic to existing keyed lookup and
validation; allocation and hash-collision bounds remain those documented for
R50 accounting. Acquisition adds an O(B) clamp to its existing O(B) reserve
validation. Reaping adds O(1) clamp arithmetic per amulet in its existing inventory
scan. Wait adds O(1) arithmetic; its maximum-reaction lookup still scans the fixed
reaction-ID list against owned ability IDs. These are source-level bounds, not
measured timings.

## Verification limits

This patch was prepared against an incomplete recovered R52 source candidate.
Focused core integration checks and legacy tests are reported in the separate
receipts. They are not a complete-source workspace pass, final build or Windows
playthrough. Missing/unverified Floods sources remain a separate recovery issue.
