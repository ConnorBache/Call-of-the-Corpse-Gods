# Revision 50 combat, movement and resistance rings

This checkpoint implements the designer-approved fixed powers below. Explicit
`kind: "ring"` and `Item.effect` identify each power. Wearing is read from the
existing equipped field. None are inferred from display names, enchantments,
quantity, rarity, Tinker grants or legacy numeric bonuses. There are no new loot
entries, tiers, new-run defaults or release-gate changes. These fixed powers need
no extra persisted state; amulet energy/Reaping integration is documented separately.

- **Vampirism** (`vampirism`): restore 5% of the HP actually removed by the
  wearer's melee hit, capped by the victim owner's pre-hit HP. This adds to
  Fight Long's 20%, yielding 25%. Overkill, missed/zero-damage hits, ranged hits
  (even zero-distance projectiles), reflected environmental packets and another
  ally's damage do not heal the wearer. A separately equipped ally can heal its
  own pool. Existing healing suppression, living-owner checks and HP caps remain.
- **Flight** (`flight`): continuously participates in the existing run-aware
  flight predicates while worn, with no temporary condition or extra clock.
  Chasm traversal, swimming/contact checks, currents, terrain footing, hazards,
  active actor pathing and inactive actor pathing all see this flight source.
  Fire, gas and attacks are unaffected. Equipment mutation captures flight before
  removal and immediately settles flight loss afterward, including shared Half
  Here endpoints. A second Flight ring or independent source keeps flight alive.
- **Fire Resistance** (`fire-resistance`): multiply the finalized fire HP packet
  by 0.5, after the existing damage scaling/upward rounding and immunity checks.
  Thus 3 damage becomes 1.5 and 1 becomes 0.5. This also covers normal-clock
  burning. Ignition, burning duration, fire propagation and other actors are
  unchanged. Existing fire immunity still prevents the entire packet.
- **Poison Resistance** (`poison-resistance`): multiply finalized `poison` and
  `venom` HP packets by 0.5 without curing or shortening either status. This does
  not mitigate caustic or Breather gas.
- **Corrosion Resistance** (`corrosion-resistance`): multiply finalized `caustic`,
  `ichor`, `acid` and `breatherGas` HP packets by 0.5. Existing gas/ichor immunities remain
  stronger. The Breather environmental channel can still emit `breatherGas`, so
  it is included even though contemporary Breather emissions use caustic gas.
  The subsequently authored Acid amulet supplies a direct acid packet and a
  temporary corrosion DR penalty, whose source integration owns halving that
  penalty without shortening its duration. Ordinary unrelated DR debuffs are
  not silently relabeled as corrosion.

Copies of these five fixed powers never stack. Different named rings combine.
Their independent Tinker grants and unrelated existing bonuses are retained.
Light is the separately approved additive exception, handled by the lighting
integration. `count_effect` counts distinct worn item entries, never Item.quantity;
`has_effect` uses the same ownership/exclusion rules and tests count > 0.

## Integration and ownership

The shared melee aftermath already receives gross HP loss capped before overkill
and death rescue. Vampirism joins that path instead of adding another attack
or using displayed damage. It creates no RNG draws and heals only once per
qualifying strike. Active normal and haste attacks therefore use the same rules;
resistance status ticks remain on the existing normal clock. An inactive actor's
exact simulation uses the same damage, attack and environment functions.

Both `engine::flying_in` and `travel::flying` include the named Flight ring. The
two previously actor-only inactive-floor flight queries now use the run-aware
predicate too. Existing unequip, drop, throw and Tinker-removal settlement hooks
already capture before/after equipment flight and are reused rather than adding
an extra end-turn fall. A ring contributes no duration to expire or maintain.

The shared ring query resolves Half Here and dream-body equipment to their
player owner and uses actor-local inventory for independent copies. Mini-Me and
Waking Nightmare decoys inherit no ring effects even if a malformed fixture
carries copied inventory. Offscreen proxies do not borrow the absent player's
inventory. Actor-local rings continue working offscreen. Resistance modifies the
packet into the shared HP owner; it does not resist the same packet a second time
during endpoint synchronization.

Raw Run serialization, Snapshot and Future capture already carry item identity,
wearing and actor inventory. No extra persisted power state exists. Focused tests
exercise raw serialization and Snapshot restoration. Future/time-travel source
paths retain those fields, but full Engine50 admission, replay, Revert/Future
interaction and platform playtesting remain behind the unchanged R50 gate.

## Verification scope

`crates/corpse-core/tests/revision50_combat_rings.rs` covers canonical identity,
wearing and revision gates; fixed-copy nonstacking; count ignoring quantity;
5%/25% overkill-capped melee healing; HP caps; misses/zero damage; ranged exclusion;
ally isolation; healing suppression; fractional resistance and immunity; normal
versus haste status ticks; unchanged ignition and other actors; exact resistance
kind classification; chasm/water/current movement; immediate last-ring loss;
independent flight sources; Half Here ownership; Mini-Me/decoy exclusions; exact
offscreen burning/poison without absent-player effects; serialization/Snapshot;
and separate Tinker grants.

The fixtures are authored inputs, not production loot. At the time this document
was written, the cases were awaiting the coordinating native test run. Refer to
the coordinating verification record for actual executed results, not this list
of intended assertions. No performance, balance or platform certification is
claimed.

## Complexity

Let I be the equipment owner's inventory entries, C the actor's condition entries,
P the number of relevant damage packets/qualifying melee aftermath calls, Q the
number of run-aware flight checks, A the actor population and T exact catch-up
turns. A named-effect query scans I entries with a constant number of condition
lookups: expected O(I), conservative hash-collision worst case O(I+C), and O(1)
additional memory. There is no equipment-slot bound on I. `count_effect` scans
all entries; `has_effect` uses that same count rather than an early-exit scan.

Relevant resistance packets and qualifying melee healing each add one such
query. Flight queries add one query whenever an earlier intrinsic-flight branch
does not short-circuit. Incremental work is O((P+Q) × (I+C)) for equal-size
inventories, or the sum of individual queried owner sizes in mixed populations.
It is not constant-time merely because the named power is fixed. Existing
visibility, path-field construction, danger-map cell scans, actor lookup,
healing suppression and Half Here synchronization costs remain additional.
For example, a path's hazard scan can call the flight query per cell; its inventory
cost scales with those cells, not just the number of paths. Offscreen catch-up
repeats these query sites across T exact turns and retains its pre-existing
pathfinding/cache and environment costs. The inactive dynamic guard can add
O(sum(I+C)) across A examined actors; the existing feeding guard currently forces
living R50 actors through the exact route first.

Removal retains existing mutation snapshots, resource settlement, clone syncing
and travel/fall work; this is not represented as O(I) total. The new ring lookup
itself allocates no inventory copies, persistent cache, per-frame work or RNG
state. Melee and packet arithmetic is O(1) after lookup. The shared packet hook
leaves immunity and unsupported-kind admission ahead of HP mutation; direct acid packet admission is owned by the separately authored amulet effect. These are source-derived incremental bounds,
not measured timings. Run focused integration tests and the complete workspace
suite, then profile oversized-inventory, actor and pathfinding fixtures before
making speed claims.

The designer renamed Transference to Vampirism. New explicit identity is
`vampirism`; the old draft `transference` value is a read alias for the same
power, never a second stacking category. `display_name` renders both as
“Ring of Vampirism” without rewriting historical item payloads.
