# R52 stat, Jackal, and carrying follow-ups

The later [minimum starting Dodge amendment](revision52-birth-dodge-minimum.md)
supersedes below-ten creature birth values and the historical creature-Dodge
counts below. Player Dodge 15, higher creature values and the other rules here remain current.

## Approved behavior

The designer requested base player Dodge 15, 20% more base player damage, fixed
monster Dodge taken from each type's first appearance, heartless Jackal births,
and ten carried hearts. This amendment applies to R52. Pre-R52 behavior remains
unchanged. It does not change the ratio hit formula or the creature Accuracy
baseline of 15 plus species/floor offsets.

### Player

New normal, DEBUG, and playtest players have Accuracy 15 and Dodge 15. Starting
Dodge was 10 in the preceding checkpoint. Existing saved player Dodge is retained,
including earned bonuses, and copies inherit recorded stats without another base.

Only innate melee's fixed component rises from 31.5 to 37.8. The existing
`2 * effective Offense` addition, equipment bonuses, blessings, variance, DR,
multipliers and final nearest-integer rounding are unchanged. With no bonuses,
no DR and ordinary damage scaling, the old rounded variance range was 12–20;
it is now 14–24. Actual attack tests exercise the same formula with equipment
and temporary Offense plus DR. Explicit Bow and thrown item bases are unchanged.
New Mini-Me and Clay Soldier snapshots inherit the updated innate base through
their existing copy paths. Previously saved copied weapon values are retained.

The base damage helper is a live R52 rule, so existing R52 players receive the
new innate damage without a saved-stat migration. Pre-R52 innate damage remains
45 (R50) or 31.5 (R51). Older R52 replays can therefore produce different results.

Against fresh Dodge 15, early creatures with Accuracy 12–14 have ordinary melee
hit chances of 40–46.67%, before overrides. Player Accuracy stays 15.

### Creature Dodge

For every new R52 profiled body, use the Dodge offset from its exact profile
key's earliest authored floor, then add the existing creature Dodge baseline 10.
The natural, Nest, Queen, Witch, Birther, and Greater Birther keys remain distinct.
All other profile stats still use the current floor. Catalog rows are retained
unchanged as authored data; this is a new-birth projection, not a catalog rewrite.
Generic constructors without a floor use the same first-appearance Dodge.
Unprofiled species retain their authored values, including special fixed tiers.

Of 550 profile rows across 64 keys, 240 later-floor rows have lower Dodge and
310 remain unchanged. No row gains Dodge. Examples: Rat stays 7 instead of rising
to 11; Cave Bat stays 10 instead of rising to 23; Dragon stays 72 instead of 84.
Negative stored values remain negative; only the hit formula clamps its denominator.

Loaded, wounded, revived, grown, copied, travelled, and temporally restored actors
retain their recorded Dodge. Temporary and equipment modifiers still apply.
There is no per-individual random Dodge roll to remove and no new RNG draw.

### Jackals

New R52 `pale:Jackal` bodies are heartless, whether naturally generated or born
from a Birther. A Birther produced by a Greater Birther uses the ordinary Birther
child rule. Distinct Greater Jackals, other Pale species, traits, and encounter
composition are not changed. Heartless Jackals leave no heart on death.
The existing retained-loot allocator continues to compensate for lost heart slots;
there is no special Jackal budget exception. In two deterministic 40-floor audits,
ordinary ground item counts increased by four and three items respectively, all
on floors 5–7. Terrain and enemy counts/types stayed unchanged. Existing ID-based
disguises and Nightmare positions on affected loot floors can shift as item IDs
and occupied ground cells change. These seed examples are not a universal rate.

Existing saved Jackals are not retroactively marked heartless. Pre-R52 births
and heart rewards remain unchanged. Authored heart templates are retained for
legacy saves and deliberate construction, but normal heartless drop guards apply.

### Carrying

R52 base heart capacity rises from 5 to 10; ring, amulet, potion, trinket, and all
existing ability capacity bonuses are unchanged. Pickup, storage admission,
capacity preview and removal of capacity-granting jewelry use the same
revision-aware category limits. Ten hearts fit; an eleventh remains on the ground
and an over-capacity imported inventory is rejected. Existing R52 runs gain the
new carrying limit without changing their items. Pre-R52 capacity remains five.

## Verification limits

`revision52_stat_followups` covers independent numeric and actual melee checks,
all 550 first-Dodge profiles, real natural/summoned Jackal births and no-heart
deaths, pickup limits, load admission, jewelry removal and saved/Snapshot stats.
The existing R52 birth and admission suites use an independent earliest-row
oracle; they preserve all unrelated fields and normal/haste/offscreen/temporal
paths already exercised there. Matching aggregate and Windows receipts are
required after all isolated patches and recovery gaps are resolved. Focused
checks alone do not certify the recovered complete game or Windows runtime.

## Complexity

Let R be the fixed profile count (550), T the existing fixed tuning count,
I inventory entries, B owned blessings, and L the profile key byte length.
At each new profiled body, first-Dodge selection adds a linear R-row scan with
string comparisons, worst-case O(R*L) time and O(1) extra space. Generic and
summoned birth include their existing T-row scans; contextual birth already
performs another R-row floor-profile scan. Catalog JSON parsing is a one-time
O(total catalog bytes) allocation shared by both lookups. No lookup occurs
per attack, frame, load, growth, or rewind. The bounded catalog is deliberately
not duplicated into a speculative cache.

Player starting assignment, innate base arithmetic, and Jackal heartless writes
add O(1) time/space at existing boundaries. Actual attack effective-stat and
lifecycle work is unchanged. Capacity queries remain O(B); item fit/pickup
admission is O(I+B) with O(1) scratch inside the category check. Jewelry removal
retains its existing item clone/ability reconstruction costs. No new RNG,
serialization field, unbounded collection, or temporal traversal is introduced.
These are analytical bounds, not measured performance guarantees.
