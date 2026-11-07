# Revision 50 items, generation and carrying categories

Source implementation of the approved first-build defaults. This pass does not
open normal R50 startup, Engine, save/replay or Windows release admission. The
final regression run must record results; tests here are authored, not
claimed executed during this implementation checkpoint.

## Production routes

Ordinary permanent counts remain unchanged (two early, two through four later,
within the existing slot budget). The old equal equipment-kind pool is filtered
to useful kinds: weapons now contain only the existing Composite bow, alongside
rings and amulets. Armor and ordinary unequippable melee drops are removed. The
first two permanent kinds remain distinct. Named powers are uniform within their
14-ring and 12-amulet groups. There are no jewelry quality tiers, random stat
modifiers, curses or generated ability grants. The Composite bow keeps its base
shot arithmetic without unusable equip bonuses. Bow remains the starting bow.

Sphinx and Mercuria rewards call the revision-aware permanent constructor. The
old event catalog, filtered to useful candidates, has one Composite bow, one
ring and one amulet, so each group retains equal weight among eligible entries.
Both event and ordinary loot route jewelry through the same fixed named-power
constructors. New amulets have their unique item ID and initial 50 energy. No
materialization can refill an existing state. All random decisions use the
supplied GenerationRng; the seed/depth event streams and legacy paths remain
unchanged. Removing the starter Shortsword keeps Bow ID2, arrows ID3 and nextId4,
without an extra RNG draw or changing old startup payloads.

## Capacity and identity

A carrying Category is separate from Item.kind. Bows remain weapons; arrows
remain ammo; Borge remains its weapon identity and existing throw behavior;
Phantasmii remains tool/quest; Ahvamitt remains a tool; Royal Honey remains food.
Tools, bows, arrows, quest shards, Borge, Royal Honey and explicit trinkets use
unlimited trinket capacity, one slot per stack. Ordinary melee/armor are not
reclassified as special items and cannot be worn.

Rings, amulets, potions and hearts each have an independent base capacity of five.
Trinkets are unlimited. Tools of the Trade adds five rings and five potions;
Bottomless Pack adds two amulets and five potions; Appropriate adds three rings
and three amulets. All three together permit 13 rings, 10 amulets, 15 potions and
five hearts. These budgets are independent and count quantities, including old
items carrying an equipped flag. There is no shared-jewelry or worn-item escape.

Every carried ring and amulet is automatically active. Named powers, stat bonuses,
crafts and Tinker grants use the carried inventory, without a Wear/Take off action.
Dropping the last source ends its power, settles health/energy and flight loss,
and preserves spent item/blessing energy on later pickup. Non-Light duplicate
powers remain nonstacking; Light remains additive. Amulets Apply directly while
carried. Removing a capacity-granting item is refused if the reduced category
budget cannot hold the remaining items; nothing is silently discarded.

The old defensive 100-entry persistence bound exempts only trinkets; finite
categories are validated by quantities. Legacy revisions keep their old bounds
and original pickup/equip logic. Capacity represents unlimited trinkets as None,
not infinity. Pack has five sections with each category's own count/budget.
The finite total is only a summary, never a global backpack cap.

## Masque naming

New R50 rewards are named “Masque of the Trickster.” Identity recognizes both
historical “Mask of Trickery” and “Mask of the Trickster” spellings. The shared
R50 display-name path presents the canonical Masque name without rewriting old
saved items, and the teleport message uses it only at R50. The action, 100-turn
cooldown and old-revision fixture copy are unchanged.

## Verification scope

New core filters: `equipment::generated_loot::revision50_tests` and
`revision50_inventory::tests`; desktop filter: `revision50_pack_tests`;
integration target: `revision50_release_boundaries`. They cover exact uniform
single-draw jewelry selection, all 26 reachable named effects, deterministic
floor loot and IDs, initial amulet state, real pickup/carried activation/Apply, bow Apply
without equip, actual Sphinx/Mercuria routes, >100 trinkets and exact finite jewelry limits,
finite-category rejection, identity preservation and Masque compatibility.
The existing `equipment::generated_loot::tests::exact_loot_and_reward_draws`,
`equipment::generated_loot::revision43_tests` and legacy population/ahvamitt/UI
fixtures remain required legacy checks. The all-named carried-item test exercises the separately owned Reaping policy
implementation; no local admission bypass is added.

## Complexity

- R50 permanent selection: O(1) fixed-array lookup per item plus O(P) returned
  payload allocation, P item bytes. Ordinary generation with L<=20 slots is
  O(L+P+E) time and memory for total payload P and the existing E-byte exclusion
  copy; legacy catalog selection, modification and RNG order remain unchanged.
- Category lookup: O(N) string identity bytes, O(1) scratch. Trinket admission
  needs no inventory scan. Finite-category admission is O(I N+B) time and O(1)
  scratch for I inventory entries, maximum identity bytes N and B blessing
  comparisons. Pickup retains stable O(G*(I N+B)+P) scanning and payload output
  costs for G ground entries, including stack matching and item initialization.
- Boundary validation: O(P+I N+B) time, O(P+B) temporary item/ability decoding to
  reuse the runtime classifier. Serialized traversal, nested item checks and
  all other validators still run. No global count bounds unlimited categories.
- Pack capture retains its existing grouping and description costs. Limited
  occupancy adds O(I N) scanning, no per-frame simulation work; fixed summary
  strings add bounded text layout/raster work through the current glyph cache.
  Unlimited entries still cost O(I) storage and downstream inventory scans.
- Masque identity/display adds O(N) name comparison and output copying, with no
  state mutation, catalog scan, RNG or new cache. Teleport pathfinding is unchanged.

These are asymptotic source estimates, not timings. Profile large inventories
and save payloads after the focused and full native suites pass. Windows input,
rendering, save/replay and playable release acceptance remain separate checks.

## Deeper synthetic/debug loadout follow-up

The R50 synthetic playtest path preserves Bow/arrows as its two starter entries.
It retains the existing seeded sample of two rings, one amulet and up to four
potions, plus unlimited useful trinket candidates. Unselected prior loot lives in
an internal preparation-only candidate list, outside carried inventory. It cannot
apply ring bonuses, grants or resources to subsequent heart collection, level-up
choices or generation. Only final selected items acquire reserves and become
active together, without equip commands or slot assignment. The candidate list
is empty before the run is returned and absent from the save schema. Missing
categories remain unfilled; no new loot weights or random draws are added.

The debug stash retains its old twenty-slot path before R50. At R50 take checks
the normal classifier and the four independent category budgets, merges matching
ordinary stack identities, and settles carried ownership on a detached Run before
publication. Return removes jewelry powers immediately and rejects losing a
capacity-granting item if the remaining inventory would overflow. Errors leave
items, reserves, provenance and loadout revision unchanged. The historical E/equip
shortcut only carries jewelry from the stash under the same limits; on already
carried jewelry it changes nothing. No equipped flag bypasses capacity. The debug
UI shows all four finite counts, unlimited trinkets, automatic jewelry activation,
and “E: carry jewelry.” No overflow is discarded and reacquisition never refills.

Filters `playtest::revision50_loadout_tests` and
`debug_floor::revision50_loadout_tests` cover deeper floor6/floor40 collection,
selection, candidate isolation from prior-heart progression, five rings plus five
amulets, finite potion/heart overflow, >100 trinkets, arrow merging, acquired grant
ownership, spent-charge reacquisition, capacity-loss rejection and unchanged
legacy inventory. `debug_floor_ui::tests::r50_debug_e_carries_active_jewelry_without_capacity_bypass`
checks the actual keyboard action and category summary. These tests are authored
for the coordinated native run; no Cargo was run by this editing checkpoint.

Complexity: staging L new candidates costs O(L) time/item payload per prior floor,
and retains O(I) candidate payload for total I ordinary prior-floor items. There
is no serialized candidate field. Final selection shuffles I candidates in O(I),
with a bounded number of jewelry searches/removals. The inherited provenance
filter is O(S*I) worst-case for S source entries because all trinkets are retained.
Selected-item acquisition costs O(G*A*N+P) for G grant IDs, A=72 catalog entries,
maximum ID bytes N and cloned item/player payload P. One final ownership
settlement includes grant/stat, environment, linked-actor and health hooks; it is
not treated as O(1). The overall selection uses O(I+P) temporary/returned storage.

Debug take and return additionally clone owned Run metadata R while sharing large
COW world blocks, perform inventory/category/grant scans, shift O(S) stash entries,
and run ordinary ownership settlement. Temporary space is O(R+P), plus any COW
blocks detached by settlement. A rejected capacity-loss preview also clones and
scans the remaining inventory/grant state. UI summaries add four O(I) quantity
passes and the existing full item-list traversal, while only thirteen rows per
pane are painted. Font/cache/native raster costs remain those of the ordinary
renderer. These are source bounds, not measured timing claims.
