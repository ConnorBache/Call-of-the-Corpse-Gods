# Revision 50 carried inventory correction

The designer's November 6 correction replaces the draft shared-jewelry/wearing
model. Five Pack sections: trinkets, amulets, rings, potions and hearts. Each finite
section starts at five units. Trinkets are unlimited. Existing bonuses are retained:
Tools of the Trade +5 rings/+5 potions, Bottomless Pack +2 amulets/+5 potions,
Appropriate +3 rings/+3 amulets. Combined: 13 rings, 10 amulets, 15 potions, 5 hearts.

## Behavior and boundaries

- Every carried ring/amulet counts and is active, independent of Missing, Null,
  empty or legacy equipped flags. No manual Wear/Take off action or cap exemption.
- Named powers, numeric bonuses, enhancements, flight and item grants use carried
  activity. Non-Light duplicates do not multiply fixed power; Light is additive.
  Actor-local copied inventories and shared owners retain their existing scope;
  stats-only Mini-Me and decoys do not gain ring powers.
- Amulets Apply while carried. Pickup and debug acquisition initialize only new
  charge. Dropping, returning to stash, reacquiring, loading and temporal restore
  cannot refill a spent item or already-acquired blessing reserve.
- Pickup settles grants/health after jewelry insertion, so subsequent underfoot
  items see newly acquired capacity. Drop/throw settle ownership, health, energy
  and flight loss. Loss of the last Flight ring on a gap follows existing falls.
- Removing jewelry that supplies needed carrying capacity is refused before
  mutation, matching the prior non-destructive capacity-loss rule. Existing items
  are not deleted or silently relocated. Debug return uses whole-stack preview.
- Tree-tap conversion also checks the potion category before consuming the tap or
  marking the unique Sap reward harvested.
- Save and temporal admission counts all jewelry quantities. Only unlimited
  trinkets may bypass the defensive 100-entry limit; invalid finite inventories
  are rejected without destructive normalization. Item.kind identities remain
  unchanged for bows, ammunition, quest shards, tools and food.
- All behavior below revision 50 is kept on the historical branch.

## Complexity

Let I be carried entries, N maximum item identity bytes, B owned blessing IDs,
G ground entries, P copied item/player payload, T granted IDs, A=72 catalog rows,
C actor conditions and E enhancements. Trinkets mean I has no universal bound.

Category classification O(N), O(1) scratch. Counting or finite admission is
O(I*N+B), O(1) scratch, once per pickup/admission/Pack refresh. Jewelry activity
is O(1) kind comparison and uses no cache. Named-ring queries remain O(I+C), O(1)
scratch. Grant reconciliation adds existing O(I+T*A+ID bytes) enumeration and
metadata checks, plus retained reserve keys, without refill or random draws.

Pickup scans G ground entries with existing stack matching and O(I*N+B) capacity
checks each. Every accepted jewelry entry also performs full existing equipment
settlement S: stat/condition/bonus/enhancement scans, owner synchronization,
achievement/resource hooks, and flight-loss geometry/travel if needed. Its cost
is O(G*(I*N+B+S)+P), not just a linear inventory pass. Ground output is O(G+P).

Capacity-loss preview clones the inventory O(P), derives prospective grants
O(I+T*A+B) and checks category use O(I*N+B), O(P+T+B) scratch. Drop's existing
settlement S is additional. Debug wrappers still clone the Run for atomicity.
Pack category counts add O(I*N+B), O(1) counters plus five headers; existing
quadratic grouping and text/glyph/raster costs still apply. The bounded visible
rows do not make refresh cost constant. Validation includes full serialized and
nested payload traversal in addition to category checks.

These are source estimates, not measured timings. Benchmark large trinket
inventories, pickup batches and saves after the native checks below.

## Verification status

New focused unit coverage in revision50_inventory::tests covers five independent
caps, old flags counting, no manual toggles, immediate pickup activation, spent
charge roundtrips, non-destructive capacity removal and full-potion tree-tap
rejection. Pack, loadout, save/temporal and existing named-effect tests are updated
in parallel. check-revision50-source.py verifies catalog/module/source contracts
only; it does not prove native compilation or gameplay. Final native suite,
Windows build and pixel review are owned by the coordinated release pass.
