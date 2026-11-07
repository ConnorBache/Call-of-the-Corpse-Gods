> 2026-11-06 correction: carried rings are automatically active and counted toward their independent category cap. "Worn" in historical notes below means carried-active; no manual equip or worn exemption remains. See revision50-carried-inventory.md.

# Revision 50 stat and status rings

## Authored scope and identity

The fixed powers use the existing `Item.effect` identities `willpower`,
`protection`, `slaying`, and `sustain-strength`, with the shared ring query's
revision-50 gate, wearing requirement and ownership rules. Copies of each power
do not stack; legacy enchantment and quantity do not scale it. Different powers
coexist. No quality tiers, loot entries, probability or reserve capacities are
introduced. This checkpoint does not open the overall R50 release gate.

Protection adds **4 DR** to normal effective stats. It participates in existing
DR-before-damage-scaling calculations; armor-piercing and environmental packets
which already bypass DR still bypass it. Slaying adds **4 Offense**, including
the ordinary Offense-based melee/projectile calculations. It adds no Accuracy,
changes no hit roll and bypasses no existing calculation.

## Willpower: duration, not potency

Willpower reduces each new eligible duration to `ceil(duration / 2)`. A positive
fractional duration remains at least one turn. Invalid/nonpositive curse durations
are rejected by the existing admission path. Reduction happens when an effect is
applied, never by speeding its clock or dividing its stored remaining duration on
every turn. Duplicates do not repeatedly halve it.

The source audit found these current eligible paths:

- Confusion gas: the normal eight-turn application becomes four. Existing max-of-
  old/new reapplication remains, so an older longer confusion timer is not cut.
- Hallucinogenic gas: the normal two-turn refresh becomes one. Existing exact
  refresh, player/shared-body eligibility, immunities and normal-world-turn decay
  remain unchanged. This refresh is intentionally not converted to max/add.
- Witch spell phases: Slowed 5 becomes 3, Hemophilia 20 becomes 10, and Poisoned 20
  becomes 10.
- Bloodmancer Hemophilia: 20 becomes 10. Blood Wrest has no curse duration to halve.

Confusion and hallucination are unqualified in the approved power, so hostile,
source-less and self-created gas exposures are all eligible. Enemy curses are
marked at `monster_spells`'s curse call sites, not inferred from every condition
with the same name. The recipient/caster must be opposing sides under the
existing player/ally classification and have different resource owners. Thus an
allied caster can curse a hostile ring wearer, while a self/allied application is
not an enemy curse. Actual projectile interception still selects the recipient
before protection is checked.

Ordinary `apply_status` poison, Hemophilia and slowing are unchanged. Witch poison
and slowing are reduced specifically because these applications are authored
curses. Physical paralysis, silence, poison/venom damage, gas vulnerability,
Screamer Disoriented/weakening, roots/grabs and Yeti slowing receive no Willpower
duration adjustment. There is no current charm/fear condition to implement.
Equipment curses are persistent numeric penalties, not timed enemy curses.

Curse source credit, poison's two-HP packet, Hemophilia's two-percent packet,
spell Energy/cooldowns, gas propagation and RNG remain unchanged. Equipping or
removing Willpower does not rewrite active timers. Later reapplication uses the
then-current equipment and the existing per-effect refresh rule.

## Sustain Strength: every temporary stat reduction

The designer expanded the original enemy-only Offense/Accuracy proposal to
**all temporary reductions**. Sustain Strength therefore halves temporary
negative stat contributions regardless of source, including voluntary/self-
applied stat penalties. It does not halve HP/energy payment, damage, permanent
base-stat reductions, persistent equipment penalties or positive stat bonuses.

The current native stat-derivation audit covers:

- Weakened's five-point Offense penalty.
- Active Screamer `disorientedPenalty` (Accuracy) and `screamPenalty` (Offense).
- Negative active scream bonus/perception contributions, if present in an
  explicit saved/fixture status.
- Negative `buff:<stat>` entries with a positive `buffTicks:<stat>` timer, for
  Offense, Defense, Accuracy, Dodge, DR, Perception, maximum HP and maximum Energy.
  Untimed negative buff entries are not silently reclassified as temporary.

Each source retains its existing integer normalization, then its negative
magnitude is halved exactly: five becomes 2.5, three becomes 1.5. Unlike
Willpower's durations, no new rounding rule is added. Timed-buff relief is added
*after* legacy equipment/enhancement accumulation, because that existing
accumulation rounds the running stat and would otherwise erase a half point.
Existing later damage, enemy-health normalization and capacity-clamping rules are
unchanged. The retired R50 Beat to Death modifier is not revived.

The ring is a live stat modifier. Equipping/removing it immediately changes the
active penalty's effective magnitude, while stored magnitude and duration remain
unchanged. Defensive/resource buffs retain their existing shared-owner scope;
Offense/Accuracy and other body-local modifiers retain their endpoint scope.
Maximum-health settling uses its normal equipment path. The runtime-only health
dirty stamp also records whether Defense/maxHP buff timers are positive, so
starting/removing a timer cannot leave a stale maximum HP. This adds no save field.

## Ownership, offscreen and persistence

Half Here and the sleeping body use owner equipment, but conditions remain local
to the body hit. Independently equipped actors use their own inventory. Decoys
and stats-only Mini-Me gain no live ring effects. Mini-Me still freezes current
effective numerical stats at creation, including the then-current +4 bonuses or
fractional reduced penalty. R50 stats-only snapshots are not rounded a second
time when read as effective stats; ordinary and historical base-stat rounding
remains unchanged. The clone gains neither an additional bonus nor ongoing
Willpower/Sustain protection. Newly applied clone penalties still apply normally.

Normal, haste and exact offscreen paths use existing status clocks and the same
stat/source helpers. A haste-only move does not earn an extra decay step. Already
shortened offscreen timers are not halved again. Raw saves, snapshots and existing
Future anchors preserve rings and stored statuses; restoration does not reapply a
ring or halve a saved timer. No new persisted gameplay state, RNG draw, global
scan or rendering behavior is introduced.

## Regression and verification scope

`crates/corpse-core/tests/revision50_stat_status_rings.rs` covers fixed stat
bonuses, duplicate/enchantment non-scaling, older revisions, ordinary attacks and
DR bypass, all Witch/Bloodmancer curse phases, fractional duration rounding,
reapplication/equipment boundaries, gas source independence, normal status
exceptions and potency, enemy/self/allied provenance, Half Here, actor-local
inventory, Mini-Me/decoy exclusions, haste, exact offscreen clocks, raw saves,
Snapshot and existing Future restoration.

Sustain cases cover all eight stats, exact half points after enhancement
rounding, permanent/untimed/positive exclusions, self weakness, unchanged timers,
resource costs, equip/remove health settling, dirty-cache timer changes, shared
owner/local scopes and Snapshot restoration. Fixtures are explicit inputs, not
production authoring.

These cases are authored and awaiting the coordinating native run; see the
latest verification record for commands and actual results. No platform
playtest, campaign balance or full release admission is claimed here.

## Complexity

Let I be the relevant equipment owner's inventory entries, C its/body's condition
entries, A all actors indexed by the existing actor lookup, and d the existing
actor-index depth. Each effective-stat derivation adds three fixed ring queries
and at most sixteen fixed-key timed-buff lookups. Incremental expected work is
O(I), conservative hash-collision worst case O(I + C), with O(1) additional live
memory. Up to sixteen bounded-length temporary key strings are allocated and
released during the fixed eight-stat check; no collection grows with I or C.
Existing equipment/enhancement scans, actor-index cold rebuilds and downstream
health/combat work are not included in that incremental bound. Unlimited jewelry
means I is not a constant.

Each eligible gas exposure adds one ring query, O(I + C) conservative worst case,
O(1) scratch. Each curse adds the same query and constant-count existing actor
lookups for source/target classification. Warm lookups use the existing O(d)
index; a cold structural rebuild retains the index subsystem's actor/floor/state
cost, not an invented O(1) bound. These run only at existing exposure/cast
boundaries, including exact offscreen simulation, not every display frame or
remaining-duration unit. Oblivious/plane validation, projectile geometry, gas
propagation and COW mutation costs are unchanged.

The health dirty stamp grows by two fixed u64 values and two fixed-key condition
lookups per check. Expected time/additional memory remain O(1); collision-worst
lookup time is O(C). Timer positivity, not the numeric remaining duration, is
stored, so ordinary positive-to-positive ticking does not force extra health
settlement. Full health settlement retains effective-stat and resource-hook
costs. All bounds are source-derived, not measured timings.
