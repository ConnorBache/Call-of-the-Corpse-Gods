# Revision 50 Ring of Ice Immunity

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

The designer renamed Cold Resistance to **Ice Immunity** and explicitly included
prevention of slipping on ice. The canonical ring identity is `ice-immunity`;
its display name is **Ring of Ice Immunity**. Its power is active only while a
recognized ring is worn at simulation revision 50 or later. Duplicate copies
do not strengthen immunity. The shared ring registry owns identity and pack text.

## Implemented scope

- Yeti **Winter's Trail** no longer inflicts its one-turn Slowed effect on an
  immune player endpoint. Half Here reads the shared owner's worn rings, so
  neither that endpoint nor the shared player action stream receives its chill.
  Existing diagonal/portal adjacency and matching-plane rules remain intact.
- Valid voluntary departure from ice cannot slip while the walker has this
  power. Admission happens before the one-third footing roll, so immunity draws
  no RNG. Movement still obeys ordinary collision, occupancy, roots, other
  action costs and terrain rules. Actor-local inventory is supported without
  letting unrelated actors borrow the player's rings.
- Ordinary and haste commands retain their existing clocks. The change prevents
  a new Yeti delay rather than granting a bonus action. Existing pre-action and
  hidden-floor calls use the same guarded chill helper; no new phase hook exists.

The guards are in `monster_chill::apply_adjacency` and `terrain_runtime::slip`.
Both use `revision50_rings::has_effect`, which already handles revision gating,
wearing, shared equipment ownership, and Mini-Me/decoy exclusions. Historical
revisions retain their original chill and footing behavior.

## Cold-related effects audit and preserved behavior

The harmful cold-related mechanics now include Yeti chill, ice slipping, and the authored R50 Iceblast amulet. Iceblast delivers 30 HP cold damage and three turns of slowing. The shared damage-packet boundary reduces cold packets to zero for Ice Immunity before reactions or death handling, and the Iceblast caller also skips its cold-induced slow. This is not blanket immunity to unrelated slow effects.
Yeti chill is Slowed, not damage, paralysis or a Frozen status. Ice slipping
consumes a movement action without displacement; entering ice, blocked movement,
attacks and forced displacement do not make the footing roll. Flying actors,
Yetis and Wendigos retain their pre-existing footing exemptions.

Yetis leave snow on suitable destination cells. Permafrost supplies snow;
mountainous regions around actual Yeti/Wendigo spawns supply ice, snow and
additional chasms. Snow itself causes no HP damage, Slowed or slipping. Fire or a
grounded burning creature melts snow into a cosmetic puddle and quenches
ordinary burning. That beneficial contact remains unchanged. Intrinsic eternal
burning remains, and snow melting does not melt the separate ice layer. The
ring does not erase terrain, remove chasms, or grant immunity to falling.

Witchcraft's five-turn Slowed is unrelated and remains effective. The guard does
not clear a generic existing Slowed field or its source attribution: status
durations can already combine across sources, so doing so could erase unrelated
Witchcraft. The power prevents new cold infliction while worn rather than
silently curing all statuses when equipment changes. Ordinary paralysis,
confusion, poison and other statuses remain outside this power. Reverse
Entropy's `frozen_environment` and `corpse_frozen` are time suppression, not ice.

There is no current cold-damage packet, Frozen/Chilled status, frostbite,
hypothermia or temperature-exposure mechanic. No executable Iceblast or authored
Iceblast effect appears in this checkpoint's source or Revision 50 request.
No such mechanic or numeric effect is invented. A future explicitly authored
ice effect must consult this immunity at its own infliction boundary; generic
damage/status handling is not reclassified merely from an enemy's appearance.

## Verification and persistence

`tests/revision50_ice_immunity.rs` covers canonical identity, wearing, duplicate
copies, historical revision behavior, diagonal Yeti chill, no-RNG ice departure,
removal, unchanged terrain, Half Here shared ownership, actor-local equipment,
portal adjacency, separate dream/waking endpoints, normal/haste action timing,
actual Witchcraft, preservation of unrelated conditions, snow quenching, raw
serialization and Snapshot restoration with deterministic continuation.

No new serialized state is introduced. The existing item effect/wearing fields
and terrain arrays remain the persistence authority. These are explicit R50
fixtures, not playable new-run admission, full Engine50 load/replay, Future
activation or platform acceptance. The coordinating task owns Cargo execution;
new tests are authored but have not been executed during this implementation checkpoint. Consult the
coordinating verification record for actual executed results. Existing
`monster_chill`, `terrain_runtime_r37`, `sub_biome_persistence_r37` and passive
offscreen fixtures remain relevant regression suites.

## Complexity

Let I be the queried owner's inventory entries, C its condition entries, B the
player body endpoints (at most two), N current-floor actors, L the maximum actor
ability-list length, F floor count, and K nodes in the existing physical-link
metric graph. Jewelry has no slot cap, so I is not a constant.

Ice Immunity adds one shared ring query per eligible body in a chill pass and
one per eligible committed footing check. A query costs O(I) expected time,
O(I+C) conservative hash-collision worst-case time and O(1) scratch. Incremental
chill work is therefore O(B(I+C)); incremental movement work is O(I+C). Queries
allocate no cache, change no state, and run at simulation boundaries, not per
rendered frame. Existing immutable ownership may share the same inventory, but
the bound counts each endpoint query.

The rest of chill is not constant-time: body lookup and Yeti collection scan
actors, ability membership adds O(NL), and each candidate comparison can resolve
actors, find a floor and run the portal metric. A conservative bound is
O(NL + B(I+C) + BN(N+F+K²)), before successful status-attribution/condition edits
and the existing actor/floor copy-on-write costs. Temporary body/Yeti collection
is O(N+B), with O(K) portal scratch. Immune endpoints are filtered before these
candidate comparisons. Existing status application and COW work remains in the
status/actor/environment ledgers; immunity does not add such mutation.

Movement keeps existing actor lookup, flight/species admission, terrain lookup,
collision/path and COW costs. Only the new ring scan is attributed here. Snow
melting and its O(V) first puddle allocation are unchanged. Source-derived bounds
are not measured speed claims. Verify focused fixtures plus the full workspace,
then profile large inventories and linked crowded floors if timing is needed.

## Authored amulet integration

The first-six amulet suite adds a direct shared-core cold-packet immunity regression plus Iceblast friendly-fire/slow/immunity coverage. See [revision50-amulet-effects.md](revision50-amulet-effects.md). The global R50 release gate remains closed.
