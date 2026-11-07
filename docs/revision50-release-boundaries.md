# R50 startup identities and persistence boundaries

The identity/persistence preparation below is now complemented by the coordinated
[release admission and acceptance gates](revision50-release-admission.md).
Production creation, Engine/load, replay, debug and playtest accept exact revision
50, and desktop new games select it. Active legacy saves keep their recorded
revision. Approved item work is described below and in
`revision50-items-generation.md`; these boundary changes do not define mechanics.

## Corrected boundaries

`starter_abilities_for_revision` preserves every frozen starter position and
selection weight, applying only the explicit replacements in
`revision50-request.txt` lines 64–74 and `ability-identities-v50.json`:
Unyielding Assault → Execute, Never Without a Fight → Fated Duel, and
Dilute Potion → Bottomless Pack. Startup calls the versioned selector; legacy
selectors and random-draw counts remain unchanged.

Generated-grant, equipped-grant and item-grant validation now recognizes R50
identities in R50 data without accepting new identities in earlier revisions.
Active primary/additional/equipment grants reject retired identities. Historical
`generatedAbilityGrants` provenance may retain a recognized legacy identity;
that record says what was generated, not which power is currently equipped.
Unknown IDs are rejected. Tinker now adds each historical ID's explicit R50
identity from `ability-identities-v50.json` to its exclusion set. The original
history remains untouched. Active powers/item grants are not migrated, and pool
order, god selection, weights and the two RNG draws are unchanged. Earlier
revisions never enter this mapping branch. Runtime admission is documented separately.

The existing recursive boundary traversal validates additional grants and reserve
IDs/numbers in nested actor/item payloads. Additional grants are R50 jewelry
powers, not arbitrary enemy traits. Actor reserves also admit the existing
supported passive enemy-trait class used by Remember/copied-power paths. Dormant
reserves need not correspond to a currently owned ability. Missing reserve maps
remain missing; validation never creates capacity or charge.

Only R50 trinkets are exempt from the old 100-entry defensive inventory bound.
Rings, amulets, potions and hearts each have an independent base quantity limit
of five; the existing category-specific blessing bonuses remain. Every carried
ring and amulet is active and counted, regardless of historical `equipped`
flags. Neither a slot label nor more than 100 entries grants extra jewelry space.
Over-limit saves are rejected without deleting, stashing, or rewriting any item.
All legacy inventories retain their original bound and slot semantics. Carrying
categories remain separate from item identity; `Item.kind` is never rewritten.
See `revision50-items-generation.md`.

The load-time health projection now uses carried jewelry for R50, including
integer-stat and divine-revision normalization, while keeping the earlier slot
predicate below R50. This preserves existing item metadata and spent charge.
Fresh and imported Future anchors use the same strict category validation before
restoration; a rejected anchor leaves the live run unchanged. Ordinary snapshots
restore inventory, ownership and amulet charge together without inventing flags.

## Complexity

- Starter selection runs at creation. For P starter entries, maximum ID length L,
  transformation costs O(P L) time and O(P L) returned string storage. Current
  pools have at most two entries; no RNG or catalog allocation is added.
- Persisted validation runs at import/checkpoint validation, not render or turn
  upkeep. Let S be serialized nodes, D nesting depth, G total checked grant and
  reserve IDs, A catalog size (72), L maximum ID bytes, and K the largest one-item
  additional-grant list. The existing Value traversal remains O(S + G A L +
  G K L) worst-case time and O(D) traversal stack. The last term covers duplicate
  prefix comparison, not a constant-time uniqueness claim. Catalog initialization
  is O(catalog bytes) once, with retained catalog storage. Supported passive-trait
  lookup uses the existing bounded registry and its string-key lookup cost.
- Tinker history mapping runs when candidate pools are queried. For H historical
  entries, A=72 identity rows and maximum ID bytes L, it adds O(H A L) lookup work
  and at most H inserted strings. First use parses and retains O(mapping bytes).
  Let N be total exclusions from history and item grants. Hash-set construction
  is expected O(N L), worst O(N² L) under collisions; candidate membership is
  expected O(A L), worst O(A N L). The existing inventory/ground traversal and
  fixed twelve-god catalog passes remain. No per-turn or rendering cache is added.
- Inventory cap and finite-category checking is O(P + I + B) time and O(P + B)
  temporary memory for I item entries, their P payload bytes and B ability data.
  The boundary decodes once to reuse runtime item identity classification. Entries
  may exceed 100 only through trinkets; serialization/item checks still
  scale with all entries. Unlimited capacity is not constant simulation or memory cost.

`revision50_release_boundaries.rs` adds deterministic boundary fixtures. They are
authored but not executed in this pass; the final native regression run
must record results. They do not establish playable R50 or Windows behavior.

`revision50_tinker_history.rs` covers all explicit replacement rows, equivalent
old/new history pool order, fixed two-draw activation, no mutation while querying,
and legacy exact-match exclusion. Those tests are authored, not executed by the
patch-preparation worker.

Item categories are now a separate carrying classification, not a rewrite of
`Item.kind`: Borge and quest identity predicates depend on that kind. The existing
permissible Borge throw path is retained, and R50 item copy no longer suggests
equipping it. No new Borge attack was added.


## Carried-jewelry boundary regressions

`revision50_release_boundaries.rs` covers five of every finite category together,
120 trinkets, legacy entry bounds, quantity stacks, mixed/missing legacy equipment
flags, and atomic rejection of six or 101 rings/amulets. Raw and compatible loaders
are exercised without destructive inventory normalization.
`revision50_inventory_boundaries.rs` covers exact save/snapshot inventory and spent
amulet charge, native and reloaded Future restore, invalid native/text Future
rejection, and revision-specific load-time health projections. These tests are
authored for the coordinated Cargo run; no executed or platform pass is claimed
by this editing checkpoint.

The health projection changes only its O(1) activity predicate within the existing
O(actor count × (inventory entries + enhancements + ability/condition sources))
load-time scans. Future/snapshot work retains its existing owned-state clone,
serialization and validation costs; no new per-turn traversal or cache is added.
