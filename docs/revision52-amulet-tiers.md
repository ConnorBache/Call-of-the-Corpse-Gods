# Revision 52 amulet birth tiers

## Mechanics and compatibility

Unreleased simulation revision **52 exactly** adds immutable amulet birth tiers. Floors 1–4 produce Tier 1; 5–9 Tier 2; 10–14 Tier 3; successive five-floor bands continue through Tier 8 on 35–39 and Tier 9 on 40. Canonical item names are `Tier X Amulet of Y`. Newly created loot receives its birth-floor tier after the existing subtype draw, with no extra RNG, ID, count or subtype-weight changes. The twelve subtype entries remain unchanged. The early-floor schedule and special-reward/unique-bow budgets are retained.

The optional item field `amuletTier` stores an integer 1–9 and is admitted only in exact R52 on a recognized amulet. Missing means the authored base tier at every floor, including imported saves. Null, fractional, out-of-range, nonfinite, wrong-kind, and wrong-revision explicit fields reject before supported command mutation. Existing items retain their stored tier and separate 50-capacity reserves through acquisition, drop/pickup, travel, copies, Tinker, DEBUG reconstruction, saves, Snapshot and Future. Generic constructors without a known birth context retain missing/base tier. No load or pickup step infers a birth tier from current depth.

Damage and periodic rates share `revision52_amulet_tiers::DAMAGE`, one fixed row per tier. Columns are Lightning / Firebolt / Acid / Iceblast / Floods / Poison per normal tick / Roots per normal tick:

- T1: 50 / 80 / 40 / 30 / 20 / 6 / 3
- T2: 54 / 81 / 51 / 45 / 35 / 14 / 11
- T3: 56 / 82 / 54 / 46 / 42 / 15 / 12
- T4: 57 / 83 / 55 / 49 / 43 / 17 / 17
- T5: 71 / 84 / 67 / 67 / 61 / 24 / 22
- T6: 72 / 85 / 68 / 68 / 62 / 25 / 23
- T7: 73 / 86 / 69 / 69 / 63 / 26 / 24
- T8: 87 / 92 / 87 / 82 / 77 / 27 / 25
- T9: 93 / 93 / 93 / 88 / 84 / 28 / 26

Each higher tier is strictly stronger in its own damage or periodic rate. The design target is approximate rising per-individual cast counts, not HP-scaled damage or guaranteed species counts. Poison retains its 20-turn periodic duration and is reported by applications and elapsed time rather than pretending each cast is a direct packet; Roots retains four ticks. Existing defenses, immunities, flight restrictions, regeneration and area/control tradeoffs remain. Resistance scales the raw selected packet followed by the existing single nearest rounding step.

Poison stores its selected raw rate in `r50PoisonDamage` when applied. Roots stores its selected raw rate in `r52RootsDamage` when applied in R52. Each application overwrites its rate and refreshes its existing duration, without accumulating rates or refreshing a stronger previous rate from a weaker cast. Removing or changing the source item does not change an active status. Roots without its new rate key retains legacy three-damage ticks. Accepted captured rates must equal an actual rate in the nine-row table; roots rate metadata requires a live positive roots timer. Roots clearing/expiry removes the new rate key. Both tick on the existing normal/offscreen clock and keep indirect damage attribution without direct Reaping credit.

Utility profiles add one range tile per tier above one for Blinking (10–18), Beckoning (8–16), and Teleportation target selection (8–16). Discord duration rises 6–14 and Obstruction 8–16. Shape, target count, random teleport destination, movement/push rules and every activation cost stay unchanged. Range queries and actual execution use the same resolved item profile; R52 Iceblast/Blink still use reachable physical lines, Lightning still pierces, and previews remain knowledge-filtered.

The public pure profile/name/description/details functions are the single core mechanics/presentation source. `item_spec` remains the activation-cost and 50-capacity source. Legacy revisions use their original names, damage, rates, range, duration, logs, RNG and missing serialized fields. Technical nonstacking explanation belongs here; runtime item copy is concise positive-effect prose.

## Complexity

Let I be carried items, A all persisted actors including initial/copy/pilot bodies, C their condition entries, G ground items, M remembered/detected item records, J DEBUG pending/stashed rewards, and P the relevant item/name payload bytes. The fixed catalogue contains 12 effects and nine tiers. Profile lookup and tier arithmetic cost O(effect bytes) time and O(1) scratch. Birth stamping costs O(P) output allocation and adds no RNG operation or iteration over tiers. The generation loops retain their previous item bounds and order.

New tier/rate admission scans O(I+A+C+G+M+J) time per supported command and explicit snapshot restore, with O(pilot nesting) stack and no materialized JSON. Existing serialized save validation adds fixed checks to its existing canonical-tree walk, O(serialized nodes) time, O(tree depth) stack plus existing canonical Value allocation. Existing snapshot/copy/save representations gain one optional scalar per amulet and one scalar per R52 Roots status; existing clone/serde costs otherwise remain unchanged.

Tiered projectile/area algorithms retain their existing spatial/visibility/alias costs. Blink's endpoint cap rises from 10 to at most 18, and Beckoning's visible-target cap from 8 to at most 16. Beckoning still tries at most eight adjacent destinations; its physical segment is bounded by18 grid points, with existing actor, portal, habitat, diagonal and visibility costs inside the loop. No new path search, caching, retries, floor scan or target-count growth is introduced. Teleport keeps its existing full-floor safe-destination scan; tier alters only target admission. Periodic ticks add O(1) table-validated rate lookup and retain existing damage/lifecycle cost. All timings require measured runtime verification; these are asymptotic statements, not performance claims.

## Verification

The integrated optimized Linux core pass completed 654 tests with zero failures and two ignored: 624 core-library tests (including all 25 new tier regressions), four Fighter profile/lifecycle checks, seven legacy amulet-energy checks, six admission/save/replay/generation checks, three saved role-coefficient checks and ten Snapshot checks. The two full generated worlds assert every tiered item equals the legacy item plus exactly its authorized birth tier/name, then preserve complete world equality after only approved deltas.

The receipt is verification/r52-tier-role-core-02. Exit was zero, and all 460 recorded core source/catalog/lock/toolchain files matched before and after. Log SHA-256: 6c02682068f63296a984eb347f70d7cba51cfb8db9179014a4a2499457e2c11d. The earlier attempt exposed only a synthetic death fixture using a nonexistent species and the old generation expectation; both were corrected without changing runtime mechanics.

Tier tests cover generation/RNG/IDs, all nine profiles, first-four-floor and reward generation, immutable missing/birth tiers, malformed state, actual packets/DR/resistance/immunities, stronger/weaker periodic refresh, utility endpoints and physical hidden-target safety, duplicate charges, copies/drop/pickup, Snapshot/Future/save/replay, DEBUG, Tinker, offscreen upkeep, travel and Reaping. Native UI/render checks subsequently passed eight focused and 50 related regressions (three preexisting ignored), with landscape/portrait captures inspected; see revision52-amulet-tier-ui.md. The final full workspace and Windows cross-build remain separate gates. The final additional-role durability sensitivity and actual tier dispatch are recorded below.


## Actual tiered dispatch and broader-role sensitivity

The final 550-row catalog was checked against the linked library, including optional saved coefficients. An external public-dispatch diagnostic then passed 23 groups, covering all 63 tier/effect first packets, exact costs, resistance/immunity, nonstacking, serialized state/RNG, Reaping/Appropriate and multi-target recharge. Its actual results are identical to the prior tier checkpoint with the unchanged 150 Fighter profiles. Evidence is verification/r52-amulet-numeric-final; result SHA-256 fd68366194a5fcee2938f1e8bee405c1caf3b54fb5f7b2176a929b57146603ae.

Against the floor-40 Cancer profile, actual Tier 9 counts are 17 Lightning, 17 Acid and 18 Iceblast casts; Poison takes four applications over 61 normal turns, and Roots takes 17 over 66 turns. Firebolt defeats Ursa Major in 21 casts with native burning versus 22 in the direct-only model. Tier 5 Roots against the floor-24 Troll takes 62 ticks rather than the ideal arithmetic 61 because native fractional regeneration retains a tiny positive HP remainder; the application count is still 16. These fixtures deliberately use stationary targets and ample energy, not a survival guarantee.

The diagnostic above predates the subsequent designer instruction that Floods consumes shallow water and turns it deep. Its historical repeated-wave receipt confirms the older stopping rule only, not the new R52 contract. The new behavior and dedicated verification are described in revision52-floods.md. Earlier revisions retain their former rule; analytical repeated-direct-cast estimates do not promise immediate repeatability after terrain changes.

The broader selected-role sensitivity covers 138,904 weighted individuals, including 45,476 original Fighters and 93,428 additional-role members, excluding recursive Jackal children. Mixed-role floor-40 direct medians are approximately 14/18/14/15/16 for Lightning/Firebolt/Acid/Iceblast/Floods, versus 17/18/17/18/19 for Fighters. Lighter support and skirmishing bodies remain easier; later tiers and creature durability are not weakened merely to force identical per-role counts. Approximate bands remain the design policy, not a guarantee for each species or encounter.

The 2026-11-09 [radial Floods proposal](revision52-radial-floods.md) is deferred
pending its damage/push decision. Native Apply retains the released targeted
UseAt wave and the unchanged Floods damage table. See the
[release split](revision52-released-floods.md) for compatibility and verification.
